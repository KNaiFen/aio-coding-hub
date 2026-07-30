use crate::db;
use crate::shared::error::db_err;
use rusqlite::{params, Connection};

use super::{
    compute_start_ts, normalize_cli_filter, parse_range, sql_effective_input_tokens_expr,
    UsageDayRow, UsageProviderRow,
};

const USD_FEMTO_DENOM: f64 = 1_000_000_000_000_000.0;
const SQL_CANONICAL_BUCKETS_MISSING: &str = "input_tokens IS NULL
        AND output_tokens IS NULL
        AND cache_read_input_tokens IS NULL
        AND cache_creation_input_tokens IS NULL";

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub(super) struct ProviderKey {
    pub(super) cli_key: String,
    pub(super) provider_id: i64,
    pub(super) provider_name: String,
}

#[derive(Clone, Debug, Default)]
pub(super) struct ProviderAgg {
    pub(super) requests_total: i64,
    pub(super) requests_success: i64,
    pub(super) requests_failed: i64,
    pub(super) total_duration_ms: i64,
    pub(super) first_request_created_at_ms: Option<i64>,
    pub(super) last_request_created_at_ms: Option<i64>,
    pub(super) success_duration_ms_sum: i64,
    pub(super) success_ttfb_ms_sum: i64,
    pub(super) success_ttfb_ms_count: i64,
    pub(super) success_generation_ms_sum: i64,
    pub(super) success_output_tokens_for_rate_sum: i64,
    pub(super) input_tokens: i64,
    pub(super) output_tokens: i64,
    pub(super) total_tokens: i64,
    pub(super) cache_read_input_tokens: i64,
    pub(super) cache_creation_input_tokens: i64,
    pub(super) cache_creation_5m_input_tokens: i64,
    pub(super) cache_creation_1h_input_tokens: i64,
    pub(super) cost_covered_success: i64,
    pub(super) total_cost_usd_femto: f64,
}

impl ProviderAgg {
    pub(super) fn merge(&mut self, add: ProviderAgg) {
        self.requests_total = self.requests_total.saturating_add(add.requests_total);
        self.requests_success = self.requests_success.saturating_add(add.requests_success);
        self.requests_failed = self.requests_failed.saturating_add(add.requests_failed);
        self.total_duration_ms = self.total_duration_ms.saturating_add(add.total_duration_ms);
        self.first_request_created_at_ms = match (
            self.first_request_created_at_ms,
            add.first_request_created_at_ms,
        ) {
            (Some(current), Some(next)) => Some(current.min(next)),
            (None, Some(next)) => Some(next),
            (current, None) => current,
        };
        self.last_request_created_at_ms = match (
            self.last_request_created_at_ms,
            add.last_request_created_at_ms,
        ) {
            (Some(current), Some(next)) => Some(current.max(next)),
            (None, Some(next)) => Some(next),
            (current, None) => current,
        };
        self.success_duration_ms_sum = self
            .success_duration_ms_sum
            .saturating_add(add.success_duration_ms_sum);
        self.success_ttfb_ms_sum = self
            .success_ttfb_ms_sum
            .saturating_add(add.success_ttfb_ms_sum);
        self.success_ttfb_ms_count = self
            .success_ttfb_ms_count
            .saturating_add(add.success_ttfb_ms_count);
        self.success_generation_ms_sum = self
            .success_generation_ms_sum
            .saturating_add(add.success_generation_ms_sum);
        self.success_output_tokens_for_rate_sum = self
            .success_output_tokens_for_rate_sum
            .saturating_add(add.success_output_tokens_for_rate_sum);
        self.input_tokens = self.input_tokens.saturating_add(add.input_tokens);
        self.output_tokens = self.output_tokens.saturating_add(add.output_tokens);
        self.total_tokens = self.total_tokens.saturating_add(add.total_tokens);
        self.cache_read_input_tokens = self
            .cache_read_input_tokens
            .saturating_add(add.cache_read_input_tokens);
        self.cache_creation_input_tokens = self
            .cache_creation_input_tokens
            .saturating_add(add.cache_creation_input_tokens);
        self.cache_creation_5m_input_tokens = self
            .cache_creation_5m_input_tokens
            .saturating_add(add.cache_creation_5m_input_tokens);
        self.cache_creation_1h_input_tokens = self
            .cache_creation_1h_input_tokens
            .saturating_add(add.cache_creation_1h_input_tokens);
        self.cost_covered_success = self
            .cost_covered_success
            .saturating_add(add.cost_covered_success);
        self.total_cost_usd_femto += add.total_cost_usd_femto;
    }

