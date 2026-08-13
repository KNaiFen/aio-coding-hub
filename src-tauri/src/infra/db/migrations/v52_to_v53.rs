//! Usage: SQLite migration v52->v53 - stable sort-mode identities and member routing policy.

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use std::collections::HashSet;

fn table_exists(conn: &Connection, table: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1",
        [table],
        |_row| Ok(()),
    )
    .optional()
    .map(|value| value.is_some())
    .map_err(|error| format!("failed to inspect table {table}: {error}"))
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT 1 FROM pragma_table_info(?1) WHERE name = ?2 LIMIT 1",
        (table, column),
        |_row| Ok(()),
    )
    .optional()
    .map(|value| value.is_some())
    .map_err(|error| format!("failed to inspect {table}.{column}: {error}"))
}

fn create_sort_mode_identity_schema(tx: &Transaction<'_>) -> Result<(), String> {
    tx.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS sort_mode_identities (
  mode_id INTEGER PRIMARY KEY,
  mode_uuid TEXT NOT NULL UNIQUE,
  FOREIGN KEY(mode_id) REFERENCES sort_modes(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_sort_mode_identities_mode_uuid
  ON sort_mode_identities(mode_uuid);

CREATE TRIGGER IF NOT EXISTS sort_mode_identities_uuid_insert_guard
BEFORE INSERT ON sort_mode_identities
WHEN NEW.mode_uuid IS NULL
  OR length(NEW.mode_uuid) <> 36
  OR lower(NEW.mode_uuid) <> NEW.mode_uuid
  OR substr(NEW.mode_uuid, 9, 1) <> '-'
  OR substr(NEW.mode_uuid, 14, 1) <> '-'
  OR substr(NEW.mode_uuid, 19, 1) <> '-'
  OR substr(NEW.mode_uuid, 24, 1) <> '-'
  OR substr(NEW.mode_uuid, 15, 1) <> '4'
  OR substr(NEW.mode_uuid, 20, 1) NOT IN ('8', '9', 'a', 'b')
  OR length(replace(NEW.mode_uuid, '-', '')) <> 32
  OR replace(NEW.mode_uuid, '-', '') GLOB '*[^0-9a-f]*'
BEGIN
  SELECT RAISE(ABORT, 'mode_uuid must be a canonical UUID');
END;

CREATE TRIGGER IF NOT EXISTS sort_mode_identities_uuid_update_guard
BEFORE UPDATE OF mode_uuid ON sort_mode_identities
WHEN NEW.mode_uuid IS NULL OR NEW.mode_uuid <> OLD.mode_uuid
BEGIN
  SELECT RAISE(ABORT, 'mode_uuid is immutable');
END;
"#,
    )
    .map_err(|error| format!("failed to create sort-mode identity schema: {error}"))
}

fn validate_sort_mode_identity_schema(tx: &Transaction<'_>) -> Result<(), String> {
    let columns = {
        let mut statement = tx
            .prepare("PRAGMA table_info(sort_mode_identities)")
            .map_err(|error| format!("failed to inspect sort-mode identity columns: {error}"))?;
        let columns = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(5)?,
                ))
            })
            .map_err(|error| format!("failed to query sort-mode identity columns: {error}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("failed to read sort-mode identity columns: {error}"))?;
        columns
    };
    let mode_id_valid = columns
        .iter()
        .any(|(name, _, primary_key)| name == "mode_id" && *primary_key == 1);
    let mode_uuid_valid = columns
        .iter()
        .any(|(name, not_null, _)| name == "mode_uuid" && *not_null == 1);
    if !mode_id_valid || !mode_uuid_valid {
        return Err("invalid sort-mode identity table shape".to_string());
    }

    let cascade_fk_exists: bool = tx
        .query_row(
            r#"
SELECT EXISTS(
  SELECT 1
  FROM pragma_foreign_key_list('sort_mode_identities')
  WHERE "table" = 'sort_modes' AND "from" = 'mode_id'
    AND "to" = 'id' AND upper(on_delete) = 'CASCADE'
)
"#,
            [],
            |row| row.get(0),
        )
        .map_err(|error| format!("failed to inspect sort-mode identity foreign key: {error}"))?;
    if !cascade_fk_exists {
        return Err("sort-mode identity cascade foreign key is missing".to_string());
    }
    Ok(())
}

