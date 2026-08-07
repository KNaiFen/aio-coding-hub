//! Usage: Workspace (profile) preview/apply orchestration.

use crate::claude_plugins;
use crate::db;
use crate::infra::recovery_journal::{JournalEntry, RecoveryOperation};
use crate::mcp_sync;
use crate::prompt_sync;
use crate::shared::cli_key::{CliCapability, CliKey};
use crate::shared::error::{db_err, AppError};
use crate::shared::time::now_unix_seconds;
use crate::{mcp, prompts, skills, workspaces};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WorkspaceEnabledPromptPreview {
    pub name: String,
    pub excerpt: String,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WorkspacePromptsPreview {
    pub from_enabled: Option<WorkspaceEnabledPromptPreview>,
    pub to_enabled: Option<WorkspaceEnabledPromptPreview>,
    pub will_change: bool,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WorkspaceItemsPreview {
    pub from_enabled: Vec<String>,
    pub to_enabled: Vec<String>,
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WorkspacePreview {
    pub cli_key: String,
    pub from_workspace_id: Option<i64>,
    pub to_workspace_id: i64,
    pub prompts: WorkspacePromptsPreview,
    pub mcp: WorkspaceItemsPreview,
    pub skills: WorkspaceItemsPreview,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct WorkspaceApplyReport {
    pub cli_key: String,
    pub from_workspace_id: Option<i64>,
    pub to_workspace_id: i64,
    pub applied_at: i64,
}

fn excerpt(content: &str) -> String {
    const MAX_CHARS: usize = 160;
    let normalized = content.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut cutoff = normalized.len();
    for (idx, (byte_idx, _)) in normalized.char_indices().enumerate() {
        if idx == MAX_CHARS {
            cutoff = byte_idx;
            break;
        }
    }
    if cutoff == normalized.len() {
        return normalized;
    }
    format!("{}…", &normalized[..cutoff])
}

fn enabled_prompt_raw(
    conn: &Connection,
    workspace_id: i64,
) -> Result<Option<(String, String)>, String> {
    conn.query_row(
        r#"
SELECT name, content
FROM prompts
WHERE workspace_id = ?1 AND enabled = 1
ORDER BY updated_at DESC, id DESC
LIMIT 1
"#,
        params![workspace_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .optional()
    .map_err(|e| format!("DB_ERROR: failed to query enabled prompt: {e}"))
}

fn enabled_prompt_preview(
    conn: &Connection,
    workspace_id: Option<i64>,
) -> Result<Option<WorkspaceEnabledPromptPreview>, String> {
    let Some(workspace_id) = workspace_id else {
        return Ok(None);
    };
    let Some((name, content)) = enabled_prompt_raw(conn, workspace_id)? else {
        return Ok(None);
    };
    Ok(Some(WorkspaceEnabledPromptPreview {
        name,
        excerpt: excerpt(&content),
    }))
}

fn list_enabled_mcp_keys(
    conn: &Connection,
    workspace_id: Option<i64>,
) -> Result<Vec<String>, String> {
    let Some(workspace_id) = workspace_id else {
        return Ok(Vec::new());
    };

    let mut stmt = conn
        .prepare_cached(
            r#"
    SELECT s.server_key
    FROM mcp_servers s
    JOIN workspace_mcp_enabled e
      ON e.server_id = s.id
    WHERE e.workspace_id = ?1
    ORDER BY s.server_key ASC
    "#,
        )
        .map_err(|e| db_err!("failed to prepare enabled mcp query: {e}"))?;

    let rows = stmt
        .query_map([workspace_id], |row| row.get::<_, String>(0))
        .map_err(|e| db_err!("failed to query enabled mcp servers: {e}"))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| db_err!("failed to read enabled mcp row: {e}"))?);
    }
    Ok(out)
}

fn list_enabled_skill_keys(
    conn: &Connection,
    workspace_id: Option<i64>,
) -> Result<Vec<String>, String> {
    let Some(workspace_id) = workspace_id else {
        return Ok(Vec::new());
    };

    let mut stmt = conn
        .prepare_cached(
            r#"
    SELECT s.skill_key
    FROM skills s
    JOIN workspace_skill_enabled e
      ON e.skill_id = s.id
    WHERE e.workspace_id = ?1
    ORDER BY s.skill_key ASC
    "#,
        )
        .map_err(|e| db_err!("failed to prepare enabled skills query: {e}"))?;

    let rows = stmt
        .query_map([workspace_id], |row| row.get::<_, String>(0))
        .map_err(|e| db_err!("failed to query enabled skills: {e}"))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| db_err!("failed to read enabled skill row: {e}"))?);
    }
    Ok(out)
}

