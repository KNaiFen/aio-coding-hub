//! Build bounded, secret-free observer snapshots from existing read models.

use super::{FolderCacheKey, FolderLookupCache, OBSERVER_FOLDER_LOOKUP_MAX_MISSES};
use crate::gateway::active_requests::ActiveRequestSnapshotItem;
use crate::gateway::observation::is_model_inference_request;
use crate::{
    blocking, cli_sessions, gateway_runtime_access, provider_limit_usage, providers, request_logs,
    usage_stats,
};
use aio_observer_protocol::{
    CliScope, ObserverConfiguredModelRoute, ObserverContextCompaction, ObserverDominantProvider,
    ObserverGatewayStatus, ObserverPreferredProvider, ObserverProviderAccountUsage,
    ObserverProviderAvailabilityBucket, ObserverProviderAvailabilityState,
    ObserverProviderAvailabilityTimeline, ObserverProviderCollection, ObserverProviderOAuthQuota,
    ObserverProviderSpendWindow, ObserverProviderStatus, ObserverRequest, ObserverRequestState,
    ObserverRequestUsage, ObserverRouteHop, ObserverSection, ObserverSnapshotV1,
    ObserverTodayUsage, OBSERVER_HISTORY_LIMIT_MAX, OBSERVER_PROTOCOL_VERSION,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};
use tauri::Manager;
use tokio::sync::OwnedSemaphorePermit;

const ACTIVE_REQUEST_LIMIT: usize = 200;
const DOMINANT_PROVIDER_SAMPLE_LIMIT: usize = 10;
const SESSION_ID_MAX_CHARS: usize = 256;
const ROUTE_HOP_LIMIT: usize = 20;
const PROVIDER_STATUS_LIMIT: usize = 512;
const DB_SNAPSHOT_TIMEOUT: Duration = Duration::from_millis(1500);
const SPECIAL_SETTINGS_MAX_BYTES: usize = 32 * 1024;

type FolderKey = (String, String);

struct ProviderCandidate {
    id: i64,
    name: String,
}

struct ProviderObservation {
    id: i64,
    cli_key: String,
    name: String,
    enabled: bool,
    auth_kind: String,
    route_rank: Option<i64>,
    route_enabled: bool,
    spend_limited: bool,
    spend_windows: Vec<ObserverProviderSpendWindow>,
    oauth_limited: bool,
    oauth_limited_reset_at: Option<i64>,
    oauth_quota: Option<ObserverProviderOAuthQuota>,
    availability: Option<ObserverProviderAvailabilityTimeline>,
    account_usage_target:
        Option<crate::app::provider_account_usage_runtime::ProviderAccountUsageTarget>,
}

struct DbProjection {
    inference_available: bool,
    inference_rows: Vec<request_logs::RequestLogSummary>,
    recent_available: bool,
    recent_rows: Vec<request_logs::RequestLogSummary>,
    terminal_trace_ids: HashSet<String>,
    folders: HashMap<FolderKey, String>,
    provider_available: bool,
    provider_cli_key: Option<String>,
    provider_candidates: Vec<ProviderCandidate>,
    limited_provider_ids: HashSet<i64>,
    today: Option<ObserverTodayUsage>,
    provider_details_requested: bool,
    provider_details_available: bool,
    provider_details: Vec<ProviderObservation>,
    provider_details_truncated: bool,
}

struct DbProjectionRequest {
    scope: CliScope,
    active: Vec<ActiveRequestSnapshotItem>,
    history_limit: usize,
    now_unix: i64,
    include_providers: bool,
}

impl DbProjection {
    fn unavailable(provider_details_requested: bool, history_limit: usize) -> Self {
        Self {
            inference_available: false,
            inference_rows: Vec::new(),
            recent_available: history_limit == 0,
            recent_rows: Vec::new(),
            terminal_trace_ids: HashSet::new(),
            folders: HashMap::new(),
            provider_available: false,
            provider_cli_key: None,
            provider_candidates: Vec::new(),
            limited_provider_ids: HashSet::new(),
            today: None,
            provider_details_requested,
            provider_details_available: false,
            provider_details: Vec::new(),
            provider_details_truncated: false,
        }
    }
}

