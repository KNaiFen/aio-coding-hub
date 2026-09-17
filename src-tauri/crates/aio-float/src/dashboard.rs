use aio_observer_protocol::{CliScope, ObserverSnapshotV1, OBSERVER_HISTORY_LIMIT_MAX};
use aio_tui::{
    client::{ObserverClient, OfflineReason},
    input::handle_logs_key,
    palette::{with_capability, ColorCapability},
    ui::{draw_logs, DashboardView, LogsState},
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    backend::TestBackend,
    style::{Color, Modifier},
    Terminal,
};
use serde::Serialize;
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::Manager;
use unicode_width::UnicodeWidthStr;

use crate::{config::Config, AppState};

pub struct Dashboard {
    pub logs: LogsState,
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
        Self {
            logs,
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
                self.logs.apply_snapshot(snapshot);
                self.error = None;
            }
            Err(reason) => {
                self.logs.live.set_offline(reason);
                self.error = Some(if reason == OfflineReason::Unauthorized {
                    "认证失败，请更新访问令牌".into()
                } else {
                    reason.label().into()
                });
            }
        }
        self.next_refresh = Instant::now() + interval;
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

pub fn render(logs: &mut LogsState, columns: u16, rows: u16) -> Result<Vec<Cell>, String> {
    let mut terminal =
        Terminal::new(TestBackend::new(columns, rows)).map_err(|_| "无法创建字符网格")?;
    with_capability(ColorCapability::TrueColor, || {
        terminal.draw(|frame| draw_logs(frame, logs))
    })
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

#[tauri::command]
pub fn float_frame(app: tauri::AppHandle, columns: u16, rows: u16) -> Result<Frame, String> {
    let columns = columns.clamp(1, 400);
    let rows = rows.clamp(1, 240);
    let state = app.state::<Mutex<AppState>>();
    let mut state = state.lock().map_err(|_| "悬浮窗状态不可用")?;
    state
        .dashboard
        .logs
        .expire_inactive_selections(Instant::now());
    let cells = render(&mut state.dashboard.logs, columns, rows)?;
    Ok(Frame {
        columns,
        rows,
        cells,
        config: state.config.clone(),
        connected: state.dashboard.client.is_some(),
        error: state.dashboard.error.clone(),
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
    if dashboard.visible && !dashboard.pending && now >= dashboard.next_refresh {
        if let Some(client) = dashboard.client.clone() {
            dashboard.pending = true;
            let (generation, request, scope, view) = (
                dashboard.generation,
                dashboard.request,
                dashboard.logs.live.scope,
                dashboard.logs.view,
            );
            let handle = app.clone();
            dashboard.refresh_task = Some(tauri::async_runtime::spawn(async move {
                let result = if view == DashboardView::Providers {
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
pub fn float_key(app: tauri::AppHandle, key: String, control: bool) -> Result<(), String> {
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
    let action = handle_logs_key(&mut dashboard.logs, key);
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
                    state.dashboard.logs.finish_provider_probe(provider, result);
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
