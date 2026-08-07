//! Usage: Durable recovery journal for SQLite-to-filesystem projections.
//!
//! SQLite is authoritative. A row is committed before any external effect can
//! run, and a fenced lease prevents two application instances from projecting
//! the same row concurrently. Recovery always derives the desired projection
//! from committed SQLite state plus a journal-owned artifact when SQLite alone
//! cannot describe the required file contents.

use crate::db;
use crate::shared::cli_key::{CliCapability, CliKey};
use crate::shared::error::{AppError, AppResult};
use crate::shared::time::now_unix_seconds;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

const JOURNAL_RETENTION_SECONDS: i64 = 7 * 24 * 60 * 60;
const OPERATION_LEASE_SECONDS: i64 = 5 * 60;
const REPLAY_COORDINATOR_LEASE_SECONDS: i64 = 5 * 60;
const REPLAY_CONTEXT_MAX_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, Default)]
pub(crate) struct JournalContext {
    pub(crate) cli_key: Option<String>,
    pub(crate) workspace_id: Option<i64>,
    pub(crate) entity_id: Option<i64>,
    pub(crate) parent_operation_id: Option<String>,
    pub(crate) phase: Option<&'static str>,
    pub(crate) replay_context: Option<String>,
}

impl JournalContext {
    pub(crate) fn for_workspace(workspace_id: i64) -> Self {
        Self {
            workspace_id: Some(workspace_id),
            ..Self::default()
        }
    }

    pub(crate) fn for_entity(entity_id: i64) -> Self {
        Self {
            entity_id: Some(entity_id),
            ..Self::default()
        }
    }

    #[cfg(test)]
    pub(crate) fn with_cli_key(mut self, cli_key: impl Into<String>) -> Self {
        self.cli_key = Some(cli_key.into());
        self
    }
}

#[derive(Debug, Clone)]
pub(crate) struct JournalEntry {
    pub(crate) operation_id: String,
    pub(crate) parent_operation_id: Option<String>,
    pub(crate) operation_kind: String,
    pub(crate) cli_key: Option<String>,
    pub(crate) workspace_id: Option<i64>,
    pub(crate) entity_id: Option<i64>,
    pub(crate) phase: String,
    pub(crate) status: String,
    pub(crate) artifact_ref: Option<String>,
    pub(crate) artifact_sha256: Option<String>,
    pub(crate) replay_context: Option<String>,
}

#[derive(Debug, Clone)]
struct ClaimToken {
    owner: String,
    epoch: i64,
}

#[derive(Debug, Clone)]
struct ClaimedEntry {
    entry: JournalEntry,
    claim: ClaimToken,
}

#[derive(Debug)]
struct ProjectionLock {
    _file: File,
}

fn acquire_projection_lock_at(path: &Path) -> AppResult<ProjectionLock> {
    let parent = path
        .parent()
        .ok_or_else(|| AppError::new("RECOVERY_PROJECTION_LOCK_FAILED", "外部投影锁路径无效"))?;
    std::fs::create_dir_all(parent)
        .map_err(|_| AppError::new("RECOVERY_PROJECTION_LOCK_FAILED", "无法创建外部投影锁目录"))?;
    if std::fs::symlink_metadata(path)
        .is_ok_and(|metadata| metadata.file_type().is_symlink() || !metadata.is_file())
    {
        return Err(AppError::new(
            "RECOVERY_PROJECTION_LOCK_FAILED",
            "外部投影锁文件不安全",
        ));
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|_| AppError::new("RECOVERY_PROJECTION_LOCK_FAILED", "无法打开外部投影锁"))?;

    #[cfg(unix)]
    rustix::fs::flock(&file, rustix::fs::FlockOperation::NonBlockingLockExclusive)
        .map_err(|_| AppError::new("RECOVERY_REPLAY_BUSY", "另一个应用实例正在执行外部投影"))?;

    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Storage::FileSystem::{
            LockFileEx, LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY,
        };
        use windows_sys::Win32::System::IO::OVERLAPPED;

        let mut overlapped = unsafe { std::mem::zeroed::<OVERLAPPED>() };
        let locked = unsafe {
            LockFileEx(
                file.as_raw_handle() as _,
                LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY,
                0,
                1,
                0,
                &mut overlapped,
            )
        };
        if locked == 0 {
            return Err(AppError::new(
                "RECOVERY_REPLAY_BUSY",
                "另一个应用实例正在执行外部投影",
            ));
        }
    }

    Ok(ProjectionLock { _file: file })
}

fn acquire_projection_lock<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> AppResult<ProjectionLock> {
    let path = crate::app_paths::app_data_dir(app)?
        .join(".maintenance")
        .join("external-effect-projection.lock");
    acquire_projection_lock_at(&path)
}

/// Handle handed to a mutation after its prepared row has been committed.
///
/// The handle is deliberately narrow: callers can only record bounded JSON
/// context and an opaque artifact reference owned by this operation. They
/// cannot alter the journal status or another operation's row.
pub(crate) struct RecoveryOperation {
    db: db::Db,
    entry: JournalEntry,
    claim: ClaimToken,
    authoritative_committed: AtomicBool,
}

impl RecoveryOperation {
    pub(crate) fn operation_id(&self) -> &str {
        &self.entry.operation_id
    }

    pub(crate) fn entry(&self) -> &JournalEntry {
        &self.entry
    }

    /// Marks the point where SQLite (or journal-owned intent) is authoritative.
    /// Startup replay does not depend on this in-process flag; it only keeps a
    /// caught pre-commit error from projecting unrelated database state.
    pub(crate) fn mark_authoritative_committed(&self) {
        self.authoritative_committed.store(true, Ordering::Release);
    }

    fn is_authoritative_committed(&self) -> bool {
        self.authoritative_committed.load(Ordering::Acquire)
    }

    pub(crate) fn checkpoint_phase_with_conn(
        &self,
        conn: &Connection,
        phase: &'static str,
    ) -> AppResult<()> {
        validate_operation_kind(phase)?;
        let now = now_unix_seconds();
        let changed = conn
            .execute(
                r#"
UPDATE external_effect_recovery_journal
SET phase = ?1,
    lease_expires_at = ?2,
    updated_at = ?3
WHERE operation_id = ?4
  AND status != 'resolved'
  AND lease_owner = ?5
  AND claim_epoch = ?6
"#,
                params![
                    phase,
                    now.saturating_add(OPERATION_LEASE_SECONDS),
                    now,
                    self.entry.operation_id,
                    self.claim.owner,
                    self.claim.epoch,
                ],
            )
            .map_err(|_| AppError::new("RECOVERY_JOURNAL_UPDATE_FAILED", "无法记录恢复阶段"))?;
        if changed == 1 {
            return Ok(());
        }
        Err(AppError::new(
            "RECOVERY_JOURNAL_STATE_CONFLICT",
            "恢复操作状态已变化",
        ))
    }

