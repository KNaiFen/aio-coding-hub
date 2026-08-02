mod args;
mod client;
mod config;
mod format;
mod palette;
mod terminal;
mod ui;

use aio_observer_protocol::{
    CliScope, ObserverProviderAvailabilityTestResult, ObserverSnapshotV1,
    OBSERVER_HISTORY_LIMIT_MAX,
};
use args::{Mode, ParseOutcome};
use client::{ObserverClient, OfflineReason};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::io::IsTerminal;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use terminal::TerminalSession;
use ui::{DashboardView, LiveState, LogsState, StatuslinePickerState};

const ACTIVE_REFRESH_INTERVAL: Duration = Duration::from_millis(500);
const IDLE_REFRESH_INTERVAL: Duration = Duration::from_secs(2);
const CLOCK_REFRESH_INTERVAL: Duration = Duration::from_secs(1);
const EVENT_POLL_INTERVAL: Duration = Duration::from_millis(100);

#[tokio::main]
async fn main() {
    if let Err(message) = run().await {
        eprintln!("aio-tui: {message}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let args = match args::parse()? {
        ParseOutcome::Help => {
            print!("{}", args::help());
            return Ok(());
        }
        ParseOutcome::Version => {
            println!("aio-tui {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        ParseOutcome::Run(args) => args,
    };
    let client = ObserverClient::new().map_err(|reason| reason.label().to_string())?;
    let scope = args.scope;
    match args.mode {
        Mode::Status { once, items } => {
            let persisted = config::load();
            let items = items.unwrap_or(persisted.status_items);
            if once {
                status_once(&client, scope, &items).await
            } else {
                require_interactive_terminal()?;
                run_status(
                    client,
                    scope,
                    items,
                    config::colors_enabled(persisted.use_colors),
                )
                .await
            }
        }
        Mode::Statusline => {
            require_interactive_terminal()?;
            run_statusline(client, scope).await
        }
        Mode::Logs => {
            require_interactive_terminal()?;
            run_logs(client, scope).await
        }
    }
}

fn require_interactive_terminal() -> Result<(), String> {
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        return Err("交互模式需要 TTY；脚本请使用 `aio-tui status --once`".to_string());
    }
    Ok(())
}

async fn status_once(
    client: &ObserverClient,
    scope: CliScope,
    items: &[config::StatusItem],
) -> Result<(), String> {
    let snapshot = client
        .snapshot(scope, 0)
        .await
        .map_err(|reason| reason.label().to_string())?;
    println!("{}", format::status_plain(&snapshot, items));
    Ok(())
}

async fn run_status(
    client: ObserverClient,
    scope: CliScope,
    items: Vec<config::StatusItem>,
    color: bool,
) -> Result<(), String> {
    let mut terminal = TerminalSession::enter().map_err(|error| error.to_string())?;
    let mut state = LiveState::new(scope);
    let mut next_refresh = Instant::now();
    let mut next_clock = Instant::now();
    let mut redraw = true;

    loop {
        let now = Instant::now();
        if now >= next_refresh {
            match client.snapshot(scope, 0).await {
                Ok(snapshot) => {
                    let interval = refresh_interval(&snapshot);
                    state.apply_snapshot(snapshot);
                    next_refresh = Instant::now() + interval;
                }
                Err(reason) => {
                    state.set_offline(reason);
                    next_refresh = Instant::now() + IDLE_REFRESH_INTERVAL;
                }
            }
            redraw = true;
        }
        if Instant::now() >= next_clock {
            next_clock = Instant::now() + CLOCK_REFRESH_INTERVAL;
            redraw = true;
        }
        if redraw {
            terminal
                .terminal_mut()
                .draw(|frame| ui::draw_status(frame, &state, &items, color))
                .map_err(|error| error.to_string())?;
            redraw = false;
        }
        if event::poll(EVENT_POLL_INTERVAL).map_err(|error| error.to_string())? {
            match event::read().map_err(|error| error.to_string())? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if should_quit(key) {
                        break;
                    }
                    if matches!(key.code, KeyCode::Char('r')) {
                        next_refresh = Instant::now();
                    }
                }
                Event::Resize(_, _) => redraw = true,
                _ => {}
            }
        }
    }
    Ok(())
}

