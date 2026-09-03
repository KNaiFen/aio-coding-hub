//! Usage: Request log queries and attempts decoding.

use crate::db;
use crate::shared::error::db_err;
use base64::Engine as _;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use super::costing::cost_usd_from_femto;
use super::types::{
    RequestLogDetail, RequestLogErrorScope, RequestLogPage, RequestLogPageFilters,
    RequestLogRouteHop, RequestLogStatusFilter, RequestLogStatusFilterOp, RequestLogSummary,
};

pub const OBSERVER_TRACE_ID_QUERY_LIMIT: usize = 400;

const CLAUDE_VISIBLE_LOG_PATH: &str = "/v1/messages";
const CLAUDE_VISIBLE_LOG_CONDITION: &str = "(cli_key != 'claude' OR path = '/v1/messages')";
const OBSERVER_MODEL_INFERENCE_CONDITION: &str = r#"
lower(trim(method)) = 'post'
AND (
  (
    lower(trim(cli_key)) = 'claude'
    AND lower(rtrim(substr(trim(path), 1, instr(trim(path) || '?', '?') - 1), '/'))
      IN ('/v1/messages', '/messages')
  )
  OR (
    lower(trim(cli_key)) = 'codex'
    AND lower(rtrim(substr(trim(path), 1, instr(trim(path) || '?', '?') - 1), '/'))
      IN (
        '/responses',
        '/v1/responses',
        '/v1/codex/responses',
        '/responses/compact',
        '/v1/responses/compact',
        '/v1/codex/responses/compact'
      )
  )
  OR (
    lower(trim(cli_key)) = 'grok'
    AND lower(rtrim(substr(trim(path), 1, instr(trim(path) || '?', '?') - 1), '/'))
      IN ('/chat/completions', '/v1/chat/completions', '/responses', '/v1/responses')
  )
  OR (
    lower(trim(cli_key)) = 'gemini'
    AND (
      lower(rtrim(substr(trim(path), 1, instr(trim(path) || '?', '?') - 1), '/'))
        LIKE '%:generatecontent'
      OR lower(rtrim(substr(trim(path), 1, instr(trim(path) || '?', '?') - 1), '/'))
        LIKE '%:streamgeneratecontent'
    )
  )
)
"#;
const REQUEST_LOG_PAGE_CURSOR_VERSION: u8 = 1;
const REQUEST_LOG_PAGE_CURSOR_MAX_BYTES: usize = 512;
const REQUEST_LOG_PAGE_MAX_LIMIT: usize = 200;
const REQUEST_LOG_PAGE_ERROR_CODE_FILTER_MAX_BYTES: usize = 256;
const REQUEST_LOG_PAGE_METHOD_PATH_FILTER_MAX_BYTES: usize = 512;
const REQUEST_LOG_INTERRUPTED_CONDITION: &str = r#"
(
  (status IS NULL AND NULLIF(trim(COALESCE(error_code, '')), '') IS NULL)
  OR status = 499
  OR trim(COALESCE(error_code, '')) IN (
    'GW_REQUEST_ABORTED',
    'GW_STREAM_ABORTED',
    'GW_REQUEST_INTERRUPTED_BY_RESTART',
    'GW_REQUEST_INTERRUPTED_BY_GATEWAY_STOP'
  )
)
"#;
const REQUEST_LOG_STREAM_INTERNAL_ERROR_EXISTS: &str = r#"
EXISTS (
  SELECT 1
  FROM json_each(
    CASE
      WHEN json_valid(COALESCE(attempts_json, '[]')) THEN attempts_json
      ELSE '[]'
    END
  ) AS attempt
  WHERE json_type(
          CASE WHEN attempt.type = 'object' THEN attempt.value ELSE '{}' END,
          '$.stream_internal_error'
        ) IS NOT NULL
    AND json_type(
          CASE WHEN attempt.type = 'object' THEN attempt.value ELSE '{}' END,
          '$.stream_internal_error'
        ) != 'null'
)
"#;
/// Common SELECT fields for request_logs queries (summary view).
const REQUEST_LOG_SUMMARY_FIELDS: &str = "
  id,
  trace_id,
  cli_key,
  session_id,
  method,
  path,
  excluded_from_stats,
  special_settings_json,
  requested_model,
  status,
  error_code,
  duration_ms,
  ttfb_ms,
  visible_ttfb_ms,
  upstream_stream_duration_ms,
  upstream_stream_timing_version,
  final_upstream_attempt_duration_ms,
  final_upstream_attempt_timing_version,
  estimated_final_upstream_attempt_duration_ms,
  attempts_json,
  input_tokens,
  output_tokens,
  total_tokens,
  cache_read_input_tokens,
  cache_creation_input_tokens,
  cache_creation_5m_input_tokens,
  cache_creation_1h_input_tokens,
  cost_usd_femto,
  cost_multiplier,
  created_at_ms,
  last_activity_ms,
  activity_details_json,
  created_at,
  provider_chain_json,
  error_details_json
";

/// Common SELECT fields for request_logs queries (detail view).
const REQUEST_LOG_DETAIL_FIELDS: &str = "
  id,
  trace_id,
  cli_key,
  session_id,
  method,
  path,
  query,
  excluded_from_stats,
  special_settings_json,
  status,
  error_code,
  duration_ms,
  ttfb_ms,
  visible_ttfb_ms,
  upstream_stream_duration_ms,
  upstream_stream_timing_version,
  final_upstream_attempt_duration_ms,
  final_upstream_attempt_timing_version,
  estimated_final_upstream_attempt_duration_ms,
  attempts_json,
  input_tokens,
  output_tokens,
  total_tokens,
  cache_read_input_tokens,
  cache_creation_input_tokens,
  cache_creation_5m_input_tokens,
  cache_creation_1h_input_tokens,
  usage_json,
  requested_model,
  cost_usd_femto,
  cost_multiplier,
  created_at_ms,
  last_activity_ms,
  activity_details_json,
  created_at,
  provider_chain_json,
  error_details_json
";

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RequestLogPageCursor {
    v: u8,
    created_at_ms: i64,
    id: i64,
}

#[derive(Debug)]
struct ValidatedPageFilters<'a> {
    cli_key: Option<&'a str>,
    status: Option<&'a RequestLogStatusFilter>,
    error_code_contains: Option<&'a str>,
    method_path_contains: Option<&'a str>,
    error_scope: RequestLogErrorScope,
    created_at_ms_from: Option<i64>,
    created_at_ms_to: Option<i64>,
}

pub(super) fn validate_cli_key(cli_key: &str) -> Result<(), String> {
    crate::shared::cli_key::validate_cli_key(cli_key)?;
    Ok(())
}

fn invalid_page_input(message: &str) -> crate::shared::error::AppError {
    crate::shared::error::AppError::new("SEC_INVALID_INPUT", message)
}

fn normalize_contains_filter<'a>(
    value: Option<&'a str>,
    field: &str,
    max_bytes: usize,
) -> crate::shared::error::AppResult<Option<&'a str>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.len() > max_bytes {
        return Err(invalid_page_input(&format!(
            "{field} exceeds maximum length of {max_bytes} UTF-8 bytes"
        )));
    }
    let trimmed = value.trim();
    Ok((!trimmed.is_empty()).then_some(trimmed))
}

fn validate_page_filters(
    filters: &RequestLogPageFilters,
) -> crate::shared::error::AppResult<ValidatedPageFilters<'_>> {
    if let Some(cli_key) = filters.cli_key.as_deref() {
        crate::shared::cli_key::validate_cli_key(cli_key)?;
    }
    if let Some(status) = filters.status.as_ref() {
        if !(0..=999).contains(&status.value) {
            return Err(invalid_page_input(
                "status filter value must be between 0 and 999",
            ));
        }
    }
    if let Some(from) = filters.created_at_ms_from {
        if from < 0 {
            return Err(invalid_page_input(
                "created_at_ms_from must be a non-negative millisecond timestamp",
            ));
        }
    }
    if let Some(to) = filters.created_at_ms_to {
        if to < 0 {
            return Err(invalid_page_input(
                "created_at_ms_to must be a non-negative millisecond timestamp",
            ));
        }
    }
    if let (Some(from), Some(to)) = (filters.created_at_ms_from, filters.created_at_ms_to) {
        if from >= to {
            return Err(invalid_page_input(
                "created_at_ms_from must be earlier than created_at_ms_to",
            ));
        }
    }

    Ok(ValidatedPageFilters {
        cli_key: filters.cli_key.as_deref(),
        status: filters.status.as_ref(),
        error_code_contains: normalize_contains_filter(
            filters.error_code_contains.as_deref(),
            "error_code_contains",
            REQUEST_LOG_PAGE_ERROR_CODE_FILTER_MAX_BYTES,
        )?,
        method_path_contains: normalize_contains_filter(
            filters.method_path_contains.as_deref(),
            "method_path_contains",
            REQUEST_LOG_PAGE_METHOD_PATH_FILTER_MAX_BYTES,
        )?,
        error_scope: filters.error_scope,
        created_at_ms_from: filters.created_at_ms_from,
        created_at_ms_to: filters.created_at_ms_to,
    })
}

