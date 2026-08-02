//! Usage: Desktop resident mode (tray icon + window lifecycle hooks).

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};

const MAIN_WINDOW_LABEL: &str = "main";
const TRAY_PROVIDER_MINI_WINDOW_LABEL: &str = "tray-provider-mini";
#[cfg(target_os = "macos")]
const TRAY_PROVIDER_MINI_SNAPSHOT_EVENT: &str = "tray-provider-mini:snapshot";
const TRAY_ID: &str = "main-tray";
const TRAY_MENU_TOGGLE_ID: &str = "tray.toggle";
const TRAY_MENU_QUIT_ID: &str = "tray.quit";
const LIFECYCLE_INTENT_IDLE: u8 = 0;
const LIFECYCLE_INTENT_EXIT: u8 = 1;
const LIFECYCLE_INTENT_RESTART: u8 = 2;
#[cfg(target_os = "macos")]
const TRAY_PROVIDER_MINI_HIDE_DELAY_MS: u64 = 180;
#[cfg(any(target_os = "macos", test))]
const TRAY_PROVIDER_MINI_WIDTH: f64 = 440.0;
#[cfg(any(target_os = "macos", test))]
const TRAY_PROVIDER_MINI_HEADER_HEIGHT: f64 = 42.0;
#[cfg(any(target_os = "macos", test))]
const TRAY_PROVIDER_MINI_ROW_HEIGHT: f64 = 36.0;
#[cfg(any(target_os = "macos", test))]
const TRAY_PROVIDER_MINI_EMPTY_HEIGHT: f64 = 68.0;
#[cfg(any(target_os = "macos", test))]
const TRAY_PROVIDER_MINI_MAX_VISIBLE_ROWS: usize = 10;
#[cfg(any(target_os = "macos", test))]
const TRAY_PROVIDER_MINI_SCREEN_MARGIN: f64 = 8.0;
#[cfg(any(target_os = "macos", test))]
const TRAY_PROVIDER_MINI_ANCHOR_GAP: f64 = 6.0;

#[cfg(any(target_os = "macos", test))]
#[derive(Debug, Clone, Copy, PartialEq)]
struct TrayProviderMiniAnchor {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    work_x: f64,
    work_y: f64,
    work_width: f64,
    work_height: f64,
    scale_factor: f64,
}

#[cfg(any(target_os = "macos", test))]
#[derive(Debug, Clone, Copy, PartialEq)]
struct TrayProviderMiniPlacement {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[cfg(any(target_os = "macos", test))]
#[derive(Debug, Default)]
struct TrayProviderMiniRuntime {
    tray_hovered: bool,
    window_hovered: bool,
    opening: bool,
    visible: bool,
    generation: u64,
    close_token: u64,
    anchor: Option<TrayProviderMiniAnchor>,
    snapshot: Option<super::tray_provider_mini::TrayProviderMiniSnapshot>,
}

#[cfg(any(target_os = "macos", test))]
#[derive(Debug, Clone, Copy, PartialEq)]
enum TrayProviderMiniHoverAction {
    Start {
        generation: u64,
    },
    Reposition {
        anchor: TrayProviderMiniAnchor,
        provider_count: usize,
    },
    None,
}

pub struct ResidentState {
    tray_enabled: AtomicBool,
    lifecycle_intent: AtomicU8,
    #[cfg(any(target_os = "macos", test))]
    tray_provider_mini: Mutex<TrayProviderMiniRuntime>,
}

impl Default for ResidentState {
    fn default() -> Self {
        Self {
            tray_enabled: AtomicBool::new(true),
            lifecycle_intent: AtomicU8::new(LIFECYCLE_INTENT_IDLE),
            #[cfg(any(target_os = "macos", test))]
            tray_provider_mini: Mutex::new(TrayProviderMiniRuntime::default()),
        }
    }
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CloseRequestAction {
    AllowClose,
    HideToTray,
    Minimize,
}

impl ResidentState {
    #[cfg(any(target_os = "macos", test))]
    fn tray_provider_mini_runtime(&self) -> std::sync::MutexGuard<'_, TrayProviderMiniRuntime> {
        self.tray_provider_mini
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn set_tray_enabled(&self, enabled: bool) {
        let _tray_guard = tray_runtime_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        self.set_tray_enabled_unlocked(enabled);
    }

