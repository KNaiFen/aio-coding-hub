use super::fs_ops::{
    is_managed_dir, is_managed_link_to_ssot, is_symlink, is_symlink_or_junction,
    read_source_metadata, skill_md_path, SkillSourceMetadata,
};
use super::installed::{get_skill_by_id, skill_key_exists};
use super::npx_lock::NpxSkillLock;
use super::paths::{cli_skills_root, ensure_skills_roots, ssot_skills_root, validate_cli_key};
use super::recovery::{stage_artifact, SkillRecoveryContext};
use super::repo_cache::ensure_repo_cache;
use super::skill_md::parse_skill_md;
use super::types::{InstalledSkillSummary, LocalSkillSummary};
use super::util::{validate_dir_name, validate_relative_subdir};
use crate::db;
use crate::infra::recovery_journal::RecoveryOperation;
use crate::shared::error::db_err;
use crate::shared::text::normalize_name;
use crate::shared::time::now_unix_seconds;
use crate::workspaces;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

fn summarize_local_skill_dir(
    path: &Path,
    ssot_root: &Path,
    npx_lock: Option<&NpxSkillLock>,
) -> crate::shared::error::AppResult<Option<LocalSkillSummary>> {
    if !path.is_dir() && !is_symlink_or_junction(path) {
        return Ok(None);
    }
    if is_managed_link_to_ssot(path, ssot_root) {
        return Ok(None);
    }

    let dir_name = path
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_string();
    if dir_name.is_empty() {
        return Ok(None);
    }

    let Some(skill_md) = skill_md_path(path)? else {
        return Ok(None);
    };

    let (name, description) = match parse_skill_md(&skill_md) {
        Ok((name, description)) => (name, description),
        Err(_) => (dir_name.clone(), String::new()),
    };
    let source = read_source_metadata(path)?
        .or_else(|| npx_lock.and_then(|lock| lock.source_for_local_skill(&dir_name, &name)));

    Ok(Some(LocalSkillSummary {
        dir_name,
        path: path.to_string_lossy().to_string(),
        name,
        description,
        source_git_url: source.as_ref().map(|item| item.source_git_url.clone()),
        source_branch: source.as_ref().map(|item| item.source_branch.clone()),
        source_subdir: source.as_ref().map(|item| item.source_subdir.clone()),
    }))
}

pub(super) fn managed_marker_belongs_to_installed_skill(
    conn: &Connection,
    path: &Path,
) -> crate::shared::error::AppResult<bool> {
    if !is_managed_dir(path) {
        return Ok(false);
    }

    let dir_name = path.file_name().and_then(|v| v.to_str()).unwrap_or("");
    if dir_name.is_empty() {
        return Ok(true);
    }

    skill_key_exists(conn, dir_name)
}

fn installed_skill_id_by_source(
    conn: &Connection,
    source: &SkillSourceMetadata,
) -> crate::shared::error::AppResult<Option<i64>> {
    conn.query_row(
        r#"
SELECT id
FROM skills
WHERE source_git_url = ?1 AND source_branch = ?2 AND source_subdir = ?3
LIMIT 1
"#,
        params![
            source.source_git_url.trim(),
            source.source_branch.trim(),
            source.source_subdir.trim()
        ],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| db_err!("failed to query skill by source: {e}"))
}

