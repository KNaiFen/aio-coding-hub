use aio_observer_protocol::{CliScope, ObserverSnapshotV1, OBSERVER_HISTORY_LIMIT_MAX};
use aio_tui::{
    client::{ObserverClient, OfflineReason},
    input::{handle_logs_key, next_scope, KeyAction},
    palette::{with_capability, ColorCapability, Palette, Tone},
    ui::{
        dashboard_help_text, draw_header, draw_header_separator, draw_logs_content, DashboardView,
        LogsState,
    },
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    backend::TestBackend,
    style::{Color, Modifier},
    widgets::{Paragraph, Wrap},
    Terminal,
};
use serde::Serialize;
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::Manager;
use unicode_width::UnicodeWidthStr;

use crate::{
    config::Config,
    layout::{self, LayoutMode, Pane, Region},
    AppState,
};

pub struct Dashboard {
    pub logs: LogsState,
    pub providers: LogsState,
    pub focus: Pane,
    pub help: bool,
    pub client: Option<ObserverClient>,
    pub generation: u64,
    pub request: u64,
    pub pending: bool,
    refresh_task: Option<tauri::async_runtime::JoinHandle<()>>,
    pub next_refresh: Instant,
    pub visible: bool,
    pub error: Option<String>,
}

impl Dashboard {
    pub fn new(config: &Config, client: Option<ObserverClient>) -> Self {
        let mut logs = LogsState::new(CliScope::parse(&config.scope).unwrap_or(CliScope::Codex));
        logs.color = true;
        logs.quit_on_q = false;
        let mut providers = LogsState::new(logs.live.scope);
        providers.color = true;
        providers.quit_on_q = false;
        providers.switch_view(DashboardView::Providers);
        Self {
            logs,
            providers,
            focus: Pane::Requests,
            help: false,
            client,
            generation: 0,
            request: 0,
            pending: false,
            refresh_task: None,
            next_refresh: Instant::now(),
            visible: true,
            error: None,
        }
    }
    pub fn invalidate(&mut self) {
        if let Some(task) = self.refresh_task.take() {
            task.abort();
        }
        self.request = self.request.wrapping_add(1);
        self.pending = false;
        self.next_refresh = Instant::now();
    }
    pub fn set_visible(&mut self, visible: bool) {
        if self.visible != visible {
            self.visible = visible;
            self.invalidate();
        }
    }
    pub fn reconnect(&mut self, client: ObserverClient) {
        self.generation = self.generation.wrapping_add(1);
        self.invalidate();
        let scope = self.logs.live.scope;
        self.logs = LogsState::new(scope);
        self.logs.color = true;
        self.logs.quit_on_q = false;
        self.providers = LogsState::new(scope);
        self.providers.color = true;
        self.providers.quit_on_q = false;
        self.providers.switch_view(DashboardView::Providers);
        self.client = Some(client);
        self.error = None;
    }
    fn accept(
        &mut self,
        generation: u64,
        request: u64,
        result: Result<ObserverSnapshotV1, OfflineReason>,
    ) {
        if self.generation != generation || self.request != request {
            return;
        }
        self.pending = false;
        self.refresh_task = None;
        let mut interval = Duration::from_secs(2);
        match result {
            Ok(snapshot) => {
                if snapshot.active_inference_count > 0
                    || snapshot
                        .active_requests
                        .value
                        .as_ref()
                        .is_some_and(|items| !items.is_empty())
                {
                    interval = Duration::from_millis(500);
                }
                let mut provider_snapshot = snapshot.clone();
                if provider_snapshot.providers.is_none() {
                    provider_snapshot.providers = self
                        .providers
                        .live
                        .snapshot
                        .as_ref()
                        .and_then(|previous| previous.providers.clone());
                }
                self.providers.apply_snapshot(provider_snapshot);
                self.logs.apply_snapshot(snapshot);
                self.error = None;
            }
            Err(reason) => {
                self.logs.live.set_offline(reason);
                self.providers.live.set_offline(reason);
                self.error = Some(if reason == OfflineReason::Unauthorized {
                    "认证失败，请更新访问令牌".into()
                } else {
                    reason.label().into()
                });
            }
        }
        self.next_refresh = Instant::now() + interval;
    }

