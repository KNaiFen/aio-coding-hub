use crate::db;
use crate::shared::error::db_err;
use rusqlite::{params_from_iter, Connection};
use std::collections::HashSet;

use super::trend_common::{
    build_trend_source_ctes, plan_trend, trend_query_params, validate_trend_budget,
    TrendPlanQuery,
};
use super::{
    has_valid_provider_key, resolve_query_params, ProviderKey,
    UsageProviderCacheRateTrendRowV1, UsageQueryParams, UsageTrendGranularityV1,
};

#[derive(Debug, Clone, Copy)]
pub(super) struct ProviderCacheRateTrendQuery<'a> {
    pub start_ts: Option<i64>,
    pub end_ts: Option<i64>,
    pub cli_key: Option<&'a str>,
    pub provider_id: Option<i64>,
    pub limit: Option<usize>,
    pub exclude_cx2cc_gateway_bridge: bool,
}

pub(super) fn provider_cache_rate_trend_v1_with_conn(
    conn: &Connection,
    query: ProviderCacheRateTrendQuery<'_>,
) -> Result<Vec<UsageProviderCacheRateTrendRowV1>, String> {
    let tx = conn.unchecked_transaction().map_err(|error| {
        db_err!("failed to start Provider cache trend read snapshot: {error}")
    })?;
    let rows = provider_cache_rate_trend_v1_in_snapshot(&tx, query)?;
    tx.commit().map_err(|error| {
        db_err!("failed to close Provider cache trend read snapshot: {error}")
    })?;
    Ok(rows)
}

fn provider_cache_rate_trend_v1_in_snapshot(
    conn: &Connection,
    query: ProviderCacheRateTrendQuery<'_>,
) -> Result<Vec<UsageProviderCacheRateTrendRowV1>, String> {
    let source_query = TrendPlanQuery {
        start_ts: query.start_ts,
        end_ts: query.end_ts,
        cli_key: query.cli_key,
        provider_id: query.provider_id,
        requested_provider_limit: query.limit,
        exclude_cx2cc_gateway_bridge: query.exclude_cx2cc_gateway_bridge,
    };
    let plan = plan_trend(conn, source_query)?;
    let source_ctes = build_trend_source_ctes(conn, plan, source_query)?;
    let order_by_fields = match plan.granularity {
        UsageTrendGranularityV1::Hour => "b.day ASC, b.hour ASC",
        UsageTrendGranularityV1::Day
        | UsageTrendGranularityV1::Week
        | UsageTrendGranularityV1::Month
        | UsageTrendGranularityV1::Year => "b.day ASC",
    };
    let top_provider_having = if query.provider_id.is_some() {
        ""
    } else {
        "HAVING SUM(s.requests_success) > 0"
    };
    let sql = format!(
        r#"
WITH {source_ctes},
top_providers AS (
  SELECT
    s.cli_key,
    s.provider_id,
    SUM(s.requests_success) AS requests_success
  FROM trend_source s
  GROUP BY s.cli_key, s.provider_id
  {top_provider_having}
  ORDER BY requests_success DESC, s.cli_key ASC, s.provider_id ASC
  LIMIT ?5
),
bucketed AS (
  SELECT
    s.day,
    s.hour,
    s.cli_key,
    s.provider_id,
    s.provider_name_success AS provider_name,
    s.cache_denom_tokens AS denom_tokens,
    s.cache_read_input_tokens,
    s.requests_success
  FROM trend_source s
  JOIN top_providers tp
    ON tp.cli_key = s.cli_key
   AND tp.provider_id = s.provider_id
  WHERE s.requests_success > 0
)
SELECT
  b.day,
  b.hour,
  b.cli_key,
  b.provider_id,
  b.provider_name,
  b.denom_tokens,
  b.cache_read_input_tokens,
  b.requests_success
FROM bucketed b
ORDER BY {order_by_fields}, b.requests_success DESC, b.cli_key ASC, b.provider_id ASC
"#
    );

    #[derive(Debug)]
    struct RawRow {
        day: String,
        hour: Option<i64>,
        cli_key: String,
        provider_id: i64,
        provider_name: Option<String>,
        denom_tokens: i64,
        cache_read_input_tokens: i64,
        requests_success: i64,
    }

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|error| db_err!("failed to prepare provider cache trend query: {error}"))?;
    let rows = stmt
        .query_map(
            params_from_iter(trend_query_params(plan, source_query)),
            |row| {
                Ok(RawRow {
                    day: row.get("day")?,
                    hour: row.get("hour")?,
                    cli_key: row.get("cli_key")?,
                    provider_id: row.get("provider_id")?,
                    provider_name: row.get("provider_name")?,
                    denom_tokens: row
                        .get::<_, Option<i64>>("denom_tokens")?
                        .unwrap_or(0)
                        .max(0),
                    cache_read_input_tokens: row
                        .get::<_, Option<i64>>("cache_read_input_tokens")?
                        .unwrap_or(0)
                        .max(0),
                    requests_success: row
                        .get::<_, Option<i64>>("requests_success")?
                        .unwrap_or(0)
                        .max(0),
                })
            },
        )
        .map_err(|error| db_err!("failed to run provider cache trend query: {error}"))?;
    let items = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| db_err!("failed to read provider cache trend row: {error}"))?;
    let bucket_count = items
        .iter()
        .map(|row| (row.day.as_str(), row.hour))
        .collect::<HashSet<_>>()
        .len();
    let provider_count = items
        .iter()
        .map(|row| (row.cli_key.as_str(), row.provider_id))
        .collect::<HashSet<_>>()
        .len();
    validate_trend_budget(
        items.len(),
        bucket_count,
        provider_count,
        plan.provider_limit,
    )?;

    let mut out = Vec::with_capacity(items.len());
    for row in items {
        let Some(provider_name) = row
            .provider_name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
        else {
            continue;
        };
        let provider_key = ProviderKey {
            cli_key: row.cli_key.clone(),
            provider_id: row.provider_id,
            provider_name: provider_name.clone(),
        };
        if !has_valid_provider_key(&provider_key) {
            continue;
        }

        out.push(UsageProviderCacheRateTrendRowV1 {
            day: row.day,
            hour: row.hour,
            granularity: plan.granularity,
            key: format!("{}:{}", row.cli_key, row.provider_id),
            name: format!("{}/{}", row.cli_key, provider_name),
            denom_tokens: row.denom_tokens,
            cache_read_input_tokens: row.cache_read_input_tokens,
            requests_success: row.requests_success,
        });
    }
    Ok(out)
}

pub fn provider_cache_rate_trend_v1(
    db: &db::Db,
    params: &UsageQueryParams,
    limit: Option<usize>,
) -> crate::shared::error::AppResult<Vec<UsageProviderCacheRateTrendRowV1>> {
    let conn = db.open_connection()?;
    let mut params = params.clone();
    params.day_start_hour = None;
    let resolved = resolve_query_params(&conn, &params)?;
    Ok(provider_cache_rate_trend_v1_with_conn(
        &conn,
        ProviderCacheRateTrendQuery {
            start_ts: resolved.start_ts,
            end_ts: resolved.end_ts,
            cli_key: resolved.cli_key,
            provider_id: resolved.provider_id,
            limit,
            exclude_cx2cc_gateway_bridge: resolved.exclude_cx2cc_gateway_bridge,
        },
    )?)
}
