#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod dashboard;
mod layout;

use aio_tui::client::ObserverClient;
use config::Config;
use dashboard::Dashboard;
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager,
};

struct AppState {
    config: Config,
    path: PathBuf,
    dashboard: Dashboard,
    dirty_at: Option<Instant>,
    grid_size: (u16, u16),
}

struct ConnectionGate(tokio::sync::Mutex<()>);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    config: Config,
    has_token: bool,
    error: Option<String>,
}

#[tauri::command]
fn float_settings(app: tauri::AppHandle) -> Result<Settings, String> {
    let state = app.state::<Mutex<AppState>>();
    let state = state.lock().map_err(|_| "悬浮窗状态不可用")?;
    Ok(Settings {
        config: state.config.clone(),
        has_token: state.dashboard.client.is_some(),
        error: state.dashboard.error.clone(),
    })
}

#[tauri::command]
async fn float_connect(
    app: tauri::AppHandle,
    ip: String,
    port: u16,
    token: String,
) -> Result<(), String> {
    let gate = app.state::<ConnectionGate>();
    let _guard = gate.0.lock().await;
    let config = {
        let state = app.state::<Mutex<AppState>>();
        let state = state.lock().map_err(|_| "悬浮窗状态不可用")?;
        Config {
            ip,
            port,
            ..state.config.clone()
        }
    };
    config.validate()?;
    let credential_config = config.clone();
    let token = tauri::async_runtime::spawn_blocking(move || {
        if token.is_empty() {
            credential_config
                .credential()?
                .get_password()
                .map_err(|_| "请输入访问令牌".to_string())
        } else {
            Ok(token)
        }
    })
    .await
    .map_err(|_| "无法读取访问令牌")??;
    let client =
        ObserverClient::remote(&config.ip, config.port, &token).map_err(|_| "连接参数无效")?;
    let scope = aio_observer_protocol::CliScope::parse(&config.scope)
        .unwrap_or(aio_observer_protocol::CliScope::Codex);
    client.snapshot(scope, 0).await.map_err(|reason| {
        if reason == aio_tui::client::OfflineReason::Unauthorized {
            "认证失败，请检查访问令牌".to_string()
        } else {
            reason.label().to_string()
        }
    })?;
    let credential_config = config.clone();
    tauri::async_runtime::spawn_blocking(move || {
        credential_config
            .credential()?
            .set_password(&token)
            .map_err(|_| "无法保存至系统凭据存储".to_string())
    })
    .await
    .map_err(|_| "无法保存访问令牌")??;
    {
        let state = app.state::<Mutex<AppState>>();
        let mut state = state.lock().map_err(|_| "悬浮窗状态不可用")?;
        let next = Config {
            ip: config.ip,
            port: config.port,
            ..state.config.clone()
        };
        config::save(&state.path, &next)?;
        state.config = next;
        state.dashboard.reconnect(client);
    }
    app.emit("float-config", ()).map_err(|_| "无法刷新窗口")?;
    Ok(())
}

#[tauri::command]
fn float_layout(app: tauri::AppHandle, action: String) -> Result<(), String> {
    let state = app.state::<Mutex<AppState>>();
    let mut state = state.lock().map_err(|_| "悬浮窗状态不可用")?;
    let previous = state.config.layout;
    match action.as_str() {
        "cycle" => {
            state.config.layout = match previous {
                layout::LayoutMode::Single => layout::LayoutMode::Horizontal,
                layout::LayoutMode::Horizontal => layout::LayoutMode::Vertical,
                layout::LayoutMode::Vertical => layout::LayoutMode::Single,
            }
        }
        "single" => state.config.layout = layout::LayoutMode::Single,
        "horizontal" => state.config.layout = layout::LayoutMode::Horizontal,
        "vertical" => state.config.layout = layout::LayoutMode::Vertical,
        "swap" => match previous {
            layout::LayoutMode::Single => return Ok(()),
            layout::LayoutMode::Horizontal => {
                state.config.horizontal.reversed = !state.config.horizontal.reversed
            }
            layout::LayoutMode::Vertical => {
                state.config.vertical.reversed = !state.config.vertical.reversed
            }
        },
        "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown" => {
            let (columns, rows) = state.grid_size;
            layout::move_divider(&mut state.config, columns, rows, &action);
        }
        _ => return Err("未知布局操作".into()),
    }
    if previous != state.config.layout {
        state.dashboard.invalidate();
    }
    state.dirty_at = Some(Instant::now());
    drop(state);
    update_tray(&app)?;
    app.emit("float-config", ()).map_err(|_| "无法刷新窗口")?;
    Ok(())
}

