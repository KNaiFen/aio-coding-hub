//! Usage: Build the secret-free, frozen provider snapshot shown by the macOS tray panel.

#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

use crate::gateway::observation::is_model_inference_request;
use crate::{
    cli_proxy, gateway_runtime_access, provider_limit_usage, providers, request_logs, settings,
    sort_modes,
};
use serde::Serialize;
use std::collections::HashMap;

const RECENT_REQUEST_SCAN_LIMIT: usize = 200;
pub(crate) const PROVIDER_AVAILABILITY_BUCKETS: usize = 12;

#[derive(Debug, Clone, Copy, Serialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TrayProviderMiniSelectionSource {
    ActiveRequest,
    RecentRequest,
    EnabledCli,
}

#[derive(Debug, Clone, Copy, Serialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TrayProviderMiniUnavailableReason {
    CircuitOpen,
    Cooldown,
    SpendLimit,
    OAuthLimit,
}

#[derive(Debug, Clone, Serialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TrayProviderMiniProvider {
    pub provider_id: i64,
    pub provider_name: String,
    pub unavailable_reasons: Vec<TrayProviderMiniUnavailableReason>,
    pub availability: Vec<crate::domain::provider_availability::ProviderAvailabilityState>,
}

#[derive(Debug, Clone, Serialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TrayProviderMiniSnapshot {
    pub generation: u64,
    pub generated_at_ms: i64,
    pub hours: u32,
    pub cli_key: Option<String>,
    pub selection_source: Option<TrayProviderMiniSelectionSource>,
    pub route_name: Option<String>,
    pub providers: Vec<TrayProviderMiniProvider>,
    pub unavailable: bool,
}

impl TrayProviderMiniSnapshot {
    pub(crate) fn unavailable(generation: u64) -> Self {
        Self {
            generation,
            generated_at_ms: crate::shared::time::now_unix_millis(),
            hours: settings::DEFAULT_PROVIDER_AVAILABILITY_HOURS,
            cli_key: None,
            selection_source: None,
            route_name: None,
            providers: Vec::new(),
            unavailable: true,
        }
    }
}

fn is_terminal(row: &request_logs::RequestLogSummary) -> bool {
    row.status.is_some() || row.error_code.is_some() || row.is_interrupted
}

fn terminal_completed_at_ms(created_at_ms: i64, duration_ms: i64) -> i64 {
    created_at_ms.saturating_add(duration_ms.max(0))
}

fn latest_inference_cli<'a>(
    requests: impl IntoIterator<Item = (&'a str, &'a str, &'a str, i64)>,
) -> Option<&'a str> {
    requests
        .into_iter()
        .filter(|(cli_key, method, path, _)| is_model_inference_request(cli_key, method, path))
        .max_by_key(|(_, _, _, created_at_ms)| *created_at_ms)
        .map(|(cli_key, _, _, _)| cli_key)
}

fn choose_cli_key(
    active_cli_key: Option<&str>,
    recent_cli_key: Option<&str>,
    priority_order: &[String],
    mut is_enabled: impl FnMut(&str) -> bool,
) -> Option<(String, TrayProviderMiniSelectionSource)> {
    if let Some(cli_key) = active_cli_key {
        return Some((
            cli_key.to_string(),
            TrayProviderMiniSelectionSource::ActiveRequest,
        ));
    }
    if let Some(cli_key) = recent_cli_key {
        return Some((
            cli_key.to_string(),
            TrayProviderMiniSelectionSource::RecentRequest,
        ));
    }
    priority_order
        .iter()
        .find(|cli_key| is_enabled(cli_key))
        .map(|cli_key| (cli_key.clone(), TrayProviderMiniSelectionSource::EnabledCli))
}

fn bounded_text(value: &str, max_chars: usize) -> String {
    value.trim().chars().take(max_chars).collect()
}

