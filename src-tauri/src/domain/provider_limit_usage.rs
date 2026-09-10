//! Usage: Provider limit usage queries - calculates current spending against configured limits.

use crate::db;
use crate::providers::DailyResetMode;
use crate::shared::error::db_err;
use chrono::{Datelike, Months, NaiveDateTime};
use rusqlite::{params, params_from_iter, Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const USD_FEMTO_DENOM: f64 = 1_000_000_000_000_000.0;
const WINDOW_5H_SECS: i64 = 5 * 60 * 60;
// Each provider contributes 13 bindings; stay below SQLite's 999-variable floor.
const MAX_PROVIDERS_PER_USAGE_QUERY: usize = 75;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum ProviderLimitPeriod {
    #[serde(rename = "5h")]
    FiveHour,
    #[serde(rename = "daily")]
    Daily,
    #[serde(rename = "weekly")]
    Weekly,
    #[serde(rename = "monthly")]
    Monthly,
}

impl ProviderLimitPeriod {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::FiveHour => "5h",
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
        }
    }

    pub(crate) fn index(self) -> usize {
        match self {
            Self::FiveHour => 0,
            Self::Daily => 1,
            Self::Weekly => 2,
            Self::Monthly => 3,
        }
    }
}

pub(crate) const LIMIT_PERIODS: [ProviderLimitPeriod; 4] = [
    ProviderLimitPeriod::FiveHour,
    ProviderLimitPeriod::Daily,
    ProviderLimitPeriod::Weekly,
    ProviderLimitPeriod::Monthly,
];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct LimitReset {
    pub(crate) reset_at: Option<i64>,
    pub(crate) cutoff: i64,
}

pub(crate) fn read_resets(
    conn: &Connection,
    provider_id: Option<i64>,
) -> crate::shared::error::AppResult<HashMap<i64, [LimitReset; 4]>> {
    let sql = if provider_id.is_some() {
        "SELECT provider_id, period, reset_at, request_log_id_cutoff FROM provider_limit_resets WHERE provider_id = ?1"
    } else {
        "SELECT provider_id, period, reset_at, request_log_id_cutoff FROM provider_limit_resets"
    };
    let mut stmt = conn.prepare_cached(sql).map_err(|e| db_err!("failed to prepare provider limit resets: {e}"))?;
    let rows = stmt.query_map(params_from_iter(provider_id), |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, LimitReset {
            reset_at: row.get(2)?, cutoff: row.get(3)?,
        }))
    }).map_err(|e| db_err!("failed to read provider limit resets: {e}"))?;
    let mut resets = HashMap::new();
    for row in rows {
        let (id, period, reset) = row.map_err(|e| db_err!("failed to decode provider limit reset: {e}"))?;
        let index = LIMIT_PERIODS.iter().position(|p| p.as_str() == period)
            .ok_or_else(|| db_err!("invalid provider limit period: {period}"))?;
        resets.entry(id).or_insert([LimitReset::default(); 4])[index] = reset;
    }
    Ok(resets)
}

fn write_reset(
    tx: &rusqlite::Transaction<'_>,
    provider_id: i64,
    period: ProviderLimitPeriod,
    now: i64,
) -> crate::shared::error::AppResult<()> {
    let exists: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM providers WHERE id = ?1)", [provider_id], |row| row.get(0))
        .map_err(|e| db_err!("failed to find provider for limit reset: {e}"))?;
    if !exists {
        return Err("DB_NOT_FOUND: provider not found".to_string().into());
    }
    tx.execute(
        r#"INSERT INTO provider_limit_resets(provider_id, period, reset_at, request_log_id_cutoff)
VALUES (?1, ?2, ?3, COALESCE((SELECT seq FROM sqlite_sequence WHERE name = 'request_logs'), 0))
ON CONFLICT(provider_id, period) DO UPDATE SET reset_at = excluded.reset_at, request_log_id_cutoff = excluded.request_log_id_cutoff"#,
        params![provider_id, period.as_str(), now],
    ).map_err(|e| db_err!("failed to reset provider limit: {e}"))?;
    if period == ProviderLimitPeriod::FiveHour {
        tx.execute("UPDATE providers SET window_5h_start_ts = ?1 WHERE id = ?2", params![now, provider_id])
            .map_err(|e| db_err!("failed to restart provider 5h window: {e}"))?;
    }
    Ok(())
}

pub fn reset(db: &db::Db, provider_id: i64, period: ProviderLimitPeriod) -> crate::shared::error::AppResult<()> {
    let mut conn = db.open_connection()?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| db_err!("failed to start provider limit reset transaction: {e}"))?;
    write_reset(&tx, provider_id, period, current_unix_seconds(&tx)?)?;
    tx.commit().map_err(|e| db_err!("failed to commit provider limit reset: {e}"))?;
    Ok(())
}

#[cfg(test)]
pub(crate) fn reset_at(conn: &mut Connection, provider_id: i64, period: ProviderLimitPeriod, now: i64) -> crate::shared::error::AppResult<()> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| db_err!("failed to start provider limit reset transaction: {e}"))?;
    write_reset(&tx, provider_id, period, now)?;
    tx.commit().map_err(|e| db_err!("failed to commit provider limit reset: {e}"))?;
    Ok(())
}

fn local_datetime(conn: &Connection, unix: i64) -> crate::shared::error::AppResult<NaiveDateTime> {
    let value: String = conn.query_row("SELECT datetime(?1, 'unixepoch', 'localtime')", [unix], |row| row.get(0))
        .map_err(|e| db_err!("failed to read local limit timestamp: {e}"))?;
    NaiveDateTime::parse_from_str(&value, "%Y-%m-%d %H:%M:%S")
        .map_err(|e| db_err!("invalid local limit timestamp: {e}"))
}

fn monthly_boundary(conn: &Connection, anchor: NaiveDateTime, months: u32) -> crate::shared::error::AppResult<i64> {
    // Always add to the original anchor: chrono clamps month-end without drifting.
    let target = anchor.checked_add_months(Months::new(months))
        .ok_or_else(|| db_err!("monthly limit timestamp out of range"))?;
    let wall = target.and_utc().timestamp();
    let mut candidates = Vec::new();
    // Sample offsets on both sides of a transition. Exact matches select the
    // earlier repeated instant; a gap selects the smallest forward wall shift.
    for delta in [-172_800, 0, 172_800] {
        let sample = wall + delta;
        let offset = local_datetime(conn, sample)?.and_utc().timestamp() - sample;
        let unix = wall - offset;
        let shift = local_datetime(conn, unix)?.and_utc().timestamp() - wall;
        if shift >= 0 {
            candidates.push((shift, unix));
        }
    }
    candidates.into_iter().min().map(|(_, unix)| unix)
        .ok_or_else(|| db_err!("failed to resolve monthly limit boundary"))
}