#[tauri::command]
fn float_toggle_lock(app: tauri::AppHandle) -> Result<(), String> {
    run_menu(&app, "lock")
}

#[tauri::command]
fn float_appearance(
    app: tauri::AppHandle,
    font_size: f64,
    background: String,
    opacity: f64,
    always_on_top: bool,
    click_through: bool,
    locked: bool,
) -> Result<(), String> {
    let state = app.state::<Mutex<AppState>>();
    let mut state = state.lock().map_err(|_| "悬浮窗状态不可用")?;
    let next = Config {
        font_size,
        background,
        opacity,
        always_on_top,
        click_through,
        locked,
        ..state.config.clone()
    };
    next.validate()?;
    let window = app.get_webview_window("main").ok_or("窗口不存在")?;
    window
        .set_always_on_top(next.always_on_top)
        .map_err(|_| "无法更改置顶状态")?;
    if let Err(error) = window.set_resizable(!next.locked) {
        let _ = window.set_always_on_top(state.config.always_on_top);
        return Err(error.to_string());
    }
    if let Err(error) = window.set_ignore_cursor_events(next.click_through) {
        let _ = window.set_always_on_top(state.config.always_on_top);
        let _ = window.set_resizable(!state.config.locked);
        return Err(error.to_string());
    }
    if let Err(error) = config::save(&state.path, &next) {
        let _ = window.set_always_on_top(state.config.always_on_top);
        let _ = window.set_ignore_cursor_events(state.config.click_through);
        let _ = window.set_resizable(!state.config.locked);
        return Err(error);
    }
    state.config = next;
    drop(state);
    update_tray(&app)?;
    app.emit("float-config", ()).map_err(|_| "无法刷新窗口")?;
    Ok(())
}

fn reveal(app: &tauri::AppHandle, interactive: bool) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("窗口不存在")?;
    if interactive {
        window
            .set_ignore_cursor_events(false)
            .map_err(|_| "无法取消鼠标穿透")?;
        let state = app.state::<Mutex<AppState>>();
        let mut state = state.lock().map_err(|_| "悬浮窗状态不可用")?;
        state.config.click_through = false;
        state.dirty_at = Some(Instant::now());
    }
    restore_visible_position(&window)?;
    window.show().map_err(|_| "无法显示窗口")?;
    if interactive {
        let _ = window.set_focus();
    }
    let state = app.state::<Mutex<AppState>>();
    if let Ok(mut state) = state.lock() {
        state.dashboard.set_visible(true);
        state.dashboard.next_refresh = Instant::now();
    }
    update_tray(app)?;
    app.emit("float-config", ()).map_err(|_| "无法刷新窗口")?;
    Ok(())
}

fn restore_visible_position(window: &tauri::WebviewWindow) -> Result<(), String> {
    let position = window.outer_position().map_err(|_| "无法读取窗口位置")?;
    let size = window.outer_size().map_err(|_| "无法读取窗口尺寸")?;
    let monitors = window.available_monitors().map_err(|_| "无法读取显示器")?;
    let visible = monitors.iter().any(|monitor| {
        let area = monitor.work_area();
        position.x >= area.position.x
            && position.y >= area.position.y
            && i64::from(position.x) + i64::from(size.width)
                <= i64::from(area.position.x) + i64::from(area.size.width)
            && i64::from(position.y) + i64::from(size.height)
                <= i64::from(area.position.y) + i64::from(area.size.height)
    });
    if !visible {
        if let Some(monitor) = window
            .current_monitor()
            .ok()
            .flatten()
            .or_else(|| monitors.first().cloned())
        {
            let area = monitor.work_area();
            let width = size.width.min(area.size.width);
            let height = size.height.min(area.size.height);
            window
                .set_size(tauri::PhysicalSize::new(width, height))
                .map_err(|_| "无法恢复窗口尺寸")?;
            let x = position.x.clamp(
                area.position.x,
                area.position
                    .x
                    .saturating_add((area.size.width - width) as i32),
            );
            let y = position.y.clamp(
                area.position.y,
                area.position
                    .y
                    .saturating_add((area.size.height - height) as i32),
            );
            window
                .set_position(tauri::PhysicalPosition::new(x, y))
                .map_err(|_| "无法恢复窗口位置")?;
        }
    }
    Ok(())
}