pub(super) async fn build_snapshot(
    app: &tauri::AppHandle,
    db: Option<&crate::db::Db>,
    db_query_permit: Option<OwnedSemaphorePermit>,
    folder_cache: Arc<StdMutex<FolderLookupCache>>,
    scope: CliScope,
    history_limit: usize,
    include_providers: bool,
) -> ObserverSnapshotV1 {
    let history_limit = history_limit.min(usize::from(OBSERVER_HISTORY_LIMIT_MAX));
    let generated_at_ms = crate::shared::time::now_unix_millis();
    let gateway_status = gateway_runtime_access::app_gateway_status(app);
    let raw_active = gateway_runtime_access::app_gateway_active_requests_snapshot(app);

    let db_projection = load_db_projection(
        app,
        db,
        db_query_permit,
        folder_cache,
        DbProjectionRequest {
            scope,
            active: raw_active.clone(),
            history_limit,
            now_unix: generated_at_ms / 1000,
            include_providers,
        },
    )
    .await
    .unwrap_or_else(|| DbProjection::unavailable(include_providers, history_limit));
    let active = raw_active
        .iter()
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

    let terminal_inference = db_projection.inference_rows.iter().collect::<Vec<_>>();

    let last_request = if db_projection.inference_available {
        terminal_inference
            .first()
            .map(|row| project_terminal(row, &db_projection.folders))
            .map(ObserverSection::ready)
            .unwrap_or_else(ObserverSection::empty)
    } else {
        ObserverSection::unavailable()
    };
    let dominant_provider = if db_projection.inference_available {
        dominant_provider(&terminal_inference)
            .map(ObserverSection::ready)
            .unwrap_or_else(ObserverSection::empty)
    } else {
        ObserverSection::unavailable()
    };
    let recent_requests = if db_projection.recent_available {
        ObserverSection::ready(
            db_projection
                .recent_rows
                .iter()
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
    let account_usage_by_provider = if include_providers {
        if let Some(runtime) = app.try_state::<
            crate::app::provider_account_usage_runtime::ProviderAccountUsageRuntimeState,
        >() {
            let targets = db_projection
                .provider_details
                .iter()
                .filter_map(|provider| provider.account_usage_target)
                .collect::<Vec<_>>();
            let provider_ids = targets
                .iter()
                .map(|target| target.provider_id)
                .collect::<Vec<_>>();
            runtime.touch_tui(app, targets).await;
            runtime
                .display_snapshots(&provider_ids, generated_at_ms / 1000)
                .await
        } else {
            HashMap::new()
        }
    } else {
        HashMap::new()
    };
    let provider_statuses = project_provider_statuses(
        app,
        gateway_status.running,
        generated_at_ms / 1000,
        &db_projection,
        &account_usage_by_provider,
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
        providers: provider_statuses,
    }
}

async fn load_db_projection(
    app: &tauri::AppHandle,
    db: Option<&crate::db::Db>,
    db_query_permit: Option<OwnedSemaphorePermit>,
    folder_cache: Arc<StdMutex<FolderLookupCache>>,
    request: DbProjectionRequest,
) -> Option<DbProjection> {
    let db = db?.clone();
    let db_query_permit = db_query_permit?;
    let app = app.clone();
    tokio::time::timeout(
        DB_SNAPSHOT_TIMEOUT,
        blocking::run("observer_snapshot", move || {
            let _db_query_permit = db_query_permit;
            Ok::<_, crate::shared::error::AppError>(build_db_projection(
                &app,
                &db,
                &folder_cache,
                &request,
            ))
        }),
    )
    .await
    .ok()?
    .ok()
}

fn build_db_projection(
    app: &tauri::AppHandle,
    db: &crate::db::Db,
    folder_cache: &Arc<StdMutex<FolderLookupCache>>,
    request: &DbProjectionRequest,
) -> DbProjection {
    let active_trace_ids = observer_active_trace_ids(&request.active);
    let terminal_trace_ids =
        request_logs::observer_persisted_trace_ids(db, &active_trace_ids).unwrap_or_default();
    let active_trace_ids = active_trace_ids
        .into_iter()
        .filter(|trace_id| !terminal_trace_ids.contains(trace_id))
        .collect::<Vec<_>>();
    let cli_key = (request.scope != CliScope::All).then(|| request.scope.as_str());
    let inference_result = request_logs::list_observer_terminal_inferences(
        db,
        cli_key,
        DOMINANT_PROVIDER_SAMPLE_LIMIT,
    );
    let inference_available = inference_result.is_ok();
    let inference_rows = inference_result.unwrap_or_default();
    let recent_result = load_observer_recent_rows(request.history_limit, |limit| {
        request_logs::list_observer_recent_terminal(db, cli_key, limit, &active_trace_ids)
    });
    let recent_available = recent_result.is_ok();
    let recent_rows = recent_result.unwrap_or_default();
    let rendered_active = rendered_active(&request.active, &terminal_trace_ids, request.scope);
    let folders = resolve_folders(
        app,
        folder_cache,
        &rendered_active,
        inference_rows.first(),
        &recent_rows,
    );

    let provider_cli_key = preferred_cli_key(request.scope, inference_rows.first());
    let provider_result = provider_cli_key
        .as_deref()
        .map(|cli_key| load_provider_candidates(db, cli_key, request.now_unix));
    let provider_available = provider_result.as_ref().is_none_or(|result| result.is_ok());
    let (provider_candidates, limited_provider_ids) =
        provider_result.and_then(Result::ok).unwrap_or_default();

    let today = today_usage(db);
    let availability_hours = crate::settings::read(app)
        .map(|settings| settings.provider_availability_hours)
        .unwrap_or(crate::settings::DEFAULT_PROVIDER_AVAILABILITY_HOURS);
    let provider_details_result = request.include_providers.then(|| {
        load_provider_observations(
            db,
            (request.scope != CliScope::All).then(|| request.scope.as_str()),
            request.now_unix,
            availability_hours,
        )
    });
    let provider_details_available = provider_details_result
        .as_ref()
        .is_none_or(|result| result.is_ok());
    let (provider_details, provider_details_truncated) = provider_details_result
        .and_then(Result::ok)
        .unwrap_or_default();
    DbProjection {
        inference_available,
        inference_rows,
        recent_available,
        recent_rows,
        terminal_trace_ids,
        folders,
        provider_available,
        provider_cli_key,
        provider_candidates,
        limited_provider_ids,
        today,
        provider_details_requested: request.include_providers,
        provider_details_available,
        provider_details,
        provider_details_truncated,
    }
}

fn load_provider_candidates(
    db: &crate::db::Db,
    cli_key: &str,
    now_unix: i64,
) -> crate::shared::error::AppResult<(Vec<ProviderCandidate>, HashSet<i64>)> {
    let providers =
        providers::list_enabled_gateway_provider_identities_using_active_mode(db, cli_key)?;
    let mut limited_provider_ids = provider_limit_usage::list_v1(db, Some(cli_key))?
        .into_iter()
        .filter(provider_limit_usage::ProviderLimitUsageRow::is_limit_reached)
        .map(|row| row.provider_id)
        .collect::<HashSet<_>>();
    let oauth_provider_ids = providers
        .iter()
        .filter(|provider| provider.auth_mode == "oauth")
        .map(|provider| provider.id)
        .collect::<Vec<_>>();
    for provider_ids in
        oauth_provider_ids.chunks(crate::domain::provider_oauth_limits::MAX_DISPLAY_PROVIDER_IDS)
    {
        limited_provider_ids.extend(
            crate::domain::provider_oauth_limits::list_display_snapshots(
                db,
                provider_ids,
                now_unix,
            )?
            .into_iter()
            .filter(|snapshot| snapshot.limited)
            .map(|snapshot| snapshot.provider_id),
        );
    }
    let providers = providers
        .into_iter()
        .map(|provider| ProviderCandidate {
            id: provider.id,
            name: bounded_text(&provider.name, 128),
        })
        .collect();
    Ok((providers, limited_provider_ids))
}

fn load_provider_observations(
    db: &crate::db::Db,
    cli_key: Option<&str>,
    now_unix: i64,
    availability_hours: u32,
) -> crate::shared::error::AppResult<(Vec<ProviderObservation>, bool)> {
    let (rows, truncated) = providers::list_observer_rows(db, cli_key, PROVIDER_STATUS_LIMIT)?;
    let provider_ids = rows.iter().map(|row| row.id).collect::<Vec<_>>();
    let availability_by_provider = crate::domain::provider_availability::timelines(
        db,
        &provider_ids,
        availability_hours,
        crate::domain::provider_availability::TUI_PROVIDER_AVAILABILITY_BUCKETS,
        now_unix.saturating_mul(1_000),
    )
    .map(|timelines| {
        timelines
            .into_iter()
            .map(|timeline| {
                (
                    timeline.provider_id,
                    observer_availability_timeline(timeline),
                )
            })
            .collect::<HashMap<_, _>>()
    })
    .unwrap_or_else(|error| {
        tracing::warn!(error = %error.code(), "observer provider availability is unavailable");
        HashMap::new()
    });
    let spend_by_provider = provider_limit_usage::list_v1(db, cli_key)?
        .into_iter()
        .map(|row| (row.provider_id, row))
        .collect::<HashMap<_, _>>();
    let oauth_provider_ids = rows
        .iter()
        .filter(|row| row.auth_mode == "oauth")
        .map(|row| row.id)
        .collect::<Vec<_>>();
    let oauth_by_provider = crate::domain::provider_oauth_limits::list_display_snapshots(
        db,
        &oauth_provider_ids,
        now_unix,
    )?
    .into_iter()
    .map(|snapshot| (snapshot.provider_id, snapshot))
    .collect::<HashMap<_, _>>();

    let items = rows
        .into_iter()
        .map(|row| {
            let spend = spend_by_provider.get(&row.id);
            let oauth = oauth_by_provider.get(&row.id);
            let account_usage_target = row
                .account_usage_values
                .as_ref()
                .and_then(|values| {
                    crate::domain::provider_account_usage::refresh_schedule_from_value(
                        values,
                        row.account_usage_updated_at.unwrap_or_default(),
                    )
                })
                .map(|schedule| {
                    crate::app::provider_account_usage_runtime::ProviderAccountUsageTarget {
                        provider_id: row.id,
                        schedule,
                    }
                });
            ProviderObservation {
                id: row.id,
                cli_key: bounded_text(&row.cli_key, 16),
                name: bounded_text(&row.name, 128),
                enabled: row.enabled,
                auth_kind: if row.auth_mode == "oauth" {
                    "oauth".to_string()
                } else {
                    "api_key".to_string()
                },
                route_rank: row.route_rank.map(|rank| rank.max(0).saturating_add(1)),
                route_enabled: row.route_enabled,
                spend_limited: spend.is_some_and(|value| value.is_limit_reached()),
                spend_windows: spend.map(spend_windows).unwrap_or_default(),
                oauth_limited: oauth.is_some_and(|value| value.limited),
                oauth_limited_reset_at: oauth.and_then(|value| value.limited_reset_at),
                oauth_quota: oauth.map(|value| ObserverProviderOAuthQuota {
                    short_label: value
                        .limit_short_label
                        .as_deref()
                        .map(|text| bounded_text(text, 32)),
                    five_hour_text: value
                        .limit_5h_text
                        .as_deref()
                        .map(|text| bounded_text(text, 96)),
                    weekly_text: value
                        .limit_weekly_text
                        .as_deref()
                        .map(|text| bounded_text(text, 96)),
                    five_hour_reset_at_unix: value.limit_5h_reset_at,
                    weekly_reset_at_unix: value.limit_weekly_reset_at,
                    checked_at_unix: value.checked_at,
                }),
                availability: availability_by_provider.get(&row.id).cloned(),
                account_usage_target,
            }
        })
        .collect();
    Ok((items, truncated))
}

fn observer_availability_timeline(
    timeline: crate::domain::provider_availability::ProviderAvailabilityTimeline,
) -> ObserverProviderAvailabilityTimeline {
    ObserverProviderAvailabilityTimeline {
        hours: timeline.hours,
        bucket_minutes: timeline.bucket_minutes,
        success_count: timeline.success_count,
        failure_count: timeline.failure_count,
        buckets: timeline
            .buckets
            .into_iter()
            .map(|bucket| ObserverProviderAvailabilityBucket {
                start_at_ms: bucket.start_at_ms,
                end_at_ms: bucket.end_at_ms,
                success_count: bucket.success_count,
                failure_count: bucket.failure_count,
                state: match bucket.state {
                    crate::domain::provider_availability::ProviderAvailabilityState::Healthy => {
                        ObserverProviderAvailabilityState::Healthy
                    }
                    crate::domain::provider_availability::ProviderAvailabilityState::Unhealthy => {
                        ObserverProviderAvailabilityState::Unhealthy
                    }
                    crate::domain::provider_availability::ProviderAvailabilityState::NoData => {
                        ObserverProviderAvailabilityState::NoData
                    }
                },
            })
            .collect(),
    }
}

fn spend_windows(
    row: &provider_limit_usage::ProviderLimitUsageRow,
) -> Vec<ObserverProviderSpendWindow> {
    let mut windows = Vec::new();
    push_spend_window(&mut windows, "5h", row.usage_5h_usd, row.limit_5h_usd);
    push_spend_window(
        &mut windows,
        "daily",
        row.usage_daily_usd,
        row.limit_daily_usd,
    );
    push_spend_window(
        &mut windows,
        "weekly",
        row.usage_weekly_usd,
        row.limit_weekly_usd,
    );
    push_spend_window(
        &mut windows,
        "monthly",
        row.usage_monthly_usd,
        row.limit_monthly_usd,
    );
    push_spend_window(
        &mut windows,
        "total",
        row.usage_total_usd,
        row.limit_total_usd,
    );
    windows
}

fn push_spend_window(
    output: &mut Vec<ObserverProviderSpendWindow>,
    window: &str,
    usage_usd: f64,
    limit_usd: Option<f64>,
) {
    let Some(limit_usd) = limit_usd.filter(|value| value.is_finite() && *value >= 0.0) else {
        return;
    };
    let usage_usd = if usage_usd.is_finite() {
        usage_usd.max(0.0)
    } else {
        0.0
    };
    output.push(ObserverProviderSpendWindow {
        window: window.to_string(),
        usage_usd,
        limit_usd,
    });
}

fn preferred_cli_key(
    scope: CliScope,
    last_inference: Option<&request_logs::RequestLogSummary>,
) -> Option<String> {
    if scope != CliScope::All {
        return Some(scope.as_str().to_string());
    }
    last_inference.map(|row| row.cli_key.clone())
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
    first_eligible_provider(
        &projection.provider_candidates,
        &projection.limited_provider_ids,
        &by_id,
        now_unix,
    )
    .map(|(provider, status)| ObserverPreferredProvider {
        cli_key: cli_key.to_string(),
        provider_name: provider.name.clone(),
        circuit_state: status.state.clone(),
    })
    .map(ObserverSection::ready)
    .unwrap_or_else(ObserverSection::empty)
}

fn project_provider_statuses(
    app: &tauri::AppHandle,
    gateway_running: bool,
    now_unix: i64,
    projection: &DbProjection,
    account_usage_by_provider: &HashMap<
        i64,
        crate::app::provider_account_usage_runtime::ProviderAccountUsageDisplaySnapshot,
    >,
) -> Option<ObserverSection<ObserverProviderCollection>> {
    if !projection.provider_details_requested {
        return None;
    }
    if !projection.provider_details_available {
        return Some(ObserverSection::unavailable());
    }
    let provider_ids = projection
        .provider_details
        .iter()
        .map(|provider| provider.id)
        .collect::<Vec<_>>();
    let statuses = if gateway_running {
        gateway_runtime_access::app_gateway_circuit_status_peek(app, &provider_ids, now_unix)
    } else {
        Vec::new()
    };
    let by_id = statuses
        .into_iter()
        .map(|status| (status.provider_id, status))
        .collect::<HashMap<_, _>>();
    let mut preferred_cli_keys = HashSet::new();
    let items = projection
        .provider_details
        .iter()
        .map(|provider| {
            let circuit = by_id.get(&provider.id);
            let eligibility = provider_eligibility(provider, circuit, gateway_running, now_unix);
            let preferred = matches!(eligibility, "ready" | "half_open")
                && preferred_cli_keys.insert(provider.cli_key.clone());
            let circuit_recover_at = circuit
                .and_then(|status| status.cooldown_until.or(status.open_until))
                .filter(|until| *until > now_unix);
            ObserverProviderStatus {
                provider_id: provider.id,
                cli_key: provider.cli_key.clone(),
                provider_name: provider.name.clone(),
                route_rank: provider.route_rank,
                provider_enabled: provider.enabled,
                route_enabled: provider.route_enabled,
                auth_kind: provider.auth_kind.clone(),
                preferred,
                eligibility: eligibility.to_string(),
                circuit_state: circuit.map(|status| bounded_text(&status.state, 24)),
                circuit_failure_count: circuit.map(|status| status.failure_count),
                circuit_failure_threshold: circuit.map(|status| status.failure_threshold),
                recover_at_unix: if eligibility == "oauth_limited" {
                    provider.oauth_limited_reset_at
                } else {
                    circuit_recover_at
                },
                spend_windows: provider.spend_windows.clone(),
                oauth_quota: provider.oauth_quota.clone(),
                account_usage: provider.account_usage_target.map(|_| {
                    let snapshot = account_usage_by_provider.get(&provider.id);
                    ObserverProviderAccountUsage {
                        state: snapshot
                            .map(|value| value.state)
                            .unwrap_or("loading")
                            .to_string(),
                        amount: snapshot.and_then(|value| value.amount),
                        unit: snapshot
                            .and_then(|value| value.unit.as_deref())
                            .map(|value| bounded_text(value, 24)),
                        last_fetched_at_unix: snapshot.and_then(|value| value.last_fetched_at),
                    }
                }),
                availability: provider.availability.clone(),
            }
        })
        .collect();
    Some(ObserverSection::ready(ObserverProviderCollection {
        items,
        truncated: projection.provider_details_truncated,
    }))
}

fn provider_eligibility(
    provider: &ProviderObservation,
    circuit: Option<&crate::gateway::GatewayProviderCircuitStatus>,
    gateway_running: bool,
    now_unix: i64,
) -> &'static str {
    if !provider.enabled {
        return "provider_disabled";
    }
    if provider.route_rank.is_none() {
        return "not_in_route";
    }
    if !provider.route_enabled {
        return "route_disabled";
    }
    if provider.spend_limited {
        return "spend_limited";
    }
    if provider.oauth_limited {
        return "oauth_limited";
    }
    if !gateway_running {
        return "gateway_stopped";
    }
    let Some(circuit) = circuit else {
        return "unknown";
    };
    if circuit.cooldown_until.is_some_and(|until| until > now_unix) {
        return "cooldown";
    }
    match circuit.state.as_str() {
        "OPEN" => "circuit_open",
        "HALF_OPEN" => "half_open",
        "CLOSED" => "ready",
        _ => "unknown",
    }
}

fn first_eligible_provider<'a>(
    providers: &'a [ProviderCandidate],
    limited_provider_ids: &HashSet<i64>,
    statuses: &'a HashMap<i64, crate::gateway::GatewayProviderCircuitStatus>,
    now_unix: i64,
) -> Option<(
    &'a ProviderCandidate,
    &'a crate::gateway::GatewayProviderCircuitStatus,
)> {
    providers.iter().find_map(|provider| {
        if limited_provider_ids.contains(&provider.id) {
            return None;
        }
        let status = statuses.get(&provider.id)?;
        let cooldown_active = status.cooldown_until.is_some_and(|until| until > now_unix);
        (status.state != "OPEN" && !cooldown_active).then_some((provider, status))
    })
}