    pub(super) fn into_leaderboard_row(
        self,
        key: String,
        name: String,
    ) -> super::UsageLeaderboardRow {
        let avg_duration_ms = if self.requests_success > 0 {
            Some(self.success_duration_ms_sum / self.requests_success)
        } else {
            None
        };
        let avg_ttfb_ms = if self.success_ttfb_ms_count > 0 {
            Some(self.success_ttfb_ms_sum / self.success_ttfb_ms_count)
        } else {
            None
        };
        let avg_output_tokens_per_second = if self.success_generation_ms_sum > 0 {
            Some(
                self.success_output_tokens_for_rate_sum as f64
                    / (self.success_generation_ms_sum as f64 / 1000.0),
            )
        } else {
            None
        };

        let total_cost_usd_femto = self.total_cost_usd_femto.max(0.0);
        let cost_usd = if self.cost_covered_success > 0 && total_cost_usd_femto > 0.0 {
            Some(total_cost_usd_femto / USD_FEMTO_DENOM)
        } else {
            None
        };

        super::UsageLeaderboardRow {
            key,
            name,
            requests_total: self.requests_total,
            requests_success: self.requests_success,
            requests_failed: self.requests_failed,
            total_duration_ms: self.total_duration_ms,
            first_request_created_at_ms: self.first_request_created_at_ms,
            last_request_created_at_ms: self.last_request_created_at_ms,
            total_tokens: self.total_tokens,
            io_total_tokens: self.input_tokens.saturating_add(self.output_tokens),
            input_tokens: self.input_tokens,
            output_tokens: self.output_tokens,
            cache_creation_input_tokens: self.cache_creation_input_tokens,
            cache_read_input_tokens: self.cache_read_input_tokens,
            avg_duration_ms,
            avg_ttfb_ms,
            avg_output_tokens_per_second,
            cost_usd,
        }
    }
}

pub(super) fn has_valid_provider_key(key: &ProviderKey) -> bool {
    if key.provider_id <= 0 {
        return false;
    }
    let name = key.provider_name.trim();
    if name.is_empty() {
        return false;
    }
    if name == "Unknown" {
        return false;
    }
    true
}

fn resolve_range_filters<'a>(
    conn: &Connection,
    range: &str,
    cli_key: Option<&'a str>,
) -> Result<(Option<i64>, Option<&'a str>), String> {
    let range = parse_range(range)?;
    let start_ts = compute_start_ts(conn, range)?;
    let cli_key = normalize_cli_filter(cli_key)?;
    Ok((start_ts, cli_key))
}

