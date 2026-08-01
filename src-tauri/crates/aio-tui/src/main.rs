mod args;
mod client;
mod format;
mod terminal;
mod ui;

use aio_observer_protocol::{CliScope, ObserverSnapshotV1, OBSERVER_HISTORY_LIMIT_MAX};
use args::{Mode, ParseOutcome};
use client::ObserverClient;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::io::IsTerminal;
use std::time::{Duration, Instant};
use terminal::TerminalSession;
use ui::{LiveState, LogsState};

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
    match args.mode {
        Mode::Status { once: true } => status_once(&client, args.scope).await,
        Mode::Status { once: false } => {
            require_interactive_terminal()?;
            run_status(client, args.scope).await
        }
        Mode::Logs => {
            require_interactive_terminal()?;
            run_logs(client, args.scope).await
        }
    }
}

fn require_interactive_terminal() -> Result<(), String> {
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        return Err("交互模式需要 TTY；脚本请使用 `aio-tui status --once`".to_string());
    }
    Ok(())
}

async fn status_once(client: &ObserverClient, scope: CliScope) -> Result<(), String> {
    let snapshot = client
        .snapshot(scope, 0)
        .await
        .map_err(|reason| reason.label().to_string())?;
    println!("{}", format::status_plain(&snapshot));
    Ok(())
}

async fn run_status(client: ObserverClient, scope: CliScope) -> Result<(), String> {
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
                .draw(|frame| ui::draw_status(frame, &state))
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

async fn run_logs(client: ObserverClient, scope: CliScope) -> Result<(), String> {
    let mut terminal = TerminalSession::enter().map_err(|error| error.to_string())?;
    let mut state = LogsState::new(scope);
    let mut next_refresh = Instant::now();
    let mut next_clock = Instant::now();
    let mut redraw = true;

    loop {
        let now = Instant::now();
        if now >= next_refresh {
            match client
                .snapshot(state.live.scope, OBSERVER_HISTORY_LIMIT_MAX)
                .await
            {
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
}

fn handle_logs_key(state: &mut LogsState, key: KeyEvent) -> KeyAction {
    if matches!(key.code, KeyCode::Char('?')) {
        state.help = !state.help;
        return KeyAction {
            redraw: true,
            refresh: false,
        };
    }
    if state.help {
        if matches!(key.code, KeyCode::Esc) {
            state.help = false;
            return KeyAction {
                redraw: true,
                refresh: false,
            };
        }
        return KeyAction {
            redraw: false,
            refresh: false,
        };
    }
    if matches!(key.code, KeyCode::Char('r')) {
        return KeyAction {
            redraw: false,
            refresh: true,
        };
    }
    if state.detail {
        match key.code {
            KeyCode::Esc => {
                state.detail = false;
                state.detail_scroll = 0;
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
                }
            }
        }
        return KeyAction {
            redraw: true,
            refresh: false,
        };
    }

    match key.code {
        KeyCode::Up | KeyCode::Char('k') => state.move_selection(-1),
        KeyCode::Down | KeyCode::Char('j') => state.move_selection(1),
        KeyCode::PageUp => state.move_selection(-5),
        KeyCode::PageDown => state.move_selection(5),
        KeyCode::Home => state.selected = 0,
        KeyCode::End => state.selected = state.request_count().saturating_sub(1),
        KeyCode::Enter if state.selected_request().is_some() => {
            state.detail = true;
            state.detail_scroll = 0;
        }
        KeyCode::Tab => {
            state.set_scope(next_scope(state.live.scope));
            return KeyAction {
                redraw: true,
                refresh: true,
            };
        }
        _ => {
            return KeyAction {
                redraw: false,
                refresh: false,
            }
        }
    }
    KeyAction {
        redraw: true,
        refresh: false,
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
}
