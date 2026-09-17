use crate::ui::{DashboardView, LogsState};
use aio_observer_protocol::CliScope;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::time::Instant;

pub struct KeyAction {
    pub redraw: bool,
    pub refresh: bool,
    pub probe_provider_id: Option<i64>,
}

pub fn handle_logs_key(state: &mut LogsState, key: KeyEvent) -> KeyAction {
    handle_logs_key_at(state, key, Instant::now())
}

pub fn handle_logs_key_at(state: &mut LogsState, key: KeyEvent, now: Instant) -> KeyAction {
    if matches!(key.code, KeyCode::Char('?')) {
        state.help = !state.help;
        return KeyAction { redraw: true, refresh: false, probe_provider_id: None };
    }
    if state.help {
        let redraw = matches!(key.code, KeyCode::Esc);
        if redraw { state.help = false; }
        return KeyAction { redraw, refresh: false, probe_provider_id: None };
    }
    if matches!(key.code, KeyCode::Char('r')) {
        return KeyAction { redraw: false, refresh: true, probe_provider_id: None };
    }
    if state.detail {
        if matches!(key.code, KeyCode::Char('t')) && state.view == DashboardView::Providers {
            let provider_id = state.begin_provider_probe();
            return KeyAction { redraw: provider_id.is_some(), refresh: false, probe_provider_id: provider_id };
        }
        match key.code {
            KeyCode::Esc => {
                state.detail = false;
                state.detail_scroll = 0;
                state.resume_current_selection_expiry(now);
            }
            KeyCode::Up | KeyCode::Char('k') => state.detail_scroll = state.detail_scroll.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => state.detail_scroll = state.detail_scroll.saturating_add(1),
            KeyCode::PageUp => state.detail_scroll = state.detail_scroll.saturating_sub(8),
            KeyCode::PageDown => state.detail_scroll = state.detail_scroll.saturating_add(8),
            KeyCode::Home => state.detail_scroll = 0,
            _ => return KeyAction { redraw: false, refresh: false, probe_provider_id: None },
        }
        return KeyAction { redraw: true, refresh: false, probe_provider_id: None };
    }
    match key.code {
        KeyCode::Left | KeyCode::Right => {
            state.switch_view(if key.code == KeyCode::Left { DashboardView::Requests } else { DashboardView::Providers });
            state.resume_current_selection_expiry(now);
            return KeyAction { redraw: true, refresh: true, probe_provider_id: None };
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
            return KeyAction { redraw: true, refresh: true, probe_provider_id: None };
        }
        _ => return KeyAction { redraw: false, refresh: false, probe_provider_id: None },
    }
    KeyAction { redraw: true, refresh: false, probe_provider_id: None }
}

pub fn should_quit(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('q'))
        || (matches!(key.code, KeyCode::Char('c')) && key.modifiers.contains(KeyModifiers::CONTROL))
}

pub fn next_scope(scope: CliScope) -> CliScope {
    let index = CliScope::VALUES.iter().position(|candidate| *candidate == scope).unwrap_or(0);
    CliScope::VALUES[(index + 1) % CliScope::VALUES.len()]
}