fn provider_leaderboard_query() -> String {
    let effective_input_expr = sql_effective_input_tokens_expr();
    let canonical_buckets_missing_expr = SQL_CANONICAL_BUCKETS_MISSING;

    format!(
        r#"
SELECT
  cli_key,
  provider_id,
  provider_name,
  requests_total,
  requests_success,
  requests_failed,
  success_duration_ms_sum,
  success_ttfb_ms_sum,
  success_ttfb_ms_count,
  success_generation_ms_sum,
  success_output_tokens_for_rate_sum,
  input_tokens,
  output_tokens,
  input_tokens + output_tokens + cache_read_input_tokens + cache_creation_input_tokens + legacy_total_tokens AS total_tokens,
  cache_read_input_tokens,
  cache_creation_input_tokens,
  cache_creation_5m_input_tokens,
  cache_creation_1h_input_tokens
FROM (
  SELECT
    cli_key,
    final_provider_id AS provider_id,
    NULLIF(TRIM(provider_name_snapshot), '') AS provider_name,
    COUNT(*) AS requests_total,
    SUM(CASE WHEN status >= 200 AND status < 300 AND error_present = 0 THEN 1 ELSE 0 END) AS requests_success,
    SUM(CASE WHEN status >= 200 AND status < 300 AND error_present = 0 THEN 0 ELSE 1 END) AS requests_failed,
    SUM(CASE WHEN status >= 200 AND status < 300 AND error_present = 0 THEN COALESCE(duration_ms, 0) ELSE 0 END) AS success_duration_ms_sum,
    SUM(
      CASE WHEN (
        status >= 200 AND status < 300 AND error_present = 0 AND
        ttfb_ms IS NOT NULL AND ttfb_ms < COALESCE(duration_ms, 0)
      ) THEN ttfb_ms ELSE 0 END
    ) AS success_ttfb_ms_sum,
    SUM(
      CASE WHEN (
        status >= 200 AND status < 300 AND error_present = 0 AND
        ttfb_ms IS NOT NULL AND ttfb_ms < COALESCE(duration_ms, 0)
      ) THEN 1 ELSE 0 END
    ) AS success_ttfb_ms_count,
    SUM(
      CASE WHEN (
        status >= 200 AND status < 300 AND error_present = 0 AND
        output_tokens IS NOT NULL AND
        ttfb_ms IS NOT NULL AND ttfb_ms < COALESCE(duration_ms, 0)
      ) THEN COALESCE(duration_ms, 0) - ttfb_ms ELSE 0 END
    ) AS success_generation_ms_sum,
    SUM(
      CASE WHEN (
        status >= 200 AND status < 300 AND error_present = 0 AND
        output_tokens IS NOT NULL AND
        ttfb_ms IS NOT NULL AND ttfb_ms < COALESCE(duration_ms, 0)
      ) THEN output_tokens ELSE 0 END
    ) AS success_output_tokens_for_rate_sum,
    SUM({effective_input_expr}) AS input_tokens,
    SUM(COALESCE(output_tokens, 0)) AS output_tokens,
    SUM(COALESCE(cache_read_input_tokens, 0)) AS cache_read_input_tokens,
    SUM(COALESCE(cache_creation_input_tokens, 0)) AS cache_creation_input_tokens,
    SUM(COALESCE(cache_creation_5m_input_tokens, 0)) AS cache_creation_5m_input_tokens,
    SUM(COALESCE(cache_creation_1h_input_tokens, 0)) AS cache_creation_1h_input_tokens,
    SUM(
      CASE WHEN {canonical_buckets_missing_expr}
      THEN COALESCE(total_tokens, 0) ELSE 0 END
    ) AS legacy_total_tokens
  FROM usage_events
  WHERE excluded_from_stats = 0
    AND (?1 IS NULL OR created_at >= ?1)
    AND (?2 IS NULL OR cli_key = ?2)
    AND final_provider_id IS NOT NULL
    AND final_provider_id > 0
    AND NULLIF(TRIM(provider_name_snapshot), '') IS NOT NULL
    AND TRIM(provider_name_snapshot) != 'Unknown'
  GROUP BY cli_key, final_provider_id, NULLIF(TRIM(provider_name_snapshot), '')
) aggregated
ORDER BY total_tokens DESC, requests_total DESC, cli_key ASC, provider_name ASC
LIMIT ?3
"#
    )
}

pub fn leaderboard_provider(
    db: &db::Db,
    range: &str,
    cli_key: Option<&str>,
    limit: usize,
) -> crate::shared::error::AppResult<Vec<UsageProviderRow>> {
    let conn = db.open_connection()?;
    let (start_ts, cli_key) = resolve_range_filters(&conn, range, cli_key)?;

    let query = provider_leaderboard_query();

    let mut stmt = conn
        .prepare_cached(&query)
        .map_err(|e| db_err!("failed to prepare provider leaderboard query: {e}"))?;

    let rows = stmt
        .query_map(
            params![
                start_ts,
                cli_key,
                i64::try_from(limit.max(1)).unwrap_or(i64::MAX)
            ],
            |row| {
                let requests_success: i64 = row.get("requests_success")?;
                let success_duration_ms_sum: i64 = row.get("success_duration_ms_sum")?;
                let success_ttfb_ms_count: i64 = row.get("success_ttfb_ms_count")?;
                let success_ttfb_ms_sum: i64 = row.get("success_ttfb_ms_sum")?;
                let success_generation_ms_sum: i64 = row.get("success_generation_ms_sum")?;
                let success_output_tokens_for_rate_sum: i64 =
                    row.get("success_output_tokens_for_rate_sum")?;

                Ok(UsageProviderRow {
                    cli_key: row.get("cli_key")?,
                    provider_id: row.get("provider_id")?,
                    provider_name: row.get("provider_name")?,
                    requests_total: row.get("requests_total")?,
                    requests_success,
                    requests_failed: row.get("requests_failed")?,
                    avg_duration_ms: (requests_success > 0)
                        .then(|| success_duration_ms_sum / requests_success),
                    avg_ttfb_ms: (success_ttfb_ms_count > 0)
                        .then(|| success_ttfb_ms_sum / success_ttfb_ms_count),
                    avg_output_tokens_per_second: (success_generation_ms_sum > 0).then(|| {
                        success_output_tokens_for_rate_sum as f64
                            / (success_generation_ms_sum as f64 / 1000.0)
                    }),
                    input_tokens: row.get("input_tokens")?,
                    output_tokens: row.get("output_tokens")?,
                    total_tokens: row.get("total_tokens")?,
                    cache_read_input_tokens: row.get("cache_read_input_tokens")?,
                    cache_creation_input_tokens: row.get("cache_creation_input_tokens")?,
                    cache_creation_5m_input_tokens: row.get("cache_creation_5m_input_tokens")?,
                    cache_creation_1h_input_tokens: row.get("cache_creation_1h_input_tokens")?,
                })
            },
        )
        .map_err(|e| db_err!("failed to run provider leaderboard query: {e}"))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| db_err!("failed to read provider leaderboard row: {e}"))?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_leaderboard_aggregates_orders_and_limits_in_sql() {
        let query = provider_leaderboard_query();

        assert!(query.contains("COUNT(*) AS requests_total"));
        assert!(query.contains(
            "GROUP BY cli_key, final_provider_id, NULLIF(TRIM(provider_name_snapshot), '')"
        ));
        assert!(query.contains(
            "ORDER BY total_tokens DESC, requests_total DESC, cli_key ASC, provider_name ASC"
        ));
        assert!(query.contains("LIMIT ?3"));
    }
}

