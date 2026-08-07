use super::*;

#[test]
fn migrate_v38_to_v39_converts_valid_retry_overrides_and_preserves_malformed_rows() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.execute_batch(
        r#"
CREATE TABLE providers (
  id INTEGER PRIMARY KEY,
  upstream_retry_policy_json TEXT
);
PRAGMA user_version = 38;
"#,
    )
    .expect("create v38 fixture");
    let legacy = r#"{"enabled":true,"status_codes":[429,503],"transport_errors":["timeout"],"max_retries":2,"backoff_ms":25,"counts_toward_circuit_breaker":false}"#;
    let current = r#"{"enabled":false,"http_rules":[],"transport_errors":[],"max_retries":1,"backoff_ms":0,"counts_toward_circuit_breaker":false}"#;
    let malformed = "{not-json";
    for (id, value) in [(1_i64, legacy), (2, current), (3, malformed)] {
        conn.execute(
            "INSERT INTO providers(id, upstream_retry_policy_json) VALUES (?1, ?2)",
            rusqlite::params![id, value],
        )
        .expect("insert fixture row");
    }
    let non_text = vec![0xff_u8, 0x00, 0x7f];
    conn.execute(
        "INSERT INTO providers(id, upstream_retry_policy_json) VALUES (?1, ?2)",
        rusqlite::params![4_i64, non_text],
    )
    .expect("insert non-text fixture row");

    v38_to_v39::migrate_v38_to_v39(&mut conn).expect("migrate v38->v39");

    let version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read version");
    assert_eq!(version, 39);
    let migrated: String = conn
        .query_row(
            "SELECT upstream_retry_policy_json FROM providers WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .expect("read migrated row");
    let migrated: serde_json::Value = serde_json::from_str(&migrated).expect("parse migrated row");
    assert!(migrated.get("status_codes").is_none());
    assert_eq!(migrated["http_rules"][0]["status_code"], 429);
    assert_eq!(migrated["http_rules"][1]["status_code"], 503);
    assert_eq!(migrated["transport_errors"], serde_json::json!(["timeout"]));

    let unchanged_current: String = conn
        .query_row(
            "SELECT upstream_retry_policy_json FROM providers WHERE id = 2",
            [],
            |row| row.get(0),
        )
        .expect("read current row");
    let unchanged_malformed: String = conn
        .query_row(
            "SELECT upstream_retry_policy_json FROM providers WHERE id = 3",
            [],
            |row| row.get(0),
        )
        .expect("read malformed row");
    let unchanged_non_text: Vec<u8> = conn
        .query_row(
            "SELECT upstream_retry_policy_json FROM providers WHERE id = 4",
            [],
            |row| row.get(0),
        )
        .expect("read non-text row");
    assert_eq!(unchanged_current, current);
    assert_eq!(unchanged_malformed, malformed);
    assert_eq!(unchanged_non_text, vec![0xff, 0x00, 0x7f]);
}

#[test]
fn migrate_v38_to_v39_adds_missing_retry_policy_column() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.execute_batch(
        r#"
CREATE TABLE providers (id INTEGER PRIMARY KEY);
PRAGMA user_version = 38;
"#,
    )
    .expect("create minimal v38 fixture");

    v38_to_v39::migrate_v38_to_v39(&mut conn).expect("migrate v38->v39");

    let column_count: i64 = conn
        .query_row(
            "SELECT COUNT(1) FROM pragma_table_info('providers') WHERE name = 'upstream_retry_policy_json'",
            [],
            |row| row.get(0),
        )
        .expect("inspect migrated columns");
    let version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read version");
    assert_eq!(column_count, 1);
    assert_eq!(version, 39);
}

#[test]
fn migrate_v32_to_v33_backfills_pool_and_default_route_orders() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.execute_batch(
        r#"
CREATE TABLE providers (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  cli_key TEXT NOT NULL,
  name TEXT NOT NULL,
  base_url TEXT NOT NULL,
  base_urls_json TEXT NOT NULL DEFAULT '[]',
  base_url_mode TEXT NOT NULL DEFAULT 'order',
  claude_models_json TEXT NOT NULL DEFAULT '{}',
  api_key_plaintext TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  priority INTEGER NOT NULL DEFAULT 100,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0,
  cost_multiplier REAL NOT NULL DEFAULT 1.0,
  supported_models_json TEXT NOT NULL DEFAULT '{}',
  model_mapping_json TEXT NOT NULL DEFAULT '{}',
  auth_mode TEXT NOT NULL DEFAULT 'api_key',
  oauth_provider_type TEXT,
  oauth_access_token TEXT,
  oauth_refresh_token TEXT,
  oauth_expires_at INTEGER,
  oauth_email TEXT,
  oauth_last_error TEXT,
  limit_5h_usd REAL,
  limit_daily_usd REAL,
  daily_reset_mode TEXT NOT NULL DEFAULT 'fixed',
  daily_reset_time TEXT NOT NULL DEFAULT '00:00:00',
  limit_weekly_usd REAL,
  limit_monthly_usd REAL,
  limit_total_usd REAL,
  tags_json TEXT NOT NULL DEFAULT '[]',
  note TEXT NOT NULL DEFAULT '',
  source_provider_id INTEGER,
  bridge_type TEXT,
  stream_idle_timeout_seconds INTEGER,
  upstream_retry_policy_json TEXT,
  UNIQUE(cli_key, name)
);
"#,
    )
    .expect("create providers table");

    for (id, name, enabled, sort_order) in [
        (1_i64, "p1", 1_i64, 0_i64),
        (2_i64, "p2", 0_i64, 1_i64),
        (3_i64, "p3", 1_i64, 2_i64),
    ] {
        conn.execute(
            r#"
INSERT INTO providers(
  id,
  cli_key,
  name,
  base_url,
  api_key_plaintext,
  enabled,
  created_at,
  updated_at,
  sort_order
) VALUES (?1, 'claude', ?2, 'https://example.com', 'sk', ?3, 1, 1, ?4)
"#,
            rusqlite::params![id, name, enabled, sort_order],
        )
        .expect("insert provider");
    }

    v32_to_v33::migrate_v32_to_v33(&mut conn).expect("migrate v32->v33");

    let pool_ids: Vec<i64> = {
        let mut stmt = conn
            .prepare("SELECT provider_id FROM provider_pool_order ORDER BY sort_order ASC")
            .expect("prepare pool");
        stmt.query_map([], |row| row.get(0))
            .expect("query pool")
            .map(|row| row.expect("pool row"))
            .collect()
    };
    assert_eq!(pool_ids, vec![1, 2, 3]);

    let default_ids: Vec<i64> = {
        let mut stmt = conn
            .prepare("SELECT provider_id FROM default_route_providers ORDER BY sort_order ASC")
            .expect("prepare default");
        stmt.query_map([], |row| row.get(0))
            .expect("query default")
            .map(|row| row.expect("default row"))
            .collect()
    };
    assert_eq!(default_ids, vec![1, 3]);
}

#[test]
fn ensure_patches_do_not_repopulate_default_route_members() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    apply_migrations(&mut conn).expect("apply migrations");

    for (id, name, sort_order) in [
        (1_i64, "p1", 0_i64),
        (2_i64, "p2", 1_i64),
        (3_i64, "p3", 2_i64),
    ] {
        let provider_uuid = crate::shared::uuid::new_uuid_v4();
        conn.execute(
            r#"
INSERT INTO providers(
  id,
  provider_uuid,
  cli_key,
  name,
  base_url,
  api_key_plaintext,
  enabled,
  created_at,
  updated_at,
  sort_order
) VALUES (?1, ?2, 'claude', ?3, 'https://example.com', 'sk', 1, 1, 1, ?4)
"#,
            rusqlite::params![id, provider_uuid, name, sort_order],
        )
        .expect("insert provider");
    }

    for (provider_id, sort_order) in [(1_i64, 0_i64), (2_i64, 1_i64), (3_i64, 2_i64)] {
        conn.execute(
            r#"
INSERT INTO default_route_providers(
  cli_key,
  provider_id,
  sort_order,
  created_at,
  updated_at
) VALUES ('claude', ?1, ?2, 1, 1)
"#,
            rusqlite::params![provider_id, sort_order],
        )
        .expect("insert default route provider");
    }
    conn.execute(
        "DELETE FROM default_route_providers WHERE cli_key = 'claude' AND provider_id = 2",
        [],
    )
    .expect("simulate removing provider from default route");

    ensure::apply_ensure_patches(&mut conn).expect("apply ensure patches");

    let default_ids: Vec<i64> = {
        let mut stmt = conn
            .prepare("SELECT provider_id FROM default_route_providers ORDER BY sort_order ASC")
            .expect("prepare default");
        stmt.query_map([], |row| row.get(0))
            .expect("query default")
            .map(|row| row.expect("default row"))
            .collect()
    };
    assert_eq!(default_ids, vec![1, 3]);

    let pool_ids: Vec<i64> = {
        let mut stmt = conn
            .prepare("SELECT provider_id FROM provider_pool_order ORDER BY sort_order ASC")
            .expect("prepare pool");
        stmt.query_map([], |row| row.get(0))
            .expect("query pool")
            .map(|row| row.expect("pool row"))
            .collect()
    };
    assert_eq!(pool_ids, vec![1, 2, 3]);
}

#[test]
fn migrate_v25_to_v26_backfills_claude_models_json_from_legacy_mapping() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");

    conn.execute_batch(
        r#"
CREATE TABLE providers (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  cli_key TEXT NOT NULL,
  name TEXT NOT NULL,
  base_url TEXT NOT NULL,
  base_urls_json TEXT NOT NULL DEFAULT '[]',
  base_url_mode TEXT NOT NULL DEFAULT 'order',
  api_key_plaintext TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  priority INTEGER NOT NULL DEFAULT 100,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0,
  cost_multiplier REAL NOT NULL DEFAULT 1.0,
  supported_models_json TEXT NOT NULL DEFAULT '{}',
  model_mapping_json TEXT NOT NULL DEFAULT '{}',
  UNIQUE(cli_key, name)
);
"#,
    )
    .expect("create providers table");

    let legacy_mapping = serde_json::json!({
        "*": "glm-4-plus",
        "claude-*sonnet*": "glm-4-plus-sonnet",
        "claude-*haiku*": "glm-4-plus-haiku",
        "claude-*thinking*": "glm-4-plus-thinking"
    })
    .to_string();

    conn.execute(
        r#"
INSERT INTO providers(
  cli_key,
  name,
  base_url,
  base_urls_json,
  base_url_mode,
  api_key_plaintext,
  enabled,
  priority,
  created_at,
  updated_at,
  sort_order,
  cost_multiplier,
  supported_models_json,
  model_mapping_json
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, 100, 1, 1, 0, 1.0, '{}', ?7)
"#,
        rusqlite::params![
            "claude",
            "legacy",
            "https://example.com",
            "[]",
            "order",
            "sk-test",
            legacy_mapping
        ],
    )
    .expect("insert legacy provider");

    v25_to_v26::migrate_v25_to_v26(&mut conn).expect("migrate v25->v26");

    let claude_models_json: String = conn
        .query_row(
            "SELECT claude_models_json FROM providers WHERE name = 'legacy'",
            [],
            |row| row.get(0),
        )
        .expect("read claude_models_json");

    let value: serde_json::Value =
        serde_json::from_str(&claude_models_json).expect("claude_models_json valid json");

    assert_eq!(value["main_model"], "glm-4-plus");
    assert_eq!(value["sonnet_model"], "glm-4-plus-sonnet");
    assert_eq!(value["haiku_model"], "glm-4-plus-haiku");
    assert_eq!(value["reasoning_model"], "glm-4-plus-thinking");

    let supported_models_json: String = conn
        .query_row(
            "SELECT supported_models_json FROM providers WHERE name = 'legacy'",
            [],
            |row| row.get(0),
        )
        .expect("read supported_models_json");
    assert_eq!(supported_models_json.trim(), "{}");

    let model_mapping_json: String = conn
        .query_row(
            "SELECT model_mapping_json FROM providers WHERE name = 'legacy'",
            [],
            |row| row.get(0),
        )
        .expect("read model_mapping_json");
    assert_eq!(model_mapping_json.trim(), "{}");
}

#[test]
fn ensure_plugin_tables_is_idempotent() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    apply_migrations(&mut conn).expect("apply migrations once");
    apply_migrations(&mut conn).expect("apply migrations twice");

    for table in [
        "plugins",
        "plugin_versions",
        "plugin_configs",
        "plugin_permissions",
        "plugin_audit_logs",
        "plugin_market_sources",
        "plugin_runtime_failures",
        "plugin_hook_execution_reports",
    ] {
        assert!(
            test_has_table(&conn, table),
            "missing plugin table after ensure patches: {table}"
        );
    }

    assert!(test_has_column(&conn, "plugins", "plugin_id"));
    assert!(test_has_column(&conn, "plugins", "current_version"));
    assert!(test_has_column(&conn, "plugins", "status"));
    assert!(test_has_column(&conn, "plugins", "manifest_json"));
    assert!(test_has_column(&conn, "plugins", "last_error"));
    assert!(test_has_column(&conn, "plugin_configs", "config_json"));
    assert!(test_has_column(
        &conn,
        "plugin_permissions",
        "permissions_json"
    ));
    assert!(test_has_index(
        &conn,
        "idx_plugin_hook_execution_reports_created_at"
    ));
}

#[test]
fn migrations_create_provider_extension_values_table() {
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    apply_migrations(&mut conn).expect("apply migrations");

    assert!(test_has_table(&conn, "provider_extension_values"));
    assert!(test_has_column(
        &conn,
        "provider_extension_values",
        "provider_id"
    ));
    assert!(test_has_column(
        &conn,
        "provider_extension_values",
        "plugin_id"
    ));
    assert!(test_has_column(
        &conn,
        "provider_extension_values",
        "namespace"
    ));
    assert!(test_has_column(
        &conn,
        "provider_extension_values",
        "values_json"
    ));
}