    fn set_tray_enabled_unlocked(&self, enabled: bool) {
        self.tray_enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn tray_enabled(&self) -> bool {
        self.tray_enabled.load(Ordering::Relaxed)
    }

    pub fn begin_exit(&self) {
        self.lifecycle_intent
            .store(LIFECYCLE_INTENT_EXIT, Ordering::Release);
    }

    pub fn begin_restart(&self) {
        self.lifecycle_intent
            .store(LIFECYCLE_INTENT_RESTART, Ordering::Release);
    }

    pub fn is_terminating(&self) -> bool {
        self.lifecycle_intent.load(Ordering::Acquire) != LIFECYCLE_INTENT_IDLE
    }

    fn close_request_action(&self) -> CloseRequestAction {
        if self.is_terminating() {
            return CloseRequestAction::AllowClose;
        }

        if self.tray_enabled() {
            CloseRequestAction::HideToTray
        } else {
            CloseRequestAction::Minimize
        }
    }

    #[cfg(any(target_os = "macos", test))]
    fn begin_tray_provider_mini_hover(
        &self,
        anchor: TrayProviderMiniAnchor,
    ) -> TrayProviderMiniHoverAction {
        let mut runtime = self.tray_provider_mini_runtime();
        runtime.tray_hovered = true;
        runtime.anchor = Some(anchor);
        runtime.close_token = runtime.close_token.wrapping_add(1);
        if runtime.visible {
            let provider_count = runtime
                .snapshot
                .as_ref()
                .map(|snapshot| snapshot.providers.len())
                .unwrap_or_default();
            return TrayProviderMiniHoverAction::Reposition {
                anchor,
                provider_count,
            };
        }
        if runtime.opening {
            return TrayProviderMiniHoverAction::None;
        }
        runtime.opening = true;
        runtime.generation = runtime.generation.wrapping_add(1).max(1);
        runtime.snapshot = None;
        TrayProviderMiniHoverAction::Start {
            generation: runtime.generation,
        }
    }

    #[cfg(any(target_os = "macos", test))]
    fn leave_tray_provider_mini_hover(&self) -> Option<u64> {
        let mut runtime = self.tray_provider_mini_runtime();
        runtime.tray_hovered = false;
        schedule_tray_provider_mini_close(&mut runtime)
    }

    #[cfg(any(target_os = "macos", test))]
    fn set_tray_provider_mini_window_hovered(&self, hovered: bool) -> Option<u64> {
        let mut runtime = self.tray_provider_mini_runtime();
        if hovered && !runtime.visible {
            return None;
        }
        runtime.window_hovered = hovered;
        if hovered {
            runtime.close_token = runtime.close_token.wrapping_add(1);
            None
        } else {
            schedule_tray_provider_mini_close(&mut runtime)
        }
    }

    #[cfg(any(target_os = "macos", test))]
    fn complete_tray_provider_mini_open(
        &self,
        generation: u64,
        snapshot: super::tray_provider_mini::TrayProviderMiniSnapshot,
    ) -> Option<TrayProviderMiniAnchor> {
        let mut runtime = self.tray_provider_mini_runtime();
        if !runtime.opening || runtime.generation != generation {
            return None;
        }
        runtime.opening = false;
        runtime.visible = true;
        runtime.snapshot = Some(snapshot);
        runtime.anchor
    }

    #[cfg(any(target_os = "macos", test))]
    fn close_tray_provider_mini_if_current(&self, close_token: u64) -> bool {
        let mut runtime = self.tray_provider_mini_runtime();
        if runtime.close_token != close_token || runtime.tray_hovered || runtime.window_hovered {
            return false;
        }
        reset_tray_provider_mini_runtime(&mut runtime)
    }

    #[cfg(target_os = "macos")]
    fn reset_tray_provider_mini(&self) -> bool {
        let mut runtime = self.tray_provider_mini_runtime();
        reset_tray_provider_mini_runtime(&mut runtime)
    }

    #[cfg(any(target_os = "macos", test))]
    pub(crate) fn tray_provider_mini_snapshot(
        &self,
    ) -> Option<super::tray_provider_mini::TrayProviderMiniSnapshot> {
        self.tray_provider_mini_runtime().snapshot.clone()
    }

    #[cfg(all(not(target_os = "macos"), not(test)))]
    pub(crate) fn tray_provider_mini_snapshot(
        &self,
    ) -> Option<super::tray_provider_mini::TrayProviderMiniSnapshot> {
        None
    }
}

#[cfg(any(target_os = "macos", test))]
fn schedule_tray_provider_mini_close(runtime: &mut TrayProviderMiniRuntime) -> Option<u64> {
    if runtime.tray_hovered || runtime.window_hovered {
        return None;
    }
    runtime.close_token = runtime.close_token.wrapping_add(1);
    Some(runtime.close_token)
}

#[cfg(any(target_os = "macos", test))]
fn reset_tray_provider_mini_runtime(runtime: &mut TrayProviderMiniRuntime) -> bool {
    let changed = runtime.opening || runtime.visible || runtime.snapshot.is_some();
    runtime.tray_hovered = false;
    runtime.window_hovered = false;
    runtime.opening = false;
    runtime.visible = false;
    runtime.generation = runtime.generation.wrapping_add(1).max(1);
    runtime.close_token = runtime.close_token.wrapping_add(1);
    runtime.anchor = None;
    runtime.snapshot = None;
    changed
}

#[cfg(any(target_os = "macos", test))]
fn tray_provider_mini_logical_height(provider_count: usize) -> f64 {
    let content_height = if provider_count == 0 {
        TRAY_PROVIDER_MINI_EMPTY_HEIGHT
    } else {
        provider_count.min(TRAY_PROVIDER_MINI_MAX_VISIBLE_ROWS) as f64
            * TRAY_PROVIDER_MINI_ROW_HEIGHT
    };
    TRAY_PROVIDER_MINI_HEADER_HEIGHT + content_height + 2.0
}

#[cfg(any(target_os = "macos", test))]
fn clamp_panel_axis(value: f64, minimum: f64, maximum: f64) -> f64 {
    if maximum <= minimum {
        minimum
    } else {
        value.clamp(minimum, maximum)
    }
}

#[cfg(any(target_os = "macos", test))]
fn tray_provider_mini_placement(
    anchor: TrayProviderMiniAnchor,
    provider_count: usize,
) -> TrayProviderMiniPlacement {
    let scale_factor = anchor.scale_factor.max(1.0);
    let width = TRAY_PROVIDER_MINI_WIDTH * scale_factor;
    let height = tray_provider_mini_logical_height(provider_count) * scale_factor;
    let margin = TRAY_PROVIDER_MINI_SCREEN_MARGIN * scale_factor;
    let gap = TRAY_PROVIDER_MINI_ANCHOR_GAP * scale_factor;
    let work_right = anchor.work_x + anchor.work_width;
    let work_bottom = anchor.work_y + anchor.work_height;
    let desired_x = anchor.x + anchor.width / 2.0 - width / 2.0;
    let below_y = anchor.y + anchor.height + gap;
    let above_y = anchor.y - height - gap;
    let desired_y = if below_y + height <= work_bottom - margin {
        below_y
    } else {
        above_y
    };

    TrayProviderMiniPlacement {
        x: clamp_panel_axis(
            desired_x,
            anchor.work_x + margin,
            work_right - width - margin,
        ),
        y: clamp_panel_axis(
            desired_y,
            anchor.work_y + margin,
            work_bottom - height - margin,
        ),
        width,
        height,
    }
}

fn tray_runtime_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Read the latest canonical settings and apply the tray side effect while
/// holding the same coordinator used by every resident-mode writer. A caller
/// must never apply an old import/settings snapshot directly to the resident
/// state after this function exists.
pub(crate) fn sync_tray_enabled_from_canonical<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<bool, String> {
    let _tray_guard = tray_runtime_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let canonical = crate::settings::read(app).map_err(|error| error.to_string())?;
    let enabled = canonical.tray_enabled;
    if let Some(resident) = app.try_state::<ResidentState>() {
        resident.set_tray_enabled_unlocked(enabled);
    }
    if !enabled {
        hide_tray_provider_mini(app);
    }
    Ok(enabled)
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn hide_tray_provider_mini<R: tauri::Runtime>(_app: &tauri::AppHandle<R>) {}

#[cfg(not(target_os = "macos"))]
pub(crate) fn set_tray_provider_mini_window_hovered<R: tauri::Runtime>(
    _app: &tauri::AppHandle<R>,
    _hovered: bool,
) {
}

#[cfg(not(desktop))]
pub fn setup_tray(_app: &tauri::AppHandle) -> crate::shared::error::AppResult<()> {
    Ok(())
}

#[cfg(not(desktop))]
pub fn show_main_window(_app: &tauri::AppHandle) {}

#[cfg(not(desktop))]
pub fn on_window_event(_window: &tauri::Window, _event: &tauri::WindowEvent) {}

#[cfg(desktop)]
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
#[cfg(desktop)]
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
#[cfg(target_os = "macos")]
use tauri::Emitter;
#[cfg(desktop)]
use tauri::Manager;

#[cfg(target_os = "macos")]
fn tray_provider_mini_anchor(
    app: &tauri::AppHandle,
    cursor: tauri::PhysicalPosition<f64>,
    rect: tauri::Rect,
) -> TrayProviderMiniAnchor {
    let monitor = app
        .monitor_from_point(cursor.x, cursor.y)
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten());
    let scale_factor = monitor
        .as_ref()
        .map(|monitor| monitor.scale_factor())
        .filter(|scale| scale.is_finite() && *scale > 0.0)
        .unwrap_or(1.0);
    let position = rect.position.to_physical::<f64>(scale_factor);
    let size = rect.size.to_physical::<f64>(scale_factor);
    let (work_x, work_y, work_width, work_height) = monitor
        .as_ref()
        .map(|monitor| {
            let work_area = monitor.work_area();
            (
                f64::from(work_area.position.x),
                f64::from(work_area.position.y),
                f64::from(work_area.size.width),
                f64::from(work_area.size.height),
            )
        })
        .unwrap_or((
            position.x - 960.0 * scale_factor,
            position.y,
            1_920.0 * scale_factor,
            1_080.0 * scale_factor,
        ));
    TrayProviderMiniAnchor {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
        work_x,
        work_y,
        work_width,
        work_height,
        scale_factor,
    }
}

#[cfg(target_os = "macos")]
fn apply_tray_provider_mini_geometry(
    app: &tauri::AppHandle,
    anchor: TrayProviderMiniAnchor,
    provider_count: usize,
) -> Result<tauri::WebviewWindow, String> {
    let logical_height = tray_provider_mini_logical_height(provider_count);
    let window = match app.get_webview_window(TRAY_PROVIDER_MINI_WINDOW_LABEL) {
        Some(window) => window,
        None => tauri::WebviewWindowBuilder::new(
            app,
            TRAY_PROVIDER_MINI_WINDOW_LABEL,
            tauri::WebviewUrl::App("index.html?window=tray-provider-mini".into()),
        )
        .title("AIO Coding Hub")
        .inner_size(TRAY_PROVIDER_MINI_WIDTH, logical_height)
        .resizable(false)
        .focusable(false)
        .focused(false)
        .visible(false)
        .decorations(false)
        .transparent(true)
        .background_color(tauri::window::Color(0, 0, 0, 0))
        .effects(
            tauri::window::EffectsBuilder::new()
                .effect(tauri::window::Effect::Popover)
                .state(tauri::window::EffectState::Active)
                .radius(14.0)
                .build(),
        )
        .always_on_top(true)
        .visible_on_all_workspaces(true)
        .skip_taskbar(true)
        .shadow(true)
        .accept_first_mouse(true)
        .build()
        .map_err(|error| format!("failed to create tray provider mini window: {error}"))?,
    };
    let placement = tray_provider_mini_placement(anchor, provider_count);
    window
        .set_size(tauri::PhysicalSize::new(
            placement.width.round().max(1.0) as u32,
            placement.height.round().max(1.0) as u32,
        ))
        .map_err(|error| format!("failed to size tray provider mini window: {error}"))?;
    window
        .set_position(tauri::PhysicalPosition::new(
            placement.x.round() as i32,
            placement.y.round() as i32,
        ))
        .map_err(|error| format!("failed to position tray provider mini window: {error}"))?;
    window
        .set_focusable(false)
        .map_err(|error| format!("failed to keep tray provider mini window unfocused: {error}"))?;
    Ok(window)
}

#[cfg(target_os = "macos")]
fn emit_tray_provider_mini_snapshot(
    app: &tauri::AppHandle,
    snapshot: Option<super::tray_provider_mini::TrayProviderMiniSnapshot>,
) {
    if let Err(error) = app.emit_to(
        TRAY_PROVIDER_MINI_WINDOW_LABEL,
        TRAY_PROVIDER_MINI_SNAPSHOT_EVENT,
        snapshot,
    ) {
        tracing::debug!("failed to emit tray provider mini snapshot: {error}");
    }
}

#[cfg(target_os = "macos")]
fn start_tray_provider_mini_open(app: &tauri::AppHandle, generation: u64) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let db = {
            let db_state = app.state::<crate::app_state::DbInitState>();
            crate::app_state::ensure_db_ready(app.clone(), db_state.inner()).await
        };
        let snapshot = match db {
            Ok(db) => {
                let build_app = app.clone();
                crate::blocking::run("tray_provider_mini_snapshot", move || {
                    super::tray_provider_mini::build_snapshot(&build_app, &db, generation)
                })
                .await
                .unwrap_or_else(|error| {
                    tracing::warn!(
                        error = %error.code(),
                        "tray provider mini snapshot is unavailable"
                    );
                    super::tray_provider_mini::TrayProviderMiniSnapshot::unavailable(generation)
                })
            }
            Err(error) => {
                tracing::warn!(
                    error = %error.code(),
                    "tray provider mini database is unavailable"
                );
                super::tray_provider_mini::TrayProviderMiniSnapshot::unavailable(generation)
            }
        };
        let resident = app.state::<ResidentState>();
        if !resident.tray_enabled() {
            resident.reset_tray_provider_mini();
            return;
        }
        let Some(anchor) = resident.complete_tray_provider_mini_open(generation, snapshot.clone())
        else {
            return;
        };
        match apply_tray_provider_mini_geometry(&app, anchor, snapshot.providers.len()) {
            Ok(window) => {
                if let Err(error) = window.show() {
                    tracing::warn!("failed to show tray provider mini window: {error}");
                    resident.reset_tray_provider_mini();
                    return;
                }
                emit_tray_provider_mini_snapshot(&app, Some(snapshot));
            }
            Err(error) => {
                tracing::warn!("{error}");
                resident.reset_tray_provider_mini();
            }
        }
    });
}

