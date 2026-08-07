//! Usage: Skill update detection and execution.

use super::fs_ops::skill_dir_content_hash;
use super::git_url::{normalize_repo_branch, parse_github_owner_repo};
use super::installed::{get_skill_by_id_for_workspace, installed_list_for_workspace};
use super::ops::ensure_installed_content_hash;
use super::paths::ssot_skills_root;
use super::recovery::{stage_artifact, SkillRecoveryContext};
use super::repo_cache::{ensure_repo_cache, get_repo_head_commit, github_get_branch_commit};
use super::skill_md::parse_skill_md;
use super::types::{InstalledSkillSummary, SkillUpdateInfo};
use super::util::validate_relative_subdir;
use crate::db;
use crate::infra::recovery_journal::RecoveryOperation;
use crate::shared::error::db_err;
use crate::shared::text::normalize_name;
use crate::shared::time::now_unix_seconds;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

static SKILL_UPDATE_LOCKS: OnceLock<Mutex<HashSet<i64>>> = OnceLock::new();

#[derive(Debug)]
struct SkillUpdateGuard {
    skill_id: i64,
}

impl Drop for SkillUpdateGuard {
    fn drop(&mut self) {
        if let Some(locks) = SKILL_UPDATE_LOCKS.get() {
            if let Ok(mut locked) = locks.lock() {
                locked.remove(&self.skill_id);
            }
        }
    }
}

fn acquire_skill_update_lock(skill_id: i64) -> crate::shared::error::AppResult<SkillUpdateGuard> {
    let locks = SKILL_UPDATE_LOCKS.get_or_init(|| Mutex::new(HashSet::new()));
    let mut locked = locks
        .lock()
        .map_err(|_| "SKILL_UPDATE_LOCK_POISONED: failed to acquire update lock")?;
    if !locked.insert(skill_id) {
        return Err("SKILL_UPDATE_IN_PROGRESS: skill is already being updated".into());
    }
    Ok(SkillUpdateGuard { skill_id })
}

/// Local-only skills (local://) are not eligible for remote update checks.
fn is_updatable_skill_source(source_git_url: &str) -> bool {
    let url = source_git_url.trim().to_lowercase();
    !url.is_empty() && !url.starts_with("local://")
}

/// Get the latest commit for a skill from its source repository.
/// For GitHub repos, uses the API to get the branch commit.
/// For git repos, refreshes the cache and reads HEAD.
fn get_latest_commit_for_skill(
    app: &tauri::AppHandle<impl tauri::Runtime>,
    git_url: &str,
    branch: &str,
) -> crate::shared::error::AppResult<String> {
    let normalized_branch = normalize_repo_branch(branch);

    // For GitHub repos, try using the API first (works for snapshot mode).
    if let Some((owner, repo)) = parse_github_owner_repo(git_url) {
        // Determine the effective branch. If "auto", try common defaults.
        let effective_branch = if normalized_branch == "auto" {
            // Try main first, then master. If both fail, return error.
            match github_get_branch_commit(&owner, &repo, "main") {
                Ok(commit) => return Ok(commit),
                Err(_) => match github_get_branch_commit(&owner, &repo, "master") {
                    Ok(commit) => return Ok(commit),
                    Err(e) => return Err(e),
                },
            }
        } else {
            normalized_branch.clone()
        };

        return github_get_branch_commit(&owner, &repo, &effective_branch);
    }

    // For non-GitHub repos, refresh the cache and read HEAD.
    let repo_dir = ensure_repo_cache(app, git_url, &normalized_branch, true)?;
    get_repo_head_commit(&repo_dir)
}

fn get_latest_content_hash_for_skill(
    app: &tauri::AppHandle<impl tauri::Runtime>,
    git_url: &str,
    branch: &str,
    source_subdir: &str,
) -> crate::shared::error::AppResult<String> {
    validate_relative_subdir(source_subdir)?;
    let normalized_branch = normalize_repo_branch(branch);
    let repo_dir = ensure_repo_cache(app, git_url, &normalized_branch, true)?;
    let src_dir = repo_dir.join(source_subdir.trim());
    if !src_dir.is_dir() {
        return Err(format!("SKILL_SOURCE_NOT_FOUND: {}", src_dir.display()).into());
    }
    skill_dir_content_hash(&src_dir)
}