fn page_conditions_and_params(
    filters: &RequestLogPageFilters,
    excluded_trace_ids: &[String],
    cursor: Option<&RequestLogPageCursor>,
) -> crate::shared::error::AppResult<(Vec<String>, Vec<rusqlite::types::Value>)> {
    let filters = validate_page_filters(filters)?;
    let mut conditions = vec![CLAUDE_VISIBLE_LOG_CONDITION.to_string()];
    let mut query_params = Vec::<rusqlite::types::Value>::new();

    if let Some(cli_key) = filters.cli_key {
        conditions.push("cli_key = ?".to_string());
        query_params.push(cli_key.to_owned().into());
    }
    if let Some(status) = filters.status {
        let condition = match status.op {
            RequestLogStatusFilterOp::Eq => "status = ?",
            RequestLogStatusFilterOp::Neq => "(status IS NULL OR status != ?)",
            RequestLogStatusFilterOp::Gte => "status >= ?",
            RequestLogStatusFilterOp::Lte => "status <= ?",
        };
        conditions.push(condition.to_string());
        query_params.push(status.value.into());
    }
    if let Some(needle) = filters.error_code_contains {
        conditions.push("instr(lower(COALESCE(error_code, '')), lower(?)) > 0".to_string());
        query_params.push(needle.to_owned().into());
    }
    if let Some(needle) = filters.method_path_contains {
        conditions.push("instr(lower(method || ' ' || path), lower(?)) > 0".to_string());
        query_params.push(needle.to_owned().into());
    }
    match filters.error_scope {
        RequestLogErrorScope::All => {}
        RequestLogErrorScope::AllErrors => conditions.push(format!(
            "NOT ({REQUEST_LOG_INTERRUPTED_CONDITION}) AND \
             ((status IS NOT NULL AND (status < 200 OR status >= 300)) \
              OR NULLIF(trim(COALESCE(error_code, '')), '') IS NOT NULL \
              OR ({REQUEST_LOG_STREAM_INTERNAL_ERROR_EXISTS}))"
        )),
        RequestLogErrorScope::StreamInternalError => {
            conditions.push(format!("({REQUEST_LOG_STREAM_INTERNAL_ERROR_EXISTS})"));
        }
    }
    if let Some(from) = filters.created_at_ms_from {
        conditions.push("created_at_ms >= ?".to_string());
        query_params.push(from.into());
    }
    if let Some(to) = filters.created_at_ms_to {
        conditions.push("created_at_ms < ?".to_string());
        query_params.push(to.into());
    }
    if !excluded_trace_ids.is_empty() {
        if let Ok(encoded_trace_ids) = serde_json::to_string(excluded_trace_ids) {
            conditions.push(
                "trace_id NOT IN (SELECT value FROM json_each(?) WHERE type = 'text')".to_string(),
            );
            query_params.push(encoded_trace_ids.into());
        }
    }
    if let Some(cursor) = cursor {
        conditions.push("(created_at_ms < ? OR (created_at_ms = ? AND id < ?))".to_string());
        query_params.push(cursor.created_at_ms.into());
        query_params.push(cursor.created_at_ms.into());
        query_params.push(cursor.id.into());
    }

    Ok((conditions, query_params))
}

fn decode_page_cursor(
    cursor: Option<&str>,
) -> crate::shared::error::AppResult<Option<RequestLogPageCursor>> {
    let Some(cursor) = cursor else {
        return Ok(None);
    };
    if cursor.len() > REQUEST_LOG_PAGE_CURSOR_MAX_BYTES {
        return Err(invalid_page_input("invalid request logs cursor"));
    }
    let cursor = cursor.trim();
    if cursor.is_empty() {
        return Err(invalid_page_input("invalid request logs cursor"));
    }
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(cursor.as_bytes())
        .map_err(|_| invalid_page_input("invalid request logs cursor"))?;
    if bytes.len() > REQUEST_LOG_PAGE_CURSOR_MAX_BYTES {
        return Err(invalid_page_input("invalid request logs cursor"));
    }
    let decoded: RequestLogPageCursor = serde_json::from_slice(&bytes)
        .map_err(|_| invalid_page_input("invalid request logs cursor"))?;
    if decoded.v != REQUEST_LOG_PAGE_CURSOR_VERSION {
        return Err(invalid_page_input(
            "unsupported request logs cursor version",
        ));
    }
    if decoded.created_at_ms < 0 || decoded.id <= 0 {
        return Err(invalid_page_input("invalid request logs cursor"));
    }
    Ok(Some(decoded))
}

fn encode_page_cursor(row: &RequestLogSummary) -> crate::shared::error::AppResult<String> {
    let bytes = serde_json::to_vec(&RequestLogPageCursor {
        v: REQUEST_LOG_PAGE_CURSOR_VERSION,
        created_at_ms: row.created_at_ms,
        id: row.id,
    })
    .map_err(|e| {
        crate::shared::error::AppError::new(
            "SYSTEM_ERROR",
            format!("failed to encode request logs cursor: {e}"),
        )
    })?;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
}

#[derive(Debug, Deserialize)]
pub(super) struct AttemptRow {
    provider_id: i64,
    provider_name: String,
    outcome: String,
    status: Option<i64>,
    error_code: Option<String>,
    decision: Option<String>,
    reason: Option<String>,
    session_reuse: Option<bool>,
}

pub(super) fn parse_attempts(attempts_json: &str) -> Vec<AttemptRow> {
    serde_json::from_str(attempts_json).unwrap_or_default()
}

pub(super) fn start_provider_from_attempts(attempts: &[AttemptRow]) -> (i64, String) {
    if attempts.iter().all(|a| a.outcome == "skipped") {
        return (0, "Unknown".to_string());
    }

    let first = attempts
        .iter()
        .find(|a| a.outcome != "skipped")
        .or_else(|| attempts.first());

    match first {
        Some(a) => (a.provider_id, a.provider_name.clone()),
        None => (0, "Unknown".to_string()),
    }
}

pub(super) fn final_provider_from_attempts(attempts: &[AttemptRow]) -> (i64, String) {
    if attempts.iter().all(|a| a.outcome == "skipped") {
        return (0, "Unknown".to_string());
    }

    let picked = attempts
        .iter()
        .rev()
        .find(|a| a.outcome == "success")
        .or_else(|| attempts.iter().rev().find(|a| a.outcome != "skipped"))
        .or_else(|| attempts.last());

    match picked {
        Some(a) => (a.provider_id, a.provider_name.clone()),
        None => (0, "Unknown".to_string()),
    }
}

pub(super) fn route_from_attempts(attempts: &[AttemptRow]) -> Vec<RequestLogRouteHop> {
    let mut out: Vec<RequestLogRouteHop> = Vec::new();
    let mut last_provider_id: i64 = 0;
    let mut last_hop_attempt_count: i64 = 0;
    for attempt in attempts {
        if attempt.provider_id <= 0 {
            continue;
        }
        if attempt.provider_id == last_provider_id {
            // 同一 provider 连续尝试，累加计数
            last_hop_attempt_count += 1;
            if let Some(hop) = out.last_mut() {
                hop.attempts = last_hop_attempt_count;
            }
            continue;
        }
        last_provider_id = attempt.provider_id;
        last_hop_attempt_count = 1;

        let skipped = attempt.outcome == "skipped";
        let ok = !skipped
            && attempts
                .iter()
                .any(|row| row.provider_id == attempt.provider_id && row.outcome == "success");

        let picked = if skipped {
            Some(attempt)
        } else if ok {
            attempts
                .iter()
                .find(|row| row.provider_id == attempt.provider_id && row.outcome == "success")
                .or_else(|| {
                    attempts
                        .iter()
                        .rev()
                        .find(|row| row.provider_id == attempt.provider_id)
                })
        } else {
            attempts
                .iter()
                .rev()
                .find(|row| row.provider_id == attempt.provider_id)
        };

        let (status, error_code, decision, reason) = match picked {
            Some(row) => (
                row.status,
                row.error_code.clone(),
                row.decision.clone(),
                row.reason.clone(),
            ),
            None => (None, None, None, None),
        };

        out.push(RequestLogRouteHop {
            provider_id: attempt.provider_id,
            provider_name: attempt.provider_name.clone(),
            ok,
            attempts: 1,
            skipped,
            status,
            error_code,
            decision,
            reason,
        });
    }
    out
}

#[derive(Debug, Clone, Default)]
struct SourceProviderInfo {
    source_provider_id: Option<i64>,
    source_provider_name: Option<String>,
    // Same predicate as the usage-stats SQL: source id present OR cx2cc bridge.
    bridged: bool,
}

