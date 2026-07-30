//! Usage: SQLite migration v42->v43 - durable usage ledger and backfill state.

use rusqlite::{params, Connection};

pub(super) const USAGE_LEDGER_STATUS_INCOMPLETE: &str = "incomplete";
pub(super) const USAGE_LEDGER_STATUS_COMPLETE: &str = "complete";

pub(super) fn create_usage_ledger_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS usage_ledger (
  request_log_id INTEGER PRIMARY KEY,
  trace_id TEXT NOT NULL UNIQUE,
  cli_key TEXT NOT NULL,
  session_id TEXT,
  created_at INTEGER NOT NULL,
  created_at_ms INTEGER NOT NULL DEFAULT 0,
  status INTEGER,
  error_present INTEGER NOT NULL DEFAULT 0 CHECK(error_present IN (0, 1)),
  excluded_from_stats INTEGER NOT NULL DEFAULT 0 CHECK(excluded_from_stats IN (0, 1)),
  duration_ms INTEGER NOT NULL DEFAULT 0,
  ttfb_ms INTEGER,
  visible_ttfb_ms INTEGER,
  requested_model TEXT,
  final_provider_id INTEGER,
  provider_name_snapshot TEXT,
  usage_present INTEGER NOT NULL DEFAULT 0 CHECK(usage_present IN (0, 1)),
  input_tokens INTEGER,
  output_tokens INTEGER,
  total_tokens INTEGER,
  cache_read_input_tokens INTEGER,
  cache_creation_input_tokens INTEGER,
  cache_creation_5m_input_tokens INTEGER,
  cache_creation_1h_input_tokens INTEGER,
  persisted_openai_input_semantics INTEGER NOT NULL DEFAULT 0
    CHECK(persisted_openai_input_semantics IN (0, 1)),
  cost_usd_femto INTEGER,
  cost_multiplier REAL NOT NULL DEFAULT 1.0,
  cost_basis_cli_key TEXT,
  cost_basis_model TEXT,
  priority_service_tier_applied INTEGER NOT NULL DEFAULT 0
    CHECK(priority_service_tier_applied IN (0, 1))
);

CREATE INDEX IF NOT EXISTS idx_usage_ledger_cli_created_at_excluded
  ON usage_ledger(cli_key, created_at, excluded_from_stats);
CREATE INDEX IF NOT EXISTS idx_usage_ledger_created_at
  ON usage_ledger(created_at);
CREATE INDEX IF NOT EXISTS idx_usage_ledger_provider_created_at
  ON usage_ledger(final_provider_id, created_at);
CREATE INDEX IF NOT EXISTS idx_usage_ledger_provider_success_cost
  ON usage_ledger(final_provider_id, created_at)
  WHERE status >= 200 AND status < 300
    AND error_present = 0
    AND cost_usd_femto IS NOT NULL
    AND excluded_from_stats = 0;
CREATE INDEX IF NOT EXISTS idx_usage_ledger_session_id
  ON usage_ledger(session_id);

CREATE TABLE IF NOT EXISTS usage_ledger_backfill_state (
  id INTEGER PRIMARY KEY CHECK(id = 1),
  status TEXT NOT NULL CHECK(status IN ('incomplete', 'complete')),
  target_request_log_id INTEGER NOT NULL DEFAULT 0 CHECK(target_request_log_id >= 0),
  last_request_log_id INTEGER NOT NULL DEFAULT 0 CHECK(last_request_log_id >= 0),
  completed_at INTEGER,
  updated_at INTEGER NOT NULL,
  CHECK(last_request_log_id <= target_request_log_id)
);
"#,
    )
    .map_err(|error| format!("failed to create usage ledger schema: {error}"))
}

