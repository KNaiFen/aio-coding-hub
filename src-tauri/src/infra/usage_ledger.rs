//! Usage: Durable analytics projection, resumable backfill, and retention gates.

use crate::shared::error::{db_err, AppResult};
use crate::shared::time::now_unix_seconds;
use crate::{db, request_logs};
use rusqlite::{params, params_from_iter, Connection, OptionalExtension, Row, TransactionBehavior};
use std::sync::OnceLock;
use std::time::Duration;

pub(crate) const USAGE_LEDGER_BACKFILL_INCOMPLETE_ERROR_CODE: &str =
    "USAGE_LEDGER_BACKFILL_INCOMPLETE";

const BACKFILL_BATCH_SIZE: i64 = 250;
const BACKFILL_BATCH_PAUSE: Duration = Duration::from_millis(10);
const BACKFILL_RETRY_BASE_DELAY: Duration = Duration::from_secs(5);
const BACKFILL_RETRY_MAX_DELAY: Duration = Duration::from_secs(5 * 60);
const STATUS_INCOMPLETE: &str = "incomplete";
const STATUS_COMPLETE: &str = "complete";

const SOURCE_SELECT: &str = r#"
SELECT
  r.id,
  r.trace_id,
  r.cli_key,
  r.session_id,
  r.created_at,
  r.created_at_ms,
  r.status,
  r.error_code,
  r.excluded_from_stats,
  r.duration_ms,
  r.ttfb_ms,
  r.visible_ttfb_ms,
  r.requested_model,
  r.final_provider_id,
  r.input_tokens,
  r.output_tokens,
  r.total_tokens,
  r.cache_read_input_tokens,
  r.cache_creation_input_tokens,
  r.cache_creation_5m_input_tokens,
  r.cache_creation_1h_input_tokens,
  r.usage_json,
  r.cost_usd_femto,
  r.cost_multiplier,
  r.special_settings_json,
  r.attempts_json,
  r.upstream_stream_duration_ms,
  r.upstream_stream_timing_version
FROM request_logs r
"#;