fn diff(from_enabled: &[String], to_enabled: &[String]) -> (Vec<String>, Vec<String>) {
    let from_set: HashSet<&str> = from_enabled.iter().map(String::as_str).collect();
    let to_set: HashSet<&str> = to_enabled.iter().map(String::as_str).collect();

    let mut added: Vec<String> = to_set
        .difference(&from_set)
        .map(|v| v.to_string())
        .collect();
    let mut removed: Vec<String> = from_set
        .difference(&to_set)
        .map(|v| v.to_string())
        .collect();

    added.sort();
    removed.sort();
    (added, removed)
}

pub fn preview(
    db: &db::Db,
    workspace_id: i64,
) -> crate::shared::error::AppResult<WorkspacePreview> {
    let conn = db.open_connection()?;

    let cli_key = workspaces::get_cli_key_by_id(&conn, workspace_id)?;
    let from_workspace_id = workspaces::active_id_by_cli(&conn, &cli_key)?;

    let from_enabled_prompt = enabled_prompt_preview(&conn, from_workspace_id)?;
    let to_enabled_prompt = enabled_prompt_preview(&conn, Some(workspace_id))?;

    let will_change = match (from_workspace_id, Some(workspace_id)) {
        (None, _) => to_enabled_prompt.is_some(),
        (Some(from_id), Some(to_id)) => {
            let from_raw = enabled_prompt_raw(&conn, from_id)?;
            let to_raw = enabled_prompt_raw(&conn, to_id)?;
            from_raw.map(|v| v.1).unwrap_or_default() != to_raw.map(|v| v.1).unwrap_or_default()
        }
        _ => false,
    };

    let from_mcp = list_enabled_mcp_keys(&conn, from_workspace_id)?;
    let to_mcp = list_enabled_mcp_keys(&conn, Some(workspace_id))?;
    let (mcp_added, mcp_removed) = diff(&from_mcp, &to_mcp);

    let from_skills = list_enabled_skill_keys(&conn, from_workspace_id)?;
    let to_skills = list_enabled_skill_keys(&conn, Some(workspace_id))?;
    let (skills_added, skills_removed) = diff(&from_skills, &to_skills);

    Ok(WorkspacePreview {
        cli_key,
        from_workspace_id,
        to_workspace_id: workspace_id,
        prompts: WorkspacePromptsPreview {
            from_enabled: from_enabled_prompt,
            to_enabled: to_enabled_prompt,
            will_change,
        },
        mcp: WorkspaceItemsPreview {
            from_enabled: from_mcp,
            to_enabled: to_mcp,
            added: mcp_added,
            removed: mcp_removed,
        },
        skills: WorkspaceItemsPreview {
            from_enabled: from_skills,
            to_enabled: to_skills,
            added: skills_added,
            removed: skills_removed,
        },
    })
}

const PHASE_CONTEXT: &str = "workspace.context";
const PHASE_ACTIVE: &str = "workspace.active";
const PHASE_PROMPT: &str = "workspace.prompt";
const PHASE_MCP_CAPTURE: &str = "workspace.mcp_capture";
const PHASE_MCP_MANAGED: &str = "workspace.mcp_managed";
const PHASE_MCP_RESTORE: &str = "workspace.mcp_restore";
const PHASE_CLAUDE_CAPTURE: &str = "workspace.claude_capture";
const PHASE_CLAUDE_RESTORE: &str = "workspace.claude_restore";
const PHASE_SKILLS_MANAGED: &str = "workspace.skills_managed";
const PHASE_SKILLS_CAPTURE: &str = "workspace.skills_capture";
const PHASE_SKILLS_RESTORE: &str = "workspace.skills_restore";
const PHASE_COMPLETE: &str = "workspace.complete";

