use super::cache_rate_trend_v1::{
    provider_cache_rate_trend_v1_with_conn, ProviderCacheRateTrendQuery,
};
use super::day_detail::{day_detail_v1_with_conn, UsageDayResolvedFolder};
use super::folder_options::folder_options_v1_with_conn;
use super::leaderboard_v2::{
    leaderboard_v2_folder_filtered_with_conn, leaderboard_v2_with_conn,
    leaderboard_v2_with_conn_day_start, FolderFilteredLeaderboardParams,
};
use super::metrics_trend_v1::{provider_metric_trend_v1_with_conn, ProviderMetricTrendQuery};
use super::summary::{summary_query, summary_v2_with_conn};
use super::trend_common::{
    plan_trend, TrendPlanQuery, TREND_MAX_BUCKETS, TREND_MAX_PROVIDERS, TREND_MAX_ROWS,
};
use super::*;
use crate::db;
use rusqlite::{params, Connection};
use tempfile::tempdir;

fn setup_conn() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.execute_batch(
        r#"
	CREATE TABLE providers (
	  id INTEGER PRIMARY KEY,
	  name TEXT NOT NULL,
	  source_provider_id INTEGER,
	  bridge_type TEXT
	);

	CREATE TABLE usage_events (
	  cli_key TEXT NOT NULL,
	  attempts_json TEXT NOT NULL,
	  final_provider_id INTEGER,
	  provider_name_snapshot TEXT,
	  requested_model TEXT,
	  status INTEGER,
	  error_code TEXT,
	  error_present INTEGER,
		  duration_ms INTEGER NOT NULL,
		  ttfb_ms INTEGER,
		  visible_ttfb_ms INTEGER,
		  final_upstream_attempt_duration_ms INTEGER,
		  final_upstream_attempt_timing_version INTEGER NOT NULL DEFAULT 0,
	  input_tokens INTEGER,
	  output_tokens INTEGER,
	  total_tokens INTEGER,
	  cache_read_input_tokens INTEGER,
	  cache_creation_input_tokens INTEGER,
	  cache_creation_5m_input_tokens INTEGER,
	  cache_creation_1h_input_tokens INTEGER,
	  special_settings_json TEXT,
	  cost_usd_femto INTEGER,
	  usage_json TEXT,
	  usage_present INTEGER,
	  persisted_openai_input_semantics INTEGER,
	  excluded_from_stats INTEGER NOT NULL DEFAULT 0,
	  session_id TEXT,
	  created_at INTEGER NOT NULL,
	  created_at_ms INTEGER NOT NULL DEFAULT 0
	);

	CREATE TRIGGER normalize_usage_event_fixture
	AFTER INSERT ON usage_events
	BEGIN
	  UPDATE usage_events
	  SET
	    provider_name_snapshot = COALESCE(
	      NULLIF(TRIM(NEW.provider_name_snapshot), ''),
	      (
	        SELECT NULLIF(TRIM(json_extract(attempt.value, '$.provider_name')), '')
	        FROM json_each(NEW.attempts_json) attempt
	        WHERE attempt.type = 'object'
	        AND json_extract(attempt.value, '$.outcome') = 'success'
	        ORDER BY CAST(attempt.key AS INTEGER) DESC
	        LIMIT 1
	      ),
	      (
	        SELECT NULLIF(TRIM(json_extract(attempt.value, '$.provider_name')), '')
	        FROM json_each(NEW.attempts_json) attempt
	        WHERE attempt.type = 'object'
	        AND json_extract(attempt.value, '$.outcome') != 'skipped'
	        ORDER BY CAST(attempt.key AS INTEGER) DESC
	        LIMIT 1
	      ),
	      (
	        SELECT NULLIF(TRIM(p.name), '')
	        FROM providers p
	        WHERE p.id = NEW.final_provider_id
	      )
	    ),
	    error_present = COALESCE(
	      NEW.error_present,
	      CASE WHEN NEW.error_code IS NULL THEN 0 ELSE 1 END
	    ),
	    usage_present = COALESCE(
	      NEW.usage_present,
	      CASE WHEN (
	        NEW.usage_json IS NOT NULL OR
	        NEW.input_tokens IS NOT NULL OR
	        NEW.output_tokens IS NOT NULL OR
	        NEW.total_tokens IS NOT NULL OR
	        NEW.cache_read_input_tokens IS NOT NULL OR
	        NEW.cache_creation_input_tokens IS NOT NULL OR
	        NEW.cache_creation_5m_input_tokens IS NOT NULL OR
	        NEW.cache_creation_1h_input_tokens IS NOT NULL
	      ) THEN 1 ELSE 0 END
	    ),
	    persisted_openai_input_semantics = COALESCE(
	      NEW.persisted_openai_input_semantics,
	      CASE
	        WHEN NEW.cli_key IN ('codex', 'grok') THEN 1
	        WHEN EXISTS (
	          SELECT 1
	          FROM providers p
	          WHERE p.id = NEW.final_provider_id
	          AND (p.source_provider_id IS NOT NULL OR p.bridge_type = 'cx2cc')
	        ) THEN 1
	        ELSE 0
	      END
	    )
	  WHERE rowid = NEW.rowid;
	END;
	"#,
    )
    .expect("create schema");
    conn
}

fn setup_temp_db() -> (tempfile::TempDir, db::Db) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("usage-stats-test.db");
    let db = db::init_for_tests(&path).expect("init test db");
    let conn = db.open_connection().expect("open migrated test db");
    conn.execute(
        r#"
UPDATE usage_ledger_backfill_state
SET status = 'incomplete', completed_at = NULL
WHERE id = 1
        "#,
        [],
    )
    .expect("switch usage events fixture to request-log compatibility mode");
    drop(conn);
    (dir, db)
}

fn local_day_key(conn: &Connection, ts: i64) -> String {
    conn.query_row(
        "SELECT strftime('%Y-%m-%d', ?1, 'unixepoch', 'localtime')",
        params![ts],
        |row| row.get(0),
    )
    .expect("query local day key")
}

fn local_day_start_ts(conn: &Connection, day: &str) -> i64 {
    conn.query_row(
        "SELECT CAST(strftime('%s', ?1 || ' 00:00:00', 'utc') AS INTEGER)",
        params![day],
        |row| row.get(0),
    )
    .expect("query local day start ts")
}

fn local_usage_day_start_ts(conn: &Connection, day: &str, day_start_hour: i64) -> i64 {
    let time = format!("{day_start_hour:02}:00:00");
    conn.query_row(
        "SELECT CAST(strftime('%s', ?1 || ' ' || ?2, 'utc') AS INTEGER)",
        params![day, time],
        |row| row.get(0),
    )
    .expect("query local usage day start ts")
}

#[derive(Clone)]
struct TestUsageLog<'a> {
    cli_key: &'a str,
    provider_id: i64,
    provider_name: &'a str,
    requested_model: &'a str,
    status: Option<i64>,
    error_code: Option<&'a str>,
    duration_ms: i64,
    ttfb_ms: Option<i64>,
    final_upstream_attempt_duration_ms: Option<i64>,
    final_upstream_attempt_timing_version: i64,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    total_tokens: Option<i64>,
    cache_read_input_tokens: Option<i64>,
    cache_creation_input_tokens: Option<i64>,
    cost_usd_femto: Option<i64>,
    session_id: Option<&'a str>,
    excluded_from_stats: i64,
    created_at: i64,
}

fn base_usage_log(created_at: i64) -> TestUsageLog<'static> {
    TestUsageLog {
        cli_key: "codex",
        provider_id: 123,
        provider_name: "OpenAI",
        requested_model: "model-test",
        status: Some(200),
        error_code: None,
        duration_ms: 1000,
        ttfb_ms: Some(100),
        final_upstream_attempt_duration_ms: Some(900),
        final_upstream_attempt_timing_version: 1,
        input_tokens: Some(100),
        output_tokens: Some(20),
        total_tokens: None,
        cache_read_input_tokens: Some(0),
        cache_creation_input_tokens: Some(0),
        cost_usd_femto: None,
        session_id: None,
        excluded_from_stats: 0,
        created_at,
    }
}

fn insert_usage_log(conn: &Connection, log: TestUsageLog<'_>) {
    let attempts_json = format!(
        r#"[{{"provider_id":{},"provider_name":"{}","outcome":"success"}}]"#,
        log.provider_id, log.provider_name
    );
    conn.execute(
        r#"
INSERT INTO usage_events (
  cli_key,
  attempts_json,
  final_provider_id,
  requested_model,
  status,
  error_code,
  duration_ms,
  ttfb_ms,
  visible_ttfb_ms,
  final_upstream_attempt_duration_ms,
  final_upstream_attempt_timing_version,
  input_tokens,
  output_tokens,
  total_tokens,
  cache_read_input_tokens,
  cache_creation_input_tokens,
  cache_creation_5m_input_tokens,
  cache_creation_1h_input_tokens,
  cost_usd_femto,
  usage_json,
  excluded_from_stats,
  session_id,
  created_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23);
        "#,
        params![
            log.cli_key,
            attempts_json,
            log.provider_id,
            log.requested_model,
            log.status,
            log.error_code,
            log.duration_ms,
            log.ttfb_ms,
            log.ttfb_ms,
            log.final_upstream_attempt_duration_ms,
            log.final_upstream_attempt_timing_version,
            log.input_tokens,
            log.output_tokens,
            log.total_tokens,
            log.cache_read_input_tokens,
            log.cache_creation_input_tokens,
            0i64,
            0i64,
            log.cost_usd_femto,
            Option::<String>::None,
            log.excluded_from_stats,
            log.session_id,
            log.created_at
        ],
    )
    .expect("insert usage log");
}

#[test]
fn lifecycle_interruption_rows_are_excluded_from_usage_summary_and_leaderboard() {
    let conn = setup_conn();
    insert_usage_log(
        &conn,
        TestUsageLog {
            provider_id: 1,
            provider_name: "Included Provider",
            duration_ms: 1000,
            input_tokens: Some(80),
            output_tokens: Some(20),
            total_tokens: Some(100),
            cost_usd_femto: Some(1_000_000_000_000_000),
            ..base_usage_log(1_000)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            provider_id: 1,
            provider_name: "Included Provider",
            status: Some(500),
            error_code: Some("UPSTREAM_ERROR"),
            duration_ms: 2500,
            input_tokens: None,
            output_tokens: None,
            total_tokens: None,
            cost_usd_femto: None,
            ..base_usage_log(1_001)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            provider_id: 2,
            provider_name: "Interrupted Provider",
            status: Some(499),
            error_code: Some("GW_REQUEST_INTERRUPTED_BY_RESTART"),
            duration_ms: 99_000,
            input_tokens: Some(8_000),
            output_tokens: Some(2_000),
            total_tokens: Some(10_000),
            cost_usd_femto: Some(99_000_000_000_000_000),
            excluded_from_stats: 1,
            ..base_usage_log(1_002)
        },
    );

    let summary = summary_query(&conn, None, None, None, None, false).expect("summary");
    assert_eq!(summary.requests_total, 2);
    assert_eq!(summary.requests_failed, 1);
    assert_eq!(summary.total_duration_ms, 3500);
    assert_eq!(summary.total_tokens, 100);

    let rows = leaderboard_v2_with_conn(
        &conn,
        UsageScopeV2::Provider,
        None,
        None,
        None,
        None,
        Some(50),
        false,
    )
    .expect("leaderboard");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].key, "codex:1");
    assert_eq!(rows[0].requests_total, 2);
    assert_eq!(rows[0].requests_failed, 1);
    assert_eq!(rows[0].total_duration_ms, 3500);
    assert_eq!(rows[0].total_tokens, 100);
}

fn insert_migrated_provider(
    conn: &Connection,
    id: i64,
    cli_key: &str,
    name: &str,
    source_provider_id: Option<i64>,
    bridge_type: Option<&str>,
) {
    conn.execute(
        r#"
INSERT INTO providers (
  id,
  provider_uuid,
  cli_key,
  name,
  base_url,
  base_urls_json,
  base_url_mode,
  claude_models_json,
  api_key_plaintext,
  enabled,
  priority,
  created_at,
  updated_at,
  sort_order,
  cost_multiplier,
  supported_models_json,
  model_mapping_json,
  source_provider_id,
  bridge_type
) VALUES (
  ?1,
  ?2,
  ?3,
  ?4,
  'https://example.invalid',
  '[]',
  'order',
  '{}',
  'test-key',
  1,
  100,
  1000,
  1000,
  0,
  1.0,
  '{}',
  '{}',
  ?5,
  ?6
);
        "#,
        params![
            id,
            format!("00000000-0000-4000-8000-{id:012x}"),
            cli_key,
            name,
            source_provider_id,
            bridge_type
        ],
    )
    .expect("insert migrated provider");
}

#[allow(clippy::too_many_arguments)]
fn insert_migrated_usage_log(
    conn: &Connection,
    trace_id: &str,
    cli_key: &str,
    provider_id: i64,
    provider_name: &str,
    input_tokens: i64,
    output_tokens: i64,
    created_at: i64,
    session_id: Option<&str>,
) {
    let attempts_json = format!(
        r#"[{{"provider_id":{provider_id},"provider_name":"{provider_name}","outcome":"success"}}]"#
    );
    conn.execute(
        r#"
INSERT INTO request_logs (
  trace_id,
  cli_key,
  method,
  path,
  status,
  error_code,
  duration_ms,
  attempts_json,
  created_at,
  input_tokens,
  output_tokens,
  total_tokens,
  cache_read_input_tokens,
  cache_creation_input_tokens,
  cache_creation_5m_input_tokens,
  cache_creation_1h_input_tokens,
  usage_json,
  ttfb_ms,
  requested_model,
  cost_usd_femto,
  excluded_from_stats,
  session_id,
  final_provider_id
) VALUES (
  ?1,
  ?2,
  'POST',
  '/v1/messages',
  200,
  NULL,
  1000,
  ?3,
  ?4,
  ?5,
  ?6,
  NULL,
  0,
  0,
  0,
  0,
  NULL,
  100,
  'model-test',
  NULL,
  0,
  ?7,
  ?8
);
        "#,
        params![
            trace_id,
            cli_key,
            attempts_json,
            created_at,
            input_tokens,
            output_tokens,
            session_id,
            provider_id
        ],
    )
    .expect("insert migrated usage log");
}