fn current_route_name(db: &crate::db::Db, mode_id: Option<i64>) -> String {
    let Some(mode_id) = mode_id else {
        return "默认".to_string();
    };
    sort_modes::list_modes(db)
        .ok()
        .and_then(|modes| modes.into_iter().find(|mode| mode.id == mode_id))
        .map(|mode| bounded_text(&mode.name, 32))
        .unwrap_or_else(|| "自定义路由".to_string())
}

fn no_data_availability() -> Vec<crate::domain::provider_availability::ProviderAvailabilityState> {
    vec![
        crate::domain::provider_availability::ProviderAvailabilityState::NoData;
        PROVIDER_AVAILABILITY_BUCKETS
    ]
}

pub(crate) fn build_snapshot<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &crate::db::Db,
    generation: u64,
) -> crate::shared::error::AppResult<TrayProviderMiniSnapshot> {
    let now_ms = crate::shared::time::now_unix_millis();
    let now_unix = now_ms / 1_000;
    let app_settings = settings::read(app).unwrap_or_default();
    let hours = crate::domain::provider_availability::normalized_availability_hours(
        app_settings.provider_availability_hours,
    );
    let active = gateway_runtime_access::app_gateway_active_requests_snapshot(app);
    let active_cli_key = latest_inference_cli(active.iter().map(|request| {
        (
            request.cli_key.as_str(),
            request.method.as_str(),
            request.path.as_str(),
            request.created_at_ms,
        )
    }));
    let recent =
        request_logs::list_recent_all(db, RECENT_REQUEST_SCAN_LIMIT).unwrap_or_else(|error| {
            tracing::warn!(
                error = %error.code(),
                "tray provider mini could not read recent requests"
            );
            Vec::new()
        });
    let recent_cli_key =
        latest_inference_cli(recent.iter().filter(|row| is_terminal(row)).map(|row| {
            (
                row.cli_key.as_str(),
                row.method.as_str(),
                row.path.as_str(),
                terminal_completed_at_ms(row.created_at_ms, row.duration_ms),
            )
        }));
    let selected = choose_cli_key(
        active_cli_key,
        recent_cli_key,
        &app_settings.cli_priority_order,
        |cli_key| cli_proxy::is_enabled(app, cli_key).unwrap_or(false),
    );
    let Some((cli_key, selection_source)) = selected else {
        return Ok(TrayProviderMiniSnapshot {
            generation,
            generated_at_ms: now_ms,
            hours,
            cli_key: None,
            selection_source: None,
            route_name: None,
            providers: Vec::new(),
            unavailable: false,
        });
    };

    let selection = providers::list_enabled_for_gateway_using_active_mode(db, &cli_key)?;
    let route_name = current_route_name(db, selection.sort_mode_id);
    let provider_ids = selection
        .providers
        .iter()
        .map(|provider| provider.id)
        .collect::<Vec<_>>();
    let spend_limited = provider_limit_usage::list_v1(db, Some(&cli_key))
        .unwrap_or_default()
        .into_iter()
        .map(|row| (row.provider_id, row.is_limit_reached()))
        .collect::<HashMap<_, _>>();
    let oauth_provider_ids = selection
        .providers
        .iter()
        .filter(|provider| provider.auth_mode == "oauth")
        .map(|provider| provider.id)
        .collect::<Vec<_>>();
    let oauth_limited = crate::domain::provider_oauth_limits::list_display_snapshots(
        db,
        &oauth_provider_ids,
        now_unix,
    )
    .unwrap_or_default()
    .into_iter()
    .map(|snapshot| (snapshot.provider_id, snapshot.limited))
    .collect::<HashMap<_, _>>();
    let circuit_status =
        gateway_runtime_access::app_gateway_circuit_status_peek(app, &provider_ids, now_unix)
            .into_iter()
            .map(|status| (status.provider_id, status))
            .collect::<HashMap<_, _>>();
    let availability = crate::domain::provider_availability::timelines(
        db,
        &provider_ids,
        hours,
        crate::domain::provider_availability::TUI_PROVIDER_AVAILABILITY_BUCKETS,
        now_ms,
    )
    .unwrap_or_else(|error| {
        tracing::warn!(
            error = %error.code(),
            "tray provider mini availability is unavailable"
        );
        Vec::new()
    })
    .into_iter()
    .map(|timeline| {
        (
            timeline.provider_id,
            timeline
                .buckets
                .into_iter()
                .map(|bucket| bucket.state)
                .collect::<Vec<_>>(),
        )
    })
    .collect::<HashMap<_, _>>();

    let providers = selection
        .providers
        .into_iter()
        .map(|provider| {
            let mut unavailable_reasons = Vec::new();
            if spend_limited.get(&provider.id).copied().unwrap_or(false) {
                unavailable_reasons.push(TrayProviderMiniUnavailableReason::SpendLimit);
            }
            if oauth_limited.get(&provider.id).copied().unwrap_or(false) {
                unavailable_reasons.push(TrayProviderMiniUnavailableReason::OAuthLimit);
            }
            if let Some(status) = circuit_status.get(&provider.id) {
                if status.cooldown_until.is_some_and(|until| until > now_unix) {
                    unavailable_reasons.push(TrayProviderMiniUnavailableReason::Cooldown);
                }
                if status.state == "OPEN" {
                    unavailable_reasons.push(TrayProviderMiniUnavailableReason::CircuitOpen);
                }
            }
            let availability = availability
                .get(&provider.id)
                .filter(|states| states.len() == PROVIDER_AVAILABILITY_BUCKETS)
                .cloned()
                .unwrap_or_else(no_data_availability);
            TrayProviderMiniProvider {
                provider_id: provider.id,
                provider_name: bounded_text(&provider.name, 128),
                unavailable_reasons,
                availability,
            }
        })
        .collect();

    Ok(TrayProviderMiniSnapshot {
        generation,
        generated_at_ms: now_ms,
        hours,
        cli_key: Some(cli_key),
        selection_source: Some(selection_source),
        route_name: Some(route_name),
        providers,
        unavailable: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latest_inference_cli_uses_the_newest_started_request() {
        let selected = latest_inference_cli([
            ("claude", "POST", "/v1/messages", 100),
            ("codex", "POST", "/v1/responses", 300),
            ("gemini", "GET", "/v1beta/models", 500),
        ]);

        assert_eq!(selected, Some("codex"));
    }

    #[test]
    fn cli_selection_prefers_active_then_recent_then_enabled_order() {
        let order = vec!["gemini".to_string(), "codex".to_string()];

        assert_eq!(
            choose_cli_key(Some("claude"), Some("codex"), &order, |_| true),
            Some((
                "claude".to_string(),
                TrayProviderMiniSelectionSource::ActiveRequest
            ))
        );
        assert_eq!(
            choose_cli_key(None, Some("codex"), &order, |_| true),
            Some((
                "codex".to_string(),
                TrayProviderMiniSelectionSource::RecentRequest
            ))
        );
        assert_eq!(
            choose_cli_key(None, None, &order, |cli_key| cli_key == "codex"),
            Some((
                "codex".to_string(),
                TrayProviderMiniSelectionSource::EnabledCli
            ))
        );
    }

    #[test]
    fn recent_inference_cli_uses_completion_time_instead_of_start_time() {
        let selected = latest_inference_cli([
            (
                "claude",
                "POST",
                "/v1/messages",
                terminal_completed_at_ms(100, 500),
            ),
            (
                "codex",
                "POST",
                "/v1/responses",
                terminal_completed_at_ms(300, 10),
            ),
        ]);

        assert_eq!(selected, Some("claude"));
    }

    #[test]
    fn no_data_timeline_always_has_twelve_cells() {
        assert_eq!(
            no_data_availability(),
            vec![crate::domain::provider_availability::ProviderAvailabilityState::NoData; 12]
        );
    }
}