const WORKSPACE_PHASES: &[&str] = &[
    PHASE_CONTEXT,
    PHASE_ACTIVE,
    PHASE_PROMPT,
    PHASE_MCP_CAPTURE,
    PHASE_MCP_MANAGED,
    PHASE_MCP_RESTORE,
    PHASE_CLAUDE_CAPTURE,
    PHASE_CLAUDE_RESTORE,
    PHASE_SKILLS_MANAGED,
    PHASE_SKILLS_CAPTURE,
    PHASE_SKILLS_RESTORE,
    PHASE_COMPLETE,
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceRecoveryContext {
    schema_version: u8,
    cli_key: String,
    from_workspace_id: Option<i64>,
    to_workspace_id: i64,
}

fn phase_index(phase: &str) -> crate::shared::error::AppResult<usize> {
    if phase == "prepare" {
        return Ok(0);
    }
    WORKSPACE_PHASES
        .iter()
        .position(|candidate| *candidate == phase)
        .map(|index| index + 1)
        .ok_or_else(|| {
            AppError::new(
                "RECOVERY_JOURNAL_INVALID",
                "工作区恢复阶段无效",
            )
        })
}

fn run_phase(
    operation: &RecoveryOperation,
    completed: &mut usize,
    phase: &'static str,
    work: impl FnOnce() -> crate::shared::error::AppResult<()>,
) -> crate::shared::error::AppResult<()> {
    let index = phase_index(phase)?;
    if *completed >= index {
        return Ok(());
    }
    operation.renew_lease()?;
    work()?;
    operation.checkpoint_phase(phase)?;
    *completed = index;
    Ok(())
}

fn context_from_entry(entry: &JournalEntry) -> crate::shared::error::AppResult<WorkspaceRecoveryContext> {
    if entry.operation_kind != "workspace.apply" {
        return Err(AppError::new(
            "RECOVERY_JOURNAL_INVALID",
            "工作区恢复操作类型不匹配",
        ));
    }
    let raw = entry.replay_context.as_deref().ok_or_else(|| {
        AppError::new("RECOVERY_JOURNAL_INVALID", "工作区恢复上下文缺失")
    })?;
    let context: WorkspaceRecoveryContext = serde_json::from_str(raw).map_err(|_| {
        AppError::new("RECOVERY_JOURNAL_INVALID", "工作区恢复上下文损坏")
    })?;
    if context.schema_version != 1
        || context.to_workspace_id <= 0
        || context.from_workspace_id.is_some_and(|value| value <= 0)
        || entry.workspace_id != Some(context.to_workspace_id)
        || entry.cli_key.as_deref() != Some(context.cli_key.as_str())
    {
        return Err(AppError::new(
            "RECOVERY_JOURNAL_INVALID",
            "工作区恢复上下文不匹配",
        ));
    }
    CliKey::parse(&context.cli_key)?;
    Ok(context)
}

fn execute_workspace_projection<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &db::Db,
    operation: &RecoveryOperation,
    context: &WorkspaceRecoveryContext,
) -> crate::shared::error::AppResult<()> {
    let conn = db.open_connection()?;
    let cli_key = workspaces::get_cli_key_by_id(&conn, context.to_workspace_id)?;
    if cli_key != context.cli_key {
        return Err(AppError::new(
            "RECOVERY_JOURNAL_INVALID",
            "工作区恢复 CLI 不匹配",
        ));
    }
    let cli = CliKey::parse(&cli_key)?;
    let mut completed = phase_index(&operation.entry().phase)?;
    if completed >= phase_index(PHASE_ACTIVE)?
        && workspaces::active_id_by_cli(&conn, &cli_key)? != Some(context.to_workspace_id)
    {
        return Err(AppError::new(
            "RECOVERY_JOURNAL_STATE_CONFLICT",
            "SQLite 中的活动工作区已变化，拒绝重放旧投影",
        ));
    }

    run_phase(operation, &mut completed, PHASE_CONTEXT, || Ok(()))?;
    run_phase(operation, &mut completed, PHASE_ACTIVE, || {
        workspaces::set_active(&conn, context.to_workspace_id)?;
        operation.mark_authoritative_committed();
        Ok(())
    })?;

    if cli.supports(CliCapability::Prompts) {
        run_phase(operation, &mut completed, PHASE_PROMPT, || {
            prompts::sync_cli_for_workspace(app, &conn, context.to_workspace_id)
        })?;
    } else {
        run_phase(operation, &mut completed, PHASE_PROMPT, || Ok(()))?;
    }

    if cli.supports(CliCapability::Mcp) {
        let managed_from = list_enabled_mcp_keys(&conn, context.from_workspace_id)?
            .into_iter()
            .collect::<HashSet<_>>();
        let managed_to = list_enabled_mcp_keys(&conn, Some(context.to_workspace_id))?
            .into_iter()
            .collect::<HashSet<_>>();
        run_phase(operation, &mut completed, PHASE_MCP_CAPTURE, || {
            mcp::capture_local_mcp_servers_for_workspace_switch(
                app,
                &cli_key,
                &managed_from,
                context.from_workspace_id,
            )
        })?;
        run_phase(operation, &mut completed, PHASE_MCP_MANAGED, || {
            mcp::sync_cli_for_workspace(app, &conn, context.to_workspace_id)
        })?;
        run_phase(operation, &mut completed, PHASE_MCP_RESTORE, || {
            mcp::restore_local_mcp_servers_for_workspace_switch(
                app,
                &cli_key,
                &managed_to,
                context.to_workspace_id,
            )
        })?;
    } else {
        for phase in [PHASE_MCP_CAPTURE, PHASE_MCP_MANAGED, PHASE_MCP_RESTORE] {
            run_phase(operation, &mut completed, phase, || Ok(()))?;
        }
    }

    if cli_key == "claude" {
        run_phase(operation, &mut completed, PHASE_CLAUDE_CAPTURE, || {
            claude_plugins::capture_local_plugins_for_workspace_switch(
                app,
                &cli_key,
                context.from_workspace_id,
            )
        })?;
        run_phase(operation, &mut completed, PHASE_CLAUDE_RESTORE, || {
            claude_plugins::restore_local_plugins_for_workspace_switch(
                app,
                &cli_key,
                context.to_workspace_id,
            )
        })?;
    } else {
        for phase in [PHASE_CLAUDE_CAPTURE, PHASE_CLAUDE_RESTORE] {
            run_phase(operation, &mut completed, phase, || Ok(()))?;
        }
    }

    if cli.supports(CliCapability::Skills) {
        run_phase(operation, &mut completed, PHASE_SKILLS_MANAGED, || {
            skills::sync_cli_for_workspace(app, &conn, context.to_workspace_id)
        })?;
        run_phase(operation, &mut completed, PHASE_SKILLS_CAPTURE, || {
            skills::capture_staged_local_skills_for_workspace_switch(
                app,
                &conn,
                &cli_key,
                context.from_workspace_id,
                operation,
            )
        })?;
        run_phase(operation, &mut completed, PHASE_SKILLS_RESTORE, || {
            skills::restore_staged_local_skills_for_workspace_switch(
                app,
                &conn,
                &cli_key,
                context.to_workspace_id,
                operation,
            )
        })?;
    } else {
        for phase in [PHASE_SKILLS_MANAGED, PHASE_SKILLS_CAPTURE, PHASE_SKILLS_RESTORE] {
            run_phase(operation, &mut completed, phase, || Ok(()))?;
        }
    }

    run_phase(operation, &mut completed, PHASE_COMPLETE, || Ok(()))
}