#[test]
fn ensure_patch_drops_legacy_request_attempt_logs_table() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    apply_migrations(&mut conn).expect("create current schema");

    conn.execute_batch(
        r#"
CREATE TABLE request_attempt_logs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  trace_id TEXT NOT NULL,
  provider_id INTEGER NOT NULL
);
"#,
    )
    .expect("create legacy request_attempt_logs table");

    assert!(test_has_table(&conn, "request_attempt_logs"));

    apply_migrations(&mut conn).expect("apply migrations");

    assert!(!test_has_table(&conn, "request_attempt_logs"));

    apply_migrations(&mut conn).expect("apply migrations twice");
    assert!(!test_has_table(&conn, "request_attempt_logs"));
}

#[test]
fn ensure_patch_adds_reset_credit_count_to_existing_oauth_snapshot_table() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.execute_batch(
        r#"
CREATE TABLE providers (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  cli_key TEXT NOT NULL,
  name TEXT NOT NULL,
  base_url TEXT NOT NULL,
  base_urls_json TEXT NOT NULL DEFAULT '[]',
  base_url_mode TEXT NOT NULL DEFAULT 'order',
  claude_models_json TEXT NOT NULL DEFAULT '{}',
  api_key_plaintext TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  priority INTEGER NOT NULL DEFAULT 100,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0,
  cost_multiplier REAL NOT NULL DEFAULT 1.0,
  supported_models_json TEXT NOT NULL DEFAULT '{}',
  model_mapping_json TEXT NOT NULL DEFAULT '{}',
  UNIQUE(cli_key, name)
);

CREATE TABLE provider_oauth_limit_snapshots (
  provider_id INTEGER PRIMARY KEY,
  limit_short_label TEXT,
  limit_5h_text TEXT,
  limit_weekly_text TEXT,
  limit_5h_reset_at INTEGER,
  limit_weekly_reset_at INTEGER,
  checked_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  FOREIGN KEY(provider_id) REFERENCES providers(id) ON DELETE CASCADE
);

CREATE TABLE prompts (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  workspace_id INTEGER NOT NULL,
  name TEXT NOT NULL,
  content TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

INSERT INTO providers(
  id,
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
  model_mapping_json
) VALUES (1, 'codex', 'legacy oauth', 'https://example.com', '[]', 'order', '{}', '', 1, 100, 1, 1, 0, 1.0, '{}', '{}');

INSERT INTO provider_oauth_limit_snapshots(
  provider_id,
  limit_short_label,
  limit_5h_text,
  limit_weekly_text,
  limit_5h_reset_at,
  limit_weekly_reset_at,
  checked_at,
  updated_at
) VALUES (1, '5h', '25%', '80%', 10, 20, 30, 30);

PRAGMA user_version = 32;
"#,
    )
    .expect("create legacy snapshot schema");

    assert!(!test_has_column(
        &conn,
        "provider_oauth_limit_snapshots",
        "reset_credit_available_count"
    ));

    apply_migrations(&mut conn).expect("apply migrations");

    assert!(test_has_column(
        &conn,
        "provider_oauth_limit_snapshots",
        "reset_credit_available_count"
    ));
    let row: (String, Option<i64>) = conn
        .query_row(
            "SELECT limit_5h_text, reset_credit_available_count FROM provider_oauth_limit_snapshots WHERE provider_id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read migrated snapshot");
    assert_eq!(row, ("25%".to_string(), None));

    apply_migrations(&mut conn).expect("apply migrations twice");
}

#[test]
fn migrate_v27_to_v28_drops_provider_mode_and_deletes_official_providers() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("enable foreign_keys");

    conn.execute_batch(
        r#"
CREATE TABLE providers (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  cli_key TEXT NOT NULL,
  name TEXT NOT NULL,
  base_url TEXT NOT NULL,
  base_urls_json TEXT NOT NULL DEFAULT '[]',
  base_url_mode TEXT NOT NULL DEFAULT 'order',
  claude_models_json TEXT NOT NULL DEFAULT '{}',
  api_key_plaintext TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  priority INTEGER NOT NULL DEFAULT 100,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0,
  cost_multiplier REAL NOT NULL DEFAULT 1.0,
  supported_models_json TEXT NOT NULL DEFAULT '{}',
  model_mapping_json TEXT NOT NULL DEFAULT '{}',
  provider_mode TEXT NOT NULL DEFAULT 'relay',
  UNIQUE(cli_key, name)
);

CREATE TABLE provider_circuit_breakers (
  provider_id INTEGER PRIMARY KEY,
  state TEXT NOT NULL,
  failure_count INTEGER NOT NULL DEFAULT 0,
  open_until INTEGER,
  updated_at INTEGER NOT NULL,
  FOREIGN KEY(provider_id) REFERENCES providers(id) ON DELETE CASCADE
);

CREATE TABLE sort_modes (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(name)
);

CREATE TABLE sort_mode_providers (
  mode_id INTEGER NOT NULL,
  cli_key TEXT NOT NULL,
  provider_id INTEGER NOT NULL,
  sort_order INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY(mode_id, cli_key, provider_id),
  FOREIGN KEY(mode_id) REFERENCES sort_modes(id) ON DELETE CASCADE,
  FOREIGN KEY(provider_id) REFERENCES providers(id) ON DELETE CASCADE
);

CREATE TABLE claude_model_validation_runs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  provider_id INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  request_json TEXT NOT NULL,
  result_json TEXT NOT NULL,
  FOREIGN KEY(provider_id) REFERENCES providers(id) ON DELETE CASCADE
);
"#,
    )
    .expect("create v27 schema");

    conn.execute(
        r#"
INSERT INTO providers(
  id,
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
  provider_mode
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
"#,
        rusqlite::params![
            1i64,
            "codex",
            "relay",
            "https://relay.example.com/v1",
            "[\"https://relay.example.com/v1\"]",
            "order",
            "{}",
            "sk-relay",
            1i64,
            100i64,
            1i64,
            1i64,
            0i64,
            1.0f64,
            "{}",
            "{}",
            "relay",
        ],
    )
    .expect("insert relay provider");

    conn.execute(
        r#"
INSERT INTO providers(
  id,
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
  provider_mode
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
"#,
        rusqlite::params![
            2i64,
            "codex",
            "official",
            "https://api.openai.com/v1",
            "[\"https://api.openai.com/v1\"]",
            "order",
            "{}",
            "",
            1i64,
            100i64,
            1i64,
            1i64,
            1i64,
            1.0f64,
            "{}",
            "{}",
            "official",
        ],
    )
    .expect("insert official provider");

    conn.execute(
            "INSERT INTO provider_circuit_breakers(provider_id, state, failure_count, open_until, updated_at) VALUES (?1, 'CLOSED', 0, NULL, 1)",
            rusqlite::params![1i64],
        )
        .expect("insert relay breaker");
    conn.execute(
            "INSERT INTO provider_circuit_breakers(provider_id, state, failure_count, open_until, updated_at) VALUES (?1, 'CLOSED', 0, NULL, 1)",
            rusqlite::params![2i64],
        )
        .expect("insert official breaker");

    conn.execute(
        "INSERT INTO sort_modes(id, name, created_at, updated_at) VALUES (1, 'mode', 1, 1)",
        [],
    )
    .expect("insert sort mode");
    conn.execute(
            "INSERT INTO sort_mode_providers(mode_id, cli_key, provider_id, sort_order, created_at, updated_at) VALUES (1, 'codex', 1, 0, 1, 1)",
            [],
        )
        .expect("insert relay sort_mode_provider");
    conn.execute(
            "INSERT INTO sort_mode_providers(mode_id, cli_key, provider_id, sort_order, created_at, updated_at) VALUES (1, 'codex', 2, 1, 1, 1)",
            [],
        )
        .expect("insert official sort_mode_provider");

    conn.execute(
            "INSERT INTO claude_model_validation_runs(id, provider_id, created_at, request_json, result_json) VALUES (1, 1, 1, '{}', '{}')",
            [],
        )
        .expect("insert relay validation run");
    conn.execute(
            "INSERT INTO claude_model_validation_runs(id, provider_id, created_at, request_json, result_json) VALUES (2, 2, 1, '{}', '{}')",
            [],
        )
        .expect("insert official validation run");

    v27_to_v28::migrate_v27_to_v28(&mut conn).expect("migrate v27->v28");

    let user_version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read user_version");
    assert_eq!(user_version, 28);

    let provider_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM providers", [], |row| row.get(0))
        .expect("count providers");
    assert_eq!(provider_count, 1);

    let breaker_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM provider_circuit_breakers",
            [],
            |row| row.get(0),
        )
        .expect("count breakers");
    assert_eq!(breaker_count, 1);

    let sort_mode_provider_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM sort_mode_providers", [], |row| {
            row.get(0)
        })
        .expect("count sort_mode_providers");
    assert_eq!(sort_mode_provider_count, 1);

    let validation_run_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM claude_model_validation_runs",
            [],
            |row| row.get(0),
        )
        .expect("count validation runs");
    assert_eq!(validation_run_count, 1);

    let remaining_name: String = conn
        .query_row("SELECT name FROM providers WHERE id = 1", [], |row| {
            row.get(0)
        })
        .expect("read remaining provider name");
    assert_eq!(remaining_name, "relay");

    let mut has_provider_mode = false;
    {
        let mut stmt = conn
            .prepare("PRAGMA table_info(providers)")
            .expect("prepare providers table_info query");
        let mut rows = stmt.query([]).expect("query providers table_info");
        while let Some(row) = rows.next().expect("read table_info row") {
            let name: String = row.get(1).expect("read column name");
            if name == "provider_mode" {
                has_provider_mode = true;
                break;
            }
        }
    }
    assert!(!has_provider_mode);
}

fn test_has_column(conn: &Connection, table: &str, column: &str) -> bool {
    let sql = format!("PRAGMA table_info({table})");
    let mut stmt = conn.prepare(&sql).expect("prepare table_info");
    let mut rows = stmt.query([]).expect("query table_info");
    while let Some(row) = rows.next().expect("read table_info row") {
        let name: String = row.get(1).expect("read column name");
        if name == column {
            return true;
        }
    }
    false
}

fn test_has_table(conn: &Connection, table: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1",
        [table],
        |_| Ok(true),
    )
    .unwrap_or(false)
}

fn test_has_view(conn: &Connection, view: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'view' AND name = ?1 LIMIT 1",
        [view],
        |_| Ok(true),
    )
    .unwrap_or(false)
}

fn test_has_index(conn: &Connection, index: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = ?1 LIMIT 1",
        [index],
        |_| Ok(true),
    )
    .unwrap_or(false)
}

fn test_has_trigger(conn: &Connection, trigger: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'trigger' AND name = ?1 LIMIT 1",
        [trigger],
        |_| Ok(true),
    )
    .unwrap_or(false)
}

fn assert_no_v40_provider_model_schema(conn: &Connection) {
    for table in [
        "provider_model_catalogs",
        "provider_models",
        "codex_managed_profiles",
    ] {
        assert!(!test_has_table(conn, table), "unexpected {table}");
    }
    assert!(!test_has_index(conn, "idx_providers_provider_uuid"));
    for trigger in [
        "providers_provider_uuid_insert_guard",
        "providers_provider_uuid_update_guard",
    ] {
        assert!(!test_has_trigger(conn, trigger), "unexpected {trigger}");
    }
}

#[test]
fn strict_v29_patch_adds_sort_mode_provider_enabled_column() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("enable foreign_keys");

    conn.execute_batch(
        r#"
CREATE TABLE prompts (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  cli_key TEXT NOT NULL,
  name TEXT NOT NULL,
  content TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(cli_key, name)
);

CREATE TABLE providers (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  cli_key TEXT NOT NULL,
  name TEXT NOT NULL,
  base_url TEXT NOT NULL,
  base_urls_json TEXT NOT NULL DEFAULT '[]',
  base_url_mode TEXT NOT NULL DEFAULT 'order',
  claude_models_json TEXT NOT NULL DEFAULT '{}',
  supported_models_json TEXT NOT NULL DEFAULT '{}',
  model_mapping_json TEXT NOT NULL DEFAULT '{}',
  api_key_plaintext TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  priority INTEGER NOT NULL DEFAULT 100,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0,
  cost_multiplier REAL NOT NULL DEFAULT 1.0,
  UNIQUE(cli_key, name)
);

CREATE TABLE sort_mode_providers (
  mode_id INTEGER NOT NULL,
  cli_key TEXT NOT NULL,
  provider_id INTEGER NOT NULL,
  sort_order INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY(mode_id, cli_key, provider_id)
);

PRAGMA user_version = 29;
"#,
    )
    .expect("create legacy sort_mode_providers schema");

    assert!(!test_has_column(&conn, "sort_mode_providers", "enabled"));

    apply_migrations(&mut conn).expect("apply migrations");
    assert!(test_has_column(&conn, "sort_mode_providers", "enabled"));

    // Idempotent: second run should succeed.
    apply_migrations(&mut conn).expect("apply migrations twice");
}