#[cfg(target_os = "macos")]
fn schedule_tray_provider_mini_hide(app: &tauri::AppHandle, close_token: u64) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(
            TRAY_PROVIDER_MINI_HIDE_DELAY_MS,
        ))
        .await;
        let resident = app.state::<ResidentState>();
        if resident.close_tray_provider_mini_if_current(close_token) {
            if let Some(window) = app.get_webview_window(TRAY_PROVIDER_MINI_WINDOW_LABEL) {
                let _ = window.hide();
            }
            emit_tray_provider_mini_snapshot(&app, None);
        }
    });
}

#[cfg(target_os = "macos")]
fn handle_tray_provider_mini_hover(
    app: &tauri::AppHandle,
    cursor: tauri::PhysicalPosition<f64>,
    rect: tauri::Rect,
) {
    let resident = app.state::<ResidentState>();
    if !resident.tray_enabled() {
        hide_tray_provider_mini(app);
        return;
    }
    let anchor = tray_provider_mini_anchor(app, cursor, rect);
    match resident.begin_tray_provider_mini_hover(anchor) {
        TrayProviderMiniHoverAction::Start { generation } => {
            start_tray_provider_mini_open(app, generation);
        }
        TrayProviderMiniHoverAction::Reposition {
            anchor,
            provider_count,
        } => {
            if let Err(error) = apply_tray_provider_mini_geometry(app, anchor, provider_count) {
                tracing::debug!("failed to reposition tray provider mini window: {error}");
            }
        }
        TrayProviderMiniHoverAction::None => {}
    }
}