fn normalize_source_provider_name(name: Option<String>) -> Option<String> {
    name.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn load_source_provider_info_map(
    conn: &Connection,
    bridge_provider_ids: &[i64],
) -> crate::shared::error::AppResult<HashMap<i64, SourceProviderInfo>> {
    let ids: Vec<i64> = bridge_provider_ids
        .iter()
        .copied()
        .filter(|id| *id > 0)
        .collect();
    if ids.is_empty() {
        return Ok(HashMap::new());
    }

    let placeholders = crate::db::sql_placeholders(ids.len());
    let sql = format!(
        r#"
SELECT
  bridge.id,
  bridge.source_provider_id,
  source.name,
  bridge.bridge_type
FROM providers bridge
LEFT JOIN providers source ON source.id = bridge.source_provider_id
WHERE bridge.id IN ({placeholders})
"#
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| db_err!("failed to prepare provider source query: {e}"))?;
    let mut rows = stmt
        .query(params_from_iter(ids.iter()))
        .map_err(|e| db_err!("failed to query provider sources: {e}"))?;

    let mut out = HashMap::new();
    while let Some(row) = rows
        .next()
        .map_err(|e| db_err!("failed to read provider source row: {e}"))?
    {
        let bridge_id: i64 = row
            .get(0)
            .map_err(|e| db_err!("invalid provider source bridge id: {e}"))?;
        let source_provider_id: Option<i64> = row
            .get(1)
            .map_err(|e| db_err!("invalid provider source id: {e}"))?;
        let source_provider_name: Option<String> = row
            .get(2)
            .map_err(|e| db_err!("invalid provider source name: {e}"))?;
        let bridge_type: Option<String> = row
            .get(3)
            .map_err(|e| db_err!("invalid provider bridge type: {e}"))?;

        out.insert(
            bridge_id,
            SourceProviderInfo {
                source_provider_id,
                source_provider_name: normalize_source_provider_name(source_provider_name),
                bridged: crate::usage_stats::is_bridged_input_semantics(
                    source_provider_id,
                    bridge_type.as_deref(),
                ),
            },
        );
    }

    Ok(out)
}

fn attach_source_provider_info(
    conn: &Connection,
    items: &mut [RequestLogSummary],
) -> crate::shared::error::AppResult<()> {
    let ids: Vec<i64> = items.iter().map(|item| item.final_provider_id).collect();
    let info_by_bridge_id = load_source_provider_info_map(conn, &ids)?;

    for item in items.iter_mut() {
        let mut bridged = false;
        if let Some(info) = info_by_bridge_id.get(&item.final_provider_id) {
            item.final_provider_source_id = info.source_provider_id;
            item.final_provider_source_name = info.source_provider_name.clone();
            bridged = info.bridged;
        }
        let persisted_openai_semantics = super::semantics::resolve_cx2cc_cost_basis(
            item.special_settings_json.as_deref(),
            (item.final_provider_id > 0).then_some(item.final_provider_id),
        )
        .openai_input_semantics_override();
        item.effective_input_tokens = crate::usage_stats::effective_input_tokens_display(
            &item.cli_key,
            persisted_openai_semantics,
            bridged,
            item.input_tokens,
            item.cache_read_input_tokens,
            item.cache_creation_input_tokens,
        );
    }

    Ok(())
}

fn row_to_summary(row: &rusqlite::Row<'_>) -> Result<RequestLogSummary, rusqlite::Error> {
    let attempts_json: String = row.get("attempts_json")?;
    let attempts = parse_attempts(&attempts_json);
    let attempt_count = attempts.len() as i64;
    let (start_provider_id, start_provider_name) = start_provider_from_attempts(&attempts);
    let (final_provider_id, final_provider_name) = final_provider_from_attempts(&attempts);
    let route = route_from_attempts(&attempts);
    // has_failover: 切换过 provider（route 中有多个 hop）。注意 provider_id>0 的
    // skipped attempt 也计入 hop（见 route_includes_skipped_attempts 测试）；前端
    // src/services/gateway/traceRoute.ts 复刻此语义，两侧需保持同步。
    let has_failover = route.len() > 1;
    let session_reuse = attempts
        .iter()
        .any(|row| row.session_reuse.unwrap_or(false));
    let cost_usd = cost_usd_from_femto(row.get("cost_usd_femto")?);

    let status: Option<i64> = row.get("status")?;
    let error_code: Option<String> = row.get("error_code")?;
    let is_interrupted = status.is_none() && error_code.is_none();

    Ok(RequestLogSummary {
        id: row.get("id")?,
        trace_id: row.get("trace_id")?,
        cli_key: row.get("cli_key")?,
        session_id: row.get("session_id")?,
        method: row.get("method")?,
        path: row.get("path")?,
        excluded_from_stats: row.get::<_, i64>("excluded_from_stats").unwrap_or(0) != 0,
        special_settings_json: row.get("special_settings_json")?,
        requested_model: row.get("requested_model")?,
        status,
        error_code,
        is_interrupted,
        duration_ms: row.get("duration_ms")?,
        ttfb_ms: row.get("ttfb_ms")?,
        visible_ttfb_ms: row.get("visible_ttfb_ms")?,
        upstream_stream_duration_ms: row.get("upstream_stream_duration_ms")?,
        upstream_stream_timing_version: row
            .get::<_, Option<i64>>("upstream_stream_timing_version")?
            .filter(|value| *value == 1)
            .unwrap_or(0),
        final_upstream_attempt_duration_ms: row.get("final_upstream_attempt_duration_ms")?,
        final_upstream_attempt_timing_version: row
            .get::<_, Option<i64>>("final_upstream_attempt_timing_version")?
            .filter(|value| *value == 1)
            .unwrap_or(0),
        estimated_final_upstream_attempt_duration_ms: row
            .get("estimated_final_upstream_attempt_duration_ms")?,
        attempt_count,
        has_failover,
        start_provider_id,
        start_provider_name,
        final_provider_id,
        final_provider_name,
        final_provider_source_id: None,
        final_provider_source_name: None,
        route,
        session_reuse,
        input_tokens: row.get("input_tokens")?,
        output_tokens: row.get("output_tokens")?,
        total_tokens: row.get("total_tokens")?,
        cache_read_input_tokens: row.get("cache_read_input_tokens")?,
        cache_creation_input_tokens: row.get("cache_creation_input_tokens")?,
        cache_creation_5m_input_tokens: row.get("cache_creation_5m_input_tokens")?,
        cache_creation_1h_input_tokens: row.get("cache_creation_1h_input_tokens")?,
        // Filled by attach_source_provider_info (needs the providers table).
        effective_input_tokens: None,
        cost_usd,
        cost_multiplier: row.get("cost_multiplier")?,
        created_at_ms: row.get("created_at_ms")?,
        last_activity_ms: row.get("last_activity_ms")?,
        activity_details_json: row.get("activity_details_json").unwrap_or(None),
        created_at: row.get("created_at")?,
        provider_chain_json: row.get("provider_chain_json").unwrap_or(None),
        error_details_json: row.get("error_details_json").unwrap_or(None),
    })
}

fn row_to_detail(row: &rusqlite::Row<'_>) -> Result<RequestLogDetail, rusqlite::Error> {
    let attempts_json: String = row.get("attempts_json")?;
    let attempts = parse_attempts(&attempts_json);
    let (final_provider_id, final_provider_name) = final_provider_from_attempts(&attempts);
    let cost_usd = cost_usd_from_femto(row.get("cost_usd_femto")?);
    let status: Option<i64> = row.get("status")?;
    let error_code: Option<String> = row.get("error_code")?;
    let is_interrupted = status.is_none() && error_code.is_none();

    Ok(RequestLogDetail {
        id: row.get("id")?,
        trace_id: row.get("trace_id")?,
        cli_key: row.get("cli_key")?,
        session_id: row.get("session_id")?,
        method: row.get("method")?,
        path: row.get("path")?,
        query: row.get("query")?,
        excluded_from_stats: row.get::<_, i64>("excluded_from_stats").unwrap_or(0) != 0,
        special_settings_json: row.get("special_settings_json")?,
        status,
        error_code,
        is_interrupted,
        duration_ms: row.get("duration_ms")?,
        ttfb_ms: row.get("ttfb_ms")?,
        visible_ttfb_ms: row.get("visible_ttfb_ms")?,
        upstream_stream_duration_ms: row.get("upstream_stream_duration_ms")?,
        upstream_stream_timing_version: row
            .get::<_, Option<i64>>("upstream_stream_timing_version")?
            .filter(|value| *value == 1)
            .unwrap_or(0),
        final_upstream_attempt_duration_ms: row.get("final_upstream_attempt_duration_ms")?,
        final_upstream_attempt_timing_version: row
            .get::<_, Option<i64>>("final_upstream_attempt_timing_version")?
            .filter(|value| *value == 1)
            .unwrap_or(0),
        estimated_final_upstream_attempt_duration_ms: row
            .get("estimated_final_upstream_attempt_duration_ms")?,
        attempts_json,
        input_tokens: row.get("input_tokens")?,
        output_tokens: row.get("output_tokens")?,
        total_tokens: row.get("total_tokens")?,
        cache_read_input_tokens: row.get("cache_read_input_tokens")?,
        cache_creation_input_tokens: row.get("cache_creation_input_tokens")?,
        cache_creation_5m_input_tokens: row.get("cache_creation_5m_input_tokens")?,
        cache_creation_1h_input_tokens: row.get("cache_creation_1h_input_tokens")?,
        // Filled by attach_source_provider_info_to_detail.
        effective_input_tokens: None,
        usage_json: row.get("usage_json")?,
        requested_model: row.get("requested_model")?,
        final_provider_id,
        final_provider_name,
        final_provider_source_id: None,
        final_provider_source_name: None,
        cost_usd,
        cost_multiplier: row.get("cost_multiplier")?,
        created_at_ms: row.get("created_at_ms")?,
        last_activity_ms: row.get("last_activity_ms")?,
        activity_details_json: row.get("activity_details_json").unwrap_or(None),
        created_at: row.get("created_at")?,
        provider_chain_json: row.get("provider_chain_json").unwrap_or(None),
        error_details_json: row.get("error_details_json").unwrap_or(None),
    })
}

fn attach_source_provider_info_to_detail(
    conn: &Connection,
    item: &mut RequestLogDetail,
) -> crate::shared::error::AppResult<()> {
    let info_by_bridge_id = load_source_provider_info_map(conn, &[item.final_provider_id])?;
    let mut bridged = false;
    if let Some(info) = info_by_bridge_id.get(&item.final_provider_id) {
        item.final_provider_source_id = info.source_provider_id;
        item.final_provider_source_name = info.source_provider_name.clone();
        bridged = info.bridged;
    }
    let persisted_openai_semantics = super::semantics::resolve_cx2cc_cost_basis(
        item.special_settings_json.as_deref(),
        (item.final_provider_id > 0).then_some(item.final_provider_id),
    )
    .openai_input_semantics_override();
    item.effective_input_tokens = crate::usage_stats::effective_input_tokens_display(
        &item.cli_key,
        persisted_openai_semantics,
        bridged,
        item.input_tokens,
        item.cache_read_input_tokens,
        item.cache_creation_input_tokens,
    );
    Ok(())
}

pub fn list_recent(
    db: &db::Db,
    cli_key: &str,
    limit: usize,
) -> crate::shared::error::AppResult<Vec<RequestLogSummary>> {
    validate_cli_key(cli_key)?;
    let conn = db.open_connection()?;

    let sql = if cli_key == "claude" {
        format!(
            "SELECT{}FROM request_logs WHERE cli_key = ?1 AND path = ?2 ORDER BY created_at_ms DESC, id DESC LIMIT ?3",
            REQUEST_LOG_SUMMARY_FIELDS
        )
    } else {
        format!(
            "SELECT{}FROM request_logs WHERE cli_key = ?1 ORDER BY created_at_ms DESC, id DESC LIMIT ?2",
            REQUEST_LOG_SUMMARY_FIELDS
        )
    };
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| db_err!("failed to prepare query: {e}"))?;

    let rows = if cli_key == "claude" {
        stmt.query_map(
            params![cli_key, CLAUDE_VISIBLE_LOG_PATH, limit as i64],
            row_to_summary,
        )
    } else {
        stmt.query_map(params![cli_key, limit as i64], row_to_summary)
    }
    .map_err(|e| db_err!("failed to list request_logs: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| db_err!("failed to read request_log row: {e}"))?);
    }
    attach_source_provider_info(&conn, &mut items)?;
    Ok(items)
}

pub fn list_recent_all(
    db: &db::Db,
    limit: usize,
) -> crate::shared::error::AppResult<Vec<RequestLogSummary>> {
    let conn = db.open_connection()?;

    let sql = format!(
        "SELECT{}FROM request_logs WHERE {} ORDER BY created_at_ms DESC, id DESC LIMIT ?1",
        REQUEST_LOG_SUMMARY_FIELDS, CLAUDE_VISIBLE_LOG_CONDITION
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| db_err!("failed to prepare query: {e}"))?;

    let rows = stmt
        .query_map(params![limit as i64], row_to_summary)
        .map_err(|e| db_err!("failed to list request_logs: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| db_err!("failed to read request_log row: {e}"))?);
    }
    attach_source_provider_info(&conn, &mut items)?;
    Ok(items)
}

fn list_observer_rows(
    db: &db::Db,
    cli_key: Option<&str>,
    limit: usize,
    excluded_trace_ids: &[String],
    inference_only: bool,
) -> crate::shared::error::AppResult<Vec<RequestLogSummary>> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let filters = RequestLogPageFilters {
        cli_key: cli_key.map(str::to_owned),
        ..RequestLogPageFilters::default()
    };
    let (mut conditions, mut query_params) =
        page_conditions_and_params(&filters, excluded_trace_ids, None)?;
    if inference_only {
        conditions.push(format!("({OBSERVER_MODEL_INFERENCE_CONDITION})"));
    }
    let query_limit = i64::try_from(limit)
        .map_err(|_| invalid_page_input("observer request-log limit is invalid"))?;
    query_params.push(query_limit.into());
    let sql = format!(
        "SELECT{}FROM request_logs WHERE {} ORDER BY created_at_ms DESC, id DESC LIMIT ?",
        REQUEST_LOG_SUMMARY_FIELDS,
        conditions.join(" AND ")
    );
    let conn = db.open_connection()?;
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| db_err!("failed to prepare observer request-log query: {e}"))?;
    let rows = stmt
        .query_map(params_from_iter(query_params.iter()), row_to_summary)
        .map_err(|e| db_err!("failed to query observer request logs: {e}"))?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| db_err!("failed to read observer request-log row: {e}"))?);
    }
    attach_source_provider_info(&conn, &mut items)?;
    Ok(items)
}