fn installed_content_hash_for_skill(
    db: &db::Db,
    skill_id: i64,
) -> crate::shared::error::AppResult<Option<String>> {
    let conn = db.open_connection()?;
    installed_content_hash_for_conn(&conn, skill_id)
}

fn installed_content_hash_for_conn(
    conn: &Connection,
    skill_id: i64,
) -> crate::shared::error::AppResult<Option<String>> {
    conn.query_row(
        "SELECT installed_content_hash FROM skills WHERE id = ?1",
        params![skill_id],
        |row| row.get(0),
    )
    .optional()
    .map(|value| value.flatten())
    .map_err(|e| db_err!("failed to query installed_content_hash: {e}"))
}

fn set_installed_content_hash(
    db: &db::Db,
    skill_id: i64,
    hash: &str,
) -> crate::shared::error::AppResult<()> {
    let conn = db.open_connection()?;
    conn.execute(
        "UPDATE skills SET installed_content_hash = ?1 WHERE id = ?2",
        params![hash, skill_id],
    )
    .map_err(|e| db_err!("failed to update installed_content_hash: {e}"))?;
    Ok(())
}

fn get_or_backfill_installed_content_hash(
    app: &tauri::AppHandle<impl tauri::Runtime>,
    db: &db::Db,
    skill: &InstalledSkillSummary,
) -> Option<String> {
    if let Ok(Some(hash)) = installed_content_hash_for_skill(db, skill.id) {
        return Some(hash);
    }

    let ssot_dir = ssot_skills_root(app).ok()?.join(&skill.skill_key);
    let hash = skill_dir_content_hash(&ssot_dir).ok()?;
    let _ = set_installed_content_hash(db, skill.id, &hash);
    Some(hash)
}

/// Check for updates for all remotely sourced skills in a workspace.
pub fn check_updates_for_workspace(
    app: &tauri::AppHandle<impl tauri::Runtime>,
    db: &db::Db,
    workspace_id: i64,
) -> crate::shared::error::AppResult<Vec<SkillUpdateInfo>> {
    use std::collections::HashMap;

    let skills = installed_list_for_workspace(db, workspace_id)?;
    let mut results = Vec::new();

    // Cache latest commits by (git_url, branch) to avoid redundant API calls
    // when multiple skills share the same source repository.
    let mut commit_cache: HashMap<(String, String), Option<String>> = HashMap::new();
    let mut content_hash_cache: HashMap<(String, String, String), Option<String>> = HashMap::new();

    for skill in skills {
        if !is_updatable_skill_source(&skill.source_git_url) {
            continue;
        }

        let content_cache_key = (
            skill.source_git_url.clone(),
            skill.source_branch.clone(),
            skill.source_subdir.clone(),
        );
        let latest_content_hash = content_hash_cache
            .entry(content_cache_key)
            .or_insert_with(|| {
                get_latest_content_hash_for_skill(
                    app,
                    &skill.source_git_url,
                    &skill.source_branch,
                    &skill.source_subdir,
                )
                .ok()
            })
            .clone();

        let installed_content_hash = get_or_backfill_installed_content_hash(app, db, &skill);

        let cache_key = (skill.source_git_url.clone(), skill.source_branch.clone());
        let latest_commit = commit_cache
            .entry(cache_key)
            .or_insert_with(|| {
                get_latest_commit_for_skill(app, &skill.source_git_url, &skill.source_branch).ok()
            })
            .clone();

        let installed_commit = skill.installed_commit.clone();
        let has_update = match (&installed_content_hash, &latest_content_hash) {
            (Some(installed), Some(latest)) => installed != latest,
            _ => match (&installed_commit, &latest_commit) {
                (Some(installed), Some(latest)) => installed != latest,
                _ => false,
            },
        };

        results.push(SkillUpdateInfo {
            skill_id: skill.id,
            has_update,
            installed_commit,
            latest_commit,
        });
    }

    Ok(results)
}