#[test]
fn ensure_patch_backfills_oauth_columns_for_legacy_v30_schema() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("enable foreign_keys");

    conn.execute_batch(
        r#"
CREATE TABLE providers (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  cli_key TEXT NOT NULL,
  name TEXT NOT NULL,
  base_url TEXT NOT NULL,
  base_urls_json TEXT NOT NULL DEFAULT '[]',
  base_url_mode TEXT NOT NULL DEFAULT 'order',
  claude_models_json TEXT NOT NULL DEFAULT '{}',
  api_key_plaintext TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  priority INTEGER NOT NULL DEFAULT 100,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0,
  cost_multiplier REAL NOT NULL DEFAULT 1.0
);

CREATE TABLE prompts (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  workspace_id INTEGER NOT NULL,
  name TEXT NOT NULL,
  content TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

PRAGMA user_version = 30;
"#,
    )
    .expect("create legacy v30 schema without oauth columns");

    conn.execute(
        r#"
INSERT INTO providers(
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
  cost_multiplier
) VALUES ('claude', 'legacy', 'https://example.com', '[]', 'order', '{}', 'sk-test', 1, 100, 1, 1, 0, 1.0)
"#,
        [],
    )
    .expect("insert legacy provider");

    apply_migrations(&mut conn).expect("apply migrations");

    let user_version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read user_version");
    assert_eq!(user_version, LATEST_SCHEMA_VERSION);

    for column in [
        "auth_mode",
        "oauth_provider_type",
        "oauth_access_token",
        "oauth_refresh_token",
        "oauth_id_token",
        "oauth_token_uri",
        "oauth_client_id",
        "oauth_client_secret",
        "oauth_expires_at",
        "oauth_email",
        "oauth_last_refreshed_at",
        "oauth_last_error",
        "oauth_refresh_lead_s",
    ] {
        assert!(test_has_column(&conn, "providers", column));
    }

    let (auth_mode, oauth_refresh_lead_s): (String, i64) = conn
        .query_row(
            "SELECT auth_mode, oauth_refresh_lead_s FROM providers WHERE name = 'legacy'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read oauth defaults");
    assert_eq!(auth_mode, "api_key");
    assert_eq!(oauth_refresh_lead_s, 3600);

    // Idempotent: second run should succeed.
    apply_migrations(&mut conn).expect("apply migrations twice");
}

#[test]
fn strict_v29_patch_migrates_legacy_workspace_cluster_tables() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("enable foreign_keys");

    conn.execute_batch(
        r#"
CREATE TABLE prompts (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  cli_key TEXT NOT NULL,
  name TEXT NOT NULL,
  content TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(cli_key, name)
);

CREATE TABLE mcp_servers (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  server_key TEXT NOT NULL,
  name TEXT NOT NULL,
  transport TEXT NOT NULL,
  command TEXT,
  args_json TEXT NOT NULL DEFAULT '[]',
  env_json TEXT NOT NULL DEFAULT '{}',
  cwd TEXT,
  url TEXT,
  headers_json TEXT NOT NULL DEFAULT '{}',
  enabled_claude INTEGER NOT NULL DEFAULT 0,
  enabled_codex INTEGER NOT NULL DEFAULT 0,
  enabled_gemini INTEGER NOT NULL DEFAULT 0,
  normalized_name TEXT NOT NULL DEFAULT '',
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(server_key)
);

CREATE TABLE skills (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  skill_key TEXT NOT NULL,
  name TEXT NOT NULL,
  normalized_name TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  source_git_url TEXT NOT NULL,
  source_branch TEXT NOT NULL,
  source_subdir TEXT NOT NULL,
  enabled_claude INTEGER NOT NULL DEFAULT 0,
  enabled_codex INTEGER NOT NULL DEFAULT 0,
  enabled_gemini INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(skill_key)
);

CREATE TABLE providers (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  cli_key TEXT NOT NULL,
  name TEXT NOT NULL,
  base_url TEXT NOT NULL,
  base_urls_json TEXT NOT NULL DEFAULT '[]',
  base_url_mode TEXT NOT NULL DEFAULT 'order',
  claude_models_json TEXT NOT NULL DEFAULT '{}',
  api_key_plaintext TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  priority INTEGER NOT NULL DEFAULT 100,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0,
  cost_multiplier REAL NOT NULL DEFAULT 1.0,
  supported_models_json TEXT NOT NULL DEFAULT '{}',
  model_mapping_json TEXT NOT NULL DEFAULT '{}',
  UNIQUE(cli_key, name)
);

PRAGMA user_version = 29;
"#,
    )
    .expect("create legacy v29 tables");

    conn.execute(
        r#"
INSERT INTO prompts(id, cli_key, name, content, enabled, created_at, updated_at)
VALUES (1, 'claude', 'default', 'hello', 1, 1, 1)
"#,
        [],
    )
    .expect("insert prompt");
    conn.execute(
        r#"
INSERT INTO prompts(id, cli_key, name, content, enabled, created_at, updated_at)
VALUES (2, 'codex', 'p2', 'world', 0, 1, 1)
"#,
        [],
    )
    .expect("insert prompt");

    conn.execute(
        r#"
INSERT INTO mcp_servers(
  id,
  server_key,
  name,
  transport,
  command,
  args_json,
  env_json,
  cwd,
  url,
  headers_json,
  enabled_claude,
  enabled_codex,
  enabled_gemini,
  normalized_name,
  created_at,
  updated_at
) VALUES (
  1,
  'srv1',
  'S1',
  'stdio',
  'echo',
  '[]',
  '{}',
  NULL,
  NULL,
  '{}',
  1,
  0,
  0,
  's1',
  1,
  1
)
"#,
        [],
    )
    .expect("insert mcp server");
    conn.execute(
        r#"
INSERT INTO mcp_servers(
  id,
  server_key,
  name,
  transport,
  command,
  args_json,
  env_json,
  cwd,
  url,
  headers_json,
  enabled_claude,
  enabled_codex,
  enabled_gemini,
  normalized_name,
  created_at,
  updated_at
) VALUES (
  2,
  'srv2',
  'S2',
  'stdio',
  'echo',
  '[]',
  '{}',
  NULL,
  NULL,
  '{}',
  0,
  1,
  0,
  's2',
  1,
  1
)
"#,
        [],
    )
    .expect("insert mcp server");

    conn.execute(
        r#"
INSERT INTO skills(
  id,
  skill_key,
  name,
  normalized_name,
  description,
  source_git_url,
  source_branch,
  source_subdir,
  enabled_claude,
  enabled_codex,
  enabled_gemini,
  created_at,
  updated_at
) VALUES (
  1,
  'sk1',
  'Skill 1',
  'skill-1',
  '',
  'https://example.com',
  'main',
  'skills/skill1',
  0,
  1,
  0,
  1,
  1
)
"#,
        [],
    )
    .expect("insert skill");

    apply_migrations(&mut conn).expect("apply migrations");

    let user_version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read user_version");
    assert_eq!(user_version, LATEST_SCHEMA_VERSION);

    assert!(test_has_column(&conn, "workspaces", "cli_key"));
    assert!(test_has_column(&conn, "workspace_active", "workspace_id"));

    assert!(test_has_column(&conn, "prompts", "workspace_id"));
    assert!(!test_has_column(&conn, "prompts", "cli_key"));

    assert!(test_has_column(&conn, "providers", "limit_5h_usd"));
    assert!(test_has_column(&conn, "providers", "limit_daily_usd"));
    assert!(test_has_column(&conn, "providers", "daily_reset_mode"));
    assert!(test_has_column(&conn, "providers", "daily_reset_time"));
    assert!(test_has_column(&conn, "providers", "limit_weekly_usd"));
    assert!(test_has_column(&conn, "providers", "limit_monthly_usd"));
    assert!(test_has_column(&conn, "providers", "limit_total_usd"));
    assert!(test_has_column(&conn, "skills", "installed_content_hash"));

    let claude_default_ws_id: i64 = conn
        .query_row(
            "SELECT id FROM workspaces WHERE cli_key = 'claude' AND name = '默认' ORDER BY id DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("read default claude workspace id");
    let codex_default_ws_id: i64 = conn
        .query_row(
            "SELECT id FROM workspaces WHERE cli_key = 'codex' AND name = '默认' ORDER BY id DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("read default codex workspace id");

    let p1_cli: String = conn
        .query_row(
            r#"
SELECT w.cli_key
FROM prompts p
JOIN workspaces w ON w.id = p.workspace_id
WHERE p.id = 1
"#,
            [],
            |row| row.get(0),
        )
        .expect("read migrated prompt cli_key");
    assert_eq!(p1_cli, "claude");

    let p2_cli: String = conn
        .query_row(
            r#"
SELECT w.cli_key
FROM prompts p
JOIN workspaces w ON w.id = p.workspace_id
WHERE p.id = 2
"#,
            [],
            |row| row.get(0),
        )
        .expect("read migrated prompt cli_key");
    assert_eq!(p2_cli, "codex");

    let claude_enabled_mcp: i64 = conn
        .query_row(
            "SELECT COUNT(1) FROM workspace_mcp_enabled WHERE workspace_id = ?1 AND server_id = 1",
            [claude_default_ws_id],
            |row| row.get(0),
        )
        .expect("count enabled mcp for claude");
    assert_eq!(claude_enabled_mcp, 1);

    let codex_enabled_mcp: i64 = conn
        .query_row(
            "SELECT COUNT(1) FROM workspace_mcp_enabled WHERE workspace_id = ?1 AND server_id = 2",
            [codex_default_ws_id],
            |row| row.get(0),
        )
        .expect("count enabled mcp for codex");
    assert_eq!(codex_enabled_mcp, 1);

    let legacy_mcp_flags: (i64, i64, i64) = conn
        .query_row(
            "SELECT enabled_claude, enabled_codex, enabled_gemini FROM mcp_servers WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("read legacy mcp flags");
    assert_eq!(legacy_mcp_flags, (0, 0, 0));

    let enabled_skill: i64 = conn
        .query_row(
            "SELECT COUNT(1) FROM workspace_skill_enabled WHERE workspace_id = ?1 AND skill_id = 1",
            [codex_default_ws_id],
            |row| row.get(0),
        )
        .expect("count enabled skills");
    assert_eq!(enabled_skill, 1);

    let legacy_skill_flags: (i64, i64, i64) = conn
        .query_row(
            "SELECT enabled_claude, enabled_codex, enabled_gemini FROM skills WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("read legacy skill flags");
    assert_eq!(legacy_skill_flags, (0, 0, 0));

    // Idempotent: second run should succeed without changing schema.
    apply_migrations(&mut conn).expect("apply migrations twice");
}

#[test]
fn ensure_patch_backfills_request_log_activity_columns_for_existing_request_logs_table() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("request-log-activity-drift.db");

    {
        let mut conn = Connection::open(&db_path).expect("open sqlite file");
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .expect("enable foreign_keys");

        apply_migrations(&mut conn).expect("create current schema");
        conn.execute_batch(
            r#"
ALTER TABLE request_logs DROP COLUMN last_activity_ms;
ALTER TABLE request_logs DROP COLUMN activity_details_json;
"#,
        )
        .expect("remove request log activity columns");

        assert!(!test_has_column(&conn, "request_logs", "last_activity_ms"));
        assert!(!test_has_column(
            &conn,
            "request_logs",
            "activity_details_json"
        ));
    }

    let db = crate::db::init_for_tests(&db_path).expect("repair drifted db");

    {
        let conn = db.open_connection().expect("open repaired db");
        assert!(test_has_column(&conn, "request_logs", "last_activity_ms"));
        assert!(test_has_column(
            &conn,
            "request_logs",
            "activity_details_json"
        ));

        conn.prepare("SELECT last_activity_ms, activity_details_json FROM request_logs LIMIT 1")
            .expect("prepare request log activity column select");
    }

    let summaries =
        crate::request_logs::list_recent_all(&db, 10).expect("list recent all after repair");
    assert!(summaries.is_empty());

    {
        let mut conn = db.open_connection().expect("open repaired db twice");
        apply_migrations(&mut conn).expect("apply migrations twice");
    }
}

#[test]
fn baseline_v25_creates_complete_schema_for_fresh_install() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("enable foreign_keys");

    // Fresh install: user_version = 0
    apply_migrations(&mut conn).expect("apply migrations on fresh db");

    let user_version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read user_version");
    assert_eq!(user_version, LATEST_SCHEMA_VERSION);

    // Verify all tables exist
    let tables: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
            .expect("prepare");
        let rows = stmt.query_map([], |row| row.get(0)).expect("query");
        rows.filter_map(|r| r.ok()).collect()
    };

    // Core tables from baseline
    assert!(tables.contains(&"providers".to_string()));
    assert!(tables.contains(&"request_logs".to_string()));
    assert!(tables.contains(&"provider_availability_observations".to_string()));
    assert!(tables.contains(&"prompts".to_string()));
    assert!(tables.contains(&"mcp_servers".to_string()));
    assert!(tables.contains(&"skills".to_string()));
    assert!(tables.contains(&"skill_repos".to_string()));
    assert!(tables.contains(&"model_prices".to_string()));
    assert!(tables.contains(&"provider_pool_order".to_string()));
    assert!(tables.contains(&"provider_account_usage_credentials".to_string()));
    assert!(tables.contains(&"default_route_providers".to_string()));
    assert!(tables.contains(&"sort_modes".to_string()));
    assert!(tables.contains(&"sort_mode_providers".to_string()));
    assert!(tables.contains(&"sort_mode_active".to_string()));
    assert!(tables.contains(&"claude_model_validation_runs".to_string()));
    assert!(tables.contains(&"image_gen_configs".to_string()));
    assert!(tables.contains(&"image_gen_tasks".to_string()));
    assert!(tables.contains(&"plugin_hook_execution_reports".to_string()));
    assert!(tables.contains(&"schema_migrations".to_string()));

    // Tables from ensure patches
    assert!(tables.contains(&"workspaces".to_string()));
    assert!(tables.contains(&"workspace_active".to_string()));
    assert!(tables.contains(&"workspace_mcp_enabled".to_string()));
    assert!(tables.contains(&"workspace_skill_enabled".to_string()));

    // Verify ensure patches ran (provider limit columns)
    assert!(test_has_column(&conn, "providers", "limit_5h_usd"));
    assert!(test_has_column(&conn, "providers", "limit_daily_usd"));
    assert!(test_has_column(&conn, "providers", "source_provider_id"));
    assert!(test_has_column(&conn, "providers", "bridge_type"));
    assert!(test_has_column(&conn, "providers", "tags_json"));
    assert!(test_has_column(&conn, "skills", "installed_commit"));
    assert!(test_has_column(&conn, "skills", "installed_content_hash"));

    // Verify v25->v26 migration ran (claude_models_json)
    assert!(test_has_column(&conn, "providers", "claude_models_json"));

    // Verify sort_mode_providers.enabled from ensure patch
    assert!(test_has_column(&conn, "sort_mode_providers", "enabled"));

    // Verify request log read-path indexes from ensure patches
    assert!(test_has_index(
        &conn,
        "idx_request_logs_cli_path_created_at_ms_id"
    ));
    assert!(test_has_index(
        &conn,
        "idx_request_logs_cli_created_at_ms_id"
    ));
    assert!(test_has_index(
        &conn,
        "idx_request_logs_visible_created_at_ms_id"
    ));
    assert!(test_has_index(&conn, "idx_request_logs_cli_id"));
    assert!(test_has_column(&conn, "request_logs", "last_activity_ms"));
    assert!(test_has_column(
        &conn,
        "request_logs",
        "activity_details_json"
    ));

    // Verify prompts was migrated to workspace_id
    assert!(test_has_column(&conn, "prompts", "workspace_id"));
    assert!(!test_has_column(&conn, "prompts", "cli_key"));

    // Idempotent: second run should succeed
    apply_migrations(&mut conn).expect("apply migrations twice");
}