pub(crate) fn manual_window(
    conn: &Connection,
    period: ProviderLimitPeriod,
    anchor: i64,
    now: i64,
) -> crate::shared::error::AppResult<(i64, i64)> {
    let seconds = match period {
        ProviderLimitPeriod::FiveHour => Some(WINDOW_5H_SECS),
        ProviderLimitPeriod::Daily => Some(86_400),
        ProviderLimitPeriod::Weekly => Some(604_800),
        ProviderLimitPeriod::Monthly => None,
    };
    if let Some(seconds) = seconds {
        let start = anchor + (now - anchor).max(0) / seconds * seconds;
        return Ok((start, start + seconds));
    }
    let local_anchor = local_datetime(conn, anchor)?;
    let local_now = local_datetime(conn, now)?;
    let mut months = ((local_now.year() - local_anchor.year()) * 12
        + local_now.month() as i32 - local_anchor.month() as i32).max(0) as u32;
    let mut start = if months == 0 { anchor } else { monthly_boundary(conn, local_anchor, months)? };
    if start > now && months > 0 {
        months -= 1;
        start = if months == 0 { anchor } else { monthly_boundary(conn, local_anchor, months)? };
    }
    let end = monthly_boundary(conn, local_anchor, months + 1)?;
    Ok((start, end))
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ProviderLimitUsageRow {
    pub cli_key: String,
    pub provider_id: i64,
    pub provider_name: String,
    pub enabled: bool,
    // Limits (null if not configured)
    pub limit_5h_usd: Option<f64>,
    pub limit_daily_usd: Option<f64>,
    pub daily_reset_mode: Option<String>,
    pub daily_reset_time: Option<String>,
    pub limit_weekly_usd: Option<f64>,
    pub limit_monthly_usd: Option<f64>,
    pub limit_total_usd: Option<f64>,
    // Current usage for each window
    pub usage_5h_usd: f64,
    pub usage_daily_usd: f64,
    pub usage_weekly_usd: f64,
    pub usage_monthly_usd: f64,
    pub usage_total_usd: f64,
    // Window start timestamps (unix seconds) for UI display
    pub window_5h_start_ts: i64,
    pub window_daily_start_ts: i64,
    pub window_weekly_start_ts: i64,
    pub window_monthly_start_ts: i64,
    pub window_5h_end_ts: i64,
    pub window_daily_end_ts: i64,
    pub window_weekly_end_ts: i64,
    pub window_monthly_end_ts: i64,
    pub daily_manual_anchor: bool,
}

fn usage_reaches_limit(limit: Option<f64>, usage: f64) -> bool {
    limit.is_some_and(|limit| {
        limit.is_finite() && limit >= 0.0 && usage.is_finite() && usage.max(0.0) >= limit
    })
}

impl ProviderLimitUsageRow {
    pub(crate) fn is_limit_reached(&self) -> bool {
        usage_reaches_limit(self.limit_5h_usd, self.usage_5h_usd)
            || usage_reaches_limit(self.limit_daily_usd, self.usage_daily_usd)
            || usage_reaches_limit(self.limit_weekly_usd, self.usage_weekly_usd)
            || usage_reaches_limit(self.limit_monthly_usd, self.usage_monthly_usd)
            || usage_reaches_limit(self.limit_total_usd, self.usage_total_usd)
    }
}

fn validate_cli_key(cli_key: &str) -> crate::shared::error::AppResult<()> {
    crate::shared::cli_key::validate_cli_key(cli_key)
}

fn normalize_cli_filter(cli_key: Option<&str>) -> crate::shared::error::AppResult<Option<&str>> {
    if let Some(k) = cli_key {
        validate_cli_key(k)?;
        return Ok(Some(k));
    }
    Ok(None)
}

fn cost_usd_from_femto(v: f64) -> f64 {
    v.max(0.0) / USD_FEMTO_DENOM
}

fn current_unix_seconds(conn: &Connection) -> crate::shared::error::AppResult<i64> {
    conn.query_row("SELECT CAST(strftime('%s', 'now') AS INTEGER)", [], |row| {
        row.get::<_, i64>(0)
    })
    .map_err(|e| db_err!("failed to get current timestamp: {e}"))
}

fn values_clause(row_count: usize, column_count: usize) -> String {
    let row = format!("({})", crate::db::sql_placeholders(column_count));
    std::iter::repeat_n(row, row_count)
        .collect::<Vec<_>>()
        .join(",")
}

/// Resolves fixed 5h window starts in batches. Valid stored windows are used
/// directly; expired/null windows fall back to the first successful request in
/// the recent 5h window, matching the previous per-provider behavior.
fn resolve_5h_starts(
    conn: &Connection,
    provider_windows: &[(i64, Option<i64>)],
    now_unix: i64,
) -> crate::shared::error::AppResult<HashMap<i64, i64>> {
    let recent_threshold = now_unix.saturating_sub(WINDOW_5H_SECS);
    let mut out = HashMap::with_capacity(provider_windows.len());
    let mut expired_ids = Vec::new();

    for (provider_id, stored_window) in provider_windows.iter().copied() {
        if let Some(start_ts) = stored_window {
            if now_unix < start_ts.saturating_add(WINDOW_5H_SECS) {
                out.insert(provider_id, start_ts);
                continue;
            }
        }
        expired_ids.push(provider_id);
    }

    for chunk in expired_ids.chunks(MAX_PROVIDERS_PER_USAGE_QUERY) {
        if chunk.is_empty() {
            continue;
        }
        let values = values_clause(chunk.len(), 1);
        let sql = format!(
            r#"
WITH candidates(provider_id) AS (VALUES {values})
SELECT
  c.provider_id,
  MIN(r.created_at) AS first_request_ts
FROM candidates c
LEFT JOIN usage_events r
  ON r.final_provider_id = c.provider_id
 AND r.excluded_from_stats = 0
 AND r.status >= 200 AND r.status < 300
 AND r.error_present = 0
 AND r.created_at >= ?
GROUP BY c.provider_id
"#
        );

        let mut params_vec: Vec<i64> = chunk.to_vec();
        params_vec.push(recent_threshold);
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| db_err!("failed to prepare 5h window query: {e}"))?;
        let rows = stmt
            .query_map(params_from_iter(params_vec.iter()), |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, Option<i64>>(1)?))
            })
            .map_err(|e| db_err!("failed to query 5h windows: {e}"))?;

        for row in rows {
            let (provider_id, first_request_ts) =
                row.map_err(|e| db_err!("failed to read 5h window row: {e}"))?;
            out.insert(provider_id, first_request_ts.unwrap_or(now_unix));
        }
    }

    Ok(out)
}