/// Newest terminal inference records used by the Observer last/dominant summary.
pub fn list_observer_terminal_inferences(
    db: &db::Db,
    cli_key: Option<&str>,
    limit: usize,
) -> crate::shared::error::AppResult<Vec<RequestLogSummary>> {
    list_observer_rows(db, cli_key, limit, &[], true)
}

/// Newest persisted Observer rows. A row without status/error remains an
/// interrupted terminal row unless its trace is still present in the active set.
pub fn list_observer_recent_terminal(
    db: &db::Db,
    cli_key: Option<&str>,
    limit: usize,
    excluded_trace_ids: &[String],
) -> crate::shared::error::AppResult<Vec<RequestLogSummary>> {
    ensure_observer_trace_id_query_limit(excluded_trace_ids)?;
    list_observer_rows(db, cli_key, limit, excluded_trace_ids, false)
}

fn ensure_observer_trace_id_query_limit(
    trace_ids: &[String],
) -> crate::shared::error::AppResult<()> {
    if trace_ids.len() > OBSERVER_TRACE_ID_QUERY_LIMIT {
        return Err(format!(
            "SEC_INVALID_INPUT: observer trace-id query exceeds {OBSERVER_TRACE_ID_QUERY_LIMIT} entries"
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
pub fn page_all(
    db: &db::Db,
    filters: &RequestLogPageFilters,
    cursor: Option<&str>,
    limit: usize,
) -> crate::shared::error::AppResult<RequestLogPage> {
    page_all_excluding_traces(db, filters, cursor, limit, &[])
}

pub fn page_all_excluding_traces(
    db: &db::Db,
    filters: &RequestLogPageFilters,
    cursor: Option<&str>,
    limit: usize,
    excluded_trace_ids: &[String],
) -> crate::shared::error::AppResult<RequestLogPage> {
    if !(1..=REQUEST_LOG_PAGE_MAX_LIMIT).contains(&limit) {
        return Err(invalid_page_input(
            "request logs page limit must be between 1 and 200",
        ));
    }
    let cursor = decode_page_cursor(cursor)?;
    let (conditions, mut query_params) =
        page_conditions_and_params(filters, excluded_trace_ids, cursor.as_ref())?;

    let query_limit = i64::try_from(limit + 1)
        .map_err(|_| invalid_page_input("request logs page limit is invalid"))?;
    query_params.push(query_limit.into());
    let sql = format!(
        "SELECT{}FROM request_logs WHERE {} ORDER BY created_at_ms DESC, id DESC LIMIT ?",
        REQUEST_LOG_SUMMARY_FIELDS,
        conditions.join(" AND ")
    );
    let conn = db.open_connection()?;
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| db_err!("failed to prepare request_logs page query: {e}"))?;
    let rows = stmt
        .query_map(params_from_iter(query_params.iter()), row_to_summary)
        .map_err(|e| db_err!("failed to query request_logs page: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| db_err!("failed to read request_log page row: {e}"))?);
    }
    let has_more = items.len() > limit;
    if has_more {
        items.pop();
    }
    attach_source_provider_info(&conn, &mut items)?;
    let next_cursor = if has_more {
        items.last().map(encode_page_cursor).transpose()?
    } else {
        None
    };

    Ok(RequestLogPage { items, next_cursor })
}

pub fn snapshot_membership_excluding_traces(
    db: &db::Db,
    filters: &RequestLogPageFilters,
    excluded_trace_ids: &[String],
    max_memberships: usize,
) -> crate::shared::error::AppResult<Vec<i64>> {
    let (conditions, mut query_params) =
        page_conditions_and_params(filters, excluded_trace_ids, None)?;
    let query_limit = i64::try_from(max_memberships.saturating_add(1)).unwrap_or(i64::MAX);
    query_params.push(query_limit.into());
    let sql = format!(
        "SELECT id FROM request_logs WHERE {} ORDER BY created_at_ms DESC, id DESC LIMIT ?",
        conditions.join(" AND ")
    );
    let conn = db.open_connection()?;
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| db_err!("failed to prepare request-log snapshot membership query: {e}"))?;
    let rows = stmt
        .query_map(params_from_iter(query_params.iter()), |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|e| db_err!("failed to query request-log snapshot membership: {e}"))?;
    let mut ids = Vec::new();
    for row in rows {
        ids.push(row.map_err(|e| db_err!("failed to read request-log snapshot member: {e}"))?);
    }
    if ids.len() > max_memberships {
        return Err(crate::shared::error::AppError::new(
            "REQUEST_LOG_SNAPSHOT_TOO_LARGE",
            "narrow the request-log filters or time range",
        ));
    }
    Ok(ids)
}

pub fn summaries_by_ids(
    db: &db::Db,
    ids: &[i64],
) -> crate::shared::error::AppResult<Vec<RequestLogSummary>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    if ids.len() > REQUEST_LOG_PAGE_MAX_LIMIT {
        return Err(invalid_page_input(
            "request-log snapshot page exceeds the maximum page size",
        ));
    }
    let conn = db.open_connection()?;
    let placeholders = crate::db::sql_placeholders(ids.len());
    let sql = format!(
        "SELECT{}FROM request_logs WHERE id IN ({})",
        REQUEST_LOG_SUMMARY_FIELDS, placeholders
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| db_err!("failed to prepare request-log snapshot page query: {e}"))?;
    let rows = stmt
        .query_map(params_from_iter(ids.iter()), row_to_summary)
        .map_err(|e| db_err!("failed to query request-log snapshot page: {e}"))?;
    let mut by_id = HashMap::new();
    for row in rows {
        let item = row.map_err(|e| db_err!("failed to read request-log snapshot page row: {e}"))?;
        by_id.insert(item.id, item);
    }
    let mut items = Vec::with_capacity(ids.len());
    for id in ids {
        let Some(item) = by_id.remove(id) else {
            return Err(crate::shared::error::AppError::new(
                "REQUEST_LOG_SNAPSHOT_EXPIRED",
                "a request-log row was removed while this snapshot was open",
            ));
        };
        items.push(item);
    }
    attach_source_provider_info(&conn, &mut items)?;
    Ok(items)
}

pub fn list_after_id(
    db: &db::Db,
    cli_key: &str,
    after_id: i64,
    limit: usize,
) -> crate::shared::error::AppResult<Vec<RequestLogSummary>> {
    validate_cli_key(cli_key)?;
    let conn = db.open_connection()?;

    let after_id = after_id.max(0);
    let sql = if cli_key == "claude" {
        format!(
            "SELECT{}FROM request_logs WHERE cli_key = ?1 AND path = ?2 AND id > ?3 ORDER BY id ASC LIMIT ?4",
            REQUEST_LOG_SUMMARY_FIELDS
        )
    } else {
        format!(
            "SELECT{}FROM request_logs WHERE cli_key = ?1 AND id > ?2 ORDER BY id ASC LIMIT ?3",
            REQUEST_LOG_SUMMARY_FIELDS
        )
    };
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| db_err!("failed to prepare query: {e}"))?;

    let rows = if cli_key == "claude" {
        stmt.query_map(
            params![cli_key, CLAUDE_VISIBLE_LOG_PATH, after_id, limit as i64],
            row_to_summary,
        )
    } else {
        stmt.query_map(params![cli_key, after_id, limit as i64], row_to_summary)
    }
    .map_err(|e| db_err!("failed to list request_logs: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| db_err!("failed to read request_log row: {e}"))?);
    }
    attach_source_provider_info(&conn, &mut items)?;
    Ok(items)
}

pub fn list_after_id_all(
    db: &db::Db,
    after_id: i64,
    limit: usize,
) -> crate::shared::error::AppResult<Vec<RequestLogSummary>> {
    let conn = db.open_connection()?;

    let after_id = after_id.max(0);
    let sql = format!(
        "SELECT{}FROM request_logs WHERE {} AND id > ?1 ORDER BY id ASC LIMIT ?2",
        REQUEST_LOG_SUMMARY_FIELDS, CLAUDE_VISIBLE_LOG_CONDITION
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| db_err!("failed to prepare query: {e}"))?;

    let rows = stmt
        .query_map(params![after_id, limit as i64], row_to_summary)
        .map_err(|e| db_err!("failed to list request_logs: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| db_err!("failed to read request_log row: {e}"))?);
    }
    attach_source_provider_info(&conn, &mut items)?;
    Ok(items)
}

pub fn get_by_id(db: &db::Db, log_id: i64) -> crate::shared::error::AppResult<RequestLogDetail> {
    let conn = db.open_connection()?;
    let sql = format!(
        "SELECT{}FROM request_logs WHERE id = ?1 AND {}",
        REQUEST_LOG_DETAIL_FIELDS, CLAUDE_VISIBLE_LOG_CONDITION
    );
    let mut item = conn
        .query_row(&sql, params![log_id], row_to_detail)
        .optional()
        .map_err(|e| db_err!("failed to query request_log: {e}"))?
        .ok_or_else(|| {
            crate::shared::error::AppError::from("DB_NOT_FOUND: request_log not found".to_string())
        })?;
    attach_source_provider_info_to_detail(&conn, &mut item)?;
    Ok(item)
}

pub fn get_by_trace_id(
    db: &db::Db,
    trace_id: &str,
) -> crate::shared::error::AppResult<Option<RequestLogDetail>> {
    if trace_id.trim().is_empty() {
        return Err("SEC_INVALID_INPUT: trace_id is required".to_string().into());
    }

    let conn = db.open_connection()?;
    let sql = format!(
        "SELECT{}FROM request_logs WHERE trace_id = ?1 AND {}",
        REQUEST_LOG_DETAIL_FIELDS, CLAUDE_VISIBLE_LOG_CONDITION
    );
    let mut item = conn
        .query_row(&sql, params![trace_id], row_to_detail)
        .optional()
        .map_err(|e| db_err!("failed to query request_log: {e}"))?;
    if let Some(detail) = item.as_mut() {
        attach_source_provider_info_to_detail(&conn, detail)?;
    }
    Ok(item)
}

pub fn terminal_trace_ids(
    db: &db::Db,
    trace_ids: &[String],
) -> crate::shared::error::AppResult<HashSet<String>> {
    const SQLITE_PARAM_CHUNK: usize = 900;

    let mut terminal = HashSet::new();
    if trace_ids.is_empty() {
        return Ok(terminal);
    }

    let conn = db.open_connection()?;
    for chunk in trace_ids.chunks(SQLITE_PARAM_CHUNK) {
        let placeholders = std::iter::repeat_n("?", chunk.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT trace_id FROM request_logs \
             WHERE trace_id IN ({placeholders}) \
             AND (status IS NOT NULL OR error_code IS NOT NULL)"
        );
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| db_err!("failed to prepare terminal trace query: {e}"))?;
        let rows = stmt
            .query_map(params_from_iter(chunk.iter().map(String::as_str)), |row| {
                row.get::<_, String>(0)
            })
            .map_err(|e| db_err!("failed to query terminal traces: {e}"))?;
        for row in rows {
            terminal.insert(row.map_err(|e| db_err!("failed to read terminal trace: {e}"))?);
        }
    }

    Ok(terminal)
}

/// Observer projections treat every visible persisted row as terminal. Rows
/// without a status/error represent an interrupted request after it leaves the
/// active registry, so they must also suppress a matching active trace.
pub fn observer_persisted_trace_ids(
    db: &db::Db,
    trace_ids: &[String],
) -> crate::shared::error::AppResult<HashSet<String>> {
    const SQLITE_PARAM_CHUNK: usize = 900;

    ensure_observer_trace_id_query_limit(trace_ids)?;
    let mut persisted = HashSet::new();
    if trace_ids.is_empty() {
        return Ok(persisted);
    }

    let conn = db.open_connection()?;
    for chunk in trace_ids.chunks(SQLITE_PARAM_CHUNK) {
        let placeholders = std::iter::repeat_n("?", chunk.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT trace_id FROM request_logs \
             WHERE trace_id IN ({placeholders}) \
             AND {CLAUDE_VISIBLE_LOG_CONDITION}"
        );
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| db_err!("failed to prepare observer persisted trace query: {e}"))?;
        let rows = stmt
            .query_map(params_from_iter(chunk.iter().map(String::as_str)), |row| {
                row.get::<_, String>(0)
            })
            .map_err(|e| db_err!("failed to query observer persisted traces: {e}"))?;
        for row in rows {
            persisted
                .insert(row.map_err(|e| db_err!("failed to read observer persisted trace: {e}"))?);
        }
    }

    Ok(persisted)
}

#[cfg(test)]
mod tests {
    use super::{
        final_provider_from_attempts, get_by_id, get_by_trace_id, list_after_id_all,
        list_observer_recent_terminal, list_observer_terminal_inferences, list_recent,
        list_recent_all, load_source_provider_info_map, observer_persisted_trace_ids, page_all,
        page_all_excluding_traces, parse_attempts, route_from_attempts,
        snapshot_membership_excluding_traces, start_provider_from_attempts, summaries_by_ids,
        terminal_trace_ids, OBSERVER_TRACE_ID_QUERY_LIMIT,
    };
    use crate::db;
    use crate::request_logs::{
        RequestLogErrorScope, RequestLogPageFilters, RequestLogStatusFilter,
        RequestLogStatusFilterOp,
    };
    use base64::Engine as _;
    use rusqlite::Connection;
    use std::collections::HashSet;
    use tempfile::tempdir;

    fn seed_request_log(conn: &Connection, id: i64, trace_id: &str, cli_key: &str, path: &str) {
        conn.execute(
            r#"
INSERT INTO request_logs (
  id, trace_id, cli_key, session_id, method, path, query, excluded_from_stats,
  special_settings_json, status, error_code, duration_ms, ttfb_ms, attempts_json,
  input_tokens, output_tokens, total_tokens, cache_read_input_tokens,
  cache_creation_input_tokens, cache_creation_5m_input_tokens,
  cache_creation_1h_input_tokens, usage_json, requested_model, cost_usd_femto,
  cost_multiplier, created_at_ms, created_at, final_provider_id
) VALUES (?1, ?2, ?3, NULL, 'POST', ?4, NULL, 0, NULL, 200, NULL, 10, 5, '[]',
  NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 'model', NULL, 1.0, ?5, ?6, 0)
"#,
            rusqlite::params![id, trace_id, cli_key, path, id * 1000, id],
        )
        .unwrap();
    }

    #[test]
    fn page_all_excludes_active_placeholders_without_consuming_page_capacity() {
        let dir = tempdir().expect("tempdir");
        let db = db::init_for_tests(&dir.path().join("request-logs-page-active.db"))
            .expect("initialize test db");
        let conn = db.open_connection().expect("open db");
        for id in 1..=53 {
            seed_request_log(&conn, id, &format!("trace-{id}"), "codex", "/v1/responses");
        }
        conn.execute(
            "UPDATE request_logs SET status = NULL WHERE trace_id = 'trace-53'",
            [],
        )
        .expect("mark newest row as active placeholder");
        drop(conn);

        let excluded = vec!["trace-53".to_string()];
        let first =
            page_all_excluding_traces(&db, &RequestLogPageFilters::default(), None, 50, &excluded)
                .expect("load first page");
        assert_eq!(first.items.len(), 50);
        assert_eq!(
            first.items.first().map(|item| item.trace_id.as_str()),
            Some("trace-52")
        );
        assert_eq!(
            first.items.last().map(|item| item.trace_id.as_str()),
            Some("trace-3")
        );
        assert!(first.items.iter().all(|item| item.trace_id != "trace-53"));

        let second = page_all_excluding_traces(
            &db,
            &RequestLogPageFilters::default(),
            first.next_cursor.as_deref(),
            50,
            &excluded,
        )
        .expect("load second page");
        assert_eq!(
            second
                .items
                .iter()
                .map(|item| item.trace_id.as_str())
                .collect::<Vec<_>>(),
            vec!["trace-2", "trace-1"]
        );
        assert!(second.next_cursor.is_none());
    }

    #[test]
    fn page_all_error_scopes_exclude_interruptions_and_respect_time_range() {
        let dir = tempdir().expect("tempdir");
        let db = db::init_for_tests(&dir.path().join("request-logs-error-scopes.db"))
            .expect("initialize test db");
        let conn = db.open_connection().expect("open db");
        for id in 1..=8 {
            seed_request_log(
                &conn,
                id,
                &format!("trace-error-scope-{id}"),
                "codex",
                "/v1/responses",
            );
        }
        conn.execute(
            "UPDATE request_logs SET attempts_json = ?1 WHERE id = 1",
            rusqlite::params![r#"[{"stream_internal_error":{"classification":"protocol"}}]"#],
        )
        .expect("add stream-internal error");
        conn.execute(
            "UPDATE request_logs SET status = 503, error_code = 'GW_UPSTREAM_TIMEOUT' WHERE id = 2",
            [],
        )
        .expect("add upstream error");
        conn.execute(
            "UPDATE request_logs SET status = 499, error_code = 'GW_STREAM_ABORTED' WHERE id = 3",
            [],
        )
        .expect("add interruption");
        conn.execute(
            "UPDATE request_logs SET status = NULL, attempts_json = ?1 WHERE id = 4",
            rusqlite::params![r#"[{"stream_internal_error":{"classification":"protocol"}}]"#],
        )
        .expect("add unresolved interruption with stream evidence");
        conn.execute("UPDATE request_logs SET status = 418 WHERE id = 5", [])
            .expect("add status-only error");
        for (id, attempts_json) in [
            (6, "not-json"),
            (7, r#"["legacy"]"#),
            (8, r#"{"legacy":"value"}"#),
        ] {
            conn.execute(
                "UPDATE request_logs SET attempts_json = ?1 WHERE id = ?2",
                rusqlite::params![attempts_json, id],
            )
            .expect("add legacy attempts shape");
        }
        drop(conn);

        let all_errors = page_all(
            &db,
            &RequestLogPageFilters {
                error_scope: RequestLogErrorScope::AllErrors,
                ..RequestLogPageFilters::default()
            },
            None,
            50,
        )
        .expect("list all errors");
        assert_eq!(
            all_errors
                .items
                .iter()
                .map(|item| item.id)
                .collect::<Vec<_>>(),
            vec![5, 2, 1],
            "all errors includes real failures but excludes interruptions and user cancellation"
        );

        let stream_errors = page_all(
            &db,
            &RequestLogPageFilters {
                error_scope: RequestLogErrorScope::StreamInternalError,
                ..RequestLogPageFilters::default()
            },
            None,
            50,
        )
        .expect("list stream internal errors");
        assert_eq!(
            stream_errors
                .items
                .iter()
                .map(|item| item.id)
                .collect::<Vec<_>>(),
            vec![4, 1]
        );

        let bounded = page_all(
            &db,
            &RequestLogPageFilters {
                error_scope: RequestLogErrorScope::AllErrors,
                created_at_ms_from: Some(1_500),
                created_at_ms_to: Some(5_500),
                ..RequestLogPageFilters::default()
            },
            None,
            50,
        )
        .expect("list bounded errors");
        assert_eq!(
            bounded.items.iter().map(|item| item.id).collect::<Vec<_>>(),
            vec![5, 2]
        );
    }

    #[test]
    fn snapshot_membership_preserves_order_when_new_rows_arrive() {
        let dir = tempdir().expect("tempdir");
        let db = db::init_for_tests(&dir.path().join("request-logs-snapshot.db"))
            .expect("initialize test db");
        let conn = db.open_connection().expect("open db");
        for id in 1..=7 {
            seed_request_log(
                &conn,
                id,
                &format!("trace-snapshot-{id}"),
                "codex",
                "/v1/responses",
            );
        }
        drop(conn);

        let membership = snapshot_membership_excluding_traces(
            &db,
            &RequestLogPageFilters::default(),
            &[],
            1_000_000,
        )
        .expect("capture membership");
        assert_eq!(membership, vec![7, 6, 5, 4, 3, 2, 1]);

        let conn = db.open_connection().expect("reopen db");
        seed_request_log(&conn, 8, "trace-snapshot-8", "codex", "/v1/responses");
        drop(conn);
        let second_page = summaries_by_ids(&db, &membership[3..6]).expect("load snapshot page");
        assert_eq!(
            second_page.iter().map(|item| item.id).collect::<Vec<_>>(),
            vec![4, 3, 2]
        );

        let conn = db.open_connection().expect("reopen db");
        conn.execute("DELETE FROM request_logs WHERE id = 3", [])
            .expect("delete snapshotted row");
        drop(conn);
        let error = summaries_by_ids(&db, &[4, 3]).unwrap_err();
        assert_eq!(error.code(), "REQUEST_LOG_SNAPSHOT_EXPIRED");
    }

    #[test]
    fn snapshot_membership_stops_after_the_configured_bound() {
        let dir = tempdir().expect("tempdir");
        let db = db::init_for_tests(&dir.path().join("request-logs-snapshot-limit.db"))
            .expect("initialize test db");
        let conn = db.open_connection().expect("open db");
        for id in 1..=4 {
            seed_request_log(
                &conn,
                id,
                &format!("trace-snapshot-limit-{id}"),
                "codex",
                "/v1/responses",
            );
        }
        drop(conn);

        let error =
            snapshot_membership_excluding_traces(&db, &RequestLogPageFilters::default(), &[], 3)
                .expect_err("oversized membership must be rejected");
        assert_eq!(error.code(), "REQUEST_LOG_SNAPSHOT_TOO_LARGE");
    }

    #[test]
    fn route_includes_skipped_attempts() {
        let attempts = parse_attempts(
            r#"[
                {"provider_id":1,"provider_name":"A","outcome":"skipped","status":null,"error_code":"GW_PROVIDER_RATE_LIMITED","decision":"skip","reason":"provider skipped by rate limit"},
                {"provider_id":2,"provider_name":"B","outcome":"success","status":200,"error_code":null,"decision":"success","reason":null}
            ]"#,
        );
        let route = route_from_attempts(&attempts);
        assert_eq!(route.len(), 2);
        assert_eq!(route[0].provider_id, 1);
        assert!(route[0].skipped);
        assert!(!route[0].ok);
        assert_eq!(route[0].attempts, 1);
        assert_eq!(
            route[0].error_code.as_deref(),
            Some("GW_PROVIDER_RATE_LIMITED")
        );
        assert_eq!(route[0].decision.as_deref(), Some("skip"));
        assert_eq!(
            route[0].reason.as_deref(),
            Some("provider skipped by rate limit")
        );
        assert_eq!(route[1].provider_id, 2);
        assert!(!route[1].skipped);
        assert!(route[1].ok);
        assert_eq!(route[1].attempts, 1);
    }

    #[test]
    fn route_includes_gate_only_skip_attempts() {
        let attempts = parse_attempts(
            r#"[
                {"provider_id":1,"provider_name":"A","outcome":"skipped","status":null,"error_code":"GW_PROVIDER_CIRCUIT_OPEN","decision":"skip","reason":"provider skipped by circuit breaker"}
            ]"#,
        );
        let route = route_from_attempts(&attempts);
        assert_eq!(route.len(), 1);
        assert_eq!(route[0].provider_id, 1);
        assert!(route[0].skipped);
        assert!(!route[0].ok);
        assert_eq!(route[0].attempts, 1);
    }

    #[test]
    fn start_and_final_provider_prefer_non_skipped_attempts() {
        let attempts = parse_attempts(
            r#"[
                {"provider_id":1,"provider_name":"A","outcome":"skipped","status":null,"error_code":"GW_PROVIDER_RATE_LIMITED","decision":"skip","reason":"provider skipped by rate limit"},
                {"provider_id":2,"provider_name":"B","outcome":"failed","status":429,"error_code":"GW_UPSTREAM_4XX","decision":"abort","reason":"status=429"}
            ]"#,
        );

        let (start_id, start_name) = start_provider_from_attempts(&attempts);
        assert_eq!(start_id, 2);
        assert_eq!(start_name, "B");

        let (final_id, final_name) = final_provider_from_attempts(&attempts);
        assert_eq!(final_id, 2);
        assert_eq!(final_name, "B");
    }

    #[test]
    fn start_and_final_provider_hide_gate_only_skips() {
        let attempts = parse_attempts(
            r#"[
                {"provider_id":1,"provider_name":"A","outcome":"skipped","status":null,"error_code":"GW_PROVIDER_CIRCUIT_OPEN","decision":"skip","reason":"provider skipped by circuit breaker"}
            ]"#,
        );

        let (start_id, start_name) = start_provider_from_attempts(&attempts);
        assert_eq!(start_id, 0);
        assert_eq!(start_name, "Unknown");

        let (final_id, final_name) = final_provider_from_attempts(&attempts);
        assert_eq!(final_id, 0);
        assert_eq!(final_name, "Unknown");

        let route = route_from_attempts(&attempts);
        assert_eq!(route.len(), 1);
        assert!(route[0].skipped);
        assert!(!route[0].ok);
    }

    #[test]
    fn route_counts_consecutive_same_provider_attempts() {
        let attempts = parse_attempts(
            r#"[
                {"provider_id":1,"provider_name":"A","outcome":"failed","status":500,"error_code":"GW_UPSTREAM_5XX","decision":"retry","reason":"status=500"},
                {"provider_id":1,"provider_name":"A","outcome":"failed","status":500,"error_code":"GW_UPSTREAM_5XX","decision":"retry","reason":"status=500"},
                {"provider_id":1,"provider_name":"A","outcome":"failed","status":500,"error_code":"GW_UPSTREAM_5XX","decision":"failover","reason":"status=500"},
                {"provider_id":2,"provider_name":"B","outcome":"success","status":200,"error_code":null,"decision":"success","reason":null}
            ]"#,
        );
        let route = route_from_attempts(&attempts);
        assert_eq!(route.len(), 2);
        assert_eq!(route[0].provider_id, 1);
        assert_eq!(route[0].attempts, 3);
        assert_eq!(route[0].provider_name, "A");
        assert!(!route[0].ok);
        assert_eq!(route[1].provider_id, 2);
        assert_eq!(route[1].attempts, 1);
        assert_eq!(route[1].provider_name, "B");
        assert!(route[1].ok);
    }

    #[test]
    fn route_single_provider_single_attempt() {
        let attempts = parse_attempts(
            r#"[
                {"provider_id":1,"provider_name":"A","outcome":"success","status":200,"error_code":null,"decision":"success","reason":null}
            ]"#,
        );
        let route = route_from_attempts(&attempts);
        assert_eq!(route.len(), 1);
        assert_eq!(route[0].provider_id, 1);
        assert_eq!(route[0].attempts, 1);
        assert!(route[0].ok);
    }

    #[test]
    fn started_attempt_still_resolves_provider_for_abort_logs() {
        let attempts = parse_attempts(
            r#"[
                {"provider_id":12,"provider_name":"Claude Bridge","outcome":"started","status":null,"error_code":null,"decision":null,"reason":null}
            ]"#,
        );

        let (final_id, final_name) = final_provider_from_attempts(&attempts);
        assert_eq!(final_id, 12);
        assert_eq!(final_name, "Claude Bridge");

        let route = route_from_attempts(&attempts);
        assert_eq!(route.len(), 1);
        assert_eq!(route[0].provider_id, 12);
        assert_eq!(route[0].provider_name, "Claude Bridge");
        assert!(!route[0].ok);
    }

    #[test]
    fn loads_source_provider_names_for_bridge_providers() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"