    pub(crate) fn renew_lease(&self) -> AppResult<()> {
        let conn = self.db.open_connection()?;
        self.renew_lease_with_conn(&conn)
    }

    pub(crate) fn renew_lease_with_conn(&self, conn: &Connection) -> AppResult<()> {
        let now = now_unix_seconds();
        let changed = conn
            .execute(
                r#"
UPDATE external_effect_recovery_journal
SET lease_expires_at = ?1, updated_at = ?2
WHERE operation_id = ?3
  AND status != 'resolved'
  AND lease_owner = ?4
  AND claim_epoch = ?5
"#,
                params![
                    now.saturating_add(OPERATION_LEASE_SECONDS),
                    now,
                    self.entry.operation_id,
                    self.claim.owner,
                    self.claim.epoch,
                ],
            )
            .map_err(|_| AppError::new("RECOVERY_JOURNAL_UPDATE_FAILED", "无法续期恢复操作"))?;
        if changed == 1 {
            return Ok(());
        }
        Err(AppError::new(
            "RECOVERY_JOURNAL_STATE_CONFLICT",
            "恢复操作已被其他实例接管",
        ))
    }

    pub(crate) fn configure_replay_with_conn(
        &self,
        conn: &Connection,
        replay_context: &str,
        artifact_ref: Option<&str>,
        artifact_sha256: Option<&str>,
    ) -> AppResult<()> {
        validate_replay_context_for_kind(&self.entry.operation_kind, replay_context)?;
        match (artifact_ref, artifact_sha256) {
            (Some(reference), Some(digest)) => {
                validate_artifact_ref(reference)?;
                validate_sha256(digest)?;
            }
            (None, None) => {}
            _ => {
                return Err(AppError::new(
                    "RECOVERY_JOURNAL_INVALID",
                    "恢复制品引用与摘要必须同时提供",
                ))
            }
        }

        let now = now_unix_seconds();
        let changed = conn
            .execute(
                r#"
UPDATE external_effect_recovery_journal
SET replay_context = ?1,
    artifact_ref = ?2,
    artifact_sha256 = ?3,
    updated_at = ?4,
    lease_expires_at = ?5
WHERE operation_id = ?6
  AND status != 'resolved'
  AND lease_owner = ?7
  AND claim_epoch = ?8
"#,
                params![
                    replay_context,
                    artifact_ref,
                    artifact_sha256,
                    now,
                    now.saturating_add(OPERATION_LEASE_SECONDS),
                    self.entry.operation_id,
                    self.claim.owner,
                    self.claim.epoch,
                ],
            )
            .map_err(|_| AppError::new("RECOVERY_JOURNAL_UPDATE_FAILED", "无法登记恢复制品"))?;
        if changed == 1 {
            return Ok(());
        }
        Err(AppError::new(
            "RECOVERY_JOURNAL_STATE_CONFLICT",
            "恢复操作状态已变化",
        ))
    }

    pub(crate) fn set_replay_context_with_conn(
        &self,
        conn: &Connection,
        replay_context: &str,
    ) -> AppResult<()> {
        self.configure_replay_with_conn(conn, replay_context, None, None)
    }
}

fn validate_operation_kind(kind: &str) -> AppResult<()> {
    if kind.is_empty()
        || kind.len() > 96
        || !kind
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(AppError::new(
            "RECOVERY_JOURNAL_INVALID",
            "恢复操作类型无效",
        ));
    }
    Ok(())
}

fn validate_replay_context(context: &str) -> AppResult<()> {
    if context.is_empty()
        || context.len() > REPLAY_CONTEXT_MAX_BYTES
        || serde_json::from_str::<serde_json::Value>(context).is_err()
    {
        return Err(AppError::new("RECOVERY_JOURNAL_INVALID", "恢复上下文无效"));
    }
    Ok(())
}

fn validate_replay_context_for_kind(kind: &str, context: &str) -> AppResult<()> {
    validate_replay_context(context)?;
    let value = serde_json::from_str::<serde_json::Value>(context)
        .map_err(|_| AppError::new("RECOVERY_JOURNAL_INVALID", "恢复上下文无效"))?;
    let object = value
        .as_object()
        .ok_or_else(|| AppError::new("RECOVERY_JOURNAL_INVALID", "恢复上下文必须是对象"))?;

    let (expected_operation, allowed, required): (&str, &[&str], &[&str]) = match kind {
        "workspace.apply" => (
            "",
            &[
                "schema_version",
                "cli_key",
                "from_workspace_id",
                "to_workspace_id",
            ],
            &["schema_version", "cli_key", "to_workspace_id"],
        ),
        "skill.install" => (
            "install",
            &["operation", "workspace_id", "skill_key"],
            &["operation", "workspace_id", "skill_key"],
        ),
        "skill.import_local" => (
            "import_local",
            &[
                "operation",
                "workspace_id",
                "cli_key",
                "skill_key",
                "local_dir_name",
            ],
            &[
                "operation",
                "workspace_id",
                "cli_key",
                "skill_key",
                "local_dir_name",
            ],
        ),
        "skill.update" => (
            "update",
            &["operation", "workspace_id", "skill_id", "skill_key"],
            &["operation", "workspace_id", "skill_id", "skill_key"],
        ),
        "skill.uninstall" => (
            "uninstall",
            &["operation", "skill_id", "skill_key"],
            &["operation", "skill_id", "skill_key"],
        ),
        "skill.return_to_local" => (
            "return_to_local",
            &[
                "operation",
                "workspace_id",
                "cli_key",
                "skill_id",
                "skill_key",
            ],
            &[
                "operation",
                "workspace_id",
                "cli_key",
                "skill_id",
                "skill_key",
            ],
        ),
        "skill.install_to_local" => (
            "install_to_local",
            &["operation", "workspace_id", "cli_key", "dir_name"],
            &["operation", "workspace_id", "cli_key", "dir_name"],
        ),
        "skill.local_delete" => (
            "delete_local",
            &["operation", "workspace_id", "cli_key", "dir_name"],
            &["operation", "workspace_id", "cli_key", "dir_name"],
        ),
        _ => {
            return Err(AppError::new(
                "RECOVERY_JOURNAL_INVALID",
                "该恢复操作不允许登记上下文",
            ))
        }
    };

    if !expected_operation.is_empty()
        && object.get("operation").and_then(serde_json::Value::as_str) != Some(expected_operation)
    {
        return Err(AppError::new(
            "RECOVERY_JOURNAL_INVALID",
            "恢复操作上下文不匹配",
        ));
    }
    if object.keys().any(|key| !allowed.contains(&key.as_str()))
        || required.iter().any(|key| !object.contains_key(*key))
    {
        return Err(AppError::new(
            "RECOVERY_JOURNAL_INVALID",
            "恢复上下文字段无效",
        ));
    }
    for key in ["workspace_id", "to_workspace_id", "skill_id"] {
        if let Some(value) = object.get(key) {
            if value.as_i64().is_none_or(|id| id <= 0) {
                return Err(AppError::new(
                    "RECOVERY_JOURNAL_INVALID",
                    "恢复工作区标识无效",
                ));
            }
        }
    }
    if let Some(value) = object.get("from_workspace_id") {
        if !value.is_null() && value.as_i64().is_none_or(|id| id <= 0) {
            return Err(AppError::new(
                "RECOVERY_JOURNAL_INVALID",
                "恢复工作区标识无效",
            ));
        }
    }
    for key in ["cli_key", "skill_key", "local_dir_name", "dir_name"] {
        if let Some(value) = object.get(key) {
            let Some(raw) = value.as_str() else {
                return Err(AppError::new(
                    "RECOVERY_JOURNAL_INVALID",
                    "恢复上下文字段类型无效",
                ));
            };
            if raw.is_empty() || raw.len() > 256 || raw.chars().any(char::is_control) {
                return Err(AppError::new(
                    "RECOVERY_JOURNAL_INVALID",
                    "恢复上下文字段无效",
                ));
            }
            if key == "cli_key" {
                crate::shared::cli_key::validate_cli_key(raw)?;
            } else {
                crate::skills::validate_recovery_path_component(raw).map_err(|_| {
                    AppError::new("RECOVERY_JOURNAL_INVALID", "恢复上下文路径组件无效")
                })?;
            }
        }
    }
    if object
        .get("schema_version")
        .is_some_and(|value| value.as_u64() != Some(1))
    {
        return Err(AppError::new(
            "RECOVERY_JOURNAL_INVALID",
            "恢复上下文版本无效",
        ));
    }
    Ok(())
}

