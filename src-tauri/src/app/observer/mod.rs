//! Authenticated loopback-only observation service for the standalone TUI.

mod descriptor;
mod snapshot;

use aio_observer_protocol::{
    CliScope, ObserverApiError, ObserverApiErrorResponse, ObserverHealthV1,
    ObserverProviderAvailabilityTestResult, ObserverSnapshotV1, OBSERVER_HISTORY_LIMIT_MAX,
    OBSERVER_PROTOCOL_VERSION, OBSERVER_PROVIDER_PROBE_TIMEOUT_MS,
};
use axum::extract::rejection::{PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::Manager;
use tokio::sync::{oneshot, Mutex, OwnedSemaphorePermit, Semaphore};

const OBSERVER_MAX_CONCURRENT_REQUESTS: usize = 2;
const OBSERVER_MAX_CONCURRENT_PROBES: usize = 2;
const OBSERVER_PROBE_TIMEOUT: Duration =
    Duration::from_millis(OBSERVER_PROVIDER_PROBE_TIMEOUT_MS);
const ACTIVE_CACHE_TTL: Duration = Duration::from_millis(400);
const IDLE_CACHE_TTL: Duration = Duration::from_millis(1500);
const DB_QUERY_PERMIT_TIMEOUT: Duration = Duration::from_millis(1600);
const DB_RETRY_INTERVAL: Duration = Duration::from_secs(5);
const STOP_TIMEOUT: Duration = Duration::from_secs(1);

#[derive(Default)]
pub(crate) struct ObserverRuntimeState {
    runtime: Mutex<Option<ObserverRuntime>>,
    starting: AtomicBool,
    stopping: AtomicBool,
}

struct ObserverRuntime {
    shutdown: Option<oneshot::Sender<()>>,
    task: tauri::async_runtime::JoinHandle<()>,
    descriptor_path: PathBuf,
    pid: u32,
    token: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct CacheKey {
    scope: CliScope,
    history_limit: u16,
    include_providers: bool,
}

struct CachedSnapshot {
    created_at: Instant,
    snapshot: ObserverSnapshotV1,
}

#[derive(Clone)]
struct ObserverHttpState {
    app: tauri::AppHandle,
    db: Arc<Mutex<ObserverDbState>>,
    token: Arc<str>,
    limiter: Arc<Semaphore>,
    probe_limiter: Arc<Semaphore>,
    db_query_limiter: Arc<Semaphore>,
    cache: Arc<Mutex<HashMap<CacheKey, CachedSnapshot>>>,
}

#[derive(Default)]
struct ObserverDbState {
    db: Option<crate::db::Db>,
    retry_after: Option<Instant>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotQuery {
    cli: Option<String>,
    history_limit: Option<u16>,
    #[serde(default)]
    include_providers: bool,
}

pub(crate) async fn start_best_effort(app: tauri::AppHandle) {
    let already_starting = match app.try_state::<ObserverRuntimeState>() {
        Some(state) => state.starting.swap(true, Ordering::AcqRel),
        None => {
            tracing::warn!("local observer runtime state is unavailable");
            return;
        }
    };
    if already_starting {
        return;
    }
    let result = start(app.clone()).await;
    if let Some(state) = app.try_state::<ObserverRuntimeState>() {
        state.starting.store(false, Ordering::Release);
    }
    if let Err(err) = result {
        tracing::warn!(error = %err.code(), "local observer service is unavailable");
    }
}

async fn start(app: tauri::AppHandle) -> crate::shared::error::AppResult<()> {
    let state = app
        .try_state::<ObserverRuntimeState>()
        .ok_or_else(|| "observer runtime state is unavailable".to_string())?;
    if state.stopping.load(Ordering::Acquire) || state.runtime.lock().await.is_some() {
        return Ok(());
    }

    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .map_err(|err| format!("failed to bind observer loopback listener: {err}"))?;
    let port = listener
        .local_addr()
        .map_err(|err| format!("failed to read observer listener address: {err}"))?
        .port();
    let started_at_ms = crate::shared::time::now_unix_millis();
    let descriptor = descriptor::new_descriptor(port, env!("CARGO_PKG_VERSION"), started_at_ms);
    let descriptor_path = descriptor::path(&app)?;
    let descriptor_to_write = descriptor.clone();
    let write_path = descriptor_path.clone();
    crate::blocking::run("observer_descriptor_write", move || {
        descriptor::write(&write_path, &descriptor_to_write)
    })
    .await?;

    let http_state = ObserverHttpState {
        app: app.clone(),
        db: Arc::new(Mutex::new(ObserverDbState::default())),
        token: Arc::from(descriptor.token.as_str()),
        limiter: Arc::new(Semaphore::new(OBSERVER_MAX_CONCURRENT_REQUESTS)),
        probe_limiter: Arc::new(Semaphore::new(OBSERVER_MAX_CONCURRENT_PROBES)),
        db_query_limiter: Arc::new(Semaphore::new(1)),
        cache: Arc::new(Mutex::new(HashMap::new())),
    };
    let router = Router::new()
        .route("/api/observer/v1/health", get(health))
        .route("/api/observer/v1/snapshot", get(snapshot_handler))
        .route(
            "/api/observer/v1/providers/:provider_id/test-availability",
            post(provider_test_availability_handler),
        )
        .with_state(http_state);
    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    let mut runtime = state.runtime.lock().await;
    if state.stopping.load(Ordering::Acquire) {
        drop(runtime);
        let _ = crate::blocking::run(
            "observer_descriptor_remove_cancelled_start",
            move || -> crate::shared::error::AppResult<()> {
                descriptor::remove_if_owned(&descriptor_path, descriptor.pid, &descriptor.token);
                Ok(())
            },
        )
        .await;
        return Ok(());
    }
    if runtime.is_some() {
        return Ok(());
    }

    let task = tauri::async_runtime::spawn(async move {
        let server = axum::serve(listener, router).with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        });
        if server.await.is_err() {
            tracing::warn!("local observer server stopped unexpectedly");
        }
    });
    tracing::info!(port, "local observer service started");
    *runtime = Some(ObserverRuntime {
        shutdown: Some(shutdown_tx),
        task,
        descriptor_path,
        pid: descriptor.pid,
        token: descriptor.token,
    });
    Ok(())
}

pub(crate) async fn stop_best_effort(app: &tauri::AppHandle) {
    let Some(state) = app.try_state::<ObserverRuntimeState>() else {
        return;
    };
    state.stopping.store(true, Ordering::Release);
    let runtime = state.runtime.lock().await.take();
    let Some(mut runtime) = runtime else {
        return;
    };
    if let Some(shutdown) = runtime.shutdown.take() {
        let _ = shutdown.send(());
    }
    if tokio::time::timeout(STOP_TIMEOUT, &mut runtime.task)
        .await
        .is_err()
    {
        runtime.task.abort();
    }
    let path = runtime.descriptor_path;
    let pid = runtime.pid;
    let token = runtime.token;
    let _ = crate::blocking::run(
        "observer_descriptor_remove",
        move || -> crate::shared::error::AppResult<()> {
            descriptor::remove_if_owned(&path, pid, &token);
            Ok(())
        },
    )
    .await;
}

async fn health(State(state): State<ObserverHttpState>, headers: HeaderMap) -> Response {
    if !authorized(&headers, &state.token) {
        return api_error(StatusCode::UNAUTHORIZED, "OBS_UNAUTHORIZED", "unauthorized");
    }
    secured(
        Json(ObserverHealthV1 {
            protocol_version: OBSERVER_PROTOCOL_VERSION,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            pid: std::process::id(),
        })
        .into_response(),
    )
}

async fn snapshot_handler(
    State(state): State<ObserverHttpState>,
    headers: HeaderMap,
    query: Result<Query<SnapshotQuery>, QueryRejection>,
) -> Response {
    if !authorized(&headers, &state.token) {
        return api_error(StatusCode::UNAUTHORIZED, "OBS_UNAUTHORIZED", "unauthorized");
    }
    let Query(query) = match query {
        Ok(query) => query,
        Err(_) => {
            return api_error(
                StatusCode::BAD_REQUEST,
                "OBS_INVALID_INPUT",
                "invalid query",
            )
        }
    };
    let scope = match query.cli.as_deref().and_then(CliScope::parse) {
        Some(scope) => scope,
        None => {
            return api_error(
                StatusCode::BAD_REQUEST,
                "OBS_INVALID_INPUT",
                "invalid cli scope",
            )
        }
    };
    let history_limit = query.history_limit.unwrap_or(OBSERVER_HISTORY_LIMIT_MAX);
    if history_limit > OBSERVER_HISTORY_LIMIT_MAX {
        return api_error(
            StatusCode::BAD_REQUEST,
            "OBS_INVALID_INPUT",
            "invalid history limit",
        );
    }
    let _permit = match state.limiter.clone().try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => {
            return api_error(
                StatusCode::TOO_MANY_REQUESTS,
                "OBS_BUSY",
                "observer is busy",
            )
        }
    };

    let key = CacheKey {
        scope,
        history_limit,
        include_providers: query.include_providers,
    };
    if let Some(snapshot) = cached_snapshot(&state, key).await {
        return secured(Json(snapshot).into_response());
    }
    let db = read_only_db(&state).await;
    let db_query_permit = if db.is_some() {
        match wait_for_db_query_permit(state.db_query_limiter.clone()).await {
            Some(permit) => Some(permit),
            None => {
                return api_error(
                    StatusCode::TOO_MANY_REQUESTS,
                    "OBS_BUSY",
                    "observer is busy",
                )
            }
        }
    } else {
        None
    };
    if let Some(snapshot) = cached_snapshot(&state, key).await {
        return secured(Json(snapshot).into_response());
    }
    let snapshot = snapshot::build_snapshot(
        &state.app,
        db.as_ref(),
        db_query_permit,
        scope,
        usize::from(history_limit),
        query.include_providers,
    )
    .await;
    state.cache.lock().await.insert(
        key,
        CachedSnapshot {
            created_at: Instant::now(),
            snapshot: snapshot.clone(),
        },
    );
    secured(Json(snapshot).into_response())
}