#[tauri::command]
async fn float_open_settings(app: tauri::AppHandle) -> Result<(), String> {
    reveal(&app, true)?;
    if let Some(window) = app.get_webview_window("settings") {
        window.show().map_err(|_| "无法显示设置")?;
        window.set_focus().map_err(|_| "无法聚焦设置")?;
    } else {
        tauri::WebviewWindowBuilder::new(
            &app,
            "settings",
            tauri::WebviewUrl::App("settings.html".into()),
        )
        .title("AIO Float 设置")
        .inner_size(420.0, 600.0)
        .min_inner_size(340.0, 420.0)
        .always_on_top(true)
        .skip_taskbar(cfg!(target_os = "windows"))
        .center()
        .build()
        .map_err(|_| "无法创建设置窗口")?;
    }
    Ok(())
}

fn menu(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let state = app.state::<Mutex<AppState>>();
    let state = state.lock().unwrap_or_else(|error| error.into_inner());
    Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "show", "显示窗口 / 取消穿透", true, None::<&str>)?,
            &MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?,
            &CheckMenuItem::with_id(
                app,
                "single",
                "单视图",
                true,
                state.config.layout == layout::LayoutMode::Single,
                None::<&str>,
            )?,
            &CheckMenuItem::with_id(
                app,
                "horizontal",
                "左右双列",
                true,
                state.config.layout == layout::LayoutMode::Horizontal,
                None::<&str>,
            )?,
            &CheckMenuItem::with_id(
                app,
                "vertical",
                "上下单列",
                true,
                state.config.layout == layout::LayoutMode::Vertical,
                None::<&str>,
            )?,
            &MenuItem::with_id(
                app,
                "swap",
                "互换位置 (s)",
                state.config.layout != layout::LayoutMode::Single,
                None::<&str>,
            )?,
            &CheckMenuItem::with_id(
                app,
                "lock",
                "锁定窗口 (l)",
                true,
                state.config.locked,
                None::<&str>,
            )?,
            &CheckMenuItem::with_id(
                app,
                "top",
                "始终置顶",
                true,
                state.config.always_on_top,
                None::<&str>,
            )?,
            &CheckMenuItem::with_id(
                app,
                "through",
                "鼠标穿透",
                true,
                state.config.click_through,
                None::<&str>,
            )?,
            &MenuItem::with_id(app, "hide", "隐藏", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?,
        ],
    )
}

#[tauri::command]
fn float_menu(app: tauri::AppHandle) -> Result<(), String> {
    let menu = menu(&app).map_err(|_| "无法创建菜单")?;
    let window = app.get_webview_window("main").ok_or("窗口不存在")?;
    window.popup_menu(&menu).map_err(|_| "无法显示菜单".into())
}

fn run_menu(app: &tauri::AppHandle, id: &str) -> Result<(), String> {
    match id {
        "quit" => app.exit(0),
        "show" => reveal(app, true)?,
        "settings" => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = float_open_settings(app.clone()).await {
                    let _ = app.emit("float-error", error);
                }
            });
        }
        "hide" => {
            app.get_webview_window("main")
                .ok_or("窗口不存在")?
                .hide()
                .map_err(|_| "无法隐藏窗口")?;
            if let Ok(mut state) = app.state::<Mutex<AppState>>().lock() {
                state.dashboard.set_visible(false);
            }
        }
        "single" | "horizontal" | "vertical" | "swap" => float_layout(app.clone(), id.into())?,
        "top" | "through" | "lock" => {
            let config = app
                .state::<Mutex<AppState>>()
                .lock()
                .map_err(|_| "悬浮窗状态不可用")?
                .config
                .clone();
            float_appearance(
                app.clone(),
                config.font_size,
                config.background,
                config.opacity,
                if id == "top" {
                    !config.always_on_top
                } else {
                    config.always_on_top
                },
                if id == "through" {
                    !config.click_through
                } else {
                    config.click_through
                },
                if id == "lock" {
                    !config.locked
                } else {
                    config.locked
                },
            )?;
        }
        _ => {}
    }
    update_tray(app)
}