fn validate_artifact_ref(reference: &str) -> AppResult<()> {
    if !crate::shared::uuid::is_canonical_uuid_v4(reference) {
        return Err(AppError::new(
            "RECOVERY_JOURNAL_INVALID",
            "恢复制品引用无效",
        ));
    }
    Ok(())
}

fn validate_sha256(value: &str) -> AppResult<()> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(AppError::new(
            "RECOVERY_JOURNAL_INVALID",
            "恢复制品摘要无效",
        ));
    }
    Ok(())
}

fn validate_context(context: &JournalContext) -> AppResult<()> {
    if let Some(cli_key) = context.cli_key.as_deref() {
        crate::shared::cli_key::validate_cli_key(cli_key)?;
    }
    if context.workspace_id.is_some_and(|value| value <= 0)
        || context.entity_id.is_some_and(|value| value <= 0)
    {
        return Err(AppError::new(
            "RECOVERY_JOURNAL_INVALID",
            "恢复操作标识无效",
        ));
    }
    if context.parent_operation_id.is_some() {
        return Err(AppError::new(
            "RECOVERY_JOURNAL_INVALID",
            "当前版本不支持子恢复操作",
        ));
    }
    if context.replay_context.is_some() {
        return Err(AppError::new(
            "RECOVERY_JOURNAL_INVALID",
            "恢复上下文必须在操作提交前登记",
        ));
    }
    Ok(())
}