/// Computes the start timestamp for the daily window based on reset mode
fn compute_ts_daily(
    conn: &Connection,
    daily_reset_mode: DailyResetMode,
    daily_reset_time: &str,
    now: i64,
) -> crate::shared::error::AppResult<i64> {
    match daily_reset_mode {
        DailyResetMode::Rolling => {
            // Rolling: now - 24 hours
            conn.query_row(
                "SELECT ?1 - 86400",
                [now],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|e| db_err!("failed to compute rolling daily timestamp: {e}"))
        }
        DailyResetMode::Fixed => {
            // Fixed: start of day based on daily_reset_time in local timezone
            // daily_reset_time is in format "HH:MM:SS"
            conn.query_row(
                r#"
                SELECT CASE
                    WHEN strftime('%H:%M:%S', ?2, 'unixepoch', 'localtime') >= ?1
                    THEN CAST(strftime('%s', date(?2, 'unixepoch', 'localtime') || ' ' || ?1, 'utc') AS INTEGER)
                    ELSE CAST(strftime('%s', date(?2, 'unixepoch', 'localtime', '-1 day') || ' ' || ?1, 'utc') AS INTEGER)
                END
                "#,
                params![daily_reset_time, now],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|e| db_err!("failed to compute fixed daily timestamp: {e}"))
        }
    }
}

/// Computes the start timestamp for the weekly window (Monday 00:00:00 local time)
fn compute_ts_weekly(conn: &Connection, now: i64) -> crate::shared::error::AppResult<i64> {
    // Get Monday of current week at 00:00:00 local time, converted to UTC
    conn.query_row(
        r#"
        SELECT CAST(strftime('%s',
            date(?1, 'unixepoch', 'localtime', 'weekday 0', '-6 days') || ' 00:00:00',
            'utc'
        ) AS INTEGER)
        "#,
        [now],
        |row| row.get::<_, i64>(0),
    )
    .map_err(|e| db_err!("failed to compute weekly timestamp: {e}"))
}

/// Computes the start timestamp for the monthly window (1st of month 00:00:00 local time)
fn compute_ts_monthly(conn: &Connection, now: i64) -> crate::shared::error::AppResult<i64> {
    conn.query_row(
        "SELECT CAST(strftime('%s', date(?1, 'unixepoch', 'localtime', 'start of month') || ' 00:00:00', 'utc') AS INTEGER)",
        [now],
        |row| row.get::<_, i64>(0),
    )
    .map_err(|e| db_err!("failed to compute monthly timestamp: {e}"))
}

fn local_window_end(conn: &Connection, start: i64, modifier: &str) -> crate::shared::error::AppResult<i64> {
    conn.query_row("SELECT CAST(strftime('%s', ?1, 'unixepoch', 'localtime', ?2, 'utc') AS INTEGER)", params![start, modifier], |row| row.get(0))
        .map_err(|e| db_err!("failed to compute provider limit window end: {e}"))
}

#[derive(Debug, Clone)]
struct ProviderLimitCandidate {
    provider_id: i64,
    cli_key: String,
    name: String,
    enabled: bool,
    limit_5h_usd: Option<f64>,
    limit_daily_usd: Option<f64>,
    daily_reset_mode_raw: String,
    daily_reset_time: String,
    limit_weekly_usd: Option<f64>,
    limit_monthly_usd: Option<f64>,
    limit_total_usd: Option<f64>,
    windows: [(i64, i64); 4],
    resets: [LimitReset; 4],
}

#[derive(Debug, Clone, Copy, Default)]
struct ProviderUsageSums {
    usage_5h_femto: f64,
    usage_daily_femto: f64,
    usage_weekly_femto: f64,
    usage_monthly_femto: f64,
    usage_total_femto: f64,
}

fn aggregate_costs_sql(provider_count: usize) -> String {
    let values = values_clause(provider_count, 13);
    format!(
        r#"
WITH provider_windows(provider_id, ts_5h, end_5h, cutoff_5h, ts_daily, end_daily, cutoff_daily, ts_weekly, end_weekly, cutoff_weekly, ts_monthly, end_monthly, cutoff_monthly) AS (VALUES {values}),
eligible_costs AS MATERIALIZED (
  SELECT final_provider_id, created_at, cost_usd_femto, id AS request_log_id
  FROM usage_events
  WHERE final_provider_id IN (SELECT provider_id FROM provider_windows)
    AND excluded_from_stats = 0
    AND status >= 200 AND status < 300
    AND error_present = 0
    AND cost_usd_femto IS NOT NULL
)
SELECT
  w.provider_id,
  TOTAL(CASE WHEN r.created_at >= w.ts_5h AND r.created_at < w.end_5h AND r.request_log_id > w.cutoff_5h THEN r.cost_usd_femto ELSE 0 END) AS usage_5h_femto,
  TOTAL(CASE WHEN r.created_at >= w.ts_daily AND r.created_at < w.end_daily AND r.request_log_id > w.cutoff_daily THEN r.cost_usd_femto ELSE 0 END) AS usage_daily_femto,
  TOTAL(CASE WHEN r.created_at >= w.ts_weekly AND r.created_at < w.end_weekly AND r.request_log_id > w.cutoff_weekly THEN r.cost_usd_femto ELSE 0 END) AS usage_weekly_femto,
  TOTAL(CASE WHEN r.created_at >= w.ts_monthly AND r.created_at < w.end_monthly AND r.request_log_id > w.cutoff_monthly THEN r.cost_usd_femto ELSE 0 END) AS usage_monthly_femto,
  TOTAL(r.cost_usd_femto) AS usage_total_femto
FROM provider_windows w
LEFT JOIN eligible_costs r
  ON r.final_provider_id = w.provider_id
GROUP BY w.provider_id
"#
    )
}

