//! Usage: Provider spend-limit gating (5h/daily/weekly/monthly/total).

use super::context::CommonCtx;
use crate::providers;
use crate::shared::error::db_err;
use rusqlite::{params, Connection};

pub(super) struct ProviderLimitsInput<'a, R: tauri::Runtime = tauri::Wry> {
    pub(super) ctx: CommonCtx<'a, R>,
    pub(super) provider: &'a providers::ProviderForGateway,
    pub(super) earliest_available_unix: &'a mut Option<i64>,
    pub(super) skipped_limits: &'a mut usize,
}

const USD_FEMTO_DENOM: f64 = 1_000_000_000_000_000.0;
const WINDOW_5H_SECS: i64 = 5 * 60 * 60;
const WINDOW_24H_SECS: i64 = 24 * 60 * 60;

fn update_earliest(earliest: &mut Option<i64>, candidate: i64) {
    if candidate <= 0 {
        return;
    }
    match earliest {
        Some(existing) if *existing <= candidate => {}
        _ => *earliest = Some(candidate),
    }
}

fn update_latest(latest: &mut Option<i64>, candidate: i64) {
    if candidate <= 0 {
        return;
    }
    match latest {
        Some(existing) if *existing >= candidate => {}
        _ => *latest = Some(candidate),
    }
}

fn limit_usd_to_femto(limit_usd: f64) -> Option<i128> {
    if !limit_usd.is_finite() || limit_usd < 0.0 {
        return None;
    }
    if limit_usd == 0.0 {
        return Some(0);
    }

    let limit_femto = (limit_usd * USD_FEMTO_DENOM).round();
    if !limit_femto.is_finite() {
        return None;
    }

    let limit_femto = limit_femto as i128;
    if limit_femto <= 0 {
        // Ensure tiny positive limits never collapse to zero due to rounding.
        return Some(1);
    }

    Some(limit_femto)
}

fn limit_exceeded(limit_usd: f64, spent_femto: i128) -> bool {
    let Some(limit_femto) = limit_usd_to_femto(limit_usd) else {
        return false;
    };
    spent_femto.max(0) >= limit_femto
}

fn has_any_limit(provider: &providers::ProviderForGateway) -> bool {
    provider.limit_5h_usd.is_some()
        || provider.limit_daily_usd.is_some()
        || provider.limit_weekly_usd.is_some()
        || provider.limit_monthly_usd.is_some()
        || provider.limit_total_usd.is_some()
}

#[derive(Debug, Clone, Copy, Default)]
struct SpendSums {
    spent_5h: i128,
    spent_daily_rolling: i128,
    spent_daily_fixed: i128,
    spent_weekly: i128,
    spent_monthly: i128,
    spent_total: i128,
}

fn min_start_ts(values: &[Option<i64>]) -> Option<i64> {
    values.iter().copied().flatten().min()
}

#[derive(Debug, Clone, Copy)]
struct SpendQueryBounds {
    start_5h: Option<i64>,
    start_daily_rolling: Option<i64>,
    start_daily_fixed: Option<i64>,
    start_weekly: Option<i64>,
    start_monthly: Option<i64>,
    end_ts: i64,
    min_start: Option<i64>,
}

/// The ledger is authoritative only after its fixed-high-water backfill has
/// completed. While it is incomplete, the gateway hot path must use the
/// request-log provider index directly instead of expanding `usage_events`'s
/// attribution compatibility view for every candidate provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProviderCostSource {
    UsageLedger,
    RequestLogs,
}

const USAGE_LEDGER_COST_WINDOW_ROWS_SQL: &str = r#"
SELECT
  created_at,
  cost_usd_femto
FROM usage_ledger
WHERE excluded_from_stats = 0
  AND status >= 200 AND status < 300 AND error_present = 0
  AND cost_usd_femto IS NOT NULL
  AND final_provider_id = ?1
  AND created_at < ?2
  AND (?3 IS NULL OR created_at >= ?3)
"#;

const REQUEST_LOG_COST_WINDOW_ROWS_SQL: &str = r#"
SELECT
  created_at,
  cost_usd_femto
FROM request_logs
WHERE excluded_from_stats = 0
  AND status >= 200 AND status < 300 AND error_code IS NULL
  AND cost_usd_femto IS NOT NULL
  AND final_provider_id = ?1
  AND created_at < ?2
  AND (?3 IS NULL OR created_at >= ?3)
"#;

// Older request-log rows can lack the persisted final provider id. Keep the
// expensive attempts fallback strictly scoped to that indexed NULL subset;
// current rows use the fast provider-id branch above.
const REQUEST_LOG_NULL_PROVIDER_COST_WINDOW_ROWS_SQL: &str = r#"
SELECT
  r.created_at,
  r.cost_usd_femto
FROM request_logs r
WHERE r.excluded_from_stats = 0
  AND r.status >= 200 AND r.status < 300 AND r.error_code IS NULL
  AND r.cost_usd_femto IS NOT NULL
  AND r.final_provider_id IS NULL
  AND r.created_at < ?2
  AND (?3 IS NULL OR r.created_at >= ?3)
  AND COALESCE(
    (
      SELECT json_extract(attempt.value, '$.provider_id')
      FROM json_each(CASE
        WHEN json_valid(r.attempts_json)
         AND json_type(r.attempts_json) = 'array'
        THEN r.attempts_json
        ELSE '[]'
      END) attempt
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
      FROM json_each(CASE
        WHEN json_valid(r.attempts_json)
         AND json_type(r.attempts_json) = 'array'
        THEN r.attempts_json
        ELSE '[]'
      END) attempt
      WHERE attempt.type = 'object'
        AND json_type(attempt.value, '$.outcome') = 'text'
        AND json_extract(attempt.value, '$.outcome') != 'skipped'
        AND json_type(attempt.value, '$.provider_id') = 'integer'
        AND typeof(json_extract(attempt.value, '$.provider_id')) = 'integer'
        AND json_extract(attempt.value, '$.provider_id') > 0
      ORDER BY CAST(attempt.key AS INTEGER) DESC
      LIMIT 1
    )
  ) = ?1