fn infer_cli_key(
    conn: &rusqlite::Connection,
    kind: &str,
    context: &JournalContext,
) -> AppResult<Option<String>> {
    if context.cli_key.is_some() {
        return Ok(context.cli_key.clone());
    }
    if let Some(workspace_id) = context.workspace_id {
        return conn
            .query_row(
                "SELECT cli_key FROM workspaces WHERE id = ?1",
                [workspace_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| AppError::new("RECOVERY_JOURNAL_DB_FAILED", "无法解析恢复操作范围"));
    }

    // Prompt rows have a single workspace scope. MCP and Skills entities can
    // affect multiple active CLI projections and are intentionally handled by
    // an explicit global replay branch below instead of accidental fan-out.
    if kind.starts_with("prompt.") {
        if let Some(prompt_id) = context.entity_id {
            return conn
                .query_row(
                    r#"
SELECT w.cli_key
FROM prompts p
JOIN workspaces w ON w.id = p.workspace_id
WHERE p.id = ?1
"#,
                    [prompt_id],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(|_| {
                    AppError::new("RECOVERY_JOURNAL_DB_FAILED", "无法解析 Prompt 恢复范围")
                });
        }
    }
    Ok(None)
}

fn row_to_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<JournalEntry> {
    Ok(JournalEntry {
        operation_id: row.get(0)?,
        parent_operation_id: row.get(1)?,
        operation_kind: row.get(2)?,
        cli_key: row.get(3)?,
        workspace_id: row.get(4)?,
        entity_id: row.get(5)?,
        phase: row.get(6)?,
        status: row.get(7)?,
        artifact_ref: row.get(8)?,
        artifact_sha256: row.get(9)?,
        replay_context: row.get(10)?,
    })
}

const ENTRY_COLUMNS: &str = r#"
operation_id,
parent_operation_id,
operation_kind,
cli_key,
workspace_id,
entity_id,
phase,
status,
artifact_ref,
artifact_sha256,
replay_context
"#;

fn load_entry(conn: &rusqlite::Connection, operation_id: &str) -> AppResult<Option<JournalEntry>> {
    conn.query_row(
        &format!(
            "SELECT {ENTRY_COLUMNS} FROM external_effect_recovery_journal WHERE operation_id = ?1"
        ),
        [operation_id],
        row_to_entry,
    )
    .optional()
    .map_err(|_| AppError::new("RECOVERY_JOURNAL_DB_FAILED", "无法读取恢复日志"))
}

fn prepare(db: &db::Db, kind: &str, context: &JournalContext) -> AppResult<ClaimedEntry> {
    validate_operation_kind(kind)?;
    validate_context(context)?;

    let mut conn = db.open_connection()?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| AppError::new("RECOVERY_JOURNAL_PREPARE_FAILED", "无法锁定恢复日志"))?;
    let cli_key = infer_cli_key(&tx, kind, context)?;
    let operation_id = crate::shared::uuid::new_uuid_v4();
    let owner = crate::shared::uuid::new_uuid_v4();
    let now = now_unix_seconds();
    let phase = context.phase.unwrap_or("prepare");
    tx.execute(
        r#"
INSERT INTO external_effect_recovery_journal(
  operation_id,
  parent_operation_id,
  operation_kind,
  cli_key,
  workspace_id,
  entity_id,
  phase,
  status,
  replay_context,
  lease_owner,
  lease_expires_at,
  claim_epoch,
  attempt_count,
  created_at,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'prepared', ?8, ?9, ?10, 1, 0, ?11, ?11)
"#,
        params![
            operation_id,
            context.parent_operation_id,
            kind,
            cli_key,
            context.workspace_id,
            context.entity_id,
            phase,
            context.replay_context,
            owner,
            now.saturating_add(OPERATION_LEASE_SECONDS),
            now,
        ],
    )
    .map_err(|_| {
        AppError::new(
            "RECOVERY_JOURNAL_PREPARE_FAILED",
            "无法持久化恢复日志，未执行外部写入",
        )
    })?;
    tx.commit()
        .map_err(|_| AppError::new("RECOVERY_JOURNAL_PREPARE_FAILED", "无法提交恢复日志"))?;

    Ok(ClaimedEntry {
        entry: JournalEntry {
            operation_id,
            parent_operation_id: context.parent_operation_id.clone(),
            operation_kind: kind.to_string(),
            cli_key,
            workspace_id: context.workspace_id,
            entity_id: context.entity_id,
            phase: phase.to_string(),
            status: "prepared".to_string(),
            artifact_ref: None,
            artifact_sha256: None,
            replay_context: context.replay_context.clone(),
        },
        claim: ClaimToken { owner, epoch: 1 },
    })
}

fn update_claimed_status(
    db: &db::Db,
    claimed: &ClaimedEntry,
    status: &str,
    phase: &str,
    release_lease: bool,
) -> AppResult<()> {
    let now = now_unix_seconds();
    let conn = db.open_connection()?;
    let changed = conn
        .execute(
            r#"
UPDATE external_effect_recovery_journal
SET status = ?1,
    phase = ?2,
    lease_owner = CASE WHEN ?3 THEN NULL ELSE lease_owner END,
    lease_expires_at = CASE WHEN ?3 THEN 0 ELSE ?4 END,
    updated_at = ?5
WHERE operation_id = ?6
  AND lease_owner = ?7
  AND claim_epoch = ?8
  AND status != 'resolved'
"#,
            params![
                status,
                phase,
                release_lease,
                now.saturating_add(OPERATION_LEASE_SECONDS),
                now,
                claimed.entry.operation_id,
                claimed.claim.owner,
                claimed.claim.epoch,
            ],
        )
        .map_err(|_| AppError::new("RECOVERY_JOURNAL_UPDATE_FAILED", "无法更新恢复日志"))?;
    if changed == 1 {
        return Ok(());
    }
    if load_entry(&conn, &claimed.entry.operation_id)?
        .is_some_and(|entry| entry.status == "resolved")
    {
        return Ok(());
    }
    Err(AppError::new(
        "RECOVERY_JOURNAL_STATE_CONFLICT",
        "恢复日志状态冲突",
    ))
}

fn error_summary(
    primary: &AppError,
    replay_error: Option<&AppError>,
    journal_error: Option<&AppError>,
) -> String {
    let mut summary = format!("primary={}", primary.code());
    if let Some(replay_error) = replay_error {
        summary.push_str("; replay=");
        summary.push_str(replay_error.code());
    }
    if let Some(journal_error) = journal_error {
        summary.push_str("; journal=");
        summary.push_str(journal_error.code());
    }
    summary.chars().take(512).collect()
}

fn record_failure(
    db: &db::Db,
    claimed: &ClaimedEntry,
    primary: &AppError,
    replay_error: Option<&AppError>,
) -> AppResult<()> {
    let conn = db.open_connection()?;
    let summary = error_summary(primary, replay_error, None);
    let changed = conn
        .execute(
            r#"
UPDATE external_effect_recovery_journal
SET status = 'failed',
    phase = phase,
    lease_owner = NULL,
    lease_expires_at = 0,
    attempt_count = attempt_count + 1,
    error_code = ?1,
    error_summary = ?2,
    updated_at = ?3
WHERE operation_id = ?4
  AND lease_owner = ?5
  AND claim_epoch = ?6
  AND status != 'resolved'
"#,
            params![
                primary.code(),
                summary,
                now_unix_seconds(),
                claimed.entry.operation_id,
                claimed.claim.owner,
                claimed.claim.epoch,
            ],
        )
        .map_err(|_| AppError::new("RECOVERY_JOURNAL_UPDATE_FAILED", "无法记录恢复失败"))?;
    if changed == 1 {
        return Ok(());
    }
    if load_entry(&conn, &claimed.entry.operation_id)?
        .is_some_and(|entry| entry.status == "resolved")
    {
        return Ok(());
    }
    Err(AppError::new(
        "RECOVERY_JOURNAL_STATE_CONFLICT",
        "恢复日志状态冲突",
    ))
}

fn resolve_claimed(db: &db::Db, claimed: &ClaimedEntry) -> AppResult<()> {
    let conn = db.open_connection()?;
    let changed = conn
        .execute(
            r#"
UPDATE external_effect_recovery_journal
SET status = 'resolved',
    phase = 'resolved',
    lease_owner = NULL,
    lease_expires_at = 0,
    error_code = NULL,
    error_summary = NULL,
    updated_at = ?1
WHERE operation_id = ?2
  AND lease_owner = ?3
  AND claim_epoch = ?4
  AND status != 'resolved'
"#,
            params![
                now_unix_seconds(),
                claimed.entry.operation_id,
                claimed.claim.owner,
                claimed.claim.epoch,
            ],
        )
        .map_err(|_| AppError::new("RECOVERY_JOURNAL_UPDATE_FAILED", "无法完成恢复日志"))?;
    if changed != 1
        && load_entry(&conn, &claimed.entry.operation_id)?
            .is_none_or(|entry| entry.status != "resolved")
    {
        return Err(AppError::new(
            "RECOVERY_JOURNAL_STATE_CONFLICT",
            "恢复日志状态冲突",
        ));
    }

    let cutoff = now_unix_seconds().saturating_sub(JOURNAL_RETENTION_SECONDS);
    let _ = conn.execute(
        "DELETE FROM external_effect_recovery_journal WHERE status = 'resolved' AND updated_at < ?1",
        [cutoff],
    );
    Ok(())
}

fn operation_domain(kind: &str) -> &'static str {
    if kind.starts_with("prompt.") {
        "prompt"
    } else if kind.starts_with("mcp.") {
        "mcp"
    } else if kind.starts_with("skill.") {
        "skill"
    } else {
        "workspace"
    }
}

