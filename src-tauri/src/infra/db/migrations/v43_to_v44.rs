//! Usage: SQLite migration v43->v44 - provider-scoped model routing policy.

use rusqlite::Connection;

pub(super) fn migrate_v43_to_v44(conn: &mut Connection) -> crate::shared::error::AppResult<()> {
    let tx = conn
        .transaction()
        .map_err(|error| format!("failed to start sqlite transaction: {error}"))?;
    if !super::ensure::column_exists(&tx, "providers", "model_routing_policy_json")? {
        tx.execute(
            "ALTER TABLE providers ADD COLUMN model_routing_policy_json TEXT DEFAULT NULL",
            [],
        )
        .map_err(|error| format!("failed to add providers.model_routing_policy_json: {error}"))?;
    }
    super::v42_to_v43::refresh_usage_events_view(&tx)?;
    super::set_user_version(&tx, 44)?;
    tx.commit()
        .map_err(|error| format!("failed to commit sqlite transaction: {error}"))?;
    Ok(())
}