"#;

const USAGE_LEDGER_COST_BUCKET_ROWS_SQL: &str = r#"
SELECT
  created_at,
  cost_usd_femto
FROM usage_ledger
WHERE excluded_from_stats = 0
  AND status >= 200 AND status < 300 AND error_present = 0
  AND cost_usd_femto IS NOT NULL
  AND final_provider_id = ?1
  AND created_at >= ?2 AND created_at < ?3
ORDER BY created_at ASC
"#;

const REQUEST_LOG_COST_BUCKET_ROWS_SQL: &str = r#"
SELECT
  created_at,
  cost_usd_femto
FROM request_logs
WHERE excluded_from_stats = 0
  AND status >= 200 AND status < 300 AND error_code IS NULL
  AND cost_usd_femto IS NOT NULL
  AND final_provider_id = ?1
  AND created_at >= ?2 AND created_at < ?3
ORDER BY created_at ASC
"#;

const REQUEST_LOG_NULL_PROVIDER_COST_BUCKET_ROWS_SQL: &str = r#"
SELECT
  r.created_at,
  r.cost_usd_femto
FROM request_logs r
WHERE r.excluded_from_stats = 0
  AND r.status >= 200 AND r.status < 300 AND r.error_code IS NULL
  AND r.cost_usd_femto IS NOT NULL
  AND r.final_provider_id IS NULL
  AND r.created_at >= ?2 AND r.created_at < ?3
  AND COALESCE(
    (
      SELECT json_extract(attempt.value, '$.provider_id')
      FROM json_each(CASE
        WHEN json_valid(r.attempts_json)
         AND json_type(r.attempts_json) = 'array'
        THEN r.attempts_json
        ELSE '[]'
      END) attempt
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
      FROM json_each(CASE
        WHEN json_valid(r.attempts_json)
         AND json_type(r.attempts_json) = 'array'
        THEN r.attempts_json
        ELSE '[]'
      END) attempt
      WHERE attempt.type = 'object'
        AND json_type(attempt.value, '$.outcome') = 'text'
        AND json_extract(attempt.value, '$.outcome') != 'skipped'
        AND json_type(attempt.value, '$.provider_id') = 'integer'
        AND typeof(json_extract(attempt.value, '$.provider_id')) = 'integer'
        AND json_extract(attempt.value, '$.provider_id') > 0
      ORDER BY CAST(attempt.key AS INTEGER) DESC
      LIMIT 1
    )
  ) = ?1
ORDER BY r.created_at ASC
"#;

fn provider_cost_source(conn: &Connection) -> crate::shared::error::AppResult<ProviderCostSource> {
    crate::usage_ledger::is_backfill_complete(conn)
        .map(|complete| {
            if complete {
                ProviderCostSource::UsageLedger
            } else {
                ProviderCostSource::RequestLogs
            }
        })
        .map_err(|error| {
            db_err!("failed to read usage ledger backfill state for provider limits: {error}")
        })
}

fn visit_cost_window_rows(
    conn: &Connection,
    sql: &str,
    provider_id: i64,
    end_ts: i64,
    min_start: Option<i64>,
    mut visit: impl FnMut(i64, i128),
) -> crate::shared::error::AppResult<()> {
    let mut stmt = conn
        .prepare_cached(sql)
        .map_err(|error| db_err!("failed to prepare provider cost window query: {error}"))?;
    let rows = stmt
        .query_map(params![provider_id, end_ts, min_start], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|error| db_err!("failed to query provider cost windows: {error}"))?;

    for row in rows {
        let (created_at, raw_cost) =
            row.map_err(|error| db_err!("failed to read provider cost window row: {error}"))?;
        visit(created_at, i128::from(raw_cost.max(0)));
    }
    Ok(())
}

fn append_cost_bucket_rows(
    conn: &Connection,
    sql: &str,
    provider_id: i64,
    start_ts: i64,
    end_ts: i64,
    rows_out: &mut Vec<(i64, i128)>,
) -> crate::shared::error::AppResult<()> {
    let mut stmt = conn
        .prepare_cached(sql)
        .map_err(|error| db_err!("failed to prepare provider cost bucket query: {error}"))?;
    let rows = stmt
        .query_map(params![provider_id, start_ts, end_ts], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|error| db_err!("failed to query provider cost buckets: {error}"))?;
    for row in rows {
        let (created_at, raw_cost) =
            row.map_err(|error| db_err!("failed to read provider cost bucket row: {error}"))?;
        rows_out.push((created_at, i128::from(raw_cost.max(0))));
    }
    Ok(())
}