pub(super) fn recreate_usage_events_view(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
DROP VIEW IF EXISTS usage_events;
CREATE VIEW usage_events AS
WITH
backfill_mode AS (
  SELECT CASE WHEN EXISTS (
    SELECT 1
    FROM usage_ledger_backfill_state
    WHERE id = 1 AND status = 'complete'
  ) THEN 1 ELSE 0 END AS is_complete
),
request_raw AS (
  SELECT
    r.*,
    CASE
      WHEN json_valid(r.special_settings_json)
       AND json_type(r.special_settings_json) = 'array'
      THEN r.special_settings_json
      ELSE '[]'
    END AS safe_settings_json,
    CASE
      WHEN json_valid(r.attempts_json)
       AND json_type(r.attempts_json) = 'array'
      THEN r.attempts_json
      ELSE '[]'
    END AS safe_attempts_json
  FROM backfill_mode mode
  CROSS JOIN request_logs r
  WHERE mode.is_complete = 0
),
request_identity AS (
  SELECT
    raw.*,
    COALESCE(
      (
        SELECT json_extract(attempt.value, '$.provider_id')
        FROM json_each(raw.safe_attempts_json) attempt
        WHERE attempt.type = 'object'
          AND json_type(attempt.value, '$.outcome') = 'text'
          AND json_extract(attempt.value, '$.outcome') = 'success'
          AND json_type(attempt.value, '$.provider_id') = 'integer'
          AND typeof(json_extract(attempt.value, '$.provider_id')) = 'integer'
          AND json_extract(attempt.value, '$.provider_id') > 0
        ORDER BY CAST(attempt.key AS INTEGER) DESC
        LIMIT 1
      ),
      (
        SELECT json_extract(attempt.value, '$.provider_id')
        FROM json_each(raw.safe_attempts_json) attempt
        WHERE attempt.type = 'object'
          AND json_type(attempt.value, '$.outcome') = 'text'
          AND json_extract(attempt.value, '$.outcome') != 'skipped'
          AND json_type(attempt.value, '$.provider_id') = 'integer'
          AND typeof(json_extract(attempt.value, '$.provider_id')) = 'integer'
          AND json_extract(attempt.value, '$.provider_id') > 0
        ORDER BY CAST(attempt.key AS INTEGER) DESC
        LIMIT 1
      ),
      raw.final_provider_id
    ) AS normalized_final_provider_id
  FROM request_raw raw
),
request_base AS (
  SELECT
    identity.*,
    COALESCE(
      projected.final_provider_id,
      identity.normalized_final_provider_id
    ) AS effective_final_provider_id,
    p.name AS current_provider_name,
    p.source_provider_id AS current_source_provider_id,
    p.bridge_type AS current_bridge_type,
    projected.trace_id AS projected_trace_id,
    projected.provider_name_snapshot AS projected_provider_name_snapshot,
    projected.persisted_openai_input_semantics AS projected_openai_semantics,
    projected.cost_basis_cli_key AS projected_cost_basis_cli_key,
    projected.cost_basis_model AS projected_cost_basis_model,
    projected.priority_service_tier_applied AS projected_priority_applied
  FROM request_identity identity
  LEFT JOIN usage_ledger projected ON projected.trace_id = identity.trace_id
  LEFT JOIN providers p ON p.id = COALESCE(
    projected.final_provider_id,
    identity.normalized_final_provider_id
  )
),
request_markers AS (
  SELECT
    base.*,
    (
      SELECT marker.value
      FROM json_each(base.safe_settings_json) marker
      WHERE marker.type = 'object'
        AND json_extract(marker.value, '$.type') = 'cx2cc_cost_basis'
        AND json_type(marker.value, '$.source_cli_key') = 'text'
        AND TRIM(
          json_extract(marker.value, '$.source_cli_key'),
          char(32) || char(9) || char(10) || char(13)
        ) != ''
        AND (
          (
            json_type(marker.value, '$.bridge_provider_id') = 'integer'
            AND typeof(json_extract(marker.value, '$.bridge_provider_id')) = 'integer'
            AND json_extract(marker.value, '$.bridge_provider_id') > 0
            AND json_extract(marker.value, '$.bridge_provider_id') =
              base.effective_final_provider_id
          )
          OR (
            json_type(marker.value, '$.bridge_provider_id') IS NULL
            AND NOT EXISTS (
              SELECT 1
              FROM json_each(base.safe_settings_json) scoped
              WHERE scoped.type = 'object'
                AND json_extract(scoped.value, '$.type') = 'cx2cc_cost_basis'
                AND json_type(scoped.value, '$.source_cli_key') = 'text'
                AND TRIM(
                  json_extract(scoped.value, '$.source_cli_key'),
                  char(32) || char(9) || char(10) || char(13)
                ) != ''
                AND json_type(scoped.value, '$.bridge_provider_id') = 'integer'
                AND typeof(json_extract(scoped.value, '$.bridge_provider_id')) = 'integer'
                AND json_extract(scoped.value, '$.bridge_provider_id') > 0
            )
          )
        )
      ORDER BY
        CASE
          WHEN json_type(marker.value, '$.bridge_provider_id') = 'integer' THEN 1
          ELSE 0
        END DESC,
        CAST(marker.key AS INTEGER) DESC
      LIMIT 1
    ) AS cx2cc_marker_json,
    EXISTS (
      SELECT 1
      FROM json_each(base.safe_settings_json) scoped
      WHERE scoped.type = 'object'
        AND json_extract(scoped.value, '$.type') = 'cx2cc_cost_basis'
        AND json_type(scoped.value, '$.source_cli_key') = 'text'
        AND TRIM(
          json_extract(scoped.value, '$.source_cli_key'),
          char(32) || char(9) || char(10) || char(13)
        ) != ''
        AND json_type(scoped.value, '$.bridge_provider_id') = 'integer'
        AND typeof(json_extract(scoped.value, '$.bridge_provider_id')) = 'integer'
        AND json_extract(scoped.value, '$.bridge_provider_id') > 0
    ) AS has_scoped_cx2cc_marker,
    (
      SELECT TRIM(
        json_extract(route.value, '$.pricedModel'),
        char(32) || char(9) || char(10) || char(13)
      )
      FROM json_each(base.safe_settings_json) route
      WHERE route.type = 'object'
        AND json_extract(route.value, '$.type') = 'aio_managed_model_route'
        AND json_type(route.value, '$.applied') = 'true'
        AND json_extract(route.value, '$.applied') = 1
        AND json_type(route.value, '$.providerId') = 'integer'
        AND json_extract(route.value, '$.providerId') = base.effective_final_provider_id
        AND json_type(route.value, '$.pricedModel') = 'text'
        AND TRIM(
          json_extract(route.value, '$.pricedModel'),
          char(32) || char(9) || char(10) || char(13)
        ) != ''
      ORDER BY CAST(route.key AS INTEGER) DESC
      LIMIT 1
    ) AS managed_priced_model,
    COALESCE((
      SELECT CASE
        WHEN json_type(tier.value, '$.effectivePriority') = 'true' THEN 1
        WHEN json_type(tier.value, '$.effectivePriority') = 'false' THEN 0
        WHEN json_extract(tier.value, '$.actualServiceTier') = 'priority' THEN 1
        ELSE 0
      END
      FROM json_each(base.safe_settings_json) tier
      WHERE tier.type = 'object'
        AND json_extract(tier.value, '$.type') = 'codex_service_tier_result'
        AND (
          json_type(tier.value, '$.effectivePriority') IN ('true', 'false')
          OR (
            json_type(tier.value, '$.billingSourcePreference') IS NULL
            AND json_type(tier.value, '$.resolvedFrom') IS NULL
            AND json_type(tier.value, '$.actualServiceTier') = 'text'
          )
        )
      ORDER BY CAST(tier.key AS INTEGER) DESC
      LIMIT 1
    ), 0) AS derived_priority_applied,
    COALESCE(
      (
        SELECT CASE
          WHEN json_type(attempt.value, '$.provider_name') = 'text'
          THEN NULLIF(TRIM(
            json_extract(attempt.value, '$.provider_name'),
            char(32) || char(9) || char(10) || char(13)
          ), '')
          ELSE NULL
        END
        FROM json_each(base.safe_attempts_json) attempt
        WHERE attempt.type = 'object'
          AND json_type(attempt.value, '$.outcome') = 'text'
          AND json_extract(attempt.value, '$.outcome') = 'success'
          AND json_type(attempt.value, '$.provider_id') = 'integer'
          AND typeof(json_extract(attempt.value, '$.provider_id')) = 'integer'
          AND json_extract(attempt.value, '$.provider_id') > 0
          AND json_extract(attempt.value, '$.provider_id') =
            base.effective_final_provider_id
        ORDER BY CAST(attempt.key AS INTEGER) DESC
        LIMIT 1
      ),
      (
        SELECT CASE
          WHEN json_type(attempt.value, '$.provider_name') = 'text'
          THEN NULLIF(TRIM(
            json_extract(attempt.value, '$.provider_name'),
            char(32) || char(9) || char(10) || char(13)
          ), '')
          ELSE NULL
        END
        FROM json_each(base.safe_attempts_json) attempt
        WHERE attempt.type = 'object'
          AND json_type(attempt.value, '$.outcome') = 'text'
          AND json_extract(attempt.value, '$.outcome') != 'skipped'
          AND json_type(attempt.value, '$.provider_id') = 'integer'
          AND typeof(json_extract(attempt.value, '$.provider_id')) = 'integer'
          AND json_extract(attempt.value, '$.provider_id') > 0
          AND json_extract(attempt.value, '$.provider_id') =
            base.effective_final_provider_id
        ORDER BY CAST(attempt.key AS INTEGER) DESC
        LIMIT 1
      ),
      NULLIF(TRIM(
        base.current_provider_name,
        char(32) || char(9) || char(10) || char(13)
      ), '')
    ) AS derived_provider_name_snapshot
  FROM request_base base
),
request_normalized AS (
  SELECT
    markers.*,
    CASE
      WHEN json_type(markers.cx2cc_marker_json, '$.priced_model') = 'text'
      THEN NULLIF(TRIM(
        json_extract(markers.cx2cc_marker_json, '$.priced_model'),
        char(32) || char(9) || char(10) || char(13)
      ), '')
      ELSE NULL
    END AS cx2cc_priced_model,
    CASE
      WHEN json_type(markers.cx2cc_marker_json, '$.source_cli_key') = 'text'
      THEN NULLIF(TRIM(
        json_extract(markers.cx2cc_marker_json, '$.source_cli_key'),
        char(32) || char(9) || char(10) || char(13)
      ), '')
      ELSE NULL
    END AS cx2cc_source_cli_key
  FROM request_markers markers
)
SELECT
  ledger.request_log_id AS id,
  ledger.trace_id,
  ledger.cli_key,
  ledger.session_id,
  ledger.created_at,
  ledger.created_at_ms,
  ledger.status,
  ledger.error_present,
  CASE WHEN ledger.error_present = 1 THEN 'present' ELSE NULL END AS error_code,
  ledger.excluded_from_stats,
  ledger.duration_ms,
  ledger.ttfb_ms,
  ledger.visible_ttfb_ms,
  ledger.requested_model,
  ledger.final_provider_id,
  ledger.provider_name_snapshot,
  ledger.usage_present,
  ledger.input_tokens,
  ledger.output_tokens,
  ledger.total_tokens,
  ledger.cache_read_input_tokens,
  ledger.cache_creation_input_tokens,
  ledger.cache_creation_5m_input_tokens,
  ledger.cache_creation_1h_input_tokens,
  ledger.persisted_openai_input_semantics,
  ledger.cost_usd_femto,
  ledger.cost_multiplier,
  ledger.cost_basis_cli_key,
  ledger.cost_basis_model,
  ledger.priority_service_tier_applied
FROM backfill_mode mode
CROSS JOIN usage_ledger ledger
WHERE mode.is_complete = 1
UNION ALL
SELECT
  request.id,
  request.trace_id,
  request.cli_key,
  request.session_id,
  request.created_at,
  request.created_at_ms,
  request.status,
  CASE WHEN request.error_code IS NULL THEN 0 ELSE 1 END AS error_present,
  CASE WHEN request.error_code IS NULL THEN NULL ELSE 'present' END AS error_code,
  request.excluded_from_stats,
  request.duration_ms,
  request.ttfb_ms,
  request.visible_ttfb_ms,
  request.requested_model,
  request.effective_final_provider_id AS final_provider_id,
  CASE
    WHEN request.projected_trace_id IS NOT NULL
    THEN request.projected_provider_name_snapshot
    ELSE request.derived_provider_name_snapshot
  END AS provider_name_snapshot,
  CASE WHEN (
    request.usage_json IS NOT NULL OR
    request.input_tokens IS NOT NULL OR
    request.output_tokens IS NOT NULL OR
    request.total_tokens IS NOT NULL OR
    request.cache_read_input_tokens IS NOT NULL OR
    request.cache_creation_input_tokens IS NOT NULL OR
    request.cache_creation_5m_input_tokens IS NOT NULL OR
    request.cache_creation_1h_input_tokens IS NOT NULL
  ) THEN 1 ELSE 0 END AS usage_present,
  request.input_tokens,
  request.output_tokens,
  request.total_tokens,
  request.cache_read_input_tokens,
  request.cache_creation_input_tokens,
  request.cache_creation_5m_input_tokens,
  request.cache_creation_1h_input_tokens,
  CASE
    WHEN request.projected_trace_id IS NOT NULL
    THEN request.projected_openai_semantics
    WHEN request.cli_key IN ('codex', 'grok') THEN 1
    WHEN request.cx2cc_marker_json IS NOT NULL
    THEN CASE WHEN request.cx2cc_source_cli_key = 'codex' THEN 1 ELSE 0 END
    WHEN request.has_scoped_cx2cc_marker = 1 THEN 0
    WHEN request.current_source_provider_id IS NOT NULL
      OR request.current_bridge_type = 'cx2cc' THEN 1
    ELSE 0
  END AS persisted_openai_input_semantics,
  request.cost_usd_femto,
  request.cost_multiplier,
  CASE
    WHEN request.projected_trace_id IS NOT NULL
    THEN request.projected_cost_basis_cli_key
    WHEN COALESCE(
      request.cx2cc_priced_model,
      request.managed_priced_model,
      NULLIF(TRIM(request.requested_model), '')
    ) IS NULL THEN NULL
    WHEN request.cx2cc_priced_model IS NOT NULL
    THEN request.cx2cc_source_cli_key
    ELSE request.cli_key
  END AS cost_basis_cli_key,
  CASE
    WHEN request.projected_trace_id IS NOT NULL
    THEN request.projected_cost_basis_model
    ELSE COALESCE(
      request.cx2cc_priced_model,
      request.managed_priced_model,
      NULLIF(TRIM(request.requested_model), '')
    )
  END AS cost_basis_model,
  CASE
    WHEN request.projected_trace_id IS NOT NULL
    THEN request.projected_priority_applied
    ELSE request.derived_priority_applied
  END AS priority_service_tier_applied
FROM request_normalized request;
"#,
    )
    .map_err(|error| format!("failed to create usage_events view: {error}"))
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
        [table],
        |row| row.get(0),
    )
    .map_err(|error| format!("failed to inspect table {table}: {error}"))
}

