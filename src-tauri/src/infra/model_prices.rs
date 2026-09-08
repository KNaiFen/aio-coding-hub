//! Usage: Model price persistence (sqlite CRUD helpers).

use crate::db;
use crate::shared::error::db_err;
use crate::shared::time::now_unix_seconds;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ModelPriceSummary {
    pub id: i64,
    pub cli_key: String,
    pub model: String,
    pub currency: String,
    pub created_at: i64,
    pub updated_at: i64,
}

fn validate_cli_key(cli_key: &str) -> Result<(), String> {
    crate::shared::cli_key::validate_cli_key(cli_key)?;
    Ok(())
}

fn row_to_summary(row: &rusqlite::Row<'_>) -> Result<ModelPriceSummary, rusqlite::Error> {
    Ok(ModelPriceSummary {
        id: row.get("id")?,
        cli_key: row.get("cli_key")?,
        model: row.get("model")?,
        currency: row.get("currency")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

pub fn list_by_cli(
    db: &db::Db,
    cli_key: &str,
) -> crate::shared::error::AppResult<Vec<ModelPriceSummary>> {
    validate_cli_key(cli_key)?;
    let conn = db.open_connection()?;

    let mut stmt = conn
        .prepare_cached(
            r#"
    SELECT
      id,
      cli_key,
      model,
      currency,
      created_at,
      updated_at
    FROM model_prices
    WHERE cli_key = ?1
    ORDER BY model ASC, id DESC
    "#,
        )
        .map_err(|e| db_err!("failed to prepare model_prices list: {e}"))?;

    let rows = stmt
        .query_map(params![cli_key], row_to_summary)
        .map_err(|e| db_err!("failed to list model_prices: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| db_err!("failed to read model_price row: {e}"))?);
    }
    Ok(items)
}

pub fn upsert(
    db: &db::Db,
    cli_key: &str,
    model: &str,
    price_json: &str,
) -> crate::shared::error::AppResult<ModelPriceSummary> {
    validate_cli_key(cli_key)?;

    let model = model.trim();
    if model.is_empty() {
        return Err("SEC_INVALID_INPUT: model is required".to_string().into());
    }

    let normalized_price = match serde_json::from_str::<serde_json::Value>(price_json) {
        Ok(v) => serde_json::to_string(&v).unwrap_or_else(|_| "{}".to_string()),
        Err(_) => {
            return Err("SEC_INVALID_INPUT: price_json must be valid JSON"
                .to_string()
                .into())
        }
    };

    if normalized_price == "{}" {
        return Err("SEC_INVALID_INPUT: price_json is empty".to_string().into());
    }

    let conn = db.open_connection()?;
    let now = now_unix_seconds();

    conn.execute(
        r#"
INSERT INTO model_prices(cli_key, model, price_json, created_at, updated_at)
VALUES (?1, ?2, ?3, ?4, ?4)
ON CONFLICT(cli_key, model) DO UPDATE SET
  price_json = excluded.price_json,
  updated_at = excluded.updated_at
"#,
        params![cli_key, model, normalized_price, now],
    )
    .map_err(|e| db_err!("failed to upsert model_price: {e}"))?;

    conn.query_row(
        r#"
SELECT
  id,
  cli_key,
  model,
  currency,
  created_at,
  updated_at
FROM model_prices
WHERE cli_key = ?1 AND model = ?2
"#,
        params![cli_key, model],
        row_to_summary,
    )
    .optional()
    .map_err(|e| db_err!("failed to query model_price: {e}"))?
    .ok_or_else(|| "DB_NOT_FOUND: model_price not found".to_string().into())
}

pub fn reference_get(
    db: &db::Db,
    cli_key: &str,
    model: &str,
    aliases: &crate::model_price_aliases::ModelPriceAliasesV1,
) -> crate::shared::error::AppResult<Option<crate::cost::ModelPriceReference>> {
    validate_cli_key(cli_key)?;
    if model.is_empty() || model.trim() != model || model.len() > 200 || model.contains('*') {
        return Err("SEC_INVALID_INPUT: model must be a complete name (max 200 bytes)".to_string().into());
    }
    let conn = db.open_connection()?;
    let mut stmt = conn.prepare_cached("SELECT price_json FROM model_prices WHERE cli_key = ?1 AND model = ?2")
        .map_err(|e| db_err!("failed to prepare reference price: {e}"))?;
    let mut reference_model = model;
    let mut json: Option<String> = stmt.query_row(params![cli_key, model], |row| row.get(0)).optional()
        .map_err(|e| db_err!("failed to read reference price: {e}"))?;
    if json.is_none() {
        if let Some(target) = aliases.resolve_target_model(cli_key, model) {
            reference_model = target;
            json = stmt.query_row(params![cli_key, target], |row| row.get(0)).optional()
                .map_err(|e| db_err!("failed to read alias reference price: {e}"))?;
        }
    }
    let Some(json) = json else { return Ok(None); };
    crate::cost::reference_prices(&json, cli_key, reference_model).map(Some)
        .ok_or_else(|| "invalid reference price JSON".to_string().into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_uses_alias_only_after_exact_miss_and_exposes_price_variants() {
        let dir = tempfile::tempdir().unwrap();
        let db = db::init_for_tests(&dir.path().join("reference.db")).unwrap();
        let aliases = crate::model_price_aliases::ModelPriceAliasesV1::default();
        assert!(reference_get(&db, "grok", "grok-build", &aliases).unwrap().is_none());
        upsert(&db, "grok", "grok-build-0.1", r#"{"input_cost_per_token":0.000002,"input_cost_per_token_priority":0.000004,"input_cost_per_token_above_200k_tokens":0.000006}"#).unwrap();
        let reference = reference_get(&db, "grok", "grok-build", &aliases).unwrap().unwrap();
        assert_eq!(reference.reference_model, "grok-build-0.1");
        assert_eq!(reference.input.standard, Some(2.0));
        assert_eq!(reference.input.priority, Some(4.0));
        assert_eq!(reference.input.above_200k, Some(6.0));
        assert_eq!(reference.output.standard, None);
        upsert(&db, "grok", "grok-build", r#"{"input_cost_per_token":0.000001}"#).unwrap();
        assert_eq!(reference_get(&db, "grok", "grok-build", &aliases).unwrap().unwrap().reference_model, "grok-build");
    }
}