fn sum_cost_usd_femto_windows(
    conn: &Connection,
    provider_id: i64,
    bounds: SpendQueryBounds,
) -> crate::shared::error::AppResult<SpendSums> {
    let SpendQueryBounds {
        start_5h,
        start_daily_rolling,
        start_daily_fixed,
        start_weekly,
        start_monthly,
        end_ts,
        min_start,
    } = bounds;

    let mut sums = SpendSums::default();
    let mut accumulate = |created_at: i64, cost: i128| {
        sums.spent_total = sums.spent_total.saturating_add(cost);
        if start_5h.is_some_and(|start| created_at >= start) {
            sums.spent_5h = sums.spent_5h.saturating_add(cost);
        }
        if start_daily_rolling.is_some_and(|start| created_at >= start) {
            sums.spent_daily_rolling = sums.spent_daily_rolling.saturating_add(cost);
        }
        if start_daily_fixed.is_some_and(|start| created_at >= start) {
            sums.spent_daily_fixed = sums.spent_daily_fixed.saturating_add(cost);
        }
        if start_weekly.is_some_and(|start| created_at >= start) {
            sums.spent_weekly = sums.spent_weekly.saturating_add(cost);
        }
        if start_monthly.is_some_and(|start| created_at >= start) {
            sums.spent_monthly = sums.spent_monthly.saturating_add(cost);
        }
    };

    match provider_cost_source(conn)? {
        ProviderCostSource::UsageLedger => visit_cost_window_rows(
            conn,
            USAGE_LEDGER_COST_WINDOW_ROWS_SQL,
            provider_id,
            end_ts,
            min_start,
            &mut accumulate,
        )?,
        ProviderCostSource::RequestLogs => {
            visit_cost_window_rows(
                conn,
                REQUEST_LOG_COST_WINDOW_ROWS_SQL,
                provider_id,
                end_ts,
                min_start,
                &mut accumulate,
            )?;
            visit_cost_window_rows(
                conn,
                REQUEST_LOG_NULL_PROVIDER_COST_WINDOW_ROWS_SQL,
                provider_id,
                end_ts,
                min_start,
                &mut accumulate,
            )?;
        }
    }

    Ok(sums)
}

fn fetch_cost_buckets(
    conn: &Connection,
    provider_id: i64,
    start_ts: i64,
    end_ts: i64,
) -> crate::shared::error::AppResult<Vec<(i64, i128)>> {
    let mut raw_rows = Vec::new();
    match provider_cost_source(conn)? {
        ProviderCostSource::UsageLedger => append_cost_bucket_rows(
            conn,
            USAGE_LEDGER_COST_BUCKET_ROWS_SQL,
            provider_id,
            start_ts,
            end_ts,
            &mut raw_rows,
        )?,
        ProviderCostSource::RequestLogs => {
            append_cost_bucket_rows(
                conn,
                REQUEST_LOG_COST_BUCKET_ROWS_SQL,
                provider_id,
                start_ts,
                end_ts,
                &mut raw_rows,
            )?;
            append_cost_bucket_rows(
                conn,
                REQUEST_LOG_NULL_PROVIDER_COST_BUCKET_ROWS_SQL,
                provider_id,
                start_ts,
                end_ts,
                &mut raw_rows,
            )?;
        }
    }

    raw_rows.sort_unstable_by_key(|(created_at, _)| *created_at);
    let mut out: Vec<(i64, i128)> = Vec::with_capacity(raw_rows.len());
    for (ts, cost) in raw_rows {
        if let Some((last_ts, last_cost)) = out.last_mut() {
            if *last_ts == ts {
                *last_cost = last_cost.saturating_add(cost);
                continue;
            }
        }
        out.push((ts, cost));
    }
    Ok(out)
}

fn compute_next_available_rolling_from_buckets(
    buckets: &[(i64, i128)],
    window_start: i64,
    window_secs: i64,
    limit_femto: i128,
) -> Option<i64> {
    if window_secs <= 0 {
        return None;
    }
    if limit_femto <= 0 {
        return None;
    }

    let mut total: i128 = 0;
    for (ts, cost) in buckets.iter().copied() {
        if ts < window_start {
            continue;
        }
        total = total.saturating_add(cost.max(0));
    }
    if total < limit_femto {
        return None;
    }

    let threshold = total.saturating_sub(limit_femto).saturating_add(1);
    let mut prefix: i128 = 0;
    for (ts, cost) in buckets.iter().copied() {
        if ts < window_start {
            continue;
        }
        prefix = prefix.saturating_add(cost.max(0));
        if prefix >= threshold {
            return Some(ts.saturating_add(1).saturating_add(window_secs));
        }
    }

    None
}

fn parse_reset_time_hms_lossy(input: &str) -> (u8, u8, u8) {
    let trimmed = input.trim();
    let mut parts = trimmed.split(':');

    let h_raw = parts.next().unwrap_or("0");
    let m_raw = parts.next().unwrap_or("0");
    let s_raw = parts.next().unwrap_or("0");

    let h = h_raw.parse::<u8>().ok().filter(|v| *v <= 23).unwrap_or(0);
    let m = m_raw.parse::<u8>().ok().filter(|v| *v <= 59).unwrap_or(0);
    let s = s_raw.parse::<u8>().ok().filter(|v| *v <= 59).unwrap_or(0);
    (h, m, s)
}

fn compute_daily_fixed_bounds(
    conn: &Connection,
    now_unix: i64,
    reset_time: &str,
) -> crate::shared::error::AppResult<(i64, i64)> {
    let (h, m, s) = parse_reset_time_hms_lossy(reset_time);
    let mod_h = format!("+{h} hours");
    let mod_m = format!("+{m} minutes");
    let mod_s = format!("+{s} seconds");

    conn.query_row(
        r#"
WITH bounds AS (
  SELECT
    CAST(strftime('%s', ?1, 'unixepoch','localtime','start of day', ?2, ?3, ?4, 'utc') AS INTEGER) AS today_reset,
    CAST(strftime('%s', ?1, 'unixepoch','localtime','start of day','-1 day', ?2, ?3, ?4, 'utc') AS INTEGER) AS yesterday_reset,
    CAST(strftime('%s', ?1, 'unixepoch','localtime','start of day','+1 day', ?2, ?3, ?4, 'utc') AS INTEGER) AS tomorrow_reset
)
SELECT
  CASE WHEN ?1 >= today_reset THEN today_reset ELSE yesterday_reset END AS start_ts,
  CASE WHEN ?1 < today_reset THEN today_reset ELSE tomorrow_reset END AS next_reset
FROM bounds
"#,
        params![now_unix, mod_h, mod_m, mod_s],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
    )
    .map_err(|e| db_err!("failed to compute daily reset bounds: {e}"))
}