fn aggregate_costs_for_providers(
    conn: &Connection,
    providers: &[ProviderLimitCandidate],
) -> crate::shared::error::AppResult<HashMap<i64, ProviderUsageSums>> {
    let mut out = HashMap::with_capacity(providers.len());
    for chunk in providers.chunks(MAX_PROVIDERS_PER_USAGE_QUERY) {
        let sql = aggregate_costs_sql(chunk.len());

        let mut params_vec = Vec::with_capacity(chunk.len() * 13);
        for provider in chunk {
            params_vec.push(provider.provider_id);
            for (window, reset) in provider.windows.iter().zip(provider.resets) {
                params_vec.extend([window.0, window.1, reset.cutoff]);
            }
        }

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| db_err!("failed to prepare provider usage query: {e}"))?;
        let rows = stmt
            .query_map(params_from_iter(params_vec.iter()), |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    ProviderUsageSums {
                        usage_5h_femto: row.get::<_, f64>(1)?.max(0.0),
                        usage_daily_femto: row.get::<_, f64>(2)?.max(0.0),
                        usage_weekly_femto: row.get::<_, f64>(3)?.max(0.0),
                        usage_monthly_femto: row.get::<_, f64>(4)?.max(0.0),
                        usage_total_femto: row.get::<_, f64>(5)?.max(0.0),
                    },
                ))
            })
            .map_err(|e| db_err!("failed to query provider usage: {e}"))?;

        for row in rows {
            let (provider_id, sums) =
                row.map_err(|e| db_err!("failed to read provider usage row: {e}"))?;
            out.insert(provider_id, sums);
        }
    }

    Ok(out)
}

pub fn list_v1(
    db: &db::Db,
    cli_key: Option<&str>,
) -> crate::shared::error::AppResult<Vec<ProviderLimitUsageRow>> {
    let cli_key = normalize_cli_filter(cli_key)?;
    let conn = db.open_connection()?;
    list_at(&conn, cli_key, current_unix_seconds(&conn)?)
}