pub(crate) fn apply_with_recovery<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &db::Db,
    workspace_id: i64,
    operation: &RecoveryOperation,
) -> crate::shared::error::AppResult<WorkspaceApplyReport> {
    let conn = db.open_connection()?;
    let cli_key = workspaces::get_cli_key_by_id(&conn, workspace_id)?;
    let from_workspace_id = workspaces::active_id_by_cli(&conn, &cli_key)?;
    let context = WorkspaceRecoveryContext {
        schema_version: 1,
        cli_key: cli_key.clone(),
        from_workspace_id,
        to_workspace_id: workspace_id,
    };
    let serialized = serde_json::to_string(&context).map_err(|_| {
        AppError::new("RECOVERY_JOURNAL_INVALID", "无法序列化工作区恢复上下文")
    })?;
    operation.set_replay_context(&serialized)?;
    if CliKey::parse(&cli_key)?.supports(CliCapability::Skills) {
        let artifact_digest = skills::stage_local_skills_for_workspace_switch(
            app,
            &conn,
            &cli_key,
            from_workspace_id,
            operation,
        )?;
        operation.configure_replay(
            &serialized,
            Some(operation.operation_id()),
            Some(&artifact_digest),
        )?;
    }
    execute_workspace_projection(app, db, operation, &context)?;

    Ok(WorkspaceApplyReport {
        cli_key,
        from_workspace_id,
        to_workspace_id: workspace_id,
        applied_at: now_unix_seconds(),
    })
}