#[test]
fn completed_usage_ledger_keeps_summary_and_provider_stats_after_log_deletion() {
    let (_dir, db) = setup_temp_db();
    let conn = db.open_connection().expect("open test db connection");
    insert_migrated_provider(&conn, 410, "codex", "Ledger Provider", None, None);
    insert_migrated_usage_log(
        &conn,
        "trace-ledger-stats",
        "codex",
        410,
        "Ledger Provider",
        100,
        20,
        1_000,
        Some("ledger-session"),
    );

    assert_eq!(
        crate::usage_ledger::project_trace(&conn, "trace-ledger-stats")
            .expect("project request into usage ledger"),
        1
    );
    conn.execute(
        r#"
UPDATE usage_ledger_backfill_state
SET status = 'complete', completed_at = 1000
WHERE id = 1
        "#,
        [],
    )
    .expect("complete usage ledger backfill");
    conn.execute(
        "DELETE FROM request_logs WHERE trace_id = ?1",
        ["trace-ledger-stats"],
    )
    .expect("delete request log detail");

    let summary = summary_query(&conn, None, None, None, None, false).expect("ledger summary");
    assert_eq!(summary.requests_total, 1);
    assert_eq!(summary.requests_with_usage, 1);
    assert_eq!(summary.total_tokens, 120);

    let rows = leaderboard_v2_with_conn(
        &conn,
        UsageScopeV2::Provider,
        None,
        None,
        None,
        None,
        Some(50),
        false,
    )
    .expect("ledger provider leaderboard");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].key, "codex:410");
    assert_eq!(rows[0].name, "codex/Ledger Provider");
    assert_eq!(rows[0].total_tokens, 120);
}

#[test]
fn provider_trends_are_invariant_after_detail_and_provider_deletion() {
    let (_dir, db) = setup_temp_db();
    let conn = db.open_connection().expect("open test db connection");
    let start_ts = 1_704_067_200i64;
    insert_migrated_provider(&conn, 411, "codex", "Ledger Trend Provider", None, None);
    for (trace_id, created_at, input_tokens, output_tokens) in [
        ("trace-ledger-trend-1", start_ts + 3600, 100, 20),
        ("trace-ledger-trend-2", start_ts + 7200, 200, 40),
    ] {
        insert_migrated_usage_log(
            &conn,
            trace_id,
            "codex",
            411,
            "Ledger Trend Provider",
            input_tokens,
            output_tokens,
            created_at,
            Some("ledger-trend-session"),
        );
        assert_eq!(
            crate::usage_ledger::project_trace(&conn, trace_id)
                .expect("project request into usage ledger"),
            1
        );
    }
    conn.execute(
        r#"
UPDATE usage_ledger_backfill_state
SET status = 'complete', completed_at = ?1
WHERE id = 1
        "#,
        params![start_ts],
    )
    .expect("complete usage ledger backfill");

    let metric_query = ProviderMetricTrendQuery {
        start_ts: Some(start_ts),
        end_ts: Some(start_ts + 86_400),
        cli_key: None,
        provider_id: Some(411),
        limit: None,
        exclude_cx2cc_gateway_bridge: false,
    };
    let cache_query = ProviderCacheRateTrendQuery {
        start_ts: Some(start_ts),
        end_ts: Some(start_ts + 86_400),
        cli_key: None,
        provider_id: Some(411),
        limit: None,
        exclude_cx2cc_gateway_bridge: false,
    };
    let metric_before = provider_metric_trend_v1_with_conn(&conn, metric_query)
        .expect("metric trend before detail deletion");
    let cache_before = provider_cache_rate_trend_v1_with_conn(&conn, cache_query)
        .expect("cache trend before detail deletion");
    assert_eq!(metric_before.len(), 2);
    assert_eq!(cache_before.len(), 2);

    conn.execute(
        "DELETE FROM request_logs WHERE trace_id LIKE 'trace-ledger-trend-%'",
        [],
    )
    .expect("delete request log details");
    conn.execute("DELETE FROM providers WHERE id = 411", [])
        .expect("delete provider record");

    let metric_after = provider_metric_trend_v1_with_conn(&conn, metric_query)
        .expect("metric trend after detail deletion");
    let cache_after = provider_cache_rate_trend_v1_with_conn(&conn, cache_query)
        .expect("cache trend after detail deletion");
    assert_eq!(metric_after, metric_before);
    assert_eq!(cache_after, cache_before);
    assert!(metric_after
        .iter()
        .all(|row| row.provider_name == "Ledger Trend Provider"));
}

#[test]
fn usage_params_accept_generated_and_legacy_cx2cc_filter_keys() {
    let params: UsageQueryParams = serde_json::from_value(serde_json::json!({
        "period": "daily",
        "startTs": null,
        "endTs": null,
        "cliKey": null,
        "providerId": null,
        "folderKeys": null,
        "dayStartHour": 5,
        "excludeCx2CcGatewayBridge": true
    }))
    .expect("deserialize usage query params");
    assert_eq!(params.exclude_cx2cc_gateway_bridge, Some(true));
    assert_eq!(params.day_start_hour, Some(5));

    let legacy_params: UsageQueryParams = serde_json::from_value(serde_json::json!({
        "period": "daily",
        "startTs": null,
        "endTs": null,
        "cliKey": null,
        "providerId": null,
        "folderKeys": null,
        "excludeCx2ccGatewayBridge": true
    }))
    .expect("deserialize legacy usage query params");
    assert_eq!(legacy_params.exclude_cx2cc_gateway_bridge, Some(true));

    let detail_params: UsageDayDetailParams = serde_json::from_value(serde_json::json!({
        "day": "2026-04-22",
        "cliKey": null,
        "providerId": null,
        "folderLimit": 8,
        "folderKeys": null,
        "dayStartHour": 5,
        "excludeCx2CcGatewayBridge": true
    }))
    .expect("deserialize usage day detail params");
    assert_eq!(detail_params.exclude_cx2cc_gateway_bridge, Some(true));
    assert_eq!(detail_params.day_start_hour, Some(5));

    let legacy_detail_params: UsageDayDetailParams = serde_json::from_value(serde_json::json!({
        "day": "2026-04-22",
        "cliKey": null,
        "providerId": null,
        "folderLimit": 8,
        "folderKeys": null,
        "excludeCx2ccGatewayBridge": true
    }))
    .expect("deserialize legacy usage day detail params");
    assert_eq!(
        legacy_detail_params.exclude_cx2cc_gateway_bridge,
        Some(true)
    );
}

fn fixture_folder_lookup(keys: &[UsageSessionLookupKey]) -> Vec<UsageResolvedFolder> {
    let requested: std::collections::HashSet<String> = keys
        .iter()
        .map(|key| format!("{}:{}", key.cli_key, key.session_id))
        .collect();
    let fixtures = [
        ("codex", "codex-alpha-1", "alpha", "/work/alpha"),
        ("codex", "codex-alpha-2", "alpha", "/work/alpha"),
        ("claude", "claude-alpha-1", "alpha", "/work/alpha"),
        ("codex", "codex-beta-1", "beta", "/work/beta"),
    ];

    fixtures
        .into_iter()
        .filter(|(cli_key, session_id, _, _)| {
            requested.contains(&format!("{cli_key}:{session_id}"))
        })
        .map(
            |(cli_key, session_id, folder_name, folder_path)| UsageResolvedFolder {
                cli_key: cli_key.to_string(),
                session_id: session_id.to_string(),
                folder_name: folder_name.to_string(),
                folder_path: folder_path.to_string(),
            },
        )
        .collect()
}

#[test]
fn cost_aggregates_above_sqlite_integer_ceiling_across_leaderboard_and_folder_paths() {
    let conn = setup_conn();
    let start_ts = 1_000;
    for created_at in [1_001, 1_002] {
        insert_usage_log(
            &conn,
            TestUsageLog {
                cost_usd_femto: Some(i64::MAX),
                session_id: Some("codex-alpha-1"),
                created_at,
                ..base_usage_log(created_at)
            },
        );
    }

    let expected_cost_usd = (i64::MAX as f64 * 2.0) / 1_000_000_000_000_000.0;
    let assert_cost = |actual: Option<f64>| {
        let actual = actual.expect("covered cost");
        assert!(actual > i64::MAX as f64 / 1_000_000_000_000_000.0);
        assert!((actual - expected_cost_usd).abs() < expected_cost_usd * 1e-12);
    };

    let rows = leaderboard_v2_with_conn(
        &conn,
        UsageScopeV2::Provider,
        Some(start_ts),
        Some(2_000),
        None,
        None,
        Some(50),
        false,
    )
    .expect("provider leaderboard");
    assert_eq!(rows.len(), 1);
    assert_cost(rows[0].cost_usd);

    let folder_rows = leaderboard_v2_folder_filtered_with_conn(
        &conn,
        FolderFilteredLeaderboardParams {
            scope: UsageScopeV2::Provider,
            start_ts: Some(start_ts),
            end_ts: Some(2_000),
            cli_key: None,
            provider_id: None,
            folder_keys: &["/work/alpha".to_string()],
            limit: Some(50),
            exclude_cx2cc_gateway_bridge: false,
            day_start_hour: 0,
        },
        fixture_folder_lookup,
    )
    .expect("folder-filtered provider leaderboard");
    assert_eq!(folder_rows.len(), 1);
    assert_cost(folder_rows[0].cost_usd);

    let summary = summary_v2_with_conn(
        &conn,
        &UsageQueryParams {
            period: "custom".to_string(),
            start_ts: Some(start_ts),
            end_ts: Some(2_000),
            cli_key: None,
            provider_id: None,
            folder_keys: Some(vec!["/work/alpha".to_string()]),
            day_start_hour: None,
            exclude_cx2cc_gateway_bridge: None,
        },
        fixture_folder_lookup,
    )
    .expect("folder-filtered summary");
    assert_eq!(summary.requests_total, 2);
    assert_eq!(summary.cost_covered_success, 2);
}

#[test]
fn cx2cc_gateway_bridge_filter_covers_overview_and_home_usage_queries() {
    let (_dir, db) = setup_temp_db();
    let conn = db.open_connection().expect("open test db connection");
    let start_ts = compute_start_ts_last_n_days(&conn, 1).expect("today start ts");
    let day = local_day_key(&conn, start_ts);

    insert_migrated_provider(&conn, 100, "claude", "CX2CC Gateway", None, Some("cx2cc"));
    insert_migrated_provider(&conn, 200, "codex", "Codex Inner", None, None);
    insert_migrated_provider(
        &conn,
        300,
        "claude",
        "CX2CC Fixed Source",
        Some(200),
        Some("cx2cc"),
    );

    insert_migrated_usage_log(
        &conn,
        "trace-outer",
        "claude",
        100,
        "CX2CC Gateway",
        1_000,
        100,
        start_ts + 60,
        Some("claude-alpha-1"),
    );
    insert_migrated_usage_log(
        &conn,
        "trace-inner",
        "codex",
        200,
        "Codex Inner",
        2_000,
        200,
        start_ts + 120,
        Some("codex-alpha-1"),
    );
    insert_migrated_usage_log(
        &conn,
        "trace-fixed-source",
        "claude",
        300,
        "CX2CC Fixed Source",
        3_000,
        300,
        start_ts + 180,
        Some("claude-alpha-1"),
    );

    let unfiltered = summary_query(
        &conn,
        Some(start_ts),
        Some(start_ts + 86_400),
        None,
        None,
        false,
    )
    .expect("unfiltered summary");
    assert_eq!(unfiltered.requests_total, 3);
    assert_eq!(unfiltered.total_tokens, 6_600);

    let filtered = summary_query(
        &conn,
        Some(start_ts),
        Some(start_ts + 86_400),
        None,
        None,
        true,
    )
    .expect("filtered summary");
    assert_eq!(filtered.requests_total, 2);
    assert_eq!(filtered.total_tokens, 5_500);

    let rows = leaderboard_v2_with_conn(
        &conn,
        UsageScopeV2::Provider,
        Some(start_ts),
        Some(start_ts + 86_400),
        None,
        None,
        Some(50),
        true,
    )
    .expect("filtered provider leaderboard");
    let keys: std::collections::HashSet<&str> = rows.iter().map(|row| row.key.as_str()).collect();
    assert!(!keys.contains("claude:100"));
    assert!(keys.contains("codex:200"));
    assert!(keys.contains("claude:300"));

    let summary_v2_filtered = summary_v2_with_conn(
        &conn,
        &UsageQueryParams {
            period: "custom".to_string(),
            start_ts: Some(start_ts),
            end_ts: Some(start_ts + 86_400),
            cli_key: None,
            provider_id: None,
            folder_keys: None,
            day_start_hour: None,
            exclude_cx2cc_gateway_bridge: Some(true),
        },
        fixture_folder_lookup,
    )
    .expect("filtered summary v2");
    assert_eq!(summary_v2_filtered.requests_total, 2);
    assert_eq!(summary_v2_filtered.total_tokens, 5_500);

    let summary_v2_unfiltered = summary_v2_with_conn(
        &conn,
        &UsageQueryParams {
            period: "custom".to_string(),
            start_ts: Some(start_ts),
            end_ts: Some(start_ts + 86_400),
            cli_key: None,
            provider_id: None,
            folder_keys: None,
            day_start_hour: None,
            exclude_cx2cc_gateway_bridge: Some(false),
        },
        fixture_folder_lookup,
    )
    .expect("unfiltered summary v2");
    assert_eq!(summary_v2_unfiltered.requests_total, 3);
    assert_eq!(summary_v2_unfiltered.total_tokens, 6_600);

    let folder_options = folder_options_v1_with_conn(
        &conn,
        &UsageQueryParams {
            period: "custom".to_string(),
            start_ts: Some(start_ts),
            end_ts: Some(start_ts + 86_400),
            cli_key: None,
            provider_id: None,
            folder_keys: None,
            day_start_hour: None,
            exclude_cx2cc_gateway_bridge: Some(true),
        },
        fixture_folder_lookup,
    )
    .expect("filtered folder options");
    let alpha = folder_options
        .iter()
        .find(|row| row.key == "/work/alpha")
        .expect("alpha folder option");
    assert_eq!(alpha.requests_total, 2);
    assert_eq!(alpha.total_tokens, 5_500);

    let detail = day_detail_v1_with_conn(
        &conn,
        &UsageDayDetailParams {
            day: day.to_string(),
            cli_key: None,
            provider_id: None,
            folder_limit: None,
            folder_keys: Some(vec!["/work/alpha".to_string()]),
            day_start_hour: None,
            exclude_cx2cc_gateway_bridge: Some(true),
        },
        fixture_folder_lookup,
    )
    .expect("filtered day detail");
    assert_eq!(
        detail
            .hours
            .iter()
            .map(|row| row.requests_total)
            .sum::<i64>(),
        2
    );
    assert_eq!(detail.folders.len(), 1);
    assert_eq!(detail.folders[0].key, "/work/alpha");
    assert_eq!(detail.folders[0].total_tokens, 5_500);

    drop(conn);

    let hourly_rows = hourly_series(&db, 1).expect("hourly series");
    let hourly_total: i64 = hourly_rows.iter().map(|row| row.total_tokens).sum();
    assert_eq!(hourly_total, 5_500);
}