pub(super) fn migrate_v42_to_v43(conn: &mut Connection) -> Result<(), String> {
    let tx = conn
        .transaction()
        .map_err(|error| format!("failed to start v42->v43: {error}"))?;

    create_usage_ledger_schema(&tx)?;
    // `usage_events` joins these provider columns. Add them in this
    // transaction before creating the view so direct v42 upgrades cannot
    // commit a view that references missing columns.
    super::ensure::ensure_provider_bridge_columns(&tx)?;

    // Historical development schemas can be partial. A missing request_logs
    // table means there is no usage history to backfill.
    let target_request_log_id: i64 = if table_exists(&tx, "request_logs")? {
        tx.query_row("SELECT COALESCE(MAX(id), 0) FROM request_logs", [], |row| {
            row.get(0)
        })
        .map_err(|error| format!("failed to capture usage ledger high-water mark: {error}"))?
    } else {
        0
    };
    let now: i64 = tx
        .query_row("SELECT CAST(strftime('%s', 'now') AS INTEGER)", [], |row| {
            row.get(0)
        })
        .map_err(|error| format!("failed to read migration timestamp: {error}"))?;
    let status = if target_request_log_id == 0 {
        USAGE_LEDGER_STATUS_COMPLETE
    } else {
        USAGE_LEDGER_STATUS_INCOMPLETE
    };
    let completed_at = (target_request_log_id == 0).then_some(now);

    tx.execute(
        r#"
INSERT INTO usage_ledger_backfill_state(
  id,
  status,
  target_request_log_id,
  last_request_log_id,
  completed_at,
  updated_at
) VALUES (1, ?1, ?2, 0, ?3, ?4)
ON CONFLICT(id) DO UPDATE SET
  status = excluded.status,
  target_request_log_id = excluded.target_request_log_id,
  last_request_log_id = 0,
  completed_at = excluded.completed_at,
  updated_at = excluded.updated_at
"#,
        params![status, target_request_log_id, completed_at, now],
    )
    .map_err(|error| format!("failed to initialize usage ledger backfill state: {error}"))?;

    recreate_usage_events_view(&tx)?;
    super::set_user_version(&tx, 43)?;
    tx.commit()
        .map_err(|error| format!("failed to commit v42->v43: {error}"))?;
    Ok(())
}