#[cfg(target_os = "macos")]
fn handle_tray_provider_mini_leave(app: &tauri::AppHandle) {
    if let Some(close_token) = app
        .state::<ResidentState>()
        .leave_tray_provider_mini_hover()
    {
        schedule_tray_provider_mini_hide(app, close_token);
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn set_tray_provider_mini_window_hovered<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    hovered: bool,
) {
    let Some(resident) = app.try_state::<ResidentState>() else {
        return;
    };
    let Some(close_token) = resident.set_tray_provider_mini_window_hovered(hovered) else {
        return;
    };
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(
            TRAY_PROVIDER_MINI_HIDE_DELAY_MS,
        ))
        .await;
        let Some(resident) = app.try_state::<ResidentState>() else {
            return;
        };
        if resident.close_tray_provider_mini_if_current(close_token) {
            if let Some(window) = app.get_webview_window(TRAY_PROVIDER_MINI_WINDOW_LABEL) {
                let _ = window.hide();
            }
            if let Err(error) = app.emit_to(
                TRAY_PROVIDER_MINI_WINDOW_LABEL,
                TRAY_PROVIDER_MINI_SNAPSHOT_EVENT,
                Option::<super::tray_provider_mini::TrayProviderMiniSnapshot>::None,
            ) {
                tracing::debug!("failed to clear tray provider mini snapshot: {error}");
            }
        }
    });
}