#[test]
fn ensure_patches_seed_grok_workspace_once_without_resetting_active_workspace() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("enable foreign_keys");

    apply_migrations(&mut conn).expect("apply migrations on fresh db");

    let default_id: i64 = conn
        .query_row(
            "SELECT id FROM workspaces WHERE cli_key = 'grok' AND name = '默认'",
            [],
            |row| row.get(0),
        )
        .expect("read Grok default workspace");
    let initial_active_id: i64 = conn
        .query_row(
            "SELECT workspace_id FROM workspace_active WHERE cli_key = 'grok'",
            [],
            |row| row.get(0),
        )
        .expect("read Grok active workspace");
    assert_eq!(initial_active_id, default_id);

    conn.execute(
        "INSERT INTO workspaces(cli_key, name, normalized_name, created_at, updated_at) VALUES ('grok', 'Custom', 'custom', 1, 1)",
        [],
    )
    .expect("insert custom Grok workspace");
    let custom_id = conn.last_insert_rowid();
    conn.execute(
        "UPDATE workspace_active SET workspace_id = ?1, updated_at = 2 WHERE cli_key = 'grok'",
        [custom_id],
    )
    .expect("activate custom Grok workspace");

    apply_migrations(&mut conn).expect("apply migrations twice");

    let default_count: i64 = conn
        .query_row(
            "SELECT COUNT(1) FROM workspaces WHERE cli_key = 'grok' AND name = '默认'",
            [],
            |row| row.get(0),
        )
        .expect("count Grok default workspaces");
    let active_id: i64 = conn
        .query_row(
            "SELECT workspace_id FROM workspace_active WHERE cli_key = 'grok'",
            [],
            |row| row.get(0),
        )
        .expect("read preserved Grok active workspace");

    assert_eq!(default_count, 1);
    assert_eq!(active_id, custom_id);
}

#[test]
fn migrate_v35_to_v36_creates_image_gen_configs_and_is_idempotent() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");

    v35_to_v36::migrate_v35_to_v36(&mut conn).expect("migrate v35->v36");

    assert!(test_has_table(&conn, "image_gen_configs"));
    for column in [
        "adapter_id",
        "base_url",
        "api_key_plaintext",
        "model",
        "created_at",
        "updated_at",
    ] {
        assert!(
            test_has_column(&conn, "image_gen_configs", column),
            "missing image_gen_configs column: {column}"
        );
    }

    let user_version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read user_version");
    assert_eq!(user_version, 36);

    // Idempotent: second run should succeed.
    v35_to_v36::migrate_v35_to_v36(&mut conn).expect("migrate v35->v36 twice");
}

#[test]
fn apply_migrations_upgrades_v35_schema_to_v36() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    apply_migrations(&mut conn).expect("create current schema");

    // Simulate a v35 database (before image_gen_configs existed).
    conn.execute_batch(
        r#"
DROP TABLE image_gen_configs;
PRAGMA user_version = 35;
"#,
    )
    .expect("simulate v35 schema");
    assert!(!test_has_table(&conn, "image_gen_configs"));

    apply_migrations(&mut conn).expect("apply migrations from v35");

    assert!(test_has_table(&conn, "image_gen_configs"));
    let user_version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read user_version");
    assert_eq!(user_version, LATEST_SCHEMA_VERSION);
}

#[test]
fn migrate_v36_to_v37_creates_image_gen_tasks_and_is_idempotent() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");

    v36_to_v37::migrate_v36_to_v37(&mut conn).expect("migrate v36->v37");

    assert!(test_has_table(&conn, "image_gen_tasks"));
    for column in [
        "id",
        "adapter_id",
        "prompt",
        "request_json",
        "status",
        "error",
        "usage_json",
        "images_json",
        "ref_images_json",
        "dir",
        "created_at",
        "elapsed_ms",
    ] {
        assert!(
            test_has_column(&conn, "image_gen_tasks", column),
            "missing image_gen_tasks column: {column}"
        );
    }
    assert!(test_has_index(&conn, "idx_image_gen_tasks_created"));

    let user_version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read user_version");
    assert_eq!(user_version, 37);

    // Idempotent: second run should succeed.
    v36_to_v37::migrate_v36_to_v37(&mut conn).expect("migrate v36->v37 twice");
}

#[test]
fn apply_migrations_upgrades_v36_schema_to_v37() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    apply_migrations(&mut conn).expect("create current schema");

    // Simulate a v36 database (before image_gen_tasks existed).
    conn.execute_batch(
        r#"
DROP TABLE image_gen_tasks;
PRAGMA user_version = 36;
"#,
    )
    .expect("simulate v36 schema");
    assert!(!test_has_table(&conn, "image_gen_tasks"));

    apply_migrations(&mut conn).expect("apply migrations from v36");

    assert!(test_has_table(&conn, "image_gen_tasks"));
    assert!(test_has_index(&conn, "idx_image_gen_tasks_created"));
    let user_version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read user_version");
    assert_eq!(user_version, LATEST_SCHEMA_VERSION);
}

#[test]
fn rejects_unsupported_old_schema_version() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.execute_batch("PRAGMA user_version = 10;")
        .expect("set old version");

    let result = apply_migrations(&mut conn);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("minimum supported: 25"));
}