fn sync_cli_domain<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    conn: &rusqlite::Connection,
    cli_key: &str,
    domain: &str,
) -> AppResult<()> {
    let cli = CliKey::parse(cli_key)?;
    if matches!(domain, "prompt" | "workspace") && cli.supports(CliCapability::Prompts) {
        crate::prompts::sync_one_cli(app, conn, cli_key)?;
    }
    if matches!(domain, "mcp" | "workspace") && cli.supports(CliCapability::Mcp) {
        crate::mcp::sync_one_cli(app, conn, cli_key)?;
    }
    if matches!(domain, "skill" | "workspace") && cli.supports(CliCapability::Skills) {
        crate::skills::sync_one_cli(app, conn, cli_key)?;
    }
    Ok(())
}

fn replay_global_domain<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    conn: &rusqlite::Connection,
    domain: &str,
) -> AppResult<()> {
    match domain {
        "prompt" => {
            for cli_key in crate::shared::cli_key::cli_keys_with(CliCapability::Prompts) {
                sync_cli_domain(app, conn, cli_key, domain)?;
            }
        }
        "mcp" => {
            for cli_key in crate::shared::cli_key::cli_keys_with(CliCapability::Mcp) {
                sync_cli_domain(app, conn, cli_key, domain)?;
            }
        }
        "skill" => {
            for cli_key in crate::shared::cli_key::cli_keys_with(CliCapability::Skills) {
                sync_cli_domain(app, conn, cli_key, domain)?;
            }
        }
        _ => {
            return Err(AppError::new(
                "RECOVERY_JOURNAL_SCOPE_MISSING",
                "恢复操作缺少明确的 CLI 范围",
            ))
        }
    }
    Ok(())
}

fn replay_entry<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &db::Db,
    claimed: &ClaimedEntry,
) -> AppResult<()> {
    if claimed.entry.parent_operation_id.is_some() {
        return Err(AppError::new(
            "RECOVERY_JOURNAL_CHILD_UNSUPPORTED",
            "当前版本不支持子恢复操作",
        ));
    }
    ensure_claim_is_active(db, claimed)?;
    let entry = &claimed.entry;
    let operation = RecoveryOperation {
        db: db.clone(),
        entry: entry.clone(),
        claim: claimed.claim.clone(),
        authoritative_committed: AtomicBool::new(true),
    };
    operation.renew_lease()?;
    if entry.operation_kind == "prompt.default_sync" {
        // This operation imports files into SQLite. It must never project the
        // normalized database representation back over those source files.
        return Ok(());
    }
    if entry.operation_kind == "workspace.apply" {
        return crate::workspace_switch::replay_recovery_operation(app, db, &operation);
    }
    if entry.operation_kind.starts_with("skill.") {
        return crate::skills::replay_recovery_operation(app, db, &operation);
    }

    let conn = db.open_connection()?;
    let domain = operation_domain(&entry.operation_kind);
    if let Some(cli_key) = entry.cli_key.as_deref() {
        return sync_cli_domain(app, &conn, cli_key, domain);
    }
    replay_global_domain(app, &conn, domain)
}

fn cleanup_entry<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    entry: &JournalEntry,
) -> AppResult<()> {
    if entry.operation_kind == "workspace.apply" {
        return crate::workspace_switch::cleanup_recovery_operation(app, entry);
    }
    if entry.operation_kind.starts_with("skill.") {
        return crate::skills::cleanup_recovery_operation(app, entry);
    }
    Ok(())
}

fn refresh_claimed(db: &db::Db, claimed: &ClaimedEntry) -> AppResult<ClaimedEntry> {
    let conn = db.open_connection()?;
    let Some(entry) = load_entry(&conn, &claimed.entry.operation_id)? else {
        return Err(AppError::new(
            "RECOVERY_JOURNAL_STATE_CONFLICT",
            "恢复日志已不存在",
        ));
    };
    Ok(ClaimedEntry {
        entry,
        claim: claimed.claim.clone(),
    })
}

fn ensure_claim_is_active(db: &db::Db, claimed: &ClaimedEntry) -> AppResult<()> {
    let conn = db.open_connection()?;
    let active: bool = conn
        .query_row(
            r#"
SELECT EXISTS(
  SELECT 1
  FROM external_effect_recovery_journal
  WHERE operation_id = ?1
    AND status != 'resolved'
    AND lease_owner = ?2
    AND claim_epoch = ?3
)
"#,
            params![
                claimed.entry.operation_id,
                claimed.claim.owner,
                claimed.claim.epoch,
            ],
            |row| row.get(0),
        )
        .map_err(|_| AppError::new("RECOVERY_JOURNAL_DB_FAILED", "无法确认恢复操作租约"))?;
    if active {
        Ok(())
    } else {
        Err(AppError::new(
            "RECOVERY_JOURNAL_STATE_CONFLICT",
            "恢复操作已被其他实例接管",
        ))
    }
}

fn finish_projection<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &db::Db,
    claimed: &ClaimedEntry,
) -> AppResult<()> {
    let claimed = refresh_claimed(db, claimed)?;
    if claimed.entry.phase != "cleanup_pending" {
        replay_entry(app, db, &claimed)?;
        // Persist this boundary before deleting an artifact. A crash after
        // cleanup is then safely retried as cleanup-only rather than trying to
        // reconstruct from a deliberately removed artifact.
        update_claimed_status(db, &claimed, "committed", "cleanup_pending", false)?;
    }
    cleanup_entry(app, &claimed.entry)?;
    resolve_claimed(db, &claimed)
}

fn finish_without_projection<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &db::Db,
    claimed: &ClaimedEntry,
) -> AppResult<()> {
    update_claimed_status(db, claimed, "committed", "cleanup_pending", false)?;
    let claimed = refresh_claimed(db, claimed)?;
    cleanup_entry(app, &claimed.entry)?;
    resolve_claimed(db, &claimed)
}

fn composite_error(
    primary: &AppError,
    replay_error: Option<&AppError>,
    journal_error: Option<&AppError>,
) -> AppError {
    AppError::new(
        primary.code(),
        format!(
            "外部投影操作失败（{}）",
            error_summary(primary, replay_error, journal_error)
        ),
    )
}

fn fail_after_replay(
    db: &db::Db,
    claimed: &ClaimedEntry,
    primary: AppError,
    replay_error: Option<AppError>,
) -> AppError {
    match record_failure(db, claimed, &primary, replay_error.as_ref()) {
        Ok(()) => composite_error(&primary, replay_error.as_ref(), None),
        Err(journal_error) => {
            composite_error(&primary, replay_error.as_ref(), Some(&journal_error))
        }
    }
}