fn recent_query_limit(history_limit: usize) -> Option<usize> {
    (history_limit > 0).then_some(history_limit)
}

fn load_observer_recent_rows<F>(
    history_limit: usize,
    load: F,
) -> crate::shared::error::AppResult<Vec<request_logs::RequestLogSummary>>
where
    F: FnOnce(usize) -> crate::shared::error::AppResult<Vec<request_logs::RequestLogSummary>>,
{
    match recent_query_limit(history_limit) {
        Some(limit) => load(limit),
        None => Ok(Vec::new()),
    }
}

fn observer_active_trace_ids(active: &[ActiveRequestSnapshotItem]) -> Vec<String> {
    active.iter().map(|item| item.trace_id.clone()).collect()
}

fn rendered_active<'a>(
    active: &'a [ActiveRequestSnapshotItem],
    terminal_trace_ids: &HashSet<String>,
    scope: CliScope,
) -> Vec<&'a ActiveRequestSnapshotItem> {
    active
        .iter()
        .filter(|item| !terminal_trace_ids.contains(&item.trace_id))
        .filter(|item| scope.matches(&item.cli_key))
        .take(ACTIVE_REQUEST_LIMIT)
        .collect()
}

fn folder_lookup_keys<'a>(
    items: impl Iterator<Item = (&'a str, Option<&'a str>)>,
) -> Vec<FolderCacheKey> {
    let mut keys = Vec::new();
    let mut seen = HashSet::new();
    for (cli_key, session_id) in items {
        let Ok(source) = cli_key.parse::<cli_sessions::CliSessionsSource>() else {
            continue;
        };
        let Some(session_id) = session_id.map(str::trim) else {
            continue;
        };
        if session_id.is_empty() || session_id.chars().count() > SESSION_ID_MAX_CHARS {
            continue;
        }
        let key = (source, session_id.to_string());
        if seen.insert(key.clone()) {
            keys.push(key);
        }
    }
    keys
}