#[test]
fn strict_v29_patch_accepts_dev_schema_and_normalizes_user_version_to_29() {
    let mut conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("enable foreign_keys");

    conn.execute_batch(
        r#"
CREATE TABLE workspaces (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  cli_key TEXT NOT NULL,
  name TEXT NOT NULL,
  normalized_name TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(cli_key, normalized_name)
);

CREATE TABLE workspace_active (
  cli_key TEXT PRIMARY KEY,
  workspace_id INTEGER,
  updated_at INTEGER NOT NULL,
  FOREIGN KEY(workspace_id) REFERENCES workspaces(id) ON DELETE SET NULL
);

CREATE TABLE prompts (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  workspace_id INTEGER NOT NULL,
  name TEXT NOT NULL,
  content TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  FOREIGN KEY(workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
  UNIQUE(workspace_id, name)
);

CREATE TABLE providers (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  cli_key TEXT NOT NULL,
  name TEXT NOT NULL,
  base_url TEXT NOT NULL,
  base_urls_json TEXT NOT NULL DEFAULT '[]',
  base_url_mode TEXT NOT NULL DEFAULT 'order',
  claude_models_json TEXT NOT NULL DEFAULT '{}',
  api_key_plaintext TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  priority INTEGER NOT NULL DEFAULT 100,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0,
  cost_multiplier REAL NOT NULL DEFAULT 1.0,
  supported_models_json TEXT NOT NULL DEFAULT '{}',
  model_mapping_json TEXT NOT NULL DEFAULT '{}',
  UNIQUE(cli_key, name)
);

CREATE TABLE mcp_servers (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  server_key TEXT NOT NULL,
  name TEXT NOT NULL,
  normalized_name TEXT NOT NULL DEFAULT '',
  transport TEXT NOT NULL,
  command TEXT,
  args_json TEXT NOT NULL DEFAULT '[]',
  env_json TEXT NOT NULL DEFAULT '{}',
  cwd TEXT,
  url TEXT,
  headers_json TEXT NOT NULL DEFAULT '{}',
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(server_key)
);

CREATE TABLE workspace_mcp_enabled (
  workspace_id INTEGER NOT NULL,
  server_id INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY(workspace_id, server_id),
  FOREIGN KEY(workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
  FOREIGN KEY(server_id) REFERENCES mcp_servers(id) ON DELETE CASCADE
);

CREATE TABLE skills (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  skill_key TEXT NOT NULL,
  name TEXT NOT NULL,
  normalized_name TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  source_git_url TEXT NOT NULL,
  source_branch TEXT NOT NULL,
  source_subdir TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(skill_key)
);

CREATE TABLE workspace_skill_enabled (
  workspace_id INTEGER NOT NULL,
  skill_id INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY(workspace_id, skill_id),
  FOREIGN KEY(workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
  FOREIGN KEY(skill_id) REFERENCES skills(id) ON DELETE CASCADE
);

PRAGMA user_version = 33;
"#,
    )
    .expect("create dev schema");

    conn.execute(
        "INSERT INTO workspaces(id, cli_key, name, normalized_name, created_at, updated_at) VALUES (1, 'claude', 'Dev', 'dev', 1, 1)",
        [],
    )
    .expect("insert workspace");
    conn.execute(
        "INSERT INTO workspace_active(cli_key, workspace_id, updated_at) VALUES ('claude', 1, 1)",
        [],
    )
    .expect("insert workspace_active");
    conn.execute(
        "INSERT INTO prompts(id, workspace_id, name, content, enabled, created_at, updated_at) VALUES (1, 1, 'default', 'hello', 1, 1, 1)",
        [],
    )
    .expect("insert prompt");
    conn.execute(
        "INSERT INTO mcp_servers(id, server_key, name, normalized_name, transport, command, args_json, env_json, cwd, url, headers_json, created_at, updated_at) VALUES (1, 'srv1', 'S1', 's1', 'stdio', 'echo', '[]', '{}', NULL, NULL, '{}', 1, 1)",
        [],
    )
    .expect("insert mcp server");
    conn.execute(
        "INSERT INTO workspace_mcp_enabled(workspace_id, server_id, created_at, updated_at) VALUES (1, 1, 1, 1)",
        [],
    )
    .expect("insert mcp enabled");
    conn.execute(
        "INSERT INTO skills(id, skill_key, name, normalized_name, description, source_git_url, source_branch, source_subdir, created_at, updated_at) VALUES (1, 'sk1', 'Skill 1', 'skill-1', '', 'https://example.com', 'main', 'skills/skill1', 1, 1)",
        [],
    )
    .expect("insert skill");
    conn.execute(
        "INSERT INTO workspace_skill_enabled(workspace_id, skill_id, created_at, updated_at) VALUES (1, 1, 1, 1)",
        [],
    )
    .expect("insert skill enabled");

    apply_migrations(&mut conn).expect("apply migrations");

    let user_version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read user_version");
    assert_eq!(user_version, LATEST_SCHEMA_VERSION);

    assert!(test_has_column(&conn, "providers", "limit_5h_usd"));
    assert!(test_has_column(&conn, "providers", "limit_daily_usd"));
    assert!(test_has_column(&conn, "providers", "daily_reset_mode"));
    assert!(test_has_column(&conn, "providers", "daily_reset_time"));
    assert!(test_has_column(&conn, "providers", "limit_weekly_usd"));
    assert!(test_has_column(&conn, "providers", "limit_monthly_usd"));
    assert!(test_has_column(&conn, "providers", "limit_total_usd"));
    assert!(test_has_column(&conn, "skills", "installed_content_hash"));
    assert!(test_has_table(&conn, "plugin_hook_execution_reports"));

    let active_id: i64 = conn
        .query_row(
            "SELECT workspace_id FROM workspace_active WHERE cli_key = 'claude'",
            [],
            |row| row.get(0),
        )
        .expect("read active workspace");
    assert_eq!(active_id, 1);

    let enabled_mcp: i64 = conn
        .query_row(
            "SELECT COUNT(1) FROM workspace_mcp_enabled WHERE workspace_id = 1 AND server_id = 1",
            [],
            |row| row.get(0),
        )
        .expect("count enabled mcp");
    assert_eq!(enabled_mcp, 1);

    apply_migrations(&mut conn).expect("apply migrations twice");
}

#[test]
fn migrate_v37_to_v38_moves_valid_user_ids_and_sanitizes_extension_values() {
    let mut conn = Connection::open_in_memory().expect("open migration db");
    conn.execute_batch(
        r#"
PRAGMA foreign_keys = ON;
CREATE TABLE providers(id INTEGER PRIMARY KEY);
CREATE TABLE provider_extension_values(
  provider_id INTEGER NOT NULL,
  plugin_id TEXT NOT NULL,
  namespace TEXT NOT NULL,
  values_json TEXT NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY(provider_id, plugin_id, namespace),
  FOREIGN KEY(provider_id) REFERENCES providers(id) ON DELETE CASCADE
);
INSERT INTO providers(id) VALUES (1), (2), (3);
INSERT INTO provider_extension_values(provider_id, plugin_id, namespace, values_json, updated_at)
VALUES
  (1, 'core.provider-account-usage', 'accountUsage', '{"adapterKind":"newapi","newApiUserId":"00042","newApiAccessToken":"SYNTHETIC_PRIVATE_A"}', 1),
  (2, 'core.provider-account-usage', 'accountUsage', '{"adapterKind":"newapi","newApiUserId":"invalid","systemAccessToken":"SYNTHETIC_PRIVATE_B"}', 1),
  (3, 'core.provider-account-usage', 'accountUsage', '{"adapterKind":"newapi","newApiQueryMode":"account","timedRefreshEnabled":false,"refreshIntervalSeconds":120}', 1);
PRAGMA user_version = 37;
"#,
    )
    .expect("create v37 fixture");

    v37_to_v38::migrate_v37_to_v38(&mut conn).expect("migrate v37->v38");
    v37_to_v38::migrate_v37_to_v38(&mut conn).expect("repeat v37->v38");

    let user_version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("user version");
    assert_eq!(user_version, 38);
    assert!(test_has_table(&conn, "provider_account_usage_credentials"));
    let migrated_user_id: String = conn
        .query_row(
            "SELECT newapi_user_id FROM provider_account_usage_credentials WHERE provider_id = 1",
            [],
            |row| row.get(0),
        )
        .expect("migrated user id");
    assert_eq!(migrated_user_id, "42");
    let invalid_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM provider_account_usage_credentials WHERE provider_id = 2",
            [],
            |row| row.get(0),
        )
        .expect("invalid user id count");
    assert_eq!(invalid_count, 0);

    let mut statement = conn
        .prepare("SELECT values_json FROM provider_extension_values ORDER BY provider_id")
        .expect("extension query");
    let values = statement
        .query_map([], |row| row.get::<_, String>(0))
        .expect("extension rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("extension values");
    assert_eq!(values.len(), 3);
    for value in &values {
        assert!(!value.contains("UserId"));
        assert!(!value.contains("AccessToken"));
        assert!(!value.contains("SYNTHETIC_PRIVATE"));
    }
    assert!(values[0].contains("\"newApiQueryMode\":\"billing\""));
    assert!(values[2].contains("\"newApiQueryMode\":\"account\""));
    drop(statement);

    conn.execute("DELETE FROM providers WHERE id = 1", [])
        .expect("delete provider");
    let credential_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM provider_account_usage_credentials",
            [],
            |row| row.get(0),
        )
        .expect("credential count after cascade");
    assert_eq!(credential_count, 0);
}

#[test]
fn migrate_v39_to_v40_rejects_missing_providers_without_advancing() {
    let mut conn = Connection::open_in_memory().expect("open migration db");
    conn.execute_batch("PRAGMA user_version = 39;")
        .expect("create missing-provider fixture");

    let error =
        v39_to_v40::migrate_v39_to_v40(&mut conn).expect_err("missing providers table must fail");
    assert!(error.contains("requires the providers table"));
    let user_version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("user version after failure");
    assert_eq!(user_version, 39);
    assert_no_v40_provider_model_schema(&conn);
}

#[test]
fn migrate_v39_to_v40_rejects_existing_invalid_provider_uuids_without_repair() {
    for invalid in [None, Some(""), Some("not-a-canonical-uuid")] {
        let mut conn = Connection::open_in_memory().expect("open migration db");
        conn.execute_batch(
            r#"
CREATE TABLE providers(
  id INTEGER PRIMARY KEY,
  cli_key TEXT NOT NULL,
  name TEXT NOT NULL,
  provider_uuid TEXT
);
PRAGMA user_version = 39;
"#,
        )
        .expect("create dirty v39 fixture");
        conn.execute(
            "INSERT INTO providers(id, cli_key, name, provider_uuid) VALUES (1, 'codex', 'dirty', ?1)",
            rusqlite::params![invalid],
        )
        .expect("insert dirty provider UUID");

        let error = v39_to_v40::migrate_v39_to_v40(&mut conn)
            .expect_err("existing invalid provider UUID must fail");
        assert_eq!(error, "existing provider UUID is invalid");
        if let Some(invalid) = invalid.filter(|value| !value.is_empty()) {
            assert!(!error.contains(invalid), "error must not echo stored UUID");
        }
        let user_version: i64 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("user version after failure");
        assert_eq!(user_version, 39);
        assert_no_v40_provider_model_schema(&conn);
    }
}

#[test]
fn migrate_v39_to_v40_rejects_duplicate_existing_provider_uuids_without_partial_schema() {
    let mut conn = Connection::open_in_memory().expect("open migration db");
    conn.execute_batch(
        r#"
CREATE TABLE providers(
  id INTEGER PRIMARY KEY,
  cli_key TEXT NOT NULL,
  name TEXT NOT NULL,
  provider_uuid TEXT
);
PRAGMA user_version = 39;
"#,
    )
    .expect("create duplicate v39 fixture");
    let duplicate = "550e8400-e29b-41d4-a716-446655440000";
    for id in [1_i64, 2_i64] {
        conn.execute(
            "INSERT INTO providers(id, cli_key, name, provider_uuid) VALUES (?1, 'codex', 'duplicate', ?2)",
            rusqlite::params![id, duplicate],
        )
        .expect("insert duplicate provider UUID");
    }

    let error =
        v39_to_v40::migrate_v39_to_v40(&mut conn).expect_err("duplicate provider UUIDs must fail");
    assert_eq!(error, "existing provider UUIDs are not unique");
    assert!(
        !error.contains(duplicate),
        "error must not echo stored UUID"
    );
    let user_version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("user version after failure");
    assert_eq!(user_version, 39);
    assert_no_v40_provider_model_schema(&conn);
}

#[test]
fn migrate_v39_to_v40_backfills_canonical_provider_uuids_and_is_idempotent() {
    let mut conn = Connection::open_in_memory().expect("open migration db");
    conn.execute_batch(
        r#"
PRAGMA foreign_keys = ON;
CREATE TABLE providers(
  id INTEGER PRIMARY KEY,
  cli_key TEXT NOT NULL,
  name TEXT NOT NULL
);
INSERT INTO providers(id, cli_key, name) VALUES
  (1, 'codex', 'one'),
  (2, 'claude', 'two');
PRAGMA user_version = 39;
"#,
    )
    .expect("create v39 fixture");

    v39_to_v40::migrate_v39_to_v40(&mut conn).expect("migrate v39->v40");
    v39_to_v40::migrate_v39_to_v40(&mut conn).expect("repeat v39->v40");

    let provider_uuids = conn
        .prepare("SELECT provider_uuid FROM providers ORDER BY id ASC")
        .expect("prepare UUID query")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query UUIDs")
        .collect::<Result<Vec<_>, _>>()
        .expect("read UUIDs");
    assert_eq!(provider_uuids.len(), 2);
    assert_ne!(provider_uuids[0], provider_uuids[1]);
    assert!(provider_uuids
        .iter()
        .all(|value| crate::shared::uuid::is_canonical_uuid_v4(value)));
    for table in [
        "provider_model_catalogs",
        "provider_models",
        "codex_managed_profiles",
    ] {
        assert!(test_has_table(&conn, table), "missing {table}");
    }
    let user_version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("user version");
    assert_eq!(user_version, 40);

    let update_error = conn
        .execute(
            "UPDATE providers SET provider_uuid = ?1 WHERE id = 1",
            rusqlite::params![crate::shared::uuid::new_uuid_v4()],
        )
        .expect_err("provider UUID must be immutable");
    assert!(update_error
        .to_string()
        .contains("provider_uuid is immutable"));

    let extra_hyphen = "550e8400-e29b-41d4-a716-44665544000-";
    let insert_error = conn
        .execute(
            "INSERT INTO providers(id, cli_key, name, provider_uuid) VALUES (3, 'codex', 'bad', ?1)",
            rusqlite::params![extra_hyphen],
        )
        .expect_err("incremental trigger must reject an extra UUID hyphen");
    assert!(insert_error
        .to_string()
        .contains("provider_uuid must be a canonical UUID"));
}

#[test]
fn migrate_v40_to_v41_requires_provider_models_without_advancing() {
    let mut conn = Connection::open_in_memory().expect("open migration db");
    conn.execute_batch("PRAGMA user_version = 40;")
        .expect("create missing-model fixture");

    let error =
        v40_to_v41::migrate_v40_to_v41(&mut conn).expect_err("missing provider_models must fail");
    assert!(error.contains("requires the provider_models table"));
    let user_version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("user version after failure");
    assert_eq!(user_version, 40);
}

#[test]
fn migrate_v40_to_v41_backfills_existing_models_and_defaults_new_rows_unconfigured() {
    let mut conn = Connection::open_in_memory().expect("open migration db");
    conn.execute_batch(
        r#"
CREATE TABLE provider_models (
  model_uuid TEXT PRIMARY KEY,
  provider_id INTEGER NOT NULL,
  remote_model_id TEXT NOT NULL,
  source TEXT NOT NULL,
  stale INTEGER NOT NULL DEFAULT 0,
  last_seen_at INTEGER,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(provider_id, remote_model_id)
);
INSERT INTO provider_models(
  model_uuid, provider_id, remote_model_id, source, stale, created_at, updated_at
) VALUES ('old-model', 1, 'grok-4.5', 'manual', 0, 1, 1);
PRAGMA user_version = 40;
"#,
    )
    .expect("create v40 fixture");

    v40_to_v41::migrate_v40_to_v41(&mut conn).expect("migrate v40->v41");
    v40_to_v41::migrate_v40_to_v41(&mut conn).expect("repeat v40->v41");

    let existing = conn
        .query_row(
            r#"
SELECT capabilities_configured, supported_reasoning_efforts_json,
       default_reasoning_effort, context_window
FROM provider_models
WHERE model_uuid = 'old-model'
"#,
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                ))
            },
        )
        .expect("read backfilled model");
    assert_eq!(
        existing,
        (
            1,
            r#"["low","medium","high"]"#.to_string(),
            Some("medium".to_string()),
            None,
        )
    );

    conn.execute(
        r#"
INSERT INTO provider_models(
  model_uuid, provider_id, remote_model_id, source, stale, created_at, updated_at
) VALUES ('new-model', 1, 'gpt-new', 'manual', 0, 2, 2)
"#,
        [],
    )
    .expect("insert new model");
    let new_model = conn
        .query_row(
            r#"
SELECT capabilities_configured, supported_reasoning_efforts_json,
       default_reasoning_effort, context_window
FROM provider_models
WHERE model_uuid = 'new-model'
"#,
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                ))
            },
        )
        .expect("read new model defaults");
    assert_eq!(new_model, (0, "[]".to_string(), None, None));

    let context_error = conn
        .execute(
            "UPDATE provider_models SET context_window = 100 WHERE model_uuid = 'new-model'",
            [],
        )
        .expect_err("bounded context window must reject tiny values");
    assert!(context_error
        .to_string()
        .contains("CHECK constraint failed"));

    let user_version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("user version");
    assert_eq!(user_version, 41);
}

#[test]
fn migrate_v41_to_v42_adds_route_priorities_with_legacy_defaults() {
    let mut conn = Connection::open_in_memory().expect("open migration db");
    conn.execute_batch(
        r#"
CREATE TABLE default_route_providers (
  cli_key TEXT NOT NULL,
  provider_id INTEGER NOT NULL,
  sort_order INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY(cli_key, provider_id)
);
CREATE TABLE sort_mode_providers (
  mode_id INTEGER NOT NULL,
  cli_key TEXT NOT NULL,
  provider_id INTEGER NOT NULL,
  sort_order INTEGER NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY(mode_id, cli_key, provider_id)
);
INSERT INTO default_route_providers(cli_key, provider_id, sort_order, created_at, updated_at)
VALUES ('claude', 1, 0, 1, 1);
INSERT INTO sort_mode_providers(mode_id, cli_key, provider_id, sort_order, enabled, created_at, updated_at)
VALUES (1, 'claude', 1, 0, 1, 1, 1);
PRAGMA user_version = 41;
"#,
    )
    .expect("create v41 fixture");

    v41_to_v42::migrate_v41_to_v42(&mut conn).expect("migrate v41->v42");
    v41_to_v42::migrate_v41_to_v42(&mut conn).expect("repeat v41->v42");

    assert!(test_has_column(
        &conn,
        "default_route_providers",
        "session_reuse_priority"
    ));
    assert!(test_has_column(
        &conn,
        "sort_mode_providers",
        "session_reuse_priority"
    ));
    let defaults: (i64, i64) = conn
        .query_row(
            "SELECT (SELECT session_reuse_priority FROM default_route_providers), (SELECT session_reuse_priority FROM sort_mode_providers)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read priority defaults");
    assert_eq!(defaults, (0, 0));

    let error = conn
        .execute(
            "UPDATE default_route_providers SET session_reuse_priority = 1001",
            [],
        )
        .expect_err("priority range must be constrained");
    assert!(error.to_string().contains("CHECK constraint failed"));

    let user_version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read user version");
    assert_eq!(user_version, 42);
}

#[test]
fn migrate_v41_to_v42_defers_missing_route_tables_to_ensure_patches() {
    let mut conn = Connection::open_in_memory().expect("open migration db");
    conn.execute_batch("PRAGMA user_version = 41;")
        .expect("set v41 fixture version");

    v41_to_v42::migrate_v41_to_v42(&mut conn)
        .expect("missing route tables must not block the migration");

    let user_version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read user version");
    assert_eq!(user_version, 42);
}