#[test]
fn legacy_leaderboards_use_effective_input_for_persisted_cx2cc_semantics() {
    let (_dir, db) = setup_temp_db();
    let conn = db.open_connection().expect("open test db connection");
    let created_at = compute_start_ts_last_n_days(&conn, 1)
        .expect("today start ts")
        .saturating_add(60);

    insert_migrated_usage_log(
        &conn,
        "trace-legacy-leaderboard-cx2cc",
        "claude",
        901,
        "Deleted CX2CC Provider",
        1_000,
        50,
        created_at,
        None,
    );
    conn.execute(
        r#"
UPDATE request_logs
SET cache_read_input_tokens = 100,
    cache_creation_input_tokens = 200,
    special_settings_json = ?2
WHERE trace_id = ?1
        "#,
        params![
            "trace-legacy-leaderboard-cx2cc",
            r#"[{"type":"cx2cc_cost_basis","source_cli_key":"codex"}]"#
        ],
    )
    .expect("seed persisted CX2CC semantics");
    drop(conn);

    let provider_rows =
        leaderboard_provider(&db, "all", None, 10).expect("legacy provider leaderboard");
    assert_eq!(provider_rows.len(), 1);
    let provider = &provider_rows[0];
    assert_eq!(provider.input_tokens, 700);
    assert_eq!(provider.output_tokens, 50);
    assert_eq!(provider.total_tokens, 1_050);
    assert_eq!(provider.cache_read_input_tokens, 100);
    assert_eq!(provider.cache_creation_input_tokens, 200);

    let day_rows = leaderboard_day(&db, "all", None, 10).expect("legacy day leaderboard");
    assert_eq!(day_rows.len(), 1);
    let day = &day_rows[0];
    assert_eq!(day.input_tokens, 700);
    assert_eq!(day.output_tokens, 50);
    assert_eq!(day.total_tokens, 1_050);
    assert_eq!(day.cache_read_input_tokens, 100);
    assert_eq!(day.cache_creation_input_tokens, 200);
}

#[test]
fn legacy_leaderboards_fallback_to_persisted_total_only_without_double_counting() {
    let (_dir, db) = setup_temp_db();
    let conn = db.open_connection().expect("open test db connection");
    let created_at = compute_start_ts_last_n_days(&conn, 1)
        .expect("today start ts")
        .saturating_add(120);

    insert_migrated_usage_log(
        &conn,
        "trace-legacy-total-only",
        "codex",
        902,
        "Legacy Total Provider",
        1,
        1,
        created_at,
        None,
    );
    conn.execute(
        r#"
UPDATE request_logs
SET input_tokens = NULL,
    output_tokens = NULL,
    total_tokens = 777,
    cache_read_input_tokens = NULL,
    cache_creation_input_tokens = NULL,
    cache_creation_5m_input_tokens = NULL,
    cache_creation_1h_input_tokens = NULL
WHERE trace_id = ?1
        "#,
        params!["trace-legacy-total-only"],
    )
    .expect("seed total-only legacy row");

    insert_migrated_usage_log(
        &conn,
        "trace-legacy-canonical-buckets",
        "codex",
        902,
        "Legacy Total Provider",
        100,
        20,
        created_at.saturating_add(1),
        None,
    );
    conn.execute(
        r#"
UPDATE request_logs
SET total_tokens = 9999,
    cache_read_input_tokens = 10,
    cache_creation_input_tokens = 5
WHERE trace_id = ?1
        "#,
        params!["trace-legacy-canonical-buckets"],
    )
    .expect("seed canonical bucket row with stale persisted total");
    drop(conn);

    let provider_rows =
        leaderboard_provider(&db, "all", None, 10).expect("legacy provider leaderboard");
    assert_eq!(provider_rows.len(), 1);
    let provider = &provider_rows[0];
    assert_eq!(provider.input_tokens, 85);
    assert_eq!(provider.output_tokens, 20);
    assert_eq!(provider.total_tokens, 897);
    assert_eq!(provider.cache_read_input_tokens, 10);
    assert_eq!(provider.cache_creation_input_tokens, 5);

    let day_rows = leaderboard_day(&db, "all", None, 10).expect("legacy day leaderboard");
    assert_eq!(day_rows.len(), 1);
    assert_eq!(day_rows[0].total_tokens, 897);
}

#[test]
fn v2_cache_rate_denominator_aligns_across_clis() {
    let conn = setup_conn();

    // Codex/Gemini: cache_read_input_tokens is a subset of input_tokens.
    conn.execute(
        r#"
INSERT INTO usage_events (
  cli_key,
  attempts_json,
  final_provider_id,
  requested_model,
  status,
  error_code,
  duration_ms,
  ttfb_ms,
  input_tokens,
  output_tokens,
	  total_tokens,
	  cache_read_input_tokens,
	  cache_creation_input_tokens,
	  cache_creation_5m_input_tokens,
	  cache_creation_1h_input_tokens,
	  cost_usd_femto,
	  usage_json,
	  excluded_from_stats,
	  created_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19);
	"#,
        params![
            "codex",
            r#"[{"provider_id":123,"provider_name":"OpenAI","outcome":"success"}]"#,
            123,
            "gpt-test",
            200,
            Option::<String>::None,
            1000,
            100,
            100,
            10,
            999,
            30,
            0,
            0,
            0,
            1_000_000_000_000_000i64,
            Option::<String>::None,
            0,
            1000
        ],
    )
    .expect("insert codex");

    conn.execute(
        r#"
INSERT INTO usage_events (
  cli_key,
  attempts_json,
  final_provider_id,
  requested_model,
  status,
  error_code,
  duration_ms,
  ttfb_ms,
  input_tokens,
  output_tokens,
	  total_tokens,
	  cache_read_input_tokens,
	  cache_creation_input_tokens,
	  cache_creation_5m_input_tokens,
	  cache_creation_1h_input_tokens,
	  cost_usd_femto,
	  usage_json,
	  excluded_from_stats,
	  created_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19);
	"#,
        params![
            "gemini",
            r#"[{"provider_id":456,"provider_name":"GeminiUpstream","outcome":"success"}]"#,
            456,
            "gemini-test",
            200,
            Option::<String>::None,
            1000,
            100,
            200,
            20,
            0,
            50,
            0,
            0,
            0,
            2_000_000_000_000_000i64,
            Option::<String>::None,
            0,
            1000
        ],
    )
    .expect("insert gemini");

    // Claude: cache_read/cache_creation are additional buckets (not a subset of input_tokens).
    conn.execute(
        r#"
INSERT INTO usage_events (
  cli_key,
  attempts_json,
  final_provider_id,
  requested_model,
  status,
  error_code,
  duration_ms,
  ttfb_ms,
  input_tokens,
  output_tokens,
	  total_tokens,
	  cache_read_input_tokens,
	  cache_creation_input_tokens,
	  cache_creation_5m_input_tokens,
	  cache_creation_1h_input_tokens,
	  cost_usd_femto,
	  usage_json,
	  excluded_from_stats,
	  created_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19);
	"#,
        params![
            "claude",
            r#"[{"provider_id":789,"provider_name":"ClaudeUpstream","outcome":"success"}]"#,
            789,
            "claude-test",
            200,
            Option::<String>::None,
            1000,
            100,
            300,
            30,
            Option::<i64>::None,
            40,
            25,
            0,
            0,
            Option::<i64>::None,
            Option::<String>::None,
            0,
            1000
        ],
    )
    .expect("insert claude");

    let summary = summary_query(&conn, None, None, None, None, false).expect("summary_query");
    assert_eq!(summary.requests_total, 3);
    assert_eq!(summary.cost_covered_success, 2);
    assert_eq!(summary.input_tokens, 520);
    assert_eq!(summary.output_tokens, 60);
    assert_eq!(summary.io_total_tokens, 580);
    assert_eq!(summary.cache_read_input_tokens, 120);
    assert_eq!(summary.cache_creation_input_tokens, 25);
    assert_eq!(summary.total_tokens, 725);

    let rows = leaderboard_v2_with_conn(
        &conn,
        UsageScopeV2::Provider,
        None,
        None,
        None,
        None,
        Some(50),
        false,
    )
    .expect("leaderboard_v2_with_conn");
    assert_eq!(rows.len(), 3);

    let by_key: std::collections::HashMap<String, UsageLeaderboardRow> =
        rows.into_iter().map(|row| (row.key.clone(), row)).collect();

    let codex = by_key.get("codex:123").expect("codex row");
    assert_eq!(codex.input_tokens, 70);
    assert_eq!(codex.output_tokens, 10);
    assert_eq!(codex.io_total_tokens, 80);
    assert_eq!(codex.cache_read_input_tokens, 30);
    assert_eq!(codex.cache_creation_input_tokens, 0);
    assert_eq!(codex.total_tokens, 110);
    assert_eq!(codex.cost_usd, Some(1.0));

    let gemini = by_key.get("gemini:456").expect("gemini row");
    assert_eq!(gemini.input_tokens, 150);
    assert_eq!(gemini.output_tokens, 20);
    assert_eq!(gemini.io_total_tokens, 170);
    assert_eq!(gemini.cache_read_input_tokens, 50);
    assert_eq!(gemini.cache_creation_input_tokens, 0);
    assert_eq!(gemini.total_tokens, 220);
    assert_eq!(gemini.cost_usd, Some(2.0));

    let claude = by_key.get("claude:789").expect("claude row");
    assert_eq!(claude.input_tokens, 300);
    assert_eq!(claude.output_tokens, 30);
    assert_eq!(claude.io_total_tokens, 330);
    assert_eq!(claude.cache_read_input_tokens, 40);
    assert_eq!(claude.cache_creation_input_tokens, 25);
    assert_eq!(claude.total_tokens, 395);
    assert_eq!(claude.cost_usd, None);

    let rows = leaderboard_v2_with_conn(
        &conn,
        UsageScopeV2::Cli,
        None,
        None,
        None,
        None,
        Some(50),
        false,
    )
    .expect("leaderboard_v2_with_conn cli");
    let by_key: std::collections::HashMap<String, UsageLeaderboardRow> =
        rows.into_iter().map(|row| (row.key.clone(), row)).collect();
    assert_eq!(
        by_key.get("codex").expect("codex cli row").cost_usd,
        Some(1.0)
    );
    assert_eq!(
        by_key.get("gemini").expect("gemini cli row").cost_usd,
        Some(2.0)
    );
    assert_eq!(by_key.get("claude").expect("claude cli row").cost_usd, None);

    let rows = leaderboard_v2_with_conn(
        &conn,
        UsageScopeV2::Model,
        None,
        None,
        None,
        None,
        Some(50),
        false,
    )
    .expect("leaderboard_v2_with_conn model");
    let by_key: std::collections::HashMap<String, UsageLeaderboardRow> =
        rows.into_iter().map(|row| (row.key.clone(), row)).collect();
    assert_eq!(
        by_key.get("gpt-test").expect("gpt-test model row").cost_usd,
        Some(1.0)
    );
    assert_eq!(
        by_key
            .get("gemini-test")
            .expect("gemini-test model row")
            .cost_usd,
        Some(2.0)
    );
    assert_eq!(
        by_key
            .get("claude-test")
            .expect("claude-test model row")
            .cost_usd,
        None
    );
}

#[test]
fn v2_cache_rate_denominator_treats_cx2cc_like_cached_input_subtract() {
    let conn = setup_conn();

    conn.execute(
        r#"INSERT INTO providers (id, name, source_provider_id, bridge_type) VALUES (?1, ?2, ?3, ?4);"#,
        params![900, "Bridge CX2CC", 42, "cx2cc"],
    )
    .expect("insert provider");

    conn.execute(
        r#"
INSERT INTO usage_events (
  cli_key,
  attempts_json,
  final_provider_id,
  requested_model,
  status,
  error_code,
  duration_ms,
  ttfb_ms,
  input_tokens,
  output_tokens,
  total_tokens,
  cache_read_input_tokens,
  cache_creation_input_tokens,
  cache_creation_5m_input_tokens,
  cache_creation_1h_input_tokens,
  cost_usd_femto,
  usage_json,
  excluded_from_stats,
  created_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19);
        "#,
        params![
            "claude",
            r#"[{"provider_id":900,"provider_name":"Bridge CX2CC","outcome":"success"}]"#,
            900,
            "claude-through-cx2cc",
            200,
            Option::<String>::None,
            1000,
            100,
            100,
            10,
            Option::<i64>::None,
            30,
            0,
            0,
            0,
            Option::<i64>::None,
            Option::<String>::None,
            0,
            1000
        ],
    )
    .expect("insert cx2cc request");

    let summary = summary_query(&conn, None, None, None, None, false).expect("summary_query");
    assert_eq!(summary.cost_covered_success, 0);
    assert_eq!(summary.input_tokens, 70);
    assert_eq!(summary.cache_read_input_tokens, 30);
    assert_eq!(summary.total_tokens, 110);

    let rows = leaderboard_v2_with_conn(
        &conn,
        UsageScopeV2::Provider,
        None,
        None,
        None,
        None,
        Some(50),
        false,
    )
    .expect("leaderboard_v2_with_conn");
    let row = rows
        .iter()
        .find(|row| row.key == "claude:900")
        .expect("cx2cc provider row");
    assert_eq!(row.input_tokens, 70);
    assert_eq!(row.cache_read_input_tokens, 30);
    assert_eq!(row.total_tokens, 110);
}