const LEDGER_UPSERT_SQL: &str = r#"
INSERT INTO usage_ledger (
  request_log_id,
  trace_id,
  cli_key,
  session_id,
  created_at,
  created_at_ms,
  status,
  error_present,
  excluded_from_stats,
  duration_ms,
  ttfb_ms,
  visible_ttfb_ms,
  requested_model,
  final_provider_id,
  provider_name_snapshot,
  usage_present,
  input_tokens,
  output_tokens,
  total_tokens,
  cache_read_input_tokens,
  cache_creation_input_tokens,
  cache_creation_5m_input_tokens,
  cache_creation_1h_input_tokens,
  persisted_openai_input_semantics,
  cost_usd_femto,
  cost_multiplier,
  cost_basis_cli_key,
  cost_basis_model,
  priority_service_tier_applied,
  upstream_stream_duration_ms,
  upstream_stream_timing_version
) VALUES (
  ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
  ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20,
  ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30,
  ?31
)
ON CONFLICT(trace_id) DO UPDATE SET
  request_log_id = excluded.request_log_id,
  cli_key = excluded.cli_key,
  session_id = excluded.session_id,
  created_at = excluded.created_at,
  created_at_ms = excluded.created_at_ms,
  status = excluded.status,
  error_present = excluded.error_present,
  excluded_from_stats = excluded.excluded_from_stats,
  duration_ms = excluded.duration_ms,
  ttfb_ms = excluded.ttfb_ms,
  visible_ttfb_ms = excluded.visible_ttfb_ms,
  requested_model = excluded.requested_model,
  final_provider_id = excluded.final_provider_id,
  provider_name_snapshot = COALESCE(
    excluded.provider_name_snapshot,
    usage_ledger.provider_name_snapshot
  ),
  usage_present = excluded.usage_present,
  input_tokens = excluded.input_tokens,
  output_tokens = excluded.output_tokens,
  total_tokens = excluded.total_tokens,
  cache_read_input_tokens = excluded.cache_read_input_tokens,
  cache_creation_input_tokens = excluded.cache_creation_input_tokens,
  cache_creation_5m_input_tokens = excluded.cache_creation_5m_input_tokens,
  cache_creation_1h_input_tokens = excluded.cache_creation_1h_input_tokens,
  persisted_openai_input_semantics = excluded.persisted_openai_input_semantics,
  cost_usd_femto = excluded.cost_usd_femto,
  cost_multiplier = excluded.cost_multiplier,
  cost_basis_cli_key = excluded.cost_basis_cli_key,
  cost_basis_model = excluded.cost_basis_model,
  priority_service_tier_applied = excluded.priority_service_tier_applied,
  upstream_stream_duration_ms = excluded.upstream_stream_duration_ms,
  upstream_stream_timing_version = excluded.upstream_stream_timing_version
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BackfillReport {
    pub(crate) projected_rows: u64,
    pub(crate) batches: u64,
    pub(crate) completed: bool,
    pub(crate) transitioned_to_complete: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProviderUsageProjectionBatch {
    pub(crate) last_request_log_id: i64,
    pub(crate) scanned_rows: usize,
    pub(crate) projected_rows: usize,
    pub(crate) done: bool,
}

#[derive(Debug)]
struct BackfillState {
    status: String,
    target_request_log_id: i64,
    last_request_log_id: i64,
}

#[derive(Debug)]
struct SourceRow {
    request_log_id: i64,
    trace_id: String,
    cli_key: String,
    session_id: Option<String>,
    created_at: i64,
    created_at_ms: i64,
    status: Option<i64>,
    error_present: bool,
    excluded_from_stats: bool,
    duration_ms: i64,
    ttfb_ms: Option<i64>,
    visible_ttfb_ms: Option<i64>,
    upstream_stream_duration_ms: Option<i64>,
    upstream_stream_timing_version: i64,
    requested_model: Option<String>,
    stored_final_provider_id: Option<i64>,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    total_tokens: Option<i64>,
    cache_read_input_tokens: Option<i64>,
    cache_creation_input_tokens: Option<i64>,
    cache_creation_5m_input_tokens: Option<i64>,
    cache_creation_1h_input_tokens: Option<i64>,
    usage_present: bool,
    cost_usd_femto: Option<i64>,
    cost_multiplier: f64,
    special_settings_json: Option<String>,
    attempts_json: String,
}

impl SourceRow {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        let usage_json: Option<String> = row.get(21)?;
        let input_tokens: Option<i64> = row.get(14)?;
        let output_tokens: Option<i64> = row.get(15)?;
        let total_tokens: Option<i64> = row.get(16)?;
        let cache_read_input_tokens: Option<i64> = row.get(17)?;
        let cache_creation_input_tokens: Option<i64> = row.get(18)?;
        let cache_creation_5m_input_tokens: Option<i64> = row.get(19)?;
        let cache_creation_1h_input_tokens: Option<i64> = row.get(20)?;
        let usage_present = usage_json.is_some()
            || input_tokens.is_some()
            || output_tokens.is_some()
            || total_tokens.is_some()
            || cache_read_input_tokens.is_some()
            || cache_creation_input_tokens.is_some()
            || cache_creation_5m_input_tokens.is_some()
            || cache_creation_1h_input_tokens.is_some();

        Ok(Self {
            request_log_id: row.get(0)?,
            trace_id: row.get(1)?,
            cli_key: row.get(2)?,
            session_id: row.get(3)?,
            created_at: row.get(4)?,
            created_at_ms: row.get(5)?,
            status: row.get(6)?,
            error_present: row.get::<_, Option<String>>(7)?.is_some(),
            excluded_from_stats: row.get::<_, i64>(8)? != 0,
            duration_ms: row.get(9)?,
            ttfb_ms: row.get(10)?,
            visible_ttfb_ms: row.get(11)?,
            requested_model: row.get(12)?,
            stored_final_provider_id: row.get(13)?,
            input_tokens,
            output_tokens,
            total_tokens,
            cache_read_input_tokens,
            cache_creation_input_tokens,
            cache_creation_5m_input_tokens,
            cache_creation_1h_input_tokens,
            usage_present,
            cost_usd_femto: row.get(22)?,
            cost_multiplier: row.get(23)?,
            special_settings_json: row.get(24)?,
            attempts_json: row.get(25)?,
            upstream_stream_duration_ms: row.get(26)?,
            upstream_stream_timing_version: row
                .get::<_, Option<i64>>(27)?
                .filter(|value| *value == 1)
                .unwrap_or(0),
        })
    }
}

#[derive(Debug)]
struct ProviderAttempt {
    provider_id: i64,
    provider_name: Option<String>,
    outcome: String,
}

#[derive(Debug)]
struct ExistingProjection {
    final_provider_id: Option<i64>,
    provider_name_snapshot: Option<String>,
    persisted_openai_input_semantics: bool,
}

fn parse_provider_attempts(attempts_json: &str) -> Option<Vec<ProviderAttempt>> {
    let values: Vec<serde_json::Value> = serde_json::from_str(attempts_json).ok()?;
    Some(
        values
            .into_iter()
            .filter_map(|value| {
                let object = value.as_object()?;
                let provider_id = object.get("provider_id")?.as_i64()?;
                if provider_id <= 0 {
                    return None;
                }
                let outcome = object.get("outcome")?.as_str()?.to_string();
                let provider_name = object
                    .get("provider_name")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string);
                Some(ProviderAttempt {
                    provider_id,
                    provider_name,
                    outcome,
                })
            })
            .collect(),
    )
}

fn final_provider_from_attempts(attempts_json: &str) -> Option<(i64, Option<String>)> {
    let attempts = parse_provider_attempts(attempts_json)?;
    if attempts.is_empty() || attempts.iter().all(|attempt| attempt.outcome == "skipped") {
        return None;
    }

    let picked = attempts
        .iter()
        .rev()
        .find(|attempt| attempt.outcome == "success")
        .or_else(|| {
            attempts
                .iter()
                .rev()
                .find(|attempt| attempt.outcome != "skipped")
        })?;
    Some((picked.provider_id, picked.provider_name.clone()))
}