fn update_tray(app: &tauri::AppHandle) -> Result<(), String> {
    if let Some(tray) = app.tray_by_id("float") {
        tray.set_menu(Some(menu(app).map_err(|_| "无法更新菜单")?))
            .map_err(|_| "无法更新菜单")?;
    }
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            let _ = reveal(app, true);
        }))
        .manage(ConnectionGate(tokio::sync::Mutex::new(())))
        .invoke_handler(tauri::generate_handler![
            float_settings,
            float_connect,
            float_appearance,
            float_open_settings,
            float_menu,
            dashboard::float_frame,
            dashboard::float_key,
            dashboard::float_focus,
            float_layout,
            float_toggle_lock
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let path = app.path().app_config_dir()?.join("settings.json");
            let (config, error) = match config::load(&path) {
                Ok(config) => (config, None),
                Err(error) => (Config::default(), Some(error)),
            };
            let client = config
                .credential()
                .ok()
                .and_then(|entry| entry.get_password().ok())
                .and_then(|token| ObserverClient::remote(&config.ip, config.port, &token).ok());
            let needs_settings = client.is_none();
            let mut dashboard = Dashboard::new(&config, client);
            dashboard.error = error;
            app.manage(Mutex::new(AppState {
                config: config.clone(),
                path,
                dashboard,
                dirty_at: None,
                grid_size: (1, 1),
            }));
            let window = app
                .get_webview_window("main")
                .ok_or("missing main window")?;
            #[cfg(target_os = "windows")]
            window.set_skip_taskbar(true)?;
            window.set_size(tauri::LogicalSize::new(config.width, config.height))?;
            if let (Some(x), Some(y)) = (config.x, config.y) {
                window.set_position(tauri::LogicalPosition::new(x, y))?;
            }
            window.set_always_on_top(config.always_on_top)?;
            window.set_resizable(!config.locked)?;
            let mut tray = TrayIconBuilder::with_id("float")
                .tooltip("AIO Float")
                .menu(&menu(app.handle())?)
                .show_menu_on_left_click(true);
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;
            // Restore click-through only after the tray recovery entry exists.
            window.set_ignore_cursor_events(config.click_through && !needs_settings)?;
            reveal(app.handle(), false)?;
            let handle = app.handle().clone();
            window.on_window_event(move |event| {
                if matches!(
                    event,
                    tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_)
                ) {
                    if let Some(window) = handle.get_webview_window("main") {
                        if let (Ok(scale), Ok(position), Ok(size)) = (
                            window.scale_factor(),
                            window.outer_position(),
                            window.inner_size(),
                        ) {
                            if size.width > 0 && size.height > 0 {
                                if let Ok(mut state) = handle.state::<Mutex<AppState>>().lock() {
                                    state.config.x = Some(f64::from(position.x) / scale);
                                    state.config.y = Some(f64::from(position.y) / scale);
                                    state.config.width = f64::from(size.width) / scale;
                                    state.config.height = f64::from(size.height) / scale;
                                    state.dirty_at = Some(Instant::now());
                                }
                            }
                        }
                    }
                }
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = run_menu(&handle, "hide");
                }
            });
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut topology = Vec::new();
                let mut next_window_check = Instant::now();
                loop {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    if Instant::now() >= next_window_check {
                        next_window_check = Instant::now() + Duration::from_millis(500);
                        if let Some(window) = handle.get_webview_window("main") {
                            let visible = window.is_visible().unwrap_or(false)
                                && !window.is_minimized().unwrap_or(false);
                            if let Ok(mut state) = handle.state::<Mutex<AppState>>().lock() {
                                state.dashboard.set_visible(visible);
                            }
                            if let Ok(monitors) = window.available_monitors() {
                                let current = monitors
                                    .iter()
                                    .map(|m| {
                                        (
                                            m.position().x,
                                            m.position().y,
                                            m.size().width,
                                            m.size().height,
                                            m.scale_factor().to_bits(),
                                        )
                                    })
                                    .collect::<Vec<_>>();
                                if topology != current {
                                    topology = current;
                                    let _ = restore_visible_position(&window);
                                }
                            }
                        }
                    }
                    dashboard::refresh(&handle);
                    if let Ok(mut state) = handle.state::<Mutex<AppState>>().lock() {
                        if state
                            .dirty_at
                            .is_some_and(|at| at.elapsed() >= Duration::from_millis(300))
                        {
                            if let Err(error) = config::save(&state.path, &state.config) {
                                state.dashboard.error = Some(error);
                            }
                            state.dirty_at = None;
                        }
                    }
                }
            });
            if needs_settings {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let _ = float_open_settings(handle).await;
                });
            }
            Ok(())
        })
        .on_menu_event(|app, event| {
            if let Err(error) = run_menu(app, event.id.as_ref()) {
                let _ = app.emit("float-error", error);
            }
        })
        .build(tauri::generate_context!())
        .expect("AIO Float startup failed")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                if let Ok(state) = app.state::<Mutex<AppState>>().lock() {
                    let _ = config::save(&state.path, &state.config);
                }
            }
        });
}