fn compute_weekly_bounds(
    conn: &Connection,
    now_unix: i64,
) -> crate::shared::error::AppResult<(i64, i64)> {
    conn.query_row(
        r#"
WITH w AS (
  SELECT (CAST(strftime('%w', ?1, 'unixepoch','localtime') AS INTEGER) + 6) % 7 AS offset
)
SELECT
  CAST(strftime('%s', ?1, 'unixepoch','localtime','start of day', printf('-%d days', offset), 'utc') AS INTEGER) AS start_ts,
  CAST(strftime('%s', ?1, 'unixepoch','localtime','start of day', printf('+%d days', 7 - offset), 'utc') AS INTEGER) AS next_reset
FROM w
"#,
        params![now_unix],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
    )
    .map_err(|e| db_err!("failed to compute weekly bounds: {e}"))
}

fn compute_monthly_bounds(
    conn: &Connection,
    now_unix: i64,
) -> crate::shared::error::AppResult<(i64, i64)> {
    conn.query_row(
        r#"
SELECT
  CAST(strftime('%s', ?1, 'unixepoch','localtime','start of month','utc') AS INTEGER) AS start_ts,
  CAST(strftime('%s', ?1, 'unixepoch','localtime','start of month','+1 month','utc') AS INTEGER) AS next_reset
"#,
        params![now_unix],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
    )
    .map_err(|e| db_err!("failed to compute monthly bounds: {e}"))
}

/// Resolve the fixed 5h window start for a provider.
/// Reads stored `window_5h_start_ts`; if NULL or expired, sets it to `now_unix` (the current request time).
fn resolve_fixed_5h_start(
    conn: &Connection,
    provider_id: i64,
    now_unix: i64,
) -> crate::shared::error::AppResult<i64> {
    let stored: Option<i64> = conn
        .query_row(
            "SELECT window_5h_start_ts FROM providers WHERE id = ?1",
            params![provider_id],
            |row| row.get(0),
        )
        .map_err(|e| db_err!("failed to read window_5h_start_ts: {e}"))?;

    if let Some(start_ts) = stored {
        let window_end = start_ts.saturating_add(WINDOW_5H_SECS);
        if now_unix < window_end {
            return Ok(start_ts);
        }
    }

    // Window expired or null -> start a new window from the current request
    conn.execute(
        "UPDATE providers SET window_5h_start_ts = ?1 WHERE id = ?2",
        params![now_unix, provider_id],
    )
    .map_err(|e| db_err!("failed to update window_5h_start_ts: {e}"))?;

    Ok(now_unix)
}