fn run_operation<R, T, F>(
    app: &tauri::AppHandle<R>,
    db: &db::Db,
    kind: &str,
    context: JournalContext,
    work: F,
) -> AppResult<T>
where
    R: tauri::Runtime,
    F: FnOnce(&RecoveryOperation) -> AppResult<T>,
{
    let _projection_lock = acquire_projection_lock(app)?;
    let claimed = prepare(db, kind, &context)?;
    let operation = RecoveryOperation {
        db: db.clone(),
        entry: claimed.entry.clone(),
        claim: claimed.claim.clone(),
        authoritative_committed: AtomicBool::new(false),
    };

    match work(&operation) {
        Ok(value) => {
            let committed_phase = if kind == "workspace.apply" {
                let phase = (|| {
                    let conn = db.open_connection()?;
                    load_entry(&conn, operation.operation_id())?.ok_or_else(|| {
                        AppError::new("RECOVERY_JOURNAL_STATE_CONFLICT", "恢复操作已不存在")
                    })
                })();
                match phase {
                    Ok(phase) => phase.phase,
                    Err(error) => {
                        let failure = fail_after_replay(db, &claimed, error, None);
                        crate::app::maintenance::fail_recovery_replay(app, failure.clone());
                        return Err(failure);
                    }
                }
            } else {
                "authoritative_projection".to_string()
            };
            if let Err(error) =
                update_claimed_status(db, &claimed, "committed", &committed_phase, false)
            {
                let failure = fail_after_replay(db, &claimed, error, None);
                crate::app::maintenance::fail_recovery_replay(app, failure.clone());
                return Err(failure);
            }
            if let Err(replay_error) = finish_projection(app, db, &claimed) {
                let primary = AppError::new("RECOVERY_REPLAY_FAILED", "提交后的外部投影尚未收敛");
                let failure = fail_after_replay(db, &claimed, primary, Some(replay_error));
                crate::app::maintenance::fail_recovery_replay(app, failure.clone());
                return Err(failure);
            }
            Ok(value)
        }
        Err(primary) => {
            let recovery = if operation.is_authoritative_committed() {
                finish_projection(app, db, &claimed)
            } else {
                finish_without_projection(app, db, &claimed)
            };
            match recovery {
                Ok(()) => Err(composite_error(&primary, None, None)),
                Err(replay_error) => {
                    let failure = fail_after_replay(db, &claimed, primary, Some(replay_error));
                    crate::app::maintenance::fail_recovery_replay(app, failure.clone());
                    Err(failure)
                }
            }
        }
    }
}

pub(crate) async fn run_blocking_operation<T, F>(
    task_name: &'static str,
    app: tauri::AppHandle,
    db: db::Db,
    kind: &'static str,
    context: JournalContext,
    work: F,
) -> AppResult<T>
where
    T: Send + 'static,
    F: FnOnce(&RecoveryOperation) -> AppResult<T> + Send + 'static,
{
    crate::blocking::run(task_name, move || {
        run_operation(&app, &db, kind, context, work)
    })
    .await
}

pub(crate) fn run_operation_for_test<R, T, F>(
    app: &tauri::AppHandle<R>,
    db: &db::Db,
    kind: &str,
    context: JournalContext,
    work: F,
) -> AppResult<T>
where
    R: tauri::Runtime,
    F: FnOnce(&RecoveryOperation) -> AppResult<T>,
{
    run_operation(app, db, kind, context, work)
}

fn acquire_replay_coordinator(db: &db::Db) -> AppResult<Option<ClaimToken>> {
    let now = now_unix_seconds();
    let owner = crate::shared::uuid::new_uuid_v4();
    let mut conn = db.open_connection()?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| AppError::new("RECOVERY_JOURNAL_DB_FAILED", "无法锁定恢复协调器"))?;
    let changed = tx
        .execute(
            r#"
UPDATE external_effect_recovery_coordinator
SET lease_owner = ?1,
    lease_expires_at = ?2,
    claim_epoch = claim_epoch + 1,
    updated_at = ?3
WHERE coordinator_key = 'replay'
  AND (lease_owner IS NULL OR lease_expires_at <= ?3)
"#,
            params![
                owner,
                now.saturating_add(REPLAY_COORDINATOR_LEASE_SECONDS),
                now,
            ],
        )
        .map_err(|_| AppError::new("RECOVERY_JOURNAL_DB_FAILED", "无法获取恢复协调器"))?;
    if changed == 0 {
        tx.commit()
            .map_err(|_| AppError::new("RECOVERY_JOURNAL_DB_FAILED", "无法释放恢复协调器读取锁"))?;
        return Ok(None);
    }
    let epoch = tx
        .query_row(
            "SELECT claim_epoch FROM external_effect_recovery_coordinator WHERE coordinator_key = 'replay' AND lease_owner = ?1",
            [&owner],
            |row| row.get(0),
        )
        .map_err(|_| AppError::new("RECOVERY_JOURNAL_DB_FAILED", "无法读取恢复协调器"))?;
    tx.commit()
        .map_err(|_| AppError::new("RECOVERY_JOURNAL_DB_FAILED", "无法提交恢复协调器"))?;
    Ok(Some(ClaimToken { owner, epoch }))
}

fn renew_replay_coordinator(db: &db::Db, claim: &ClaimToken) -> AppResult<()> {
    let now = now_unix_seconds();
    let conn = db.open_connection()?;
    let changed = conn
        .execute(
            r#"
UPDATE external_effect_recovery_coordinator
SET lease_expires_at = ?1, updated_at = ?2
WHERE coordinator_key = 'replay'
  AND lease_owner = ?3
  AND claim_epoch = ?4
  AND lease_expires_at > ?2
"#,
            params![
                now.saturating_add(REPLAY_COORDINATOR_LEASE_SECONDS),
                now,
                claim.owner,
                claim.epoch,
            ],
        )
        .map_err(|_| AppError::new("RECOVERY_JOURNAL_DB_FAILED", "无法续期恢复协调器"))?;
    if changed == 1 {
        Ok(())
    } else {
        Err(AppError::new(
            "RECOVERY_REPLAY_BUSY",
            "恢复协调器已被其他实例接管",
        ))
    }
}

fn release_replay_coordinator(db: &db::Db, claim: &ClaimToken) -> AppResult<()> {
    let conn = db.open_connection()?;
    let changed = conn
        .execute(
            r#"
UPDATE external_effect_recovery_coordinator
SET lease_owner = NULL, lease_expires_at = 0, updated_at = ?1
WHERE coordinator_key = 'replay' AND lease_owner = ?2 AND claim_epoch = ?3
"#,
            params![now_unix_seconds(), claim.owner, claim.epoch],
        )
        .map_err(|_| AppError::new("RECOVERY_JOURNAL_DB_FAILED", "无法释放恢复协调器"))?;
    if changed == 1 {
        return Ok(());
    }
    Err(AppError::new(
        "RECOVERY_REPLAY_BUSY",
        "恢复协调器已被其他实例接管",
    ))
}