fn suggested_local_dir_name(source_subdir: &str, skill_name: &str) -> String {
    Path::new(source_subdir)
        .file_name()
        .and_then(|v| v.to_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .or_else(|| {
            let trimmed = skill_name.trim();
            (!trimmed.is_empty()).then_some(trimmed)
        })
        .unwrap_or("skill")
        .to_string()
}

fn next_available_local_dir_name(
    root: &Path,
    preferred: &str,
) -> crate::shared::error::AppResult<String> {
    let base = validate_dir_name(preferred)?;

    let mut candidate = base.clone();
    let mut idx = 2;
    while root.join(&candidate).exists() && idx < 1000 {
        candidate = format!("{base}-{idx}");
        idx += 1;
    }

    if root.join(&candidate).exists() {
        return Ok(validate_dir_name(&format!("{base}-{}", now_unix_seconds()))?);
    }

    Ok(validate_dir_name(&candidate)?)
}

fn find_local_skill_by_source(
    root: &Path,
    ssot_root: &Path,
    source: &SkillSourceMetadata,
) -> crate::shared::error::AppResult<Option<LocalSkillSummary>> {
    if !root.exists() {
        return Ok(None);
    }

    let entries = std::fs::read_dir(root)
        .map_err(|e| format!("failed to read dir {}: {e}", root.display()))?;
    for entry in entries {
        let entry =
            entry.map_err(|e| format!("failed to read dir entry {}: {e}", root.display()))?;
        let path = entry.path();
        let Some(summary) = summarize_local_skill_dir(&path, ssot_root, None)? else {
            continue;
        };

        if summary.source_git_url.as_deref() == Some(source.source_git_url.as_str())
            && summary.source_branch.as_deref() == Some(source.source_branch.as_str())
            && summary.source_subdir.as_deref() == Some(source.source_subdir.as_str())
        {
            return Ok(Some(summary));
        }
    }

    Ok(None)
}

pub fn local_list<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &db::Db,
    workspace_id: i64,
) -> crate::shared::error::AppResult<Vec<LocalSkillSummary>> {
    let conn = db.open_connection()?;
    let cli_key = workspaces::get_cli_key_by_id(&conn, workspace_id)?;
    validate_cli_key(&cli_key)?;

    if !workspaces::is_active_workspace(&conn, workspace_id)? {
        return Err(
            "SKILL_LOCAL_REQUIRES_ACTIVE_WORKSPACE: local skills only available for active workspace"
                .to_string()
                .into(),
        );
    }

    let root = cli_skills_root(app, &cli_key)?;
    if !root.exists() {
        return Ok(Vec::new());
    }
    let ssot_root = ssot_skills_root(app)?;
    let npx_lock = NpxSkillLock::read(app);

    let entries = std::fs::read_dir(&root)
        .map_err(|e| format!("failed to read dir {}: {e}", root.display()))?;

    let mut out = Vec::new();
    for entry in entries {
        let entry =
            entry.map_err(|e| format!("failed to read dir entry {}: {e}", root.display()))?;
        let path = entry.path();
        if managed_marker_belongs_to_installed_skill(&conn, &path)? {
            continue;
        }
        let Some(summary) = summarize_local_skill_dir(&path, &ssot_root, Some(&npx_lock))? else {
            continue;
        };
        out.push(summary);
    }

    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

pub fn install_to_local<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &db::Db,
    workspace_id: i64,
    git_url: &str,
    branch: &str,
    source_subdir: &str,
    operation: &RecoveryOperation,
) -> crate::shared::error::AppResult<LocalSkillSummary> {
    ensure_skills_roots(app)?;
    validate_relative_subdir(source_subdir)?;

    let conn = db.open_connection()?;
    let cli_key = workspaces::get_cli_key_by_id(&conn, workspace_id)?;
    validate_cli_key(&cli_key)?;
    if !workspaces::is_active_workspace(&conn, workspace_id)? {
        return Err(
            "SKILL_LOCAL_INSTALL_REQUIRES_ACTIVE_WORKSPACE: switch to the target workspace before installing to local"
                .to_string()
                .into(),
        );
    }

    let source = SkillSourceMetadata {
        source_git_url: git_url.trim().to_string(),
        source_branch: branch.trim().to_string(),
        source_subdir: source_subdir.trim().to_string(),
    };

    if installed_skill_id_by_source(&conn, &source)?.is_some() {
        return Err(
            "SKILL_ALREADY_INSTALLED: skill already exists in generic skills"
                .to_string()
                .into(),
        );
    }

    let cli_root = cli_skills_root(app, &cli_key)?;
    let ssot_root = ssot_skills_root(app)?;
    std::fs::create_dir_all(&cli_root)
        .map_err(|e| format!("failed to create {}: {e}", cli_root.display()))?;

    if let Some(existing) = find_local_skill_by_source(&cli_root, &ssot_root, &source)? {
        return Ok(existing);
    }

    let repo_dir = ensure_repo_cache(app, &source.source_git_url, &source.source_branch, false)?;
    let src_dir = repo_dir.join(source.source_subdir.trim());
    if !src_dir.exists() {
        return Err(format!("SKILL_SOURCE_NOT_FOUND: {}", src_dir.display()).into());
    }
    if !src_dir.is_dir() {
        return Err("SEC_INVALID_INPUT: source_subdir is not a directory"
            .to_string()
            .into());
    }

    let Some(skill_md) = skill_md_path(&src_dir)? else {
        return Err("SEC_INVALID_INPUT: SKILL.md not found in source_subdir"
            .to_string()
            .into());
    };

    let (name, description) = match parse_skill_md(&skill_md) {
        Ok(v) => v,
        Err(_) => {
            return Err(
                "SEC_INVALID_INPUT: failed to parse SKILL.md in source_subdir"
                    .to_string()
                    .into(),
            )
        }
    };

    let dir_name = next_available_local_dir_name(
        &cli_root,
        &suggested_local_dir_name(&source.source_subdir, &name),
    )?;
    let context = SkillRecoveryContext::InstallToLocal {
        workspace_id,
        cli_key: cli_key.clone(),
        dir_name: dir_name.clone(),
    };
    let _artifact = stage_artifact(
        app,
        operation,
        &context,
        &[("desired", src_dir.as_path())],
        Some(&source),
    )?;

    Ok(LocalSkillSummary {
        dir_name: dir_name.clone(),
        path: cli_root.join(&dir_name).to_string_lossy().to_string(),
        name,
        description,
        source_git_url: Some(source.source_git_url),
        source_branch: Some(source.source_branch),
        source_subdir: Some(source.source_subdir),
    })
}