#[test]
fn fresh_baseline_includes_route_session_reuse_priorities() {
    let mut conn = Connection::open_in_memory().expect("open fresh migration db");

    apply_migrations(&mut conn).expect("initialize fresh database");

    for table in ["default_route_providers", "sort_mode_providers"] {
        assert!(
            test_has_column(&conn, table, "session_reuse_priority"),
            "fresh baseline must include {table}.session_reuse_priority"
        );
    }
}

#[test]
fn fresh_baseline_creates_complete_usage_ledger_schema() {
    let mut conn = Connection::open_in_memory().expect("open fresh migration db");

    apply_migrations(&mut conn).expect("initialize fresh database");

    let user_version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read fresh user version");
    assert_eq!(user_version, LATEST_SCHEMA_VERSION);
    for object in [
        ("table", "usage_ledger"),
        ("table", "usage_ledger_backfill_state"),
        ("view", "usage_events"),
        ("index", "idx_usage_ledger_created_at"),
    ] {
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = ?1 AND name = ?2)",
                [object.0, object.1],
                |row| row.get(0),
            )
            .expect("inspect fresh usage ledger object");
        assert!(exists, "fresh schema is missing {} {}", object.0, object.1);
    }

    let view_sql: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'view' AND name = 'usage_events'",
            [],
            |row| row.get(0),
        )
        .expect("read usage events view definition");
    let normalized_view_sql = view_sql.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        normalized_view_sql.contains(
            "FROM backfill_mode mode CROSS JOIN request_logs r WHERE mode.is_complete = 0"
        ),
        "usage_events must gate request_logs before scanning the detail source"
    );

    let state: (String, i64, i64, Option<i64>) = conn
        .query_row(
            r#"
SELECT status, target_request_log_id, last_request_log_id, completed_at
FROM usage_ledger_backfill_state
WHERE id = 1
"#,
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("read fresh usage ledger state");
    assert_eq!(state.0, "complete");
    assert_eq!((state.1, state.2), (0, 0));
    assert!(state.3.is_some());

    let ledger_columns: std::collections::HashSet<String> = {
        let mut statement = conn
            .prepare("PRAGMA table_info(usage_ledger)")
            .expect("prepare usage ledger columns");
        statement
            .query_map([], |row| row.get(1))
            .expect("query usage ledger columns")
            .collect::<Result<_, _>>()
            .expect("read usage ledger columns")
    };
    for forbidden in [
        "attempts_json",
        "special_settings_json",
        "usage_json",
        "error_code",
    ] {
        assert!(
            !ledger_columns.contains(forbidden),
            "usage ledger must not persist {forbidden}"
        );
    }
}

#[test]
fn migrate_v43_to_v44_adds_provider_model_routing_policy_column() {
    let mut conn = Connection::open_in_memory().expect("open v43 migration db");
    apply_migrations(&mut conn).expect("create current schema fixture");
    conn.execute_batch(
        r#"
DROP VIEW IF EXISTS usage_events;
ALTER TABLE providers DROP COLUMN model_routing_policy_json;
PRAGMA user_version = 43;
"#,
    )
    .expect("downgrade fixture to v43 shape");

    v43_to_v44::migrate_v43_to_v44(&mut conn).expect("migrate v43->v44");

    assert!(test_has_column(
        &conn,
        "providers",
        "model_routing_policy_json"
    ));
    assert!(test_has_view(&conn, "usage_events"));
    let user_version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read v44 user version");
    assert_eq!(user_version, 44);
}

#[test]
fn migrate_v44_to_v45_adds_provider_availability_observations() {
    let mut conn = Connection::open_in_memory().expect("open v44 migration db");
    conn.execute_batch(
        r#"
CREATE TABLE providers (id INTEGER PRIMARY KEY);
PRAGMA user_version = 44;
"#,
    )
    .expect("create v44 fixture");

    v44_to_v45::migrate_v44_to_v45(&mut conn).expect("migrate v44->v45");

    let user_version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read migrated user version");
    let columns: i64 = conn
        .query_row(
            "SELECT COUNT(1) FROM pragma_table_info('provider_availability_observations')",
            [],
            |row| row.get(0),
        )
        .expect("inspect provider availability columns");
    let indexes: i64 = conn
        .query_row(
            "SELECT COUNT(1) FROM sqlite_master WHERE type = 'index' AND tbl_name = 'provider_availability_observations'",
            [],
            |row| row.get(0),
        )
        .expect("inspect provider availability indexes");

    assert_eq!(user_version, 45);
    assert_eq!(columns, 5);
    assert!(indexes >= 3, "primary key and both time indexes must exist");
}

#[test]
fn usage_events_normalizes_incomplete_rows_and_matches_completed_ledger() {
    let mut conn = Connection::open_in_memory().expect("open usage-events migration db");
    apply_migrations(&mut conn).expect("create current schema");
    conn.execute(
        r#"
INSERT INTO request_logs(
  trace_id,
  cli_key,
  session_id,
  method,
  path,
  status,
  duration_ms,
  ttfb_ms,
  visible_ttfb_ms,
  attempts_json,
  created_at,
  created_at_ms,
  input_tokens,
  output_tokens,
  usage_json,
  requested_model,
  special_settings_json,
  excluded_from_stats
) VALUES (
  'normalized-usage-event',
  'claude',
  'session-normalized',
  'POST',
  '/v1/messages',
  200,
  25,
  5,
  5,
  '[
    {"provider_id":77,"provider_name":" \t\n","outcome":"success"},
    {"provider_id":"broken","provider_name":"Malformed","outcome":"success"},
    {"provider_id":9223372036854775808,"provider_name":"Overflow","outcome":"success"}
  ]',
  10,
  10000,
  100,
  20,
  '{"input_tokens":100,"output_tokens":20}',
  'client-model',
  '[
    {
      "type":"cx2cc_cost_basis",
      "bridge_provider_id":77,
      "source_cli_key":"codex",
      "priced_model":"gpt-priced"
    },
    {
      "type":"codex_service_tier_result",
      "effectivePriority":true
    }
  ]',
  0
)
"#,
        [],
    )
    .expect("insert normalized usage event fixture");
    let request_log_id: i64 = conn
        .query_row(
            "SELECT id FROM request_logs WHERE trace_id = 'normalized-usage-event'",
            [],
            |row| row.get(0),
        )
        .expect("read normalized request id");
    conn.execute(
        r#"
UPDATE usage_ledger_backfill_state
SET status = 'incomplete',
    target_request_log_id = ?1,
    last_request_log_id = 0,
    completed_at = NULL
WHERE id = 1
"#,
        [request_log_id],
    )
    .expect("mark normalized fixture incomplete");

    let read_normalized = |conn: &Connection| {
        conn.query_row(
            r#"
SELECT
  final_provider_id,
  provider_name_snapshot,
  usage_present,
  persisted_openai_input_semantics,
  cost_basis_cli_key,
  cost_basis_model,
  priority_service_tier_applied,
  error_present
FROM usage_events
WHERE trace_id = 'normalized-usage-event'
"#,
            [],
            |row| {
                Ok((
                    row.get::<_, Option<i64>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, bool>(2)?,
                    row.get::<_, bool>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, bool>(6)?,
                    row.get::<_, bool>(7)?,
                ))
            },
        )
        .expect("read normalized usage event")
    };
    let expected = (
        Some(77),
        None,
        true,
        true,
        Some("codex".to_string()),
        Some("gpt-priced".to_string()),
        true,
        false,
    );
    assert_eq!(read_normalized(&conn), expected);

    crate::usage_ledger::project_trace(&conn, "normalized-usage-event")
        .expect("project normalized usage event");
    conn.execute(
        r#"
UPDATE usage_ledger_backfill_state
SET status = 'complete',
    last_request_log_id = target_request_log_id,
    completed_at = 11,
    updated_at = 11
WHERE id = 1
"#,
        [],
    )
    .expect("complete normalized fixture");
    assert_eq!(read_normalized(&conn), expected);
}