#[test]
fn v2_cache_rate_denominator_treats_source_provider_id_as_bridged_input_semantics() {
    let conn = setup_conn();

    conn.execute(
        r#"INSERT INTO providers (id, name, source_provider_id, bridge_type) VALUES (?1, ?2, ?3, ?4);"#,
        params![
            901,
            "Source Link Bridge Semantics",
            42,
            Option::<String>::None
        ],
    )
    .expect("insert provider");

    conn.execute(
        r#"
INSERT INTO usage_events (
  cli_key,
  attempts_json,
  final_provider_id,
  requested_model,
  status,
  error_code,
  duration_ms,
  ttfb_ms,
  input_tokens,
  output_tokens,
  total_tokens,
  cache_read_input_tokens,
  cache_creation_input_tokens,
  cache_creation_5m_input_tokens,
  cache_creation_1h_input_tokens,
  cost_usd_femto,
  usage_json,
  excluded_from_stats,
  created_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19);
        "#,
        params![
            "claude",
            r#"[{"provider_id":901,"provider_name":"Source Link Bridge Semantics","outcome":"success"}]"#,
            901,
            "claude-with-source-link",
            200,
            Option::<String>::None,
            1000,
            100,
            100,
            10,
            Option::<i64>::None,
            30,
            0,
            0,
            0,
            Option::<i64>::None,
            Option::<String>::None,
            0,
            1000
        ],
    )
    .expect("insert source-linked request");

    let summary = summary_query(&conn, None, None, None, None, false).expect("summary_query");
    assert_eq!(summary.input_tokens, 70);
    assert_eq!(summary.cache_read_input_tokens, 30);
    assert_eq!(summary.total_tokens, 110);

    let rows = leaderboard_v2_with_conn(
        &conn,
        UsageScopeV2::Provider,
        None,
        None,
        None,
        None,
        Some(50),
        false,
    )
    .expect("leaderboard_v2_with_conn");
    let row = rows
        .iter()
        .find(|row| row.key == "claude:901")
        .expect("source-linked provider row");
    assert_eq!(row.input_tokens, 70);
    assert_eq!(row.cache_read_input_tokens, 30);
    assert_eq!(row.total_tokens, 110);

    conn.execute(
        "UPDATE providers SET source_provider_id = NULL, bridge_type = NULL WHERE id = 901",
        [],
    )
    .expect("remove live bridge relationship");
    let summary_after_provider_change =
        summary_query(&conn, None, None, None, None, false).expect("summary after provider change");
    assert_eq!(
        summary_after_provider_change.input_tokens, 70,
        "persisted input semantics must not drift with later provider edits"
    );
}

#[test]
fn v2_provider_leaderboard_dedupes_by_provider_id() {
    let conn = setup_conn();

    for (provider_name, created_at) in [("OpenAI", 1000i64), ("OpenAI ", 1001i64)] {
        let attempts_json = format!(
            r#"[{{"provider_id":123,"provider_name":"{provider_name}","outcome":"success"}}]"#
        );

        conn.execute(
            r#"
INSERT INTO usage_events (
  cli_key,
  attempts_json,
  final_provider_id,
  status,
  error_code,
  duration_ms,
  created_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);
        "#,
            params![
                "codex",
                attempts_json,
                123,
                200,
                Option::<String>::None,
                1000,
                created_at
            ],
        )
        .expect("insert request log");
    }

    let rows = leaderboard_v2_with_conn(
        &conn,
        UsageScopeV2::Provider,
        None,
        None,
        None,
        None,
        Some(50),
        false,
    )
    .expect("leaderboard_v2_with_conn provider");

    let keys: std::collections::HashSet<&str> = rows.iter().map(|row| row.key.as_str()).collect();
    assert_eq!(keys.len(), rows.len());

    let row = rows
        .iter()
        .find(|row| row.key == "codex:123")
        .expect("codex provider row");
    assert_eq!(row.name, "codex/OpenAI");
    assert_eq!(row.requests_total, 2);
    assert_eq!(row.requests_success, 2);
    assert_eq!(row.requests_failed, 0);
}

