use crate::db;
use crate::shared::error::db_err;
use rusqlite::{params_from_iter, Connection};
use std::collections::HashSet;

use super::trend_common::{
    build_trend_source_ctes, plan_trend, trend_query_params, validate_trend_budget, TrendPlanQuery,
};
use super::{
    has_valid_provider_key, resolve_query_params, ProviderKey, UsageProviderMetricTrendRowV1,
    UsageQueryParams, UsageTrendGranularityV1,
};

#[derive(Debug, Clone, Copy)]
pub(super) struct ProviderMetricTrendQuery<'a> {
    pub start_ts: Option<i64>,
    pub end_ts: Option<i64>,
    pub cli_key: Option<&'a str>,
    pub provider_id: Option<i64>,
    pub limit: Option<usize>,
    pub exclude_cx2cc_gateway_bridge: bool,
}

pub(super) fn provider_metric_trend_v1_with_conn(
    conn: &Connection,
    query: ProviderMetricTrendQuery<'_>,
) -> Result<Vec<UsageProviderMetricTrendRowV1>, String> {
    let tx = conn
        .unchecked_transaction()
        .map_err(|error| db_err!("failed to start Provider metric trend read snapshot: {error}"))?;
    let rows = provider_metric_trend_v1_in_snapshot(&tx, query)?;
    tx.commit()
        .map_err(|error| db_err!("failed to close Provider metric trend read snapshot: {error}"))?;
    Ok(rows)
}

fn provider_metric_trend_v1_in_snapshot(
    conn: &Connection,
    query: ProviderMetricTrendQuery<'_>,
) -> Result<Vec<UsageProviderMetricTrendRowV1>, String> {
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
    s.provider_name_all AS provider_name,
    s.requests_total,
    s.requests_success,
    s.success_duration_ms_sum,
    s.success_ttfb_ms_sum,
    s.success_ttfb_ms_count,
    s.success_output_tokens_per_second_sum,
    s.success_output_rate_count
  FROM trend_source s
  JOIN top_providers tp
    ON tp.cli_key = s.cli_key
   AND tp.provider_id = s.provider_id
)
SELECT
  b.day,
  b.hour,
  b.cli_key,
  b.provider_id,
  b.provider_name,
  b.requests_total,
  b.requests_success,
  b.success_duration_ms_sum,
  b.success_ttfb_ms_sum,
  b.success_ttfb_ms_count,
  b.success_output_tokens_per_second_sum,
  b.success_output_rate_count
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
        requests_total: i64,
        requests_success: i64,
        success_duration_ms_sum: i64,
        success_ttfb_ms_sum: i64,
        success_ttfb_ms_count: i64,
        success_output_tokens_per_second_sum: f64,
        success_output_rate_count: i64,
    }

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|error| db_err!("failed to prepare provider metric trend query: {error}"))?;
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
                    requests_total: row.get("requests_total")?,
                    requests_success: row.get::<_, Option<i64>>("requests_success")?.unwrap_or(0),
                    success_duration_ms_sum: row
                        .get::<_, Option<i64>>("success_duration_ms_sum")?
                        .unwrap_or(0),
                    success_ttfb_ms_sum: row
                        .get::<_, Option<i64>>("success_ttfb_ms_sum")?
                        .unwrap_or(0),
                    success_ttfb_ms_count: row
                        .get::<_, Option<i64>>("success_ttfb_ms_count")?
                        .unwrap_or(0),
                    success_output_tokens_per_second_sum: row
                        .get::<_, Option<f64>>("success_output_tokens_per_second_sum")?
                        .unwrap_or(0.0),
                    success_output_rate_count: row
                        .get::<_, Option<i64>>("success_output_rate_count")?
                        .unwrap_or(0),
                })
            },
        )
        .map_err(|error| db_err!("failed to run provider metric trend query: {error}"))?;

    let items = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| db_err!("failed to read provider metric trend row: {error}"))?;
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

        let avg_duration_ms = if row.requests_success > 0 {
            Some(row.success_duration_ms_sum / row.requests_success)
        } else {
            None
        };
        let avg_ttfb_ms = if row.success_ttfb_ms_count > 0 {
            Some(row.success_ttfb_ms_sum / row.success_ttfb_ms_count)
        } else {
            None
        };
        let avg_output_tokens_per_second = if row.success_output_rate_count > 0 {
            Some(row.success_output_tokens_per_second_sum / row.success_output_rate_count as f64)
        } else {
            None
        };
        out.push(UsageProviderMetricTrendRowV1 {
            day: row.day,
            hour: row.hour,
            granularity: plan.granularity,
            key: format!("{}:{}", row.cli_key, row.provider_id),
            name: format!("{}/{}", row.cli_key, provider_name),
            cli_key: row.cli_key,
            provider_id: row.provider_id,
            provider_name,
            requests_total: row.requests_total,
            requests_success: row.requests_success,
            duration_samples: row.requests_success,
            ttfb_samples: row.success_ttfb_ms_count,
            output_rate_samples: row.success_output_rate_count,
            avg_duration_ms,
            avg_ttfb_ms,
            avg_output_tokens_per_second,
        });
    }
    Ok(out)
}

pub fn provider_metric_trend_v1(
    db: &db::Db,
    params: &UsageQueryParams,
    limit: Option<usize>,
) -> crate::shared::error::AppResult<Vec<UsageProviderMetricTrendRowV1>> {
    let conn = db.open_connection()?;
    let mut params = params.clone();
    params.day_start_hour = None;
    let resolved = resolve_query_params(&conn, &params)?;
    Ok(provider_metric_trend_v1_with_conn(
        &conn,
        ProviderMetricTrendQuery {
            start_ts: resolved.start_ts,
            end_ts: resolved.end_ts,
            cli_key: resolved.cli_key,
            provider_id: resolved.provider_id,
            limit,
            exclude_cx2cc_gateway_bridge: resolved.exclude_cx2cc_gateway_bridge,
        },
    )?)
}