/// Stage the desired and previous SSOT contents, then commit SQLite metadata.
/// The journal projects the committed state after this function returns.
pub fn update_skill(
    app: &tauri::AppHandle<impl tauri::Runtime>,
    db: &db::Db,
    workspace_id: i64,
    skill_id: i64,
    operation: &RecoveryOperation,
) -> crate::shared::error::AppResult<InstalledSkillSummary> {
    let mut conn = db.open_connection()?;
    let _guard = acquire_skill_update_lock(skill_id)?;
    let skill = get_skill_by_id_for_workspace(&conn, workspace_id, skill_id)?;

    // Local-only imports do not have a remote source to refresh from.
    if !is_updatable_skill_source(&skill.source_git_url) {
        return Err("SKILL_UPDATE_NOT_SUPPORTED: local skills cannot be updated".into());
    }
    validate_relative_subdir(&skill.source_subdir)?;

    let normalized_branch = normalize_repo_branch(&skill.source_branch);
    let repo_dir = ensure_repo_cache(app, &skill.source_git_url, &normalized_branch, true)?;
    let src_dir = repo_dir.join(skill.source_subdir.trim());
    if !src_dir.is_dir() {
        return Err(format!("SKILL_SOURCE_NOT_FOUND: {}", src_dir.display()).into());
    }
    let skill_md = src_dir.join("SKILL.md");
    if !skill_md.exists() {
        return Err("SEC_INVALID_INPUT: SKILL.md not found in source_subdir"
            .to_string()
            .into());
    }

    let (name, description) = parse_skill_md(&skill_md)?;
    let normalized_name = normalize_name(&name);
    let installed_commit = get_repo_head_commit(&repo_dir).ok().or_else(|| {
        get_latest_commit_for_skill(app, &skill.source_git_url, &skill.source_branch).ok()
    });

    let ssot_dir = ssot_skills_root(app)?.join(&skill.skill_key);
    if !ssot_dir.is_dir() {
        return Err(format!("SKILL_SSOT_MISSING: {}", skill.skill_key).into());
    }
    ensure_installed_content_hash(&conn, skill_id, &ssot_dir)?;
    let context = SkillRecoveryContext::Update {
        workspace_id,
        skill_id,
        skill_key: skill.skill_key.clone(),
    };
    let artifact = stage_artifact(
        app,
        operation,
        &context,
        &[
            ("desired", src_dir.as_path()),
            ("previous", ssot_dir.as_path()),
        ],
        None,
    )?;
    let installed_content_hash = artifact.role_hash("desired")?.to_string();

    let now = now_unix_seconds();
    let tx = conn
        .transaction()
        .map_err(|e| db_err!("failed to start transaction: {e}"))?;
    let updated_rows = tx
        .execute(
            r#"
UPDATE skills
SET
  name = ?1,
  normalized_name = ?2,
  description = ?3,
  installed_commit = ?4,
  installed_content_hash = ?5,
  updated_at = ?6
WHERE id = ?7
"#,
            params![
                name.trim(),
                normalized_name,
                description,
                installed_commit,
                installed_content_hash,
                now,
                skill_id
            ],
        )
        .map_err(|err| db_err!("failed to update skill metadata: {err}"))?;
    if updated_rows != 1 {
        return Err("SKILL_UPDATE_CONFLICT: skill no longer exists".into());
    }

    tx.commit()
        .map_err(|err| db_err!("failed to commit: {err}"))?;

    operation.mark_authoritative_committed();
    get_skill_by_id_for_workspace(&conn, workspace_id, skill_id)
}

/// Update the installed_commit for a skill in the database.
#[allow(dead_code)]
pub(super) fn update_installed_commit(
    db: &db::Db,
    skill_id: i64,
    commit: Option<&str>,
) -> crate::shared::error::AppResult<()> {
    let conn = db.open_connection()?;
    let now = crate::shared::time::now_unix_seconds();
    conn.execute(
        "UPDATE skills SET installed_commit = ?1, updated_at = ?2 WHERE id = ?3",
        params![commit, now, skill_id],
    )
    .map_err(|e| crate::shared::error::db_err!("failed to update installed_commit: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_update_lock_rejects_concurrent_same_skill() {
        let guard = acquire_skill_update_lock(i64::MIN).expect("first lock");

        let err = acquire_skill_update_lock(i64::MIN)
            .expect_err("second lock should fail")
            .to_string();
        assert!(err.contains("SKILL_UPDATE_IN_PROGRESS"));

        drop(guard);
        acquire_skill_update_lock(i64::MIN).expect("lock released");
    }
}