fn create_provider_daily_rollup_fixture_schema(conn: &Connection) {
    conn.execute_batch(
        r#"
CREATE TABLE usage_provider_daily_rollup_days (
  local_day TEXT PRIMARY KEY,
  day_start_ts INTEGER NOT NULL,
  day_end_ts INTEGER NOT NULL,
  status TEXT NOT NULL,
  source_row_count INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE usage_provider_daily_rollups (
  local_day TEXT NOT NULL,
  cli_key TEXT NOT NULL,
  final_provider_id INTEGER NOT NULL,
  provider_name_all_snapshot TEXT,
  provider_name_success_snapshot TEXT,
  created_at_min INTEGER NOT NULL,
  created_at_max INTEGER NOT NULL,
  requests_total INTEGER NOT NULL,
  requests_success INTEGER NOT NULL,
  success_duration_ms_sum INTEGER NOT NULL,
  success_ttfb_ms_sum INTEGER NOT NULL,
  success_ttfb_ms_count INTEGER NOT NULL,
  success_generation_ms_sum INTEGER NOT NULL,
  success_output_tokens_for_rate_sum INTEGER NOT NULL,
  success_output_rate_count INTEGER NOT NULL,
  cache_denom_tokens INTEGER NOT NULL,
  cache_read_input_tokens INTEGER NOT NULL,
  PRIMARY KEY(local_day, cli_key, final_provider_id)
);

CREATE TABLE usage_provider_daily_rollup_backfill_state (
  id INTEGER PRIMARY KEY,
  next_local_day TEXT,
  updated_at INTEGER NOT NULL
);

CREATE TABLE usage_ledger AS SELECT * FROM usage_events;
CREATE INDEX idx_usage_ledger_created_at ON usage_ledger(created_at);

CREATE TABLE usage_ledger_backfill_state (
  id INTEGER PRIMARY KEY,
  status TEXT NOT NULL
);

INSERT INTO usage_provider_daily_rollup_backfill_state(id, next_local_day, updated_at)
VALUES (1, NULL, 0);
INSERT INTO usage_ledger_backfill_state(id, status) VALUES (1, 'complete');
"#,
    )
    .expect("create Provider daily rollup fixture schema");
}

fn materialize_provider_daily_rollup_fixture(
    conn: &Connection,
    local_day: &str,
    day_start_ts: i64,
    day_end_ts: i64,
    status: &str,
) {
    let source_row_count = conn
        .query_row(
            r#"
SELECT COUNT(*)
FROM usage_events
WHERE created_at >= ?1
  AND created_at < ?2
  AND excluded_from_stats = 0
  AND final_provider_id IS NOT NULL
  AND final_provider_id > 0
"#,
            params![day_start_ts, day_end_ts],
            |row| row.get::<_, i64>(0),
        )
        .expect("count daily rollup source rows");
    conn.execute(
        r#"
INSERT INTO usage_provider_daily_rollup_days(
  local_day,
  day_start_ts,
  day_end_ts,
  status,
  source_row_count,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?3)
"#,
        params![
            local_day,
            day_start_ts,
            day_end_ts,
            status,
            source_row_count
        ],
    )
    .expect("insert daily rollup day fixture");
    if status != "complete" {
        return;
    }

    let success = "r.status >= 200 AND r.status < 300 AND r.error_present = 0";
    let valid_ttfb = "r.ttfb_ms IS NOT NULL AND r.ttfb_ms < r.duration_ms";
    let valid_output_rate =
        "r.output_tokens > 0 AND r.final_upstream_attempt_timing_version = 1 AND r.final_upstream_attempt_duration_ms IS NOT NULL AND r.final_upstream_attempt_duration_ms > 0";
    let effective_input = sql_effective_input_tokens_expr_with_alias("r");
    let cache_denom = format!(
        "({effective_input}) + COALESCE(r.cache_creation_input_tokens, 0) + COALESCE(r.cache_read_input_tokens, 0)"
    );
    let sql = format!(
        r#"
INSERT INTO usage_provider_daily_rollups(
  local_day,
  cli_key,
  final_provider_id,
  provider_name_all_snapshot,
  provider_name_success_snapshot,
  created_at_min,
  created_at_max,
  requests_total,
  requests_success,
  success_duration_ms_sum,
  success_ttfb_ms_sum,
  success_ttfb_ms_count,
  success_generation_ms_sum,
  success_output_tokens_for_rate_sum,
  success_output_rate_count,
  cache_denom_tokens,
  cache_read_input_tokens
)
SELECT
  ?1,
  r.cli_key,
  r.final_provider_id,
  MAX(NULLIF(TRIM(r.provider_name_snapshot), '')),
  MAX(CASE WHEN {success} THEN NULLIF(TRIM(r.provider_name_snapshot), '') END),
  MIN(r.created_at),
  MAX(r.created_at),
  COUNT(*),
  SUM(CASE WHEN {success} THEN 1 ELSE 0 END),
  SUM(CASE WHEN {success} THEN r.duration_ms ELSE 0 END),
  SUM(CASE WHEN {success} AND {valid_ttfb} THEN r.ttfb_ms ELSE 0 END),
  SUM(CASE WHEN {success} AND {valid_ttfb} THEN 1 ELSE 0 END),
  SUM(CASE WHEN {success} AND {valid_output_rate} THEN r.final_upstream_attempt_duration_ms ELSE 0 END),
  SUM(CASE WHEN {success} AND {valid_output_rate} THEN r.output_tokens ELSE 0 END),
  SUM(CASE WHEN {success} AND {valid_output_rate} THEN 1 ELSE 0 END),
  SUM(CASE WHEN {success} THEN {cache_denom} ELSE 0 END),
  SUM(CASE WHEN {success} THEN COALESCE(r.cache_read_input_tokens, 0) ELSE 0 END)
FROM usage_events r
WHERE r.created_at >= ?2
  AND r.created_at < ?3
  AND r.excluded_from_stats = 0
  AND r.final_provider_id IS NOT NULL
  AND r.final_provider_id > 0
GROUP BY r.cli_key, r.final_provider_id
"#
    );
    conn.execute(&sql, params![local_day, day_start_ts, day_end_ts])
        .expect("materialize daily Provider rollup fixture");
}

#[test]
fn provider_trends_mix_complete_rollups_with_raw_gaps_without_overlap() {
    let conn = setup_conn();
    conn.execute(
        "INSERT INTO providers (id, name) VALUES (?1, ?2)",
        params![123, "Alpha Success"],
    )
    .expect("insert normal Provider");
    conn.execute(
        r#"INSERT INTO providers (id, name, source_provider_id, bridge_type) VALUES (?1, ?2, ?3, ?4)"#,
        params![900, "Bridge CX2CC", Option::<i64>::None, "cx2cc"],
    )
    .expect("insert CX2CC Provider");

    let calendar_start = local_day_start_ts(&conn, "2024-01-01");
    for day in 0..=6i64 {
        let created_at = calendar_start + day * 86_400 + 3 * 3600;
        insert_usage_log(
            &conn,
            TestUsageLog {
                provider_name: "Alpha Success",
                duration_ms: 1000 + day,
                ttfb_ms: Some(100 + day),
                input_tokens: Some(200 + day),
                output_tokens: Some(20 + day),
                cache_read_input_tokens: Some(40 + day),
                created_at,
                ..base_usage_log(created_at)
            },
        );
    }
    insert_usage_log(
        &conn,
        TestUsageLog {
            provider_name: "Zulu Failure",
            status: Some(500),
            error_code: Some("UPSTREAM_ERROR"),
            created_at: calendar_start + 86_400 + 4 * 3600,
            ..base_usage_log(calendar_start + 86_400 + 4 * 3600)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "claude",
            provider_id: 900,
            provider_name: "Bridge CX2CC",
            input_tokens: Some(300),
            cache_read_input_tokens: Some(75),
            created_at: calendar_start + 3 * 86_400 + 5 * 3600,
            ..base_usage_log(calendar_start + 3 * 86_400 + 5 * 3600)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            provider_name: "Alpha Success",
            created_at: calendar_start + 6 * 86_400 + 30 * 60,
            ..base_usage_log(calendar_start + 6 * 86_400 + 30 * 60)
        },
    );

    let query_start = calendar_start + 2 * 3600;
    let query_end = calendar_start + 6 * 86_400 + 3600;
    let metric_query = ProviderMetricTrendQuery {
        start_ts: Some(query_start),
        end_ts: Some(query_end),
        cli_key: None,
        provider_id: None,
        limit: None,
        exclude_cx2cc_gateway_bridge: false,
    };
    let cache_query = ProviderCacheRateTrendQuery {
        start_ts: metric_query.start_ts,
        end_ts: metric_query.end_ts,
        cli_key: None,
        provider_id: None,
        limit: None,
        exclude_cx2cc_gateway_bridge: false,
    };
    let raw_metrics =
        provider_metric_trend_v1_with_conn(&conn, metric_query).expect("raw metric trend");
    let raw_cache =
        provider_cache_rate_trend_v1_with_conn(&conn, cache_query).expect("raw cache trend");
    let all_time_metric_query = ProviderMetricTrendQuery {
        start_ts: None,
        end_ts: None,
        ..metric_query
    };
    let all_time_cache_query = ProviderCacheRateTrendQuery {
        start_ts: None,
        end_ts: None,
        ..cache_query
    };
    let raw_all_time_metrics = provider_metric_trend_v1_with_conn(&conn, all_time_metric_query)
        .expect("raw all-time metric trend");
    let raw_all_time_cache = provider_cache_rate_trend_v1_with_conn(&conn, all_time_cache_query)
        .expect("raw all-time cache trend");
    let hour_metric_query = ProviderMetricTrendQuery {
        start_ts: Some(calendar_start + 86_400),
        end_ts: Some(calendar_start + 2 * 86_400),
        provider_id: Some(123),
        ..metric_query
    };
    let raw_hour_metrics = provider_metric_trend_v1_with_conn(&conn, hour_metric_query)
        .expect("raw hourly metric trend");

    create_provider_daily_rollup_fixture_schema(&conn);
    for (day_offset, status) in [(1i64, "complete"), (2, "dirty"), (3, "complete")] {
        let day_start = calendar_start + day_offset * 86_400;
        let local_day = local_day_key(&conn, day_start);
        materialize_provider_daily_rollup_fixture(
            &conn,
            &local_day,
            day_start,
            day_start + 86_400,
            status,
        );
    }

    let hybrid_metrics =
        provider_metric_trend_v1_with_conn(&conn, metric_query).expect("hybrid metric trend");
    let hybrid_cache =
        provider_cache_rate_trend_v1_with_conn(&conn, cache_query).expect("hybrid cache trend");
    assert_eq!(hybrid_metrics, raw_metrics);
    assert_eq!(hybrid_cache, raw_cache);
    assert_eq!(
        provider_metric_trend_v1_with_conn(&conn, all_time_metric_query)
            .expect("hybrid all-time metric trend"),
        raw_all_time_metrics
    );
    assert_eq!(
        provider_cache_rate_trend_v1_with_conn(&conn, all_time_cache_query)
            .expect("hybrid all-time cache trend"),
        raw_all_time_cache
    );
    assert_eq!(
        provider_metric_trend_v1_with_conn(&conn, hour_metric_query)
            .expect("hourly trend with daily rollup schema"),
        raw_hour_metrics,
        "hour granularity must remain raw-only"
    );
    assert!(hybrid_metrics.iter().any(|row| {
        row.provider_id == 123
            && row.provider_name == "Zulu Failure"
            && row.granularity == UsageTrendGranularityV1::Day
    }));
    assert!(hybrid_cache
        .iter()
        .any(|row| row.key == "codex:123" && row.name == "codex/Alpha Success"));

    let raw_excluded_metrics = provider_metric_trend_v1_with_conn(
        &conn,
        ProviderMetricTrendQuery {
            exclude_cx2cc_gateway_bridge: true,
            ..metric_query
        },
    )
    .expect("hybrid metric trend excluding CX2CC");
    let raw_excluded_cache = provider_cache_rate_trend_v1_with_conn(
        &conn,
        ProviderCacheRateTrendQuery {
            exclude_cx2cc_gateway_bridge: true,
            ..cache_query
        },
    )
    .expect("hybrid cache trend excluding CX2CC");
    assert!(raw_excluded_metrics
        .iter()
        .all(|row| row.provider_id != 900));
    assert!(raw_excluded_cache.iter().all(|row| row.key != "claude:900"));

    let stale_day = local_day_key(&conn, calendar_start + 86_400);
    conn.execute(
        r#"
UPDATE usage_provider_daily_rollups
SET requests_total = requests_total + 1,
    success_duration_ms_sum = success_duration_ms_sum + 999999,
    cache_denom_tokens = cache_denom_tokens + 999999
WHERE local_day = ?1
"#,
        [&stale_day],
    )
    .expect("make rollup source count inconsistent");
    assert_eq!(
        provider_metric_trend_v1_with_conn(&conn, metric_query)
            .expect("raw metric fallback for inconsistent rollup count"),
        raw_metrics
    );
    assert_eq!(
        provider_cache_rate_trend_v1_with_conn(&conn, cache_query)
            .expect("raw cache fallback for inconsistent rollup count"),
        raw_cache
    );
    conn.execute(
        r#"
UPDATE usage_provider_daily_rollups
SET requests_total = requests_total - 1,
    success_duration_ms_sum = success_duration_ms_sum - 999999,
    cache_denom_tokens = cache_denom_tokens - 999999
WHERE local_day = ?1
"#,
        [&stale_day],
    )
    .expect("restore rollup source count");
    conn.execute(
        "UPDATE usage_provider_daily_rollup_days SET day_start_ts = day_start_ts + 1 WHERE local_day = ?1",
        [&stale_day],
    )
    .expect("make rollup calendar boundary stale");
    conn.execute(
        r#"
UPDATE usage_provider_daily_rollups
SET success_duration_ms_sum = success_duration_ms_sum + 999999,
    cache_denom_tokens = cache_denom_tokens + 999999
WHERE local_day = ?1
"#,
        [&stale_day],
    )
    .expect("make stale-boundary rollup observably incorrect");
    assert_eq!(
        provider_metric_trend_v1_with_conn(&conn, metric_query)
            .expect("raw metric fallback for stale rollup boundary"),
        raw_metrics
    );
    assert_eq!(
        provider_cache_rate_trend_v1_with_conn(&conn, cache_query)
            .expect("raw cache fallback for stale rollup boundary"),
        raw_cache
    );

    conn.execute(
        "UPDATE usage_ledger_backfill_state SET status = 'incomplete' WHERE id = 1",
        [],
    )
    .expect("mark ledger backfill incomplete");
    conn.execute(
        r#"
UPDATE usage_provider_daily_rollups
SET success_duration_ms_sum = success_duration_ms_sum + 999999,
    cache_denom_tokens = cache_denom_tokens + 999999
"#,
        [],
    )
    .expect("make rollup observably stale");
    assert_eq!(
        provider_metric_trend_v1_with_conn(&conn, metric_query)
            .expect("raw metric fallback while ledger backfill is incomplete"),
        raw_metrics
    );
    assert_eq!(
        provider_cache_rate_trend_v1_with_conn(&conn, cache_query)
            .expect("raw cache fallback while ledger backfill is incomplete"),
        raw_cache
    );
}

#[test]
fn v1_provider_cache_rate_trend_uses_effective_denom_and_bucket() {
    let conn = setup_conn();

    conn.execute(
        r#"INSERT INTO providers (id, name) VALUES (?1, ?2);"#,
        params![123, "OpenAI"],
    )
    .expect("insert provider");

    let start_ts_today = compute_start_ts(&conn, UsageRange::Today)
        .expect("compute_start_ts today")
        .expect("start ts exists");

    for (created_at, input_tokens, cache_read_input_tokens, cache_creation_input_tokens) in [
        (start_ts_today + 3600, 500i64, 200i64, 20i64),
        (start_ts_today + 7200, 100i64, 50i64, 10i64),
    ] {
        conn.execute(
            r#"
INSERT INTO usage_events (
  cli_key,
  attempts_json,
  final_provider_id,
  status,
  error_code,
  duration_ms,
  input_tokens,
  cache_read_input_tokens,
  cache_creation_input_tokens,
  created_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10);
            "#,
            params![
                "codex",
                r#"[{"provider_id":123,"provider_name":"OpenAI","outcome":"success"}]"#,
                123,
                200,
                Option::<String>::None,
                1000,
                input_tokens,
                cache_read_input_tokens,
                cache_creation_input_tokens,
                created_at
            ],
        )
        .expect("insert request log");
    }

    let rows_hour = provider_cache_rate_trend_v1_with_conn(
        &conn,
        ProviderCacheRateTrendQuery {
            start_ts: Some(start_ts_today),
            end_ts: Some(start_ts_today + 86_400),
            cli_key: None,
            provider_id: None,
            limit: None,
            exclude_cx2cc_gateway_bridge: false,
        },
    )
    .expect("provider_cache_rate_trend_v1_with_conn hour");

    assert_eq!(rows_hour.len(), 2);
    assert_eq!(rows_hour[0].name, "codex/OpenAI");
    assert_eq!(rows_hour[0].hour, Some(1));
    assert_eq!(rows_hour[0].granularity, UsageTrendGranularityV1::Hour);
    assert_eq!(rows_hour[0].denom_tokens, 500);
    assert_eq!(rows_hour[0].cache_read_input_tokens, 200);

    assert_eq!(rows_hour[1].hour, Some(2));
    assert_eq!(rows_hour[1].denom_tokens, 100);
    assert_eq!(rows_hour[1].cache_read_input_tokens, 50);

    // A six-day range no longer fits the 120-hour budget, so it uses day buckets.
    let rows_day = provider_cache_rate_trend_v1_with_conn(
        &conn,
        ProviderCacheRateTrendQuery {
            start_ts: Some(start_ts_today),
            end_ts: Some(start_ts_today + 6 * 86_400),
            cli_key: None,
            provider_id: None,
            limit: None,
            exclude_cx2cc_gateway_bridge: false,
        },
    )
    .expect("provider_cache_rate_trend_v1_with_conn day");

    assert_eq!(rows_day.len(), 1);
    assert_eq!(rows_day[0].hour, None);
    assert_eq!(rows_day[0].granularity, UsageTrendGranularityV1::Day);
    assert_eq!(rows_day[0].denom_tokens, 600);
    assert_eq!(rows_day[0].cache_read_input_tokens, 250);
    assert_eq!(rows_day[0].requests_success, 2);
}

#[test]
fn provider_metric_trend_matches_summary_formulas_and_sample_guards() {
    let conn = setup_conn();
    conn.execute(
        "INSERT INTO providers (id, name) VALUES (?1, ?2)",
        params![123, "Formula Provider"],
    )
    .expect("insert provider");
    let start_ts = local_day_start_ts(&conn, "2024-01-01");
    let bucket_ts = start_ts + 3600;

    insert_usage_log(
        &conn,
        TestUsageLog {
            provider_name: "Formula Provider",
            duration_ms: 1000,
            ttfb_ms: Some(200),
            final_upstream_attempt_duration_ms: Some(800),
            output_tokens: Some(300),
            created_at: bucket_ts,
            ..base_usage_log(bucket_ts)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            provider_name: "Formula Provider",
            duration_ms: 2000,
            ttfb_ms: Some(5000),
            final_upstream_attempt_duration_ms: Some(500),
            final_upstream_attempt_timing_version: 0,
            output_tokens: Some(999),
            created_at: bucket_ts + 1,
            ..base_usage_log(bucket_ts + 1)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            provider_name: "Formula Provider",
            duration_ms: 1200,
            ttfb_ms: Some(100),
            output_tokens: None,
            created_at: bucket_ts + 2,
            ..base_usage_log(bucket_ts + 2)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            provider_name: "Formula Provider",
            status: Some(500),
            error_code: Some("UPSTREAM_ERROR"),
            duration_ms: 90_000,
            ttfb_ms: Some(50),
            output_tokens: Some(90_000),
            created_at: bucket_ts + 3,
            ..base_usage_log(bucket_ts + 3)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            provider_name: "Formula Provider",
            duration_ms: 90_000,
            ttfb_ms: Some(50),
            output_tokens: Some(90_000),
            excluded_from_stats: 1,
            created_at: bucket_ts + 4,
            ..base_usage_log(bucket_ts + 4)
        },
    );

    let summary = summary_query(
        &conn,
        Some(start_ts),
        Some(start_ts + 86_400),
        None,
        Some(123),
        false,
    )
    .expect("summary");
    let rows = provider_metric_trend_v1_with_conn(
        &conn,
        ProviderMetricTrendQuery {
            start_ts: Some(start_ts),
            end_ts: Some(start_ts + 86_400),
            cli_key: None,
            provider_id: Some(123),
            limit: None,
            exclude_cx2cc_gateway_bridge: false,
        },
    )
    .expect("provider metric trend");

    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.granularity, UsageTrendGranularityV1::Hour);
    assert_eq!(row.requests_total, 4);
    assert_eq!(row.requests_success, 3);
    assert_eq!(row.duration_samples, 3);
    assert_eq!(row.ttfb_samples, 2);
    assert_eq!(row.output_rate_samples, 1);
    assert_eq!(row.avg_duration_ms, summary.avg_duration_ms);
    assert_eq!(row.avg_ttfb_ms, summary.avg_ttfb_ms);
    assert_eq!(
        row.avg_output_tokens_per_second,
        summary.avg_output_tokens_per_second
    );
    assert_eq!(row.avg_duration_ms, Some(1400));
    assert_eq!(row.avg_ttfb_ms, Some(150));
    assert_eq!(row.avg_output_tokens_per_second, Some(375.0));
}

#[test]
fn output_rate_uses_final_successful_attempt_instead_of_retry_elapsed_time() {
    let conn = setup_conn();
    conn.execute(
        "INSERT INTO providers (id, name) VALUES (?1, ?2)",
        params![123, "Retry Provider"],
    )
    .expect("insert provider");
    let start_ts = local_day_start_ts(&conn, "2024-01-01");
    let bucket_ts = start_ts + 3600;

    insert_usage_log(
        &conn,
        TestUsageLog {
            provider_name: "Retry Provider",
            duration_ms: 60_000,
            ttfb_ms: Some(19_800),
            final_upstream_attempt_duration_ms: Some(20_000),
            final_upstream_attempt_timing_version: 1,
            output_tokens: Some(1_200),
            created_at: bucket_ts,
            ..base_usage_log(bucket_ts)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            provider_name: "Retry Provider",
            duration_ms: 100,
            ttfb_ms: Some(1),
            final_upstream_attempt_duration_ms: Some(1),
            final_upstream_attempt_timing_version: 0,
            output_tokens: Some(9_999),
            created_at: bucket_ts + 1,
            ..base_usage_log(bucket_ts + 1)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            provider_name: "Retry Provider",
            duration_ms: 10_000,
            ttfb_ms: Some(10),
            final_upstream_attempt_duration_ms: Some(9_000),
            final_upstream_attempt_timing_version: 1,
            output_tokens: Some(0),
            created_at: bucket_ts + 2,
            ..base_usage_log(bucket_ts + 2)
        },
    );

    let summary = summary_query(
        &conn,
        Some(start_ts),
        Some(start_ts + 86_400),
        None,
        Some(123),
        false,
    )
    .expect("summary");
    let rows = provider_metric_trend_v1_with_conn(
        &conn,
        ProviderMetricTrendQuery {
            start_ts: Some(start_ts),
            end_ts: Some(start_ts + 86_400),
            cli_key: None,
            provider_id: Some(123),
            limit: None,
            exclude_cx2cc_gateway_bridge: false,
        },
    )
    .expect("provider metric trend");
    let leaderboard = leaderboard_v2_with_conn(
        &conn,
        UsageScopeV2::Provider,
        Some(start_ts),
        Some(start_ts + 86_400),
        None,
        Some(123),
        Some(50),
        false,
    )
    .expect("provider leaderboard");

    assert_eq!(summary.avg_output_tokens_per_second, Some(60.0));
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].output_rate_samples, 1);
    assert_eq!(rows[0].avg_output_tokens_per_second, Some(60.0));
    assert_eq!(leaderboard.len(), 1);
    assert_eq!(leaderboard[0].avg_output_tokens_per_second, Some(60.0));
}