async fn run_statusline(client: ObserverClient, scope: CliScope) -> Result<(), String> {
    let mut terminal = TerminalSession::enter().map_err(|error| error.to_string())?;
    let mut picker = StatuslinePickerState::new(config::load());
    let mut live = LiveState::new(scope);
    let mut next_refresh = Instant::now();
    let mut next_clock = Instant::now();
    let mut redraw = true;
    let mut saved = false;

    loop {
        let now = Instant::now();
        if now >= next_refresh {
            match client.snapshot(scope, 0).await {
                Ok(snapshot) => {
                    let interval = refresh_interval(&snapshot);
                    live.apply_snapshot(snapshot);
                    next_refresh = Instant::now() + interval;
                }
                Err(reason) => {
                    live.set_offline(reason);
                    next_refresh = Instant::now() + IDLE_REFRESH_INTERVAL;
                }
            }
            redraw = true;
        }
        if Instant::now() >= next_clock {
            next_clock = Instant::now() + CLOCK_REFRESH_INTERVAL;
            redraw = true;
        }
        if redraw {
            terminal
                .terminal_mut()
                .draw(|frame| ui::draw_statusline_picker(frame, &mut picker, &live))
                .map_err(|error| error.to_string())?;
            redraw = false;
        }
        if event::poll(EVENT_POLL_INTERVAL).map_err(|error| error.to_string())? {
            match event::read().map_err(|error| error.to_string())? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if should_quit(key) || matches!(key.code, KeyCode::Esc) {
                        break;
                    }
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') => picker.move_selection(-1),
                        KeyCode::Down | KeyCode::Char('j') => picker.move_selection(1),
                        KeyCode::Left => picker.move_selected_item(-1),
                        KeyCode::Right => picker.move_selected_item(1),
                        KeyCode::Char(' ') => picker.toggle_selected(),
                        KeyCode::Char('c') => picker.toggle_colors(),
                        KeyCode::Char('r') => picker.reset(),
                        KeyCode::Home => {
                            picker.selected = 0;
                        }
                        KeyCode::End => {
                            picker.selected = config::StatusItem::ALL.len().saturating_sub(1);
                        }
                        KeyCode::Enter => {
                            config::save(&picker.config())?;
                            saved = true;
                            break;
                        }
                        _ => continue,
                    }
                    redraw = true;
                }
                Event::Resize(_, _) => redraw = true,
                _ => {}
            }
        }
    }
    drop(terminal);
    if saved {
        println!("状态栏配置已保存");
    }
    Ok(())
}

async fn run_logs(client: ObserverClient, scope: CliScope) -> Result<(), String> {
    let mut terminal = TerminalSession::enter().map_err(|error| error.to_string())?;
    let mut state = LogsState::new(scope);
    let (provider_probe_tx, provider_probe_rx) = mpsc::channel::<(
        i64,
        Result<ObserverProviderAvailabilityTestResult, OfflineReason>,
    )>();
    let mut next_refresh = Instant::now();
    let mut next_clock = Instant::now();
    let mut redraw = true;

    loop {
        while let Ok((provider_id, result)) = provider_probe_rx.try_recv() {
            state.finish_provider_probe(provider_id, result);
            redraw = true;
        }
        let now = Instant::now();
        if state.expire_inactive_selections(now) {
            redraw = true;
        }
        if now >= next_refresh {
            let snapshot = match state.view {
                DashboardView::Requests => {
                    client
                        .snapshot(state.live.scope, OBSERVER_HISTORY_LIMIT_MAX)
                        .await
                }
                DashboardView::Providers => {
                    client
                        .snapshot_with_providers(state.live.scope, OBSERVER_HISTORY_LIMIT_MAX)
                        .await
                }
            };
            match snapshot {
                Ok(snapshot) => {
                    let interval = refresh_interval(&snapshot);
                    state.apply_snapshot(snapshot);
                    next_refresh = Instant::now() + interval;
                }
                Err(reason) => {
                    state.live.set_offline(reason);
                    next_refresh = Instant::now() + IDLE_REFRESH_INTERVAL;
                }
            }
            redraw = true;
        }
        if Instant::now() >= next_clock {
            next_clock = Instant::now() + CLOCK_REFRESH_INTERVAL;
            redraw = true;
        }
        if redraw {
            terminal
                .terminal_mut()
                .draw(|frame| ui::draw_logs(frame, &mut state))
                .map_err(|error| error.to_string())?;
            redraw = false;
        }
        if event::poll(EVENT_POLL_INTERVAL).map_err(|error| error.to_string())? {
            match event::read().map_err(|error| error.to_string())? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if should_quit(key) {
                        break;
                    }
                    let action = handle_logs_key(&mut state, key);
                    if action.refresh {
                        next_refresh = Instant::now();
                    }
                    if let Some(provider_id) = action.probe_provider_id {
                        let probe_client = client.clone();
                        let probe_tx = provider_probe_tx.clone();
                        tokio::spawn(async move {
                            let result = probe_client.test_provider_availability(provider_id).await;
                            let _ = probe_tx.send((provider_id, result));
                        });
                    }
                    redraw |= action.redraw;
                }
                Event::Resize(_, _) => redraw = true,
                _ => {}
            }
        }
    }
    Ok(())
}

struct KeyAction {
    redraw: bool,
    refresh: bool,
    probe_provider_id: Option<i64>,
}

fn handle_logs_key(state: &mut LogsState, key: KeyEvent) -> KeyAction {
    handle_logs_key_at(state, key, Instant::now())
}