fn resolve_folders(
    app: &tauri::AppHandle,
    folder_cache: &Arc<StdMutex<FolderLookupCache>>,
    active: &[&ActiveRequestSnapshotItem],
    last_inference: Option<&request_logs::RequestLogSummary>,
    recent: &[request_logs::RequestLogSummary],
) -> HashMap<FolderKey, String> {
    let keys = folder_lookup_keys(
        active
            .iter()
            .map(|item| (item.cli_key.as_str(), item.session_id.as_deref()))
            .chain(
                last_inference
                    .into_iter()
                    .chain(recent.iter())
                    .map(|row| (row.cli_key.as_str(), row.session_id.as_deref())),
            ),
    );
    let lookup_started_at = Instant::now();
    let (mut folders, misses) = {
        let mut cache = folder_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        cache.lookup(&keys, lookup_started_at)
    };
    let misses = misses
        .into_iter()
        .take(OBSERVER_FOLDER_LOOKUP_MAX_MISSES)
        .collect::<Vec<_>>();
    if !misses.is_empty() {
        let items = misses
            .iter()
            .map(
                |(source, session_id)| cli_sessions::CliSessionsFolderLookupKey {
                    source: *source,
                    session_id: session_id.clone(),
                },
            )
            .collect::<Vec<_>>();
        let requested = misses.iter().cloned().collect::<HashSet<_>>();
        let scanned = cli_sessions::folder_lookup_by_ids(app, &items, None).unwrap_or_default();
        let mut found = HashMap::new();
        for entry in scanned {
            let Ok(source) = entry.source.parse::<cli_sessions::CliSessionsSource>() else {
                continue;
            };
            let key = (source, entry.session_id.trim().to_string());
            if requested.contains(&key) {
                found.insert(key, bounded_text(&entry.folder_name, 256));
            }
        }
        let recorded_at = Instant::now();
        let mut cache = folder_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for key in misses {
            let folder_name = found.get(&key).cloned();
            cache.record(key.clone(), folder_name.clone(), recorded_at);
            if let Some(folder_name) = folder_name {
                folders.insert(key, folder_name);
            }
        }
    }
    folders
        .into_iter()
        .map(|((source, session_id), folder_name)| {
            ((source.as_str().to_string(), session_id), folder_name)
        })
        .collect()
}

