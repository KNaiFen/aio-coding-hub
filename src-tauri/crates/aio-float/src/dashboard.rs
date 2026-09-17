use aio_observer_protocol::{CliScope, ObserverSnapshotV1, OBSERVER_HISTORY_LIMIT_MAX};
use aio_tui::{client::{ObserverClient, OfflineReason}, input::handle_logs_key, palette::{with_capability, ColorCapability}, ui::{draw_logs, DashboardView, LogsState}};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, style::{Color, Modifier}, Terminal};
use serde::Serialize;
use std::{sync::Mutex, time::{Duration, Instant}};
use tauri::Manager;
use unicode_width::UnicodeWidthStr;

use crate::{config::Config, AppState};

pub struct Dashboard {
    pub logs: LogsState,
    pub client: Option<ObserverClient>,
    pub generation: u64,
    pub request: u64,
    pub pending: bool,
    pub next_refresh: Instant,
    pub visible: bool,
    pub error: Option<String>,
}

impl Dashboard {
    pub fn new(config: &Config, client: Option<ObserverClient>) -> Self {
        let mut logs = LogsState::new(CliScope::parse(&config.scope).unwrap_or(CliScope::Codex));
        logs.color = true;
        Self { logs, client, generation: 0, request: 0, pending: false, next_refresh: Instant::now(), visible: true, error: None }
    }
    pub fn invalidate(&mut self) {
        self.request = self.request.wrapping_add(1);
        self.pending = false;
        self.next_refresh = Instant::now();
    }
    pub fn reconnect(&mut self, client: ObserverClient) {
        self.generation = self.generation.wrapping_add(1);
        self.invalidate();
        let scope = self.logs.live.scope;
        self.logs = LogsState::new(scope);
        self.logs.color = true;
        self.client = Some(client);
        self.error = None;
    }
    fn accept(&mut self, generation: u64, request: u64, result: Result<ObserverSnapshotV1, OfflineReason>) {
        if self.generation != generation || self.request != request { return; }
        self.pending = false;
        let mut interval = Duration::from_secs(2);
        match result {
            Ok(snapshot) => {
                if snapshot.active_inference_count > 0 || snapshot.active_requests.value.as_ref().is_some_and(|items| !items.is_empty()) { interval = Duration::from_millis(500); }
                self.logs.apply_snapshot(snapshot);
                self.error = None;
            }
            Err(reason) => {
                self.logs.live.set_offline(reason);
                self.error = Some(if reason == OfflineReason::Unauthorized { "认证失败，请更新访问令牌".into() } else { reason.label().into() });
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
    x: u16, y: u16, text: String, width: usize,
    fg: String, bg: Option<String>, bold: bool, italic: bool, underline: bool,
}

fn css_color(color: Color, fallback: &str) -> String {
    match color {
        Color::Rgb(r,g,b) => format!("#{r:02x}{g:02x}{b:02x}"),
        Color::Reset => fallback.into(),
        _ => fallback.into(),
    }
}

pub fn render(logs: &mut LogsState, columns: u16, rows: u16) -> Result<Vec<Cell>, String> {
    let mut terminal = Terminal::new(TestBackend::new(columns, rows)).map_err(|_| "无法创建字符网格")?;
    with_capability(ColorCapability::TrueColor, || terminal.draw(|frame| draw_logs(frame, logs))).map_err(|_| "无法绘制仪表盘")?;
    let buffer = terminal.backend().buffer();
    let mut cells = Vec::new();
    for y in 0..rows {
        let mut x = 0;
        while x < columns {
            let cell = &buffer[(x,y)];
            let width = cell.symbol().width().max(1).min(usize::from(columns - x));
            let reversed = cell.modifier.contains(Modifier::REVERSED);
            let mut fg = css_color(cell.fg, "#cbd1d7");
            let mut bg = if cell.bg == Color::Reset { None } else { Some(css_color(cell.bg, "#353a41")) };
            if reversed { let previous = fg; fg = bg.unwrap_or_else(|| "#272b33".into()); bg = Some(previous); }
            if cell.modifier.contains(Modifier::DIM) {
                let rgb = u32::from_str_radix(&fg[1..], 16).unwrap_or(0x8b949e);
                fg = format!("#{:02x}{:02x}{:02x}", ((rgb>>16)&255)*65/100, ((rgb>>8)&255)*65/100, (rgb&255)*65/100);
            }
            if cell.symbol() != " " || bg.is_some() {
                cells.push(Cell { x, y, text: cell.symbol().into(), width, fg, bg, bold: cell.modifier.contains(Modifier::BOLD), italic: cell.modifier.contains(Modifier::ITALIC), underline: cell.modifier.contains(Modifier::UNDERLINED) });
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
    let dashboard = &mut state.dashboard;
    let now = Instant::now();
    dashboard.logs.expire_inactive_selections(now);
    if dashboard.visible && !dashboard.pending && now >= dashboard.next_refresh {
        if let Some(client) = dashboard.client.clone() {
            dashboard.pending = true;
            let (generation, request, scope, view) = (dashboard.generation, dashboard.request, dashboard.logs.live.scope, dashboard.logs.view);
            let handle = app.clone();
            tauri::async_runtime::spawn(async move {
                let result = if view == DashboardView::Providers { client.snapshot_with_providers(scope, OBSERVER_HISTORY_LIMIT_MAX).await } else { client.snapshot(scope, OBSERVER_HISTORY_LIMIT_MAX).await };
                if let Ok(mut state) = handle.state::<Mutex<AppState>>().lock() { state.dashboard.accept(generation, request, result); }
            });
        }
    }
    let cells = render(&mut state.dashboard.logs, columns, rows)?;
    Ok(Frame { columns, rows, cells, config: state.config.clone(), connected: state.dashboard.client.is_some(), error: state.dashboard.error.clone() })
}

pub fn key_event(key: &str, control: bool) -> Option<KeyEvent> {
    let code = match key {
        "ArrowUp" => KeyCode::Up, "ArrowDown" => KeyCode::Down, "ArrowLeft" => KeyCode::Left, "ArrowRight" => KeyCode::Right,
        "PageUp" => KeyCode::PageUp, "PageDown" => KeyCode::PageDown, "Home" => KeyCode::Home, "End" => KeyCode::End,
        "Enter" => KeyCode::Enter, "Escape" => KeyCode::Esc, "Tab" => KeyCode::Tab,
        value if value.chars().count() == 1 => KeyCode::Char(value.chars().next()?), _ => return None,
    };
    Some(KeyEvent::new(code, if control { KeyModifiers::CONTROL } else { KeyModifiers::NONE }))
}

#[tauri::command]
pub fn float_key(app: tauri::AppHandle, key: String, control: bool) -> Result<(), String> {
    if app.get_webview_window("settings").is_some_and(|w| w.is_visible().unwrap_or(false)) { return Ok(()); }
    let Some(key) = key_event(&key, control) else { return Ok(()); };
    if aio_tui::input::should_quit(key) { app.exit(0); return Ok(()); }
    let state = app.state::<Mutex<AppState>>();
    let mut state = state.lock().map_err(|_| "悬浮窗状态不可用")?;
    let dashboard = &mut state.dashboard;
    let action = handle_logs_key(&mut dashboard.logs, key);
    if action.refresh { dashboard.invalidate(); }
    if let (Some(provider), Some(client)) = (action.probe_provider_id, dashboard.client.clone()) {
        let generation = dashboard.generation;
        let handle = app.clone();
        tauri::async_runtime::spawn(async move {
            let result = client.test_provider_availability(provider).await;
            if let Ok(mut state) = handle.state::<Mutex<AppState>>().lock() {
                if state.dashboard.generation == generation { state.dashboard.logs.finish_provider_probe(provider, result); }
            }
        });
    }
    state.config.scope = state.dashboard.logs.live.scope.as_str().into();
    crate::config::save(&state.path, &state.config)
}

#[cfg(test)]
mod tests {
    use super::*;
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
            assert!(cells.iter().all(|cell| usize::from(cell.x) + cell.width <= usize::from(columns) && cell.y < 12));
        }
    }
}