#[cfg(target_os = "macos")]
pub(crate) fn hide_tray_provider_mini<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(resident) = app.try_state::<ResidentState>() {
        resident.reset_tray_provider_mini();
    }
    if let Some(window) = app.get_webview_window(TRAY_PROVIDER_MINI_WINDOW_LABEL) {
        let _ = window.hide();
    }
    if let Err(error) = app.emit_to(
        TRAY_PROVIDER_MINI_WINDOW_LABEL,
        TRAY_PROVIDER_MINI_SNAPSHOT_EVENT,
        Option::<super::tray_provider_mini::TrayProviderMiniSnapshot>::None,
    ) {
        tracing::debug!("failed to clear tray provider mini snapshot: {error}");
    }
}

#[cfg(desktop)]
pub fn setup_tray(app: &tauri::AppHandle) -> crate::shared::error::AppResult<()> {
    let toggle_item = MenuItem::with_id(app, TRAY_MENU_TOGGLE_ID, "显示/隐藏", true, None::<&str>)
        .map_err(|e| format!("failed to create tray toggle menu item: {e}"))?;
    let quit_item = MenuItem::with_id(app, TRAY_MENU_QUIT_ID, "退出", true, None::<&str>)
        .map_err(|e| format!("failed to create tray quit menu item: {e}"))?;
    let separator = PredefinedMenuItem::separator(app)
        .map_err(|e| format!("failed to create tray menu separator: {e}"))?;

    let menu = Menu::with_items(app, &[&toggle_item, &separator, &quit_item])
        .map_err(|e| format!("failed to create tray menu: {e}"))?;

    let toggle_id = toggle_item.id().clone();
    let quit_id = quit_item.id().clone();

    #[cfg(target_os = "macos")]
    let icon_bytes = include_bytes!("../../icons/trayTemplate.png");
    #[cfg(not(target_os = "macos"))]
    let icon_bytes = include_bytes!("../../icons/32x32.png");

    let icon = tauri::image::Image::from_bytes(icon_bytes)
        .map_err(|e| format!("failed to load tray icon: {e}"))?;

    let tray_builder = TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("AIO Coding Hub")
        .menu(&menu);

    #[cfg(target_os = "macos")]
    let tray_builder = tray_builder.icon_as_template(true);

    tray_builder
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| {
            if event.id == quit_id {
                app.state::<ResidentState>().begin_exit();
                app.exit(0);
                return;
            }
            if event.id == toggle_id {
                #[cfg(target_os = "macos")]
                hide_tray_provider_mini(app);
                toggle_main_window(app);
            }
        })
        .on_tray_icon_event(|tray, event| {
            #[cfg(target_os = "macos")]
            match &event {
                TrayIconEvent::Enter { position, rect, .. }
                | TrayIconEvent::Move { position, rect, .. } => {
                    handle_tray_provider_mini_hover(tray.app_handle(), *position, *rect);
                }
                TrayIconEvent::Leave { .. } => {
                    handle_tray_provider_mini_leave(tray.app_handle());
                }
                _ => {}
            }
            if let TrayIconEvent::Click {
                button,
                button_state,
                ..
            } = event
            {
                if button == MouseButton::Left && button_state == MouseButtonState::Up {
                    #[cfg(target_os = "macos")]
                    hide_tray_provider_mini(tray.app_handle());
                    show_main_window(tray.app_handle());
                }
            }
        })
        .build(app)
        .map_err(|e| format!("failed to build tray icon: {e}"))?;

    Ok(())
}

