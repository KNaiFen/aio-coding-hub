//! Usage: Request logs and trace detail related Tauri commands.

use crate::app_state::{ensure_db_ready, DbInitState};
use crate::app::request_log_snapshot_state::{
    RequestLogSnapshotSlice, RequestLogSnapshotState, MAX_SNAPSHOT_MEMBERSHIPS,
};
use crate::commands::limit::normalize_limit;
use crate::gateway_runtime_access::app_gateway_active_requests_snapshot;
use crate::{blocking, request_attempt_logs, request_logs};

const REQUEST_LOGS_DEFAULT_LIMIT: u32 = 50;
const REQUEST_LOGS_MAX_LIMIT: u32 = 500;
const REQUEST_LOGS_PAGE_DEFAULT_LIMIT: i64 = 50;
const REQUEST_LOGS_PAGE_MAX_LIMIT: i64 = 200;
const REQUEST_ATTEMPT_LOGS_MAX_LIMIT: u32 = 200;

fn request_logs_limit(limit: Option<u32>) -> usize {
    normalize_limit(limit, REQUEST_LOGS_DEFAULT_LIMIT, 1, REQUEST_LOGS_MAX_LIMIT)
}

fn request_attempt_logs_limit(limit: Option<u32>) -> usize {
    normalize_limit(
        limit,
        REQUEST_LOGS_DEFAULT_LIMIT,
        1,
        REQUEST_ATTEMPT_LOGS_MAX_LIMIT,
    )
}

fn request_logs_page_limit(limit: Option<i64>) -> Result<usize, String> {
    let limit = limit.unwrap_or(REQUEST_LOGS_PAGE_DEFAULT_LIMIT);
    if !(1..=REQUEST_LOGS_PAGE_MAX_LIMIT).contains(&limit) {
        return Err(format!(
            "SEC_INVALID_INPUT: request logs page limit must be between 1 and {REQUEST_LOGS_PAGE_MAX_LIMIT}"
        ));
    }
    Ok(limit as usize)
}

fn request_logs_snapshot_page_number(page: i64) -> Result<usize, String> {
    if page < 1 {
        return Err("SEC_INVALID_INPUT: request logs page must be at least 1".to_string());
    }
    usize::try_from(page)
        .map_err(|_| "SEC_INVALID_INPUT: request logs page is out of range".to_string())
}

fn request_logs_snapshot_filter_fingerprint(
    filters: &request_logs::RequestLogPageFilters,
) -> Result<String, String> {
    serde_json::to_string(filters)
        .map_err(|error| format!("SYSTEM_ERROR: failed to encode request logs filters: {error}"))
}