fn claim_next_root_entry(db: &db::Db, coordinator: &ClaimToken) -> AppResult<Option<ClaimedEntry>> {
    let now = now_unix_seconds();
    let mut conn = db.open_connection()?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| AppError::new("RECOVERY_JOURNAL_DB_FAILED", "无法锁定恢复日志"))?;
    let coordinator_active: bool = tx
        .query_row(
            r#"
SELECT EXISTS(
  SELECT 1 FROM external_effect_recovery_coordinator
  WHERE coordinator_key = 'replay'
    AND lease_owner = ?1
    AND claim_epoch = ?2
    AND lease_expires_at > ?3
)
"#,
            params![coordinator.owner, coordinator.epoch, now],
            |row| row.get(0),
        )
        .map_err(|_| AppError::new("RECOVERY_JOURNAL_DB_FAILED", "无法确认恢复协调器"))?;
    if !coordinator_active {
        return Err(AppError::new(
            "RECOVERY_REPLAY_BUSY",
            "恢复协调器已被其他实例接管",
        ));
    }
    let candidate = tx
        .query_row(
            &format!(
                r#"
SELECT {ENTRY_COLUMNS}
FROM external_effect_recovery_journal
WHERE parent_operation_id IS NULL
  AND status != 'resolved'
  AND (lease_owner IS NULL OR lease_expires_at <= ?1)
ORDER BY created_at ASC, operation_id ASC
LIMIT 1
"#
            ),
            [now],
            row_to_entry,
        )
        .optional()
        .map_err(|_| AppError::new("RECOVERY_JOURNAL_DB_FAILED", "无法查询恢复日志"))?;
    let Some(mut entry) = candidate else {
        tx.commit()
            .map_err(|_| AppError::new("RECOVERY_JOURNAL_DB_FAILED", "无法提交恢复日志读取"))?;
        return Ok(None);
    };
    let changed = tx
        .execute(
            r#"
UPDATE external_effect_recovery_journal
SET status = CASE WHEN phase = 'cleanup_pending' THEN status ELSE 'projecting' END,
    lease_owner = ?1,
    lease_expires_at = ?2,
    claim_epoch = claim_epoch + 1,
    attempt_count = attempt_count + 1,
    updated_at = ?3
WHERE operation_id = ?4
  AND status != 'resolved'
  AND (lease_owner IS NULL OR lease_expires_at <= ?3)
"#,
            params![
                coordinator.owner,
                now.saturating_add(OPERATION_LEASE_SECONDS),
                now,
                entry.operation_id,
            ],
        )
        .map_err(|_| AppError::new("RECOVERY_JOURNAL_DB_FAILED", "无法认领恢复日志"))?;
    if changed != 1 {
        tx.commit()
            .map_err(|_| AppError::new("RECOVERY_JOURNAL_DB_FAILED", "无法提交恢复日志认领"))?;
        return Ok(None);
    }
    let epoch = tx
        .query_row(
            "SELECT claim_epoch FROM external_effect_recovery_journal WHERE operation_id = ?1 AND lease_owner = ?2",
            params![entry.operation_id, coordinator.owner],
            |row| row.get(0),
        )
        .map_err(|_| AppError::new("RECOVERY_JOURNAL_DB_FAILED", "无法读取恢复日志认领"))?;
    if entry.phase != "cleanup_pending" {
        entry.status = "projecting".to_string();
    }
    tx.commit()
        .map_err(|_| AppError::new("RECOVERY_JOURNAL_DB_FAILED", "无法提交恢复日志认领"))?;
    Ok(Some(ClaimedEntry {
        entry,
        claim: ClaimToken {
            owner: coordinator.owner.clone(),
            epoch,
        },
    }))
}

fn pending_count(db: &db::Db) -> AppResult<i64> {
    let conn = db.open_connection()?;
    conn.query_row(
        "SELECT COUNT(*) FROM external_effect_recovery_journal WHERE status != 'resolved'",
        [],
        |row| row.get(0),
    )
    .map_err(|_| AppError::new("RECOVERY_JOURNAL_DB_FAILED", "无法检查恢复日志"))
}

