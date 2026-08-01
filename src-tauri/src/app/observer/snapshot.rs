//! Build bounded, secret-free observer snapshots from existing read models.

use crate::gateway::active_requests::ActiveRequestSnapshotItem;
use crate::gateway::observation::is_model_inference_request;
use crate::{blocking, cli_sessions, gateway_runtime_access, providers, request_logs, usage_stats};
use aio_observer_protocol::{
    CliScope, ObserverContextCompaction, ObserverDominantProvider, ObserverGatewayStatus,
    ObserverPreferredProvider, ObserverRequest, ObserverRequestState, ObserverRequestUsage,
    ObserverRouteHop, ObserverSection, ObserverSnapshotV1, ObserverTodayUsage,
    OBSERVER_PROTOCOL_VERSION,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use tokio::sync::OwnedSemaphorePermit;

const HISTORY_SCAN_LIMIT: usize = 500;
const ACTIVE_REQUEST_LIMIT: usize = 200;
const ROUTE_HOP_LIMIT: usize = 20;
const DB_SNAPSHOT_TIMEOUT: Duration = Duration::from_millis(1500);
const SPECIAL_SETTINGS_MAX_BYTES: usize = 32 * 1024;

type FolderKey = (String, String);

struct ProviderCandidate {
    id: i64,
    name: String,
}

struct DbProjection {
    logs_available: bool,
    rows: Vec<request_logs::RequestLogSummary>,
    terminal_trace_ids: HashSet<String>,
    folders: HashMap<FolderKey, String>,
    provider_available: bool,
    provider_cli_key: Option<String>,
    provider_candidates: Vec<ProviderCandidate>,
    today: Option<ObserverTodayUsage>,
}

impl DbProjection {
    fn unavailable() -> Self {
        Self {
            logs_available: false,
            rows: Vec::new(),
            terminal_trace_ids: HashSet::new(),
            folders: HashMap::new(),
            provider_available: false,
            provider_cli_key: None,
            provider_candidates: Vec::new(),
            today: None,
        }
    }
}

pub(super) async fn build_snapshot(
    app: &tauri::AppHandle,
    db: Option<&crate::db::Db>,
    db_query_permit: Option<OwnedSemaphorePermit>,
    scope: CliScope,
    history_limit: usize,
) -> ObserverSnapshotV1 {
    let generated_at_ms = crate::shared::time::now_unix_millis();
    let gateway_status = gateway_runtime_access::app_gateway_status(app);
    let raw_active = gateway_runtime_access::app_gateway_active_requests_snapshot(app);

    let db_projection = load_db_projection(app, db, db_query_permit, scope, &raw_active)
        .await
        .unwrap_or_else(DbProjection::unavailable);
    let active = raw_active
        .into_iter()
        .filter(|item| !db_projection.terminal_trace_ids.contains(&item.trace_id))
        .collect::<Vec<_>>();
    let active_inference_count = active
        .iter()
        .filter(|item| is_model_inference_request(&item.cli_key, &item.method, &item.path))
        .count();
    let active_requests = active
        .iter()
        .filter(|item| scope.matches(&item.cli_key))
        .take(ACTIVE_REQUEST_LIMIT)
        .map(|item| project_active(item, &db_projection.folders, generated_at_ms))
        .collect::<Vec<_>>();

    let filtered_rows = db_projection
        .rows
        .iter()
        .filter(|row| scope.matches(&row.cli_key))
        .collect::<Vec<_>>();
    let terminal_inference = filtered_rows
        .iter()
        .copied()
        .filter(|row| is_terminal(row))
        .filter(|row| is_model_inference_request(&row.cli_key, &row.method, &row.path))
        .collect::<Vec<_>>();

    let last_request = if db_projection.logs_available {
        terminal_inference
            .first()
            .map(|row| project_terminal(row, &db_projection.folders))
            .map(ObserverSection::ready)
            .unwrap_or_else(ObserverSection::empty)
    } else {
        ObserverSection::unavailable()
    };
    let dominant_provider = if db_projection.logs_available {
        dominant_provider(&terminal_inference)
            .map(ObserverSection::ready)
            .unwrap_or_else(ObserverSection::empty)
    } else {
        ObserverSection::unavailable()
    };
    let recent_requests = if db_projection.logs_available {
        ObserverSection::ready(
            filtered_rows
                .into_iter()
                .filter(|row| is_terminal(row))
                .filter(|row| !active.iter().any(|item| item.trace_id == row.trace_id))
                .take(history_limit)
                .map(|row| project_terminal(row, &db_projection.folders))
                .collect(),
        )
    } else {
        ObserverSection::unavailable()
    };

    let preferred_provider = preferred_provider(
        app,
        gateway_status.running,
        generated_at_ms / 1000,
        &db_projection,
    );

    ObserverSnapshotV1 {
        protocol_version: OBSERVER_PROTOCOL_VERSION,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        generated_at_ms,
        scope,
        gateway: ObserverGatewayStatus {
            running: gateway_status.running,
            port: gateway_status.port,
        },
        preferred_provider,
        last_request,
        dominant_provider,
        active_inference_count,
        today: db_projection
            .today
            .map(ObserverSection::ready)
            .unwrap_or_else(ObserverSection::unavailable),
        active_requests: ObserverSection::ready(active_requests),
        recent_requests,
    }
}

async fn load_db_projection(
    app: &tauri::AppHandle,
    db: Option<&crate::db::Db>,
    db_query_permit: Option<OwnedSemaphorePermit>,
    scope: CliScope,
    active: &[ActiveRequestSnapshotItem],
) -> Option<DbProjection> {
    let db = db?.clone();
    let db_query_permit = db_query_permit?;
    let app = app.clone();
    let active = active.to_vec();
    tokio::time::timeout(
        DB_SNAPSHOT_TIMEOUT,
        blocking::run("observer_snapshot", move || {
            let _db_query_permit = db_query_permit;
            Ok::<_, crate::shared::error::AppError>(build_db_projection(&app, &db, scope, &active))
        }),
    )
    .await
    .ok()?
    .ok()
}

fn build_db_projection(
    app: &tauri::AppHandle,
    db: &crate::db::Db,
    scope: CliScope,
    active: &[ActiveRequestSnapshotItem],
) -> DbProjection {
    let rows_result = if scope == CliScope::All {
        request_logs::list_recent_all(db, HISTORY_SCAN_LIMIT)
    } else {
        request_logs::list_recent(db, scope.as_str(), HISTORY_SCAN_LIMIT)
    };
    let logs_available = rows_result.is_ok();
    let rows = rows_result.unwrap_or_default();
    let terminal_trace_ids = rows
        .iter()
        .filter(|row| is_terminal(row))
        .map(|row| row.trace_id.clone())
        .collect::<HashSet<_>>();
    let folders = resolve_folders(app, active, &rows);

    let provider_cli_key = preferred_cli_key(scope, &rows);
    let provider_result = provider_cli_key.as_deref().map(|cli_key| {
        providers::list_enabled_gateway_provider_identities_using_active_mode(db, cli_key).map(
            |providers| {
                providers
                    .into_iter()
                    .map(|provider| ProviderCandidate {
                        id: provider.id,
                        name: bounded_text(&provider.name, 128),
                    })
                    .collect::<Vec<_>>()
            },
        )
    });
    let provider_available = provider_result.as_ref().is_none_or(|result| result.is_ok());
    let provider_candidates = provider_result.and_then(Result::ok).unwrap_or_default();

    let today = today_usage(db);
    DbProjection {
        logs_available,
        rows,
        terminal_trace_ids,
        folders,
        provider_available,
        provider_cli_key,
        provider_candidates,
        today,
    }
}

fn preferred_cli_key(scope: CliScope, rows: &[request_logs::RequestLogSummary]) -> Option<String> {
    if scope != CliScope::All {
        return Some(scope.as_str().to_string());
    }
    rows.iter()
        .find(|row| {
            is_terminal(row) && is_model_inference_request(&row.cli_key, &row.method, &row.path)
        })
        .map(|row| row.cli_key.clone())
}

fn today_usage(db: &crate::db::Db) -> Option<ObserverTodayUsage> {
    let summary = usage_stats::summary(db, "today", None).ok()?;
    let params = usage_stats::UsageQueryParams {
        period: "daily".to_string(),
        start_ts: None,
        end_ts: None,
        cli_key: None,
        provider_id: None,
        folder_keys: None,
        day_start_hour: None,
        exclude_cx2cc_gateway_bridge: None,
    };
    let rows = usage_stats::leaderboard_v2(db, "cli", &params, None, |_| Vec::new()).ok()?;
    let mut covered = false;
    let mut cost_usd = 0.0_f64;
    for value in rows.into_iter().filter_map(|row| row.cost_usd) {
        if value.is_finite() && value >= 0.0 {
            covered = true;
            cost_usd += value;
        }
    }
    Some(ObserverTodayUsage {
        total_tokens: summary.total_tokens.max(0),
        cost_usd: covered.then_some(cost_usd),
    })
}

fn preferred_provider(
    app: &tauri::AppHandle,
    gateway_running: bool,
    now_unix: i64,
    projection: &DbProjection,
) -> ObserverSection<ObserverPreferredProvider> {
    if !projection.provider_available {
        return ObserverSection::unavailable();
    }
    if !gateway_running {
        return ObserverSection::empty();
    }
    let Some(cli_key) = projection.provider_cli_key.as_deref() else {
        return ObserverSection::empty();
    };
    let provider_ids = projection
        .provider_candidates
        .iter()
        .map(|provider| provider.id)
        .collect::<Vec<_>>();
    let statuses =
        gateway_runtime_access::app_gateway_circuit_status_peek(app, &provider_ids, now_unix);
    let by_id = statuses
        .into_iter()
        .map(|status| (status.provider_id, status))
        .collect::<HashMap<_, _>>();
    projection
        .provider_candidates
        .iter()
        .find_map(|provider| {
            let status = by_id.get(&provider.id)?;
            let cooldown_active = status.cooldown_until.is_some_and(|until| until > now_unix);
            (status.state != "OPEN" && !cooldown_active).then(|| ObserverPreferredProvider {
                cli_key: cli_key.to_string(),
                provider_name: provider.name.clone(),
                circuit_state: status.state.clone(),
            })
        })
        .map(ObserverSection::ready)
        .unwrap_or_else(ObserverSection::empty)
}

fn resolve_folders(
    app: &tauri::AppHandle,
    active: &[ActiveRequestSnapshotItem],
    rows: &[request_logs::RequestLogSummary],
) -> HashMap<FolderKey, String> {
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    for (cli_key, session_id) in active
        .iter()
        .filter_map(|item| item.session_id.as_ref().map(|id| (&item.cli_key, id)))
        .chain(
            rows.iter()
                .filter_map(|row| row.session_id.as_ref().map(|id| (&row.cli_key, id))),
        )
        .take(HISTORY_SCAN_LIMIT + 100)
    {
        let Ok(source) = cli_key.parse::<cli_sessions::CliSessionsSource>() else {
            continue;
        };
        let session_id = session_id.trim();
        if session_id.is_empty() || session_id.chars().count() > 256 {
            continue;
        }
        let key = (source.as_str().to_string(), session_id.to_string());
        if seen.insert(key) {
            items.push(cli_sessions::CliSessionsFolderLookupKey {
                source,
                session_id: session_id.to_string(),
            });
        }
    }
    cli_sessions::folder_lookup_by_ids(app, &items, None)
        .unwrap_or_default()
        .into_iter()
        .map(|entry| {
            (
                (entry.source, entry.session_id),
                bounded_text(&entry.folder_name, 256),
            )
        })
        .collect()
}

fn project_active(
    item: &ActiveRequestSnapshotItem,
    folders: &HashMap<FolderKey, String>,
    now_ms: i64,
) -> ObserverRequest {
    let attempt_count = item
        .current_attempt
        .as_ref()
        .map(|attempt| attempt.observer_attempt_index())
        .unwrap_or(0);
    let provider_name = item
        .current_attempt
        .as_ref()
        .map(|attempt| bounded_text(attempt.observer_provider_name(), 128));
    let route = item
        .current_attempt
        .as_ref()
        .map(|attempt| {
            vec![ObserverRouteHop {
                provider_name: bounded_text(attempt.observer_provider_name(), 128),
                attempts: 1,
                skipped: false,
                ok: false,
                status: attempt.observer_status().map(i64::from),
                error_code: None,
            }]
        })
        .unwrap_or_default();
    ObserverRequest {
        key: bounded_text(&item.trace_id, 256),
        state: ObserverRequestState::Active,
        cli_key: bounded_text(&item.cli_key, 32),
        method: bounded_text(&item.method, 32),
        path: bounded_text(&item.path, 512),
        model: bounded_optional(item.requested_model.as_deref(), 256),
        provider_name,
        status: None,
        error_code: None,
        interrupted: false,
        created_at_ms: item.created_at_ms.max(0),
        last_activity_ms: item.last_activity_ms.max(item.created_at_ms).max(0),
        duration_ms: Some(now_ms.saturating_sub(item.created_at_ms).max(0)),
        ttfb_ms: None,
        attempt_count,
        retry_count: attempt_count.saturating_sub(1),
        provider_switch_count: 0,
        has_failover: false,
        session_reuse: item
            .current_attempt
            .as_ref()
            .is_some_and(|attempt| attempt.observer_session_reuse()),
        session_id: bounded_optional(item.session_id.as_deref(), 256),
        folder_name: folder_name(folders, &item.cli_key, item.session_id.as_deref()),
        usage: None,
        cost_usd: None,
        route,
        context_compaction: parse_context_compaction(item.special_settings_json.as_deref()),
    }
}

fn project_terminal(
    row: &request_logs::RequestLogSummary,
    folders: &HashMap<FolderKey, String>,
) -> ObserverRequest {
    let attempt_count = u32::try_from(row.attempt_count.max(0)).unwrap_or(u32::MAX);
    let route = row
        .route
        .iter()
        .take(ROUTE_HOP_LIMIT)
        .map(|hop| ObserverRouteHop {
            provider_name: bounded_text(&hop.provider_name, 128),
            attempts: u32::try_from(hop.attempts.max(0)).unwrap_or(u32::MAX),
            skipped: hop.skipped,
            ok: hop.ok,
            status: hop.status.filter(|status| (100..=999).contains(status)),
            error_code: bounded_optional(hop.error_code.as_deref(), 128),
        })
        .collect::<Vec<_>>();
    let provider_switch_count = route
        .windows(2)
        .filter(|pair| pair[0].provider_name != pair[1].provider_name)
        .count();
    let provider_switch_count = u32::try_from(provider_switch_count).unwrap_or(u32::MAX);
    let provider_name = non_empty(&row.final_provider_name)
        .or_else(|| non_empty(&row.start_provider_name))
        .map(|name| bounded_text(name, 128));
    let usage_present = row.input_tokens.is_some()
        || row.output_tokens.is_some()
        || row.total_tokens.is_some()
        || row.cache_read_input_tokens.is_some()
        || row.cache_creation_input_tokens.is_some();
    ObserverRequest {
        key: bounded_text(&row.trace_id, 256),
        state: ObserverRequestState::Terminal,
        cli_key: bounded_text(&row.cli_key, 32),
        method: bounded_text(&row.method, 32),
        path: bounded_text(&row.path, 512),
        model: bounded_optional(row.requested_model.as_deref(), 256),
        provider_name,
        status: row.status.filter(|status| (100..=999).contains(status)),
        error_code: bounded_optional(row.error_code.as_deref(), 128),
        interrupted: row.is_interrupted,
        created_at_ms: row.created_at_ms.max(0),
        last_activity_ms: row.last_activity_ms.unwrap_or(row.created_at_ms).max(0),
        duration_ms: Some(row.duration_ms.max(0)),
        ttfb_ms: row.ttfb_ms.filter(|value| *value >= 0),
        attempt_count,
        retry_count: attempt_count.saturating_sub(1),
        provider_switch_count,
        has_failover: row.has_failover,
        session_reuse: row.session_reuse,
        session_id: bounded_optional(row.session_id.as_deref(), 256),
        folder_name: folder_name(folders, &row.cli_key, row.session_id.as_deref()),
        usage: usage_present.then(|| ObserverRequestUsage {
            input_tokens: non_negative(row.effective_input_tokens.or(row.input_tokens)),
            output_tokens: non_negative(row.output_tokens),
            total_tokens: non_negative(row.total_tokens),
            cache_read_tokens: non_negative(row.cache_read_input_tokens),
            cache_creation_tokens: non_negative(row.cache_creation_input_tokens),
        }),
        cost_usd: row
            .cost_usd
            .filter(|value| value.is_finite() && *value >= 0.0),
        route,
        context_compaction: parse_context_compaction(row.special_settings_json.as_deref()),
    }
}

fn dominant_provider(
    rows: &[&request_logs::RequestLogSummary],
) -> Option<ObserverDominantProvider> {
    let sample = rows.iter().copied().take(10).collect::<Vec<_>>();
    let mut counts = HashMap::<String, u8>::new();
    for row in &sample {
        let Some(name) = terminal_provider_name(row) else {
            continue;
        };
        let count = counts.entry(bounded_text(name, 128)).or_insert(0);
        *count = count.saturating_add(1);
    }
    let max_count = counts.values().copied().max()?;
    let provider_name = sample.iter().find_map(|row| {
        let name = terminal_provider_name(row)?;
        let name = bounded_text(name, 128);
        (counts.get(&name).copied() == Some(max_count)).then_some(name)
    })?;
    Some(ObserverDominantProvider {
        provider_name,
        count: max_count,
        sample_size: u8::try_from(sample.len()).unwrap_or(u8::MAX),
    })
}

fn terminal_provider_name(row: &request_logs::RequestLogSummary) -> Option<&str> {
    non_empty(&row.final_provider_name).or_else(|| non_empty(&row.start_provider_name))
}

fn is_terminal(row: &request_logs::RequestLogSummary) -> bool {
    row.status.is_some() || row.error_code.is_some() || row.is_interrupted
}

fn folder_name(
    folders: &HashMap<FolderKey, String>,
    cli_key: &str,
    session_id: Option<&str>,
) -> Option<String> {
    let session_id = session_id?.trim();
    folders
        .get(&(cli_key.trim().to_string(), session_id.to_string()))
        .cloned()
}

fn parse_context_compaction(raw: Option<&str>) -> Option<ObserverContextCompaction> {
    let raw = raw?.trim();
    if raw.is_empty() || raw.len() > SPECIAL_SETTINGS_MAX_BYTES {
        return None;
    }
    let parsed = serde_json::from_str::<Value>(raw).ok()?;
    let values = match &parsed {
        Value::Array(items) => items.iter().collect::<Vec<_>>(),
        Value::Object(_) => vec![&parsed],
        _ => return None,
    };
    values.into_iter().rev().find_map(|value| {
        let object = value.as_object()?;
        (object.get("type")?.as_str()? == "codex_context_compaction").then_some(())?;
        Some(ObserverContextCompaction {
            mode: known_value(object.get("mode")?, &["local", "remote", "unknown"])?,
            implementation: known_value(
                object.get("implementation")?,
                &[
                    "responses",
                    "responses_compact",
                    "responses_compaction_v2",
                    "unknown",
                ],
            )?,
            trigger: known_value(object.get("trigger")?, &["manual", "auto", "unknown"])?,
            reason: known_value(
                object.get("reason")?,
                &[
                    "user_requested",
                    "context_limit",
                    "model_downshift",
                    "comp_hash_changed",
                    "unknown",
                ],
            )?,
            phase: known_value(
                object.get("phase")?,
                &["standalone_turn", "pre_turn", "mid_turn", "unknown"],
            )?,
            strategy: known_value(
                object.get("strategy")?,
                &["memento", "prefix_compaction", "unknown"],
            )?,
        })
    })
}

fn known_value(value: &Value, allowed: &[&str]) -> Option<String> {
    let value = value.as_str()?;
    allowed.contains(&value).then(|| value.to_string())
}

fn non_negative(value: Option<i64>) -> Option<i64> {
    value.filter(|value| *value >= 0)
}

fn non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

fn bounded_optional(value: Option<&str>, max_chars: usize) -> Option<String> {
    value
        .and_then(non_empty)
        .map(|value| bounded_text(value, max_chars))
}

fn bounded_text(value: &str, max_chars: usize) -> String {
    value
        .chars()
        .filter(|character| {
            !character.is_control()
                && !matches!(
                    *character,
                    '\u{202A}'
                        ..='\u{202E}'
                            | '\u{2066}'
                                ..='\u{2069}'
                )
        })
        .take(max_chars)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compaction_projection_is_strict_and_fail_open() {
        let marker = parse_context_compaction(Some(
            r#"[{"type":"codex_context_compaction","mode":"remote","implementation":"responses_compact","trigger":"manual","reason":"user_requested","phase":"standalone_turn","strategy":"memento"}]"#,
        ))
        .expect("valid marker");
        assert_eq!(marker.mode, "remote");
        assert_eq!(marker.implementation, "responses_compact");
        assert!(parse_context_compaction(Some("not-json")).is_none());
        assert!(parse_context_compaction(Some(
            r#"[{"type":"codex_context_compaction","mode":"future"}]"#
        ))
        .is_none());
    }

    #[test]
    fn dominant_provider_prefers_most_recent_on_tie() {
        let make = |id: i64, name: &str| request_logs::RequestLogSummary {
            id,
            trace_id: format!("trace-{id}"),
            cli_key: "codex".to_string(),
            session_id: None,
            method: "POST".to_string(),
            path: "/v1/responses".to_string(),
            excluded_from_stats: false,
            special_settings_json: None,
            requested_model: None,
            status: Some(200),
            error_code: None,
            is_interrupted: false,
            duration_ms: 1,
            ttfb_ms: None,
            visible_ttfb_ms: None,
            attempt_count: 1,
            has_failover: false,
            start_provider_id: id,
            start_provider_name: name.to_string(),
            final_provider_id: id,
            final_provider_name: name.to_string(),
            final_provider_source_id: None,
            final_provider_source_name: None,
            route: Vec::new(),
            session_reuse: false,
            input_tokens: None,
            output_tokens: None,
            total_tokens: None,
            cache_read_input_tokens: None,
            cache_creation_input_tokens: None,
            cache_creation_5m_input_tokens: None,
            cache_creation_1h_input_tokens: None,
            effective_input_tokens: None,
            cost_usd: None,
            provider_chain_json: None,
            error_details_json: None,
            cost_multiplier: 1.0,
            created_at_ms: id,
            last_activity_ms: None,
            activity_details_json: None,
            created_at: id,
        };
        let newest = make(3, "A");
        let middle = make(2, "B");
        let oldest = make(1, "B");
        let fourth = make(0, "A");
        let result = dominant_provider(&[&newest, &middle, &oldest, &fourth]).expect("dominant");
        assert_eq!(result.provider_name, "A");
        assert_eq!(result.count, 2);
    }

    #[test]
    fn observer_text_projection_strips_terminal_and_bidi_controls() {
        assert_eq!(
            bounded_text("safe\u{1b}[31m\n供应\u{202e}商", 128),
            "safe[31m供应商"
        );
    }
}