#[test]
fn selected_provider_metric_trend_keeps_failure_only_buckets() {
    let conn = setup_conn();
    conn.execute(
        "INSERT INTO providers (id, name) VALUES (?1, ?2)",
        params![777, "Failure Only Provider"],
    )
    .expect("insert provider");
    let start_ts = local_day_start_ts(&conn, "2024-01-01");
    insert_usage_log(
        &conn,
        TestUsageLog {
            provider_id: 777,
            provider_name: "Failure Only Provider",
            status: Some(500),
            error_code: Some("UPSTREAM_ERROR"),
            created_at: start_ts + 3600,
            ..base_usage_log(start_ts + 3600)
        },
    );

    let rows = provider_metric_trend_v1_with_conn(
        &conn,
        ProviderMetricTrendQuery {
            start_ts: Some(start_ts),
            end_ts: Some(start_ts + 86_400),
            cli_key: None,
            provider_id: Some(777),
            limit: None,
            exclude_cx2cc_gateway_bridge: false,
        },
    )
    .expect("failure-only selected provider trend");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].requests_total, 1);
    assert_eq!(rows[0].requests_success, 0);
    assert_eq!(rows[0].avg_duration_ms, None);
    assert_eq!(rows[0].avg_ttfb_ms, None);
    assert_eq!(rows[0].avg_output_tokens_per_second, None);
}

#[test]
fn provider_trend_planner_selects_the_finest_bounded_granularity() {
    let conn = setup_conn();
    let start_ts = local_day_start_ts(&conn, "2024-01-01");
    let plan_for_days = |days: i64, provider_id: Option<i64>, limit: Option<usize>| {
        plan_trend(
            &conn,
            TrendPlanQuery {
                start_ts: Some(start_ts),
                end_ts: Some(start_ts + days * 86_400),
                cli_key: None,
                provider_id,
                requested_provider_limit: limit,
                exclude_cx2cc_gateway_bridge: false,
            },
        )
        .expect("trend plan")
    };

    assert_eq!(
        plan_for_days(4, None, None).granularity,
        UsageTrendGranularityV1::Hour
    );
    assert_eq!(
        plan_for_days(6, None, None).granularity,
        UsageTrendGranularityV1::Day
    );
    assert_eq!(
        plan_for_days(121, None, None).granularity,
        UsageTrendGranularityV1::Week
    );
    assert_eq!(
        plan_for_days(900, None, None).granularity,
        UsageTrendGranularityV1::Month
    );
    assert_eq!(
        plan_for_days(11 * 366, None, None).granularity,
        UsageTrendGranularityV1::Year
    );
    assert_eq!(plan_for_days(4, None, None).provider_limit, 10);
    assert_eq!(plan_for_days(4, None, Some(0)).provider_limit, 1);
    assert_eq!(plan_for_days(4, None, Some(999)).provider_limit, 10);
    assert_eq!(plan_for_days(4, Some(123), Some(999)).provider_limit, 1);

    let oversized = plan_trend(
        &conn,
        TrendPlanQuery {
            start_ts: Some(start_ts),
            end_ts: Some(start_ts + 121 * 366 * 86_400),
            cli_key: None,
            provider_id: None,
            requested_provider_limit: None,
            exclude_cx2cc_gateway_bridge: false,
        },
    )
    .expect_err("more than 120 calendar years must fail before aggregation");
    assert!(oversized.contains("120-bucket year budget"));
}

#[test]
fn provider_metric_and_cache_trends_share_top_request_and_row_budgets() {
    let mut conn = setup_conn();
    let start_ts = local_day_start_ts(&conn, "2024-01-01");
    let end_ts = start_ts + 130 * 86_400;
    let tx = conn.transaction().expect("start fixture transaction");
    for provider_id in 1..=12i64 {
        let provider_name = format!("Provider {provider_id}");
        tx.execute(
            "INSERT INTO providers (id, name) VALUES (?1, ?2)",
            params![provider_id, provider_name],
        )
        .expect("insert provider");
        for day in 0..130i64 {
            let created_at = start_ts + day * 86_400 + provider_id;
            insert_usage_log(
                &tx,
                TestUsageLog {
                    provider_id,
                    provider_name: &provider_name,
                    input_tokens: Some(if provider_id == 11 { 1_000_000 } else { 1 }),
                    output_tokens: Some(1),
                    created_at,
                    ..base_usage_log(created_at)
                },
            );
        }
    }
    insert_usage_log(
        &tx,
        TestUsageLog {
            provider_id: 1,
            provider_name: "Provider 1",
            input_tokens: Some(1),
            output_tokens: Some(1),
            created_at: start_ts + 1,
            ..base_usage_log(start_ts + 1)
        },
    );
    tx.commit().expect("commit fixture transaction");

    let metric_rows = provider_metric_trend_v1_with_conn(
        &conn,
        ProviderMetricTrendQuery {
            start_ts: Some(start_ts),
            end_ts: Some(end_ts),
            cli_key: None,
            provider_id: None,
            limit: None,
            exclude_cx2cc_gateway_bridge: false,
        },
    )
    .expect("bounded provider metric trend");
    let metric_providers = metric_rows
        .iter()
        .map(|row| row.key.as_str())
        .collect::<std::collections::HashSet<_>>();
    let metric_buckets = metric_rows
        .iter()
        .map(|row| (row.day.as_str(), row.hour))
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(metric_providers.len(), TREND_MAX_PROVIDERS);
    assert!(!metric_providers.contains("codex:11"));
    assert!(!metric_providers.contains("codex:12"));
    assert!(metric_buckets.len() <= TREND_MAX_BUCKETS);
    assert!(metric_rows.len() <= TREND_MAX_ROWS);
    assert!(metric_rows
        .iter()
        .all(|row| row.granularity == UsageTrendGranularityV1::Week));

    let selected_rows = provider_metric_trend_v1_with_conn(
        &conn,
        ProviderMetricTrendQuery {
            start_ts: Some(start_ts),
            end_ts: Some(end_ts),
            cli_key: None,
            provider_id: Some(12),
            limit: None,
            exclude_cx2cc_gateway_bridge: false,
        },
    )
    .expect("selected provider metric trend");
    assert!(selected_rows.len() <= TREND_MAX_BUCKETS);
    assert!(selected_rows.iter().all(|row| row.provider_id == 12));

    let cache_rows = provider_cache_rate_trend_v1_with_conn(
        &conn,
        ProviderCacheRateTrendQuery {
            start_ts: Some(start_ts),
            end_ts: Some(end_ts),
            cli_key: None,
            provider_id: None,
            limit: None,
            exclude_cx2cc_gateway_bridge: false,
        },
    )
    .expect("bounded provider cache trend");
    assert!(cache_rows.len() <= TREND_MAX_ROWS);
    assert_eq!(
        cache_rows
            .iter()
            .map(|row| row.key.as_str())
            .collect::<std::collections::HashSet<_>>()
            .len(),
        TREND_MAX_PROVIDERS
    );

    let cache_top = provider_cache_rate_trend_v1_with_conn(
        &conn,
        ProviderCacheRateTrendQuery {
            start_ts: Some(start_ts),
            end_ts: Some(end_ts),
            cli_key: None,
            provider_id: None,
            limit: Some(1),
            exclude_cx2cc_gateway_bridge: false,
        },
    )
    .expect("top provider cache trend");
    assert!(cache_top.iter().all(|row| row.key == "codex:1"));
}

#[test]
fn provider_trends_exclude_cx2cc_gateway_bridge_when_requested() {
    let conn = setup_conn();

    conn.execute(
        r#"INSERT INTO providers (id, name, source_provider_id, bridge_type) VALUES (?1, ?2, ?3, ?4);"#,
        params![123, "OpenAI", Option::<i64>::None, Option::<String>::None],
    )
    .expect("insert normal provider");
    conn.execute(
        r#"INSERT INTO providers (id, name, source_provider_id, bridge_type) VALUES (?1, ?2, ?3, ?4);"#,
        params![900, "Bridge CX2CC", Option::<i64>::None, "cx2cc"],
    )
    .expect("insert cx2cc provider");

    insert_usage_log(
        &conn,
        TestUsageLog {
            provider_id: 123,
            provider_name: "OpenAI",
            input_tokens: Some(120),
            cache_read_input_tokens: Some(20),
            created_at: 1000,
            ..base_usage_log(1000)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "claude",
            provider_id: 900,
            provider_name: "Bridge CX2CC",
            input_tokens: Some(240),
            cache_read_input_tokens: Some(80),
            created_at: 1010,
            ..base_usage_log(1010)
        },
    );

    let rows_with_bridge = provider_cache_rate_trend_v1_with_conn(
        &conn,
        ProviderCacheRateTrendQuery {
            start_ts: None,
            end_ts: None,
            cli_key: None,
            provider_id: None,
            limit: None,
            exclude_cx2cc_gateway_bridge: false,
        },
    )
    .expect("cache trend with bridge");
    assert_eq!(rows_with_bridge.len(), 2);

    let rows_without_bridge = provider_cache_rate_trend_v1_with_conn(
        &conn,
        ProviderCacheRateTrendQuery {
            start_ts: None,
            end_ts: None,
            cli_key: None,
            provider_id: None,
            limit: None,
            exclude_cx2cc_gateway_bridge: true,
        },
    )
    .expect("cache trend without bridge");
    assert_eq!(rows_without_bridge.len(), 1);
    assert_eq!(rows_without_bridge[0].key, "codex:123");

    let metric_rows_without_bridge = provider_metric_trend_v1_with_conn(
        &conn,
        ProviderMetricTrendQuery {
            start_ts: None,
            end_ts: None,
            cli_key: None,
            provider_id: None,
            limit: None,
            exclude_cx2cc_gateway_bridge: true,
        },
    )
    .expect("metric trend without bridge");
    assert_eq!(metric_rows_without_bridge.len(), 1);
    assert_eq!(metric_rows_without_bridge[0].key, "codex:123");

    conn.execute("UPDATE providers SET bridge_type = NULL WHERE id = 900", [])
        .expect("remove live gateway bridge relationship");
    let rows_after_provider_change = provider_cache_rate_trend_v1_with_conn(
        &conn,
        ProviderCacheRateTrendQuery {
            start_ts: None,
            end_ts: None,
            cli_key: None,
            provider_id: None,
            limit: None,
            exclude_cx2cc_gateway_bridge: true,
        },
    )
    .expect("cache trend after bridge relationship change");
    assert_eq!(
        rows_after_provider_change.len(),
        2,
        "gateway exclusion must follow the current provider relationship"
    );
    let metric_rows_after_provider_change = provider_metric_trend_v1_with_conn(
        &conn,
        ProviderMetricTrendQuery {
            start_ts: None,
            end_ts: None,
            cli_key: None,
            provider_id: None,
            limit: None,
            exclude_cx2cc_gateway_bridge: true,
        },
    )
    .expect("metric trend after bridge relationship change");
    assert_eq!(metric_rows_after_provider_change.len(), 2);
}

#[test]
fn v2_queries_apply_provider_filter() {
    let conn = setup_conn();

    for (provider_id, provider_name) in [(123, "OpenAI"), (456, "Gemini Upstream")] {
        conn.execute(
            "INSERT INTO providers (id, name) VALUES (?1, ?2)",
            params![provider_id, provider_name],
        )
        .expect("insert provider");
    }

    for (provider_id, cli_key, provider_name, input_tokens, created_at) in [
        (123, "codex", "OpenAI", 120, 1000i64),
        (456, "gemini", "Gemini Upstream", 240, 1010i64),
    ] {
        let attempts_json = format!(
            r#"[{{"provider_id":{provider_id},"provider_name":"{provider_name}","outcome":"success"}}]"#
        );

        conn.execute(
            r#"
INSERT INTO usage_events (
  cli_key,
  attempts_json,
  final_provider_id,
  requested_model,
  status,
  error_code,
  duration_ms,
  ttfb_ms,
  input_tokens,
  output_tokens,
  total_tokens,
  cache_read_input_tokens,
  cache_creation_input_tokens,
  cache_creation_5m_input_tokens,
  cache_creation_1h_input_tokens,
  cost_usd_femto,
  usage_json,
  excluded_from_stats,
  created_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19);
            "#,
            params![
                cli_key,
                attempts_json,
                provider_id,
                "model-test",
                200,
                Option::<String>::None,
                1000,
                100,
                input_tokens,
                20,
                Option::<i64>::None,
                10,
                0,
                0,
                0,
                Option::<i64>::None,
                Option::<String>::None,
                0,
                created_at
            ],
        )
        .expect("insert request log");
    }

    let summary =
        summary_query(&conn, None, None, None, Some(123), false).expect("filtered summary");
    assert_eq!(summary.requests_total, 1);
    assert_eq!(summary.requests_success, 1);
    assert_eq!(summary.cost_covered_success, 0);
    assert_eq!(summary.input_tokens, 110);

    let cli_rows = leaderboard_v2_with_conn(
        &conn,
        UsageScopeV2::Cli,
        None,
        None,
        None,
        Some(123),
        Some(50),
        false,
    )
    .expect("filtered cli leaderboard");
    assert_eq!(cli_rows.len(), 1);
    assert_eq!(cli_rows[0].key, "codex");
    assert_eq!(cli_rows[0].requests_total, 1);

    let cache_rows = provider_cache_rate_trend_v1_with_conn(
        &conn,
        ProviderCacheRateTrendQuery {
            start_ts: None,
            end_ts: None,
            cli_key: None,
            provider_id: Some(123),
            limit: None,
            exclude_cx2cc_gateway_bridge: false,
        },
    )
    .expect("filtered cache trend");
    assert_eq!(cache_rows.len(), 1);
    assert_eq!(cache_rows[0].key, "codex:123");
}

