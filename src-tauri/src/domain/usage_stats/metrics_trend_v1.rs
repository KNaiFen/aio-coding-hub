use crate::db;
use crate::shared::error::db_err;
use rusqlite::{params_from_iter, Connection};
use std::collections::HashSet;

use super::filters::{
    build_optional_range_cli_provider_filters, sql_exclude_cx2cc_gateway_bridge_clause,
};
use super::trend_common::{
    bucket_select_and_group, plan_trend, validate_trend_budget, TrendPlanQuery,
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

const SUCCESS: &str = "r.status >= 200 AND r.status < 300 AND r.error_present = 0";
const VALID_TTFB: &str = "r.ttfb_ms IS NOT NULL AND r.ttfb_ms < r.duration_ms";
const VALID_OUTPUT_RATE: &str =
    "r.output_tokens IS NOT NULL AND r.ttfb_ms IS NOT NULL AND r.ttfb_ms < r.duration_ms";

pub(super) fn provider_metric_trend_v1_with_conn(
    conn: &Connection,
    query: ProviderMetricTrendQuery<'_>,
) -> Result<Vec<UsageProviderMetricTrendRowV1>, String> {
    let plan = plan_trend(
        conn,
        TrendPlanQuery {
            start_ts: query.start_ts,
            end_ts: query.end_ts,
            cli_key: query.cli_key,
            provider_id: query.provider_id,
            requested_provider_limit: query.limit,
            exclude_cx2cc_gateway_bridge: query.exclude_cx2cc_gateway_bridge,
        },
    )?;
    let (select_fields, group_by_fields) = bucket_select_and_group(plan.granularity);
    let order_by_fields = match plan.granularity {
        UsageTrendGranularityV1::Hour => "b.day ASC, b.hour ASC",
        UsageTrendGranularityV1::Day
        | UsageTrendGranularityV1::Week
        | UsageTrendGranularityV1::Month
        | UsageTrendGranularityV1::Year => "b.day ASC",
    };
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
    let top_provider_success_clause = if query.provider_id.is_some() {
        ""
    } else {
        "AND r.status >= 200 AND r.status < 300 AND r.error_present = 0"
    };
    let sql = format!(
        r#"
WITH top_providers AS (
  SELECT
    r.cli_key AS cli_key,
    r.final_provider_id AS provider_id,
    COUNT(*) AS requests_success
  FROM usage_events r
  WHERE r.excluded_from_stats = 0
  {top_provider_success_clause}
  AND r.final_provider_id IS NOT NULL
  AND r.final_provider_id > 0
  {where_clause}
  {cx2cc_filter_clause}
  GROUP BY r.cli_key, r.final_provider_id
  ORDER BY requests_success DESC, r.cli_key ASC, r.final_provider_id ASC
  LIMIT ?{limit_bind_idx}
),
bucketed AS (
  SELECT
    {select_fields},
    r.cli_key AS cli_key,
    r.final_provider_id AS provider_id,
    MAX(NULLIF(TRIM(r.provider_name_snapshot), '')) AS provider_name,
    COUNT(*) AS requests_total,
    SUM(CASE WHEN {success} THEN 1 ELSE 0 END) AS requests_success,
    SUM(CASE WHEN {success} THEN r.duration_ms ELSE 0 END) AS success_duration_ms_sum,
    SUM(CASE WHEN {success} AND {valid_ttfb} THEN r.ttfb_ms ELSE 0 END) AS success_ttfb_ms_sum,
    SUM(CASE WHEN {success} AND {valid_ttfb} THEN 1 ELSE 0 END) AS success_ttfb_ms_count,
    SUM(CASE WHEN {success} AND {valid_output_rate} THEN r.duration_ms - r.ttfb_ms ELSE 0 END) AS success_generation_ms_sum,
    SUM(CASE WHEN {success} AND {valid_output_rate} THEN r.output_tokens ELSE 0 END) AS success_output_tokens_for_rate_sum,
    SUM(CASE WHEN {success} AND {valid_output_rate} THEN 1 ELSE 0 END) AS success_output_rate_count
  FROM usage_events r
  JOIN top_providers tp
    ON tp.cli_key = r.cli_key
   AND tp.provider_id = r.final_provider_id
  WHERE r.excluded_from_stats = 0
  AND r.final_provider_id IS NOT NULL
  AND r.final_provider_id > 0
  {where_clause}
  {cx2cc_filter_clause}
  GROUP BY {group_by_fields}, r.cli_key, r.final_provider_id
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
  b.success_generation_ms_sum,
  b.success_output_tokens_for_rate_sum,
  b.success_output_rate_count
FROM bucketed b
ORDER BY {order_by_fields}, b.requests_success DESC, b.cli_key ASC, b.provider_id ASC
"#,
        success = SUCCESS,
        valid_ttfb = VALID_TTFB,
        valid_output_rate = VALID_OUTPUT_RATE,
        limit_bind_idx = where_params.len() + 1,
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
        success_generation_ms_sum: i64,
        success_output_tokens_for_rate_sum: i64,
        success_output_rate_count: i64,
    }

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|error| db_err!("failed to prepare provider metric trend query: {error}"))?;
    let rows = stmt
        .query_map(
            params_from_iter({
                let mut params = where_params;
                params.push((plan.provider_limit as i64).into());
                params
            }),
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
                    success_generation_ms_sum: row
                        .get::<_, Option<i64>>("success_generation_ms_sum")?
                        .unwrap_or(0),
                    success_output_tokens_for_rate_sum: row
                        .get::<_, Option<i64>>("success_output_tokens_for_rate_sum")?
                        .unwrap_or(0),
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
        let avg_output_tokens_per_second = if row.success_generation_ms_sum > 0 {
            Some(
                row.success_output_tokens_for_rate_sum as f64
                    / (row.success_generation_ms_sum as f64 / 1000.0),
            )
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