fn normalized_final_provider_id(source: &SourceRow) -> Option<i64> {
    final_provider_from_attempts(&source.attempts_json)
        .map(|(provider_id, _)| provider_id)
        .or(source
            .stored_final_provider_id
            .filter(|provider_id| *provider_id > 0))
}

fn normalize_snapshot_name(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        // Match the SQLite compatibility view's explicit trim character set.
        // Keeping this intentionally narrow avoids cutover-only changes for
        // provider names containing non-ASCII whitespace.
        let trimmed = value.trim_matches(|ch| matches!(ch, ' ' | '\t' | '\n' | '\r'));
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

fn provider_snapshot(
    conn: &Connection,
    source: &SourceRow,
    existing: Option<&ExistingProjection>,
) -> rusqlite::Result<(Option<i64>, Option<String>, Option<bool>)> {
    let attempt_provider = final_provider_from_attempts(&source.attempts_json);
    let final_provider_id = attempt_provider
        .as_ref()
        .map(|(provider_id, _)| *provider_id)
        .or(source
            .stored_final_provider_id
            .filter(|provider_id| *provider_id > 0));
    let attempt_provider_name = attempt_provider.and_then(|(_, provider_name)| provider_name);

    let current_provider = match final_provider_id {
        Some(provider_id) => conn
            .query_row(
                r#"
SELECT name, source_provider_id, bridge_type
FROM providers
WHERE id = ?1
"#,
                [provider_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<i64>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .optional()?,
        None => None,
    };

    let existing_for_provider =
        existing.filter(|projection| projection.final_provider_id == final_provider_id);
    let provider_name_snapshot = normalize_snapshot_name(attempt_provider_name)
        .or_else(|| {
            existing_for_provider.and_then(|projection| projection.provider_name_snapshot.clone())
        })
        .or_else(|| {
            normalize_snapshot_name(current_provider.as_ref().map(|(name, _, _)| name.clone()))
        });
    let legacy_provider_bridged =
        current_provider
            .as_ref()
            .map(|(_, source_provider_id, bridge_type)| {
                crate::providers::has_bridged_input_semantics(
                    *source_provider_id,
                    bridge_type.as_deref(),
                )
            });

    Ok((
        final_provider_id,
        provider_name_snapshot,
        legacy_provider_bridged,
    ))
}

fn existing_projection(
    conn: &Connection,
    trace_id: &str,
) -> rusqlite::Result<Option<ExistingProjection>> {
    conn.query_row(
        r#"
SELECT
  final_provider_id,
  provider_name_snapshot,
  persisted_openai_input_semantics
FROM usage_ledger
WHERE trace_id = ?1
"#,
        [trace_id],
        |row| {
            Ok(ExistingProjection {
                final_provider_id: row.get(0)?,
                provider_name_snapshot: row.get(1)?,
                persisted_openai_input_semantics: row.get::<_, i64>(2)? != 0,
            })
        },
    )
    .optional()
}

fn project_source_row(conn: &Connection, source: &SourceRow) -> rusqlite::Result<()> {
    let existing = existing_projection(conn, &source.trace_id)?;
    let (final_provider_id, provider_name_snapshot, legacy_provider_bridged) =
        provider_snapshot(conn, source, existing.as_ref())?;
    let persisted_openai_input_semantics = if matches!(source.cli_key.as_str(), "codex" | "grok") {
        true
    } else if let Some(explicit_semantics) = request_logs::cx2cc_openai_input_semantics_override(
        source.special_settings_json.as_deref(),
        final_provider_id,
    ) {
        explicit_semantics
    } else {
        existing
            .as_ref()
            .filter(|projection| projection.final_provider_id == final_provider_id)
            .map(|projection| projection.persisted_openai_input_semantics)
            .or(legacy_provider_bridged)
            .unwrap_or(false)
    };
    let cost_basis = request_logs::effective_cost_basis(
        &source.cli_key,
        source.requested_model.as_deref(),
        source.special_settings_json.as_deref(),
        final_provider_id,
    );
    let (cost_basis_cli_key, cost_basis_model) = cost_basis
        .map(|basis| (Some(basis.cli_key), Some(basis.model)))
        .unwrap_or((None, None));
    let priority_service_tier_applied =
        request_logs::parse_effective_priority(source.special_settings_json.as_deref());
    let cost_multiplier = if source.cost_multiplier.is_finite() && source.cost_multiplier >= 0.0 {
        source.cost_multiplier
    } else {
        1.0
    };

    conn.execute(
        LEDGER_UPSERT_SQL,
        params![
            source.request_log_id,
            source.trace_id,
            source.cli_key,
            source.session_id,
            source.created_at,
            source.created_at_ms,
            source.status,
            if source.error_present { 1_i64 } else { 0_i64 },
            if source.excluded_from_stats {
                1_i64
            } else {
                0_i64
            },
            source.duration_ms,
            source.ttfb_ms,
            source.visible_ttfb_ms,
            source.requested_model,
            final_provider_id,
            provider_name_snapshot,
            if source.usage_present { 1_i64 } else { 0_i64 },
            source.input_tokens,
            source.output_tokens,
            source.total_tokens,
            source.cache_read_input_tokens,
            source.cache_creation_input_tokens,
            source.cache_creation_5m_input_tokens,
            source.cache_creation_1h_input_tokens,
            if persisted_openai_input_semantics {
                1_i64
            } else {
                0_i64
            },
            source.cost_usd_femto,
            cost_multiplier,
            cost_basis_cli_key,
            cost_basis_model,
            if priority_service_tier_applied {
                1_i64
            } else {
                0_i64
            },
            source.upstream_stream_duration_ms,
            source.upstream_stream_timing_version,
        ],
    )?;
    Ok(())
}

fn load_source_rows<P: rusqlite::Params>(
    conn: &Connection,
    where_and_order: &str,
    query_params: P,
) -> rusqlite::Result<Vec<SourceRow>> {
    let sql = format!("{SOURCE_SELECT}\n{where_and_order}");
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map(query_params, SourceRow::from_row)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub(crate) fn project_trace(conn: &Connection, trace_id: &str) -> rusqlite::Result<usize> {
    let rows = load_source_rows(conn, "WHERE r.trace_id = ?1 LIMIT 1", [trace_id])?;
    for row in &rows {
        project_source_row(conn, row)?;
    }
    Ok(rows.len())
}

pub(crate) fn update_missing_cost_usd_femto(
    conn: &Connection,
    trace_id: &str,
    cost_usd_femto: i64,
) -> rusqlite::Result<usize> {
    let request_logs_updated = conn.execute(
        r#"
UPDATE request_logs
SET cost_usd_femto = ?1
WHERE trace_id = ?2 AND cost_usd_femto IS NULL
"#,
        params![cost_usd_femto, trace_id],
    )?;
    if request_logs_updated > 0 {
        project_trace(conn, trace_id)?;
    }
    let ledger_updated = conn.execute(
        r#"
UPDATE usage_ledger
SET cost_usd_femto = ?1
WHERE trace_id = ?2 AND cost_usd_femto IS NULL
"#,
        params![cost_usd_femto, trace_id],
    )?;
    Ok(request_logs_updated.saturating_add(ledger_updated))
}

pub(crate) fn is_backfill_complete(conn: &Connection) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT status FROM usage_ledger_backfill_state WHERE id = 1",
        [],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .map(|status| status.as_deref() == Some(STATUS_COMPLETE))
}

fn read_backfill_state(conn: &Connection) -> rusqlite::Result<Option<BackfillState>> {
    conn.query_row(
        r#"
SELECT status, target_request_log_id, last_request_log_id
FROM usage_ledger_backfill_state
WHERE id = 1
"#,
        [],
        |row| {
            Ok(BackfillState {
                status: row.get(0)?,
                target_request_log_id: row.get(1)?,
                last_request_log_id: row.get(2)?,
            })
        },
    )
    .optional()
}

fn project_cursor_batch(
    conn: &Connection,
    cursor: i64,
    target: i64,
) -> rusqlite::Result<(usize, i64)> {
    let rows = load_source_rows(
        conn,
        r#"
WHERE r.id > ?1 AND r.id <= ?2
ORDER BY r.id ASC
LIMIT ?3
"#,
        params![cursor, target, BACKFILL_BATCH_SIZE],
    )?;
    let last_request_log_id = rows.last().map(|row| row.request_log_id).unwrap_or(target);
    for row in &rows {
        project_source_row(conn, row)?;
    }
    Ok((rows.len(), last_request_log_id))
}

fn project_missing_batch(conn: &Connection, target: i64) -> rusqlite::Result<usize> {
    let rows = load_source_rows(
        conn,
        r#"
WHERE r.id <= ?1
  AND NOT EXISTS (
    SELECT 1
    FROM usage_ledger ledger
    WHERE ledger.request_log_id = r.id
      AND ledger.trace_id = r.trace_id
  )
ORDER BY r.id ASC
LIMIT ?2
"#,
        params![target, BACKFILL_BATCH_SIZE],
    )?;
    for row in &rows {
        project_source_row(conn, row)?;
    }
    Ok(rows.len())
}

fn has_missing_rows(conn: &Connection, target: i64) -> rusqlite::Result<bool> {
    conn.query_row(
        r#"
SELECT EXISTS (
  SELECT 1
  FROM request_logs r
  WHERE r.id <= ?1
    AND NOT EXISTS (
      SELECT 1
      FROM usage_ledger ledger
      WHERE ledger.request_log_id = r.id
        AND ledger.trace_id = r.trace_id
    )
)
"#,
        [target],
        |row| row.get(0),
    )
}

pub(crate) fn project_expired_batch(
    conn: &Connection,
    cutoff: i64,
    limit: usize,
) -> rusqlite::Result<Vec<i64>> {
    let limit = i64::try_from(limit.max(1)).unwrap_or(i64::MAX);
    let rows = load_source_rows(
        conn,
        r#"
WHERE r.created_at > 0 AND r.created_at < ?1
ORDER BY r.created_at ASC, r.id ASC
LIMIT ?2
"#,
        params![cutoff, limit],
    )?;
    let mut ids = Vec::with_capacity(rows.len());
    for row in &rows {
        project_source_row(conn, row)?;
        ids.push(row.request_log_id);
    }
    Ok(ids)
}

/// Scans and projects at most `limit` rows from the fixed upgrade high-water.
pub(crate) fn project_provider_usage_batch(
    conn: &Connection,
    provider_id: i64,
    after_request_log_id: i64,
    limit: usize,
) -> rusqlite::Result<ProviderUsageProjectionBatch> {
    let state = read_backfill_state(conn)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    if state.status == STATUS_COMPLETE {
        return Ok(ProviderUsageProjectionBatch {
            last_request_log_id: after_request_log_id,
            scanned_rows: 0,
            projected_rows: 0,
            done: true,
        });
    }
    if state.status != STATUS_INCOMPLETE {
        return Err(rusqlite::Error::InvalidQuery);
    }

    let cursor = after_request_log_id.max(0);
    if cursor >= state.target_request_log_id {
        return Ok(ProviderUsageProjectionBatch {
            last_request_log_id: state.target_request_log_id,
            scanned_rows: 0,
            projected_rows: 0,
            done: true,
        });
    }

    let bounded_limit = limit.max(1);
    let sql_limit = i64::try_from(bounded_limit).unwrap_or(i64::MAX);
    let rows = load_source_rows(
        conn,
        r#"
WHERE r.id > ?1 AND r.id <= ?2
ORDER BY r.id ASC
LIMIT ?3
"#,
        params![cursor, state.target_request_log_id, sql_limit],
    )?;
    let scanned_rows = rows.len();
    let last_request_log_id = rows
        .last()
        .map(|row| row.request_log_id)
        .unwrap_or(state.target_request_log_id);
    let mut projected_rows = 0;
    for row in &rows {
        if normalized_final_provider_id(row) == Some(provider_id) {
            project_source_row(conn, row)?;
            projected_rows += 1;
        }
    }

    Ok(ProviderUsageProjectionBatch {
        last_request_log_id,
        scanned_rows,
        projected_rows,
        done: scanned_rows < bounded_limit || last_request_log_id >= state.target_request_log_id,
    })
}

pub(crate) fn run_backfill(db: &db::Db) -> AppResult<BackfillReport> {
    let mut projected_rows = 0_u64;
    let mut batches = 0_u64;

    loop {
        let mut connection = db.open_connection()?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| db_err!("failed to start usage ledger backfill batch: {error}"))?;
        let state = read_backfill_state(&tx)
            .map_err(|error| db_err!("failed to read usage ledger backfill state: {error}"))?
            .ok_or_else(|| db_err!("usage ledger backfill state is missing"))?;

        if state.status == STATUS_COMPLETE {
            tx.commit()
                .map_err(|error| db_err!("failed to close completed ledger check: {error}"))?;
            return Ok(BackfillReport {
                projected_rows,
                batches,
                completed: true,
                transitioned_to_complete: false,
            });
        }
        if state.status != STATUS_INCOMPLETE {
            return Err(db_err!(
                "invalid usage ledger backfill status: {}",
                state.status
            ));
        }

        let now = now_unix_seconds();
        if state.last_request_log_id < state.target_request_log_id {
            let (projected, next_cursor) =
                project_cursor_batch(&tx, state.last_request_log_id, state.target_request_log_id)
                    .map_err(|error| db_err!("failed to project usage ledger batch: {error}"))?;
            tx.execute(
                r#"
UPDATE usage_ledger_backfill_state
SET last_request_log_id = ?1, updated_at = ?2
WHERE id = 1 AND status = 'incomplete'
"#,
                params![next_cursor, now],
            )
            .map_err(|error| db_err!("failed to advance usage ledger cursor: {error}"))?;
            tx.commit()
                .map_err(|error| db_err!("failed to commit usage ledger batch: {error}"))?;
            projected_rows = projected_rows.saturating_add(projected as u64);
            batches = batches.saturating_add(1);
            std::thread::sleep(BACKFILL_BATCH_PAUSE);
            continue;
        }

        let repaired = project_missing_batch(&tx, state.target_request_log_id)
            .map_err(|error| db_err!("failed to reconcile usage ledger coverage: {error}"))?;
        let still_missing = has_missing_rows(&tx, state.target_request_log_id)
            .map_err(|error| db_err!("failed to verify usage ledger coverage: {error}"))?;
        if !still_missing {
            tx.execute(
                r#"
UPDATE usage_ledger_backfill_state
SET status = 'complete', completed_at = ?1, updated_at = ?1
WHERE id = 1 AND status = 'incomplete'
"#,
                [now],
            )
            .map_err(|error| db_err!("failed to complete usage ledger backfill: {error}"))?;
        } else if repaired == 0 {
            return Err(db_err!(
                "usage ledger anti-join remains non-empty after reconciliation"
            ));
        }
        tx.commit()
            .map_err(|error| db_err!("failed to commit usage ledger reconciliation: {error}"))?;
        projected_rows = projected_rows.saturating_add(repaired as u64);
        batches = batches.saturating_add(1);

        if !still_missing {
            return Ok(BackfillReport {
                projected_rows,
                batches,
                completed: true,
                transitioned_to_complete: true,
            });
        }
        std::thread::sleep(BACKFILL_BATCH_PAUSE);
    }
}

fn is_retryable_backfill_error(error: &crate::shared::error::AppError) -> bool {
    let message = error.to_string().to_ascii_lowercase();
    [
        "database is busy",
        "database is locked",
        "database table is locked",
        "sqlite_busy",
        "sqlite_locked",
    ]
    .iter()
    .any(|needle| message.contains(needle))
}

fn backfill_retry_delay(retry_attempt: u32) -> Duration {
    let exponent = retry_attempt.min(20);
    let multiplier = 1_u64.checked_shl(exponent).unwrap_or(u64::MAX);
    Duration::from_secs(
        BACKFILL_RETRY_BASE_DELAY
            .as_secs()
            .saturating_mul(multiplier)
            .min(BACKFILL_RETRY_MAX_DELAY.as_secs()),
    )
}

pub(crate) fn spawn_backfill(app: tauri::AppHandle, db: db::Db) {
    static STARTED: OnceLock<()> = OnceLock::new();
    if STARTED.set(()).is_err() {
        return;
    }

    tauri::async_runtime::spawn_blocking(move || {
        let mut retry_attempt = 0_u32;
        loop {
            match run_backfill(&db) {
                Ok(report) => {
                    tracing::info!(
                        projected_rows = report.projected_rows,
                        batches = report.batches,
                        completed = report.completed,
                        transitioned_to_complete = report.transitioned_to_complete,
                        "usage ledger background backfill finished"
                    );
                    if report.transitioned_to_complete {
                        request_logs::spawn_retention_once(app, db.clone());
                    }
                    return;
                }
                Err(error) if is_retryable_backfill_error(&error) => {
                    let delay = backfill_retry_delay(retry_attempt);
                    tracing::warn!(
                        retry_attempt = retry_attempt.saturating_add(1),
                        delay_ms = delay.as_millis(),
                        error = %error,
                        "usage ledger backfill hit sqlite busy/locked; retrying while request-log retention remains paused"
                    );
                    retry_attempt = retry_attempt.saturating_add(1);
                    std::thread::sleep(delay);
                }
                Err(error) => {
                    tracing::warn!(
                        error = %error,
                        "usage ledger background backfill failed; request-log retention remains paused"
                    );
                    return;
                }
            }
        }
    });
}

pub(crate) fn delete_ids_with_coverage(
    conn: &Connection,
    request_log_ids: &[i64],
) -> rusqlite::Result<usize> {
    if request_log_ids.is_empty() {
        return Ok(0);
    }
    let placeholders = crate::db::sql_placeholders(request_log_ids.len());
    let sql = format!(
        r#"
DELETE FROM request_logs
WHERE id IN ({placeholders})
  AND EXISTS (
    SELECT 1
    FROM usage_ledger ledger
    WHERE ledger.request_log_id = request_logs.id
      AND ledger.trace_id = request_logs.trace_id
  )
"#
    );
    conn.execute(&sql, params_from_iter(request_log_ids.iter()))
}

#[cfg(test)]
mod tests {
    use super::{
        backfill_retry_delay, final_provider_from_attempts, is_backfill_complete,
        is_retryable_backfill_error, normalize_snapshot_name, project_provider_usage_batch,
        project_trace, run_backfill,
    };
    use rusqlite::params;
    use std::time::Duration;
    use tempfile::TempDir;

    fn init_test_db() -> (crate::db::Db, TempDir) {
        let dir = TempDir::new().expect("temp dir");
        let db_path = dir.path().join("usage-ledger.sqlite");
        let db = crate::db::init_for_tests(&db_path).expect("initialize test db");
        (db, dir)
    }

    #[test]
    fn backfill_retries_only_transient_sqlite_lock_errors_with_bounded_backoff() {
        let locked = crate::shared::error::AppError::new("DB_ERROR", "sqlite: database is locked");
        let busy = crate::shared::error::AppError::new("DB_ERROR", "SQLITE_BUSY: busy");
        let malformed =
            crate::shared::error::AppError::new("DB_ERROR", "usage ledger state is missing");

        assert!(is_retryable_backfill_error(&locked));
        assert!(is_retryable_backfill_error(&busy));
        assert!(!is_retryable_backfill_error(&malformed));
        assert_eq!(backfill_retry_delay(0), Duration::from_secs(5));
        assert_eq!(backfill_retry_delay(1), Duration::from_secs(10));
        assert_eq!(backfill_retry_delay(20), Duration::from_secs(5 * 60));
    }

    fn insert_request_log(
        conn: &rusqlite::Connection,
        trace_id: &str,
        created_at: i64,
        final_provider_id: Option<i64>,
    ) -> i64 {
        conn.execute(
            r#"
INSERT INTO request_logs(
  trace_id,
  cli_key,
  session_id,
  method,
  path,
  status,
  duration_ms,
  ttfb_ms,
  visible_ttfb_ms,
  attempts_json,
  created_at,
  created_at_ms,
  input_tokens,
  output_tokens,
  total_tokens,
  usage_json,
  requested_model,
  cost_multiplier,
  excluded_from_stats,
  final_provider_id
) VALUES (
  ?1, 'claude', 'session-ledger', 'POST', '/v1/messages', 200, 25, 5, 5,
  '[]', ?2, ?3, 100, 20, 120, '{"input_tokens":100,"output_tokens":20}',
  'claude-test', 1.0, 0, ?4
)
"#,
            params![
                trace_id,
                created_at,
                created_at.saturating_mul(1000),
                final_provider_id
            ],
        )
        .expect("insert request log");
        conn.last_insert_rowid()
    }

    #[test]
    fn backfill_resumes_from_cursor_and_completes_after_anti_join() {
        let (db, _dir) = init_test_db();
        let conn = db.open_connection().expect("open connection");
        let first_id = insert_request_log(&conn, "ledger-backfill-1", 1, None);
        let _second_id = insert_request_log(&conn, "ledger-backfill-2", 2, None);
        let target_id = insert_request_log(&conn, "ledger-backfill-3", 3, None);
        project_trace(&conn, "ledger-backfill-1").expect("project first row");
        conn.execute(
            r#"
UPDATE usage_ledger_backfill_state
SET status = 'incomplete',
    target_request_log_id = ?1,
    last_request_log_id = ?2,
    completed_at = NULL
WHERE id = 1
"#,
            params![target_id, first_id],
        )
        .expect("seed resumable state");

        insert_request_log(&conn, "ledger-after-high-water", 4, None);
        project_trace(&conn, "ledger-after-high-water").expect("dual-write post-target row");
        drop(conn);

        let report = run_backfill(&db).expect("run usage ledger backfill");

        assert!(report.completed);
        assert!(report.transitioned_to_complete);
        assert!(report.projected_rows >= 2);
        let already_complete = run_backfill(&db).expect("recheck completed usage ledger");
        assert!(!already_complete.transitioned_to_complete);
        let conn = db.open_connection().expect("open connection");
        assert!(is_backfill_complete(&conn).expect("read complete state"));
        let counts: (i64, i64) = conn
            .query_row(
                r#"
SELECT
  (SELECT COUNT(1) FROM request_logs),
  (SELECT COUNT(1) FROM usage_ledger)
"#,
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read backfill counts");
        assert_eq!(counts, (4, 4));
        let visible_count: i64 = conn
            .query_row("SELECT COUNT(1) FROM usage_events", [], |row| row.get(0))
            .expect("read completed usage source");
        assert_eq!(visible_count, 4);
    }

    #[test]
    fn backfill_repairs_anti_join_before_marking_complete() {
        let (db, _dir) = init_test_db();
        let conn = db.open_connection().expect("open connection");
        insert_request_log(&conn, "ledger-antijoin-1", 1, None);
        let target_id = insert_request_log(&conn, "ledger-antijoin-2", 2, None);
        conn.execute(
            r#"
UPDATE usage_ledger_backfill_state
SET status = 'incomplete',
    target_request_log_id = ?1,
    last_request_log_id = ?1,
    completed_at = NULL
WHERE id = 1
"#,
            [target_id],
        )
        .expect("seed anti-join state");
        drop(conn);

        let report = run_backfill(&db).expect("repair anti-join");

        assert!(report.completed);
        assert_eq!(report.projected_rows, 2);
        let conn = db.open_connection().expect("open connection");
        let missing: bool = conn
            .query_row(
                r#"
SELECT EXISTS(
  SELECT 1
  FROM request_logs request
  WHERE NOT EXISTS (
    SELECT 1
    FROM usage_ledger ledger
    WHERE ledger.request_log_id = request.id
      AND ledger.trace_id = request.trace_id
  )
)
"#,
                [],
                |row| row.get(0),
            )
            .expect("verify anti-join");
        assert!(!missing);
    }

    #[test]
    fn attempts_skip_malformed_elements_without_losing_valid_success() {
        let attempts = r#"
[
  {"provider_id": 11, "provider_name": "Fallback", "outcome": "failed"},
  {"provider_id": 22, "provider_name": "Winner", "outcome": "success"},
  {"provider_id": "broken", "provider_name": "Malformed", "outcome": "success"},
  {"provider_id": 33, "provider_name": "Later failure", "outcome": "failed"}
]
"#;

        assert_eq!(
            final_provider_from_attempts(attempts),
            Some((22, Some("Winner".to_string())))
        );
        assert_eq!(
            final_provider_from_attempts(r#"[{"provider_id":44,"outcome":"success"}]"#),
            Some((44, None))
        );
    }

    #[test]
    fn provider_snapshot_name_uses_the_compatibility_view_whitespace_rules() {
        assert_eq!(normalize_snapshot_name(Some("\t \n\r".to_string())), None);
        assert_eq!(
            normalize_snapshot_name(Some("\t Provider Snapshot \n".to_string())),
            Some("Provider Snapshot".to_string())
        );
        assert_eq!(
            normalize_snapshot_name(Some("\u{00a0}".to_string())),
            Some("\u{00a0}".to_string()),
            "non-ASCII whitespace remains a name in both Rust and SQLite"
        );
    }

    #[test]
    fn provider_usage_projection_scans_fixed_high_water_in_bounded_batches() {
        let (db, _dir) = init_test_db();
        let conn = db.open_connection().expect("open connection");
        let _first_id = insert_request_log(&conn, "provider-batch-1", 1, Some(77));
        let second_id = insert_request_log(&conn, "provider-batch-2", 2, Some(88));
        let _third_id = insert_request_log(&conn, "provider-batch-3", 3, Some(77));
        let target_id = insert_request_log(&conn, "provider-batch-4", 4, Some(77));
        conn.execute(
            r#"
UPDATE usage_ledger_backfill_state
SET status = 'incomplete',
    target_request_log_id = ?1,
    last_request_log_id = 0,
    completed_at = NULL
WHERE id = 1
"#,
            [target_id],
        )
        .expect("mark provider projection fixture incomplete");

        let first = project_provider_usage_batch(&conn, 77, 0, 2)
            .expect("project first bounded provider batch");
        assert_eq!(first.last_request_log_id, second_id);
        assert_eq!((first.scanned_rows, first.projected_rows), (2, 1));
        assert!(!first.done);

        let second = project_provider_usage_batch(&conn, 77, first.last_request_log_id, 2)
            .expect("project second bounded provider batch");
        assert_eq!(second.last_request_log_id, target_id);
        assert_eq!((second.scanned_rows, second.projected_rows), (2, 2));
        assert!(second.done);

        let projected: i64 = conn
            .query_row(
                "SELECT COUNT(1) FROM usage_ledger WHERE final_provider_id = 77",
                [],
                |row| row.get(0),
            )
            .expect("count provider ledger rows");
        assert_eq!(projected, 3);
    }

    #[test]
    fn reproject_after_provider_deletion_preserves_snapshot_and_semantics() {
        let (db, _dir) = init_test_db();
        let conn = db.open_connection().expect("open connection");
        conn.execute(
            r#"
INSERT INTO providers(
  id,
  provider_uuid,
  cli_key,
  name,
  base_url,
  api_key_plaintext,
  created_at,
  updated_at,
  source_provider_id,
  bridge_type
) VALUES (
  77,
  '550e8400-e29b-41d4-a716-446655440077',
  'claude',
  'Deleted bridge',
  'https://example.invalid',
  'key',
  1,
  1,
  NULL,
  'cx2cc'
)
"#,
            [],
        )
        .expect("insert bridge provider");
        insert_request_log(&conn, "ledger-provider-snapshot", 1, Some(77));
        project_trace(&conn, "ledger-provider-snapshot").expect("project provider snapshot");
        conn.execute("DELETE FROM providers WHERE id = 77", [])
            .expect("delete provider");
        project_trace(&conn, "ledger-provider-snapshot")
            .expect("reproject after provider deletion");

        let row: (
            Option<i64>,
            Option<String>,
            bool,
            Option<String>,
            Option<String>,
        ) = conn
            .query_row(
                r#"
SELECT
  final_provider_id,
  provider_name_snapshot,
  persisted_openai_input_semantics,
  cost_basis_cli_key,
  cost_basis_model
FROM usage_ledger
WHERE trace_id = 'ledger-provider-snapshot'
"#,
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("read preserved provider snapshot");
        assert_eq!(
            row,
            (
                Some(77),
                Some("Deleted bridge".to_string()),
                true,
                Some("claude".to_string()),
                Some("claude-test".to_string())
            )
        );

        conn.execute(
            r#"
UPDATE request_logs
SET special_settings_json = ?1
WHERE trace_id = 'ledger-provider-snapshot'
"#,
            [r#"[
              {
                "type":"cx2cc_cost_basis",
                "source_cli_key":"codex",
                "priced_model":"gpt-5",
                "bridge_provider_id":88
              }
            ]"#],
        )
        .expect("write explicit scoped mismatch");
        project_trace(&conn, "ledger-provider-snapshot")
            .expect("reproject explicit false semantics");

        let explicit_false: (Option<String>, bool) = conn
            .query_row(
                r#"
SELECT provider_name_snapshot, persisted_openai_input_semantics
FROM usage_ledger
WHERE trace_id = 'ledger-provider-snapshot'
"#,
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read explicit false semantics");
        assert_eq!(explicit_false, (Some("Deleted bridge".to_string()), false));
    }
}