#[cfg(desktop)]
pub fn show_main_window(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        return;
    };

    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();

    #[cfg(target_os = "macos")]
    set_dock_visibility(app, true);

    // A WebView that died while the window was hidden should be repaired the
    // moment the user opens the window, not on the next watchdog tick.
    crate::app::heartbeat_watchdog::on_main_window_shown(app);
}

/// Called on startup when `start_minimized` is enabled.
/// The window starts hidden (via `visible: false` in tauri.conf.json).
/// On macOS we also hide the dock icon so the app is tray-only.
#[cfg(desktop)]
pub fn hide_main_window_on_startup(_app: &tauri::AppHandle) {
    #[cfg(target_os = "macos")]
    set_dock_visibility(_app, false);
}

#[cfg(target_os = "macos")]
fn set_dock_visibility(app: &tauri::AppHandle, visible: bool) {
    use tauri::ActivationPolicy;

    let policy = if visible {
        ActivationPolicy::Regular
    } else {
        ActivationPolicy::Accessory
    };

    if let Err(err) = app.set_dock_visibility(visible) {
        tracing::warn!("failed to set Dock visibility: {err}");
    }

    if let Err(err) = app.set_activation_policy(policy) {
        tracing::warn!("failed to set activation policy: {err}");
    }
}

#[cfg(desktop)]
fn toggle_main_window(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        return;
    };

    let is_visible = window.is_visible().unwrap_or(false);
    let is_minimized = window.is_minimized().unwrap_or(false);

    if !is_visible || is_minimized {
        show_main_window(app);
        return;
    }

    let _ = window.hide();

    #[cfg(target_os = "macos")]
    set_dock_visibility(app, false);
}