pub(crate) fn list_at(conn: &Connection, cli_key: Option<&str>, now: i64) -> crate::shared::error::AppResult<Vec<ProviderLimitUsageRow>> {

    // Pre-compute common time windows (5h is computed per-provider below)
    let ts_weekly = compute_ts_weekly(conn, now)?;
    let ts_monthly = compute_ts_monthly(conn, now)?;
    let end_weekly = local_window_end(conn, ts_weekly, "+7 days")?;
    let end_monthly = local_window_end(conn, ts_monthly, "+1 month")?;
    let resets_by_provider = read_resets(conn, None)?;

    // Query all providers with at least one limit configured
    let sql = r#"
        SELECT
            id,
            cli_key,
            name,
            enabled,
            limit_5h_usd,
            limit_daily_usd,
            daily_reset_mode,
            daily_reset_time,
            limit_weekly_usd,
            limit_monthly_usd,
            limit_total_usd,
            window_5h_start_ts
        FROM providers
        WHERE (?1 IS NULL OR cli_key = ?1)
          AND (
            limit_5h_usd IS NOT NULL OR
            limit_daily_usd IS NOT NULL OR
            limit_weekly_usd IS NOT NULL OR
            limit_monthly_usd IS NOT NULL OR
            limit_total_usd IS NOT NULL OR
            EXISTS(SELECT 1 FROM provider_limit_resets WHERE provider_id = providers.id)
          )
        ORDER BY cli_key ASC, sort_order ASC, id DESC
    "#;

    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| db_err!("failed to prepare providers query: {e}"))?;

    let rows = stmt
        .query_map(params![cli_key], |row| {
            let daily_reset_mode_raw: String = row.get("daily_reset_mode")?;
            let daily_reset_time_raw: String = row.get("daily_reset_time")?;

            Ok((
                row.get::<_, i64>("id")?,
                row.get::<_, String>("cli_key")?,
                row.get::<_, String>("name")?,
                row.get::<_, i64>("enabled")? != 0,
                row.get::<_, Option<f64>>("limit_5h_usd")?,
                row.get::<_, Option<f64>>("limit_daily_usd")?,
                daily_reset_mode_raw,
                daily_reset_time_raw,
                row.get::<_, Option<f64>>("limit_weekly_usd")?,
                row.get::<_, Option<f64>>("limit_monthly_usd")?,
                row.get::<_, Option<f64>>("limit_total_usd")?,
                row.get::<_, Option<i64>>("window_5h_start_ts")?,
            ))
        })
        .map_err(|e| db_err!("failed to query providers: {e}"))?;

    let mut raw_rows = Vec::new();
    let mut provider_windows = Vec::new();
    let mut daily_window_cache: HashMap<(String, String), i64> = HashMap::new();

    for row in rows {
        let (
            provider_id,
            cli_key,
            name,
            enabled,
            limit_5h_usd,
            limit_daily_usd,
            daily_reset_mode_raw,
            daily_reset_time_raw,
            limit_weekly_usd,
            limit_monthly_usd,
            limit_total_usd,
            stored_5h_start_ts,
        ) = row.map_err(|e| db_err!("failed to read provider row: {e}"))?;

        // Parse daily reset mode for computation
        let daily_reset_mode = match daily_reset_mode_raw.as_str() {
            "rolling" => DailyResetMode::Rolling,
            _ => DailyResetMode::Fixed,
        };

        // Normalize daily_reset_time (defaults to "00:00:00")
        let daily_reset_time = if daily_reset_time_raw.trim().is_empty() {
            "00:00:00".to_string()
        } else {
            daily_reset_time_raw.clone()
        };

        // Compute daily timestamp based on provider's reset mode. Cache by
        // mode/time because most providers share the same reset settings.
        let daily_cache_key = (
            daily_reset_mode.as_str().to_string(),
            daily_reset_time.clone(),
        );
        let ts_daily = match daily_window_cache.get(&daily_cache_key).copied() {
            Some(ts) => ts,
            None => {
                let ts = compute_ts_daily(conn, daily_reset_mode, &daily_reset_time, now)?;
                daily_window_cache.insert(daily_cache_key, ts);
                ts
            }
        };

        raw_rows.push((
            provider_id,
            cli_key,
            name,
            enabled,
            limit_5h_usd,
            limit_daily_usd,
            daily_reset_mode_raw,
            daily_reset_time,
            limit_weekly_usd,
            limit_monthly_usd,
            limit_total_usd,
            stored_5h_start_ts,
            ts_daily,
        ));
        if resets_by_provider.get(&provider_id).and_then(|resets| resets[0].reset_at).is_none() {
            provider_windows.push((provider_id, stored_5h_start_ts));
        }
    }

    if raw_rows.is_empty() {
        return Ok(Vec::new());
    }

    let starts_5h = resolve_5h_starts(conn, &provider_windows, now)?;
    let mut candidates = Vec::with_capacity(raw_rows.len());
    for (
        provider_id,
        cli_key,
        name,
        enabled,
        limit_5h_usd,
        limit_daily_usd,
        daily_reset_mode_raw,
        daily_reset_time,
        limit_weekly_usd,
        limit_monthly_usd,
        limit_total_usd,
        _stored_5h_start_ts,
        ts_daily,
    ) in raw_rows
    {
        let resets = resets_by_provider.get(&provider_id).copied().unwrap_or_default();
        let (ts_5h, end_5h) = match resets[0].reset_at {
            Some(anchor) => manual_window(conn, ProviderLimitPeriod::FiveHour, anchor, now)?,
            None => {
                let start = starts_5h.get(&provider_id).copied().ok_or_else(|| db_err!("failed to resolve 5h window for provider_id={provider_id}"))?;
                (start, start + WINDOW_5H_SECS)
            }
        };
        let end_daily = if daily_reset_mode_raw == "rolling" { now + 1 } else { local_window_end(conn, ts_daily, "+1 day")? };
        let mut windows = [(ts_5h, end_5h), (ts_daily, end_daily), (ts_weekly, end_weekly), (ts_monthly, end_monthly)];
        for period in &LIMIT_PERIODS[1..] {
            if let Some(anchor) = resets[period.index()].reset_at {
                windows[period.index()] = manual_window(conn, *period, anchor, now)?;
            }
        }
        candidates.push(ProviderLimitCandidate {
            provider_id,
            cli_key,
            name,
            enabled,
            limit_5h_usd,
            limit_daily_usd,
            daily_reset_mode_raw,
            daily_reset_time,
            limit_weekly_usd,
            limit_monthly_usd,
            limit_total_usd,
            windows,
            resets,
        });
    }

    let usage_by_provider =
        aggregate_costs_for_providers(conn, &candidates)?;
    let out = candidates
        .into_iter()
        .map(|provider| {
            let sums = usage_by_provider
                .get(&provider.provider_id)
                .copied()
                .unwrap_or_default();
            ProviderLimitUsageRow {
                cli_key: provider.cli_key,
                provider_id: provider.provider_id,
                provider_name: provider.name,
                enabled: provider.enabled,
                limit_5h_usd: provider.limit_5h_usd,
                limit_daily_usd: provider.limit_daily_usd,
                daily_reset_mode: if provider.limit_daily_usd.is_some() {
                    Some(provider.daily_reset_mode_raw)
                } else {
                    None
                },
                daily_reset_time: if provider.limit_daily_usd.is_some() {
                    Some(provider.daily_reset_time)
                } else {
                    None
                },
                limit_weekly_usd: provider.limit_weekly_usd,
                limit_monthly_usd: provider.limit_monthly_usd,
                limit_total_usd: provider.limit_total_usd,
                usage_5h_usd: cost_usd_from_femto(sums.usage_5h_femto),
                usage_daily_usd: cost_usd_from_femto(sums.usage_daily_femto),
                usage_weekly_usd: cost_usd_from_femto(sums.usage_weekly_femto),
                usage_monthly_usd: cost_usd_from_femto(sums.usage_monthly_femto),
                usage_total_usd: cost_usd_from_femto(sums.usage_total_femto),
                window_5h_start_ts: provider.windows[0].0,
                window_daily_start_ts: provider.windows[1].0,
                window_weekly_start_ts: provider.windows[2].0,
                window_monthly_start_ts: provider.windows[3].0,
                window_5h_end_ts: provider.windows[0].1,
                window_daily_end_ts: provider.windows[1].1,
                window_weekly_end_ts: provider.windows[2].1,
                window_monthly_end_ts: provider.windows[3].1,
                daily_manual_anchor: provider.resets[1].reset_at.is_some(),
            }
        })
        .collect();

    Ok(out)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::providers::{self, DailyResetMode, ProviderBaseUrlMode, ProviderUpsertParams};
    use rusqlite::params;

    const FEMTO: i64 = 1_000_000_000_000_000;

    pub(crate) fn create_limited_provider(db: &db::Db, name: &str) -> i64 {
        providers::upsert(
            db,
            ProviderUpsertParams {
                provider_id: None,
                cli_key: "codex".to_string(),
                name: name.to_string(),
                base_urls: vec!["https://example.com".to_string()],
                base_url_mode: ProviderBaseUrlMode::Order,
                auth_mode: None,
                api_key: Some("sk-test".to_string()),
                enabled: true,
                cost_multiplier: 1.0,
                priority: None,
                claude_models: None,
                model_mapping: None,
                availability_test_model: None,
                availability_probe_enabled: false,
                availability_probe_interval_minutes: 10,
                limit_5h_usd: Some(10.0),
                limit_daily_usd: Some(10.0),
                daily_reset_mode: Some(DailyResetMode::Rolling),
                daily_reset_time: Some("00:00:00".to_string()),
                limit_weekly_usd: Some(10.0),
                limit_monthly_usd: Some(10.0),
                limit_total_usd: Some(10.0),
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
        .expect("create provider")
        .id
    }

    fn insert_log_with_exclusion(
        conn: &Connection,
        provider_id: i64,
        created_at: i64,
        cost_femto: i64,
        excluded_from_stats: i64,
    ) {
        conn.execute(
            r#"
INSERT INTO request_logs(
  trace_id, cli_key, method, path, status, error_code, duration_ms,
  attempts_json, created_at, created_at_ms, cost_usd_femto,
  excluded_from_stats, final_provider_id
) VALUES (?1, 'codex', 'POST', '/v1/chat/completions', ?2, ?3, 10,
  '[]', ?4, ?5, ?6, ?7, ?8)
"#,
            params![
                format!("trace-{provider_id}-{created_at}-{cost_femto}"),
                if excluded_from_stats == 0 {
                    200i64
                } else {
                    499i64
                },
                if excluded_from_stats == 0 {
                    None
                } else {
                    Some("GW_REQUEST_INTERRUPTED_BY_GATEWAY_STOP")
                },
                created_at,
                created_at.saturating_mul(1000),
                cost_femto,
                excluded_from_stats,
                provider_id
            ],
        )
        .expect("insert request log");
        conn.execute(
            r#"
INSERT INTO usage_ledger (
  request_log_id, trace_id, cli_key, created_at, created_at_ms, status,
  error_present, excluded_from_stats, duration_ms, final_provider_id,
  cost_usd_femto
)
SELECT
  id, trace_id, cli_key, created_at, created_at_ms, status,
  CASE WHEN error_code IS NULL THEN 0 ELSE 1 END,
  excluded_from_stats, duration_ms, final_provider_id, cost_usd_femto
FROM request_logs
WHERE id = last_insert_rowid()
"#,
            [],
        )
        .expect("insert usage ledger row");
    }

    fn insert_log(conn: &Connection, provider_id: i64, created_at: i64, cost_femto: i64) {
        insert_log_with_exclusion(conn, provider_id, created_at, cost_femto, 0);
    }

    #[test]
    fn list_v1_batches_provider_usage_without_changing_window_totals() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = db::init_for_tests(&dir.path().join("test.db")).expect("init db");
        let provider_id = create_limited_provider(&db, "limited");
        let conn = db.open_connection().expect("open db");
        let now = current_unix_seconds(&conn).expect("now");
        let start_5h = now.saturating_sub(60 * 60);

        conn.execute(
            "UPDATE providers SET window_5h_start_ts = ?1 WHERE id = ?2",
            params![start_5h, provider_id],
        )
        .expect("set 5h window");

        insert_log(&conn, provider_id, now.saturating_sub(30 * 60), FEMTO);
        insert_log(
            &conn,
            provider_id,
            now.saturating_sub(2 * 60 * 60),
            2 * FEMTO,
        );
        insert_log(&conn, provider_id, now.saturating_sub(15 * 60), -2 * FEMTO);
        conn.execute("DELETE FROM request_logs", [])
            .expect("remove request details");
        drop(conn);

        let rows = list_v1(&db, Some("codex")).expect("list usage");
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.provider_id, provider_id);
        assert_eq!(row.window_5h_start_ts, start_5h);
        assert!((row.usage_5h_usd - 0.0).abs() < f64::EPSILON);
        assert!((row.usage_daily_usd - 1.0).abs() < f64::EPSILON);
        assert!((row.usage_total_usd - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn list_v1_excludes_lifecycle_interruption_rows_from_provider_usage() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = db::init_for_tests(&dir.path().join("test.db")).expect("init db");
        let provider_id = create_limited_provider(&db, "limited");
        let conn = db.open_connection().expect("open db");
        let now = current_unix_seconds(&conn).expect("now");

        insert_log(&conn, provider_id, now.saturating_sub(60), FEMTO);
        insert_log_with_exclusion(&conn, provider_id, now.saturating_sub(30), 99 * FEMTO, 1);
        drop(conn);

        let rows = list_v1(&db, Some("codex")).expect("list usage");
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.provider_id, provider_id);
        assert!((row.usage_5h_usd - 1.0).abs() < f64::EPSILON);
        assert!((row.usage_daily_usd - 1.0).abs() < f64::EPSILON);
        assert!((row.usage_total_usd - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn list_v1_cost_totals_do_not_overflow_i64_and_require_error_present_zero() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = db::init_for_tests(&dir.path().join("overflow.db")).expect("init db");
        let provider_id = create_limited_provider(&db, "large-spend");
        let conn = db.open_connection().expect("open db");
        let now = current_unix_seconds(&conn).expect("now");

        insert_log(&conn, provider_id, now.saturating_sub(3), i64::MAX);
        insert_log(&conn, provider_id, now.saturating_sub(2), i64::MAX);
        insert_log(&conn, provider_id, now.saturating_sub(1), i64::MAX);
        conn.execute(
            "UPDATE usage_ledger SET error_present = 1 WHERE created_at = ?1",
            [now.saturating_sub(1)],
        )
        .expect("mark ledger event failed");
        conn.execute(
            "UPDATE usage_ledger_backfill_state SET status = 'complete' WHERE id = 1",
            [],
        )
        .expect("complete backfill");
        conn.execute("DELETE FROM request_logs", [])
            .expect("remove request details");
        drop(conn);

        let rows = list_v1(&db, Some("codex")).expect("list usage");
        let row = rows
            .iter()
            .find(|row| row.provider_id == provider_id)
            .expect("provider usage");
        let expected = (i64::MAX as f64 * 2.0) / USD_FEMTO_DENOM;
        assert!((row.usage_total_usd - expected).abs() < 0.000_001);
        assert!(
            row.usage_total_usd > i64::MAX as f64 / USD_FEMTO_DENOM,
            "aggregate must exceed the SQLite integer SUM ceiling"
        );
    }

    #[test]
    fn list_v1_surfaces_usage_source_failures_instead_of_zero_usage() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = db::init_for_tests(&dir.path().join("test.db")).expect("init db");
        create_limited_provider(&db, "limited");
        let conn = db.open_connection().expect("open db");
        conn.execute_batch("DROP VIEW usage_events")
            .expect("drop usage view");
        drop(conn);

        let error = list_v1(&db, Some("codex")).expect_err("missing usage source must fail");
        assert_eq!(error.code(), "DB_ERROR");
    }

    #[test]
    fn filtered_cost_projection_preserves_compatibility_and_complete_backfill_totals() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = db::init_for_tests(&dir.path().join("projection.db")).expect("init db");
        let provider = create_limited_provider(&db, "selected");
        let empty = create_limited_provider(&db, "empty");
        let unrelated = create_limited_provider(&db, "unrelated");
        let conn = db.open_connection().expect("connection");
        for (timestamp, amount) in [(99, 1), (100, 2), (200, 3), (300, 4), (400, 5)] {
            insert_log(&conn, provider, timestamp, amount * FEMTO);
        }
        insert_log(&conn, unrelated, 500, 80 * FEMTO);
        insert_log_with_exclusion(&conn, provider, 501, 90 * FEMTO, 1);
        insert_log(&conn, provider, 502, 100 * FEMTO);
        conn.execute(
            "UPDATE usage_ledger SET error_present = 1 WHERE created_at = 502",
            [],
        )
        .unwrap();
        conn.execute(
            "UPDATE request_logs SET error_code = 'ERROR' WHERE created_at = 502",
            [],
        )
        .unwrap();
        let query = aggregate_costs_sql(2);
        let parameters = [provider, 400, 1000, 0, 300, 1000, 0, 200, 1000, 0, 100, 1000, 0,
            empty, 400, 1000, 0, 300, 1000, 0, 200, 1000, 0, 100, 1000, 0];
        for complete in [false, true] {
            if complete {
                conn.execute(
                    "UPDATE usage_ledger_backfill_state SET status = 'complete' WHERE id = 1",
                    [],
                )
                .unwrap();
            } else {
                conn.execute(
                    "UPDATE usage_ledger_backfill_state SET status = 'incomplete' WHERE id = 1",
                    [],
                )
                .unwrap();
                conn.execute("DELETE FROM usage_ledger WHERE created_at = 99", [])
                    .unwrap();
            }
            let rows = conn
                .prepare(&query)
                .unwrap()
                .query_map(parameters, |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        (1..=5)
                            .map(|i| row.get::<_, f64>(i).unwrap())
                            .collect::<Vec<_>>(),
                    ))
                })
                .unwrap()
                .collect::<Result<HashMap<_, _>, _>>()
                .unwrap();
            let original = r#"