pub fn delete_local<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &db::Db,
    workspace_id: i64,
    dir_name: &str,
    operation: &RecoveryOperation,
) -> crate::shared::error::AppResult<()> {
    let dir_name = validate_dir_name(dir_name)?;

    let conn = db.open_connection()?;
    let cli_key = workspaces::get_cli_key_by_id(&conn, workspace_id)?;
    validate_cli_key(&cli_key)?;

    if !workspaces::is_active_workspace(&conn, workspace_id)? {
        return Err(
            "SKILL_LOCAL_DELETE_REQUIRES_ACTIVE_WORKSPACE: switch to the target workspace before deleting local skills"
                .to_string()
                .into(),
        );
    }

    let root = cli_skills_root(app, &cli_key)?;
    let local_dir = root.join(&dir_name);
    if !local_dir.exists() {
        return Err(format!("SKILL_LOCAL_NOT_FOUND: {}", local_dir.display()).into());
    }
    if is_symlink(&local_dir)? {
        return Err(format!(
            "SKILL_LOCAL_DELETE_BLOCKED_SYMLINK: {}",
            local_dir.display()
        )
        .into());
    }
    if !local_dir.is_dir() {
        return Err("SEC_INVALID_INPUT: local skill path is not a directory"
            .to_string()
            .into());
    }
    if managed_marker_belongs_to_installed_skill(&conn, &local_dir)? {
        return Err(format!(
            "SKILL_LOCAL_DELETE_BLOCKED_MANAGED: {}",
            local_dir.display()
        )
        .into());
    }

    if skill_md_path(&local_dir)?.is_none() {
        return Err("SEC_INVALID_INPUT: SKILL.md not found in local skill dir"
            .to_string()
            .into());
    }

    let source_metadata = read_source_metadata(&local_dir)?;
    let context = SkillRecoveryContext::DeleteLocal {
        workspace_id,
        cli_key,
        dir_name,
    };
    let _artifact = stage_artifact(
        app,
        operation,
        &context,
        &[("previous", local_dir.as_path())],
        source_metadata.as_ref(),
    )?;
    Ok(())
}