#[cfg(desktop)]
pub fn on_window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    if window.label() == TRAY_PROVIDER_MINI_WINDOW_LABEL {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            hide_tray_provider_mini(window.app_handle());
        }
        return;
    }
    if window.label() != MAIN_WINDOW_LABEL {
        return;
    }

    // OS-level restore paths (taskbar unminimize, Mission Control) never go
    // through show_main_window; the focus event covers them so a WebView that
    // died while minimized is repaired the moment the user comes back.
    if matches!(event, tauri::WindowEvent::Focused(true)) {
        crate::app::heartbeat_watchdog::on_main_window_shown(window.app_handle());
        return;
    }

    let tauri::WindowEvent::CloseRequested { api, .. } = event else {
        return;
    };

    let resident = window.state::<ResidentState>();
    match resident.close_request_action() {
        CloseRequestAction::AllowClose => {}
        CloseRequestAction::HideToTray => {
            api.prevent_close();
            let _ = window.hide();

            #[cfg(target_os = "macos")]
            set_dock_visibility(window.app_handle(), false);
        }
        CloseRequestAction::Minimize => {
            api.prevent_close();
            let _ = window.minimize();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_request_hides_to_tray_when_resident_mode_enabled() {
        let state = ResidentState::default();
        state.set_tray_enabled(true);

        assert_eq!(state.close_request_action(), CloseRequestAction::HideToTray);
    }

    #[test]
    fn close_request_minimizes_when_resident_mode_disabled() {
        let state = ResidentState::default();
        state.set_tray_enabled(false);

        assert_eq!(state.close_request_action(), CloseRequestAction::Minimize);
    }

    #[test]
    fn explicit_exit_allows_close() {
        let state = ResidentState::default();
        state.begin_exit();

        assert!(state.is_terminating());
        assert_eq!(state.close_request_action(), CloseRequestAction::AllowClose);
    }

    #[test]
    fn explicit_restart_allows_close() {
        let state = ResidentState::default();
        state.begin_restart();

        assert!(state.is_terminating());
        assert_eq!(state.close_request_action(), CloseRequestAction::AllowClose);
    }

    fn test_anchor() -> TrayProviderMiniAnchor {
        TrayProviderMiniAnchor {
            x: 1_700.0,
            y: 0.0,
            width: 24.0,
            height: 24.0,
            work_x: 0.0,
            work_y: 24.0,
            work_width: 1_728.0,
            work_height: 1_080.0,
            scale_factor: 1.0,
        }
    }

    #[test]
    fn tray_provider_mini_hover_freezes_one_open_generation() {
        let state = ResidentState::default();
        let first = state.begin_tray_provider_mini_hover(test_anchor());
        let second = state.begin_tray_provider_mini_hover(test_anchor());

        assert_eq!(first, TrayProviderMiniHoverAction::Start { generation: 1 });
        assert_eq!(second, TrayProviderMiniHoverAction::None);
    }

    #[test]
    fn tray_provider_mini_window_reentry_cancels_delayed_close() {
        let state = ResidentState::default();
        let generation = match state.begin_tray_provider_mini_hover(test_anchor()) {
            TrayProviderMiniHoverAction::Start { generation } => generation,
            action => panic!("unexpected action: {action:?}"),
        };
        let snapshot =
            crate::app::tray_provider_mini::TrayProviderMiniSnapshot::unavailable(generation);
        assert!(state
            .complete_tray_provider_mini_open(generation, snapshot)
            .is_some());
        let close_token = state
            .leave_tray_provider_mini_hover()
            .expect("schedule close");

        assert_eq!(state.set_tray_provider_mini_window_hovered(true), None);
        assert!(!state.close_tray_provider_mini_if_current(close_token));
        assert!(state.tray_provider_mini_snapshot().is_some());
    }

    #[test]
    fn tray_provider_mini_close_clears_the_frozen_snapshot() {
        let state = ResidentState::default();
        let generation = match state.begin_tray_provider_mini_hover(test_anchor()) {
            TrayProviderMiniHoverAction::Start { generation } => generation,
            action => panic!("unexpected action: {action:?}"),
        };
        let snapshot =
            crate::app::tray_provider_mini::TrayProviderMiniSnapshot::unavailable(generation);
        state.complete_tray_provider_mini_open(generation, snapshot);
        let close_token = state
            .leave_tray_provider_mini_hover()
            .expect("schedule close");

        assert!(state.close_tray_provider_mini_if_current(close_token));
        assert!(state.tray_provider_mini_snapshot().is_none());
    }

    #[test]
    fn tray_provider_mini_placement_stays_inside_the_work_area() {
        let placement = tray_provider_mini_placement(test_anchor(), 20);

        assert!(placement.x >= 8.0);
        assert!(placement.y >= 32.0);
        assert!(placement.x + placement.width <= 1_720.0);
        assert!(placement.y + placement.height <= 1_096.0);
        assert_eq!(placement.height, 404.0);
    }
}