async fn provider_test_availability_handler(
    State(state): State<ObserverHttpState>,
    headers: HeaderMap,
    provider_id: Result<Path<i64>, PathRejection>,
) -> Response {
    if !authorized(&headers, &state.token) {
        return api_error(StatusCode::UNAUTHORIZED, "OBS_UNAUTHORIZED", "unauthorized");
    }
    let provider_id = match provider_id {
        Ok(Path(provider_id)) if provider_id > 0 => provider_id,
        _ => {
            return api_error(
                StatusCode::BAD_REQUEST,
                "OBS_INVALID_INPUT",
                "invalid provider id",
            )
        }
    };
    let _permit = match state.probe_limiter.clone().try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => {
            return api_error(
                StatusCode::TOO_MANY_REQUESTS,
                "OBS_PROBE_BUSY",
                "provider probe is busy",
            )
        }
    };
    let Some(db_state) = state.app.try_state::<crate::app_state::DbInitState>() else {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "OBS_UNAVAILABLE",
            "provider probe is unavailable",
        );
    };
    let db = match crate::app_state::ensure_db_ready(state.app.clone(), db_state.inner()).await {
        Ok(db) => db,
        Err(_) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "OBS_UNAVAILABLE",
                "provider probe is unavailable",
            )
        }
    };
    let result = match tokio::time::timeout(
        OBSERVER_PROBE_TIMEOUT,
        crate::domain::provider_availability::test_provider_availability(
            &state.app,
            db,
            provider_id,
        ),
    )
    .await
    {
        Ok(Ok(result)) => result,
        Ok(Err(error)) if error.code() == "DB_NOT_FOUND" => {
            return api_error(
                StatusCode::NOT_FOUND,
                "OBS_PROVIDER_NOT_FOUND",
                "provider not found",
            )
        }
        Ok(Err(_)) => {
            return api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "OBS_PROBE_FAILED",
                "provider probe failed",
            )
        }
        Err(_) => {
            return api_error(
                StatusCode::GATEWAY_TIMEOUT,
                "OBS_PROBE_TIMEOUT",
                "provider probe timed out",
            )
        }
    };
    let error = (!result.ok).then(|| match result.status {
        Some(401 | 403) => "认证失败".to_string(),
        Some(status) if status >= 500 => "上游服务异常".to_string(),
        Some(_) => "供应商响应不可用".to_string(),
        None => "连接或请求失败".to_string(),
    });
    secured(
        Json(ObserverProviderAvailabilityTestResult {
            ok: result.ok,
            provider_id: result.provider_id,
            provider_name: bounded_observer_text(&result.provider_name, 128),
            base_url: observer_probe_base_url(&result.base_url),
            status: result.status,
            latency_ms: result.latency_ms,
            error,
            response_preview: result
                .response_preview
                .as_deref()
                .map(|value| bounded_observer_text(value, 500)),
        })
        .into_response(),
    )
}