fn project_active(
    item: &ActiveRequestSnapshotItem,
    folders: &HashMap<FolderKey, String>,
    now_ms: i64,
) -> ObserverRequest {
    let configured_model_route = item.current_attempt.as_ref().and_then(|attempt| {
        parse_configured_model_route(
            attempt
                .observer_special_settings_json()
                .or(item.special_settings_json.as_deref()),
            Some(attempt.observer_provider_id()),
        )
    });
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
        visible_ttfb_ms: None,
        upstream_stream_duration_ms: None,
        upstream_stream_timing_version: 0,
        final_upstream_attempt_duration_ms: None,
        final_upstream_attempt_timing_version: 0,
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
        requested_reasoning_effort: if item.cli_key == "codex" {
            parse_requested_reasoning_effort(item.special_settings_json.as_deref())
        } else {
            None
        },
        configured_model_route,
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
    let (provider_switch_count, retry_count) = effective_terminal_route_counts(&row.route);
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
        visible_ttfb_ms: row.visible_ttfb_ms.filter(|value| *value >= 0),
        upstream_stream_duration_ms: row.upstream_stream_duration_ms.filter(|value| *value > 0),
        upstream_stream_timing_version: row.upstream_stream_timing_version,
        final_upstream_attempt_duration_ms: row
            .final_upstream_attempt_duration_ms
            .filter(|value| *value > 0),
        final_upstream_attempt_timing_version: row.final_upstream_attempt_timing_version,
        attempt_count,
        retry_count,
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
        requested_reasoning_effort: if row.cli_key == "codex" {
            parse_requested_reasoning_effort(row.special_settings_json.as_deref())
        } else {
            None
        },
        configured_model_route: parse_configured_model_route(
            row.special_settings_json.as_deref(),
            Some(row.final_provider_id),
        ),
    }
}