pub(super) fn gate_provider<R: tauri::Runtime>(input: ProviderLimitsInput<'_, R>) -> bool {
    let ProviderLimitsInput {
        ctx,
        provider,
        earliest_available_unix,
        skipped_limits,
    } = input;

    let has_oauth_quota_gate = provider.auth_mode == "oauth";
    let has_spend_limit = has_any_limit(provider);
    if !has_oauth_quota_gate && !has_spend_limit {
        return true;
    }

    let conn = match ctx.state.db.open_connection() {
        Ok(conn) => conn,
        Err(_) => return true,
    };

    let now_unix = ctx.created_at;
    let end_unix = now_unix.saturating_add(1);

    if has_oauth_quota_gate {
        match crate::domain::provider_oauth_limits::gate_snapshot(&conn, provider.id, now_unix) {
            Ok(crate::domain::provider_oauth_limits::OAuthLimitGate::Allow) => {}
            Ok(crate::domain::provider_oauth_limits::OAuthLimitGate::Limited { reset_at }) => {
                *skipped_limits = skipped_limits.saturating_add(1);
                if let Some(reset_at) = reset_at {
                    update_earliest(earliest_available_unix, reset_at);
                }
                return false;
            }
            Err(err) => {
                tracing::warn!(
                    provider_id = provider.id,
                    provider_name = %provider.name,
                    "failed to gate OAuth provider quota snapshot: {err}"
                );
            }
        }
    }

    if !has_spend_limit {
        return true;
    }

    // Use fixed window for 5h limit
    let start_5h = if provider.limit_5h_usd.is_some() {
        match resolve_fixed_5h_start(&conn, provider.id, now_unix) {
            Ok(ts) => Some(ts),
            Err(_) => return true,
        }
    } else {
        None
    };

    let (start_daily_rolling, start_daily_fixed, next_daily_fixed) =
        match (provider.limit_daily_usd, provider.daily_reset_mode) {
            (Some(_), providers::DailyResetMode::Rolling) => {
                (Some(now_unix.saturating_sub(WINDOW_24H_SECS)), None, None)
            }
            (Some(_), providers::DailyResetMode::Fixed) => {
                let (start, next) = match compute_daily_fixed_bounds(
                    &conn,
                    now_unix,
                    provider.daily_reset_time.as_str(),
                ) {
                    Ok(v) => v,
                    Err(_) => return true,
                };
                (None, Some(start), Some(next))
            }
            _ => (None, None, None),
        };

    let (start_weekly, next_weekly) = if provider.limit_weekly_usd.is_some() {
        match compute_weekly_bounds(&conn, now_unix) {
            Ok((start, next)) => (Some(start), Some(next)),
            Err(_) => return true,
        }
    } else {
        (None, None)
    };

    let (start_monthly, next_monthly) = if provider.limit_monthly_usd.is_some() {
        match compute_monthly_bounds(&conn, now_unix) {
            Ok((start, next)) => (Some(start), Some(next)),
            Err(_) => return true,
        }
    } else {
        (None, None)
    };

    let needs_total = provider.limit_total_usd.is_some();
    let min_start = if needs_total {
        None
    } else {
        min_start_ts(&[
            start_5h,
            start_daily_rolling,
            start_daily_fixed,
            start_weekly,
            start_monthly,
        ])
    };

    let sums = match sum_cost_usd_femto_windows(
        &conn,
        provider.id,
        SpendQueryBounds {
            start_5h,
            start_daily_rolling,
            start_daily_fixed,
            start_weekly,
            start_monthly,
            end_ts: end_unix,
            min_start,
        },
    ) {
        Ok(v) => v,
        Err(_) => return true,
    };

    let mut exceeded = false;
    let mut provider_next_available: Option<i64> = None;
    let mut need_rolling_5h = false;
    let mut need_rolling_daily = false;

    if let Some(limit) = provider.limit_5h_usd {
        if limit_exceeded(limit, sums.spent_5h) {
            exceeded = true;
            need_rolling_5h = true;
        }
    }

    if let Some(limit) = provider.limit_daily_usd {
        match provider.daily_reset_mode {
            providers::DailyResetMode::Rolling => {
                if limit_exceeded(limit, sums.spent_daily_rolling) {
                    exceeded = true;
                    need_rolling_daily = true;
                }
            }
            providers::DailyResetMode::Fixed => {
                if limit_exceeded(limit, sums.spent_daily_fixed) {
                    exceeded = true;
                    if let Some(next_reset) = next_daily_fixed {
                        update_latest(&mut provider_next_available, next_reset);
                    }
                }
            }
        }
    }

    if let Some(limit) = provider.limit_weekly_usd {
        if limit_exceeded(limit, sums.spent_weekly) {
            exceeded = true;
            if let Some(next_reset) = next_weekly {
                update_latest(&mut provider_next_available, next_reset);
            }
        }
    }

    if let Some(limit) = provider.limit_monthly_usd {
        if limit_exceeded(limit, sums.spent_monthly) {
            exceeded = true;
            if let Some(next_reset) = next_monthly {
                update_latest(&mut provider_next_available, next_reset);
            }
        }
    }

    if let Some(limit) = provider.limit_total_usd {
        if limit_exceeded(limit, sums.spent_total) {
            exceeded = true;
        }
    }

    if !exceeded {
        return true;
    }

    if need_rolling_5h || need_rolling_daily {
        let mut buckets_start: Option<i64> = None;
        if need_rolling_daily {
            buckets_start = start_daily_rolling;
        }
        if need_rolling_5h {
            if let Some(start_5h) = start_5h {
                buckets_start = Some(match buckets_start {
                    Some(existing) => existing.min(start_5h),
                    None => start_5h,
                });
            }
        }

        if let Some(buckets_start) = buckets_start {
            if let Ok(buckets) = fetch_cost_buckets(&conn, provider.id, buckets_start, end_unix) {
                if need_rolling_5h {
                    if let (Some(start_5h), Some(limit_usd)) = (start_5h, provider.limit_5h_usd) {
                        if let Some(limit_femto) = limit_usd_to_femto(limit_usd) {
                            if let Some(next) = compute_next_available_rolling_from_buckets(
                                &buckets,
                                start_5h,
                                WINDOW_5H_SECS,
                                limit_femto,
                            ) {
                                update_latest(&mut provider_next_available, next);
                            }
                        }
                    }
                }

                if need_rolling_daily {
                    if let (Some(start_24h), Some(limit_usd)) =
                        (start_daily_rolling, provider.limit_daily_usd)
                    {
                        if let Some(limit_femto) = limit_usd_to_femto(limit_usd) {
                            if let Some(next) = compute_next_available_rolling_from_buckets(
                                &buckets,
                                start_24h,
                                WINDOW_24H_SECS,
                                limit_femto,
                            ) {
                                update_latest(&mut provider_next_available, next);
                            }
                        }
                    }
                }
            }
        }
    }

    *skipped_limits = skipped_limits.saturating_add(1);
    if let Some(next) = provider_next_available {
        update_earliest(earliest_available_unix, next);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;

    fn insert_ledger_cost(
        conn: &Connection,
        request_log_id: i64,
        provider_id: i64,
        created_at: i64,
        cost_usd_femto: i64,
    ) {
        conn.execute(
            r#"
INSERT INTO usage_ledger (
  request_log_id, trace_id, cli_key, created_at, created_at_ms, status,
  error_present, excluded_from_stats, duration_ms, final_provider_id,
  cost_usd_femto
) VALUES (
  ?1, ?2, 'codex', ?3, ?4, 200, 0, 0, 10, ?5, ?6
)
"#,
            params![
                request_log_id,
                format!("trace-ledger-{request_log_id}"),
                created_at,
                created_at.saturating_mul(1000),
                provider_id,
                cost_usd_femto
            ],
        )
        .expect("insert usage ledger cost");
    }

    fn insert_request_log_cost(
        conn: &Connection,
        request_log_id: i64,
        provider_id: Option<i64>,
        created_at: i64,
        cost_usd_femto: i64,
        attempts_json: &str,
    ) {
        conn.execute(
            r#"
INSERT INTO request_logs (
  id, trace_id, cli_key, method, path, excluded_from_stats, status,
  error_code, duration_ms, attempts_json, cost_usd_femto, created_at,
  created_at_ms, final_provider_id
) VALUES (
  ?1, ?2, 'codex', 'POST', '/v1/responses', 0, 200,
  NULL, 10, ?3, ?4, ?5, ?6, ?7
)
"#,
            params![
                request_log_id,
                format!("trace-request-log-{request_log_id}"),
                attempts_json,
                cost_usd_femto,
                created_at,
                created_at.saturating_mul(1000),
                provider_id,
            ],
        )
        .expect("insert request log cost");
    }

    fn mark_backfill_incomplete(conn: &Connection) {
        conn.execute(
            r#"
UPDATE usage_ledger_backfill_state
SET
  status = 'incomplete',
  target_request_log_id = (SELECT COALESCE(MAX(id), 0) FROM request_logs),
  last_request_log_id = 0,
  completed_at = NULL
WHERE id = 1
"#,
            [],
        )
        .expect("mark usage ledger backfill incomplete");
    }

    fn test_spend_bounds(end_ts: i64) -> SpendQueryBounds {
        SpendQueryBounds {
            start_5h: Some(100),
            start_daily_rolling: Some(100),
            start_daily_fixed: None,
            start_weekly: Some(100),
            start_monthly: Some(100),
            end_ts,
            min_start: Some(100),
        }
    }

    #[test]
    fn provider_cost_queries_read_usage_ledger_without_request_logs() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = crate::db::init_for_tests(&dir.path().join("provider-limit-ledger.db"))
            .expect("init db");
        let conn = db.open_connection().expect("open db");
        let provider_id = 42;
        insert_ledger_cost(&conn, 1, provider_id, 100, 60);
        insert_ledger_cost(&conn, 2, provider_id, 101, 50);
        assert_eq!(
            provider_cost_source(&conn).expect("read complete source"),
            ProviderCostSource::UsageLedger
        );

        let sums = sum_cost_usd_femto_windows(
            &conn,
            provider_id,
            SpendQueryBounds {
                start_5h: Some(100),
                start_daily_rolling: Some(100),
                start_daily_fixed: None,
                start_weekly: Some(100),
                start_monthly: Some(100),
                end_ts: 102,
                min_start: Some(100),
            },
        )
        .expect("sum ledger costs");
        assert_eq!(sums.spent_5h, 110);
        assert_eq!(sums.spent_daily_rolling, 110);
        assert_eq!(sums.spent_total, 110);

        let buckets =
            fetch_cost_buckets(&conn, provider_id, 100, 102).expect("fetch ledger buckets");
        assert_eq!(buckets, vec![(100, 60), (101, 50)]);
    }

    #[test]
    fn incomplete_backfill_uses_indexed_request_logs_and_null_attempt_fallback() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = crate::db::init_for_tests(&dir.path().join("provider-limit-incomplete.db"))
            .expect("init db");
        let conn = db.open_connection().expect("open db");
        let provider_id = 42;
        insert_request_log_cost(&conn, 1, Some(provider_id), 100, 60, "[]");
        insert_request_log_cost(
            &conn,
            2,
            None,
            101,
            50,
            r#"[{"provider_id":42,"provider_name":"legacy","outcome":"success"}]"#,
        );
        insert_request_log_cost(&conn, 3, Some(99), 100, 9_999, "[]");
        mark_backfill_incomplete(&conn);

        assert_eq!(
            provider_cost_source(&conn).expect("read incomplete source"),
            ProviderCostSource::RequestLogs
        );
        let sums = sum_cost_usd_femto_windows(&conn, provider_id, test_spend_bounds(102))
            .expect("sum incomplete request-log costs");
        assert_eq!(sums.spent_total, 110);
        assert_eq!(sums.spent_5h, 110);
        assert_eq!(
            fetch_cost_buckets(&conn, provider_id, 100, 102)
                .expect("read incomplete request-log buckets"),
            vec![(100, 60), (101, 50)]
        );

        let mut stmt = conn
            .prepare(&format!(
                "EXPLAIN QUERY PLAN {REQUEST_LOG_COST_BUCKET_ROWS_SQL}"
            ))
            .expect("prepare incomplete request-log query plan");
        let details = stmt
            .query_map(params![provider_id, 100_i64, 102_i64], |row| {
                row.get::<_, String>(3)
            })
            .expect("query incomplete request-log plan")
            .collect::<Result<Vec<_>, _>>()
            .expect("read incomplete request-log plan")
            .join("\n");
        assert!(
            details.contains("idx_request_logs_provider_success_cost"),
            "incomplete provider cost query must use the request-log provider cost index: {details}"
        );
    }

    #[test]
    fn provider_cost_window_and_bucket_predicates_use_the_partial_ledger_index() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db =
            crate::db::init_for_tests(&dir.path().join("provider-limit-plan.db")).expect("init db");
        let conn = db.open_connection().expect("open db");
        conn.execute(
            "UPDATE usage_ledger_backfill_state SET status = 'complete' WHERE id = 1",
            [],
        )
        .expect("complete backfill");

        let mut stmt = conn
            .prepare(
                r#"
EXPLAIN QUERY PLAN
SELECT created_at, cost_usd_femto
FROM usage_ledger
WHERE excluded_from_stats = 0
  AND status >= 200 AND status < 300
  AND error_present = 0
  AND cost_usd_femto IS NOT NULL
  AND final_provider_id = ?1
  AND created_at >= ?2 AND created_at < ?3
"#,
            )
            .expect("prepare provider cost query plan");
        let details = stmt
            .query_map(params![42i64, 100i64, 200i64], |row| {
                row.get::<_, String>(3)
            })
            .expect("query provider cost plan")
            .collect::<Result<Vec<_>, _>>()
            .expect("read provider cost plan")
            .join("\n");

        assert!(
            details.contains("idx_usage_ledger_provider_success_cost"),
            "provider cost windows and buckets should use the partial success-cost index: {details}"
        );
    }

    #[test]
    fn provider_cost_totals_do_not_overflow_and_do_not_fail_open() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = crate::db::init_for_tests(&dir.path().join("provider-limit-overflow.db"))
            .expect("init db");
        let conn = db.open_connection().expect("open db");
        let provider_id = 42;
        insert_ledger_cost(&conn, 1, provider_id, 100, i64::MAX);
        insert_ledger_cost(&conn, 2, provider_id, 101, i64::MAX);
        insert_ledger_cost(&conn, 3, provider_id, 102, i64::MAX);
        conn.execute(
            "UPDATE usage_ledger SET error_present = 1 WHERE request_log_id = 3",
            [],
        )
        .expect("mark failed event");

        let sums = sum_cost_usd_femto_windows(
            &conn,
            provider_id,
            SpendQueryBounds {
                start_5h: Some(100),
                start_daily_rolling: Some(100),
                start_daily_fixed: None,
                start_weekly: Some(100),
                start_monthly: Some(100),
                end_ts: 103,
                min_start: Some(100),
            },
        )
        .expect("exact cost accumulator must not overflow");

        assert!(sums.spent_total > i64::MAX as i128);
        assert!(sums.spent_total < i64::MAX as i128 * 3);
        assert!(
            limit_exceeded(10_000.0, sums.spent_total),
            "large ledger spend must block the provider instead of failing open"
        );
        let buckets = fetch_cost_buckets(&conn, provider_id, 100, 103).expect("fetch cost buckets");
        assert_eq!(
            buckets.len(),
            2,
            "error_present = 1 must not enter cost buckets"
        );
    }

    #[test]
    fn provider_cost_gating_preserves_single_femto_limit_boundaries() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = crate::db::init_for_tests(&dir.path().join("provider-limit-precision.db"))
            .expect("init db");
        let conn = db.open_connection().expect("open db");
        let provider_id = 42;
        let one_femto_below_ten_usd = 9_999_999_999_999_999_i64;
        insert_ledger_cost(&conn, 1, provider_id, 100, one_femto_below_ten_usd);

        let bounds = SpendQueryBounds {
            start_5h: Some(100),
            start_daily_rolling: Some(100),
            start_daily_fixed: None,
            start_weekly: Some(100),
            start_monthly: Some(100),
            end_ts: 102,
            min_start: Some(100),
        };
        let sums = sum_cost_usd_femto_windows(&conn, provider_id, bounds).expect("sum exact cost");
        assert_eq!(
            sums.spent_total,
            i128::from(one_femto_below_ten_usd),
            "the accumulator must not round a single femto"
        );
        assert!(
            !limit_exceeded(10.0, sums.spent_total),
            "one femto below the limit must remain eligible"
        );

        insert_ledger_cost(&conn, 2, provider_id, 101, 1);
        let sums =
            sum_cost_usd_femto_windows(&conn, provider_id, bounds).expect("sum exact boundary");
        assert_eq!(sums.spent_total, 10_000_000_000_000_000_i128);
        assert!(
            limit_exceeded(10.0, sums.spent_total),
            "the exact limit must block the provider"
        );
        assert_eq!(
            fetch_cost_buckets(&conn, provider_id, 100, 102).expect("read exact buckets"),
            vec![(100, i128::from(one_femto_below_ten_usd)), (101, 1),]
        );
    }

    #[test]
    fn missing_usage_events_is_an_error_instead_of_zero_usage() {
        let conn = Connection::open_in_memory().expect("open sqlite");
        let result = sum_cost_usd_femto_windows(
            &conn,
            42,
            SpendQueryBounds {
                start_5h: Some(100),
                start_daily_rolling: None,
                start_daily_fixed: None,
                start_weekly: None,
                start_monthly: None,
                end_ts: 102,
                min_start: Some(100),
            },
        );

        assert!(result.is_err());
    }

    #[test]
    fn rolling_next_available_returns_cutoff_plus_window_plus_1() {
        let window_secs = 5;
        let window_start = 100;
        let limit_femto: i128 = 100;

        let buckets = vec![(100, 60), (101, 50)];

        let next = compute_next_available_rolling_from_buckets(
            &buckets,
            window_start,
            window_secs,
            limit_femto,
        )
        .expect("next available");
        assert_eq!(next, 100 + 1 + window_secs);
    }

    #[test]
    fn rolling_next_available_handles_equal_to_limit_as_exceeded() {
        let window_secs = 10;
        let window_start = 1_000;
        let limit_femto: i128 = 100;

        let buckets = vec![(1_000, 100)];
        let next = compute_next_available_rolling_from_buckets(
            &buckets,
            window_start,
            window_secs,
            limit_femto,
        )
        .expect("next available");
        assert_eq!(next, 1_000 + 1 + window_secs);
    }

    #[test]
    fn rolling_next_available_returns_none_when_under_limit() {
        let window_secs = 10;
        let window_start = 100;
        let limit_femto: i128 = 200;

        let buckets = vec![(100, 50), (101, 49)];
        let next = compute_next_available_rolling_from_buckets(
            &buckets,
            window_start,
            window_secs,
            limit_femto,
        );
        assert!(next.is_none());
    }

    #[test]
    fn rolling_next_available_ignores_buckets_before_window_start() {
        let window_secs = 10;
        let window_start = 200;
        let limit_femto: i128 = 100;

        // Buckets before window_start should be ignored
        let buckets = vec![(100, 1000), (150, 1000), (200, 50), (201, 50)];
        let next = compute_next_available_rolling_from_buckets(
            &buckets,
            window_start,
            window_secs,
            limit_femto,
        )
        .expect("next available");
        // First bucket at 200 pushes over limit
        assert_eq!(next, 200 + 1 + window_secs);
    }

    #[test]
    fn rolling_next_available_handles_zero_or_negative_limit() {
        let buckets = vec![(100, 50)];
        assert!(compute_next_available_rolling_from_buckets(&buckets, 100, 10, 0).is_none());
        assert!(compute_next_available_rolling_from_buckets(&buckets, 100, 10, -1).is_none());
    }

    #[test]
    fn rolling_next_available_handles_zero_or_negative_window() {
        let buckets = vec![(100, 50)];
        assert!(compute_next_available_rolling_from_buckets(&buckets, 100, 0, 100).is_none());
        assert!(compute_next_available_rolling_from_buckets(&buckets, 100, -1, 100).is_none());
    }

    #[test]
    fn limit_usd_to_femto_conversion() {
        assert_eq!(limit_usd_to_femto(1.0), Some(1_000_000_000_000_000));
        assert_eq!(limit_usd_to_femto(0.001), Some(1_000_000_000_000));
        assert_eq!(limit_usd_to_femto(0.0), Some(0));
    }

    #[test]
    fn limit_usd_to_femto_tiny_positive_never_rounds_to_zero() {
        assert_eq!(limit_usd_to_femto(1e-18), Some(1));
    }

    #[test]
    fn limit_usd_to_femto_handles_invalid_inputs() {
        assert!(limit_usd_to_femto(f64::NAN).is_none());
        assert!(limit_usd_to_femto(f64::INFINITY).is_none());
        assert!(limit_usd_to_femto(f64::NEG_INFINITY).is_none());
        assert!(limit_usd_to_femto(-1.0).is_none());
    }

    #[test]
    fn limit_exceeded_checks_correctly() {
        // 1 USD limit = 1_000_000_000_000_000 femto
        let limit_usd = 1.0;
        let limit_femto = 1_000_000_000_000_000_i128;

        // Exactly at limit - should be exceeded
        assert!(limit_exceeded(limit_usd, limit_femto));

        // Under limit
        assert!(!limit_exceeded(limit_usd, limit_femto - 1));

        // Over limit
        assert!(limit_exceeded(limit_usd, limit_femto + 1));

        // Negative spent should not exceed
        assert!(!limit_exceeded(limit_usd, -100));

        // Zero limit is explicitly treated as immediate limit hit
        assert!(limit_exceeded(0.0, 0));
    }

    #[test]
    fn limit_exceeded_handles_invalid_limit() {
        // Invalid limits should never be "exceeded" (fail open)
        assert!(!limit_exceeded(f64::NAN, 1_000_000));
        assert!(!limit_exceeded(-1.0, 1_000_000));
    }

    #[test]
    fn update_earliest_selects_minimum() {
        let mut earliest: Option<i64> = None;

        update_earliest(&mut earliest, 100);
        assert_eq!(earliest, Some(100));

        update_earliest(&mut earliest, 200);
        assert_eq!(earliest, Some(100)); // Should keep 100

        update_earliest(&mut earliest, 50);
        assert_eq!(earliest, Some(50)); // Should update to 50
    }

    #[test]
    fn update_earliest_ignores_non_positive() {
        let mut earliest: Option<i64> = Some(100);
        update_earliest(&mut earliest, 0);
        assert_eq!(earliest, Some(100));

        update_earliest(&mut earliest, -50);
        assert_eq!(earliest, Some(100));
    }

    #[test]
    fn update_latest_selects_maximum() {
        let mut latest: Option<i64> = None;

        update_latest(&mut latest, 100);
        assert_eq!(latest, Some(100));

        update_latest(&mut latest, 50);
        assert_eq!(latest, Some(100)); // Should keep 100

        update_latest(&mut latest, 200);
        assert_eq!(latest, Some(200)); // Should update to 200
    }

    #[test]
    fn update_latest_ignores_non_positive() {
        let mut latest: Option<i64> = Some(100);
        update_latest(&mut latest, 0);
        assert_eq!(latest, Some(100));

        update_latest(&mut latest, -50);
        assert_eq!(latest, Some(100));
    }

    #[test]
    fn parse_reset_time_hms_lossy_valid_inputs() {
        assert_eq!(parse_reset_time_hms_lossy("00:00:00"), (0, 0, 0));
        assert_eq!(parse_reset_time_hms_lossy("12:30:45"), (12, 30, 45));
        assert_eq!(parse_reset_time_hms_lossy("23:59:59"), (23, 59, 59));
        assert_eq!(parse_reset_time_hms_lossy("  09:15:30  "), (9, 15, 30));
    }

    #[test]
    fn parse_reset_time_hms_lossy_partial_inputs() {
        assert_eq!(parse_reset_time_hms_lossy("12"), (12, 0, 0));
        assert_eq!(parse_reset_time_hms_lossy("12:30"), (12, 30, 0));
    }

    #[test]
    fn parse_reset_time_hms_lossy_invalid_inputs() {
        // Invalid hour (> 23) should default to 0
        assert_eq!(parse_reset_time_hms_lossy("25:30:00"), (0, 30, 0));
        // Invalid minute (> 59) should default to 0
        assert_eq!(parse_reset_time_hms_lossy("12:60:00"), (12, 0, 0));
        // Invalid second (> 59) should default to 0
        assert_eq!(parse_reset_time_hms_lossy("12:30:60"), (12, 30, 0));
        // Non-numeric should default to 0
        assert_eq!(parse_reset_time_hms_lossy("abc:def:ghi"), (0, 0, 0));
        // Empty string
        assert_eq!(parse_reset_time_hms_lossy(""), (0, 0, 0));
    }

    #[test]
    fn min_start_ts_returns_minimum() {
        assert_eq!(min_start_ts(&[Some(100), Some(50), Some(200)]), Some(50));
        assert_eq!(min_start_ts(&[None, Some(100), None]), Some(100));
        assert_eq!(min_start_ts(&[None, None, None]), None);
        assert_eq!(min_start_ts(&[]), None);
    }
}