pub fn leaderboard_day(
    db: &db::Db,
    range: &str,
    cli_key: Option<&str>,
    limit: usize,
) -> crate::shared::error::AppResult<Vec<UsageDayRow>> {
    let conn = db.open_connection()?;
    let (start_ts, cli_key) = resolve_range_filters(&conn, range, cli_key)?;

    let effective_input_expr = sql_effective_input_tokens_expr();
    let canonical_buckets_missing_expr = SQL_CANONICAL_BUCKETS_MISSING;
    let query = format!(
        r#"
    SELECT
      day,
      requests_total,
      input_tokens,
      output_tokens,
      input_tokens + output_tokens + cache_read_input_tokens + cache_creation_input_tokens + legacy_total_tokens AS total_tokens,
      cache_read_input_tokens,
      cache_creation_input_tokens,
      cache_creation_5m_input_tokens,
      cache_creation_1h_input_tokens
    FROM (
    SELECT
      strftime('%Y-%m-%d', created_at, 'unixepoch', 'localtime') AS day,
      COUNT(*) AS requests_total,
      SUM({effective_input_expr}) AS input_tokens,
      SUM(COALESCE(output_tokens, 0)) AS output_tokens,
      SUM(COALESCE(cache_read_input_tokens, 0)) AS cache_read_input_tokens,
      SUM(COALESCE(cache_creation_input_tokens, 0)) AS cache_creation_input_tokens,
      SUM(COALESCE(cache_creation_5m_input_tokens, 0)) AS cache_creation_5m_input_tokens,
      SUM(COALESCE(cache_creation_1h_input_tokens, 0)) AS cache_creation_1h_input_tokens,
      SUM(CASE WHEN {canonical_buckets_missing_expr}
      THEN COALESCE(total_tokens, 0) ELSE 0 END) AS legacy_total_tokens
    FROM usage_events
    WHERE excluded_from_stats = 0
    AND (?1 IS NULL OR created_at >= ?1)
    AND (?2 IS NULL OR cli_key = ?2)
    GROUP BY day
    ) aggregated
    ORDER BY total_tokens DESC, day DESC
    LIMIT ?3
	    "#
    );

    let mut stmt = conn
        .prepare_cached(&query)
        .map_err(|e| db_err!("failed to prepare day leaderboard query: {e}"))?;

    let rows = stmt
        .query_map(params![start_ts, cli_key, limit as i64], |row| {
            Ok(UsageDayRow {
                day: row.get("day")?,
                requests_total: row.get("requests_total")?,
                input_tokens: row.get::<_, Option<i64>>("input_tokens")?.unwrap_or(0),
                output_tokens: row.get::<_, Option<i64>>("output_tokens")?.unwrap_or(0),
                total_tokens: row.get::<_, Option<i64>>("total_tokens")?.unwrap_or(0),
                cache_read_input_tokens: row
                    .get::<_, Option<i64>>("cache_read_input_tokens")?
                    .unwrap_or(0),
                cache_creation_input_tokens: row
                    .get::<_, Option<i64>>("cache_creation_input_tokens")?
                    .unwrap_or(0),
                cache_creation_5m_input_tokens: row
                    .get::<_, Option<i64>>("cache_creation_5m_input_tokens")?
                    .unwrap_or(0),
                cache_creation_1h_input_tokens: row
                    .get::<_, Option<i64>>("cache_creation_1h_input_tokens")?
                    .unwrap_or(0),
            })
        })
        .map_err(|e| db_err!("failed to run day leaderboard query: {e}"))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| db_err!("failed to read day row: {e}"))?);
    }
    Ok(out)
}