fn effective_terminal_route_counts(route: &[request_logs::RequestLogRouteHop]) -> (u32, u32) {
    let mut previous_provider: Option<String> = None;
    let mut provider_switch_count = 0_u32;
    let mut retry_count = 0_u32;

    for hop in route
        .iter()
        .take(ROUTE_HOP_LIMIT)
        .filter(|hop| !hop.skipped)
    {
        let attempts = u32::try_from(hop.attempts.max(1)).unwrap_or(u32::MAX);
        retry_count = retry_count.saturating_add(attempts.saturating_sub(1));

        let provider = if hop.provider_id > 0 {
            Some(format!("id:{}", hop.provider_id))
        } else {
            non_empty(&hop.provider_name).map(|name| format!("name:{}", bounded_text(name, 128)))
        };
        let Some(provider) = provider else {
            continue;
        };
        if previous_provider
            .as_ref()
            .is_some_and(|previous| previous != &provider)
        {
            provider_switch_count = provider_switch_count.saturating_add(1);
        }
        previous_provider = Some(provider);
    }

    (provider_switch_count, retry_count)
}

fn dominant_provider(
    rows: &[&request_logs::RequestLogSummary],
) -> Option<ObserverDominantProvider> {
    let sample = rows
        .iter()
        .copied()
        .take(DOMINANT_PROVIDER_SAMPLE_LIMIT)
        .collect::<Vec<_>>();
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

fn parse_configured_model_route(
    raw: Option<&str>,
    final_provider_id: Option<i64>,
) -> Option<ObserverConfiguredModelRoute> {
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
        if object.get("type")?.as_str()? != "configured_model_route"
            || !object.get("applied")?.as_bool()?
        {
            return None;
        }
        let provider_id = object.get("providerId")?.as_i64()?;
        if provider_id <= 0 || final_provider_id.is_some_and(|expected| expected != provider_id) {
            return None;
        }
        let source_model = bounded_optional(object.get("sourceModel")?.as_str(), 256)?;
        let effective_model = bounded_optional(object.get("effectiveModel")?.as_str(), 256)?;
        let policy_source = known_value(object.get("policySource")?, &["global", "provider"])?;
        let model_applied = object.get("modelApplied")?.as_bool()?;
        let reasoning_effort_applied = object.get("reasoningEffortApplied")?.as_bool()?;
        if !model_applied && !reasoning_effort_applied {
            return None;
        }
        let reasoning_effort = object
            .get("reasoningEffort")
            .and_then(Value::as_str)
            .and_then(|value| bounded_optional(Some(value), 128));
        if reasoning_effort_applied && reasoning_effort.is_none() {
            return None;
        }
        Some(ObserverConfiguredModelRoute {
            source_model,
            effective_model,
            reasoning_effort,
            policy_source,
            model_applied,
            reasoning_effort_applied,
        })
    })
}

fn parse_requested_reasoning_effort(raw: Option<&str>) -> Option<String> {
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
        if object.get("type")?.as_str()? != "codex_reasoning_effort" {
            return None;
        }
        object
            .get("effort")
            .and_then(Value::as_str)
            .and_then(normalize_reasoning_effort)
            .or_else(|| {
                object
                    .get("rawEffort")
                    .and_then(Value::as_str)
                    .and_then(normalize_reasoning_effort)
            })
    })
}