WITH provider_windows(provider_id, ts_5h, ts_daily) AS (VALUES (?, ?, ?), (?, ?, ?))
SELECT w.provider_id,
  TOTAL(CASE WHEN r.created_at >= w.ts_5h THEN r.cost_usd_femto ELSE 0 END),
  TOTAL(CASE WHEN r.created_at >= w.ts_daily THEN r.cost_usd_femto ELSE 0 END),
  TOTAL(CASE WHEN r.created_at >= ? THEN r.cost_usd_femto ELSE 0 END),
  TOTAL(CASE WHEN r.created_at >= ? THEN r.cost_usd_femto ELSE 0 END),
  TOTAL(r.cost_usd_femto)
FROM provider_windows w
LEFT JOIN usage_events r ON r.final_provider_id = w.provider_id
  AND r.excluded_from_stats = 0 AND r.status >= 200 AND r.status < 300
  AND r.error_present = 0 AND r.cost_usd_femto IS NOT NULL
GROUP BY w.provider_id
"#;
            let original_rows = conn
                .prepare(original)
                .unwrap()
                .query_map([provider, 400, 300, empty, 400, 300, 200, 100], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        (1..=5)
                            .map(|i| row.get::<_, f64>(i).unwrap())
                            .collect::<Vec<_>>(),
                    ))
                })
                .unwrap()
                .collect::<Result<HashMap<_, _>, _>>()
                .unwrap();
            assert_eq!(rows, original_rows);
            assert_eq!(rows[&empty], vec![0.0; 5]);
            assert_eq!(
                rows[&provider],
                [5.0, 9.0, 12.0, 14.0, if complete { 14.0 } else { 15.0 }]
                    .map(|value| value * FEMTO as f64)
                    .to_vec()
            );
            let plan = conn
                .prepare(&format!("EXPLAIN QUERY PLAN {query}"))
                .unwrap()
                .query_map(parameters, |row| row.get::<_, String>(3))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
                .join("\n");
            assert!(plan.contains("MATERIALIZE eligible_costs"), "{plan}");
            assert!(!plan.contains("MATERIALIZE usage_events"), "{plan}");
            let widths = conn
                .prepare(&format!("EXPLAIN {query}"))
                .unwrap()
                .query_map(parameters, |row| {
                    Ok((row.get::<_, String>(1)?, row.get::<_, i64>(3)?))
                })
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert!(widths
                .iter()
                .any(|(opcode, width)| opcode == "OpenEphemeral" && *width == 4));
        }
    }

    #[test]
    fn usage_limit_reached_is_inclusive_and_fails_open_for_invalid_values() {
        assert!(!usage_reaches_limit(Some(10.0), 9.999));
        assert!(usage_reaches_limit(Some(10.0), 10.0));
        assert!(usage_reaches_limit(Some(0.0), 0.0));
        assert!(!usage_reaches_limit(None, 100.0));
        assert!(!usage_reaches_limit(Some(f64::NAN), 100.0));
        assert!(!usage_reaches_limit(Some(-1.0), 100.0));
        assert!(!usage_reaches_limit(Some(10.0), f64::NAN));
    }

    fn local_ts(conn: &Connection, value: &str) -> i64 {
        conn.query_row("SELECT CAST(strftime('%s', ?1, 'utc') AS INTEGER)", [value], |row| row.get(0)).unwrap()
    }

    #[test]
    fn weekly_reset_recurrence_keeps_the_full_week_and_half_open_end() {
        let dir = tempfile::tempdir().unwrap();
        let db = db::init_for_tests(&dir.path().join("week.db")).unwrap();
        let provider = create_limited_provider(&db, "week");
        let mut conn = db.open_connection().unwrap();
        let anchor = local_ts(&conn, "2025-09-07 12:00:00");
        let original_start = local_ts(&conn, "2025-09-01 00:00:00");
        let original_end = local_ts(&conn, "2025-09-08 00:00:00");
        let original = list_at(&conn, None, anchor).unwrap();
        assert_eq!(original[0].window_weekly_start_ts, original_start);
        assert_eq!(original[0].window_weekly_end_ts, original_end);
        reset_at(&mut conn, provider, ProviderLimitPeriod::Weekly, anchor).unwrap();
        insert_log(&conn, provider, anchor, FEMTO);
        let rows = list_at(&conn, None, original_end).unwrap();
        assert_eq!(rows[0].window_weekly_start_ts, anchor);
        assert_eq!(rows[0].window_weekly_end_ts, anchor + 604_800);
        assert_eq!(rows[0].usage_weekly_usd, 1.0);
        insert_log(&conn, provider, anchor + 604_800, 2 * FEMTO);
        let before = list_at(&conn, None, anchor + 604_799).unwrap();
        assert_eq!(before[0].usage_weekly_usd, 1.0);
        let after = list_at(&conn, None, anchor + 604_800).unwrap();
        assert_eq!(after[0].window_weekly_start_ts, anchor + 604_800);
        assert_eq!(after[0].window_weekly_end_ts, anchor + 2 * 604_800);
        assert_eq!(after[0].usage_weekly_usd, 2.0);
        let idle = list_at(&conn, None, anchor + 9 * 604_800 + 30).unwrap();
        assert_eq!(idle[0].window_weekly_start_ts, anchor + 9 * 604_800);
        assert_eq!(idle[0].usage_weekly_usd, 0.0);
    }

    #[test]
    fn monthly_windows_preserve_original_day_month_end_and_leap_year() {
        let conn = Connection::open_in_memory().unwrap();
        for (anchor, now, start, end) in [
            ("2026-09-07 13:15:00", "2026-10-06 00:00:00", "2026-09-07 13:15:00", "2026-10-07 13:15:00"),
            ("2026-01-31 13:15:00", "2026-03-01 00:00:00", "2026-02-28 13:15:00", "2026-03-31 13:15:00"),
            ("2024-01-31 13:15:00", "2024-03-01 00:00:00", "2024-02-29 13:15:00", "2024-03-31 13:15:00"),
            ("2024-01-31 13:15:00", "2026-09-01 00:00:00", "2026-08-31 13:15:00", "2026-09-30 13:15:00"),
        ] {
            assert_eq!(manual_window(&conn, ProviderLimitPeriod::Monthly, local_ts(&conn, anchor), local_ts(&conn, now)).unwrap(),
                (local_ts(&conn, start), local_ts(&conn, end)));
        }
    }

    #[test]
    fn monthly_dst_boundaries_use_forward_gap_and_earlier_repeat() {
        // A child test process isolates SQLite's process-local timezone from all
        // other tests and exercises the production localtime conversion.
        if std::env::var_os("AIO_LIMIT_DST_CHILD").is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", &format!("{}::monthly_dst_boundaries_use_forward_gap_and_earlier_repeat", module_path!().split_once("::").unwrap().1), "--nocapture"])
                .env("TZ", "America/New_York")
                .env("AIO_LIMIT_DST_CHILD", "1")
                .output().unwrap();
            assert!(output.status.success(), "{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
            assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
            return;
        }
        let conn = Connection::open_in_memory().unwrap();
        let unix = |value| chrono::DateTime::parse_from_rfc3339(value).unwrap().timestamp();
        for (anchor, now, start, end) in [
            ("2026-02-08T02:30:00-05:00", "2026-03-08T03:00:00-04:00", "2026-02-08T02:30:00-05:00", "2026-03-08T03:30:00-04:00"),
            ("2026-02-08T02:30:00-05:00", "2026-03-08T03:30:00-04:00", "2026-03-08T03:30:00-04:00", "2026-04-08T02:30:00-04:00"),
            ("2026-10-01T01:30:00-04:00", "2026-11-01T01:45:00-05:00", "2026-11-01T01:30:00-04:00", "2026-12-01T01:30:00-05:00"),
        ] {
            assert_eq!(manual_window(&conn, ProviderLimitPeriod::Monthly, unix(anchor), unix(now)).unwrap(), (unix(start), unix(end)));
        }
    }

    #[test]
    fn reset_high_water_excludes_pending_and_deleted_details_and_survives_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cutoff.db");
        let db = db::init_for_tests(&path).unwrap();
        let provider = create_limited_provider(&db, "cutoff");
        let mut conn = db.open_connection().unwrap();
        let now = local_ts(&conn, "2026-09-07 12:00:00");
        insert_log(&conn, provider, now, FEMTO);
        conn.execute("UPDATE request_logs SET cost_usd_femto = NULL", []).unwrap();
        conn.execute("DELETE FROM usage_ledger", []).unwrap();
        reset_at(&mut conn, provider, ProviderLimitPeriod::Monthly, now).unwrap();
        let marker = read_resets(&conn, Some(provider)).unwrap()[&provider][3];
        assert_eq!(marker.cutoff, 1);
        conn.execute("UPDATE request_logs SET cost_usd_femto = ?1", [FEMTO]).unwrap();
        conn.execute("UPDATE usage_ledger_backfill_state SET status = 'incomplete' WHERE id = 1", []).unwrap();
        let before_backfill = list_at(&conn, None, now).unwrap();
        assert_eq!(before_backfill[0].usage_monthly_usd, 0.0);
        assert_eq!(before_backfill[0].usage_total_usd, 1.0);
        conn.execute("INSERT INTO usage_ledger(request_log_id, trace_id, cli_key, created_at, created_at_ms, status, error_present, excluded_from_stats, duration_ms, final_provider_id, cost_usd_femto) SELECT id, trace_id, cli_key, created_at, created_at_ms, status, 0, 0, duration_ms, final_provider_id, cost_usd_femto FROM request_logs", []).unwrap();
        conn.execute("UPDATE usage_ledger_backfill_state SET status = 'complete' WHERE id = 1", []).unwrap();
        conn.execute("DELETE FROM request_logs", []).unwrap();
        reset_at(&mut conn, provider, ProviderLimitPeriod::Weekly, now).unwrap();
        assert_eq!(read_resets(&conn, Some(provider)).unwrap()[&provider][2].cutoff, 1);
        insert_log(&conn, provider, now - 1, 2 * FEMTO);
        insert_log(&conn, provider, now, 3 * FEMTO);
        let rows = list_at(&conn, None, now).unwrap();
        assert_eq!(rows[0].usage_monthly_usd, 3.0);
        assert_eq!(rows[0].usage_weekly_usd, 3.0);
        assert_eq!(rows[0].usage_total_usd, 6.0);
        drop(conn);
        drop(db);
        let reopened = db::init_for_tests(&path).unwrap();
        let conn = reopened.open_connection().unwrap();
        assert_eq!(read_resets(&conn, Some(provider)).unwrap()[&provider][3], marker);
        assert_eq!(list_at(&conn, None, now).unwrap()[0].usage_monthly_usd, 3.0);
    }

    #[test]
    fn reset_is_atomic_for_missing_provider_and_supports_unconfigured_periods() {
        let dir = tempfile::tempdir().unwrap();
        let db = db::init_for_tests(&dir.path().join("atomic.db")).unwrap();
        let provider = create_limited_provider(&db, "unconfigured");
        let mut conn = db.open_connection().unwrap();
        conn.execute("UPDATE providers SET limit_5h_usd=NULL, limit_daily_usd=NULL, limit_weekly_usd=NULL, limit_monthly_usd=NULL, limit_total_usd=NULL", []).unwrap();
        assert!(reset_at(&mut conn, -1, ProviderLimitPeriod::FiveHour, 1000).is_err());
        assert!(read_resets(&conn, None).unwrap().is_empty());
        assert!(serde_json::from_str::<ProviderLimitPeriod>("\"total\"").is_err());
        reset_at(&mut conn, provider, ProviderLimitPeriod::Daily, 1000).unwrap();
        let rows = list_at(&conn, None, 1000).unwrap();
        assert_eq!(rows[0].window_daily_end_ts, 87_400);
        assert!(rows[0].daily_manual_anchor);
        assert_eq!(rows[0].limit_daily_usd, None);
        conn.execute("DELETE FROM providers WHERE id = ?1", [provider]).unwrap();
        assert!(read_resets(&conn, None).unwrap().is_empty());
    }
}
