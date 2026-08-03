use crate::shared::error::db_err;
use rusqlite::{params, params_from_iter, types::Value, Connection};

use super::filters::sql_exclude_cx2cc_gateway_bridge_clause;
use super::{sql_effective_input_tokens_expr_with_alias, UsageTrendGranularityV1};

pub(super) const TREND_MAX_PROVIDERS: usize = 10;
pub(super) const TREND_MAX_BUCKETS: usize = 120;
pub(super) const TREND_MAX_ROWS: usize = TREND_MAX_PROVIDERS * TREND_MAX_BUCKETS;

const SUCCESS: &str = "r.status >= 200 AND r.status < 300 AND r.error_present = 0";
const VALID_TTFB: &str = "r.ttfb_ms IS NOT NULL AND r.ttfb_ms < r.duration_ms";
const VALID_OUTPUT_RATE: &str =
    "r.output_tokens IS NOT NULL AND r.ttfb_ms IS NOT NULL AND r.ttfb_ms < r.duration_ms";

#[derive(Debug, Clone, Copy)]
pub(super) struct TrendPlan {
    pub granularity: UsageTrendGranularityV1,
    pub provider_limit: usize,
    pub start_ts: Option<i64>,
    pub end_ts: Option<i64>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct TrendPlanQuery<'a> {
    pub start_ts: Option<i64>,
    pub end_ts: Option<i64>,
    pub cli_key: Option<&'a str>,
    pub provider_id: Option<i64>,
    pub requested_provider_limit: Option<usize>,
    pub exclude_cx2cc_gateway_bridge: bool,
}

#[derive(Debug, Clone, Copy)]
struct BucketCounts {
    hours: usize,
    days: usize,
    weeks: usize,
    months: usize,
    years: usize,
}

#[derive(Debug)]
enum RawSelection {
    Full,
    Gaps,
}

#[derive(Debug)]
struct RollupCoverage {
    use_rollups: bool,
    raw_selection: RawSelection,
}

fn normalized_provider_limit(provider_id: Option<i64>, requested: Option<usize>) -> usize {
    if provider_id.is_some() {
        return 1;
    }
    requested
        .unwrap_or(TREND_MAX_PROVIDERS)
        .clamp(1, TREND_MAX_PROVIDERS)
}

fn optional_i64_value(value: Option<i64>) -> Value {
    value.map(Value::Integer).unwrap_or(Value::Null)
}

fn optional_text_value(value: Option<&str>) -> Value {
    value
        .map(|value| Value::Text(value.to_string()))
        .unwrap_or(Value::Null)
}

fn base_query_params(query: TrendPlanQuery<'_>) -> Vec<Value> {
    vec![
        optional_i64_value(query.start_ts),
        optional_i64_value(query.end_ts),
        optional_text_value(query.cli_key),
        optional_i64_value(query.provider_id),
    ]
}

fn daily_rollup_schema_available(conn: &Connection) -> Result<bool, String> {
    let tables_available: bool = conn
        .query_row(
            r#"
SELECT COUNT(*) = 4
FROM sqlite_master
WHERE type = 'table'
  AND name IN (
    'usage_provider_daily_rollup_days',
    'usage_provider_daily_rollups',
    'usage_provider_daily_rollup_backfill_state',
    'usage_ledger_backfill_state'
  )
"#,
            [],
            |row| row.get(0),
        )
        .map_err(|error| db_err!("failed to inspect Provider daily rollup schema: {error}"))?;
    if !tables_available {
        return Ok(false);
    }

    let backfill_ready: bool = conn
        .query_row(
            r#"
SELECT EXISTS (
  SELECT 1
  FROM usage_provider_daily_rollup_backfill_state rollup_state
  JOIN usage_ledger_backfill_state ledger_state ON ledger_state.id = 1
  WHERE rollup_state.id = 1
    AND ledger_state.status = 'complete'
)
"#,
            [],
            |row| row.get(0),
        )
        .map_err(|error| db_err!("failed to inspect Provider daily rollup readiness: {error}"))?;

    Ok(backfill_ready)
}

