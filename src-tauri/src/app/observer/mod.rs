//! Authenticated loopback-only observation service for the standalone TUI.

mod descriptor;
mod snapshot;

use aio_observer_protocol::{
    CliScope, ObserverApiError, ObserverApiErrorResponse, ObserverHealthV1, ObserverSnapshotV1,
    OBSERVER_HISTORY_LIMIT_MAX, OBSERVER_PROTOCOL_VERSION,
};
use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::Manager;
use tokio::sync::{oneshot, Mutex, Semaphore};

const OBSERVER_MAX_CONCURRENT_REQUESTS: usize = 2;
const ACTIVE_CACHE_TTL: Duration = Duration::from_millis(400);
const IDLE_CACHE_TTL: Duration = Duration::from_millis(1500);
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
        db_query_limiter: Arc::new(Semaphore::new(1)),
        cache: Arc::new(Mutex::new(HashMap::new())),
    };
    let router = Router::new()
        .route("/api/observer/v1/health", get(health))
        .route("/api/observer/v1/snapshot", get(snapshot_handler))
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
    };
    if let Some(snapshot) = cached_snapshot(&state, key).await {
        return secured(Json(snapshot).into_response());
    }
    let db = read_only_db(&state).await;
    let db_query_permit = state.db_query_limiter.clone().try_acquire_owned().ok();
    let snapshot = snapshot::build_snapshot(
        &state.app,
        db.as_ref(),
        db_query_permit,
        scope,
        usize::from(history_limit),
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
}