fn backfill_and_validate_identities(tx: &Transaction<'_>) -> Result<(), String> {
    let mode_ids = {
        let mut statement = tx
            .prepare("SELECT id FROM sort_modes ORDER BY id ASC")
            .map_err(|error| format!("failed to prepare sort-mode identity backfill: {error}"))?;
        let rows = statement
            .query_map([], |row| row.get::<_, i64>(0))
            .map_err(|error| {
                format!("failed to query sort modes for identity backfill: {error}")
            })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("failed to read sort mode for identity backfill: {error}"))?
    };

    for mode_id in mode_ids {
        let existing = tx
            .query_row(
                "SELECT mode_uuid FROM sort_mode_identities WHERE mode_id = ?1",
                params![mode_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| format!("failed to query sort-mode identity: {error}"))?;
        if existing.is_none() {
            tx.execute(
                "INSERT INTO sort_mode_identities(mode_id, mode_uuid) VALUES (?1, ?2)",
                params![mode_id, crate::shared::uuid::new_uuid_v4()],
            )
            .map_err(|error| format!("failed to backfill sort-mode identity: {error}"))?;
        }
    }

    let mut statement = tx
        .prepare(
            r#"
SELECT identity.mode_id, identity.mode_uuid, mode.id
FROM sort_mode_identities identity
LEFT JOIN sort_modes mode ON mode.id = identity.mode_id
ORDER BY identity.mode_id ASC
"#,
        )
        .map_err(|error| format!("failed to prepare sort-mode identity validation: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<i64>>(2)?,
            ))
        })
        .map_err(|error| format!("failed to query sort-mode identities: {error}"))?;
    let mut identities = Vec::new();
    let mut seen = HashSet::new();
    for row in rows {
        let (mode_id, mode_uuid, matching_mode_id) =
            row.map_err(|error| format!("failed to read sort-mode identity: {error}"))?;
        if mode_id <= 0
            || matching_mode_id != Some(mode_id)
            || !crate::shared::uuid::is_canonical_uuid_v4(&mode_uuid)
            || !seen.insert(mode_uuid)
        {
            return Err("invalid or duplicate sort-mode identity".to_string());
        }
        identities.push(mode_id);
    }
    let mode_count: i64 = tx
        .query_row("SELECT COUNT(*) FROM sort_modes", [], |row| row.get(0))
        .map_err(|error| format!("failed to count sort modes: {error}"))?;
    if identities.len() as i64 != mode_count {
        return Err("sort-mode identity backfill is incomplete".to_string());
    }
    Ok(())
}

pub(super) fn migrate_v52_to_v53(conn: &mut Connection) -> crate::shared::error::AppResult<()> {
    let foreign_keys_enabled: bool = conn
        .pragma_query_value(None, "foreign_keys", |row| row.get(0))
        .map_err(|error| format!("failed to inspect foreign-key enforcement: {error}"))?;
    if !foreign_keys_enabled {
        return Err("foreign-key enforcement is required for v52->v53 migration".into());
    }
    if !table_exists(conn, "sort_modes")? || !table_exists(conn, "sort_mode_providers")? {
        return Err("missing sort-mode tables for v52->v53 migration".into());
    }
    let tx = conn
        .transaction()
        .map_err(|error| format!("failed to start v52->v53 transaction: {error}"))?;
    create_sort_mode_identity_schema(&tx)?;
    validate_sort_mode_identity_schema(&tx)?;
    backfill_and_validate_identities(&tx)?;
    if !column_exists(
        &tx,
        "sort_mode_providers",
        "cross_provider_model_routing_policy_json",
    )? {
        tx.execute_batch(
            "ALTER TABLE sort_mode_providers ADD COLUMN cross_provider_model_routing_policy_json TEXT DEFAULT NULL;",
        )
        .map_err(|error| format!("failed to add member cross-provider routing policy: {error}"))?;
    }
    super::set_user_version(&tx, 53)?;
    tx.commit()
        .map_err(|error| format!("failed to commit v52->v53 transaction: {error}"))?;
    Ok(())
}