#[test]
fn v2_day_leaderboard_groups_by_local_day_and_applies_filters() {
    let conn = setup_conn();

    for (provider_id, provider_name) in [(123, "OpenAI"), (456, "Gemini Upstream")] {
        conn.execute(
            "INSERT INTO providers (id, name) VALUES (?1, ?2)",
            params![provider_id, provider_name],
        )
        .expect("insert provider");
    }

    let day_one_ts = 1_704_108_800i64;
    let day_two_ts = day_one_ts + 86_400;
    let end_ts = day_one_ts + 172_800;

    for (
        cli_key,
        provider_id,
        provider_name,
        requested_model,
        input_tokens,
        output_tokens,
        cache_read_input_tokens,
        cost_usd_femto,
        created_at,
    ) in [
        (
            "codex",
            123,
            "OpenAI",
            "gpt-test",
            100i64,
            50i64,
            10i64,
            1_000_000_000_000_000i64,
            day_one_ts,
        ),
        (
            "codex",
            123,
            "OpenAI",
            "gpt-test",
            200i64,
            40i64,
            20i64,
            2_000_000_000_000_000i64,
            day_one_ts + 3600,
        ),
        (
            "gemini",
            456,
            "Gemini Upstream",
            "gemini-test",
            300i64,
            30i64,
            30i64,
            3_000_000_000_000_000i64,
            day_two_ts,
        ),
        (
            "codex",
            123,
            "OpenAI",
            "gpt-test",
            999i64,
            1i64,
            0i64,
            4_000_000_000_000_000i64,
            end_ts,
        ),
    ] {
        let attempts_json = format!(
            r#"[{{"provider_id":{provider_id},"provider_name":"{provider_name}","outcome":"success"}}]"#
        );

        conn.execute(
            r#"
INSERT INTO usage_events (
  cli_key,
  attempts_json,
  final_provider_id,
  requested_model,
  status,
  error_code,
  duration_ms,
  ttfb_ms,
  input_tokens,
  output_tokens,
  total_tokens,
  cache_read_input_tokens,
  cache_creation_input_tokens,
  cache_creation_5m_input_tokens,
  cache_creation_1h_input_tokens,
  cost_usd_femto,
  usage_json,
  excluded_from_stats,
  created_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19);
            "#,
            params![
                cli_key,
                attempts_json,
                provider_id,
                requested_model,
                200,
                Option::<String>::None,
                1000,
                100,
                input_tokens,
                output_tokens,
                Option::<i64>::None,
                cache_read_input_tokens,
                0,
                0,
                0,
                cost_usd_femto,
                Option::<String>::None,
                0,
                created_at
            ],
        )
        .expect("insert request log");
    }

    let day_one = local_day_key(&conn, day_one_ts);
    let day_two = local_day_key(&conn, day_two_ts);

    let rows = leaderboard_v2_with_conn(
        &conn,
        UsageScopeV2::Day,
        Some(day_one_ts),
        Some(end_ts),
        None,
        None,
        Some(50),
        false,
    )
    .expect("day leaderboard");

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].key, day_two);
    assert_eq!(rows[0].name, day_two);
    assert_eq!(rows[0].requests_total, 1);
    assert_eq!(rows[0].input_tokens, 270);
    assert_eq!(rows[0].output_tokens, 30);
    assert_eq!(rows[0].total_tokens, 330);
    assert_eq!(rows[0].cost_usd, Some(3.0));
    assert_eq!(rows[0].first_request_created_at_ms, Some(day_two_ts * 1000));
    assert_eq!(rows[0].last_request_created_at_ms, Some(day_two_ts * 1000));

    assert_eq!(rows[1].key, day_one);
    assert_eq!(rows[1].name, day_one);
    assert_eq!(rows[1].requests_total, 2);
    assert_eq!(rows[1].input_tokens, 270);
    assert_eq!(rows[1].output_tokens, 90);
    assert_eq!(rows[1].total_tokens, 390);
    assert_eq!(rows[1].cost_usd, Some(3.0));
    assert_eq!(rows[1].first_request_created_at_ms, Some(day_one_ts * 1000));
    assert_eq!(
        rows[1].last_request_created_at_ms,
        Some((day_one_ts + 3600) * 1000)
    );

    let cli_filtered = leaderboard_v2_with_conn(
        &conn,
        UsageScopeV2::Day,
        Some(day_one_ts),
        Some(end_ts),
        Some("codex"),
        None,
        Some(50),
        false,
    )
    .expect("day leaderboard cli filter");
    assert_eq!(cli_filtered.len(), 1);
    assert_eq!(cli_filtered[0].key, day_one);
    assert_eq!(cli_filtered[0].requests_total, 2);
    assert_eq!(
        cli_filtered[0].last_request_created_at_ms,
        Some((day_one_ts + 3600) * 1000)
    );

    let provider_filtered = leaderboard_v2_with_conn(
        &conn,
        UsageScopeV2::Day,
        Some(day_one_ts),
        Some(end_ts),
        None,
        Some(456),
        Some(50),
        false,
    )
    .expect("day leaderboard provider filter");
    assert_eq!(provider_filtered.len(), 1);
    assert_eq!(provider_filtered[0].key, day_two);
    assert_eq!(provider_filtered[0].requests_total, 1);

    let model_rows = leaderboard_v2_with_conn(
        &conn,
        UsageScopeV2::Model,
        Some(day_one_ts),
        Some(end_ts),
        None,
        None,
        Some(50),
        false,
    )
    .expect("model leaderboard");
    assert!(model_rows
        .iter()
        .all(|row| row.first_request_created_at_ms.is_none()
            && row.last_request_created_at_ms.is_none()));
}

#[test]
fn v2_day_leaderboard_respects_usage_day_start_hour() {
    let conn = setup_conn();
    let day_one = "2026-04-16";
    let day_two = "2026-04-17";
    let day_start_hour = 5;
    let usage_day_one_start = local_usage_day_start_ts(&conn, day_one, day_start_hour);
    let usage_day_two_start = local_usage_day_start_ts(&conn, day_two, day_start_hour);
    let query_end = local_usage_day_start_ts(&conn, "2026-04-18", day_start_hour);

    for (provider_id, provider_name) in [(123, "OpenAI"), (456, "Gemini Upstream")] {
        conn.execute(
            "INSERT INTO providers (id, name) VALUES (?1, ?2)",
            params![provider_id, provider_name],
        )
        .expect("insert provider");
    }

    for (cli_key, provider_id, provider_name, session_id, created_at, input_tokens) in [
        (
            "codex",
            123,
            "OpenAI",
            "codex-alpha-1",
            usage_day_one_start + 4 * 3600,
            100i64,
        ),
        (
            "codex",
            123,
            "OpenAI",
            "codex-alpha-2",
            usage_day_one_start + 21 * 3600,
            200i64,
        ),
        (
            "codex",
            456,
            "Gemini Upstream",
            "codex-beta-1",
            usage_day_two_start + 4 * 3600,
            300i64,
        ),
        (
            "claude",
            123,
            "OpenAI",
            "claude-alpha-1",
            usage_day_two_start + 15 * 3600,
            400i64,
        ),
    ] {
        insert_usage_log(
            &conn,
            TestUsageLog {
                cli_key,
                provider_id,
                provider_name,
                requested_model: "model-test",
                input_tokens: Some(input_tokens),
                output_tokens: Some(10),
                session_id: Some(session_id),
                created_at,
                ..base_usage_log(created_at)
            },
        );
    }

    let usage_day_rows = leaderboard_v2_with_conn_day_start(
        &conn,
        UsageScopeV2::Day,
        Some(usage_day_one_start),
        Some(query_end),
        None,
        None,
        Some(50),
        false,
        day_start_hour,
    )
    .expect("usage day leaderboard");
    assert_eq!(usage_day_rows.len(), 2);
    assert_eq!(usage_day_rows[0].key, day_two);
    assert_eq!(usage_day_rows[0].requests_total, 2);
    assert_eq!(
        usage_day_rows[0].first_request_created_at_ms,
        Some((usage_day_two_start + 4 * 3600) * 1000)
    );
    assert_eq!(
        usage_day_rows[0].last_request_created_at_ms,
        Some((usage_day_two_start + 15 * 3600) * 1000)
    );
    assert_eq!(usage_day_rows[1].key, day_one);
    assert_eq!(usage_day_rows[1].requests_total, 2);
    assert_eq!(
        usage_day_rows[1].first_request_created_at_ms,
        Some((usage_day_one_start + 4 * 3600) * 1000)
    );
    assert_eq!(
        usage_day_rows[1].last_request_created_at_ms,
        Some((usage_day_one_start + 21 * 3600) * 1000)
    );

    let natural_rows = leaderboard_v2_with_conn(
        &conn,
        UsageScopeV2::Day,
        Some(usage_day_one_start),
        Some(query_end),
        None,
        None,
        Some(50),
        false,
    )
    .expect("natural day leaderboard");
    assert_eq!(natural_rows.len(), 2);
    assert_eq!(natural_rows[0].key, day_two);
    assert_eq!(natural_rows[0].requests_total, 3);
    assert_eq!(
        natural_rows[0].first_request_created_at_ms,
        Some((usage_day_one_start + 21 * 3600) * 1000)
    );
    assert_eq!(
        natural_rows[0].last_request_created_at_ms,
        Some((usage_day_two_start + 15 * 3600) * 1000)
    );
    assert_eq!(natural_rows[1].key, day_one);
    assert_eq!(natural_rows[1].requests_total, 1);

    let folder_rows = leaderboard_v2_folder_filtered_with_conn(
        &conn,
        FolderFilteredLeaderboardParams {
            scope: UsageScopeV2::Day,
            start_ts: Some(usage_day_one_start),
            end_ts: Some(query_end),
            cli_key: None,
            provider_id: None,
            folder_keys: &["/work/alpha".to_string()],
            limit: Some(50),
            exclude_cx2cc_gateway_bridge: false,
            day_start_hour,
        },
        fixture_folder_lookup,
    )
    .expect("folder filtered usage day leaderboard");
    assert_eq!(folder_rows.len(), 2);
    assert_eq!(folder_rows[0].key, day_two);
    assert_eq!(folder_rows[0].requests_total, 1);
    assert_eq!(folder_rows[1].key, day_one);
    assert_eq!(folder_rows[1].requests_total, 2);

    let day_one_detail = day_detail_v1_with_conn(
        &conn,
        &UsageDayDetailParams {
            day: day_one.to_string(),
            cli_key: None,
            provider_id: None,
            folder_limit: None,
            folder_keys: Some(vec!["/work/alpha".to_string()]),
            day_start_hour: Some(day_start_hour),
            exclude_cx2cc_gateway_bridge: None,
        },
        fixture_folder_lookup,
    )
    .expect("usage day detail");
    assert_eq!(
        day_one_detail
            .hours
            .iter()
            .map(|row| row.requests_total)
            .sum::<i64>(),
        2
    );
    assert_eq!(day_one_detail.hours[2].requests_total, 1);
    assert_eq!(day_one_detail.hours[9].requests_total, 1);
    assert_eq!(day_one_detail.folders.len(), 1);
    assert_eq!(day_one_detail.folders[0].key, "/work/alpha");
    assert_eq!(day_one_detail.folders[0].requests_total, 2);
}

#[test]
fn day_detail_v1_filters_by_local_day_and_returns_hour_buckets() {
    let conn = setup_conn();
    let day = "2026-04-16";
    let start_ts = local_day_start_ts(&conn, day);

    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "codex",
            provider_id: 123,
            provider_name: "OpenAI",
            input_tokens: Some(120),
            output_tokens: Some(30),
            cache_read_input_tokens: Some(20),
            cache_creation_input_tokens: Some(10),
            session_id: Some("codex-hour-2"),
            created_at: start_ts + 2 * 3600,
            ..base_usage_log(start_ts)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "claude",
            provider_id: 123,
            provider_name: "OpenAI",
            input_tokens: Some(50),
            output_tokens: Some(10),
            cache_read_input_tokens: Some(5),
            cache_creation_input_tokens: Some(5),
            session_id: Some("claude-hour-2"),
            created_at: start_ts + 2 * 3600,
            ..base_usage_log(start_ts)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "codex",
            provider_id: 456,
            provider_name: "Gemini Upstream",
            input_tokens: Some(70),
            output_tokens: Some(20),
            cache_read_input_tokens: Some(10),
            session_id: Some("codex-hour-5"),
            created_at: start_ts + 5 * 3600,
            ..base_usage_log(start_ts)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            input_tokens: Some(999),
            output_tokens: Some(1),
            created_at: start_ts - 1,
            ..base_usage_log(start_ts)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            input_tokens: Some(999),
            output_tokens: Some(1),
            created_at: start_ts + 86_400,
            ..base_usage_log(start_ts)
        },
    );

    let detail = day_detail_v1_with_conn(
        &conn,
        &UsageDayDetailParams {
            day: day.to_string(),
            cli_key: None,
            provider_id: None,
            folder_limit: None,
            folder_keys: None,
            day_start_hour: None,
            exclude_cx2cc_gateway_bridge: None,
        },
        |_| Vec::new(),
    )
    .expect("day detail");

    assert_eq!(detail.day, day);
    assert_eq!(detail.hours.len(), 24);
    assert_eq!(detail.hours[0].hour, 0);
    assert_eq!(detail.hours[0].requests_total, 0);
    assert_eq!(detail.hours[0].total_tokens, 0);
    assert_eq!(detail.hours[2].requests_total, 2);
    assert_eq!(detail.hours[2].total_tokens, 220);
    assert_eq!(detail.hours[2].io_total_tokens, 180);
    assert_eq!(detail.hours[5].requests_total, 1);
    assert_eq!(detail.hours[5].total_tokens, 90);
    assert_eq!(detail.hours[23].hour, 23);
    assert_eq!(detail.folders.len(), 1);
    assert_eq!(detail.folders[0].name, "未知文件夹");
    assert_eq!(detail.folders[0].requests_total, 3);
    assert_eq!(detail.folders[0].total_tokens, 310);

    let provider_filtered = day_detail_v1_with_conn(
        &conn,
        &UsageDayDetailParams {
            day: day.to_string(),
            cli_key: None,
            provider_id: Some(456),
            folder_limit: None,
            folder_keys: None,
            day_start_hour: None,
            exclude_cx2cc_gateway_bridge: None,
        },
        |_| Vec::new(),
    )
    .expect("provider filtered day detail");
    assert_eq!(provider_filtered.hours[2].requests_total, 0);
    assert_eq!(provider_filtered.hours[5].requests_total, 1);
    assert_eq!(provider_filtered.hours[5].total_tokens, 90);
    assert_eq!(provider_filtered.folders[0].requests_total, 1);

    let cli_filtered = day_detail_v1_with_conn(
        &conn,
        &UsageDayDetailParams {
            day: day.to_string(),
            cli_key: Some("claude".to_string()),
            provider_id: None,
            folder_limit: None,
            folder_keys: None,
            day_start_hour: None,
            exclude_cx2cc_gateway_bridge: None,
        },
        |_| Vec::new(),
    )
    .expect("cli filtered day detail");
    assert_eq!(cli_filtered.hours[2].requests_total, 1);
    assert_eq!(cli_filtered.hours[2].total_tokens, 70);
    assert_eq!(cli_filtered.hours[5].requests_total, 0);
    assert_eq!(cli_filtered.folders[0].input_tokens, 50);
}