CREATE TABLE providers (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  source_provider_id INTEGER,
  bridge_type TEXT
);
INSERT INTO providers (id, name, source_provider_id, bridge_type) VALUES (7, 'OpenAI Primary', NULL, NULL);
INSERT INTO providers (id, name, source_provider_id, bridge_type) VALUES (12, 'Claude Bridge', 7, 'cx2cc');
"#,
        )
        .unwrap();

        let info = load_source_provider_info_map(&conn, &[7, 12, 99]).unwrap();
        let bridge = info.get(&12).expect("bridge provider source info");

        assert_eq!(bridge.source_provider_id, Some(7));
        assert_eq!(
            bridge.source_provider_name.as_deref(),
            Some("OpenAI Primary")
        );
        assert!(bridge.bridged);

        let plain = info.get(&7).expect("plain provider info");
        assert_eq!(plain.source_provider_id, None);
        assert!(!plain.bridged);

        assert!(!info.contains_key(&99));
    }

    #[test]
    fn list_queries_hide_claude_non_messages_rows() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("request-logs.db");
        let db = db::init_for_tests(&db_path).unwrap();
        let conn = db.open_connection().unwrap();

        seed_request_log(&conn, 1, "trace-claude-messages", "claude", "/v1/messages");
        seed_request_log(
            &conn,
            2,
            "trace-claude-count",
            "claude",
            "/v1/messages/count_tokens",
        );
        seed_request_log(&conn, 3, "trace-codex", "codex", "/v1/responses");
        drop(conn);

        let all = list_recent_all(&db, 10).unwrap();
        assert_eq!(
            all.iter().map(|item| item.id).collect::<Vec<_>>(),
            vec![3, 1]
        );

        let claude = list_recent(&db, "claude", 10).unwrap();
        assert_eq!(
            claude.iter().map(|item| item.id).collect::<Vec<_>>(),
            vec![1]
        );

        let after = list_after_id_all(&db, 1, 10).unwrap();
        assert_eq!(
            after.iter().map(|item| item.id).collect::<Vec<_>>(),
            vec![3]
        );
    }

    #[test]
    fn observer_queries_bound_inference_rows_and_exclude_active_traces() {
        let dir = tempdir().unwrap();
        let db = db::init_for_tests(&dir.path().join("observer-request-logs.db")).unwrap();
        let conn = db.open_connection().unwrap();
        for (id, trace_id, cli_key, path) in [
            (1, "claude-message", "claude", "/v1/messages"),
            (2, "claude-count", "claude", "/v1/messages/count_tokens"),
            (3, "codex-response", "codex", "/v1/responses"),
            (4, "codex-models", "codex", "/v1/models"),
            (5, "grok-response", "grok", "/v1/responses/?trace=1"),
            (
                6,
                "gemini-stream",
                "gemini",
                "/v1beta/models/gemini:streamGenerateContent?alt=sse",
            ),
            (7, "codex-compact", "codex", "/v1/codex/responses/compact/"),
            (8, "claude-alias", "claude", "/messages"),
            (9, "codex-get", "codex", "/v1/responses"),
        ] {
            seed_request_log(&conn, id, trace_id, cli_key, path);
        }
        conn.execute(
            "UPDATE request_logs SET status = NULL, error_code = NULL WHERE id = 5",
            [],
        )
        .unwrap();
        conn.execute("UPDATE request_logs SET method = 'GET' WHERE id = 9", [])
            .unwrap();
        conn.execute(
            "UPDATE request_logs SET created_at_ms = 7000 WHERE id IN (6, 7)",
            [],
        )
        .unwrap();
        drop(conn);

        let inference = list_observer_terminal_inferences(&db, None, 3).unwrap();
        assert_eq!(
            inference.iter().map(|item| item.id).collect::<Vec<_>>(),
            vec![7, 6, 5],
            "query limit applies after inference and visibility predicates"
        );
        assert!(inference[2].is_interrupted);

        let claude = list_observer_terminal_inferences(&db, Some("claude"), 10).unwrap();
        assert_eq!(
            claude.iter().map(|item| item.id).collect::<Vec<_>>(),
            vec![1],
            "Claude aliases and token-count rows retain the existing visibility boundary"
        );

        let excluded = vec!["codex-compact".to_string(), "gemini-stream".to_string()];
        let recent = list_observer_recent_terminal(&db, None, 3, &excluded).unwrap();
        assert_eq!(
            recent.iter().map(|item| item.id).collect::<Vec<_>>(),
            vec![9, 5, 4],
            "active rows do not consume the requested recent capacity"
        );
        assert!(list_observer_recent_terminal(&db, Some("codex"), 0, &[])
            .unwrap()
            .is_empty());
    }

    #[test]
    fn page_all_uses_two_key_cursor_without_gaps_for_tied_timestamps() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("request-logs-page.db");
        let db = db::init_for_tests(&db_path).unwrap();
        let conn = db.open_connection().unwrap();
        for id in 1..=7 {
            seed_request_log(
                &conn,
                id,
                &format!("trace-page-{id}"),
                "codex",
                "/v1/responses",
            );
        }
        conn.execute("UPDATE request_logs SET created_at_ms = 123456", [])
            .unwrap();
        drop(conn);

        let filters = RequestLogPageFilters::default();
        let mut cursor = None;
        let mut ids = Vec::new();
        loop {
            let page = page_all(&db, &filters, cursor.as_deref(), 3).unwrap();
            ids.extend(page.items.iter().map(|item| item.id));
            let Some(next_cursor) = page.next_cursor else {
                break;
            };
            if ids.len() == 3 {
                let cursor_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
                    .decode(next_cursor.as_bytes())
                    .unwrap();
                assert_eq!(
                    serde_json::from_slice::<serde_json::Value>(&cursor_bytes).unwrap(),
                    serde_json::json!({"v": 1, "createdAtMs": 123456, "id": 5})
                );
            }
            cursor = Some(next_cursor);
        }

        assert_eq!(ids, vec![7, 6, 5, 4, 3, 2, 1]);
        let unique = ids.iter().copied().collect::<HashSet<_>>();
        assert_eq!(unique.len(), ids.len());
    }

    #[test]
    fn page_all_applies_filters_with_literal_contains_and_null_neq() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("request-logs-page-filters.db");
        let db = db::init_for_tests(&db_path).unwrap();
        let conn = db.open_connection().unwrap();

        seed_request_log(&conn, 1, "trace-visible-null", "claude", "/v1/messages");
        seed_request_log(
            &conn,
            2,
            "trace-hidden-null",
            "claude",
            "/v1/messages/count_tokens",
        );
        seed_request_log(&conn, 3, "trace-codex-error", "codex", "/v1/responses");
        seed_request_log(&conn, 4, "trace-codex-ok", "codex", "/v1/other");
        seed_request_log(&conn, 5, "trace-gemini", "gemini", "/v1/responses");
        conn.execute(
            "UPDATE request_logs SET method = 'GET', status = NULL, error_code = 'GW_LITERAL_%_NEEDLE' WHERE id IN (1, 2)",
            [],
        )
        .unwrap();
        conn.execute(
            "UPDATE request_logs SET status = 503, error_code = 'GW_UPSTREAM_TIMEOUT' WHERE id = 3",
            [],
        )
        .unwrap();
        conn.execute(
            "UPDATE request_logs SET status = 404, error_code = 'GW_NOT_FOUND' WHERE id = 5",
            [],
        )
        .unwrap();
        drop(conn);

        let codex_error = page_all(
            &db,
            &RequestLogPageFilters {
                cli_key: Some("codex".to_string()),
                status: Some(RequestLogStatusFilter {
                    op: RequestLogStatusFilterOp::Gte,
                    value: 500,
                }),
                error_code_contains: Some("upstream".to_string()),
                method_path_contains: Some("post /V1/RESPONSES".to_string()),
                ..RequestLogPageFilters::default()
            },
            None,
            50,
        )
        .unwrap();
        assert_eq!(
            codex_error
                .items
                .iter()
                .map(|item| item.id)
                .collect::<Vec<_>>(),
            vec![3]
        );

        for (op, value, expected_ids) in [
            (RequestLogStatusFilterOp::Eq, 503, vec![3]),
            (RequestLogStatusFilterOp::Lte, 200, vec![4]),
        ] {
            let page = page_all(
                &db,
                &RequestLogPageFilters {
                    status: Some(RequestLogStatusFilter { op, value }),
                    ..RequestLogPageFilters::default()
                },
                None,
                50,
            )
            .unwrap();
            assert_eq!(
                page.items.iter().map(|item| item.id).collect::<Vec<_>>(),
                expected_ids
            );
        }

        let not_503 = page_all(
            &db,
            &RequestLogPageFilters {
                status: Some(RequestLogStatusFilter {
                    op: RequestLogStatusFilterOp::Neq,
                    value: 503,
                }),
                ..RequestLogPageFilters::default()
            },
            None,
            50,
        )
        .unwrap();
        assert_eq!(
            not_503.items.iter().map(|item| item.id).collect::<Vec<_>>(),
            vec![5, 4, 1],
            "neq must include NULL while Claude visibility still hides non-message rows"
        );

        let literal = page_all(
            &db,
            &RequestLogPageFilters {
                error_code_contains: Some("%_".to_string()),
                ..RequestLogPageFilters::default()
            },
            None,
            50,
        )
        .unwrap();
        assert_eq!(
            literal.items.iter().map(|item| item.id).collect::<Vec<_>>(),
            vec![1],
            "contains filters must treat SQL wildcard characters literally"
        );
    }

    #[test]
    fn page_all_rejects_invalid_cursor_and_filter_boundaries() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("request-logs-page-invalid.db");
        let db = db::init_for_tests(&db_path).unwrap();
        let filters = RequestLogPageFilters::default();
        let encode_json = |value: serde_json::Value| {
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode(serde_json::to_vec(&value).unwrap())
        };

        for cursor in [
            " ".to_string(),
            "*not-base64*".to_string(),
            encode_json(serde_json::json!([])),
            encode_json(serde_json::json!({"v": 2, "createdAtMs": 1, "id": 1})),
            encode_json(serde_json::json!({"v": 1, "createdAtMs": -1, "id": 1})),
            encode_json(serde_json::json!({"v": 1, "createdAtMs": 1, "id": 0})),
            encode_json(serde_json::json!({"v": 1, "createdAtMs": 1, "id": 1, "extra": true})),
            encode_json(serde_json::json!({"v": 1, "created_at_ms": 1, "id": 1})),
            "a".repeat(513),
        ] {
            let error = page_all(&db, &filters, Some(&cursor), 50).unwrap_err();
            assert_eq!(error.code(), "SEC_INVALID_INPUT", "cursor={cursor}");
        }

        let invalid_filters = [
            RequestLogPageFilters {
                status: Some(RequestLogStatusFilter {
                    op: RequestLogStatusFilterOp::Eq,
                    value: 1_000,
                }),
                ..RequestLogPageFilters::default()
            },
            RequestLogPageFilters {
                error_code_contains: Some("界".repeat(86)),
                ..RequestLogPageFilters::default()
            },
            RequestLogPageFilters {
                method_path_contains: Some("x".repeat(513)),
                ..RequestLogPageFilters::default()
            },
            RequestLogPageFilters {
                cli_key: Some("unknown".to_string()),
                ..RequestLogPageFilters::default()
            },
        ];
        for filters in invalid_filters {
            let error = page_all(&db, &filters, None, 50).unwrap_err();
            assert_eq!(error.code(), "SEC_INVALID_INPUT");
        }
        for limit in [0, 201] {
            let error = page_all(&db, &filters, None, limit).unwrap_err();
            assert_eq!(error.code(), "SEC_INVALID_INPUT");
        }
        for status_value in [0, 999] {
            let boundary_filters = RequestLogPageFilters {
                status: Some(RequestLogStatusFilter {
                    op: RequestLogStatusFilterOp::Eq,
                    value: status_value,
                }),
                error_code_contains: Some("x".repeat(256)),
                method_path_contains: Some("y".repeat(512)),
                ..RequestLogPageFilters::default()
            };
            page_all(&db, &boundary_filters, None, 200).unwrap();
        }
    }

    #[test]
    fn terminal_trace_ids_returns_only_persisted_terminal_logs() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("request-logs.db");
        let db = db::init_for_tests(&db_path).unwrap();
        let conn = db.open_connection().unwrap();

        seed_request_log(&conn, 1, "trace-status-terminal", "codex", "/v1/responses");
        seed_request_log(&conn, 2, "trace-error-terminal", "codex", "/v1/responses");
        seed_request_log(&conn, 3, "trace-pending", "codex", "/v1/responses");
        conn.execute(
            "UPDATE request_logs SET status = NULL, error_code = 'GW_REQUEST_ABORTED' WHERE trace_id = ?1",
            rusqlite::params!["trace-error-terminal"],
        )
        .unwrap();
        conn.execute(
            "UPDATE request_logs SET status = NULL, error_code = NULL WHERE trace_id = ?1",
            rusqlite::params!["trace-pending"],
        )
        .unwrap();
        drop(conn);

        let terminal = terminal_trace_ids(
            &db,
            &[
                "trace-status-terminal".to_string(),
                "trace-error-terminal".to_string(),
                "trace-pending".to_string(),
                "trace-missing".to_string(),
            ],
        )
        .unwrap();

        assert!(terminal.contains("trace-status-terminal"));
        assert!(terminal.contains("trace-error-terminal"));
        assert!(!terminal.contains("trace-pending"));
        assert!(!terminal.contains("trace-missing"));
    }

    #[test]
    fn observer_persisted_trace_ids_keep_visibility_and_interrupted_semantics() {
        let dir = tempdir().unwrap();
        let db = db::init_for_tests(&dir.path().join("observer-traces.db")).unwrap();
        let conn = db.open_connection().unwrap();
        seed_request_log(&conn, 1, "visible-claude", "claude", "/v1/messages");
        seed_request_log(
            &conn,
            2,
            "hidden-claude",
            "claude",
            "/v1/messages/count_tokens",
        );
        seed_request_log(&conn, 3, "interrupted-codex", "codex", "/v1/responses");
        conn.execute(
            "UPDATE request_logs SET status = NULL, error_code = NULL WHERE id = 3",
            [],
        )
        .unwrap();
        drop(conn);

        let persisted = observer_persisted_trace_ids(
            &db,
            &[
                "visible-claude".to_string(),
                "hidden-claude".to_string(),
                "interrupted-codex".to_string(),
            ],
        )
        .unwrap();
        assert!(persisted.contains("visible-claude"));
        assert!(persisted.contains("interrupted-codex"));
        assert!(!persisted.contains("hidden-claude"));
    }

    #[test]
    fn observer_trace_queries_reject_unbounded_exclusion_sets() {
        let dir = tempdir().unwrap();
        let db = db::init_for_tests(&dir.path().join("observer-trace-limit.db")).unwrap();
        let bounded_trace_ids = (0..OBSERVER_TRACE_ID_QUERY_LIMIT)
            .map(|index| format!("bounded-trace-{index}"))
            .collect::<Vec<_>>();
        assert!(observer_persisted_trace_ids(&db, &bounded_trace_ids)
            .expect("exact trace-id limit should be accepted")
            .is_empty());
        assert!(
            list_observer_recent_terminal(&db, None, 1, &bounded_trace_ids)
                .expect("exact exclusion limit should be accepted")
                .is_empty()
        );

        let trace_ids = (0..=OBSERVER_TRACE_ID_QUERY_LIMIT)
            .map(|index| format!("trace-{index}"))
            .collect::<Vec<_>>();

        let persisted_error = observer_persisted_trace_ids(&db, &trace_ids)
            .expect_err("persisted trace query must be bounded")
            .to_string();
        assert!(persisted_error.contains("observer trace-id query exceeds"));

        let recent_error = list_observer_recent_terminal(&db, None, 1, &trace_ids)
            .expect_err("recent exclusion query must be bounded")
            .to_string();
        assert!(recent_error.contains("observer trace-id query exceeds"));
    }

    #[test]
    fn detail_queries_hide_claude_non_messages_rows() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("request-logs.db");
        let db = db::init_for_tests(&db_path).unwrap();
        let conn = db.open_connection().unwrap();

        seed_request_log(&conn, 1, "trace-claude-messages", "claude", "/v1/messages");
        seed_request_log(
            &conn,
            2,
            "trace-claude-count",
            "claude",
            "/v1/messages/count_tokens",
        );
        seed_request_log(&conn, 3, "trace-codex", "codex", "/v1/responses");
        drop(conn);

        let visible = get_by_id(&db, 1).unwrap();
        assert_eq!(visible.id, 1);

        let hidden = get_by_id(&db, 2).unwrap_err().to_string();
        assert!(hidden.contains("request_log not found"));

        let hidden_by_trace = get_by_trace_id(&db, "trace-claude-count").unwrap();
        assert!(hidden_by_trace.is_none());

        let visible_by_trace = get_by_trace_id(&db, "trace-codex").unwrap();
        assert_eq!(visible_by_trace.as_ref().map(|item| item.id), Some(3));
    }

    #[test]
    fn summary_and_detail_expose_session_id() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("request-logs.db");
        let db = db::init_for_tests(&db_path).unwrap();
        let conn = db.open_connection().unwrap();

        conn.execute(
            r#"
INSERT INTO request_logs (
  id, trace_id, cli_key, session_id, method, path, query, excluded_from_stats,
  special_settings_json, status, error_code, duration_ms, ttfb_ms, attempts_json,
  input_tokens, output_tokens, total_tokens, cache_read_input_tokens,
  cache_creation_input_tokens, cache_creation_5m_input_tokens,
  cache_creation_1h_input_tokens, usage_json, requested_model, cost_usd_femto,
  cost_multiplier, created_at_ms, created_at, final_provider_id
) VALUES (?1, ?2, ?3, ?4, 'POST', ?5, NULL, 0, NULL, 200, NULL, 10, 5, '[]',
  NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 'model', NULL, 1.0, ?6, ?7, 0)
"#,
            rusqlite::params![
                11_i64,
                "trace-session-id",
                "codex",
                "sess-123",
                "/v1/responses",
                11_000_i64,
                11_i64
            ],
        )
        .unwrap();
        drop(conn);

        let summary = list_recent_all(&db, 10).unwrap();
        assert_eq!(summary[0].session_id.as_deref(), Some("sess-123"));

        let detail = get_by_id(&db, 11).unwrap();
        assert_eq!(detail.session_id.as_deref(), Some("sess-123"));
    }

    #[test]
    fn summary_and_detail_prefer_persisted_cx2cc_semantics_over_provider_state() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("request-log-semantics.db");
        let db = db::init_for_tests(&db_path).unwrap();
        let conn = db.open_connection().unwrap();

        conn.execute_batch(
            r#"
INSERT INTO providers (id, provider_uuid, cli_key, name, base_url, api_key_plaintext, enabled, priority,
  sort_order, cost_multiplier, created_at, updated_at)
VALUES (7, '00000000-0000-4000-8000-000000000007', 'codex', 'OpenAI Primary', 'https://example.com', '', 1, 100, 0, 1.0, 1, 1);
INSERT INTO providers (id, provider_uuid, cli_key, name, base_url, api_key_plaintext, enabled, priority,
  sort_order, cost_multiplier, source_provider_id, bridge_type, created_at, updated_at)
VALUES (12, '00000000-0000-4000-8000-000000000012', 'claude', 'Claude Bridge', 'https://example.com', '', 1, 100, 0, 1.0,
  7, 'cx2cc', 1, 1);
"#,
        )
        .unwrap();

        let fixtures = [
            (
                31_i64,
                Some(r#"[{"type":"cx2cc_cost_basis","source_cli_key":"codex"}]"#),
            ),
            (
                32_i64,
                Some(r#"[{"type":"cx2cc_cost_basis","source_cli_key":"claude"}]"#),
            ),
            (33_i64, None),
            (34_i64, Some("not-json")),
            (
                35_i64,
                Some(
                    r#"[{"type":"cx2cc_cost_basis","bridge_provider_id":12,"source_cli_key":"codex"}]"#,
                ),
            ),
            (
                36_i64,
                Some(
                    r#"[{"type":"cx2cc_cost_basis","bridge_provider_id":99,"source_cli_key":"codex"}]"#,
                ),
            ),
        ];

        for (id, special_settings_json) in fixtures {
            conn.execute(
                r#"
INSERT INTO request_logs (
  id, trace_id, cli_key, method, path, query, excluded_from_stats,
  special_settings_json, status, error_code, duration_ms, ttfb_ms, attempts_json,
  input_tokens, output_tokens, total_tokens, cache_read_input_tokens,
  cache_creation_input_tokens, cache_creation_5m_input_tokens,
  cache_creation_1h_input_tokens, usage_json, requested_model, cost_usd_femto,
  cost_multiplier, created_at_ms, created_at, final_provider_id
) VALUES (?1, ?2, 'claude', 'POST', '/v1/messages', NULL, 0, ?3, 200, NULL, 10, 5,
  '[{"provider_id":12,"provider_name":"Claude Bridge","outcome":"success","status":200}]',
  1000, 50, 1050, 100, 200, NULL, NULL, NULL, 'claude-model', NULL, 1.0,
  ?4, ?1, 12)
"#,
                rusqlite::params![
                    id,
                    format!("trace-semantics-{id}"),
                    special_settings_json,
                    id * 1000
                ],
            )
            .unwrap();
        }
        drop(conn);

        let assert_effective = |expected: &[(i64, i64)]| {
            let summaries = list_recent_all(&db, 20).unwrap();
            for (id, tokens) in expected {
                let summary = summaries
                    .iter()
                    .find(|item| item.id == *id)
                    .unwrap_or_else(|| panic!("missing summary id={id}"));
                assert_eq!(
                    summary.effective_input_tokens,
                    Some(*tokens),
                    "summary id={id}"
                );

                let detail = get_by_id(&db, *id).unwrap();
                assert_eq!(
                    detail.effective_input_tokens,
                    Some(*tokens),
                    "detail id={id}"
                );
            }
        };

        assert_effective(&[
            (31, 700),
            (32, 1000),
            (33, 700),
            (34, 700),
            (35, 700),
            (36, 1000),
        ]);

        let conn = db.open_connection().unwrap();
        conn.execute(
            "UPDATE providers SET source_provider_id = NULL, bridge_type = NULL WHERE id = 12",
            [],
        )
        .unwrap();
        drop(conn);
        assert_effective(&[
            (31, 700),
            (32, 1000),
            (33, 1000),
            (34, 1000),
            (35, 700),
            (36, 1000),
        ]);

        let conn = db.open_connection().unwrap();
        conn.execute("DELETE FROM providers WHERE id = 12", [])
            .unwrap();
        drop(conn);
        assert_effective(&[(31, 700), (32, 1000), (35, 700), (36, 1000)]);
    }
}