pub fn import_local<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &db::Db,
    workspace_id: i64,
    dir_name: &str,
    operation: &RecoveryOperation,
) -> crate::shared::error::AppResult<InstalledSkillSummary> {
    ensure_skills_roots(app)?;

    let dir_name = validate_dir_name(dir_name)?;

    let mut conn = db.open_connection()?;
    let cli_key = workspaces::get_cli_key_by_id(&conn, workspace_id)?;
    validate_cli_key(&cli_key)?;
    if !workspaces::is_active_workspace(&conn, workspace_id)? {
        return Err(
            "SKILL_IMPORT_LOCAL_REQUIRES_ACTIVE_WORKSPACE: switch to the target workspace before importing"
                .to_string()
                .into(),
        );
    }

    let cli_root = cli_skills_root(app, &cli_key)?;
    let local_dir = cli_root.join(&dir_name);
    if !local_dir.exists() {
        return Err(format!("SKILL_LOCAL_NOT_FOUND: {}", local_dir.display()).into());
    }
    if is_symlink(&local_dir)? || is_symlink_or_junction(&local_dir) || !local_dir.is_dir() {
        return Err("SEC_INVALID_INPUT: local skill path is not a directory"
            .to_string()
            .into());
    }
    let skill_key_already_exists = skill_key_exists(&conn, &dir_name)?;
    if is_managed_dir(&local_dir) && skill_key_already_exists {
        return Err(
            "SKILL_ALREADY_MANAGED: skill already managed by aio-coding-hub"
                .to_string()
                .into(),
        );
    }

    let Some(skill_md) = skill_md_path(&local_dir)? else {
        return Err("SEC_INVALID_INPUT: SKILL.md not found in local skill dir"
            .to_string()
            .into());
    };

    let (name, description) = match parse_skill_md(&skill_md) {
        Ok(v) => v,
        Err(_) => (dir_name.clone(), String::new()),
    };
    let normalized_name = normalize_name(&name);
    let npx_lock = NpxSkillLock::read(app);
    let source_meta = read_source_metadata(&local_dir)?
        .or_else(|| npx_lock.source_for_local_skill(&dir_name, &name));

    if let Some(source) = source_meta.as_ref() {
        if installed_skill_id_by_source(&conn, source)?.is_some() {
            return Err("SKILL_IMPORT_CONFLICT: same source already exists"
                .to_string()
                .into());
        }
    }

    if skill_key_already_exists {
        return Err("SKILL_IMPORT_CONFLICT: same skill_key already exists"
            .to_string()
            .into());
    }

    let now = now_unix_seconds();
    let ssot_dir = ssot_skills_root(app)?.join(&dir_name);
    if ssot_dir.exists() {
        return Err("SKILL_IMPORT_CONFLICT: ssot dir already exists"
            .to_string()
            .into());
    }
    let context = SkillRecoveryContext::ImportLocal {
        workspace_id,
        cli_key: cli_key.clone(),
        skill_key: dir_name.clone(),
        local_dir_name: dir_name.clone(),
    };
    let artifact = stage_artifact(
        app,
        operation,
        &context,
        &[("desired", local_dir.as_path())],
        source_meta.as_ref(),
    )?;
    let installed_content_hash = artifact.role_hash("desired")?.to_string();

    let tx = conn
        .transaction()
        .map_err(|e| db_err!("failed to start transaction: {e}"))?;

    tx.execute(
        r#"
INSERT INTO skills(
  skill_key,
  name,
  normalized_name,
  description,
  source_git_url,
  source_branch,
  source_subdir,
  installed_content_hash,
  created_at,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
"#,
        params![
            dir_name,
            name.trim(),
            normalized_name,
            description,
            source_meta
                .as_ref()
                .map(|item| item.source_git_url.clone())
                .unwrap_or_else(|| format!("local://{cli_key}")),
            source_meta
                .as_ref()
                .map(|item| item.source_branch.clone())
                .unwrap_or_else(|| "local".to_string()),
            source_meta
                .as_ref()
                .map(|item| item.source_subdir.clone())
                .unwrap_or_else(|| dir_name.clone()),
            installed_content_hash,
            now,
            now
        ],
    )
    .map_err(|e| db_err!("failed to insert imported skill: {e}"))?;

    let skill_id = tx.last_insert_rowid();

    tx.execute(
        r#"
INSERT INTO workspace_skill_enabled(workspace_id, skill_id, created_at, updated_at)
VALUES (?1, ?2, ?3, ?3)
ON CONFLICT(workspace_id, skill_id) DO UPDATE SET
  updated_at = excluded.updated_at
"#,
        params![workspace_id, skill_id, now],
    )
    .map_err(|e| db_err!("failed to enable imported skill for workspace: {e}"))?;

    tx.commit().map_err(|err| db_err!("failed to commit: {err}"))?;

    operation.mark_authoritative_committed();
    get_skill_by_id(&conn, skill_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_directory_allocation_rejects_skill_name_path_escape() {
        let root = tempfile::tempdir().expect("tempdir");
        for value in ["../escape", "nested/name", "nested\\name"] {
            assert!(next_available_local_dir_name(root.path(), value).is_err());
        }
        assert_eq!(
            next_available_local_dir_name(root.path(), "safe-skill").unwrap(),
            "safe-skill"
        );
    }
}