    pub fn pane(&mut self, pane: Pane) -> &mut LogsState {
        match pane {
            Pane::Requests => &mut self.logs,
            Pane::Providers => &mut self.providers,
        }
    }

    fn input(&mut self, key: KeyEvent, target: Option<Pane>) -> KeyAction {
        let none = || KeyAction {
            redraw: true,
            refresh: false,
            probe_provider_id: None,
        };
        if key.code == KeyCode::Char('?') {
            self.help = !self.help;
            return none();
        }
        if self.help {
            if key.code == KeyCode::Esc {
                self.help = false;
            }
            return none();
        }
        if key.code == KeyCode::Tab {
            let scope = next_scope(self.logs.live.scope);
            self.logs.set_scope(scope);
            self.providers.set_scope(scope);
            return KeyAction {
                refresh: true,
                ..none()
            };
        }
        if target.is_none() && matches!(key.code, KeyCode::Left | KeyCode::Right) {
            self.focus = if key.code == KeyCode::Left {
                Pane::Requests
            } else {
                Pane::Providers
            };
            return KeyAction {
                refresh: true,
                ..none()
            };
        }
        handle_logs_key(self.pane(target.unwrap_or(self.focus)), key)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Frame {
    columns: u16,
    rows: u16,
    cells: Vec<Cell>,
    config: Config,
    connected: bool,
    error: Option<String>,
    regions: Vec<Region>,
    focus: Pane,
    macos: bool,
}

#[derive(Serialize)]
pub struct Cell {
    x: u16,
    y: u16,
    text: String,
    width: usize,
    fg: String,
    bg: Option<String>,
    bold: bool,
    italic: bool,
    underline: bool,
}

fn css_color(color: Color, fallback: &str) -> String {
    match color {
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        _ => fallback.into(),
    }
}

#[cfg(test)]
pub fn render(logs: &mut LogsState, columns: u16, rows: u16) -> Result<Vec<Cell>, String> {
    render_grid(columns, rows, |frame| aio_tui::ui::draw_logs(frame, logs))
}

fn render_grid(
    columns: u16,
    rows: u16,
    draw: impl FnOnce(&mut ratatui::Frame),
) -> Result<Vec<Cell>, String> {
    let mut terminal =
        Terminal::new(TestBackend::new(columns, rows)).map_err(|_| "无法创建字符网格")?;
    with_capability(ColorCapability::TrueColor, || terminal.draw(draw))
        .map_err(|_| "无法绘制仪表盘")?;
    let buffer = terminal.backend().buffer();
    let mut cells = Vec::new();
    for y in 0..rows {
        let mut x = 0;
        while x < columns {
            let cell = &buffer[(x, y)];
            let width = cell.symbol().width().max(1).min(usize::from(columns - x));
            let reversed = cell.modifier.contains(Modifier::REVERSED);
            let mut fg = css_color(cell.fg, "#cbd1d7");
            let mut bg = if cell.bg == Color::Reset {
                None
            } else {
                Some(css_color(cell.bg, "#353a41"))
            };
            if reversed {
                let previous = fg;
                fg = bg.unwrap_or_else(|| "#272b33".into());
                bg = Some(previous);
            }
            if cell.modifier.contains(Modifier::DIM) {
                let rgb = u32::from_str_radix(&fg[1..], 16).unwrap_or(0x8b949e);
                fg = format!(
                    "#{:02x}{:02x}{:02x}",
                    ((rgb >> 16) & 255) * 65 / 100,
                    ((rgb >> 8) & 255) * 65 / 100,
                    (rgb & 255) * 65 / 100
                );
            }
            if cell.symbol() != " " || bg.is_some() {
                cells.push(Cell {
                    x,
                    y,
                    text: cell.symbol().into(),
                    width,
                    fg,
                    bg,
                    bold: cell.modifier.contains(Modifier::BOLD),
                    italic: cell.modifier.contains(Modifier::ITALIC),
                    underline: cell.modifier.contains(Modifier::UNDERLINED),
                });
            }
            x += width as u16;
        }
    }
    Ok(cells)
}

fn render_dashboard(
    dashboard: &mut Dashboard,
    config: &Config,
    columns: u16,
    rows: u16,
) -> Result<(Vec<Cell>, Vec<Region>), String> {
    let mut geometry = layout::geometry(config, columns, rows, dashboard.focus);
    if config.layout == LayoutMode::Single && dashboard.pane(dashboard.focus).detail {
        geometry.regions[0].y = 0;
        geometry.regions[0].height = rows;
    }
    let cells = render_grid(columns, rows, |frame| {
        let palette = Palette::detected(true);
        if dashboard.help {
            let divider_keys = if cfg!(target_os = "macos") {
                "Cmd+Option+Shift+方向键"
            } else {
                "Ctrl+方向键"
            };
            let text = format!("AIO Float 操作\n\nm 布局：单视图/左右/上下\ns 互换位置   l 锁定窗口\n{divider_keys} 调整分界\n\n{}", dashboard_help_text(false).replace("AIO TUI 操作\n\n", "").replace("请求/供应商视图", "聚焦请求/供应商"));
            frame.render_widget(
                Paragraph::new(text)
                    .style(palette.style(Tone::Accent))
                    .wrap(Wrap { trim: false }),
                frame.area(),
            );
            return;
        }
        if config.layout == LayoutMode::Single {
            let pane = dashboard.pane(dashboard.focus);
            aio_tui::ui::draw_logs(frame, pane);
            return;
        }
        draw_header(frame, geometry.header, &dashboard.logs.live, true);
        draw_header_separator(frame, geometry.separator, true);
        if geometry.regions.is_empty() {
            frame.render_widget(
                Paragraph::new("空间不足，请放大窗口或减小字号")
                    .style(palette.style(Tone::Muted))
                    .wrap(Wrap { trim: false }),
                geometry.body,
            );
        }
        if !geometry.divider.is_empty() {
            let separator = if config.layout == LayoutMode::Horizontal {
                vec!["│"; usize::from(geometry.divider.height)].join("\n")
            } else {
                "─".repeat(usize::from(geometry.divider.width))
            };
            frame.render_widget(
                Paragraph::new(separator).style(palette.style(Tone::Muted)),
                geometry.divider,
            );
        }
        for region in &geometry.regions {
            draw_logs_content(frame, region.area(), dashboard.pane(region.pane));
        }
        let lock = if config.locked { "已锁定" } else { "锁定" };
        frame.render_widget(
            Paragraph::new(format!("m布局 s互换 l{lock} ?帮助")).style(palette.style(Tone::Muted)),
            geometry.footer,
        );
    })?;
    Ok((
        cells,
        if dashboard.help {
            Vec::new()
        } else {
            geometry.regions
        },
    ))
}

#[tauri::command]
pub fn float_focus(app: tauri::AppHandle, pane: Pane) -> Result<(), String> {
    let state = app.state::<Mutex<AppState>>();
    let mut state = state.lock().map_err(|_| "悬浮窗状态不可用")?;
    if !state.dashboard.help && state.config.layout != LayoutMode::Single {
        state.dashboard.focus = pane;
    }
    Ok(())
}

#[tauri::command]
pub fn float_frame(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    columns: u16,
    rows: u16,
) -> Result<Frame, String> {
    let columns = columns.clamp(1, 400);
    let rows = rows.clamp(1, 240);
    let state = app.state::<Mutex<AppState>>();
    let mut state = state.lock().map_err(|_| "悬浮窗状态不可用")?;
    state
        .dashboard
        .logs
        .expire_inactive_selections(Instant::now());
    state
        .dashboard
        .providers
        .expire_inactive_selections(Instant::now());
    if window.label() == "main" {
        state.grid_size = (columns, rows);
    }
    let config = state.config.clone();
    let (cells, regions) = render_dashboard(&mut state.dashboard, &config, columns, rows)?;
    Ok(Frame {
        columns,
        rows,
        cells,
        config: state.config.clone(),
        connected: state.dashboard.client.is_some(),
        error: state.dashboard.error.clone(),
        regions,
        focus: state.dashboard.focus,
        macos: cfg!(target_os = "macos"),
    })
}

pub fn refresh(app: &tauri::AppHandle) {
    let state = app.state::<Mutex<AppState>>();
    let Ok(mut state) = state.lock() else {
        return;
    };
    let dashboard = &mut state.dashboard;
    let now = Instant::now();
    dashboard.logs.expire_inactive_selections(now);
    dashboard.providers.expire_inactive_selections(now);
    if dashboard.visible && !dashboard.pending && now >= dashboard.next_refresh {
        if let Some(client) = dashboard.client.clone() {
            dashboard.pending = true;
            let (generation, request, scope, include_providers) = (
                dashboard.generation,
                dashboard.request,
                dashboard.logs.live.scope,
                dashboard.focus == Pane::Providers,
            );
            let include_providers = include_providers || state.config.layout != LayoutMode::Single;
            let dashboard = &mut state.dashboard;
            let handle = app.clone();
            dashboard.refresh_task = Some(tauri::async_runtime::spawn(async move {
                let result = if include_providers {
                    client
                        .snapshot_with_providers(scope, OBSERVER_HISTORY_LIMIT_MAX)
                        .await
                } else {
                    client.snapshot(scope, OBSERVER_HISTORY_LIMIT_MAX).await
                };
                if let Ok(mut state) = handle.state::<Mutex<AppState>>().lock() {
                    state.dashboard.accept(generation, request, result);
                }
            }));
        }
    }
}

pub fn key_event(key: &str, control: bool) -> Option<KeyEvent> {
    let code = match key {
        "q" | "Q" => return None,
        "ArrowUp" => KeyCode::Up,
        "ArrowDown" => KeyCode::Down,
        "ArrowLeft" => KeyCode::Left,
        "ArrowRight" => KeyCode::Right,
        "PageUp" => KeyCode::PageUp,
        "PageDown" => KeyCode::PageDown,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "Enter" => KeyCode::Enter,
        "Escape" => KeyCode::Esc,
        "Tab" => KeyCode::Tab,
        value if value.chars().count() == 1 => KeyCode::Char(value.chars().next()?),
        _ => return None,
    };
    Some(KeyEvent::new(
        code,
        if control {
            KeyModifiers::CONTROL
        } else {
            KeyModifiers::NONE
        },
    ))
}

#[tauri::command]
pub fn float_key(
    app: tauri::AppHandle,
    key: String,
    control: bool,
    target: Option<Pane>,
) -> Result<(), String> {
    if app
        .get_webview_window("settings")
        .is_some_and(|w| w.is_focused().unwrap_or(false))
    {
        return Ok(());
    }
    let Some(key) = key_event(&key, control) else {
        return Ok(());
    };
    if aio_tui::input::should_quit(key) {
        app.exit(0);
        return Ok(());
    }
    let state = app.state::<Mutex<AppState>>();
    let mut state = state.lock().map_err(|_| "悬浮窗状态不可用")?;
    let dashboard = &mut state.dashboard;
    let action = dashboard.input(key, target);
    if action.refresh {
        dashboard.invalidate();
    }
    if let (Some(provider), Some(client)) = (action.probe_provider_id, dashboard.client.clone()) {
        let generation = dashboard.generation;
        let handle = app.clone();
        tauri::async_runtime::spawn(async move {
            let result = client.test_provider_availability(provider).await;
            if let Ok(mut state) = handle.state::<Mutex<AppState>>().lock() {
                if state.dashboard.generation == generation {
                    state
                        .dashboard
                        .providers
                        .finish_provider_probe(provider, result);
                }
            }
        });
    }
    let scope = state.dashboard.logs.live.scope.as_str();
    if state.config.scope != scope {
        state.config.scope = scope.into();
        state.dirty_at = Some(Instant::now());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aio_tui::ui::draw_logs;

    #[test]
    fn q_is_ignored_while_other_shortcuts_remain_available() {
        for control in [false, true] {
            assert!(key_event("q", control).is_none());
            assert!(key_event("Q", control).is_none());
        }
        assert_eq!(key_event("Tab", false).unwrap().code, KeyCode::Tab);
        assert!(aio_tui::input::should_quit(key_event("c", true).unwrap()));
        assert!(aio_tui::input::should_quit(KeyEvent::new(
            KeyCode::Char('q'),
            KeyModifiers::NONE
        )));
    }

    #[test]
    fn float_omits_q_hints_in_all_views_and_after_reconnection() {
        let mut dashboard = Dashboard::new(&Config::default(), None);
        for reconnect in [false, true] {
            if reconnect {
                let client = ObserverClient::remote("127.0.0.1", 13799, &"x".repeat(43)).unwrap();
                dashboard.reconnect(client);
            }
            for view in [DashboardView::Requests, DashboardView::Providers] {
                for (help, detail) in [(false, false), (false, true), (true, false)] {
                    dashboard.logs.view = view;
                    dashboard.logs.help = help;
                    dashboard.logs.detail = detail;
                    let text: String = render(&mut dashboard.logs, 100, 30)
                        .unwrap()
                        .into_iter()
                        .map(|cell| cell.text)
                        .collect();
                    assert!(!text.contains('q'));
                    if help {
                        assert!(text.contains("Ctrl-C"));
                    }
                }
            }
        }
        let mut terminal_state = LogsState::new(CliScope::Codex);
        let text: String = render(&mut terminal_state, 100, 30)
            .unwrap()
            .into_iter()
            .map(|cell| cell.text)
            .collect();
        assert!(text.contains("q退出"));
    }

    fn snapshot() -> ObserverSnapshotV1 {
        serde_json::from_value(serde_json::json!({
            "protocolVersion": 1, "appVersion": "test", "generatedAtMs": 42, "scope": "codex",
            "gateway": {"running": true, "port": 37123},
            "preferredProvider": {"available": true, "value": {"cliKey": "codex", "providerName": "中文 e\u{301}", "circuitState": "closed"}},
            "lastRequest": {"available": true}, "dominantProvider": {"available": true},
            "activeInferenceCount": 0, "today": {"available": true},
            "activeRequests": {"available": true, "value": []}, "recentRequests": {"available": true, "value": []}
        })).unwrap()
    }

    fn populated_snapshot() -> ObserverSnapshotV1 {
        let mut value = serde_json::to_value(snapshot()).unwrap();
        value["recentRequests"]["value"] = serde_json::json!([{
            "key":"request-one", "state":"terminal", "cliKey":"codex", "method":"POST", "path":"/v1/responses",
            "providerName":"中文 e\u{301}", "model":"gpt-test", "interrupted":false, "createdAtMs":1, "lastActivityMs":1,
            "attemptCount":1, "retryCount":0, "providerSwitchCount":0, "hasFailover":false, "sessionReuse":false, "route":[]
        }]);
        value["providers"] = serde_json::json!({"available":true,"value":{"items":[{
            "providerId":42,"cliKey":"codex","providerName":"中文供应商", "providerEnabled":true,"routeEnabled":true,
            "authKind":"api_key","preferred":true,"eligibility":"eligible","spendWindows":[]
        }],"truncated":false}});
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn panes_keep_independent_details_scroll_focus_and_provider_tests() {
        let mut d = Dashboard::new(&Config::default(), None);
        d.accept(0, 0, Ok(populated_snapshot()));
        let key = |name| key_event(name, false).unwrap();
        d.input(key("ArrowDown"), None);
        d.input(key("Enter"), None);
        d.input(key("ArrowDown"), None);
        d.input(key("ArrowRight"), None);
        d.input(key("ArrowDown"), None);
        d.input(key("Enter"), None);
        assert!(d.logs.detail && d.providers.detail);
        // Request-only polling while the provider pane is hidden must not erase it.
        let mut requests_only = populated_snapshot();
        requests_only.providers = None;
        d.accept(0, 0, Ok(requests_only));
        assert!(d.providers.detail);
        d.accept(0, 0, Ok(populated_snapshot()));
        assert_eq!(d.logs.detail_scroll, 1);
        assert_eq!(d.input(key("t"), None).probe_provider_id, Some(42));
        assert_eq!(d.input(key("t"), None).probe_provider_id, None);
        d.input(key("ArrowDown"), Some(Pane::Requests));
        assert_eq!(d.focus, Pane::Providers);
        assert_eq!(d.logs.detail_scroll, 2);
        for mode in [
            LayoutMode::Single,
            LayoutMode::Horizontal,
            LayoutMode::Vertical,
        ] {
            let config = Config {
                layout: mode,
                ..Config::default()
            };
            let (cells, regions) = render_dashboard(&mut d, &config, 80, 40).unwrap();
            assert_eq!(
                regions.len(),
                if mode == LayoutMode::Single { 1 } else { 2 }
            );
            assert!(cells
                .iter()
                .all(|cell| usize::from(cell.x) + cell.width <= 80 && cell.y < 40));
        }
        d.input(key("?"), None);
        assert!(render_dashboard(&mut d, &Config::default(), 80, 40)
            .unwrap()
            .1
            .is_empty());
        d.input(key("Escape"), None);
        assert!(d.logs.detail && d.providers.detail);
        d.input(key("Escape"), None);
        assert!(!d.providers.detail && d.logs.detail);
        d.input(key("Tab"), None);
        assert!(!d.logs.detail && !d.providers.detail);
        assert_eq!(d.logs.live.scope, d.providers.live.scope);
        assert!(d.logs.live.snapshot.is_none() && d.providers.live.snapshot.is_none());
    }

    #[test]
    fn combined_grids_share_header_and_preserve_wide_characters_in_panes() {
        let mut d = Dashboard::new(&Config::default(), None);
        d.accept(0, 0, Ok(populated_snapshot()));
        for mode in [LayoutMode::Horizontal, LayoutMode::Vertical] {
            for (columns, rows) in [(1, 1), (3, 9), (24, 16), (80, 40)] {
                let config = Config {
                    layout: mode,
                    ..Config::default()
                };
                let (cells, _) = render_dashboard(&mut d, &config, columns, rows).unwrap();
                assert!(cells.iter().all(|cell| usize::from(cell.x) + cell.width
                    <= usize::from(columns)
                    && cell.y < rows));
                if columns == 80 {
                    let text: String = cells.iter().map(|cell| cell.text.as_str()).collect();
                    assert_eq!(text.matches("并发").count(), 1);
                    assert!(text.contains("gpt-test") && text.contains("中文供应商"));
                    assert!(cells.iter().any(|cell| cell.text == "e\u{301}"));
                }
            }
        }
    }

    #[test]
    fn single_view_keeps_the_original_tui_grid_and_full_window_detail() {
        let config = Config::default();
        for pane in [Pane::Requests, Pane::Providers] {
            for detail in [false, true] {
                let mut dashboard = Dashboard::new(&config, None);
                dashboard.accept(0, 0, Ok(populated_snapshot()));
                dashboard.focus = pane;
                dashboard.pane(pane).select_current(0, Instant::now());
                dashboard.pane(pane).detail = detail;
                let (actual, regions) = render_dashboard(&mut dashboard, &config, 80, 30).unwrap();
                let expected = render(dashboard.pane(pane), 80, 30).unwrap();
                assert_eq!(
                    serde_json::to_value(actual).unwrap(),
                    serde_json::to_value(expected).unwrap()
                );
                assert_eq!(regions[0].y, if detail { 0 } else { 3 });
            }
        }
    }

    #[test]
    fn combined_panes_start_with_content_without_section_titles() {
        for mode in [LayoutMode::Horizontal, LayoutMode::Vertical] {
            let config = Config { layout: mode, ..Config::default() };
            let mut dashboard = Dashboard::new(&config, None);
            dashboard.accept(0, 0, Ok(populated_snapshot()));
            let (cells, regions) = render_dashboard(&mut dashboard, &config, 80, 40).unwrap();
            for region in regions {
                let first_row: String = cells.iter()
                    .filter(|cell| cell.y == region.y && cell.x >= region.x && cell.x < region.x + region.width)
                    .map(|cell| cell.text.as_str())
                    .collect();
                assert!(!first_row.is_empty());
                assert_ne!(first_row, "请求");
                assert_ne!(first_row, "供应商");
            }
        }
    }

    #[test]
    fn disconnect_retains_snapshot_recovery_clears_error_and_switch_clears_data() {
        let mut dashboard = Dashboard::new(&Config::default(), None);
        dashboard.accept(0, 0, Ok(snapshot()));
        dashboard.accept(0, 0, Err(OfflineReason::Unreachable));
        assert_eq!(
            dashboard
                .logs
                .live
                .snapshot
                .as_ref()
                .unwrap()
                .generated_at_ms,
            42
        );
        assert!(dashboard.error.is_some());
        dashboard.accept(0, 0, Ok(snapshot()));
        assert!(dashboard.error.is_none());
        let client = ObserverClient::remote("127.0.0.1", 13799, &"x".repeat(43)).unwrap();
        dashboard.reconnect(client);
        assert!(dashboard.logs.live.snapshot.is_none());
        dashboard.accept(0, 0, Ok(snapshot()));
        assert!(dashboard.logs.live.snapshot.is_none());
    }

    #[test]
    fn hiding_invalidates_pending_response_and_showing_refreshes_immediately() {
        let mut dashboard = Dashboard::new(&Config::default(), None);
        dashboard.pending = true;
        dashboard.set_visible(false);
        assert!(!dashboard.visible && !dashboard.pending);
        dashboard.accept(0, 0, Ok(snapshot()));
        assert!(dashboard.logs.live.snapshot.is_none());
        dashboard.next_refresh = Instant::now() + Duration::from_secs(30);
        dashboard.set_visible(true);
        assert!(dashboard.visible && dashboard.next_refresh <= Instant::now());
    }

    #[test]
    fn float_grid_preserves_tui_graphemes_and_wide_cells_at_every_width() {
        for columns in [1, 8, 24, 40, 80] {
            let mut terminal_state = LogsState::new(CliScope::Codex);
            terminal_state.color = true;
            terminal_state.apply_snapshot(snapshot());
            let mut float_state = LogsState::new(CliScope::Codex);
            float_state.color = true;
            float_state.apply_snapshot(snapshot());
            let cells = render(&mut float_state, columns, 20).unwrap();
            let mut terminal = Terminal::new(TestBackend::new(columns, 20)).unwrap();
            with_capability(ColorCapability::TrueColor, || {
                terminal.draw(|frame| draw_logs(frame, &mut terminal_state))
            })
            .unwrap();
            for cell in cells {
                let tui_cell = &terminal.backend().buffer()[(cell.x, cell.y)];
                assert_eq!(cell.text, tui_cell.symbol());
                assert_eq!(
                    cell.width,
                    tui_cell
                        .symbol()
                        .width()
                        .max(1)
                        .min(usize::from(columns - cell.x))
                );
            }
        }
    }
    #[test]
    fn late_responses_cannot_change_a_new_connection_or_view() {
        let mut dashboard = Dashboard::new(&Config::default(), None);
        dashboard.generation = 2;
        dashboard.request = 4;
        dashboard.accept(1, 4, Err(OfflineReason::Unauthorized));
        dashboard.accept(2, 3, Err(OfflineReason::Unauthorized));
        assert!(dashboard.error.is_none());
        dashboard.accept(2, 4, Err(OfflineReason::Unauthorized));
        assert!(dashboard.error.unwrap().contains("令牌"));
    }
    #[test]
    fn narrow_grids_never_place_cells_outside_the_view() {
        for columns in [1, 8, 24, 40] {
            let mut state = LogsState::new(CliScope::Codex);
            let cells = render(&mut state, columns, 12).unwrap();
            assert!(cells.iter().all(|cell| usize::from(cell.x) + cell.width
                <= usize::from(columns)
                && cell.y < 12));
        }
    }
}