fn reject_unresolved_children(db: &db::Db) -> AppResult<()> {
    let conn = db.open_connection()?;
    let present: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM external_effect_recovery_journal WHERE parent_operation_id IS NOT NULL AND status != 'resolved')",
            [],
            |row| row.get(0),
        )
        .map_err(|_| AppError::new("RECOVERY_JOURNAL_DB_FAILED", "无法检查子恢复日志"))?;
    if present {
        Err(AppError::new(
            "RECOVERY_JOURNAL_CHILD_UNSUPPORTED",
            "检测到不受支持的子恢复日志，已停止启动",
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
pub(crate) fn has_pending(db: &db::Db) -> AppResult<bool> {
    let conn = db.open_connection()?;
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM external_effect_recovery_journal WHERE status != 'resolved')",
        [],
        |row| row.get(0),
    )
    .map_err(|_| AppError::new("RECOVERY_JOURNAL_DB_FAILED", "无法检查恢复日志"))
}

pub(crate) fn replay_pending<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &db::Db,
) -> AppResult<usize> {
    let _projection_lock = acquire_projection_lock(app)?;
    reject_unresolved_children(db)?;
    let Some(initial_coordinator) = acquire_replay_coordinator(db)? else {
        return Err(AppError::new(
            "RECOVERY_REPLAY_BUSY",
            "另一个应用实例正在恢复外部投影",
        ));
    };
    let mut coordinator = Some(initial_coordinator);

    let result = (|| {
        let mut resolved = 0usize;
        loop {
            let coordinator_ref = coordinator.as_ref().ok_or_else(|| {
                AppError::new("RECOVERY_REPLAY_BUSY", "恢复协调器已被其他实例接管")
            })?;
            renew_replay_coordinator(db, coordinator_ref)?;
            let Some(claimed) = claim_next_root_entry(db, coordinator_ref)? else {
                if pending_count(db)? > 0 {
                    return Err(AppError::new(
                        "RECOVERY_REPLAY_BUSY",
                        "恢复日志仍由其他实例持有",
                    ));
                }
                return Ok(resolved);
            };
            if let Err(replay_error) = finish_projection(app, db, &claimed) {
                let primary = AppError::new("RECOVERY_REPLAY_FAILED", "启动恢复尚未完成");
                let failure = fail_after_replay(db, &claimed, primary, Some(replay_error));
                return Err(failure);
            }
            resolved += 1;
            // A single projection may legitimately exceed the advisory lease.
            // Releasing and reacquiring between entries renews the fenced
            // coordinator without letting an expired owner extend itself.
            let released = coordinator.take().ok_or_else(|| {
                AppError::new("RECOVERY_REPLAY_BUSY", "恢复协调器已被其他实例接管")
            })?;
            release_replay_coordinator(db, &released)?;
            coordinator = acquire_replay_coordinator(db)?;
        }
    })();
    let release_result = coordinator.as_ref().map_or(Ok(()), |coordinator| {
        release_replay_coordinator(db, coordinator)
    });
    match (result, release_result) {
        (Ok(resolved), Ok(())) => Ok(resolved),
        (Ok(_), Err(release_error)) => Err(release_error),
        (Err(primary), Ok(())) => Err(primary),
        (Err(primary), Err(release_error)) => {
            Err(composite_error(&primary, None, Some(&release_error)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_lock_is_exclusive_and_released_with_guard() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("projection.lock");
        let first = acquire_projection_lock_at(&path).expect("first lock");
        let error = acquire_projection_lock_at(&path).expect_err("second lock must fail");
        assert_eq!(error.code(), "RECOVERY_REPLAY_BUSY");
        drop(first);
        acquire_projection_lock_at(&path).expect("lock released with guard");
    }

    #[test]
    fn journal_error_summary_never_contains_raw_error_text() {
        let error = AppError::new(
            "MCP_SYNC_FAILED",
            "Authorization: Bearer secret-value; password=hunter2; /Users/alice/private",
        );
        let summary = error_summary(&error, None, None);
        assert_eq!(summary, "primary=MCP_SYNC_FAILED");
        assert!(!summary.contains("secret-value"));
        assert!(!summary.contains("hunter2"));
        assert!(!summary.contains("/Users"));
    }

    #[test]
    fn resolve_is_idempotently_terminal() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = crate::db::init_for_tests(&dir.path().join("journal.db")).expect("init db");
        let claimed = prepare(
            &db,
            "prompt.upsert",
            &JournalContext::for_workspace(1).with_cli_key("claude"),
        )
        .expect("prepare journal");
        assert!(has_pending(&db).expect("pending"));
        resolve_claimed(&db, &claimed).expect("resolve journal");
        resolve_claimed(&db, &claimed).expect("repeat resolve journal");
        assert!(!has_pending(&db).expect("resolved"));
    }

    #[test]
    fn invalid_context_is_rejected_before_a_row_is_written() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = crate::db::init_for_tests(&dir.path().join("journal.db")).expect("init db");
        let error = prepare(&db, "prompt.upsert", &JournalContext::for_entity(-1))
            .expect_err("negative entity id must fail");
        assert_eq!(error.code(), "RECOVERY_JOURNAL_INVALID");
        assert!(!has_pending(&db).expect("no row"));
    }

    #[test]
    fn replay_claim_preserves_domain_checkpoint_and_fences_stale_epoch() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = crate::db::init_for_tests(&dir.path().join("journal.db")).expect("init db");
        let claimed = prepare(
            &db,
            "workspace.apply",
            &JournalContext {
                phase: Some("workspace.prompt"),
                ..JournalContext::default()
            },
        )
        .expect("prepare journal");
        let operation = RecoveryOperation {
            db: db.clone(),
            entry: claimed.entry.clone(),
            claim: claimed.claim.clone(),
            authoritative_committed: AtomicBool::new(true),
        };
        db.open_connection()
            .expect("open db")
            .execute(
                "UPDATE external_effect_recovery_journal SET lease_expires_at = 0 WHERE operation_id = ?1",
                [claimed.entry.operation_id.as_str()],
            )
            .expect("expire operation lease");
        let coordinator = acquire_replay_coordinator(&db)
            .expect("acquire coordinator")
            .expect("coordinator available");
        let replay_claim = claim_next_root_entry(&db, &coordinator)
            .expect("claim replay entry")
            .expect("entry available");
        assert_eq!(replay_claim.entry.phase, "workspace.prompt");
        assert_eq!(
            operation
                .renew_lease()
                .expect_err("new epoch must fence stale owner")
                .code(),
            "RECOVERY_JOURNAL_STATE_CONFLICT"
        );
        assert_eq!(
            update_claimed_status(&db, &claimed, "committed", "stale", false)
                .expect_err("stale epoch must reject status update")
                .code(),
            "RECOVERY_JOURNAL_STATE_CONFLICT"
        );
        let stale_error = AppError::new("TEST_FAILURE", "stale owner");
        assert_eq!(
            record_failure(&db, &claimed, &stale_error, None)
                .expect_err("stale epoch must reject failure record")
                .code(),
            "RECOVERY_JOURNAL_STATE_CONFLICT"
        );
        assert_eq!(
            resolve_claimed(&db, &claimed)
                .expect_err("stale epoch must reject resolve")
                .code(),
            "RECOVERY_JOURNAL_STATE_CONFLICT"
        );
        release_replay_coordinator(&db, &coordinator).expect("release coordinator");
    }

    #[test]
    fn coordinator_release_reports_lost_ownership() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = crate::db::init_for_tests(&dir.path().join("journal.db")).expect("init db");
        let coordinator = acquire_replay_coordinator(&db)
            .expect("acquire coordinator")
            .expect("coordinator available");
        let stale = ClaimToken {
            owner: crate::shared::uuid::new_uuid_v4(),
            epoch: coordinator.epoch,
        };

        assert_eq!(
            release_replay_coordinator(&db, &stale)
                .expect_err("lost coordinator ownership must be visible")
                .code(),
            "RECOVERY_REPLAY_BUSY"
        );
        release_replay_coordinator(&db, &coordinator).expect("release coordinator");
    }

    #[test]
    fn replay_context_rejects_unknown_fields_and_parent_rows() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = crate::db::init_for_tests(&dir.path().join("journal.db")).expect("init db");
        let error = prepare(
            &db,
            "skill.install",
            &JournalContext {
                parent_operation_id: Some(crate::shared::uuid::new_uuid_v4()),
                ..JournalContext::default()
            },
        )
        .expect_err("child operation must be rejected");
        assert_eq!(error.code(), "RECOVERY_JOURNAL_INVALID");

        let claimed = prepare(&db, "skill.install", &JournalContext::default())
            .expect("prepare skill journal");
        let operation = RecoveryOperation {
            db,
            entry: claimed.entry,
            claim: claimed.claim,
            authoritative_committed: AtomicBool::new(true),
        };
        let conn = operation.db.open_connection().expect("open journal connection");
        let error = operation
            .set_replay_context_with_conn(
                &conn,
                r#"{"operation":"install","workspace_id":1,"skill_key":"demo","extra":true}"#,
            )
            .expect_err("unknown context field must be rejected");
        assert_eq!(error.code(), "RECOVERY_JOURNAL_INVALID");
    }
}