fn observer_probe_base_url(value: &str) -> String {
    let Ok(mut url) = reqwest::Url::parse(value) else {
        return String::new();
    };
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.set_query(None);
    url.set_fragment(None);
    bounded_observer_text(url.as_str(), 2_048)
}

fn bounded_observer_text(value: &str, max_chars: usize) -> String {
    value
        .chars()
        .filter(|character| {
            !character.is_control()
                && !matches!(
                    *character,
                    '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}'
                )
        })
        .take(max_chars)
        .collect()
}

async fn wait_for_db_query_permit(limiter: Arc<Semaphore>) -> Option<OwnedSemaphorePermit> {
    tokio::time::timeout(DB_QUERY_PERMIT_TIMEOUT, limiter.acquire_owned())
        .await
        .ok()?
        .ok()
}

async fn read_only_db(state: &ObserverHttpState) -> Option<crate::db::Db> {
    {
        let db_state = state.db.lock().await;
        if let Some(db) = db_state.db.as_ref() {
            return Some(db.clone());
        }
        if db_state
            .retry_after
            .is_some_and(|retry_after| Instant::now() < retry_after)
        {
            return None;
        }
    }

    let app = state.app.clone();
    match crate::blocking::run("observer_read_only_db_open", move || {
        crate::db::open_read_only(&app)
    })
    .await
    {
        Ok(db) => {
            let mut db_state = state.db.lock().await;
            let db = db_state.db.get_or_insert(db).clone();
            db_state.retry_after = None;
            Some(db)
        }
        Err(err) => {
            let mut db_state = state.db.lock().await;
            db_state.retry_after = Some(Instant::now() + DB_RETRY_INTERVAL);
            tracing::warn!(error = %err.code(), "observer read-only database is unavailable");
            None
        }
    }
}