#[test]
fn migrate_v42_to_v43_records_fixed_high_water_without_sync_backfill() {
    let mut conn = Connection::open_in_memory().expect("open v42 migration db");
    baseline_v25::create_baseline_v25(&mut conn).expect("create current fixture schema");
    conn.execute_batch(
        r#"
DROP VIEW usage_events;
DROP TABLE provider_extension_values;
DROP TABLE providers;
CREATE TABLE providers (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  cli_key TEXT NOT NULL,
  name TEXT NOT NULL
);
DROP TABLE usage_ledger_backfill_state;
DROP TABLE usage_ledger;
INSERT INTO request_logs(
  id, trace_id, cli_key, method, path, attempts_json, created_at, created_at_ms
) VALUES
  (4, 'upgrade-trace-4', 'claude', 'POST', '/v1/messages', '[]', 4, 4000),
  (9, 'upgrade-trace-9', 'codex', 'POST', '/v1/responses', '[]', 9, 9000);
PRAGMA user_version = 42;
"#,
    )
    .expect("create v42 usage fixture");

    assert!(!test_has_column(&conn, "providers", "source_provider_id"));
    assert!(!test_has_column(&conn, "providers", "bridge_type"));
    v42_to_v43::migrate_v42_to_v43(&mut conn).expect("migrate v42->v43");

    assert!(test_has_column(&conn, "providers", "source_provider_id"));
    assert!(test_has_column(&conn, "providers", "bridge_type"));
    let usage_event_count: i64 = conn
        .query_row("SELECT COUNT(1) FROM usage_events", [], |row| row.get(0))
        .expect("query usage events after v42->v43 upgrade");
    assert_eq!(usage_event_count, 2);

    let state: (String, i64, i64, Option<i64>) = conn
        .query_row(
            r#"
SELECT status, target_request_log_id, last_request_log_id, completed_at
FROM usage_ledger_backfill_state
WHERE id = 1
"#,
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("read upgraded usage ledger state");
    assert_eq!(state, ("incomplete".to_string(), 9, 0, None));
    let ledger_count: i64 = conn
        .query_row("SELECT COUNT(1) FROM usage_ledger", [], |row| row.get(0))
        .expect("count migration-time usage rows");
    assert_eq!(ledger_count, 0, "v43 migration must remain DDL-only");
}

#[test]
fn migrate_v42_to_v43_treats_missing_request_logs_as_empty() {
    let mut conn = Connection::open_in_memory().expect("open reduced v42 migration db");
    conn.execute_batch(
        r#"
CREATE TABLE providers (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  cli_key TEXT NOT NULL,
  name TEXT NOT NULL
);
PRAGMA user_version = 42;
"#,
    )
    .expect("create reduced v42 fixture without request_logs");

    v42_to_v43::migrate_v42_to_v43(&mut conn).expect("migrate reduced v42->v43");

    let state: (String, i64, i64, Option<i64>) = conn
        .query_row(
            r#"
SELECT status, target_request_log_id, last_request_log_id, completed_at
FROM usage_ledger_backfill_state
WHERE id = 1
"#,
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("read reduced upgraded usage ledger state");
    assert_eq!(state.0, "complete");
    assert_eq!((state.1, state.2), (0, 0));
    assert!(state.3.is_some());

    let user_version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read reduced upgraded user version");
    assert_eq!(user_version, 43);
    assert!(!test_has_view(&conn, "usage_events"));
}

#[test]
fn ensure_repairs_missing_usage_ledger_as_incomplete() {
    let mut conn = Connection::open_in_memory().expect("open current migration db");
    apply_migrations(&mut conn).expect("create current schema");
    conn.execute(
        r#"
INSERT INTO request_logs(
  trace_id, cli_key, method, path, attempts_json, created_at, created_at_ms
) VALUES ('ensure-ledger-trace', 'claude', 'POST', '/v1/messages', '[]', 7, 7000)
"#,
        [],
    )
    .expect("seed request log before schema drift");
    let target_id: i64 = conn
        .query_row(
            "SELECT id FROM request_logs WHERE trace_id = 'ensure-ledger-trace'",
            [],
            |row| row.get(0),
        )
        .expect("read schema drift target");
    conn.execute_batch(
        r#"
DROP VIEW usage_events;
DROP TABLE usage_ledger_backfill_state;
DROP TABLE usage_ledger;
"#,
    )
    .expect("create usage ledger schema drift");

    apply_migrations(&mut conn).expect("repair usage ledger schema");

    let state: (String, i64, i64) = conn
        .query_row(
            r#"
SELECT status, target_request_log_id, last_request_log_id
FROM usage_ledger_backfill_state
WHERE id = 1
"#,
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("read repaired usage ledger state");
    assert_eq!(state, ("incomplete".to_string(), target_id, 0));
}

#[test]
fn ensure_restores_usage_ledger_created_at_index() {
    let mut conn = Connection::open_in_memory().expect("open current migration db");
    apply_migrations(&mut conn).expect("create current schema");
    conn.execute_batch("DROP INDEX idx_usage_ledger_created_at;")
        .expect("drop standalone usage ledger time index");

    apply_migrations(&mut conn).expect("repair usage ledger indexes");

    let exists: bool = conn
        .query_row(
            r#"
SELECT EXISTS(
  SELECT 1
  FROM sqlite_master
  WHERE type = 'index' AND name = 'idx_usage_ledger_created_at'
)
"#,
            [],
            |row| row.get(0),
        )
        .expect("inspect repaired usage ledger time index");
    assert!(exists);
}

#[test]
fn ensure_patches_restore_route_session_reuse_priorities_on_schema_drift() {
    let mut conn = Connection::open_in_memory().expect("open current-schema migration db");
    baseline_v25::create_baseline_v25(&mut conn).expect("create current baseline");
    conn.execute_batch(
        r#"
ALTER TABLE default_route_providers DROP COLUMN session_reuse_priority;
ALTER TABLE sort_mode_providers DROP COLUMN session_reuse_priority;
"#,
    )
    .expect("create current-schema drift fixture");

    apply_migrations(&mut conn).expect("apply current-schema ensure patches");

    for table in ["default_route_providers", "sort_mode_providers"] {
        assert!(
            test_has_column(&conn, table, "session_reuse_priority"),
            "ensure patches must restore {table}.session_reuse_priority"
        );
    }
}

#[test]
fn fresh_v41_schema_rejects_missing_or_noncanonical_provider_uuid() {
    let mut conn = Connection::open_in_memory().expect("open migration db");
    apply_migrations(&mut conn).expect("apply migrations");

    let missing = conn
        .execute(
            r#"
INSERT INTO providers(cli_key, name, base_url, api_key_plaintext, created_at, updated_at)
VALUES ('codex', 'missing', 'https://example.invalid/v1', 'key', 1, 1)
"#,
            [],
        )
        .expect_err("missing UUID must fail");
    assert!(missing.to_string().contains("provider_uuid"));

    for invalid in [
        "550E8400-E29B-41D4-A716-446655440000",
        "550e8400-e29b-11d4-a716-446655440000",
        "550e8400-e29b-41d4-c716-446655440000",
        "550e8400-e29b-41d4-a716-44665544000z",
        "550e8400-e29b-41d4-a716-44665544000-",
    ] {
        let error = conn
            .execute(
                r#"
INSERT INTO providers(
  provider_uuid, cli_key, name, base_url, api_key_plaintext, created_at, updated_at
) VALUES (?1, 'codex', ?1, 'https://example.invalid/v1', 'key', 1, 1)
"#,
                rusqlite::params![invalid],
            )
            .expect_err("invalid UUID must fail");
        assert!(
            error
                .to_string()
                .contains("provider_uuid must be a canonical UUID"),
            "unexpected error for {invalid}: {error}"
        );
    }
}

#[test]
fn migrate_v46_to_v47_creates_daily_rollup_schema_and_marks_changed_days_dirty() {
    let mut conn = Connection::open_in_memory().expect("open v46 migration db");
    v42_to_v43::create_usage_ledger_schema(&conn).expect("create v46 usage ledger fixture");
    conn.pragma_update(None, "user_version", 46)
        .expect("mark rollup fixture as v46");

    v46_to_v47::migrate_v46_to_v47(&mut conn).expect("migrate v46->v47");

    let user_version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read v47 user version");
    assert_eq!(user_version, 47);
    for (object_type, name) in [
        ("table", "usage_provider_daily_rollup_days"),
        ("table", "usage_provider_daily_rollups"),
        ("table", "usage_provider_daily_rollup_backfill_state"),
        ("trigger", "trg_usage_ledger_daily_rollup_insert"),
        ("trigger", "trg_usage_ledger_daily_rollup_update"),
        ("trigger", "trg_usage_ledger_daily_rollup_delete"),
        ("index", "idx_usage_provider_daily_rollups_provider_day"),
        ("index", "idx_usage_provider_daily_rollups_cli_day"),
        ("index", "idx_usage_provider_daily_rollup_days_status_day"),
    ] {
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = ?1 AND name = ?2)",
                [object_type, name],
                |row| row.get(0),
            )
            .expect("inspect v46 object");
        assert!(exists, "missing {object_type} {name}");
    }

    let first_ts = 1_700_000_000_i64;
    let second_ts = first_ts + 2 * 24 * 60 * 60;
    conn.execute(
        r#"
INSERT INTO usage_ledger(request_log_id, trace_id, cli_key, created_at)
VALUES (1, 'trace-rollup-migration', 'codex', ?1)
"#,
        [first_ts],
    )
    .expect("insert ledger fixture");
    conn.execute(
        "UPDATE usage_provider_daily_rollup_days SET status = 'complete', updated_at = 1",
        [],
    )
    .expect("complete inserted fixture day");
    conn.execute(
        "UPDATE usage_ledger SET created_at = created_at WHERE request_log_id = 1",
        [],
    )
    .expect("repeat identical trend fields");
    let unchanged_after_identical_update: (String, i64) = conn
        .query_row(
            "SELECT status, updated_at FROM usage_provider_daily_rollup_days",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read day after identical update");
    assert_eq!(
        unchanged_after_identical_update,
        ("complete".to_string(), 1),
        "identical ledger projection must not dirty a completed day"
    );
    conn.execute(
        "UPDATE usage_ledger SET cost_usd_femto = 10 WHERE request_log_id = 1",
        [],
    )
    .expect("update non-trend ledger field");
    let unchanged_day: (String, i64) = conn
        .query_row(
            "SELECT status, updated_at FROM usage_provider_daily_rollup_days",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read day after non-trend update");
    assert_eq!(unchanged_day, ("complete".to_string(), 1));
    conn.execute(
        "UPDATE usage_ledger SET created_at = ?1 WHERE request_log_id = 1",
        [second_ts],
    )
    .expect("move ledger fixture to another day");
    conn.execute("DELETE FROM usage_ledger WHERE request_log_id = 1", [])
        .expect("delete ledger fixture");

    let dirty_days: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM usage_provider_daily_rollup_days WHERE status = 'dirty'",
            [],
            |row| row.get(0),
        )
        .expect("count dirty days");
    assert_eq!(
        dirty_days, 2,
        "updates must dirty both old and new local days"
    );

    let invalid_ts = i64::MAX;
    conn.execute(
        r#"
INSERT INTO usage_ledger(request_log_id, trace_id, cli_key, created_at)
VALUES (2, 'trace-rollup-invalid-timestamp', 'codex', ?1)
"#,
        [invalid_ts],
    )
    .expect("malformed legacy timestamp must not block ledger writes");
    let unchanged_days: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM usage_provider_daily_rollup_days",
            [],
            |row| row.get(0),
        )
        .expect("count rollup days after malformed timestamp");
    assert_eq!(unchanged_days, 2);
}

fn remove_stream_timing_schema_for_drift_fixture(conn: &Connection) {
    conn.execute_batch(
        r#"
DROP VIEW IF EXISTS usage_events;
DROP TRIGGER IF EXISTS trg_usage_ledger_daily_rollup_insert;
DROP TRIGGER IF EXISTS trg_usage_ledger_daily_rollup_update;
DROP TRIGGER IF EXISTS trg_usage_ledger_daily_rollup_delete;
ALTER TABLE request_logs DROP COLUMN upstream_stream_timing_version;
ALTER TABLE request_logs DROP COLUMN upstream_stream_duration_ms;
ALTER TABLE usage_ledger DROP COLUMN upstream_stream_timing_version;
ALTER TABLE usage_ledger DROP COLUMN upstream_stream_duration_ms;
"#,
    )
    .expect("remove stream timing schema from drift fixture");
}

fn remove_final_attempt_timing_schema_for_drift_fixture(conn: &Connection) {
    conn.execute_batch(
        r#"
DROP VIEW IF EXISTS usage_events;
DROP TRIGGER IF EXISTS trg_usage_ledger_daily_rollup_insert;
DROP TRIGGER IF EXISTS trg_usage_ledger_daily_rollup_update;
DROP TRIGGER IF EXISTS trg_usage_ledger_daily_rollup_delete;
ALTER TABLE request_logs DROP COLUMN final_upstream_attempt_timing_version;
ALTER TABLE request_logs DROP COLUMN final_upstream_attempt_duration_ms;
ALTER TABLE usage_ledger DROP COLUMN final_upstream_attempt_timing_version;
ALTER TABLE usage_ledger DROP COLUMN final_upstream_attempt_duration_ms;
"#,
    )
    .expect("remove final-attempt timing schema from drift fixture");
}

#[test]
fn migrate_v47_to_v48_adds_stream_timing_and_invalidates_old_rollups() {
    let mut conn = Connection::open_in_memory().expect("open v47 migration db");
    apply_migrations(&mut conn).expect("create current schema fixture");
    let created_at = 1_700_000_000_i64;

    conn.execute(
        r#"
INSERT INTO request_logs(trace_id, cli_key, method, path, duration_ms, created_at)
VALUES ('trace-v47-stream-timing', 'codex', 'POST', '/v1/responses', 30000, ?1)
"#,
        [created_at],
    )
    .expect("insert v47 request log fixture");
    conn.execute(
        r#"
INSERT INTO usage_ledger(
  request_log_id,
  trace_id,
  cli_key,
  created_at,
  status,
  error_present,
  excluded_from_stats,
  duration_ms,
  final_provider_id,
  provider_name_snapshot
) VALUES (1, 'trace-v47-stream-timing', 'codex', ?1, 200, 0, 0, 30000, 7, 'Provider')
"#,
        [created_at],
    )
    .expect("insert v47 usage ledger fixture");
    conn.execute_batch(
        r#"
UPDATE usage_provider_daily_rollup_days
SET status = 'complete', source_row_count = 1, updated_at = 1;
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
  local_day,
  'codex',
  7,
  'Provider',
  'Provider',
  day_start_ts,
  day_start_ts,
  1,
  1,
  30000,
  100,
  1,
  1000,
  100,
  1,
  1,
  0
FROM usage_provider_daily_rollup_days;
UPDATE usage_provider_daily_rollup_backfill_state
SET next_local_day = '2023-01-01', updated_at = 1
WHERE id = 1;
"#,
    )
    .expect("seed stale v47 rollup state");

    remove_stream_timing_schema_for_drift_fixture(&conn);
    conn.pragma_update(None, "user_version", 47)
        .expect("mark fixture as v47");

    v47_to_v48::migrate_v47_to_v48(&mut conn).expect("migrate v47->v48");

    let version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read v48 version");
    assert_eq!(version, 48);
    for table in ["request_logs", "usage_ledger"] {
        for column in [
            "upstream_stream_duration_ms",
            "upstream_stream_timing_version",
        ] {
            let exists: bool = conn
                .query_row(
                    &format!(
                        "SELECT EXISTS(SELECT 1 FROM pragma_table_info('{table}') WHERE name = ?1)"
                    ),
                    [column],
                    |row| row.get(0),
                )
                .expect("inspect v48 timing column");
            assert!(exists, "missing {table}.{column}");
        }
    }

    let request_timing: (Option<i64>, i64) = conn
        .query_row(
            "SELECT upstream_stream_duration_ms, upstream_stream_timing_version FROM request_logs WHERE trace_id = 'trace-v47-stream-timing'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read migrated request timing");
    let ledger_timing: (Option<i64>, i64) = conn
        .query_row(
            "SELECT upstream_stream_duration_ms, upstream_stream_timing_version FROM usage_ledger WHERE trace_id = 'trace-v47-stream-timing'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read migrated ledger timing");
    let view_timing: (Option<i64>, i64) = conn
        .query_row(
            "SELECT upstream_stream_duration_ms, upstream_stream_timing_version FROM usage_events WHERE trace_id = 'trace-v47-stream-timing'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read migrated usage view timing");
    assert_eq!(request_timing, (None, 0));
    assert_eq!(ledger_timing, (None, 0));
    assert_eq!(view_timing, (None, 0));

    let stale_rollups: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM usage_provider_daily_rollups",
            [],
            |row| row.get(0),
        )
        .expect("count stale rollups");
    let day_status: String = conn
        .query_row(
            "SELECT status FROM usage_provider_daily_rollup_days LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("read invalidated rollup day");
    let cursor: Option<String> = conn
        .query_row(
            "SELECT next_local_day FROM usage_provider_daily_rollup_backfill_state WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .expect("read reset rollup cursor");
    assert_eq!(stale_rollups, 0);
    assert_eq!(day_status, "dirty");
    assert_eq!(cursor, None);

    conn.execute(
        "UPDATE usage_provider_daily_rollup_days SET status = 'complete', updated_at = 1",
        [],
    )
    .expect("mark migrated day complete");
    conn.execute(
        "UPDATE usage_ledger SET upstream_stream_duration_ms = 1000, upstream_stream_timing_version = 1 WHERE request_log_id = 1",
        [],
    )
    .expect("update migrated timing fields");
    let updated_day_status: String = conn
        .query_row(
            "SELECT status FROM usage_provider_daily_rollup_days LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("read day after timing update");
    assert_eq!(updated_day_status, "dirty");
}

#[test]
fn migrate_v47_to_v48_tolerates_missing_request_logs() {
    let mut conn = Connection::open_in_memory().expect("open partial v47 migration db");
    v42_to_v43::create_usage_ledger_schema(&conn).expect("create partial v47 ledger schema");
    conn.pragma_update(None, "user_version", 47)
        .expect("mark partial fixture as v47");

    v47_to_v48::migrate_v47_to_v48(&mut conn).expect("migrate partial v47->v48");

    let version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read partial fixture version");
    assert_eq!(version, 48);
    let request_logs_exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'request_logs')",
            [],
            |row| row.get(0),
        )
        .expect("inspect request_logs table");
    assert!(!request_logs_exists);
    let usage_events_exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'view' AND name = 'usage_events')",
            [],
            |row| row.get(0),
        )
        .expect("inspect usage_events view");
    assert!(!usage_events_exists);
    for column in [
        "upstream_stream_duration_ms",
        "upstream_stream_timing_version",
    ] {
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_table_info('usage_ledger') WHERE name = ?1)",
                [column],
                |row| row.get(0),
            )
            .expect("inspect usage ledger timing column");
        assert!(exists, "missing usage_ledger.{column}");
    }
}