#[test]
fn day_detail_v1_groups_resolved_folders_and_unknown_sessions() {
    let conn = setup_conn();
    let day = "2026-04-17";
    let start_ts = local_day_start_ts(&conn, day);

    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "codex",
            input_tokens: Some(120),
            output_tokens: Some(30),
            cache_read_input_tokens: Some(20),
            cache_creation_input_tokens: Some(10),
            session_id: Some("codex-s1"),
            created_at: start_ts + 3600,
            ..base_usage_log(start_ts)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "codex",
            input_tokens: Some(80),
            output_tokens: Some(20),
            session_id: Some("codex-s2"),
            created_at: start_ts + 2 * 3600,
            ..base_usage_log(start_ts)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "claude",
            status: Some(500),
            input_tokens: Some(60),
            output_tokens: Some(10),
            session_id: Some("claude-s3"),
            created_at: start_ts + 3 * 3600,
            ..base_usage_log(start_ts)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "codex",
            input_tokens: Some(40),
            output_tokens: Some(5),
            session_id: Some("missing-folder"),
            created_at: start_ts + 4 * 3600,
            ..base_usage_log(start_ts)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "gemini",
            input_tokens: Some(50),
            output_tokens: Some(5),
            cache_read_input_tokens: Some(10),
            session_id: Some("gemini-s1"),
            created_at: start_ts + 5 * 3600,
            ..base_usage_log(start_ts)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "claude",
            input_tokens: Some(30),
            output_tokens: Some(5),
            session_id: None,
            created_at: start_ts + 6 * 3600,
            ..base_usage_log(start_ts)
        },
    );

    let detail = day_detail_v1_with_conn(
        &conn,
        &UsageDayDetailParams {
            day: day.to_string(),
            cli_key: None,
            provider_id: None,
            folder_limit: None,
            folder_keys: None,
            day_start_hour: None,
            exclude_cx2cc_gateway_bridge: None,
        },
        |keys| {
            let mut pairs: Vec<String> = keys
                .iter()
                .map(|key| format!("{}:{}", key.cli_key, key.session_id))
                .collect();
            pairs.sort();
            assert_eq!(
                pairs,
                vec![
                    "claude:claude-s3".to_string(),
                    "codex:codex-s1".to_string(),
                    "codex:codex-s2".to_string(),
                    "codex:missing-folder".to_string(),
                ]
            );

            vec![
                UsageDayResolvedFolder {
                    cli_key: "codex".to_string(),
                    session_id: "codex-s1".to_string(),
                    folder_name: "alpha".to_string(),
                    folder_path: "/work/alpha".to_string(),
                },
                UsageDayResolvedFolder {
                    cli_key: "codex".to_string(),
                    session_id: "codex-s2".to_string(),
                    folder_name: "alpha".to_string(),
                    folder_path: "/work/alpha".to_string(),
                },
                UsageDayResolvedFolder {
                    cli_key: "claude".to_string(),
                    session_id: "claude-s3".to_string(),
                    folder_name: "beta".to_string(),
                    folder_path: "/work/beta".to_string(),
                },
            ]
        },
    )
    .expect("day detail");

    let by_key: std::collections::HashMap<String, UsageDayFolderRow> = detail
        .folders
        .into_iter()
        .map(|row| (row.key.clone(), row))
        .collect();

    let alpha = by_key.get("/work/alpha").expect("alpha folder row");
    assert_eq!(alpha.name, "alpha");
    assert_eq!(alpha.folder_path.as_deref(), Some("/work/alpha"));
    assert_eq!(alpha.requests_total, 2);
    assert_eq!(alpha.requests_success, 2);
    assert_eq!(alpha.total_tokens, 250);
    assert_eq!(alpha.io_total_tokens, 220);

    let beta = by_key.get("/work/beta").expect("beta folder row");
    assert_eq!(beta.name, "beta");
    assert_eq!(beta.requests_total, 1);
    assert_eq!(beta.requests_success, 0);
    assert_eq!(beta.requests_failed, 1);
    assert_eq!(beta.total_tokens, 70);

    let unknown = by_key.get("__unknown__").expect("unknown folder row");
    assert_eq!(unknown.name, "未知文件夹");
    assert_eq!(unknown.folder_path, None);
    assert_eq!(unknown.requests_total, 3);
    assert_eq!(unknown.total_tokens, 135);
}

#[test]
fn folder_options_v1_groups_resolved_folders_and_keeps_unknown_selectable() {
    let conn = setup_conn();
    let day = "2026-04-18";
    let start_ts = local_day_start_ts(&conn, day);

    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "codex",
            input_tokens: Some(100),
            output_tokens: Some(20),
            session_id: Some("codex-alpha-1"),
            created_at: start_ts + 60,
            ..base_usage_log(start_ts)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "codex",
            input_tokens: Some(40),
            output_tokens: Some(10),
            session_id: Some("codex-alpha-2"),
            created_at: start_ts + 120,
            ..base_usage_log(start_ts)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "gemini",
            input_tokens: Some(30),
            output_tokens: Some(5),
            session_id: Some("gemini-unknown"),
            created_at: start_ts + 180,
            ..base_usage_log(start_ts)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "codex",
            input_tokens: Some(20),
            output_tokens: Some(5),
            session_id: Some("missing-folder"),
            created_at: start_ts + 240,
            ..base_usage_log(start_ts)
        },
    );

    let options = folder_options_v1_with_conn(
        &conn,
        &UsageQueryParams {
            period: "custom".to_string(),
            start_ts: Some(start_ts),
            end_ts: Some(start_ts + 86_400),
            cli_key: None,
            provider_id: None,
            folder_keys: Some(vec!["/work/alpha".to_string()]),
            day_start_hour: None,
            exclude_cx2cc_gateway_bridge: None,
        },
        fixture_folder_lookup,
    )
    .expect("folder options");

    assert_eq!(options.len(), 2);
    assert_eq!(options[0].key, "/work/alpha");
    assert_eq!(options[0].name, "alpha");
    assert_eq!(options[0].folder_path.as_deref(), Some("/work/alpha"));
    assert_eq!(options[0].requests_total, 2);
    assert_eq!(options[0].total_tokens, 170);
    assert_eq!(options[1].key, "__unknown__");
    assert_eq!(options[1].name, "未知文件夹");
    assert_eq!(options[1].folder_path, None);
    assert_eq!(options[1].requests_total, 2);
    assert_eq!(options[1].total_tokens, 60);
}

#[test]
fn folder_keys_filter_summary_leaderboard_and_day_detail() {
    let conn = setup_conn();
    let day = "2026-04-19";
    let start_ts = local_day_start_ts(&conn, day);

    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "codex",
            provider_id: 123,
            requested_model: "gpt-alpha",
            input_tokens: Some(100),
            output_tokens: Some(20),
            cost_usd_femto: Some(0),
            session_id: Some("codex-alpha-1"),
            created_at: start_ts + 2 * 3600,
            ..base_usage_log(start_ts)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "codex",
            provider_id: 456,
            requested_model: "gpt-beta",
            input_tokens: Some(70),
            output_tokens: Some(10),
            session_id: Some("codex-beta-1"),
            created_at: start_ts + 5 * 3600,
            ..base_usage_log(start_ts)
        },
    );
    insert_usage_log(
        &conn,
        TestUsageLog {
            cli_key: "gemini",
            provider_id: 456,
            requested_model: "gemini-unknown",
            input_tokens: Some(30),
            output_tokens: Some(5),
            session_id: Some("gemini-unknown"),
            created_at: start_ts + 7 * 3600,
            ..base_usage_log(start_ts)
        },
    );

    let alpha_params = UsageQueryParams {
        period: "custom".to_string(),
        start_ts: Some(start_ts),
        end_ts: Some(start_ts + 86_400),
        cli_key: None,
        provider_id: None,
        folder_keys: Some(vec!["/work/alpha".to_string()]),
        day_start_hour: None,
        exclude_cx2cc_gateway_bridge: None,
    };
    let unfiltered_summary = summary_v2_with_conn(
        &conn,
        &UsageQueryParams {
            folder_keys: None,
            ..alpha_params.clone()
        },
        fixture_folder_lookup,
    )
    .expect("unfiltered summary");
    assert_eq!(unfiltered_summary.cost_covered_success, 1);

    let alpha_summary =
        summary_v2_with_conn(&conn, &alpha_params, fixture_folder_lookup).expect("summary");
    assert_eq!(alpha_summary.requests_total, 1);
    assert_eq!(alpha_summary.cost_covered_success, 1);
    assert_eq!(alpha_summary.total_tokens, 120);
    assert_eq!(alpha_summary.io_total_tokens, 120);

    let alpha_day_rows = leaderboard_v2_folder_filtered_with_conn(
        &conn,
        FolderFilteredLeaderboardParams {
            scope: UsageScopeV2::Day,
            start_ts: Some(start_ts),
            end_ts: Some(start_ts + 86_400),
            cli_key: None,
            provider_id: None,
            folder_keys: &["/work/alpha".to_string()],
            limit: Some(50),
            exclude_cx2cc_gateway_bridge: false,
            day_start_hour: 0,
        },
        fixture_folder_lookup,
    )
    .expect("day leaderboard");
    assert_eq!(alpha_day_rows.len(), 1);
    assert_eq!(alpha_day_rows[0].key, day);
    assert_eq!(alpha_day_rows[0].total_tokens, 120);
    assert_eq!(
        alpha_day_rows[0].first_request_created_at_ms,
        Some((start_ts + 2 * 3600) * 1000)
    );
    assert_eq!(
        alpha_day_rows[0].last_request_created_at_ms,
        Some((start_ts + 2 * 3600) * 1000)
    );

    let alpha_model_rows = leaderboard_v2_folder_filtered_with_conn(
        &conn,
        FolderFilteredLeaderboardParams {
            scope: UsageScopeV2::Model,
            start_ts: Some(start_ts),
            end_ts: Some(start_ts + 86_400),
            cli_key: None,
            provider_id: None,
            folder_keys: &["/work/alpha".to_string()],
            limit: Some(50),
            exclude_cx2cc_gateway_bridge: false,
            day_start_hour: 0,
        },
        fixture_folder_lookup,
    )
    .expect("model leaderboard");
    assert_eq!(alpha_model_rows.len(), 1);
    assert_eq!(alpha_model_rows[0].key, "gpt-alpha");
    assert_eq!(alpha_model_rows[0].first_request_created_at_ms, None);
    assert_eq!(alpha_model_rows[0].last_request_created_at_ms, None);

    let unknown_summary = summary_v2_with_conn(
        &conn,
        &UsageQueryParams {
            folder_keys: Some(vec!["__unknown__".to_string()]),
            exclude_cx2cc_gateway_bridge: None,
            ..alpha_params.clone()
        },
        fixture_folder_lookup,
    )
    .expect("unknown summary");
    assert_eq!(unknown_summary.requests_total, 1);
    assert_eq!(unknown_summary.total_tokens, 35);

    let provider_filtered = summary_v2_with_conn(
        &conn,
        &UsageQueryParams {
            provider_id: Some(456),
            folder_keys: Some(vec!["/work/beta".to_string()]),
            exclude_cx2cc_gateway_bridge: None,
            ..alpha_params
        },
        fixture_folder_lookup,
    )
    .expect("provider plus folder summary");
    assert_eq!(provider_filtered.requests_total, 1);
    assert_eq!(provider_filtered.total_tokens, 80);

    let detail = day_detail_v1_with_conn(
        &conn,
        &UsageDayDetailParams {
            day: day.to_string(),
            cli_key: None,
            provider_id: None,
            folder_limit: None,
            folder_keys: Some(vec!["/work/alpha".to_string()]),
            day_start_hour: None,
            exclude_cx2cc_gateway_bridge: None,
        },
        fixture_folder_lookup,
    )
    .expect("day detail");
    assert_eq!(detail.hours.len(), 24);
    assert_eq!(detail.hours[2].requests_total, 1);
    assert_eq!(detail.hours[2].total_tokens, 120);
    assert_eq!(detail.hours[5].requests_total, 0);
    assert_eq!(detail.folders.len(), 1);
    assert_eq!(detail.folders[0].key, "/work/alpha");
    assert_eq!(detail.folders[0].total_tokens, 120);
}