fn normalize_reasoning_effort(value: &str) -> Option<String> {
    let value = value.trim().to_ascii_lowercase();
    matches!(
        value.as_str(),
        "none" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max" | "ultra"
    )
    .then_some(value)
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

    fn active_snapshot(
        trace_id: String,
        cli_key: &str,
        created_at_ms: i64,
    ) -> ActiveRequestSnapshotItem {
        ActiveRequestSnapshotItem {
            trace_id,
            cli_key: cli_key.to_string(),
            method: "POST".to_string(),
            path: "/v1/responses".to_string(),
            query: None,
            session_id: None,
            requested_model: None,
            special_settings_json: None,
            created_at_ms,
            last_activity_ms: created_at_ms,
            current_attempt: None,
        }
    }

    fn insert_observer_provider(db: &crate::db::Db, name: &str, total_limit: Option<f64>) -> i64 {
        crate::providers::upsert(
            db,
            crate::providers::ProviderUpsertParams {
                provider_id: None,
                cli_key: "codex".to_string(),
                name: name.to_string(),
                base_urls: vec!["https://example.test".to_string()],
                base_url_mode: crate::providers::ProviderBaseUrlMode::Order,
                auth_mode: Some(crate::providers::ProviderAuthMode::ApiKey),
                api_key: Some("test-key".to_string()),
                enabled: true,
                cost_multiplier: 1.0,
                priority: Some(0),
                claude_models: None,
                model_mapping: None,
                availability_test_model: None,
                limit_5h_usd: None,
                limit_daily_usd: None,
                daily_reset_mode: None,
                daily_reset_time: None,
                limit_weekly_usd: None,
                limit_monthly_usd: None,
                limit_total_usd: total_limit,
                tags: None,
                note: None,
                source_provider_id: None,
                bridge_type: None,
                stream_idle_timeout_seconds: None,
                extension_values: None,
                account_usage_credentials_patch: None,
                account_usage_credentials_copy_from_provider_id: None,
                upstream_retry_policy_override: None,
                upstream_retry_policy_override_specified: false,
                model_routing_policy_override: None,
                model_routing_policy_override_specified: false,
            },
        )
        .expect("insert observer provider")
        .id
    }

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
    fn configured_model_route_projection_is_provider_scoped_and_fail_open() {
        let raw = r#"[{"type":"configured_model_route","providerId":7,"policySource":"provider","sourceModel":"fable5","effectiveModel":"opus4.8","reasoningEffort":"low","applied":true,"modelApplied":true,"reasoningEffortApplied":true}]"#;
        let route = parse_configured_model_route(Some(raw), Some(7)).expect("valid route marker");

        assert_eq!(route.source_model, "fable5");
        assert_eq!(route.effective_model, "opus4.8");
        assert_eq!(route.reasoning_effort.as_deref(), Some("low"));
        assert_eq!(route.policy_source, "provider");
        assert!(parse_configured_model_route(Some(raw), Some(8)).is_none());
        assert!(parse_configured_model_route(Some("not-json"), Some(7)).is_none());
        assert!(parse_configured_model_route(
            Some(
                r#"[{"type":"configured_model_route","providerId":7,"policySource":"future","sourceModel":"fable5","effectiveModel":"opus4.8","applied":true,"modelApplied":true,"reasoningEffortApplied":false}]"#,
            ),
            Some(7),
        )
        .is_none());
    }

    #[test]
    fn requested_reasoning_effort_projection_normalizes_and_fails_open() {
        let raw = r#"[{"type":"codex_reasoning_effort","effort":" MAX ","rawEffort":"max"}]"#;
        assert_eq!(
            parse_requested_reasoning_effort(Some(raw)).as_deref(),
            Some("max")
        );
        let legacy = r#"[{"type":"codex_reasoning_effort","effort":null,"rawEffort":"Ultra"}]"#;
        assert_eq!(
            parse_requested_reasoning_effort(Some(legacy)).as_deref(),
            Some("ultra")
        );
        let invalid_normalized =
            r#"[{"type":"codex_reasoning_effort","effort":"turbo","rawEffort":"high"}]"#;
        assert_eq!(
            parse_requested_reasoning_effort(Some(invalid_normalized)).as_deref(),
            Some("high")
        );
        assert!(parse_requested_reasoning_effort(Some(
            r#"[{"type":"codex_reasoning_effort","effort":"turbo"}]"#
        ))
        .is_none());
        assert!(parse_requested_reasoning_effort(Some("not-json")).is_none());
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
            upstream_stream_duration_ms: None,
            upstream_stream_timing_version: 0,
            final_upstream_attempt_duration_ms: None,
            final_upstream_attempt_timing_version: 0,
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
    fn zero_history_has_no_recent_query_but_keeps_an_available_empty_projection() {
        assert_eq!(recent_query_limit(0), None);
        assert_eq!(recent_query_limit(50), Some(50));

        let projection = DbProjection::unavailable(false, 0);
        assert!(projection.recent_available);
        assert!(projection.recent_rows.is_empty());
        assert!(!projection.inference_available);

        let called = std::cell::Cell::new(false);
        let rows = load_observer_recent_rows(0, |_| {
            called.set(true);
            Ok(Vec::new())
        })
        .expect("zero history should not query recent rows");
        assert!(rows.is_empty());
        assert!(!called.get(), "zero history must skip the recent query");
    }

    #[test]
    fn active_trace_collection_never_silently_truncates_global_concurrency() {
        let mut active = (0..250)
            .map(|index| active_snapshot(format!("claude-{index}"), "claude", index))
            .collect::<Vec<_>>();
        active.extend(
            (0..250).map(|index| active_snapshot(format!("codex-{index}"), "codex", index + 1_000)),
        );

        let trace_ids = observer_active_trace_ids(&active);

        assert_eq!(trace_ids.len(), 500);
        assert!(trace_ids.iter().any(|trace_id| trace_id == "codex-0"));
        assert!(trace_ids.iter().any(|trace_id| trace_id == "codex-249"));
        assert!(trace_ids.iter().any(|trace_id| trace_id == "claude-0"));
        assert!(trace_ids.iter().any(|trace_id| trace_id == "claude-249"));
    }

    #[test]
    fn folder_keys_are_source_aware_bounded_and_ignore_invalid_sessions() {
        let long_session = "x".repeat(SESSION_ID_MAX_CHARS + 1);
        let pairs = vec![
            ("codex", Some("same-session")),
            ("claude", Some("same-session")),
            ("codex", Some(" same-session ")),
            ("grok", Some("ignored-source")),
            ("codex", Some(long_session.as_str())),
            ("codex", None),
        ];
        let keys = folder_lookup_keys(pairs.into_iter());
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&(
            cli_sessions::CliSessionsSource::Codex,
            "same-session".to_string()
        )));
        assert!(keys.contains(&(
            cli_sessions::CliSessionsSource::Claude,
            "same-session".to_string()
        )));

        let many = (0..OBSERVER_FOLDER_LOOKUP_MAX_MISSES + 5)
            .map(|index| ("codex", Some(format!("session-{index}"))))
            .collect::<Vec<_>>();
        let all_keys = folder_lookup_keys(
            many.iter()
                .map(|(source, session)| (*source, session.as_deref())),
        );
        assert_eq!(all_keys.len(), OBSERVER_FOLDER_LOOKUP_MAX_MISSES + 5);

        let now = Instant::now();
        let mut cache = FolderLookupCache::default();
        for key in all_keys.iter().take(OBSERVER_FOLDER_LOOKUP_MAX_MISSES) {
            cache.record(key.clone(), Some("cached".to_string()), now);
        }
        let (_, misses) = cache.lookup(&all_keys, now);
        assert_eq!(
            misses,
            all_keys[OBSERVER_FOLDER_LOOKUP_MAX_MISSES..].to_vec()
        );
    }

    #[test]
    fn observer_text_projection_strips_terminal_and_bidi_controls() {
        assert_eq!(
            bounded_text("safe\u{1b}[31m\n供应\u{202e}商", 128),
            "safe[31m供应商"
        );
    }

    #[test]
    fn preferred_provider_skips_limited_and_circuit_denied_candidates() {
        let providers = vec![
            ProviderCandidate {
                id: 1,
                name: "Limited".to_string(),
            },
            ProviderCandidate {
                id: 2,
                name: "Open".to_string(),
            },
            ProviderCandidate {
                id: 3,
                name: "Ready".to_string(),
            },
        ];
        let limited = HashSet::from([1]);
        let statuses = HashMap::from([
            (
                1,
                crate::gateway::GatewayProviderCircuitStatus {
                    provider_id: 1,
                    state: "CLOSED".to_string(),
                    failure_count: 0,
                    failure_threshold: 3,
                    open_until: None,
                    cooldown_until: None,
                },
            ),
            (
                2,
                crate::gateway::GatewayProviderCircuitStatus {
                    provider_id: 2,
                    state: "OPEN".to_string(),
                    failure_count: 3,
                    failure_threshold: 3,
                    open_until: Some(2_000),
                    cooldown_until: None,
                },
            ),
            (
                3,
                crate::gateway::GatewayProviderCircuitStatus {
                    provider_id: 3,
                    state: "CLOSED".to_string(),
                    failure_count: 0,
                    failure_threshold: 3,
                    open_until: None,
                    cooldown_until: None,
                },
            ),
        ]);

        let selected = first_eligible_provider(&providers, &limited, &statuses, 1_000)
            .expect("eligible provider");
        assert_eq!(selected.0.id, 3);
    }

    #[test]
    fn provider_projection_marks_spend_and_oauth_limits() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db =
            crate::db::init_for_tests(&dir.path().join("observer-limits.db")).expect("init db");
        let spend_limited = insert_observer_provider(&db, "Spend limited", Some(0.0));
        let oauth_limited = insert_observer_provider(&db, "OAuth limited", None);
        let ready = insert_observer_provider(&db, "Ready", None);
        crate::providers::default_route_set_order(
            &db,
            "codex",
            vec![spend_limited, oauth_limited, ready],
        )
        .expect("set route order");
        let conn = db.open_connection().expect("open db");
        conn.execute(
            "UPDATE providers SET auth_mode = 'oauth' WHERE id IN (?1, ?2)",
            rusqlite::params![oauth_limited, ready],
        )
        .expect("mark oauth provider");
        drop(conn);
        crate::domain::provider_oauth_limits::save_snapshot(
            &db,
            crate::domain::provider_oauth_limits::OAuthLimitSnapshotInput {
                provider_id: oauth_limited,
                limit_short_label: None,
                limit_5h_text: Some("0"),
                limit_weekly_text: None,
                limit_5h_reset_at: Some(2_000),
                limit_weekly_reset_at: None,
                reset_credit_available_count: None,
            },
        )
        .expect("save exhausted oauth snapshot");

        let (providers, limited) =
            load_provider_candidates(&db, "codex", 1_000).expect("load candidates");
        assert_eq!(
            providers
                .iter()
                .map(|provider| provider.id)
                .collect::<Vec<_>>(),
            vec![spend_limited, oauth_limited, ready]
        );
        assert_eq!(limited, HashSet::from([spend_limited, oauth_limited]));

        let (observed, truncated) =
            load_provider_observations(&db, Some("codex"), 1_000, 6).expect("load observations");
        assert!(!truncated);
        assert_eq!(
            observed
                .iter()
                .map(|provider| provider.id)
                .collect::<Vec<_>>(),
            vec![spend_limited, oauth_limited, ready]
        );
        assert!(observed[0].spend_limited);
        assert!(!observed[0].spend_windows.is_empty());
        assert!(observed[1].oauth_limited);
        assert!(observed[1].oauth_quota.is_some());
        assert!(!observed[2].spend_limited);
        assert_eq!(observed[2].auth_kind, "oauth");
        assert!(!observed[2].oauth_limited);
        assert!(observed[2].oauth_quota.is_none());
    }

    #[test]
    fn provider_eligibility_is_fail_closed_for_observer_display_only() {
        let mut provider = ProviderObservation {
            id: 1,
            cli_key: "codex".to_string(),
            name: "Provider".to_string(),
            enabled: true,
            auth_kind: "api_key".to_string(),
            route_rank: Some(1),
            route_enabled: true,
            spend_limited: false,
            spend_windows: Vec::new(),
            oauth_limited: false,
            oauth_limited_reset_at: None,
            oauth_quota: None,
            availability: None,
            account_usage_target: None,
        };
        assert_eq!(
            provider_eligibility(&provider, None, true, 1_000),
            "unknown"
        );
        assert_eq!(
            provider_eligibility(&provider, None, false, 1_000),
            "gateway_stopped"
        );

        provider.enabled = false;
        assert_eq!(
            provider_eligibility(&provider, None, true, 1_000),
            "provider_disabled"
        );
    }

    #[test]
    fn terminal_route_counts_ignore_gate_only_skips() {
        let hop = |provider_id: i64, provider_name: &str, skipped: bool, attempts: i64| {
            request_logs::RequestLogRouteHop {
                provider_id,
                provider_name: provider_name.to_string(),
                ok: false,
                attempts,
                skipped,
                status: None,
                error_code: None,
                decision: None,
                reason: None,
            }
        };
        let route = vec![
            hop(1, "A", false, 2),
            hop(2, "Limited", true, 9),
            hop(3, "C", false, 1),
        ];
        assert_eq!(effective_terminal_route_counts(&route), (1, 1));

        let skipped_then_sent = vec![
            hop(1, "Limited A", true, 1),
            hop(2, "Open B", true, 1),
            hop(3, "C", false, 1),
        ];
        assert_eq!(effective_terminal_route_counts(&skipped_then_sent), (0, 0));
    }
}
