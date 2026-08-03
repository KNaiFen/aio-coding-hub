use crate::shared::error::db_err;
use rusqlite::{params, params_from_iter, Connection};

use super::filters::{
    build_optional_range_cli_provider_filters, sql_exclude_cx2cc_gateway_bridge_clause,
};
use super::UsageTrendGranularityV1;

pub(super) const TREND_MAX_PROVIDERS: usize = 10;
pub(super) const TREND_MAX_BUCKETS: usize = 120;
pub(super) const TREND_MAX_ROWS: usize = TREND_MAX_PROVIDERS * TREND_MAX_BUCKETS;

#[derive(Debug, Clone, Copy)]
pub(super) struct TrendPlan {
    pub granularity: UsageTrendGranularityV1,
    pub provider_limit: usize,
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

fn normalized_provider_limit(provider_id: Option<i64>, requested: Option<usize>) -> usize {
    if provider_id.is_some() {
        return 1;
    }
    requested
        .unwrap_or(TREND_MAX_PROVIDERS)
        .clamp(1, TREND_MAX_PROVIDERS)
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

    let (where_clause, where_params) = build_optional_range_cli_provider_filters(
        "r.created_at",
        "r.cli_key",
        "r.final_provider_id",
        query.start_ts,
        query.end_ts,
        query.cli_key,
        query.provider_id,
    );
    let cx2cc_filter_clause =
        sql_exclude_cx2cc_gateway_bridge_clause(Some("r"), query.exclude_cx2cc_gateway_bridge);
    let sql = format!(
        r#"
SELECT MIN(r.created_at), MAX(r.created_at)
FROM usage_events r
WHERE r.excluded_from_stats = 0
AND r.final_provider_id IS NOT NULL
AND r.final_provider_id > 0
{where_clause}
{cx2cc_filter_clause}
"#
    );
    let (min_ts, max_ts): (Option<i64>, Option<i64>) = conn
        .query_row(&sql, params_from_iter(where_params), |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .map_err(|error| db_err!("failed to resolve provider trend extent: {error}"))?;

    match (min_ts, max_ts, query.end_ts) {
        (Some(min_ts), Some(_), Some(end_ts)) => Ok(Some((min_ts, end_ts))),
        (Some(min_ts), Some(max_ts), None) => {
            Ok(Some((min_ts, max_ts.saturating_add(1))))
        }
        _ => Ok(None),
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

fn finest_bounded_granularity(
    counts: BucketCounts,
) -> Result<UsageTrendGranularityV1, String> {
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
    let granularity = match filtered_extent(conn, query)? {
        Some((start_ts, end_ts)) => {
            finest_bounded_granularity(bucket_counts_for_extent(conn, start_ts, end_ts)?)?
        }
        None => UsageTrendGranularityV1::Hour,
    };
    Ok(TrendPlan {
        granularity,
        provider_limit,
    })
}

pub(super) fn bucket_select_and_group(
    granularity: UsageTrendGranularityV1,
) -> (&'static str, &'static str) {
    match granularity {
        UsageTrendGranularityV1::Hour => (
            "strftime('%Y-%m-%d', r.created_at, 'unixepoch', 'localtime') AS day, CAST(strftime('%H', r.created_at, 'unixepoch', 'localtime') AS INTEGER) AS hour",
            "day, hour",
        ),
        UsageTrendGranularityV1::Day => (
            "strftime('%Y-%m-%d', r.created_at, 'unixepoch', 'localtime') AS day, NULL AS hour",
            "day",
        ),
        UsageTrendGranularityV1::Week => (
            "strftime('%Y-%m-%d', r.created_at, 'unixepoch', 'localtime', '-' || ((CAST(strftime('%w', r.created_at, 'unixepoch', 'localtime') AS INTEGER) + 6) % 7) || ' days') AS day, NULL AS hour",
            "day",
        ),
        UsageTrendGranularityV1::Month => (
            "strftime('%Y-%m', r.created_at, 'unixepoch', 'localtime') AS day, NULL AS hour",
            "day",
        ),
        UsageTrendGranularityV1::Year => (
            "strftime('%Y', r.created_at, 'unixepoch', 'localtime') AS day, NULL AS hour",
            "day",
        ),
    }
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