fn handle_logs_key_at(state: &mut LogsState, key: KeyEvent, now: Instant) -> KeyAction {
    if matches!(key.code, KeyCode::Char('?')) {
        state.help = !state.help;
        return KeyAction {
            redraw: true,
            refresh: false,
            probe_provider_id: None,
        };
    }
    if state.help {
        if matches!(key.code, KeyCode::Esc) {
            state.help = false;
            return KeyAction {
                redraw: true,
                refresh: false,
                probe_provider_id: None,
            };
        }
        return KeyAction {
            redraw: false,
            refresh: false,
            probe_provider_id: None,
        };
    }
    if matches!(key.code, KeyCode::Char('r')) {
        return KeyAction {
            redraw: false,
            refresh: true,
            probe_provider_id: None,
        };
    }
    if state.detail {
        if matches!(key.code, KeyCode::Char('t')) && state.view == DashboardView::Providers {
            let provider_id = state.begin_provider_probe();
            return KeyAction {
                redraw: provider_id.is_some(),
                refresh: false,
                probe_provider_id: provider_id,
            };
        }
        match key.code {
            KeyCode::Esc => {
                state.detail = false;
                state.detail_scroll = 0;
                state.resume_current_selection_expiry(now);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                state.detail_scroll = state.detail_scroll.saturating_sub(1)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                state.detail_scroll = state.detail_scroll.saturating_add(1)
            }
            KeyCode::PageUp => state.detail_scroll = state.detail_scroll.saturating_sub(8),
            KeyCode::PageDown => state.detail_scroll = state.detail_scroll.saturating_add(8),
            KeyCode::Home => state.detail_scroll = 0,
            _ => {
                return KeyAction {
                    redraw: false,
                    refresh: false,
                    probe_provider_id: None,
                }
            }
        }
        return KeyAction {
            redraw: true,
            refresh: false,
            probe_provider_id: None,
        };
    }

    match key.code {
        KeyCode::Left => {
            state.switch_view(DashboardView::Requests);
            state.resume_current_selection_expiry(now);
            return KeyAction {
                redraw: true,
                refresh: true,
                probe_provider_id: None,
            };
        }
        KeyCode::Right => {
            state.switch_view(DashboardView::Providers);
            state.resume_current_selection_expiry(now);
            return KeyAction {
                redraw: true,
                refresh: true,
                probe_provider_id: None,
            };
        }
        KeyCode::Up | KeyCode::Char('k') => state.move_selection(-1, now),
        KeyCode::Down | KeyCode::Char('j') => state.move_selection(1, now),
        KeyCode::PageUp => state.move_selection(-5, now),
        KeyCode::PageDown => state.move_selection(5, now),
        KeyCode::Home => state.select_current(0, now),
        KeyCode::End => state.select_current(state.current_count().saturating_sub(1), now),
        KeyCode::Enter if state.has_selected_item() => {
            state.detail = true;
            state.detail_scroll = 0;
            state.suspend_current_selection_expiry();
        }
        KeyCode::Tab => {
            state.set_scope(next_scope(state.live.scope));
            return KeyAction {
                redraw: true,
                refresh: true,
                probe_provider_id: None,
            };
        }
        _ => {
            return KeyAction {
                redraw: false,
                refresh: false,
                probe_provider_id: None,
            }
        }
    }
    KeyAction {
        redraw: true,
        refresh: false,
        probe_provider_id: None,
    }
}

fn should_quit(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('q'))
        || (matches!(key.code, KeyCode::Char('c')) && key.modifiers.contains(KeyModifiers::CONTROL))
}

fn refresh_interval(snapshot: &ObserverSnapshotV1) -> Duration {
    let active = snapshot.active_inference_count > 0
        || snapshot
            .active_requests
            .value
            .as_ref()
            .is_some_and(|items| !items.is_empty());
    if active {
        ACTIVE_REFRESH_INTERVAL
    } else {
        IDLE_REFRESH_INTERVAL
    }
}

fn next_scope(scope: CliScope) -> CliScope {
    let index = CliScope::VALUES
        .iter()
        .position(|candidate| *candidate == scope)
        .unwrap_or(0);
    CliScope::VALUES[(index + 1) % CliScope::VALUES.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_cycle_is_stable_and_returns_to_codex() {
        let mut scope = CliScope::Codex;
        for _ in 0..CliScope::VALUES.len() {
            scope = next_scope(scope);
        }
        assert_eq!(scope, CliScope::Codex);
    }

    #[test]
    fn arrow_keys_switch_dashboard_views_and_request_refresh() {
        let mut state = LogsState::new(CliScope::Codex);
        let right = handle_logs_key(
            &mut state,
            KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
        );
        assert_eq!(state.view, DashboardView::Providers);
        assert!(right.refresh);
        assert!(state.providers_pending);

        let left = handle_logs_key(&mut state, KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        assert_eq!(state.view, DashboardView::Requests);
        assert!(left.refresh);
    }
}