#[test]
fn migrate_v48_to_v49_adds_final_attempt_timing_without_backfilling_old_samples() {
    let mut conn = Connection::open_in_memory().expect("open v48 migration db");
    apply_migrations(&mut conn).expect("create current schema fixture");
    let created_at = 1_700_000_000_i64;

    conn.execute(
        r#"
INSERT INTO request_logs(
  trace_id,
  cli_key,
  method,
  path,
  duration_ms,
  created_at,
  upstream_stream_duration_ms,
  upstream_stream_timing_version
) VALUES (
  'trace-v48-final-attempt',
  'codex',
  'POST',
  '/v1/responses',
  30000,
  ?1,
  1000,
  1
)
"#,
        [created_at],
    )
    .expect("insert v48 request log fixture");
    conn.execute(
        r#"
INSERT INTO usage_ledger(
  request_log_id,
  trace_id,
  cli_key,
  created_at,
  status,
  error_present,
  excluded_from_stats,
  duration_ms,
  final_provider_id,
  provider_name_snapshot,
  upstream_stream_duration_ms,
  upstream_stream_timing_version
) VALUES (
  1,
  'trace-v48-final-attempt',
  'codex',
  ?1,
  200,
  0,
  0,
  30000,
  7,
  'Provider',
  1000,
  1
)
"#,
        [created_at],
    )
    .expect("insert v48 usage ledger fixture");
    conn.execute_batch(
        r#"
UPDATE usage_provider_daily_rollup_days
SET status = 'complete', source_row_count = 1, updated_at = 1;
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
  local_day,
  'codex',
  7,
  'Provider',
  'Provider',
  day_start_ts,
  day_start_ts,
  1,
  1,
  30000,
  100,
  1,
  1000,
  100,
  1,
  1,
  0
FROM usage_provider_daily_rollup_days;
UPDATE usage_provider_daily_rollup_backfill_state
SET next_local_day = '2023-01-01', updated_at = 1
WHERE id = 1;
"#,
    )
    .expect("seed stale v48 rollup state");

    remove_final_attempt_timing_schema_for_drift_fixture(&conn);
    conn.pragma_update(None, "user_version", 48)
        .expect("mark fixture as v48");

    v48_to_v49::migrate_v48_to_v49(&mut conn).expect("migrate v48->v49");

    let version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read v49 version");
    assert_eq!(version, 49);
    for table in ["request_logs", "usage_ledger"] {
        for column in [
            "final_upstream_attempt_duration_ms",
            "final_upstream_attempt_timing_version",
        ] {
            let exists: bool = conn
                .query_row(
                    &format!(
                        "SELECT EXISTS(SELECT 1 FROM pragma_table_info('{table}') WHERE name = ?1)"
                    ),
                    [column],
                    |row| row.get(0),
                )
                .expect("inspect v49 timing column");
            assert!(exists, "missing {table}.{column}");
        }
    }

    let request_timing: (Option<i64>, i64) = conn
        .query_row(
            "SELECT final_upstream_attempt_duration_ms, final_upstream_attempt_timing_version FROM request_logs WHERE trace_id = 'trace-v48-final-attempt'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read migrated request timing");
    let ledger_timing: (Option<i64>, i64) = conn
        .query_row(
            "SELECT final_upstream_attempt_duration_ms, final_upstream_attempt_timing_version FROM usage_ledger WHERE trace_id = 'trace-v48-final-attempt'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read migrated ledger timing");
    let view_timing: (Option<i64>, i64) = conn
        .query_row(
            "SELECT final_upstream_attempt_duration_ms, final_upstream_attempt_timing_version FROM usage_events WHERE trace_id = 'trace-v48-final-attempt'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read migrated usage view timing");
    assert_eq!(request_timing, (None, 0));
    assert_eq!(ledger_timing, (None, 0));
    assert_eq!(view_timing, (None, 0));

    let stale_rollups: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM usage_provider_daily_rollups",
            [],
            |row| row.get(0),
        )
        .expect("count stale rollups");
    let day_status: String = conn
        .query_row(
            "SELECT status FROM usage_provider_daily_rollup_days LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("read invalidated rollup day");
    let cursor: Option<String> = conn
        .query_row(
            "SELECT next_local_day FROM usage_provider_daily_rollup_backfill_state WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .expect("read reset rollup cursor");
    assert_eq!(stale_rollups, 0);
    assert_eq!(day_status, "dirty");
    assert_eq!(cursor, None);

    conn.execute(
        "UPDATE usage_provider_daily_rollup_days SET status = 'complete', updated_at = 1",
        [],
    )
    .expect("mark migrated day complete");
    conn.execute(
        "UPDATE usage_ledger SET final_upstream_attempt_duration_ms = 30000, final_upstream_attempt_timing_version = 1 WHERE request_log_id = 1",
        [],
    )
    .expect("update migrated final-attempt timing fields");
    let updated_day_status: String = conn
        .query_row(
            "SELECT status FROM usage_provider_daily_rollup_days LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("read day after final-attempt timing update");
    assert_eq!(updated_day_status, "dirty");
}

#[test]
fn ensure_repairs_missing_v49_final_attempt_timing_columns_before_views_and_triggers() {
    let mut conn = Connection::open_in_memory().expect("open drifted v49 db");
    apply_migrations(&mut conn).expect("create current schema fixture");
    remove_final_attempt_timing_schema_for_drift_fixture(&conn);

    apply_migrations(&mut conn).expect("repair drifted v49 schema");

    let timing_columns: i64 = conn
        .query_row(
            r#"
SELECT
  (SELECT COUNT(*) FROM pragma_table_info('request_logs')
   WHERE name IN (
     'final_upstream_attempt_duration_ms',
     'final_upstream_attempt_timing_version'
   ))
  +
  (SELECT COUNT(*) FROM pragma_table_info('usage_ledger')
   WHERE name IN (
     'final_upstream_attempt_duration_ms',
     'final_upstream_attempt_timing_version'
   ))
"#,
            [],
            |row| row.get(0),
        )
        .expect("count repaired final-attempt timing columns");
    assert_eq!(timing_columns, 4);
    let repaired_objects: i64 = conn
        .query_row(
            r#"
SELECT COUNT(*)
FROM sqlite_master
WHERE (type = 'view' AND name = 'usage_events')
   OR (type = 'trigger' AND name IN (
     'trg_usage_ledger_daily_rollup_insert',
     'trg_usage_ledger_daily_rollup_update',
     'trg_usage_ledger_daily_rollup_delete'
   ))
"#,
            [],
            |row| row.get(0),
        )
        .expect("count repaired view and triggers");
    assert_eq!(repaired_objects, 4);
}

#[test]
fn ensure_repairs_missing_v48_stream_timing_columns_before_views_and_triggers() {
    let mut conn = Connection::open_in_memory().expect("open drifted v48 db");
    apply_migrations(&mut conn).expect("create current schema fixture");
    remove_stream_timing_schema_for_drift_fixture(&conn);

    apply_migrations(&mut conn).expect("repair drifted v48 schema");

    let timing_columns: i64 = conn
        .query_row(
            r#"
SELECT
  (SELECT COUNT(*) FROM pragma_table_info('request_logs')
   WHERE name IN ('upstream_stream_duration_ms', 'upstream_stream_timing_version'))
  +
  (SELECT COUNT(*) FROM pragma_table_info('usage_ledger')
   WHERE name IN ('upstream_stream_duration_ms', 'upstream_stream_timing_version'))
"#,
            [],
            |row| row.get(0),
        )
        .expect("count repaired timing columns");
    assert_eq!(timing_columns, 4);
    let repaired_objects: i64 = conn
        .query_row(
            r#"
SELECT COUNT(*)
FROM sqlite_master
WHERE (type = 'view' AND name = 'usage_events')
   OR (type = 'trigger' AND name IN (
     'trg_usage_ledger_daily_rollup_insert',
     'trg_usage_ledger_daily_rollup_update',
     'trg_usage_ledger_daily_rollup_delete'
   ))
"#,
            [],
            |row| row.get(0),
        )
        .expect("count repaired view and triggers");
    assert_eq!(repaired_objects, 4);
}

#[test]
fn ensure_restores_daily_rollup_triggers_and_invalidates_untracked_writes() {
    let mut conn = Connection::open_in_memory().expect("open current migration db");
    apply_migrations(&mut conn).expect("create current schema");
    let missed_created_at = 1_700_000_000_i64;
    let (local_day, day_start_ts, day_end_ts): (String, i64, i64) = conn
        .query_row(
            r#"
SELECT
  date(?1, 'unixepoch', 'localtime'),
  CAST(strftime('%s', date(?1, 'unixepoch', 'localtime'), 'utc') AS INTEGER),
  CAST(strftime(
    '%s', date(?1, 'unixepoch', 'localtime', '+1 day'), 'utc'
  ) AS INTEGER)
"#,
            [missed_created_at],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("resolve stale projection day");
    conn.execute(
        r#"
INSERT INTO usage_provider_daily_rollup_days(
  local_day,
  day_start_ts,
  day_end_ts,
  status,
  source_row_count,
  updated_at
) VALUES (?1, ?2, ?3, 'complete', 0, 1)
"#,
        rusqlite::params![local_day, day_start_ts, day_end_ts],
    )
    .expect("create complete empty projection day");
    conn.execute_batch("DROP TRIGGER trg_usage_ledger_daily_rollup_insert;")
        .expect("drop daily rollup trigger");
    conn.execute(
        r#"
INSERT INTO usage_ledger(
  request_log_id,
  trace_id,
  cli_key,
  created_at,
  status,
  error_present,
  excluded_from_stats,
  duration_ms,
  final_provider_id,
  provider_name_snapshot
) VALUES (1, 'missed-daily-rollup-write', 'codex', ?1, 200, 0, 0, 100, 7, 'Missed')
"#,
        [missed_created_at],
    )
    .expect("write ledger row while the insert trigger is absent");

    apply_migrations(&mut conn).expect("repair daily rollup trigger");

    let exists: bool = conn
        .query_row(
            r#"
SELECT EXISTS(
  SELECT 1
  FROM sqlite_master
  WHERE type = 'trigger' AND name = 'trg_usage_ledger_daily_rollup_insert'
)
"#,
            [],
            |row| row.get(0),
        )
        .expect("inspect repaired daily rollup trigger");
    assert!(exists);
    let projection_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM usage_provider_daily_rollup_days",
            [],
            |row| row.get(0),
        )
        .expect("count invalidated daily projection days");
    assert_eq!(projection_rows, 0);
    let cursor: Option<String> = conn
        .query_row(
            "SELECT next_local_day FROM usage_provider_daily_rollup_backfill_state WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .expect("read reset daily rollup cursor");
    assert_eq!(cursor, None);
}

#[test]
fn migrate_v46_to_v47_recovers_a_missing_usage_ledger_before_adding_triggers() {
    let mut conn = Connection::open_in_memory().expect("open drifted v46 migration db");
    v42_to_v43::create_usage_ledger_schema(&conn).expect("create usage ledger drift fixture");
    conn.execute_batch(
        r#"
INSERT INTO usage_ledger_backfill_state(
  id,
  status,
  target_request_log_id,
  last_request_log_id,
  completed_at,
  updated_at
) VALUES (1, 'complete', 100, 100, 1, 1);
DROP TABLE usage_ledger;
PRAGMA user_version = 46;
"#,
    )
    .expect("create drifted v46 fixture");

    v46_to_v47::migrate_v46_to_v47(&mut conn).expect("migrate drifted v46->v47");

    for name in [
        "usage_ledger",
        "usage_ledger_backfill_state",
        "usage_provider_daily_rollup_days",
        "usage_provider_daily_rollups",
    ] {
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                [name],
                |row| row.get(0),
            )
            .expect("inspect recovered table");
        assert!(exists, "missing recovered table {name}");
    }
    let trigger_exists: bool = conn
        .query_row(
            r#"
SELECT EXISTS(
  SELECT 1
  FROM sqlite_master
  WHERE type = 'trigger' AND name = 'trg_usage_ledger_daily_rollup_insert'
)
"#,
            [],
            |row| row.get(0),
        )
        .expect("inspect recovered daily rollup trigger");
    assert!(trigger_exists);

    let stale_state_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM usage_ledger_backfill_state",
            [],
            |row| row.get(0),
        )
        .expect("inspect reset usage ledger backfill state");
    assert_eq!(
        stale_state_rows, 0,
        "a recreated ledger must not inherit a stale complete marker"
    );
}

#[test]
fn recovery_journal_migrates_v49_and_v50_with_idempotent_claim_schema() {
    let mut conn = Connection::open_in_memory().expect("open recovery migration db");
    conn.pragma_update(None, "user_version", 49_i64)
        .expect("set v49 fixture");

    v49_to_v50::migrate_v49_to_v50(&mut conn).expect("migrate v49->v50");
    let version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read v50 version");
    assert_eq!(version, 50);
    assert!(test_has_table(&conn, "external_effect_recovery_journal"));

    v50_to_v51::migrate_v50_to_v51(&mut conn).expect("migrate v50->v51");
    v50_to_v51::create_recovery_claim_schema(&conn).expect("repeat recovery ensure");
    let version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read v51 version");
    assert_eq!(version, 51);
    for column in ["lease_owner", "lease_expires_at", "claim_epoch", "replay_context"] {
        assert!(test_has_column(
            &conn,
            "external_effect_recovery_journal",
            column
        ));
    }
    assert!(test_has_table(
        &conn,
        "external_effect_recovery_coordinator"
    ));
    let workspace_index: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = 'idx_external_effect_recovery_workspace_apply_cli')",
            [],
            |row| row.get(0),
        )
        .expect("inspect workspace recovery index");
    assert!(workspace_index);
}

#[test]
fn fresh_schema_contains_recovery_journal_and_coordinator() {
    let mut conn = Connection::open_in_memory().expect("open fresh migration db");
    apply_migrations(&mut conn).expect("create fresh schema");

    assert!(test_has_table(&conn, "external_effect_recovery_journal"));
    assert!(test_has_table(
        &conn,
        "external_effect_recovery_coordinator"
    ));
    apply_migrations(&mut conn).expect("repeat current schema ensure");
}