async fn cached_snapshot(state: &ObserverHttpState, key: CacheKey) -> Option<ObserverSnapshotV1> {
    let cache = state.cache.lock().await;
    let cached = cache.get(&key)?;
    let active = cached.snapshot.active_inference_count > 0
        || cached
            .snapshot
            .active_requests
            .value
            .as_ref()
            .is_some_and(|items| !items.is_empty());
    let ttl = if active {
        ACTIVE_CACHE_TTL
    } else {
        IDLE_CACHE_TTL
    };
    (cached.created_at.elapsed() < ttl).then(|| cached.snapshot.clone())
}

fn authorized(headers: &HeaderMap, expected: &str) -> bool {
    let Some(value) = headers.get(header::AUTHORIZATION) else {
        return false;
    };
    let Ok(value) = value.to_str() else {
        return false;
    };
    let Some(candidate) = value.strip_prefix("Bearer ") else {
        return false;
    };
    constant_time_eq(candidate.as_bytes(), expected.as_bytes())
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (*left ^ *right)
        })
        == 0
}

fn api_error(status: StatusCode, code: &str, message: &str) -> Response {
    secured(
        (
            status,
            Json(ObserverApiErrorResponse {
                error: ObserverApiError {
                    code: code.to_string(),
                    message: message.to_string(),
                },
            }),
        )
            .into_response(),
    )
}

fn secured(mut response: Response) -> Response {
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, max-age=0"),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bearer_comparison_requires_exact_bytes() {
        assert!(constant_time_eq(b"same", b"same"));
        assert!(!constant_time_eq(b"same", b"diff"));
        assert!(!constant_time_eq(b"short", b"longer"));
    }

    #[test]
    fn authorization_rejects_missing_and_wrong_schemes() {
        let mut headers = HeaderMap::new();
        assert!(!authorized(&headers, "token"));
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Basic token"),
        );
        assert!(!authorized(&headers, "token"));
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer token"),
        );
        assert!(authorized(&headers, "token"));
    }

    #[test]
    fn provider_probe_output_strips_url_credentials_and_bounds_text() {
        assert_eq!(
            observer_probe_base_url(
                "https://user:secret@example.com/v1/responses?api_key=hidden#fragment"
            ),
            "https://example.com/v1/responses"
        );
        assert_eq!(observer_probe_base_url("not a url"), "");

        let unsafe_text = format!("ok\nsecret\u{202e}{}", "x".repeat(600));
        let bounded = bounded_observer_text(&unsafe_text, 10);
        assert_eq!(bounded.chars().count(), 10);
        assert!(!bounded.chars().any(char::is_control));
        assert!(!bounded.contains('\u{202e}'));
    }

    #[test]
    fn provider_probe_timeout_matches_the_protocol_contract() {
        assert_eq!(
            OBSERVER_PROBE_TIMEOUT,
            Duration::from_millis(aio_observer_protocol::OBSERVER_PROVIDER_PROBE_TIMEOUT_MS)
        );
    }

    #[tokio::test]
    async fn db_query_permit_waits_for_the_active_reader() {
        let limiter = Arc::new(Semaphore::new(1));
        let held = limiter.clone().acquire_owned().await.expect("first permit");
        let release = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            drop(held);
        });

        assert!(wait_for_db_query_permit(limiter).await.is_some());
        release.await.expect("release task");
    }
}