fn extent_from_min_max(
    min_ts: Option<i64>,
    max_ts: Option<i64>,
    requested_end_ts: Option<i64>,
) -> Option<(i64, i64)> {
    match (min_ts, max_ts, requested_end_ts) {
        (Some(min_ts), Some(_), Some(end_ts)) => Some((min_ts, end_ts)),
        (Some(min_ts), Some(max_ts), None) => Some((min_ts, max_ts.saturating_add(1))),
        _ => None,
    }
}

fn raw_filtered_extent(
    conn: &Connection,
    query: TrendPlanQuery<'_>,
) -> Result<Option<(i64, i64)>, String> {
    let raw_cx2cc =
        sql_exclude_cx2cc_gateway_bridge_clause(Some("r"), query.exclude_cx2cc_gateway_bridge);
    let sql = format!(
        r#"
WITH query_args(start_ts, end_ts, cli_key, provider_id) AS (
  VALUES (?1, ?2, ?3, ?4)
)
SELECT MIN(r.created_at), MAX(r.created_at)
FROM usage_events r
CROSS JOIN query_args q
WHERE r.excluded_from_stats = 0
  AND r.final_provider_id IS NOT NULL
  AND r.final_provider_id > 0
  AND (q.start_ts IS NULL OR r.created_at >= q.start_ts)
  AND (q.end_ts IS NULL OR r.created_at < q.end_ts)
  AND (q.cli_key IS NULL OR r.cli_key = q.cli_key)
  AND (q.provider_id IS NULL OR r.final_provider_id = q.provider_id)
  {raw_cx2cc}
"#
    );
    let (min_ts, max_ts): (Option<i64>, Option<i64>) = conn
        .query_row(&sql, params_from_iter(base_query_params(query)), |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .map_err(|error| db_err!("failed to resolve provider trend extent: {error}"))?;
    Ok(extent_from_min_max(min_ts, max_ts, query.end_ts))
}

fn rollup_filtered_extent(
    conn: &Connection,
    query: TrendPlanQuery<'_>,
) -> Result<Option<(i64, i64)>, String> {
    let has_trusted_days = conn
        .query_row(
            r#"
SELECT EXISTS (
  SELECT 1
  FROM usage_provider_daily_rollup_days d
  WHERE d.status = 'complete'
    AND d.day_start_ts = CAST(strftime(
      '%s', d.local_day || ' 00:00:00', 'utc'
    ) AS INTEGER)
    AND d.day_end_ts = CAST(strftime(
      '%s', datetime(d.local_day, '+1 day'), 'utc'
    ) AS INTEGER)
    AND d.source_row_count = COALESCE((
      SELECT SUM(check_rollup.requests_total)
      FROM usage_provider_daily_rollups check_rollup
      WHERE check_rollup.local_day = d.local_day
    ), 0)
    AND (?1 IS NULL OR d.day_end_ts <= ?1)
)
"#,
            [query.end_ts],
            |row| row.get::<_, bool>(0),
        )
        .map_err(|error| db_err!("failed to inspect Provider rollup extent coverage: {error}"))?;
    if !has_trusted_days {
        return raw_filtered_extent(conn, query);
    }

    let raw_cx2cc =
        sql_exclude_cx2cc_gateway_bridge_clause(Some("r"), query.exclude_cx2cc_gateway_bridge);
    let rollup_cx2cc =
        sql_exclude_cx2cc_gateway_bridge_clause(Some("rr"), query.exclude_cx2cc_gateway_bridge);
    let sql = format!(
        r#"
WITH query_args(start_ts, end_ts, cli_key, provider_id) AS (
  VALUES (?1, ?2, ?3, ?4)
),
trusted_days(local_day, day_start_ts, day_end_ts) AS MATERIALIZED (
  SELECT d.local_day, d.day_start_ts, d.day_end_ts
  FROM usage_provider_daily_rollup_days d
  CROSS JOIN query_args q
  WHERE d.status = 'complete'
    AND d.day_start_ts = CAST(strftime(
      '%s', d.local_day || ' 00:00:00', 'utc'
    ) AS INTEGER)
    AND d.day_end_ts = CAST(strftime(
      '%s', datetime(d.local_day, '+1 day'), 'utc'
    ) AS INTEGER)
    AND d.source_row_count = COALESCE((
      SELECT SUM(check_rollup.requests_total)
      FROM usage_provider_daily_rollups check_rollup
      WHERE check_rollup.local_day = d.local_day
    ), 0)
    AND EXISTS (
      SELECT 1
      FROM usage_provider_daily_rollup_backfill_state rollup_state
      JOIN usage_ledger_backfill_state ledger_state ON ledger_state.id = 1
      WHERE rollup_state.id = 1
        AND ledger_state.status = 'complete'
    )
    AND (q.end_ts IS NULL OR d.day_end_ts <= q.end_ts)
),
trusted_bounds(first_start_ts, last_end_ts) AS (
  SELECT MIN(day_start_ts), MAX(day_end_ts)
  FROM trusted_days
),
coverage_with_previous AS MATERIALIZED (
  SELECT
    trusted.day_start_ts,
    trusted.day_end_ts,
    LAG(trusted.day_end_ts) OVER (ORDER BY trusted.day_start_ts) AS previous_end_ts
  FROM trusted_days trusted
),
raw_intervals(start_ts, end_ts) AS MATERIALIZED (
  SELECT previous_end_ts, day_start_ts
  FROM coverage_with_previous
  WHERE previous_end_ts IS NOT NULL
    AND previous_end_ts < day_start_ts
),
extent_parts(min_ts, max_ts) AS (
  SELECT MIN(r.created_at), MAX(r.created_at)
  FROM usage_events r
  CROSS JOIN query_args q
  WHERE NOT EXISTS (SELECT 1 FROM trusted_days)
    AND (q.end_ts IS NULL OR r.created_at < q.end_ts)
    AND r.excluded_from_stats = 0
    AND r.final_provider_id IS NOT NULL
    AND r.final_provider_id > 0
    AND (q.cli_key IS NULL OR r.cli_key = q.cli_key)
    AND (q.provider_id IS NULL OR r.final_provider_id = q.provider_id)
    {raw_cx2cc}

  UNION ALL

  SELECT MIN(r.created_at), MAX(r.created_at)
  FROM usage_events r
  CROSS JOIN query_args q
  CROSS JOIN trusted_bounds bounds
  WHERE r.created_at < bounds.first_start_ts
    AND (q.end_ts IS NULL OR r.created_at < q.end_ts)
    AND r.excluded_from_stats = 0
    AND r.final_provider_id IS NOT NULL
    AND r.final_provider_id > 0
    AND (q.cli_key IS NULL OR r.cli_key = q.cli_key)
    AND (q.provider_id IS NULL OR r.final_provider_id = q.provider_id)
    {raw_cx2cc}

  UNION ALL

  SELECT MIN(r.created_at), MAX(r.created_at)
  FROM raw_intervals gap
  JOIN usage_events r
    ON r.created_at >= gap.start_ts
   AND r.created_at < gap.end_ts
  CROSS JOIN query_args q
  WHERE r.excluded_from_stats = 0
    AND r.final_provider_id IS NOT NULL
    AND r.final_provider_id > 0
    AND (q.cli_key IS NULL OR r.cli_key = q.cli_key)
    AND (q.provider_id IS NULL OR r.final_provider_id = q.provider_id)
    {raw_cx2cc}

  UNION ALL

  SELECT MIN(r.created_at), MAX(r.created_at)
  FROM usage_events r
  CROSS JOIN query_args q
  CROSS JOIN trusted_bounds bounds
  WHERE r.created_at >= bounds.last_end_ts
    AND (q.end_ts IS NULL OR r.created_at < q.end_ts)
    AND r.excluded_from_stats = 0
    AND r.final_provider_id IS NOT NULL
    AND r.final_provider_id > 0
    AND (q.cli_key IS NULL OR r.cli_key = q.cli_key)
    AND (q.provider_id IS NULL OR r.final_provider_id = q.provider_id)
    {raw_cx2cc}

  UNION ALL

  SELECT MIN(rr.created_at_min), MAX(rr.created_at_max)
  FROM usage_provider_daily_rollups rr
  JOIN trusted_days trusted ON trusted.local_day = rr.local_day
  CROSS JOIN query_args q
  WHERE (q.cli_key IS NULL OR rr.cli_key = q.cli_key)
    AND (q.provider_id IS NULL OR rr.final_provider_id = q.provider_id)
    {rollup_cx2cc}
)
SELECT MIN(min_ts), MAX(max_ts)
FROM extent_parts
"#
    );
    let (min_ts, max_ts): (Option<i64>, Option<i64>) = conn
        .query_row(&sql, params_from_iter(base_query_params(query)), |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .map_err(|error| db_err!("failed to resolve Provider hybrid trend extent: {error}"))?;
    Ok(extent_from_min_max(min_ts, max_ts, query.end_ts))
}

fn filtered_extent(
    conn: &Connection,
    query: TrendPlanQuery<'_>,
) -> Result<Option<(i64, i64)>, String> {
    if let (Some(start_ts), Some(end_ts)) = (query.start_ts, query.end_ts) {
        return Ok(Some((start_ts, end_ts)));
    }

    if let Some(start_ts) = query.start_ts {
        let now_exclusive = conn
            .query_row(
                "SELECT CAST(strftime('%s', 'now') AS INTEGER) + 1",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|error| db_err!("failed to resolve provider trend current time: {error}"))?;
        return Ok(Some((
            start_ts,
            now_exclusive.max(start_ts.saturating_add(1)),
        )));
    }

    if daily_rollup_schema_available(conn)? {
        rollup_filtered_extent(conn, query)
    } else {
        raw_filtered_extent(conn, query)
    }
}

fn bucket_counts_for_extent(
    conn: &Connection,
    start_ts: i64,
    end_ts: i64,
) -> Result<BucketCounts, String> {
    let end_inclusive = end_ts.saturating_sub(1).max(start_ts);
    let counts = conn
        .query_row(
            r#"
WITH local_parts AS (
  SELECT
    CAST(strftime('%H', ?1, 'unixepoch', 'localtime') AS INTEGER) AS start_hour,
    CAST(strftime('%H', ?2, 'unixepoch', 'localtime') AS INTEGER) AS end_hour,
    CAST(strftime('%m', ?1, 'unixepoch', 'localtime') AS INTEGER) AS start_month,
    CAST(strftime('%m', ?2, 'unixepoch', 'localtime') AS INTEGER) AS end_month,
    CAST(strftime('%Y', ?1, 'unixepoch', 'localtime') AS INTEGER) AS start_year,
    CAST(strftime('%Y', ?2, 'unixepoch', 'localtime') AS INTEGER) AS end_year,
    CAST(
      julianday(date(?2, 'unixepoch', 'localtime')) -
      julianday(date(?1, 'unixepoch', 'localtime'))
      AS INTEGER
    ) AS day_delta,
    CAST(
      (
        julianday(date(
          ?2,
          'unixepoch',
          'localtime',
          '-' || ((CAST(strftime('%w', ?2, 'unixepoch', 'localtime') AS INTEGER) + 6) % 7) || ' days'
        )) -
        julianday(date(
          ?1,
          'unixepoch',
          'localtime',
          '-' || ((CAST(strftime('%w', ?1, 'unixepoch', 'localtime') AS INTEGER) + 6) % 7) || ' days'
        ))
      ) / 7
      AS INTEGER
    ) AS week_delta
)
SELECT
  day_delta * 24 + end_hour - start_hour + 1 AS hours,
  day_delta + 1 AS days,
  week_delta + 1 AS weeks,
  (end_year - start_year) * 12 + end_month - start_month + 1 AS months,
  end_year - start_year + 1 AS years
FROM local_parts
            "#,
            params![start_ts, end_inclusive],
            |row| {
                Ok(BucketCounts {
                    hours: row.get::<_, i64>(0)?.max(1) as usize,
                    days: row.get::<_, i64>(1)?.max(1) as usize,
                    weeks: row.get::<_, i64>(2)?.max(1) as usize,
                    months: row.get::<_, i64>(3)?.max(1) as usize,
                    years: row.get::<_, i64>(4)?.max(1) as usize,
                })
            },
        )
        .map_err(|error| {
            db_err!("failed to compute provider trend bucket budget: {error}")
        })?;
    Ok(counts)
}

fn finest_bounded_granularity(counts: BucketCounts) -> Result<UsageTrendGranularityV1, String> {
    if counts.hours <= TREND_MAX_BUCKETS {
        Ok(UsageTrendGranularityV1::Hour)
    } else if counts.days <= TREND_MAX_BUCKETS {
        Ok(UsageTrendGranularityV1::Day)
    } else if counts.weeks <= TREND_MAX_BUCKETS {
        Ok(UsageTrendGranularityV1::Week)
    } else if counts.months <= TREND_MAX_BUCKETS {
        Ok(UsageTrendGranularityV1::Month)
    } else if counts.years <= TREND_MAX_BUCKETS {
        Ok(UsageTrendGranularityV1::Year)
    } else {
        Err(format!(
            "SEC_INVALID_INPUT: provider trend range exceeds the {TREND_MAX_BUCKETS}-bucket year budget"
        ))
    }
}

pub(super) fn plan_trend(
    conn: &Connection,
    query: TrendPlanQuery<'_>,
) -> Result<TrendPlan, String> {
    let provider_limit =
        normalized_provider_limit(query.provider_id, query.requested_provider_limit);
    let extent = filtered_extent(conn, query)?;
    let granularity = match extent {
        Some((start_ts, end_ts)) => {
            finest_bounded_granularity(bucket_counts_for_extent(conn, start_ts, end_ts)?)?
        }
        None => UsageTrendGranularityV1::Hour,
    };
    Ok(TrendPlan {
        granularity,
        provider_limit,
        start_ts: extent.map(|extent| extent.0),
        end_ts: extent.map(|extent| extent.1),
    })
}

fn bucket_select_and_group(
    granularity: UsageTrendGranularityV1,
    timestamp_expr: &str,
) -> (String, &'static str) {
    let fields = match granularity {
        UsageTrendGranularityV1::Hour => format!(
            "strftime('%Y-%m-%d', {timestamp_expr}, 'unixepoch', 'localtime') AS day, CAST(strftime('%H', {timestamp_expr}, 'unixepoch', 'localtime') AS INTEGER) AS hour"
        ),
        UsageTrendGranularityV1::Day => format!(
            "strftime('%Y-%m-%d', {timestamp_expr}, 'unixepoch', 'localtime') AS day, NULL AS hour"
        ),
        UsageTrendGranularityV1::Week => format!(
            "strftime('%Y-%m-%d', {timestamp_expr}, 'unixepoch', 'localtime', '-' || ((CAST(strftime('%w', {timestamp_expr}, 'unixepoch', 'localtime') AS INTEGER) + 6) % 7) || ' days') AS day, NULL AS hour"
        ),
        UsageTrendGranularityV1::Month => format!(
            "strftime('%Y-%m', {timestamp_expr}, 'unixepoch', 'localtime') AS day, NULL AS hour"
        ),
        UsageTrendGranularityV1::Year => format!(
            "strftime('%Y', {timestamp_expr}, 'unixepoch', 'localtime') AS day, NULL AS hour"
        ),
    };
    let group_by = match granularity {
        UsageTrendGranularityV1::Hour => "day, hour",
        UsageTrendGranularityV1::Day
        | UsageTrendGranularityV1::Week
        | UsageTrendGranularityV1::Month
        | UsageTrendGranularityV1::Year => "day",
    };
    (fields, group_by)
}

fn rollup_bucket_select_and_group(
    granularity: UsageTrendGranularityV1,
) -> (&'static str, &'static str) {
    match granularity {
        UsageTrendGranularityV1::Hour => unreachable!("hour trends never use daily rollups"),
        UsageTrendGranularityV1::Day => ("r.local_day AS day, NULL AS hour", "day"),
        UsageTrendGranularityV1::Week => (
            "date(r.local_day, '-' || ((CAST(strftime('%w', r.local_day) AS INTEGER) + 6) % 7) || ' days') AS day, NULL AS hour",
            "day",
        ),
        UsageTrendGranularityV1::Month => {
            ("substr(r.local_day, 1, 7) AS day, NULL AS hour", "day")
        }
        UsageTrendGranularityV1::Year => {
            ("substr(r.local_day, 1, 4) AS day, NULL AS hour", "day")
        }
    }
}

fn rollup_coverage(conn: &Connection, plan: TrendPlan) -> Result<RollupCoverage, String> {
    let Some((start_ts, end_ts)) = plan.start_ts.zip(plan.end_ts) else {
        return Ok(RollupCoverage {
            use_rollups: false,
            raw_selection: RawSelection::Full,
        });
    };
    if plan.granularity == UsageTrendGranularityV1::Hour || !daily_rollup_schema_available(conn)? {
        return Ok(RollupCoverage {
            use_rollups: false,
            raw_selection: RawSelection::Full,
        });
    }

    let eligible = conn
        .query_row(
            r#"
SELECT EXISTS (
  SELECT 1
  FROM usage_provider_daily_rollup_days d
  WHERE d.status = 'complete'
    AND d.day_start_ts = CAST(strftime(
      '%s', d.local_day || ' 00:00:00', 'utc'
    ) AS INTEGER)
    AND d.day_end_ts = CAST(strftime(
      '%s', datetime(d.local_day, '+1 day'), 'utc'
    ) AS INTEGER)
    AND d.source_row_count = COALESCE((
      SELECT SUM(r.requests_total)
      FROM usage_provider_daily_rollups r
      WHERE r.local_day = d.local_day
    ), 0)
    AND d.day_start_ts >= ?1
    AND d.day_end_ts <= ?2
)
"#,
            params![start_ts, end_ts],
            |row| row.get::<_, bool>(0),
        )
        .map_err(|error| db_err!("failed to query Provider rollup coverage: {error}"))?;
    if !eligible {
        return Ok(RollupCoverage {
            use_rollups: false,
            raw_selection: RawSelection::Full,
        });
    }

    Ok(RollupCoverage {
        use_rollups: true,
        raw_selection: RawSelection::Gaps,
    })
}

fn valid_rollup_day_clause() -> &'static str {
    r#"
  AND d.status = 'complete'
  AND d.day_start_ts = CAST(strftime(
    '%s', d.local_day || ' 00:00:00', 'utc'
  ) AS INTEGER)
  AND d.day_end_ts = CAST(strftime(
    '%s', datetime(d.local_day, '+1 day'), 'utc'
  ) AS INTEGER)
  AND d.source_row_count = COALESCE((
    SELECT SUM(check_rollup.requests_total)
    FROM usage_provider_daily_rollups check_rollup
    WHERE check_rollup.local_day = d.local_day
  ), 0)
  AND EXISTS (
    SELECT 1
    FROM usage_provider_daily_rollup_backfill_state rollup_state
    JOIN usage_ledger_backfill_state ledger_state ON ledger_state.id = 1
    WHERE rollup_state.id = 1
      AND ledger_state.status = 'complete'
  )
  AND d.day_start_ts >= q.start_ts
  AND d.day_end_ts <= q.end_ts
"#
}

pub(super) fn build_trend_source_ctes(
    conn: &Connection,
    plan: TrendPlan,
    query: TrendPlanQuery<'_>,
) -> Result<String, String> {
    let coverage = rollup_coverage(conn, plan)?;
    let raw_cx2cc =
        sql_exclude_cx2cc_gateway_bridge_clause(Some("r"), query.exclude_cx2cc_gateway_bridge);
    let rollup_cx2cc =
        sql_exclude_cx2cc_gateway_bridge_clause(Some("r"), query.exclude_cx2cc_gateway_bridge);
    let effective_input_expr = sql_effective_input_tokens_expr_with_alias("r");
    let cache_denom_expr = format!(
        "({effective_input_expr}) + COALESCE(r.cache_creation_input_tokens, 0) + COALESCE(r.cache_read_input_tokens, 0)"
    );
    let (raw_bucket_fields, raw_group_by) =
        bucket_select_and_group(plan.granularity, "r.created_at");

    let trusted_days_cte = if coverage.use_rollups {
        format!(
            r#"
trusted_days(local_day, day_start_ts, day_end_ts) AS MATERIALIZED (
  SELECT d.local_day, d.day_start_ts, d.day_end_ts
  FROM usage_provider_daily_rollup_days d
  CROSS JOIN query_args q
  WHERE 1 = 1
    {valid_rollup_day}
),
"#,
            valid_rollup_day = valid_rollup_day_clause(),
        )
    } else {
        String::new()
    };
    let (raw_intervals_cte, raw_from) = match &coverage.raw_selection {
        RawSelection::Full => (
            String::new(),
            "FROM usage_events r CROSS JOIN query_args q".to_string(),
        ),
        RawSelection::Gaps => (
            r#"
coverage_with_previous AS MATERIALIZED (
  SELECT
    trusted.day_start_ts,
    trusted.day_end_ts,
    COALESCE(
      LAG(trusted.day_end_ts) OVER (ORDER BY trusted.day_start_ts),
      q.start_ts
    ) AS previous_end_ts
  FROM trusted_days trusted
  CROSS JOIN query_args q
),
raw_intervals(start_ts, end_ts) AS MATERIALIZED (
  SELECT previous_end_ts, day_start_ts
  FROM coverage_with_previous
  WHERE previous_end_ts < day_start_ts

  UNION ALL

  SELECT COALESCE(MAX(trusted.day_end_ts), q.start_ts), q.end_ts
  FROM query_args q
  LEFT JOIN trusted_days trusted ON 1 = 1
  GROUP BY q.start_ts, q.end_ts
  HAVING COALESCE(MAX(trusted.day_end_ts), q.start_ts) < q.end_ts
),
"#
            .to_string(),
            "FROM usage_events r\nJOIN raw_intervals ri\n  ON r.created_at >= ri.start_ts\n AND r.created_at < ri.end_ts\nCROSS JOIN query_args q".to_string(),
        ),
    };

    let raw_part = Some(format!(
        r#"
SELECT
  {raw_bucket_fields},
  r.cli_key AS cli_key,
  r.final_provider_id AS provider_id,
  MAX(NULLIF(TRIM(r.provider_name_snapshot), '')) AS provider_name_all,
  MAX(CASE WHEN {success} THEN NULLIF(TRIM(r.provider_name_snapshot), '') END) AS provider_name_success,
  MIN(r.created_at) AS created_at_min,
  MAX(r.created_at) AS created_at_max,
  COUNT(*) AS requests_total,
  SUM(CASE WHEN {success} THEN 1 ELSE 0 END) AS requests_success,
  SUM(CASE WHEN {success} THEN r.duration_ms ELSE 0 END) AS success_duration_ms_sum,
  SUM(CASE WHEN {success} AND {valid_ttfb} THEN r.ttfb_ms ELSE 0 END) AS success_ttfb_ms_sum,
  SUM(CASE WHEN {success} AND {valid_ttfb} THEN 1 ELSE 0 END) AS success_ttfb_ms_count,
  SUM(CASE WHEN {success} AND {valid_output_rate} THEN r.duration_ms - r.ttfb_ms ELSE 0 END) AS success_generation_ms_sum,
  SUM(CASE WHEN {success} AND {valid_output_rate} THEN r.output_tokens ELSE 0 END) AS success_output_tokens_for_rate_sum,
  SUM(CASE WHEN {success} AND {valid_output_rate} THEN 1 ELSE 0 END) AS success_output_rate_count,
  SUM(CASE WHEN {success} THEN {cache_denom_expr} ELSE 0 END) AS cache_denom_tokens,
  SUM(CASE WHEN {success} THEN COALESCE(r.cache_read_input_tokens, 0) ELSE 0 END) AS cache_read_input_tokens
{raw_from}
WHERE r.excluded_from_stats = 0
  AND r.final_provider_id IS NOT NULL
  AND r.final_provider_id > 0
  AND (q.start_ts IS NULL OR r.created_at >= q.start_ts)
  AND (q.end_ts IS NULL OR r.created_at < q.end_ts)
  AND (q.cli_key IS NULL OR r.cli_key = q.cli_key)
  AND (q.provider_id IS NULL OR r.final_provider_id = q.provider_id)
  {raw_cx2cc}
GROUP BY {raw_group_by}, r.cli_key, r.final_provider_id
"#,
        success = SUCCESS,
        valid_ttfb = VALID_TTFB,
        valid_output_rate = VALID_OUTPUT_RATE,
    ));

    let rollup_part = if coverage.use_rollups {
        let (rollup_bucket_fields, rollup_group_by) =
            rollup_bucket_select_and_group(plan.granularity);
        Some(format!(
            r#"
SELECT
  {rollup_bucket_fields},
  r.cli_key AS cli_key,
  r.final_provider_id AS provider_id,
  MAX(r.provider_name_all_snapshot) AS provider_name_all,
  MAX(r.provider_name_success_snapshot) AS provider_name_success,
  MIN(r.created_at_min) AS created_at_min,
  MAX(r.created_at_max) AS created_at_max,
  SUM(r.requests_total) AS requests_total,
  SUM(r.requests_success) AS requests_success,
  SUM(r.success_duration_ms_sum) AS success_duration_ms_sum,
  SUM(r.success_ttfb_ms_sum) AS success_ttfb_ms_sum,
  SUM(r.success_ttfb_ms_count) AS success_ttfb_ms_count,
  SUM(r.success_generation_ms_sum) AS success_generation_ms_sum,
  SUM(r.success_output_tokens_for_rate_sum) AS success_output_tokens_for_rate_sum,
  SUM(r.success_output_rate_count) AS success_output_rate_count,
  SUM(r.cache_denom_tokens) AS cache_denom_tokens,
  SUM(r.cache_read_input_tokens) AS cache_read_input_tokens
FROM usage_provider_daily_rollups r
JOIN trusted_days d ON d.local_day = r.local_day
CROSS JOIN query_args q
WHERE (q.cli_key IS NULL OR r.cli_key = q.cli_key)
  AND (q.provider_id IS NULL OR r.final_provider_id = q.provider_id)
  {rollup_cx2cc}
GROUP BY {rollup_group_by}, r.cli_key, r.final_provider_id
"#
        ))
    } else {
        None
    };

    let source_parts = [raw_part, rollup_part]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("\nUNION ALL\n");
    Ok(format!(
        r#"
query_args(start_ts, end_ts, cli_key, provider_id) AS (
  VALUES (?1, ?2, ?3, ?4)
),
{trusted_days_cte}
{raw_intervals_cte}
source_parts AS (
  {source_parts}
),
trend_source AS MATERIALIZED (
  SELECT
    day,
    hour,
    cli_key,
    provider_id,
    MAX(provider_name_all) AS provider_name_all,
    MAX(provider_name_success) AS provider_name_success,
    MIN(created_at_min) AS created_at_min,
    MAX(created_at_max) AS created_at_max,
    SUM(requests_total) AS requests_total,
    SUM(requests_success) AS requests_success,
    SUM(success_duration_ms_sum) AS success_duration_ms_sum,
    SUM(success_ttfb_ms_sum) AS success_ttfb_ms_sum,
    SUM(success_ttfb_ms_count) AS success_ttfb_ms_count,
    SUM(success_generation_ms_sum) AS success_generation_ms_sum,
    SUM(success_output_tokens_for_rate_sum) AS success_output_tokens_for_rate_sum,
    SUM(success_output_rate_count) AS success_output_rate_count,
    SUM(cache_denom_tokens) AS cache_denom_tokens,
    SUM(cache_read_input_tokens) AS cache_read_input_tokens
  FROM source_parts
  GROUP BY day, hour, cli_key, provider_id
)
"#
    ))
}

pub(super) fn trend_query_params(plan: TrendPlan, query: TrendPlanQuery<'_>) -> Vec<Value> {
    vec![
        optional_i64_value(plan.start_ts),
        optional_i64_value(plan.end_ts),
        optional_text_value(query.cli_key),
        optional_i64_value(query.provider_id),
        Value::Integer(plan.provider_limit as i64),
    ]
}

pub(super) fn validate_trend_budget(
    row_count: usize,
    bucket_count: usize,
    provider_count: usize,
    provider_limit: usize,
) -> Result<(), String> {
    if bucket_count > TREND_MAX_BUCKETS {
        return Err(format!(
            "SYSTEM_ERROR: provider trend bucket budget exceeded ({bucket_count}>{TREND_MAX_BUCKETS})"
        ));
    }
    if provider_count > provider_limit || provider_count > TREND_MAX_PROVIDERS {
        return Err(format!(
            "SYSTEM_ERROR: provider trend provider budget exceeded ({provider_count}>{provider_limit})"
        ));
    }
    let row_limit = provider_limit
        .saturating_mul(TREND_MAX_BUCKETS)
        .min(TREND_MAX_ROWS);
    if row_count > row_limit {
        return Err(format!(
            "SYSTEM_ERROR: provider trend row budget exceeded ({row_count}>{row_limit})"
        ));
    }
    Ok(())
}
