//! Usage: Sort mode persistence and provider ordering configuration helpers.

use crate::db;
use crate::providers::MAX_SESSION_REUSE_PRIORITY;
use crate::shared::error::db_err;
use crate::shared::time::now_unix_seconds;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::collections::{HashMap, HashSet};

const MAX_SORT_MODE_NAME_CHARS: usize = 32;
const MAX_SORT_MODE_PROVIDER_IDS: usize = 512;

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct SortModeSummary {
    pub id: i64,
    pub mode_uuid: String,
    pub name: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct SortModeActiveRow {
    pub cli_key: String,
    pub mode_id: Option<i64>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct SortModeProviderRow {
    pub provider_id: i64,
    pub provider_uuid: String,
    pub enabled: bool,
    pub session_reuse_priority: i64,
    pub cross_policy: Option<crate::settings::CrossProviderModelRoutingPolicy>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct SortModeRoutingContext {
    pub mode_id: i64,
    pub mode_uuid: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ProviderModelRoutingPolicyView {
    pub provider_id: i64,
    pub provider_uuid: String,
    pub cli_key: String,
    pub provider_override_enabled: bool,
    pub ordinary_policy: crate::settings::ModelRoutingPolicy,
    pub ordinary_policy_revision: String,
    pub selected_mode: Option<SortModeRoutingContext>,
    pub cross_policy: Option<crate::settings::CrossProviderModelRoutingPolicy>,
    pub cross_policy_revision: Option<String>,
    pub source_member_enabled: bool,
    pub source_member_present: bool,
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProviderModelRoutingPolicySaveInput {
    pub provider_id: i64,
    pub provider_uuid: String,
    pub mode_id: Option<i64>,
    pub mode_uuid: Option<String>,
    pub provider_override_enabled: bool,
    pub ordinary_policy: crate::settings::ModelRoutingPolicy,
    pub expected_ordinary_policy_revision: String,
    pub cross_policy: Option<crate::settings::CrossProviderModelRoutingPolicy>,
    pub expected_cross_policy_revision: Option<String>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct RoutingProviderCandidate {
    pub provider_id: i64,
    pub provider_uuid: String,
    pub cli_key: String,
    pub name: String,
    pub enabled: bool,
    pub source_provider_id: Option<i64>,
    pub bridge_type: Option<String>,
    pub model_catalog_supported: bool,
}

fn enabled_to_int(enabled: bool) -> i64 {
    if enabled {
        1
    } else {
        0
    }
}

fn enabled_from_int(value: i64) -> bool {
    value != 0
}

fn cross_policy_from_json(
    raw: Option<String>,
) -> Option<crate::settings::CrossProviderModelRoutingPolicy> {
    let mut policy = serde_json::from_str::<crate::settings::CrossProviderModelRoutingPolicy>(
        raw.as_deref()?.trim(),
    )
    .ok()?;
    crate::settings::sanitize_cross_provider_model_routing_policy(&mut policy);
    Some(policy)
}

fn cross_policy_to_json(
    policy: &mut Option<crate::settings::CrossProviderModelRoutingPolicy>,
) -> crate::shared::error::AppResult<Option<String>> {
    let Some(policy) = policy.as_mut() else {
        return Ok(None);
    };
    crate::settings::normalize_cross_provider_model_routing_policy_for_write(policy)?;
    serde_json::to_string(&policy)
        .map(Some)
        .map_err(|e| format!("SYSTEM_ERROR: failed to serialize cross-provider policy: {e}").into())
}

fn routing_policy_revision(scope: &str, raw: Option<&str>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(scope.as_bytes());
    hasher.update([0]);
    match raw {
        Some(raw) => {
            hasher.update([1]);
            hasher.update(raw.as_bytes());
        }
        None => hasher.update([0]),
    }
    format!("{:x}", hasher.finalize())
}

fn is_routing_policy_revision(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn validate_session_reuse_priority(
    session_reuse_priority: i64,
) -> crate::shared::error::AppResult<()> {
    if !(0..=MAX_SESSION_REUSE_PRIORITY).contains(&session_reuse_priority) {
        return Err(format!(
            "SEC_INVALID_INPUT: session_reuse_priority must be between 0 and {MAX_SESSION_REUSE_PRIORITY}"
        )
        .into());
    }
    Ok(())
}

fn validate_cli_key(cli_key: &str) -> crate::shared::error::AppResult<()> {
    crate::shared::cli_key::validate_cli_key(cli_key)
}

fn validate_mode_name(name: &str) -> crate::shared::error::AppResult<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("SEC_INVALID_INPUT: mode name is required".into());
    }

    if name.chars().nth(MAX_SORT_MODE_NAME_CHARS).is_some() {
        return Err(format!(
            "SEC_INVALID_INPUT: mode name is too long (max {MAX_SORT_MODE_NAME_CHARS} chars)"
        )
        .into());
    }

    let lowered = name.to_ascii_lowercase();
    if lowered == "default" || name == "默认" {
        return Err("SEC_INVALID_INPUT: mode name is reserved".into());
    }

    Ok(name.to_string())
}

fn row_to_mode_summary(row: &rusqlite::Row<'_>) -> Result<SortModeSummary, rusqlite::Error> {
    Ok(SortModeSummary {
        id: row.get("id")?,
        mode_uuid: row.get("mode_uuid")?,
        name: row.get("name")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn ensure_mode_exists(conn: &Connection, mode_id: i64) -> crate::shared::error::AppResult<()> {
    if mode_id <= 0 {
        return Err("SEC_INVALID_INPUT: invalid mode_id".into());
    }

    let exists: Option<i64> = conn
        .query_row(
            "SELECT id FROM sort_modes WHERE id = ?1",
            params![mode_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| db_err!("failed to query sort_mode: {e}"))?;

    if exists.is_none() {
        return Err("DB_NOT_FOUND: sort_mode not found".into());
    }

    Ok(())
}

fn read_active_row(
    conn: &Connection,
    cli_key: &str,
) -> crate::shared::error::AppResult<SortModeActiveRow> {
    conn.query_row(
        r#"
SELECT
  cli_key,
  mode_id,
  updated_at
FROM sort_mode_active
WHERE cli_key = ?1
"#,
        params![cli_key],
        |row| {
            Ok(SortModeActiveRow {
                cli_key: row.get("cli_key")?,
                mode_id: row.get("mode_id")?,
                updated_at: row.get("updated_at")?,
            })
        },
    )
    .optional()
    .map_err(|e| db_err!("failed to query sort_mode_active: {e}"))?
    .ok_or_else(|| "DB_NOT_FOUND: sort_mode_active not found".into())
}

pub fn list_modes(db: &db::Db) -> crate::shared::error::AppResult<Vec<SortModeSummary>> {
    let conn = db.open_connection()?;
    let mut stmt = conn
        .prepare_cached(
            r#"
SELECT
  mode.id AS id,
  identity.mode_uuid AS mode_uuid,
  mode.name AS name,
  mode.created_at AS created_at,
  mode.updated_at AS updated_at
FROM sort_modes mode
JOIN sort_mode_identities identity ON identity.mode_id = mode.id
ORDER BY mode.id ASC
"#,
        )
        .map_err(|e| db_err!("failed to prepare sort_modes query: {e}"))?;

    let rows = stmt
        .query_map([], row_to_mode_summary)
        .map_err(|e| db_err!("failed to list sort_modes: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| db_err!("failed to read sort_mode row: {e}"))?);
    }
    Ok(items)
}

pub fn create_mode(db: &db::Db, name: &str) -> crate::shared::error::AppResult<SortModeSummary> {
    let name = validate_mode_name(name)?;
    let mut conn = db.open_connection()?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| db_err!("failed to start sort-mode create transaction: {e}"))?;
    let now = now_unix_seconds();

    tx.execute(
        r#"
INSERT INTO sort_modes(
  name,
  created_at,
  updated_at
) VALUES (?1, ?2, ?3)
"#,
        params![name, now, now],
    )
    .map_err(|e| match e {
        rusqlite::Error::SqliteFailure(err, _)
            if err.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            crate::shared::error::AppError::new(
                "DB_CONSTRAINT",
                format!("sort_mode already exists: name={name}"),
            )
        }
        other => db_err!("failed to insert sort_mode: {other}"),
    })?;

    let id = tx.last_insert_rowid();
    tx.execute(
        "INSERT INTO sort_mode_identities(mode_id, mode_uuid) VALUES (?1, ?2)",
        params![id, crate::shared::uuid::new_uuid_v4()],
    )
    .map_err(|e| db_err!("failed to insert sort-mode identity: {e}"))?;
    let result = tx
        .query_row(
            r#"
SELECT
  mode.id AS id,
  identity.mode_uuid AS mode_uuid,
  mode.name AS name,
  mode.created_at AS created_at,
  mode.updated_at AS updated_at
FROM sort_modes mode
JOIN sort_mode_identities identity ON identity.mode_id = mode.id
WHERE mode.id = ?1
"#,
            params![id],
            row_to_mode_summary,
        )
        .map_err(|e| db_err!("failed to query inserted sort_mode: {e}"))?;
    tx.commit()
        .map_err(|e| db_err!("failed to commit sort-mode create transaction: {e}"))?;
    Ok(result)
}

pub fn rename_mode(
    db: &db::Db,
    mode_id: i64,
    name: &str,
) -> crate::shared::error::AppResult<SortModeSummary> {
    let name = validate_mode_name(name)?;
    let conn = db.open_connection()?;
    ensure_mode_exists(&conn, mode_id)?;
    let now = now_unix_seconds();

    conn.execute(
        "UPDATE sort_modes SET name = ?1, updated_at = ?2 WHERE id = ?3",
        params![name, now, mode_id],
    )
    .map_err(|e| match e {
        rusqlite::Error::SqliteFailure(err, _)
            if err.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            crate::shared::error::AppError::new(
                "DB_CONSTRAINT",
                format!("sort_mode already exists: name={name}"),
            )
        }
        other => db_err!("failed to update sort_mode: {other}"),
    })?;

    conn.query_row(
        r#"
SELECT
  mode.id AS id,
  identity.mode_uuid AS mode_uuid,
  mode.name AS name,
  mode.created_at AS created_at,
  mode.updated_at AS updated_at
FROM sort_modes mode
JOIN sort_mode_identities identity ON identity.mode_id = mode.id
WHERE mode.id = ?1
"#,
        params![mode_id],
        row_to_mode_summary,
    )
    .map_err(|e| db_err!("failed to query sort_mode: {e}"))
}

pub fn delete_mode_with_affected_cli_keys(
    db: &db::Db,
    mode_id: i64,
) -> crate::shared::error::AppResult<Vec<String>> {
    let conn = db.open_connection()?;
    ensure_mode_exists(&conn, mode_id)?;

    let mut stmt = conn
        .prepare_cached(
            r#"
SELECT cli_key
FROM sort_mode_active
WHERE mode_id = ?1
ORDER BY cli_key ASC
"#,
        )
        .map_err(|e| db_err!("failed to prepare active sort_mode query: {e}"))?;
    let rows = stmt
        .query_map(params![mode_id], |row| row.get::<_, String>(0))
        .map_err(|e| db_err!("failed to query active sort_mode cli keys: {e}"))?;

    let mut affected_cli_keys = Vec::new();
    for row in rows {
        affected_cli_keys
            .push(row.map_err(|e| db_err!("failed to read active sort_mode cli key: {e}"))?);
    }
    drop(stmt);

    let changed = conn
        .execute("DELETE FROM sort_modes WHERE id = ?1", params![mode_id])
        .map_err(|e| db_err!("failed to delete sort_mode: {e}"))?;
    if changed == 0 {
        return Err("DB_NOT_FOUND: sort_mode not found".to_string().into());
    }
    Ok(affected_cli_keys)
}

pub fn delete_mode(db: &db::Db, mode_id: i64) -> crate::shared::error::AppResult<()> {
    delete_mode_with_affected_cli_keys(db, mode_id)?;
    Ok(())
}

pub fn list_active(db: &db::Db) -> crate::shared::error::AppResult<Vec<SortModeActiveRow>> {
    let conn = db.open_connection()?;
    let mut stmt = conn
        .prepare_cached(
            r#"
SELECT
  cli_key,
  mode_id,
  updated_at
FROM sort_mode_active
ORDER BY cli_key ASC
"#,
        )
        .map_err(|e| db_err!("failed to prepare sort_mode_active query: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(SortModeActiveRow {
                cli_key: row.get("cli_key")?,
                mode_id: row.get("mode_id")?,
                updated_at: row.get("updated_at")?,
            })
        })
        .map_err(|e| db_err!("failed to list sort_mode_active: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| db_err!("failed to read sort_mode_active row: {e}"))?);
    }
    Ok(items)
}

pub fn set_active(
    db: &db::Db,
    cli_key: &str,
    mode_id: Option<i64>,
) -> crate::shared::error::AppResult<SortModeActiveRow> {
    let cli_key = cli_key.trim();
    validate_cli_key(cli_key)?;

    let mut conn = db.open_connection()?;
    // Reserve the WAL writer before validation reads so concurrent switches cannot
    // fail while upgrading a stale deferred-transaction snapshot.
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| db_err!("failed to start transaction: {e}"))?;
    if let Some(mode_id) = mode_id {
        ensure_mode_exists(&tx, mode_id)?;
    }
    let now = now_unix_seconds();

    tx.execute(
        r#"
INSERT INTO sort_mode_active(
  cli_key,
  mode_id,
  updated_at
) VALUES (?1, ?2, ?3)
ON CONFLICT(cli_key) DO UPDATE SET
  mode_id = excluded.mode_id,
  updated_at = excluded.updated_at
"#,
        params![cli_key, mode_id, now],
    )
    .map_err(|e| db_err!("failed to upsert sort_mode_active: {e}"))?;

    let row = read_active_row(&tx, cli_key)?;
    tx.commit()
        .map_err(|e| db_err!("failed to commit transaction: {e}"))?;
    Ok(row)
}

pub fn list_mode_providers(
    db: &db::Db,
    mode_id: i64,
    cli_key: &str,
) -> crate::shared::error::AppResult<Vec<SortModeProviderRow>> {
    let cli_key = cli_key.trim();
    validate_cli_key(cli_key)?;
    let conn = db.open_connection()?;
    ensure_mode_exists(&conn, mode_id)?;

    let mut stmt = conn
        .prepare_cached(
            r#"
SELECT
  member.provider_id,
  provider.provider_uuid,
  member.enabled,
  member.session_reuse_priority,
  member.cross_provider_model_routing_policy_json
FROM sort_mode_providers member
JOIN providers provider ON provider.id = member.provider_id
WHERE member.mode_id = ?1
  AND member.cli_key = ?2
ORDER BY member.sort_order ASC
"#,
        )
        .map_err(|e| db_err!("failed to prepare sort_mode_providers query: {e}"))?;

    let rows = stmt
        .query_map(params![mode_id, cli_key], |row| {
            let provider_id: i64 = row.get(0)?;
            let enabled_raw: i64 = row.get(2)?;
            Ok(SortModeProviderRow {
                provider_id,
                provider_uuid: row.get(1)?,
                enabled: enabled_from_int(enabled_raw),
                session_reuse_priority: row.get(3)?,
                cross_policy: cross_policy_from_json(row.get(4)?),
            })
        })
        .map_err(|e| db_err!("failed to list sort_mode_providers: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| db_err!("failed to read sort_mode_provider row: {e}"))?);
    }
    Ok(items)
}

fn ensure_providers_belong_to_cli(
    conn: &Connection,
    cli_key: &str,
    provider_ids: &[i64],
) -> crate::shared::error::AppResult<()> {
    if provider_ids.is_empty() {
        return Ok(());
    }

    if provider_ids.len() > MAX_SORT_MODE_PROVIDER_IDS {
        return Err(format!(
            "SEC_INVALID_INPUT: ordered_provider_ids must contain at most {MAX_SORT_MODE_PROVIDER_IDS} entries"
        )
        .into());
    }

    let mut unique_ids = HashSet::new();
    for id in provider_ids {
        if *id <= 0 {
            return Err(format!("SEC_INVALID_INPUT: invalid provider_id={id}").into());
        }
        if !unique_ids.insert(*id) {
            return Err(format!("SEC_INVALID_INPUT: duplicate provider_id={id}").into());
        }
    }

    let placeholders = db::sql_placeholders(unique_ids.len());
    let sql = format!("SELECT id FROM providers WHERE cli_key = ?1 AND id IN ({placeholders})");

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| db_err!("failed to prepare provider validation query: {e}"))?;

    let mut params_vec: Vec<rusqlite::types::Value> = Vec::with_capacity(unique_ids.len() + 1);
    params_vec.push(rusqlite::types::Value::from(cli_key.to_string()));
    params_vec.extend(unique_ids.iter().map(|id| (*id).into()));

    let rows = stmt
        .query_map(params_from_iter(params_vec), |row| row.get::<_, i64>(0))
        .map_err(|e| db_err!("failed to query provider validation: {e}"))?;

    let mut found = HashSet::new();
    for row in rows {
        found.insert(row.map_err(|e| db_err!("failed to read provider id: {e}"))?);
    }

    if found.len() != unique_ids.len() {
        let missing: Vec<i64> = unique_ids.difference(&found).copied().collect();
        return Err(format!(
            "SEC_INVALID_INPUT: provider_id does not belong to cli_key={cli_key}: {missing:?}"
        )
        .into());
    }

    Ok(())
}

pub fn set_mode_providers_order(
    db: &db::Db,
    mode_id: i64,
    cli_key: &str,
    ordered_provider_ids: Vec<i64>,
) -> crate::shared::error::AppResult<Vec<SortModeProviderRow>> {
    let cli_key = cli_key.trim();
    validate_cli_key(cli_key)?;

    let mut conn = db.open_connection()?;
    ensure_mode_exists(&conn, mode_id)?;
    ensure_providers_belong_to_cli(&conn, cli_key, &ordered_provider_ids)?;

    let tx = conn
        .transaction()
        .map_err(|e| db_err!("failed to start transaction: {e}"))?;

    let mut existing_rows: HashMap<i64, (bool, i64, Option<String>)> = HashMap::new();
    {
        let mut stmt = tx
            .prepare_cached(
                r#"
SELECT
  provider_id,
  enabled,
  session_reuse_priority,
  cross_provider_model_routing_policy_json
FROM sort_mode_providers
WHERE mode_id = ?1
  AND cli_key = ?2
"#,
            )
            .map_err(|e| db_err!("failed to prepare sort_mode_providers query: {e}"))?;
        let rows = stmt
            .query_map(params![mode_id, cli_key], |row| {
                let provider_id: i64 = row.get(0)?;
                let enabled_raw: i64 = row.get(1)?;
                let session_reuse_priority: i64 = row.get(2)?;
                let cross_policy_json: Option<String> = row.get(3)?;
                Ok((
                    provider_id,
                    enabled_from_int(enabled_raw),
                    session_reuse_priority,
                    cross_policy_json,
                ))
            })
            .map_err(|e| db_err!("failed to list sort_mode_providers: {e}"))?;
        for row in rows {
            let (provider_id, enabled, session_reuse_priority, cross_policy_json) =
                row.map_err(|e| db_err!("failed to read sort_mode_provider row: {e}"))?;
            existing_rows.insert(
                provider_id,
                (enabled, session_reuse_priority, cross_policy_json),
            );
        }
    }

    tx.execute(
        "DELETE FROM sort_mode_providers WHERE mode_id = ?1 AND cli_key = ?2",
        params![mode_id, cli_key],
    )
    .map_err(|e| db_err!("failed to clear sort_mode_providers: {e}"))?;

    let now = now_unix_seconds();
    for (idx, provider_id) in ordered_provider_ids.iter().enumerate() {
        let (enabled, session_reuse_priority, cross_policy_json) = existing_rows
            .get(provider_id)
            .cloned()
            .unwrap_or((true, 0, None));
        tx.execute(
            r#"
INSERT INTO sort_mode_providers(
  mode_id,
  cli_key,
  provider_id,
  sort_order,
  enabled,
  session_reuse_priority,
  cross_provider_model_routing_policy_json,
  created_at,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
"#,
            params![
                mode_id,
                cli_key,
                provider_id,
                idx as i64,
                enabled_to_int(enabled),
                session_reuse_priority,
                cross_policy_json,
                now,
                now
            ],
        )
        .map_err(|e| db_err!("failed to insert sort_mode_provider: {e}"))?;
    }

    tx.commit()
        .map_err(|e| db_err!("failed to commit transaction: {e}"))?;
    drop(conn);

    list_mode_providers(db, mode_id, cli_key)
}

pub fn set_mode_provider_enabled(
    db: &db::Db,
    mode_id: i64,
    cli_key: &str,
    provider_id: i64,
    enabled: bool,
) -> crate::shared::error::AppResult<SortModeProviderRow> {
    let cli_key = cli_key.trim();
    validate_cli_key(cli_key)?;
    if provider_id <= 0 {
        return Err("SEC_INVALID_INPUT: invalid provider_id".into());
    }

    let conn = db.open_connection()?;
    ensure_mode_exists(&conn, mode_id)?;
    ensure_providers_belong_to_cli(&conn, cli_key, &[provider_id])?;

    let now = now_unix_seconds();
    let changed = conn
        .execute(
            r#"
UPDATE sort_mode_providers
SET enabled = ?1, updated_at = ?2
WHERE mode_id = ?3
  AND cli_key = ?4
  AND provider_id = ?5
"#,
            params![enabled_to_int(enabled), now, mode_id, cli_key, provider_id],
        )
        .map_err(|e| db_err!("failed to update sort_mode_provider: {e}"))?;
    if changed == 0 {
        return Err("DB_NOT_FOUND: sort_mode_provider not found".into());
    }

    conn.query_row(
        r#"
SELECT
  member.provider_id,
  provider.provider_uuid,
  member.enabled,
  member.session_reuse_priority,
  member.cross_provider_model_routing_policy_json
FROM sort_mode_providers member
JOIN providers provider ON provider.id = member.provider_id
WHERE member.mode_id = ?1
  AND member.cli_key = ?2
  AND member.provider_id = ?3
"#,
        params![mode_id, cli_key, provider_id],
        |row| {
            let provider_id: i64 = row.get(0)?;
            let enabled_raw: i64 = row.get(2)?;
            Ok(SortModeProviderRow {
                provider_id,
                provider_uuid: row.get(1)?,
                enabled: enabled_from_int(enabled_raw),
                session_reuse_priority: row.get(3)?,
                cross_policy: cross_policy_from_json(row.get(4)?),
            })
        },
    )
    .map_err(|e| db_err!("failed to read sort_mode_provider: {e}"))
}

pub fn set_mode_provider_session_reuse_priority(
    db: &db::Db,
    mode_id: i64,
    cli_key: &str,
    provider_id: i64,
    session_reuse_priority: i64,
) -> crate::shared::error::AppResult<SortModeProviderRow> {
    let cli_key = cli_key.trim();
    validate_cli_key(cli_key)?;
    if provider_id <= 0 {
        return Err("SEC_INVALID_INPUT: invalid provider_id".into());
    }
    validate_session_reuse_priority(session_reuse_priority)?;

    let conn = db.open_connection()?;
    ensure_mode_exists(&conn, mode_id)?;
    ensure_providers_belong_to_cli(&conn, cli_key, &[provider_id])?;

    let changed = conn
        .execute(
            r#"
UPDATE sort_mode_providers
SET session_reuse_priority = ?1, updated_at = ?2
WHERE mode_id = ?3 AND cli_key = ?4 AND provider_id = ?5
"#,
            params![
                session_reuse_priority,
                now_unix_seconds(),
                mode_id,
                cli_key,
                provider_id
            ],
        )
        .map_err(|e| db_err!("failed to update sort_mode session reuse priority: {e}"))?;
    if changed == 0 {
        return Err("DB_NOT_FOUND: sort_mode_provider not found".into());
    }

    conn.query_row(
        r#"
SELECT member.provider_id, provider.provider_uuid, member.enabled,
       member.session_reuse_priority, member.cross_provider_model_routing_policy_json
FROM sort_mode_providers member
JOIN providers provider ON provider.id = member.provider_id
WHERE member.mode_id = ?1 AND member.cli_key = ?2 AND member.provider_id = ?3
"#,
        params![mode_id, cli_key, provider_id],
        |row| {
            let enabled_raw: i64 = row.get(2)?;
            Ok(SortModeProviderRow {
                provider_id: row.get(0)?,
                provider_uuid: row.get(1)?,
                enabled: enabled_from_int(enabled_raw),
                session_reuse_priority: row.get(3)?,
                cross_policy: cross_policy_from_json(row.get(4)?),
            })
        },
    )
    .map_err(|e| db_err!("failed to read sort_mode_provider: {e}"))
}

fn read_provider_routing_view(
    conn: &Connection,
    provider_id: i64,
    mode_id: Option<i64>,
) -> crate::shared::error::AppResult<ProviderModelRoutingPolicyView> {
    let (
        provider_uuid,
        cli_key,
        provider_override_enabled,
        ordinary_policy_json,
    ): (String, String, i64, Option<String>) = conn
        .query_row(
            "SELECT provider_uuid, cli_key, model_routing_policy_json IS NOT NULL, model_routing_policy_json FROM providers WHERE id = ?1",
            params![provider_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(|e| db_err!("failed to query provider routing policy: {e}"))?
        .ok_or_else(|| crate::shared::error::AppError::from("DB_NOT_FOUND: provider not found"))?;
    let ordinary_policy_revision =
        routing_policy_revision("provider-ordinary", ordinary_policy_json.as_deref());
    let ordinary_policy =
        crate::providers::model_routing_policy_override_from_json(ordinary_policy_json)
            .unwrap_or_default();

    let Some(mode_id) = mode_id else {
        return Ok(ProviderModelRoutingPolicyView {
            provider_id,
            provider_uuid,
            cli_key,
            provider_override_enabled: provider_override_enabled != 0,
            ordinary_policy,
            ordinary_policy_revision,
            selected_mode: None,
            cross_policy: None,
            cross_policy_revision: None,
            source_member_enabled: false,
            source_member_present: false,
        });
    };

    let selected_mode = conn
        .query_row(
            r#"
SELECT mode.id, identity.mode_uuid, mode.name
FROM sort_modes mode
JOIN sort_mode_identities identity ON identity.mode_id = mode.id
WHERE mode.id = ?1
"#,
            params![mode_id],
            |row| {
                Ok(SortModeRoutingContext {
                    mode_id: row.get(0)?,
                    mode_uuid: row.get(1)?,
                    name: row.get(2)?,
                })
            },
        )
        .optional()
        .map_err(|e| db_err!("failed to query sort-mode routing context: {e}"))?
        .ok_or_else(|| crate::shared::error::AppError::from("DB_NOT_FOUND: sort_mode not found"))?;

    let member = conn
        .query_row(
            r#"
SELECT member.enabled, provider.enabled, member.cross_provider_model_routing_policy_json
FROM sort_mode_providers member
JOIN providers provider ON provider.id = member.provider_id
WHERE member.mode_id = ?1 AND member.cli_key = ?2 AND member.provider_id = ?3
"#,
            params![mode_id, cli_key, provider_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|e| db_err!("failed to query source mode member routing policy: {e}"))?;
    let (source_member_enabled, source_member_present, cross_policy, cross_policy_revision) =
        match member {
            Some((member_enabled, provider_enabled, raw)) => {
                let revision = routing_policy_revision("mode-member-cross", raw.as_deref());
                (
                    member_enabled != 0 && provider_enabled != 0,
                    true,
                    cross_policy_from_json(raw),
                    Some(revision),
                )
            }
            None => (false, false, None, None),
        };
    Ok(ProviderModelRoutingPolicyView {
        provider_id,
        provider_uuid,
        cli_key,
        provider_override_enabled: provider_override_enabled != 0,
        ordinary_policy,
        ordinary_policy_revision,
        selected_mode: Some(selected_mode),
        cross_policy,
        cross_policy_revision,
        source_member_enabled,
        source_member_present,
    })
}

fn validate_cross_policy_targets(
    conn: &Connection,
    mode_id: i64,
    cli_key: &str,
    source_provider_uuid: &str,
    candidate: Option<&crate::settings::CrossProviderModelRoutingPolicy>,
    stored: Option<&crate::settings::CrossProviderModelRoutingPolicy>,
) -> crate::shared::error::AppResult<()> {
    let target_uuids = candidate
        .into_iter()
        .flat_map(|policy| policy.rules.iter())
        .map(|rule| rule.target_provider_uuid.clone())
        .collect::<HashSet<_>>();
    if target_uuids.is_empty() {
        return Ok(());
    }
    if target_uuids.contains(source_provider_uuid) {
        return Err(
            "SEC_INVALID_INPUT: cross-provider target must differ from the source provider".into(),
        );
    }

    let placeholders = db::sql_placeholders(target_uuids.len());
    let sql = format!(
        r#"
SELECT provider.provider_uuid
FROM sort_mode_providers member
JOIN providers provider ON provider.id = member.provider_id
WHERE member.mode_id = ?1 AND member.cli_key = ?2
  AND member.enabled = 1 AND provider.enabled = 1
  AND provider.cli_key = ?2
  AND provider.provider_uuid IN ({placeholders})
"#
    );
    let mut values = Vec::with_capacity(target_uuids.len() + 2);
    values.push(rusqlite::types::Value::from(mode_id));
    values.push(rusqlite::types::Value::from(cli_key.to_string()));
    values.extend(
        target_uuids
            .iter()
            .cloned()
            .map(rusqlite::types::Value::from),
    );
    let mut statement = conn
        .prepare(&sql)
        .map_err(|e| db_err!("failed to prepare target routing member validation: {e}"))?;
    let rows = statement
        .query_map(params_from_iter(values), |row| row.get::<_, String>(0))
        .map_err(|e| db_err!("failed to validate target routing members: {e}"))?;
    let valid_targets = rows
        .collect::<Result<HashSet<_>, _>>()
        .map_err(|e| db_err!("failed to read target routing member: {e}"))?;
    let has_new_invalid_rule = candidate
        .into_iter()
        .flat_map(|policy| policy.rules.iter())
        .any(|rule| {
            !valid_targets.contains(&rule.target_provider_uuid)
                && !stored
                    .into_iter()
                    .flat_map(|policy| policy.rules.iter())
                    .any(|stored_rule| stored_rule == rule)
        });
    if has_new_invalid_rule {
        return Err("SEC_INVALID_INPUT: cross-provider target must be an enabled member of the same CLI and mode".into());
    };
    Ok(())
}

pub fn provider_model_routing_policy_get(
    db: &db::Db,
    provider_id: i64,
    provider_uuid: &str,
    mode_id: Option<i64>,
    mode_uuid: Option<&str>,
) -> crate::shared::error::AppResult<ProviderModelRoutingPolicyView> {
    if provider_id <= 0
        || !crate::shared::uuid::is_canonical_uuid_v4(provider_uuid)
        || mode_id.is_some_and(|id| id <= 0)
        || mode_id.is_some() != mode_uuid.is_some()
        || mode_uuid.is_some_and(|value| !crate::shared::uuid::is_canonical_uuid_v4(value))
    {
        return Err("SEC_INVALID_INPUT: invalid provider or mode identity".into());
    }
    let conn = db.open_connection()?;
    let view = read_provider_routing_view(&conn, provider_id, mode_id)?;
    if view.provider_uuid != provider_uuid {
        return Err("PROVIDER_ROUTING_IDENTITY_CHANGED: provider identity changed".into());
    }
    if view
        .selected_mode
        .as_ref()
        .map(|mode| mode.mode_uuid.as_str())
        != mode_uuid
    {
        return Err("SORT_MODE_ROUTING_IDENTITY_CHANGED: sort mode identity changed".into());
    }
    Ok(view)
}

pub fn provider_model_routing_policy_save(
    db: &db::Db,
    mut input: ProviderModelRoutingPolicySaveInput,
) -> crate::shared::error::AppResult<ProviderModelRoutingPolicyView> {
    if input.provider_id <= 0 || input.mode_id.is_some_and(|id| id <= 0) {
        return Err("SEC_INVALID_INPUT: invalid provider_id or mode_id".into());
    }
    if !crate::shared::uuid::is_canonical_uuid_v4(&input.provider_uuid)
        || input
            .mode_uuid
            .as_deref()
            .is_some_and(|value| !crate::shared::uuid::is_canonical_uuid_v4(value))
        || !is_routing_policy_revision(&input.expected_ordinary_policy_revision)
        || input
            .expected_cross_policy_revision
            .as_deref()
            .is_some_and(|value| !is_routing_policy_revision(value))
    {
        return Err("SEC_INVALID_INPUT: invalid routing policy identity or revision".into());
    }
    if input.mode_id.is_some() != input.mode_uuid.is_some()
        || input.mode_id.is_none() && input.cross_policy.is_some()
        || input.mode_id.is_none() && input.expected_cross_policy_revision.is_some()
    {
        return Err(
            "SEC_INVALID_INPUT: Default cannot persist cross-provider routing policy".into(),
        );
    }
    crate::settings::normalize_model_routing_policy_for_write(&mut input.ordinary_policy)?;
    let ordinary_json = if input.provider_override_enabled {
        Some(serde_json::to_string(&input.ordinary_policy).map_err(|e| {
            crate::shared::error::AppError::from(format!(
                "SYSTEM_ERROR: failed to serialize ordinary routing policy: {e}"
            ))
        })?)
    } else {
        None
    };
    let cross_policy_json = cross_policy_to_json(&mut input.cross_policy)?;

    let mut conn = db.open_connection()?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| db_err!("failed to start provider routing transaction: {e}"))?;
    let identity = tx
        .query_row(
            "SELECT provider_uuid, cli_key FROM providers WHERE id = ?1",
            params![input.provider_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|e| db_err!("failed to validate provider routing identity: {e}"))?
        .ok_or_else(|| crate::shared::error::AppError::from("DB_NOT_FOUND: provider not found"))?;
    if identity.0 != input.provider_uuid {
        return Err("PROVIDER_ROUTING_IDENTITY_CHANGED: provider identity changed".into());
    }

    let current_ordinary_json = tx
        .query_row(
            "SELECT model_routing_policy_json FROM providers WHERE id = ?1 AND provider_uuid = ?2",
            params![input.provider_id, input.provider_uuid],
            |row| row.get::<_, Option<String>>(0),
        )
        .map_err(|e| db_err!("failed to read ordinary provider routing policy revision: {e}"))?;
    if routing_policy_revision("provider-ordinary", current_ordinary_json.as_deref())
        != input.expected_ordinary_policy_revision
    {
        return Err("PROVIDER_ROUTING_CONCURRENT_UPDATE: ordinary routing policy changed".into());
    }

    tx.execute(
        "UPDATE providers SET model_routing_policy_json = ?1, updated_at = ?2 WHERE id = ?3 AND provider_uuid = ?4",
        params![ordinary_json, now_unix_seconds(), input.provider_id, input.provider_uuid],
    )
    .map_err(|e| db_err!("failed to update ordinary provider routing policy: {e}"))?;

    if let (Some(mode_id), Some(mode_uuid)) = (input.mode_id, input.mode_uuid.as_deref()) {
        let actual_mode_uuid = tx
            .query_row(
                "SELECT mode_uuid FROM sort_mode_identities WHERE mode_id = ?1",
                params![mode_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|e| db_err!("failed to validate sort-mode routing identity: {e}"))?
            .ok_or_else(|| {
                crate::shared::error::AppError::from("DB_NOT_FOUND: sort_mode not found")
            })?;
        if actual_mode_uuid != mode_uuid {
            return Err("SORT_MODE_ROUTING_IDENTITY_CHANGED: sort mode identity changed".into());
        }
        let member = tx
            .query_row(
                r#"
SELECT member.enabled, provider.enabled, member.cross_provider_model_routing_policy_json
FROM sort_mode_providers member
JOIN providers provider ON provider.id = member.provider_id
WHERE member.mode_id = ?1 AND member.cli_key = ?2 AND member.provider_id = ?3
"#,
                params![mode_id, identity.1, input.provider_id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)? != 0 && row.get::<_, i64>(1)? != 0,
                        row.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| db_err!("failed to query source mode member routing policy: {e}"))?;
        match (member, input.expected_cross_policy_revision.as_deref()) {
            (None, None) if input.cross_policy.is_none() => {}
            (None, _) => {
                return Err("SEC_INVALID_INPUT: source provider must be an enabled mode member".into())
            }
            (Some((member_enabled, stored_json)), Some(expected_revision)) => {
                if routing_policy_revision("mode-member-cross", stored_json.as_deref())
                    != expected_revision
                {
                    return Err(
                        "PROVIDER_ROUTING_CONCURRENT_UPDATE: cross-provider routing policy changed"
                            .into(),
                    );
                }
                let stored_policy = cross_policy_from_json(stored_json.clone());
                if member_enabled {
                    validate_cross_policy_targets(
                        &tx,
                        mode_id,
                        &identity.1,
                        &input.provider_uuid,
                        input.cross_policy.as_ref(),
                        stored_policy.as_ref(),
                    )?;
                    tx.execute(
                        r#"
UPDATE sort_mode_providers
SET cross_provider_model_routing_policy_json = ?1, updated_at = ?2
WHERE mode_id = ?3 AND cli_key = ?4 AND provider_id = ?5 AND enabled = 1
"#,
                        params![
                            cross_policy_json,
                            now_unix_seconds(),
                            mode_id,
                            identity.1,
                            input.provider_id
                        ],
                    )
                    .map_err(|e| {
                        db_err!("failed to update member cross-provider routing policy: {e}")
                    })?;
                } else if input.cross_policy != stored_policy {
                    return Err(
                        "SEC_INVALID_INPUT: disabled source member cannot change cross-provider routing policy"
                            .into(),
                    );
                }
            }
            (Some(_), None) => {
                return Err("PROVIDER_ROUTING_CONCURRENT_UPDATE: cross-provider routing policy revision is required".into())
            }
        }
    }

    let view = read_provider_routing_view(&tx, input.provider_id, input.mode_id)?;
    tx.commit()
        .map_err(|e| db_err!("failed to commit provider routing transaction: {e}"))?;
    Ok(view)
}

pub fn routing_provider_candidates_list(
    db: &db::Db,
    mode_id: i64,
    mode_uuid: &str,
    cli_key: &str,
) -> crate::shared::error::AppResult<Vec<RoutingProviderCandidate>> {
    validate_cli_key(cli_key)?;
    if mode_id <= 0 || !crate::shared::uuid::is_canonical_uuid_v4(mode_uuid) {
        return Err("SEC_INVALID_INPUT: invalid mode identity".into());
    }
    let conn = db.open_connection()?;
    let mut statement = conn
        .prepare_cached(
            r#"
SELECT provider.id, provider.provider_uuid, provider.cli_key, provider.name,
       provider.enabled, provider.source_provider_id, provider.bridge_type,
       EXISTS(SELECT 1 FROM provider_models model WHERE model.provider_id = provider.id)
FROM sort_mode_providers member
JOIN sort_mode_identities identity ON identity.mode_id = member.mode_id
JOIN providers provider ON provider.id = member.provider_id
WHERE member.mode_id = ?1 AND identity.mode_uuid = ?2
  AND member.cli_key = ?3 AND provider.cli_key = ?3
  AND member.enabled = 1 AND provider.enabled = 1
ORDER BY member.sort_order ASC
"#,
        )
        .map_err(|e| db_err!("failed to prepare routing provider candidates: {e}"))?;
    let rows = statement
        .query_map(params![mode_id, mode_uuid, cli_key], |row| {
            Ok(RoutingProviderCandidate {
                provider_id: row.get(0)?,
                provider_uuid: row.get(1)?,
                cli_key: row.get(2)?,
                name: row.get(3)?,
                enabled: row.get::<_, i64>(4)? != 0,
                source_provider_id: row.get(5)?,
                bridge_type: row.get(6)?,
                model_catalog_supported: row.get::<_, i64>(7)? != 0,
            })
        })
        .map_err(|e| db_err!("failed to query routing provider candidates: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| db_err!("failed to read routing provider candidate: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;

    fn insert_provider(db: &db::Db, name: &str) -> crate::providers::ProviderSummary {
        crate::providers::upsert(
            db,
            crate::providers::ProviderUpsertParams {
                provider_id: None,
                cli_key: "claude".to_string(),
                name: name.to_string(),
                base_urls: vec!["https://example.com".to_string()],
                base_url_mode: crate::providers::ProviderBaseUrlMode::Order,
                auth_mode: Some(crate::providers::ProviderAuthMode::ApiKey),
                api_key: Some("test-key".to_string()),
                enabled: true,
                cost_multiplier: 1.0,
                priority: Some(100),
                claude_models: None,
                availability_test_model: None,
                availability_probe_enabled: false,
                availability_probe_interval_minutes: 10,
                limit_5h_usd: None,
                limit_daily_usd: None,
                daily_reset_mode: Some(crate::providers::DailyResetMode::Fixed),
                daily_reset_time: Some("00:00:00".to_string()),
                limit_weekly_usd: None,
                limit_monthly_usd: None,
                limit_total_usd: None,
                tags: None,
                note: None,
                source_provider_id: None,
                bridge_type: None,
                stream_idle_timeout_seconds: None,
                model_mapping: None,
                extension_values: None,
                account_usage_credentials_patch: None,
                account_usage_credentials_copy_from_provider_id: None,
                upstream_retry_policy_override: None,
                upstream_retry_policy_override_specified: false,
                model_routing_policy_override: None,
                model_routing_policy_override_specified: false,
            },
        )
        .expect("insert provider")
    }

    fn active_mode_id(db: &db::Db, cli_key: &str) -> Option<i64> {
        let conn = db.open_connection().expect("open db");
        conn.query_row(
            "SELECT mode_id FROM sort_mode_active WHERE cli_key = ?1",
            params![cli_key],
            |row| row.get::<_, Option<i64>>(0),
        )
        .expect("read active mode")
    }

    fn ordinary_policy(target_model: &str) -> crate::settings::ModelRoutingPolicy {
        crate::settings::ModelRoutingPolicy {
            enabled: true,
            rules: vec![crate::settings::ModelRoutingRule {
                source_model: "source-model".to_string(),
                source_reasoning_effort: None,
                target_model: Some(target_model.to_string()),
                reasoning_effort: None,
                unrecognized_fields: Default::default(),
            }],
        }
    }

    fn cross_policy(
        target_provider_uuid: &str,
    ) -> crate::settings::CrossProviderModelRoutingPolicy {
        crate::settings::CrossProviderModelRoutingPolicy {
            enabled: true,
            rules: vec![crate::settings::CrossProviderModelRoutingRule {
                source_model: "source-model".to_string(),
                source_reasoning_effort: None,
                target_provider_uuid: target_provider_uuid.to_string(),
                target_model: None,
                target_reasoning_effort: None,
            }],
        }
    }

    fn cross_policy_with_two_rules(
        target_provider_uuid: &str,
    ) -> crate::settings::CrossProviderModelRoutingPolicy {
        let mut policy = cross_policy(target_provider_uuid);
        policy
            .rules
            .push(crate::settings::CrossProviderModelRoutingRule {
                source_model: "second-source-model".to_string(),
                source_reasoning_effort: Some("high".to_string()),
                target_provider_uuid: target_provider_uuid.to_string(),
                target_model: Some("target-model".to_string()),
                target_reasoning_effort: Some("medium".to_string()),
            });
        policy
    }

    fn save_input(
        view: &ProviderModelRoutingPolicyView,
        ordinary_policy: crate::settings::ModelRoutingPolicy,
        cross_policy: Option<crate::settings::CrossProviderModelRoutingPolicy>,
    ) -> ProviderModelRoutingPolicySaveInput {
        ProviderModelRoutingPolicySaveInput {
            provider_id: view.provider_id,
            provider_uuid: view.provider_uuid.clone(),
            mode_id: view.selected_mode.as_ref().map(|mode| mode.mode_id),
            mode_uuid: view
                .selected_mode
                .as_ref()
                .map(|mode| mode.mode_uuid.clone()),
            provider_override_enabled: true,
            ordinary_policy,
            expected_ordinary_policy_revision: view.ordinary_policy_revision.clone(),
            cross_policy,
            expected_cross_policy_revision: view.cross_policy_revision.clone(),
        }
    }

    fn routing_view(
        db: &db::Db,
        provider: &crate::providers::ProviderSummary,
        mode: &SortModeSummary,
    ) -> ProviderModelRoutingPolicyView {
        provider_model_routing_policy_get(
            db,
            provider.id,
            &provider.provider_uuid,
            Some(mode.id),
            Some(mode.mode_uuid.as_str()),
        )
        .expect("read routing view")
    }

    #[test]
    fn delete_mode_returns_active_cli_keys_before_fk_nulls_active_rows() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("sort_mode_delete_active_cli_keys.db");
        let db = db::init_for_tests(&db_path).expect("init db");

        let deleted_mode = create_mode(&db, "Review Mode").expect("create deleted mode");
        let other_mode = create_mode(&db, "Other Mode").expect("create other mode");
        set_active(&db, "claude", Some(deleted_mode.id)).expect("activate claude");
        set_active(&db, "codex", Some(deleted_mode.id)).expect("activate codex");
        set_active(&db, "gemini", Some(other_mode.id)).expect("activate gemini");

        let affected_cli_keys =
            delete_mode_with_affected_cli_keys(&db, deleted_mode.id).expect("delete mode");

        assert_eq!(
            affected_cli_keys,
            vec!["claude".to_string(), "codex".to_string()]
        );
        assert_eq!(active_mode_id(&db, "claude"), None);
        assert_eq!(active_mode_id(&db, "codex"), None);
        assert_eq!(active_mode_id(&db, "gemini"), Some(other_mode.id));
    }

    #[test]
    fn set_active_update_rolls_back_when_result_projection_fails() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("sort_mode_active_projection_failure.db");
        let db = db::init_for_tests(&db_path).expect("init db");
        let mode_a = create_mode(&db, "Mode A").expect("create Mode A");
        let mode_b = create_mode(&db, "Mode B").expect("create Mode B");
        let original = set_active(&db, "claude", Some(mode_a.id)).expect("activate Mode A");

        let conn = db.open_connection().expect("open db");
        conn.execute_batch(
            r#"
CREATE TRIGGER corrupt_sort_mode_active_projection
AFTER UPDATE ON sort_mode_active
BEGIN
  UPDATE sort_mode_active
  SET updated_at = 'invalid'
  WHERE cli_key = NEW.cli_key;
END;
"#,
        )
        .expect("create projection failure trigger");
        drop(conn);

        for target_mode_id in [Some(mode_b.id), None] {
            let error = set_active(&db, "claude", target_mode_id)
                .expect_err("invalid projected row must fail the route switch");
            assert!(error
                .to_string()
                .contains("failed to query sort_mode_active"));

            let conn = db.open_connection().expect("reopen db");
            let current = read_active_row(&conn, "claude").expect("read current active mode");
            assert_eq!(current.mode_id, Some(mode_a.id));
            assert_eq!(current.updated_at, original.updated_at);
        }
    }

    #[test]
    fn mode_reorder_preserves_session_reuse_priority() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("sort_mode_reuse_priority.db");
        let db = db::init_for_tests(&db_path).expect("init db");
        let p1 = insert_provider(&db, "P1");
        let p2 = insert_provider(&db, "P2");
        let mode = create_mode(&db, "Reuse Priority").expect("create mode");

        set_mode_providers_order(&db, mode.id, "claude", vec![p1.id, p2.id])
            .expect("set mode order");
        let updated = set_mode_provider_session_reuse_priority(&db, mode.id, "claude", p1.id, 25)
            .expect("set priority");
        assert_eq!(updated.session_reuse_priority, 25);

        let reordered = set_mode_providers_order(&db, mode.id, "claude", vec![p2.id, p1.id])
            .expect("reorder mode");
        assert_eq!(
            reordered
                .iter()
                .map(|row| (row.provider_id, row.enabled, row.session_reuse_priority))
                .collect::<Vec<_>>(),
            vec![(p2.id, true, 0), (p1.id, true, 25)]
        );

        let error = set_mode_provider_session_reuse_priority(
            &db,
            mode.id,
            "claude",
            p1.id,
            MAX_SESSION_REUSE_PRIORITY + 1,
        )
        .expect_err("out-of-range priority must fail");
        assert!(error.to_string().contains("session_reuse_priority"));
    }

    #[test]
    fn mode_identity_survives_rename_and_delete_cascades_member_policy() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("sort_mode_identity_lifecycle.db");
        let db = db::init_for_tests(&db_path).expect("init db");
        let source = insert_provider(&db, "Source");
        let target = insert_provider(&db, "Target");
        let mode = create_mode(&db, "Original").expect("create mode");
        set_mode_providers_order(&db, mode.id, "claude", vec![source.id, target.id])
            .expect("set mode members");
        let initial = routing_view(&db, &source, &mode);
        let saved = provider_model_routing_policy_save(
            &db,
            save_input(
                &initial,
                ordinary_policy("ordinary-target"),
                Some(cross_policy(&target.provider_uuid)),
            ),
        )
        .expect("save cross policy");

        let renamed = rename_mode(&db, mode.id, "Renamed").expect("rename mode");
        assert_eq!(renamed.mode_uuid, mode.mode_uuid);
        let after_rename = routing_view(&db, &source, &renamed);
        assert_eq!(after_rename.cross_policy, saved.cross_policy);

        delete_mode(&db, mode.id).expect("delete mode");
        let conn = db.open_connection().expect("open db");
        let counts: (i64, i64) = conn
            .query_row(
                "SELECT (SELECT COUNT(*) FROM sort_mode_identities WHERE mode_id = ?1), (SELECT COUNT(*) FROM sort_mode_providers WHERE mode_id = ?1)",
                [mode.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("count cascaded routing state");
        assert_eq!(counts, (0, 0));
    }

    #[test]
    fn routing_policy_save_rejects_identity_drift_and_rolls_back_ordinary_policy() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("routing_policy_identity_rollback.db");
        let db = db::init_for_tests(&db_path).expect("init db");
        let source = insert_provider(&db, "Source");
        let target = insert_provider(&db, "Target");
        let mode = create_mode(&db, "Mode").expect("create mode");
        set_mode_providers_order(&db, mode.id, "claude", vec![source.id, target.id])
            .expect("set mode members");
        let initial = routing_view(&db, &source, &mode);

        let mut mismatched_provider = save_input(
            &initial,
            ordinary_policy("must-not-commit"),
            Some(cross_policy(&target.provider_uuid)),
        );
        mismatched_provider.provider_uuid = target.provider_uuid.clone();
        let error = provider_model_routing_policy_save(&db, mismatched_provider)
            .expect_err("provider identity mismatch must fail");
        assert!(error
            .to_string()
            .contains("PROVIDER_ROUTING_IDENTITY_CHANGED"));

        let mut mismatched_mode = save_input(
            &initial,
            ordinary_policy("must-not-commit"),
            Some(cross_policy(&target.provider_uuid)),
        );
        mismatched_mode.mode_uuid = Some(crate::shared::uuid::new_uuid_v4());
        let error = provider_model_routing_policy_save(&db, mismatched_mode)
            .expect_err("mode identity mismatch must fail");
        assert!(error
            .to_string()
            .contains("SORT_MODE_ROUTING_IDENTITY_CHANGED"));

        let after = routing_view(&db, &source, &mode);
        assert!(!after.provider_override_enabled);
        assert_eq!(
            after.ordinary_policy,
            crate::settings::ModelRoutingPolicy::default()
        );
        assert_eq!(after.cross_policy, None);
    }

    #[test]
    fn routing_policy_save_rejects_cross_fields_in_ordinary_policy_without_partial_write() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("routing_policy_ordinary_cross_fields.db");
        let db = db::init_for_tests(&db_path).expect("init db");
        let source = insert_provider(&db, "Source");
        let target = insert_provider(&db, "Target");
        let mode = create_mode(&db, "Mode").expect("create mode");
        set_mode_providers_order(&db, mode.id, "claude", vec![source.id, target.id])
            .expect("set mode members");
        let initial = routing_view(&db, &source, &mode);
        let input: ProviderModelRoutingPolicySaveInput =
            serde_json::from_value(serde_json::json!({
                "providerId": source.id,
                "providerUuid": source.provider_uuid.clone(),
                "modeId": mode.id,
                "modeUuid": mode.mode_uuid.clone(),
                "providerOverrideEnabled": true,
                "ordinaryPolicy": {
                    "enabled": true,
                    "rules": [{
                        "source_model": "source-model",
                        "target_model": "ordinary-target",
                        "target_provider_uuid": target.provider_uuid.clone(),
                        "target_reasoning_effort": "high"
                    }]
                },
                "expectedOrdinaryPolicyRevision": initial.ordinary_policy_revision.clone(),
                "crossPolicy": null,
                "expectedCrossPolicyRevision": initial.cross_policy_revision.clone()
            }))
            .expect("cross-only fields remain visible to the provider write boundary");

        let error = provider_model_routing_policy_save(&db, input)
            .expect_err("provider ordinary policy must reject cross-only fields");
        assert!(error.to_string().contains("SEC_INVALID_INPUT"));
        let after = routing_view(&db, &source, &mode);
        assert_eq!(
            after.provider_override_enabled,
            initial.provider_override_enabled
        );
        assert_eq!(after.ordinary_policy, initial.ordinary_policy);
        assert_eq!(
            after.ordinary_policy_revision,
            initial.ordinary_policy_revision
        );
        assert_eq!(after.cross_policy, initial.cross_policy);
        assert_eq!(after.cross_policy_revision, initial.cross_policy_revision);
    }

    #[test]
    fn routing_policy_save_rejects_stale_owner_revision_without_overwriting_winner() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("routing_policy_cas.db");
        let db = db::init_for_tests(&db_path).expect("init db");
        let source = insert_provider(&db, "Source");
        let target = insert_provider(&db, "Target");
        let mode = create_mode(&db, "Mode").expect("create mode");
        set_mode_providers_order(&db, mode.id, "claude", vec![source.id, target.id])
            .expect("set mode members");
        let stale = routing_view(&db, &source, &mode);

        let winner = provider_model_routing_policy_save(
            &db,
            save_input(
                &stale,
                ordinary_policy("winner"),
                Some(cross_policy(&target.provider_uuid)),
            ),
        )
        .expect("save winner");
        let error = provider_model_routing_policy_save(
            &db,
            save_input(
                &stale,
                ordinary_policy("loser"),
                Some(cross_policy(&target.provider_uuid)),
            ),
        )
        .expect_err("stale save must lose CAS");
        assert!(error
            .to_string()
            .contains("PROVIDER_ROUTING_CONCURRENT_UPDATE"));

        let current = routing_view(&db, &source, &mode);
        assert_eq!(current.ordinary_policy, winner.ordinary_policy);
        assert_eq!(current.cross_policy, winner.cross_policy);
    }

    #[test]
    fn routing_policy_preserves_invalid_historical_target_but_rejects_new_invalid_target() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("routing_policy_invalid_target.db");
        let db = db::init_for_tests(&db_path).expect("init db");
        let source = insert_provider(&db, "Source");
        let target = insert_provider(&db, "Target");
        let mode = create_mode(&db, "Mode").expect("create mode");
        set_mode_providers_order(&db, mode.id, "claude", vec![source.id, target.id])
            .expect("set mode members");
        let initial = routing_view(&db, &source, &mode);
        provider_model_routing_policy_save(
            &db,
            save_input(
                &initial,
                ordinary_policy("ordinary"),
                Some(cross_policy(&target.provider_uuid)),
            ),
        )
        .expect("save valid target");
        crate::providers::set_enabled(&db, target.id, false).expect("disable target");

        let historical = routing_view(&db, &source, &mode);
        provider_model_routing_policy_save(
            &db,
            save_input(
                &historical,
                ordinary_policy("ordinary-updated"),
                historical.cross_policy.clone(),
            ),
        )
        .expect("preserve invalid historical target while saving ordinary policy");

        let current = routing_view(&db, &source, &mode);
        let invalid_uuid = crate::shared::uuid::new_uuid_v4();
        let error = provider_model_routing_policy_save(
            &db,
            save_input(
                &current,
                current.ordinary_policy.clone(),
                Some(cross_policy(&invalid_uuid)),
            ),
        )
        .expect_err("new invalid target must fail");
        assert!(error
            .to_string()
            .contains("cross-provider target must be an enabled member"));
    }

    #[test]
    fn routing_policy_allows_multiple_rules_for_one_valid_target() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("routing_policy_shared_target.db");
        let db = db::init_for_tests(&db_path).expect("init db");
        let source = insert_provider(&db, "Source");
        let target = insert_provider(&db, "Target");
        let mode = create_mode(&db, "Mode").expect("create mode");
        set_mode_providers_order(&db, mode.id, "claude", vec![source.id, target.id])
            .expect("set mode members");
        let initial = routing_view(&db, &source, &mode);

        let saved = provider_model_routing_policy_save(
            &db,
            save_input(
                &initial,
                ordinary_policy("ordinary"),
                Some(cross_policy_with_two_rules(&target.provider_uuid)),
            ),
        )
        .expect("save two rules sharing one target");
        assert_eq!(saved.cross_policy.expect("cross policy").rules.len(), 2);
    }

    #[test]
    fn default_and_disabled_member_save_only_their_owned_routing_fields() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("routing_policy_owner_boundaries.db");
        let db = db::init_for_tests(&db_path).expect("init db");
        let source = insert_provider(&db, "Source");
        let target = insert_provider(&db, "Target");
        let mode = create_mode(&db, "Mode").expect("create mode");
        set_mode_providers_order(&db, mode.id, "claude", vec![source.id, target.id])
            .expect("set mode members");
        let initial = routing_view(&db, &source, &mode);
        let saved = provider_model_routing_policy_save(
            &db,
            save_input(
                &initial,
                ordinary_policy("ordinary"),
                Some(cross_policy(&target.provider_uuid)),
            ),
        )
        .expect("seed routing policies");

        set_mode_provider_enabled(&db, mode.id, "claude", source.id, false)
            .expect("disable source member");
        let disabled = routing_view(&db, &source, &mode);
        let disabled_saved = provider_model_routing_policy_save(
            &db,
            save_input(
                &disabled,
                ordinary_policy("ordinary-while-disabled"),
                disabled.cross_policy.clone(),
            ),
        )
        .expect("save ordinary policy without changing disabled member policy");
        assert!(!disabled_saved.source_member_enabled);
        assert_eq!(disabled_saved.cross_policy, saved.cross_policy);

        let default =
            provider_model_routing_policy_get(&db, source.id, &source.provider_uuid, None, None)
                .expect("read Default routing view");
        let mut default_input = save_input(&default, ordinary_policy("default-ordinary"), None);
        default_input.provider_override_enabled = false;
        let default_saved = provider_model_routing_policy_save(&db, default_input)
            .expect("save Default ordinary policy");
        assert!(!default_saved.provider_override_enabled);
        assert!(default_saved.selected_mode.is_none());
        assert_eq!(default_saved.cross_policy, None);

        let conn = db.open_connection().expect("open db");
        let raw_cross: String = conn
            .query_row(
                "SELECT cross_provider_model_routing_policy_json FROM sort_mode_providers WHERE mode_id = ?1 AND provider_id = ?2",
                params![mode.id, source.id],
                |row| row.get(0),
            )
            .expect("read retained cross policy");
        assert!(raw_cross.contains(&target.provider_uuid));
    }

    #[test]
    fn malformed_cross_policy_json_fails_open_without_inheriting_or_blocking() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("routing_policy_malformed_json.db");
        let db = db::init_for_tests(&db_path).expect("init db");
        let source = insert_provider(&db, "Source");
        let mode = create_mode(&db, "Mode").expect("create mode");
        set_mode_providers_order(&db, mode.id, "claude", vec![source.id]).expect("set mode member");
        let conn = db.open_connection().expect("open db");
        conn.execute(
            "UPDATE sort_mode_providers SET cross_provider_model_routing_policy_json = '{bad-json' WHERE mode_id = ?1 AND provider_id = ?2",
            params![mode.id, source.id],
        )
        .expect("seed malformed policy");
        drop(conn);

        let view = routing_view(&db, &source, &mode);
        assert_eq!(view.cross_policy, None);
        assert!(view.cross_policy_revision.is_some());
    }
}