fn request_logs_snapshot_page_response(
    slice: RequestLogSnapshotSlice,
    items: Vec<request_logs::RequestLogSummary>,
) -> Result<request_logs::RequestLogSnapshotPage, String> {
    Ok(request_logs::RequestLogSnapshotPage {
        items,
        snapshot_id: slice.snapshot_id,
        total_count: i64::try_from(slice.total_count)
            .map_err(|_| "SYSTEM_ERROR: request logs snapshot count is too large".to_string())?,
        total_pages: i64::try_from(slice.total_pages)
            .map_err(|_| "SYSTEM_ERROR: request logs snapshot page count is too large".to_string())?,
        page: i64::try_from(slice.page)
            .map_err(|_| "SYSTEM_ERROR: request logs snapshot page is too large".to_string())?,
        page_size: i64::try_from(slice.page_size)
            .map_err(|_| "SYSTEM_ERROR: request logs snapshot page size is too large".to_string())?,
        expires_at_ms: slice.expires_at_ms,
    })
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn request_logs_list(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
    limit: Option<u32>,
) -> Result<Vec<request_logs::RequestLogSummary>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let limit = request_logs_limit(limit);
    blocking::run("request_logs_list", move || {
        request_logs::list_recent(&db, &cli_key, limit)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn request_logs_list_all(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    limit: Option<u32>,
) -> Result<Vec<request_logs::RequestLogSummary>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let limit = request_logs_limit(limit);
    blocking::run("request_logs_list_all", move || {
        request_logs::list_recent_all(&db, limit)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn request_logs_page_all(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    filters: request_logs::RequestLogPageFilters,
    cursor: Option<String>,
    limit: Option<i64>,
) -> Result<request_logs::RequestLogPage, String> {
    let limit = request_logs_page_limit(limit)?;
    let active_trace_ids = app_gateway_active_requests_snapshot(&app)
        .into_iter()
        .map(|item| item.trace_id)
        .collect::<Vec<_>>();
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("request_logs_page_all", move || {
        request_logs::page_all_excluding_traces(
            &db,
            &filters,
            cursor.as_deref(),
            limit,
            &active_trace_ids,
        )
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn request_logs_snapshot_page_all(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    snapshot_state: tauri::State<'_, RequestLogSnapshotState>,
    filters: request_logs::RequestLogPageFilters,
    snapshot_id: Option<String>,
    page: i64,
    limit: Option<i64>,
) -> Result<request_logs::RequestLogSnapshotPage, String> {
    let page_size = request_logs_page_limit(limit)?;
    let page = request_logs_snapshot_page_number(page)?;
    let filter_fingerprint = request_logs_snapshot_filter_fingerprint(&filters)?;
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;

    let slice = match snapshot_id.as_deref() {
        Some(snapshot_id) => snapshot_state.page(snapshot_id, &filter_fingerprint, page_size, page)?,
        None => {
            let active_trace_ids = app_gateway_active_requests_snapshot(&app)
                .into_iter()
                .map(|item| item.trace_id)
                .collect::<Vec<_>>();
            let snapshot_filters = filters.clone();
            let snapshot_db = db.clone();
            let ids = blocking::run("request_logs_snapshot_membership", move || {
                request_logs::snapshot_membership_excluding_traces(
                    &snapshot_db,
                    &snapshot_filters,
                    &active_trace_ids,
                    MAX_SNAPSHOT_MEMBERSHIPS,
                )
            })
            .await
            .map_err(Into::into)?;
            snapshot_state.create(filter_fingerprint, page_size, ids, page)?
        }
    };

    let snapshot_id = slice.snapshot_id.clone();
    let snapshot_ids = slice.ids.clone();
    let items = blocking::run("request_logs_snapshot_page", move || {
        request_logs::summaries_by_ids(&db, &snapshot_ids)
    })
    .await
    .map_err(|error| {
        let message = error.to_string();
        if message.starts_with("REQUEST_LOG_SNAPSHOT_EXPIRED:") {
            snapshot_state.invalidate(&snapshot_id);
        }
        message
    })?;

    request_logs_snapshot_page_response(slice, items)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn request_logs_list_after_id(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
    after_id: i64,
    limit: Option<u32>,
) -> Result<Vec<request_logs::RequestLogSummary>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let limit = request_logs_limit(limit);
    blocking::run("request_logs_list_after_id", move || {
        request_logs::list_after_id(&db, &cli_key, after_id, limit)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn request_logs_list_after_id_all(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    after_id: i64,
    limit: Option<u32>,
) -> Result<Vec<request_logs::RequestLogSummary>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let limit = request_logs_limit(limit);
    blocking::run("request_logs_list_after_id_all", move || {
        request_logs::list_after_id_all(&db, after_id, limit)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn request_log_get(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    log_id: i64,
) -> Result<request_logs::RequestLogDetail, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("request_log_get", move || {
        request_logs::get_by_id(&db, log_id)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn request_log_get_by_trace_id(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    trace_id: String,
) -> Result<Option<request_logs::RequestLogDetail>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    blocking::run("request_log_get_by_trace_id", move || {
        request_logs::get_by_trace_id(&db, &trace_id)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn request_attempt_logs_by_trace_id(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    trace_id: String,
    limit: Option<u32>,
) -> Result<Vec<request_attempt_logs::RequestAttemptLog>, String> {
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let limit = request_attempt_logs_limit(limit);
    blocking::run("request_attempt_logs_by_trace_id", move || {
        request_attempt_logs::list_by_trace_id(&db, &trace_id, limit)
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn active_request_logs_snapshot(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
) -> Result<Vec<crate::gateway::active_requests::ActiveRequestSnapshotItem>, String> {
    let snapshot = app_gateway_active_requests_snapshot(&app);
    if snapshot.is_empty() {
        return Ok(snapshot);
    }

    let trace_ids = snapshot
        .iter()
        .map(|item| item.trace_id.clone())
        .collect::<Vec<_>>();
    let db = ensure_db_ready(app, db_state.inner()).await?;
    let terminal_trace_ids =
        blocking::run("active_request_logs_snapshot_terminal_filter", move || {
            request_logs::terminal_trace_ids(&db, &trace_ids)
        })
        .await?;

    Ok(snapshot
        .into_iter()
        .filter(|item| !terminal_trace_ids.contains(item.trace_id.as_str()))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::{
        request_attempt_logs_limit, request_logs_limit, request_logs_page_limit,
        request_logs_snapshot_page_number,
    };

    #[test]
    fn request_logs_limit_uses_default_and_clamps() {
        assert_eq!(request_logs_limit(None), 50);
        assert_eq!(request_logs_limit(Some(0)), 1);
        assert_eq!(request_logs_limit(Some(999)), 500);
        assert_eq!(request_logs_limit(Some(200)), 200);
    }

    #[test]
    fn request_attempt_logs_limit_uses_default_and_clamps() {
        assert_eq!(request_attempt_logs_limit(None), 50);
        assert_eq!(request_attempt_logs_limit(Some(0)), 1);
        assert_eq!(request_attempt_logs_limit(Some(999)), 200);
        assert_eq!(request_attempt_logs_limit(Some(88)), 88);
    }

    #[test]
    fn request_logs_page_limit_defaults_and_rejects_out_of_range_values() {
        assert_eq!(request_logs_page_limit(None), Ok(50));
        assert_eq!(request_logs_page_limit(Some(1)), Ok(1));
        assert_eq!(request_logs_page_limit(Some(200)), Ok(200));
        for limit in [-1, 0, 201, i64::MAX] {
            let error = request_logs_page_limit(Some(limit)).unwrap_err();
            assert!(error.starts_with("SEC_INVALID_INPUT:"));
        }
    }

    #[test]
    fn request_logs_snapshot_page_number_rejects_non_positive_values() {
        assert_eq!(request_logs_snapshot_page_number(1), Ok(1));
        assert_eq!(request_logs_snapshot_page_number(9), Ok(9));
        for page in [-1, 0] {
            assert!(request_logs_snapshot_page_number(page)
                .unwrap_err()
                .starts_with("SEC_INVALID_INPUT:"));
        }
    }
}