pub(crate) fn replay_recovery_operation<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &db::Db,
    operation: &RecoveryOperation,
) -> crate::shared::error::AppResult<()> {
    let context = context_from_entry(operation.entry())?;
    if CliKey::parse(&context.cli_key)?.supports(CliCapability::Skills)
        && operation.entry().artifact_ref.is_none()
    {
        if operation.entry().phase == "prepare" {
            return Ok(());
        }
        return Err(AppError::new(
            "RECOVERY_ARTIFACT_INVALID",
            "工作区切换缺少本地 Skills 恢复制品",
        ));
    }
    execute_workspace_projection(app, db, operation, &context)
}

pub(crate) fn cleanup_recovery_operation<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    entry: &JournalEntry,
) -> crate::shared::error::AppResult<()> {
    skills::cleanup_workspace_switch_local_skills_artifact(app, entry)
}

#[cfg(test)]
fn apply<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &db::Db,
    workspace_id: i64,
) -> crate::shared::error::AppResult<WorkspaceApplyReport> {
    crate::infra::recovery_journal::run_operation_for_test(
        app,
        db,
        "workspace.apply",
        crate::infra::recovery_journal::JournalContext::for_workspace(workspace_id),
        |operation| apply_with_recovery(app, db, workspace_id, operation),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;
    use std::ffi::OsString;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::MutexGuard;

    static TEST_SEQ: AtomicU64 = AtomicU64::new(1);

    #[derive(Default)]
    struct EnvRestore(Vec<(&'static str, Option<OsString>)>);

    impl EnvRestore {
        fn set(&mut self, key: &'static str, value: impl Into<OsString>) {
            if !self.0.iter().any(|(saved, _)| *saved == key) {
                self.0.push((key, std::env::var_os(key)));
            }
            std::env::set_var(key, value.into());
        }
    }

    impl Drop for EnvRestore {
        fn drop(&mut self) {
            for (key, value) in self.0.drain(..).rev() {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
            crate::test_support::clear_settings_cache();
        }
    }

    struct GrokWorkspaceTestApp {
        _lock: MutexGuard<'static, ()>,
        _env: EnvRestore,
        _home: tempfile::TempDir,
        _db_dir: tempfile::TempDir,
        app: tauri::App<tauri::test::MockRuntime>,
        db: db::Db,
        grok_home: std::path::PathBuf,
    }

    impl GrokWorkspaceTestApp {
        fn new() -> Self {
            let lock = crate::test_support::test_env_lock();
            let home = tempfile::tempdir().expect("home tempdir");
            let db_dir = tempfile::tempdir().expect("db tempdir");
            let grok_home = home.path().join("custom-grok");
            let mut env = EnvRestore::default();
            env.set(
                "AIO_CODING_HUB_HOME_DIR",
                home.path().as_os_str().to_os_string(),
            );
            env.set(
                "AIO_CODING_HUB_DOTDIR_NAME",
                format!(
                    ".aio-grok-workspace-test-{}",
                    TEST_SEQ.fetch_add(1, Ordering::Relaxed)
                ),
            );
            env.set("GROK_HOME", grok_home.as_os_str().to_os_string());
            crate::test_support::clear_settings_cache();
            let app = tauri::test::mock_app();
            let db =
                db::init_for_tests(&db_dir.path().join("workspace.sqlite")).expect("init test db");

            Self {
                _lock: lock,
                _env: env,
                _home: home,
                _db_dir: db_dir,
                app,
                db,
                grok_home,
            }
        }

        fn handle(&self) -> tauri::AppHandle<tauri::test::MockRuntime> {
            self.app.handle().clone()
        }

        fn default_workspace_id(&self) -> i64 {
            let list = workspaces::list_by_cli(&self.db, "grok").expect("list Grok workspaces");
            list.active_id.expect("default Grok workspace active")
        }

        fn target_workspace_id(&self) -> i64 {
            workspaces::create(&self.db, "grok", "Target", false)
                .expect("create target workspace")
                .id
        }

        fn set_prompt(&self, workspace_id: i64, content: &str) {
            self.db
                .open_connection()
                .expect("open db")
                .execute(
                    "UPDATE prompts SET content = ?1, enabled = 1 WHERE workspace_id = ?2",
                    params![content, workspace_id],
                )
                .expect("update target prompt");
        }

        fn add_mcp(&self, workspace_id: i64) {
            let conn = self.db.open_connection().expect("open db");
            conn.execute(
                r#"
INSERT INTO mcp_servers(
  server_key, name, normalized_name, transport, command, args_json, env_json,
  cwd, url, headers_json, created_at, updated_at
) VALUES ('managed', 'Managed', 'managed', 'stdio', 'npx', '["-y"]', '{}', NULL, NULL, '{}', 1, 1)
"#,
                [],
            )
            .expect("insert MCP server");
            let server_id = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO workspace_mcp_enabled(workspace_id, server_id, created_at, updated_at) VALUES (?1, ?2, 1, 1)",
                params![workspace_id, server_id],
            )
            .expect("enable MCP server");
        }

        fn add_skill(&self, workspace_id: i64, create_ssot: bool) {
            let conn = self.db.open_connection().expect("open db");
            conn.execute(
                r#"
INSERT INTO skills(
  skill_key, name, normalized_name, description, source_git_url, source_branch,
  source_subdir, installed_commit, installed_content_hash, created_at, updated_at
) VALUES ('demo', 'Demo', 'demo', '', 'https://example.test/skills.git', 'main', 'demo', NULL, NULL, 1, 1)
"#,
                [],
            )
            .expect("insert skill");
            let skill_id = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO workspace_skill_enabled(workspace_id, skill_id, created_at, updated_at) VALUES (?1, ?2, 1, 1)",
                params![workspace_id, skill_id],
            )
            .expect("enable skill");

            if create_ssot {
                let paths =
                    crate::skills::paths_get(&self.handle(), "grok").expect("resolve skill paths");
                let skill_dir = std::path::PathBuf::from(paths.ssot_dir).join("demo");
                std::fs::create_dir_all(&skill_dir).expect("create SSOT skill");
                std::fs::write(skill_dir.join("SKILL.md"), "---\nname: Demo\n---\n")
                    .expect("write SKILL.md");
            }
        }

        fn write_config(&self, bytes: &[u8]) {
            std::fs::create_dir_all(&self.grok_home).expect("create Grok home");
            std::fs::write(self.grok_home.join("config.toml"), bytes).expect("write config");
        }

        fn write_prompt(&self, content: &str) {
            std::fs::create_dir_all(&self.grok_home).expect("create Grok home");
            std::fs::write(self.grok_home.join("AGENTS.md"), content).expect("write prompt");
        }

        fn assert_active(&self, workspace_id: i64) {
            let conn = self.db.open_connection().expect("open db");
            assert_eq!(
                workspaces::active_id_by_cli(&conn, "grok").expect("active workspace"),
                Some(workspace_id)
            );
        }

        fn assert_pending_workspace_journal(&self, expected_phase: &str) {
            let conn = self.db.open_connection().expect("open db");
            let (status, phase): (String, String) = conn
                .query_row(
                    r#"
SELECT status, phase
FROM external_effect_recovery_journal
WHERE operation_kind = 'workspace.apply' AND status != 'resolved'
ORDER BY created_at DESC, operation_id DESC
LIMIT 1
"#,
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .expect("pending workspace journal");
            assert_eq!(status, "failed");
            assert_eq!(phase, expected_phase);
        }
    }

    const INITIAL_CONFIG: &str = r#"# keep
[model.aio]
model = "grok-build"
base_url = "http://127.0.0.1:37123/grok/v1"

[mcp_servers.local]
command = "local"
"#;

    #[test]
    fn grok_workspace_round_trip_applies_prompt_mcp_skills_and_local_stash() {
        let test = GrokWorkspaceTestApp::new();
        let default_id = test.default_workspace_id();
        let target_id = test.target_workspace_id();
        test.set_prompt(target_id, "target instructions");
        test.add_mcp(target_id);
        test.add_skill(target_id, true);
        test.write_config(INITIAL_CONFIG.as_bytes());
        test.write_prompt("original instructions");

        let report = apply(&test.handle(), &test.db, target_id).expect("apply target workspace");

        assert_eq!(report.cli_key, "grok");
        test.assert_active(target_id);
        assert_eq!(
            std::fs::read_to_string(test.grok_home.join("AGENTS.md")).expect("read prompt"),
            "target instructions\n"
        );
        let config =
            std::fs::read_to_string(test.grok_home.join("config.toml")).expect("read config");
        let document = config
            .parse::<toml_edit::DocumentMut>()
            .expect("valid Grok TOML");
        assert_eq!(
            document["model"]["aio"]["model"].as_str(),
            Some("grok-build")
        );
        assert_eq!(
            document["mcp_servers"]["managed"]["command"].as_str(),
            Some("npx")
        );
        assert!(document["mcp_servers"].get("local").is_none());
        assert!(test.grok_home.join("skills").join("demo").exists());

        apply(&test.handle(), &test.db, default_id).expect("restore default workspace");

        test.assert_active(default_id);
        let restored = std::fs::read_to_string(test.grok_home.join("config.toml"))
            .expect("read restored config");
        let restored = restored
            .parse::<toml_edit::DocumentMut>()
            .expect("valid restored TOML");
        assert_eq!(
            restored["mcp_servers"]["local"]["command"].as_str(),
            Some("local")
        );
        assert!(restored["mcp_servers"].get("managed").is_none());
        assert_eq!(
            restored["model"]["aio"]["model"].as_str(),
            Some("grok-build")
        );
        assert!(!test.grok_home.join("skills").join("demo").exists());
    }

    #[test]
    fn grok_workspace_prompt_failure_keeps_sqlite_target_and_replays_after_repair() {
        let test = GrokWorkspaceTestApp::new();
        let default_id = test.default_workspace_id();
        let target_id = test.target_workspace_id();
        test.set_prompt(target_id, &"x".repeat(1024 * 1024 + 1));
        test.write_config(INITIAL_CONFIG.as_bytes());
        test.write_prompt("original instructions");

        let error =
            apply(&test.handle(), &test.db, target_id).expect_err("oversized prompt must fail");

        assert!(error.to_string().contains("too large"));
        test.assert_active(target_id);
        assert_eq!(
            std::fs::read(test.grok_home.join("config.toml")).expect("read config"),
            INITIAL_CONFIG.as_bytes()
        );
        assert_eq!(
            std::fs::read_to_string(test.grok_home.join("AGENTS.md")).expect("read prompt"),
            "original instructions"
        );
        assert!(prompt_sync::read_manifest_bytes(&test.handle(), "grok")
            .expect("read prompt manifest")
            .is_none());
        assert!(crate::infra::recovery_journal::has_pending(&test.db).expect("pending journal"));
        test.assert_pending_workspace_journal(PHASE_ACTIVE);

        test.set_prompt(target_id, "repaired instructions");
        crate::infra::recovery_journal::replay_pending(&test.handle(), &test.db)
            .expect("replay repaired prompt projection");
        test.assert_active(target_id);
        assert_eq!(
            std::fs::read_to_string(test.grok_home.join("AGENTS.md")).expect("read prompt"),
            "repaired instructions\n"
        );
        assert!(!crate::infra::recovery_journal::has_pending(&test.db).expect("resolved journal"));
        let _ = default_id;
    }

    #[test]
    fn grok_workspace_mcp_failure_keeps_checkpoint_and_replays_after_repair() {
        let test = GrokWorkspaceTestApp::new();
        let default_id = test.default_workspace_id();
        let target_id = test.target_workspace_id();
        test.set_prompt(target_id, "target instructions");
        test.add_mcp(target_id);
        let invalid = b"[mcp_servers\ninvalid = true\n";
        test.write_config(invalid);
        test.write_prompt("original instructions");

        let error =
            apply(&test.handle(), &test.db, target_id).expect_err("invalid Grok TOML must fail");

        assert!(error.to_string().contains("GROK_CONFIG_INVALID_TOML"));
        test.assert_active(target_id);
        assert_eq!(
            std::fs::read(test.grok_home.join("config.toml")).expect("read config"),
            invalid
        );
        assert_eq!(
            std::fs::read_to_string(test.grok_home.join("AGENTS.md")).expect("read prompt"),
            "target instructions\n"
        );
        assert!(prompt_sync::read_manifest_bytes(&test.handle(), "grok")
            .expect("read prompt manifest")
            .is_some());
        assert!(mcp_sync::read_manifest_bytes(&test.handle(), "grok")
            .expect("read MCP manifest")
            .is_none());
        assert!(crate::infra::recovery_journal::has_pending(&test.db).expect("pending journal"));
        test.assert_pending_workspace_journal(PHASE_PROMPT);

        test.write_config(INITIAL_CONFIG.as_bytes());
        crate::infra::recovery_journal::replay_pending(&test.handle(), &test.db)
            .expect("replay repaired MCP projection");
        test.assert_active(target_id);
        let config = std::fs::read_to_string(test.grok_home.join("config.toml"))
            .expect("read repaired config")
            .parse::<toml_edit::DocumentMut>()
            .expect("valid repaired TOML");
        assert_eq!(config["mcp_servers"]["managed"]["command"].as_str(), Some("npx"));
        assert!(!crate::infra::recovery_journal::has_pending(&test.db).expect("resolved journal"));
        let _ = default_id;
    }

    #[test]
    fn grok_workspace_skills_failure_keeps_target_and_replays_after_ssot_repair() {
        let test = GrokWorkspaceTestApp::new();
        let default_id = test.default_workspace_id();
        let target_id = test.target_workspace_id();
        test.set_prompt(target_id, "target instructions");
        test.add_mcp(target_id);
        test.add_skill(target_id, false);
        test.write_config(INITIAL_CONFIG.as_bytes());
        test.write_prompt("original instructions");

        let error =
            apply(&test.handle(), &test.db, target_id).expect_err("missing SSOT skill must fail");

        assert!(error.to_string().contains("SKILL_SSOT_MISSING"));
        test.assert_active(target_id);
        assert_eq!(
            std::fs::read_to_string(test.grok_home.join("AGENTS.md")).expect("read prompt"),
            "target instructions\n"
        );
        assert!(prompt_sync::read_manifest_bytes(&test.handle(), "grok")
            .expect("read prompt manifest")
            .is_some());
        assert!(mcp_sync::read_manifest_bytes(&test.handle(), "grok")
            .expect("read MCP manifest")
            .is_some());
        assert!(crate::infra::recovery_journal::has_pending(&test.db).expect("pending journal"));
        test.assert_pending_workspace_journal(PHASE_CLAUDE_RESTORE);

        let paths = crate::skills::paths_get(&test.handle(), "grok").expect("resolve skill paths");
        let skill_dir = std::path::PathBuf::from(paths.ssot_dir).join("demo");
        std::fs::create_dir_all(&skill_dir).expect("create repaired SSOT skill");
        std::fs::write(skill_dir.join("SKILL.md"), "---\nname: Demo\n---\n")
            .expect("write repaired SKILL.md");
        crate::infra::recovery_journal::replay_pending(&test.handle(), &test.db)
            .expect("replay repaired Skills projection");
        test.assert_active(target_id);
        assert!(test.grok_home.join("skills").join("demo").exists());
        assert!(!crate::infra::recovery_journal::has_pending(&test.db).expect("resolved journal"));
        let _ = default_id;
    }
}
