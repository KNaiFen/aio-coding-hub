use crate::app_paths;
use crate::infra::recovery_journal::{JournalEntry, RecoveryOperation};
use crate::shared::error::{AppError, AppResult};
use crate::shared::fs::{read_file_with_max_len, write_file_atomic};
use crate::shared::time::now_unix_seconds;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::fs_ops::{
    copy_workspace_local_skill_dir, has_skill_md, is_managed_link_to_ssot,
    is_symlink_or_junction, workspace_local_skill_content_hash,
};
use super::local::managed_marker_belongs_to_installed_skill;
use super::paths::{cli_skills_root, ssot_skills_root};
use super::util::validate_dir_name;

const WORKSPACE_ARTIFACT_DIR: &str = "recovery-artifacts";
const WORKSPACE_ARTIFACT_MANIFEST: &str = "workspace-skills.json";
const WORKSPACE_ARTIFACT_OWNER: &str = ".aio-workspace-skills-owner";
const WORKSPACE_ARTIFACT_ENTRIES: &str = "entries";
const WORKSPACE_STASH_MANIFEST: &str = ".aio-workspace-skills.json";
const WORKSPACE_STASH_OWNER: &str = ".aio-workspace-skills-owner";
const WORKSPACE_MANIFEST_MAX_BYTES: usize = 128 * 1024;

fn stash_bucket_name(workspace_id: Option<i64>) -> String {
    workspace_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| "unassigned".to_string())
}

fn stash_root<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> crate::shared::error::AppResult<PathBuf> {
    Ok(app_paths::app_data_dir(app)?
        .join("skills-local")
        .join(cli_key))
}

fn is_local_skill_dir(
    conn: &Connection,
    path: &Path,
    ssot_root: &Path,
) -> crate::shared::error::AppResult<bool> {
    if !path.is_dir() {
        return Ok(false);
    }
    if managed_marker_belongs_to_installed_skill(conn, path)?
        || is_managed_link_to_ssot(path, ssot_root)
    {
        return Ok(false);
    }
    Ok(has_skill_md(path))
}

fn rotate_existing_dir(dst: &Path) -> crate::shared::error::AppResult<()> {
    if !dst.exists() {
        return Ok(());
    }
    let Some(parent) = dst.parent() else {
        return Ok(());
    };
    let base = dst
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or("skill")
        .to_string();

    let nonce = now_unix_seconds();
    let mut candidate = parent.join(format!(".{base}.old-{nonce}"));
    let mut idx = 2;
    while candidate.exists() && idx < 100 {
        candidate = parent.join(format!(".{base}.old-{nonce}-{idx}"));
        idx += 1;
    }

    std::fs::rename(dst, &candidate)
        .map_err(|e| format!("failed to rotate {}: {e}", dst.display()))?;
    Ok(())
}

fn move_dir(src: &Path, dst: &Path) -> crate::shared::error::AppResult<()> {
    let Some(parent) = dst.parent() else {
        return Err(format!("SEC_INVALID_INPUT: invalid dst path {}", dst.display()).into());
    };
    std::fs::create_dir_all(parent)
        .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;

    if dst.exists() {
        rotate_existing_dir(dst)?;
    }

    std::fs::rename(src, dst)
        .map_err(|e| format!("failed to move {} -> {}: {e}", src.display(), dst.display()).into())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceLocalSkillEntry {
    dir_name: String,
    content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceLocalSkillsManifest {
    schema_version: u8,
    operation_id: String,
    cli_key: String,
    workspace_id: Option<i64>,
    entries: Vec<WorkspaceLocalSkillEntry>,
}

fn recovery_error(code: &'static str, message: &'static str) -> AppError {
    AppError::new(code, message)
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn is_valid_skill_hash(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|digest| digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn owner_bytes(operation_id: &str) -> Vec<u8> {
    format!("{operation_id}\n").into_bytes()
}

fn validate_snapshot_manifest(
    manifest: &WorkspaceLocalSkillsManifest,
    expected_cli_key: &str,
    expected_workspace_id: Option<i64>,
    allow_legacy_owner: bool,
) -> AppResult<()> {
    if manifest.schema_version != 1
        || manifest.cli_key != expected_cli_key
        || manifest.workspace_id != expected_workspace_id
    {
        return Err(recovery_error(
            "RECOVERY_ARTIFACT_INVALID",
            "工作区本地 Skills 快照元数据不匹配",
        ));
    }
    crate::shared::cli_key::validate_cli_key(&manifest.cli_key)?;
    if manifest.workspace_id.is_some_and(|workspace_id| workspace_id <= 0) {
        return Err(recovery_error(
            "RECOVERY_ARTIFACT_INVALID",
            "工作区本地 Skills 快照工作区无效",
        ));
    }
    if manifest.operation_id != "legacy"
        && !crate::shared::uuid::is_canonical_uuid_v4(&manifest.operation_id)
    {
        return Err(recovery_error(
            "RECOVERY_ARTIFACT_INVALID",
            "工作区本地 Skills 快照所有者无效",
        ));
    }
    if manifest.operation_id == "legacy" && !allow_legacy_owner {
        return Err(recovery_error(
            "RECOVERY_ARTIFACT_INVALID",
            "工作区本地 Skills 快照所有者无效",
        ));
    }

    let mut names = BTreeSet::new();
    for entry in &manifest.entries {
        validate_dir_name(&entry.dir_name).map_err(|_| {
            recovery_error(
                "RECOVERY_ARTIFACT_INVALID",
                "工作区本地 Skills 快照路径无效",
            )
        })?;
        if !is_valid_skill_hash(&entry.content_hash) || !names.insert(entry.dir_name.as_str()) {
            return Err(recovery_error(
                "RECOVERY_ARTIFACT_INVALID",
                "工作区本地 Skills 快照条目无效",
            ));
        }
    }
    Ok(())
}

fn ensure_safe_directory(path: &Path) -> AppResult<()> {
    if path.exists() {
        let metadata = std::fs::symlink_metadata(path)
            .map_err(|_| recovery_error("RECOVERY_ARTIFACT_INVALID", "无法检查快照目录"))?;
        if metadata.file_type().is_symlink() || is_symlink_or_junction(path) || !metadata.is_dir() {
            return Err(recovery_error(
                "RECOVERY_ARTIFACT_INVALID",
                "工作区本地 Skills 快照目录不安全",
            ));
        }
        return Ok(());
    }
    std::fs::create_dir_all(path)
        .map_err(|_| recovery_error("RECOVERY_ARTIFACT_INVALID", "无法创建快照目录"))?;
    ensure_safe_directory(path)
}

fn read_owner(path: &Path, expected: &str) -> AppResult<()> {
    let bytes = read_file_with_max_len(path, 256)?;
    if bytes != owner_bytes(expected) {
        return Err(recovery_error(
            "RECOVERY_ARTIFACT_INVALID",
            "工作区本地 Skills 快照所有者不匹配",
        ));
    }
    Ok(())
}

fn write_owner(path: &Path, owner: &str) -> AppResult<()> {
    write_file_atomic(path, &owner_bytes(owner))
}

/// Returns `true` only when a crash left an empty directory before its owner
/// marker was written and that directory was removed. Any non-empty unowned
/// directory remains untouched so recovery never adopts or deletes unknown
/// content.
fn validate_owned_directory_or_remove_empty(
    root: &Path,
    owner_file: &str,
    expected_owner: &str,
) -> AppResult<bool> {
    let metadata = std::fs::symlink_metadata(root).map_err(|_| {
        recovery_error(
            "RECOVERY_ARTIFACT_INVALID",
            "无法检查工作区本地 Skills 快照目录",
        )
    })?;
    if metadata.file_type().is_symlink() || is_symlink_or_junction(root) || !metadata.is_dir() {
        return Err(recovery_error(
            "RECOVERY_ARTIFACT_INVALID",
            "工作区本地 Skills 快照目录不安全",
        ));
    }

    let owner_path = root.join(owner_file);
    match std::fs::symlink_metadata(&owner_path) {
        Ok(_) => {
            read_owner(&owner_path, expected_owner)?;
            Ok(false)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let mut entries = std::fs::read_dir(root).map_err(|_| {
                recovery_error(
                    "RECOVERY_ARTIFACT_INVALID",
                    "无法检查无所有者的工作区本地 Skills 快照",
                )
            })?;
            match entries.next() {
                None => {
                    std::fs::remove_dir(root).map_err(|_| {
                        recovery_error(
                            "RECOVERY_ARTIFACT_INVALID",
                            "无法清理空的工作区本地 Skills 快照",
                        )
                    })?;
                    Ok(true)
                }
                Some(Ok(_)) => Err(recovery_error(
                    "RECOVERY_ARTIFACT_INVALID",
                    "无所有者的工作区本地 Skills 快照不是空目录",
                )),
                Some(Err(_)) => Err(recovery_error(
                    "RECOVERY_ARTIFACT_INVALID",
                    "无法检查无所有者的工作区本地 Skills 快照",
                )),
            }
        }
        Err(_) => Err(recovery_error(
            "RECOVERY_ARTIFACT_INVALID",
            "无法检查工作区本地 Skills 快照所有者",
        )),
    }
}

fn local_skill_content_hash(path: &Path) -> AppResult<String> {
    workspace_local_skill_content_hash(path)
}

fn copy_local_skill_dir(source: &Path, destination: &Path) -> AppResult<()> {
    if std::fs::symlink_metadata(destination).is_ok() {
        return Err(recovery_error(
            "RECOVERY_PROJECTION_CONFLICT",
            "本地 Skill 快照目标已存在",
        ));
    }
    copy_workspace_local_skill_dir(source, destination)?;
    if local_skill_content_hash(source)? != local_skill_content_hash(destination)? {
        return Err(recovery_error(
            "RECOVERY_ARTIFACT_INVALID",
            "本地 Skill 快照摘要不匹配",
        ));
    }
    Ok(())
}

fn scan_local_skill_entries(
    conn: &Connection,
    cli_root: &Path,
    ssot_root: &Path,
) -> AppResult<Vec<WorkspaceLocalSkillEntry>> {
    if !cli_root.exists() {
        return Ok(Vec::new());
    }
    let entries = std::fs::read_dir(cli_root)
        .map_err(|error| format!("failed to read dir {}: {error}", cli_root.display()))?;
    let mut out = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("failed to read local skill entry: {error}"))?;
        let path = entry.path();
        if !is_local_skill_dir(conn, &path, ssot_root)? {
            continue;
        }
        let dir_name = entry.file_name().into_string().map_err(|_| {
            recovery_error(
                "RECOVERY_ARTIFACT_INVALID",
                "本地 Skill 目录名不是 UTF-8",
            )
        })?;
        validate_dir_name(&dir_name).map_err(|_| {
            recovery_error(
                "RECOVERY_ARTIFACT_INVALID",
                "本地 Skill 目录名无效",
            )
        })?;
        out.push(WorkspaceLocalSkillEntry {
            content_hash: local_skill_content_hash(&path)?,
            dir_name,
        });
    }
    out.sort_by(|left, right| left.dir_name.cmp(&right.dir_name));
    Ok(out)
}

fn artifact_base<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> AppResult<PathBuf> {
    let base = app_paths::app_data_dir(app)?.join(WORKSPACE_ARTIFACT_DIR);
    ensure_safe_directory(&base)?;
    Ok(base)
}

fn artifact_root<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    operation_id: &str,
) -> AppResult<PathBuf> {
    if !crate::shared::uuid::is_canonical_uuid_v4(operation_id) {
        return Err(recovery_error(
            "RECOVERY_ARTIFACT_INVALID",
            "工作区恢复操作标识无效",
        ));
    }
    Ok(artifact_base(app)?.join(operation_id))
}

fn manifest_bytes(manifest: &WorkspaceLocalSkillsManifest) -> AppResult<Vec<u8>> {
    serde_json::to_vec(manifest)
        .map_err(|_| recovery_error("RECOVERY_ARTIFACT_INVALID", "无法序列化本地 Skills 快照"))
}

fn read_manifest(path: &Path) -> AppResult<(WorkspaceLocalSkillsManifest, Vec<u8>)> {
    let bytes = read_file_with_max_len(path, WORKSPACE_MANIFEST_MAX_BYTES)?;
    let manifest = serde_json::from_slice(&bytes).map_err(|_| {
        recovery_error(
            "RECOVERY_ARTIFACT_INVALID",
            "工作区本地 Skills 快照清单损坏",
        )
    })?;
    Ok((manifest, bytes))
}

fn verify_payload(
    root: &Path,
    payload_root: &Path,
    manifest: &WorkspaceLocalSkillsManifest,
    allowed_root_entries: BTreeSet<String>,
) -> AppResult<()> {
    ensure_safe_directory(root)?;
    let actual_names = std::fs::read_dir(root)
        .map_err(|_| recovery_error("RECOVERY_ARTIFACT_INVALID", "无法读取本地 Skills 快照"))?
        .map(|entry| {
            let entry = entry.map_err(|_| {
                recovery_error("RECOVERY_ARTIFACT_INVALID", "无法读取本地 Skills 快照条目")
            })?;
            let name = entry.file_name().into_string().map_err(|_| {
                recovery_error("RECOVERY_ARTIFACT_INVALID", "本地 Skills 快照包含非 UTF-8 文件名")
            })?;
            if is_symlink_or_junction(&entry.path()) {
                return Err(recovery_error(
                    "RECOVERY_ARTIFACT_INVALID",
                    "本地 Skills 快照包含链接",
                ));
            }
            Ok(name)
        })
        .collect::<AppResult<BTreeSet<_>>>()?;
    if actual_names != allowed_root_entries {
        return Err(recovery_error(
            "RECOVERY_ARTIFACT_INVALID",
            "本地 Skills 快照包含未受管条目",
        ));
    }
    if payload_root != root {
        let payload_names = std::fs::read_dir(payload_root)
            .map_err(|_| recovery_error("RECOVERY_ARTIFACT_INVALID", "无法读取本地 Skills 快照内容"))?
            .map(|entry| {
                entry
                    .map_err(|_| recovery_error("RECOVERY_ARTIFACT_INVALID", "无法读取本地 Skills 快照内容"))?
                    .file_name()
                    .into_string()
                    .map_err(|_| recovery_error("RECOVERY_ARTIFACT_INVALID", "本地 Skills 快照包含非 UTF-8 文件名"))
            })
            .collect::<AppResult<BTreeSet<_>>>()?;
        let expected_names = manifest
            .entries
            .iter()
            .map(|entry| entry.dir_name.clone())
            .collect::<BTreeSet<_>>();
        if payload_names != expected_names {
            return Err(recovery_error(
                "RECOVERY_ARTIFACT_INVALID",
                "本地 Skills 快照内容条目不匹配",
            ));
        }
    }
    for entry in &manifest.entries {
        let path = payload_root.join(&entry.dir_name);
        if is_symlink_or_junction(&path) || !path.is_dir() || local_skill_content_hash(&path)? != entry.content_hash {
            return Err(recovery_error(
                "RECOVERY_ARTIFACT_INVALID",
                "本地 Skills 快照内容摘要不匹配",
            ));
        }
    }
    Ok(())
}

fn load_workspace_artifact<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    operation: &RecoveryOperation,
    cli_key: &str,
    workspace_id: Option<i64>,
) -> AppResult<(WorkspaceLocalSkillsManifest, String)> {
    let operation_id = operation.operation_id();
    let root = artifact_root(app, operation_id)?;
    read_owner(&root.join(WORKSPACE_ARTIFACT_OWNER), operation_id)?;
    let (manifest, bytes) = read_manifest(&root.join(WORKSPACE_ARTIFACT_MANIFEST))?;
    validate_snapshot_manifest(&manifest, cli_key, workspace_id, false)?;
    if manifest.operation_id != operation_id {
        return Err(recovery_error(
            "RECOVERY_ARTIFACT_INVALID",
            "工作区本地 Skills 快照操作不匹配",
        ));
    }
    let digest = sha256_hex(&bytes);
    if let Some(reference) = operation.entry().artifact_ref.as_deref() {
        if reference != operation_id
            || operation.entry().artifact_sha256.as_deref() != Some(digest.as_str())
        {
            return Err(recovery_error(
                "RECOVERY_ARTIFACT_INVALID",
                "工作区本地 Skills 快照日志绑定不匹配",
            ));
        }
    }
    let allowed = BTreeSet::from([
        WORKSPACE_ARTIFACT_OWNER.to_string(),
        WORKSPACE_ARTIFACT_MANIFEST.to_string(),
        WORKSPACE_ARTIFACT_ENTRIES.to_string(),
    ]);
    verify_payload(
        &root,
        &root.join(WORKSPACE_ARTIFACT_ENTRIES),
        &manifest,
        allowed,
    )?;
    Ok((manifest, digest))
}

pub(crate) fn stage_local_skills_for_workspace_switch<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    conn: &Connection,
    cli_key: &str,
    from_workspace_id: Option<i64>,
    operation: &RecoveryOperation,
) -> AppResult<String> {
    crate::shared::cli_key::validate_cli_key(cli_key)?;
    operation.renew_lease()?;
    let root = artifact_root(app, operation.operation_id())?;
    if root.exists() {
        let (_manifest, digest) =
            load_workspace_artifact(app, operation, cli_key, from_workspace_id)?;
        return Ok(digest);
    }

    std::fs::create_dir(&root)
        .map_err(|_| recovery_error("RECOVERY_ARTIFACT_INVALID", "无法创建本地 Skills 快照"))?;
    write_owner(&root.join(WORKSPACE_ARTIFACT_OWNER), operation.operation_id())?;
    let entries_root = root.join(WORKSPACE_ARTIFACT_ENTRIES);
    std::fs::create_dir(&entries_root)
        .map_err(|_| recovery_error("RECOVERY_ARTIFACT_INVALID", "无法创建本地 Skills 快照条目"))?;

    let cli_root = cli_skills_root(app, cli_key)?;
    let ssot_root = ssot_skills_root(app)?;
    let entries = scan_local_skill_entries(conn, &cli_root, &ssot_root)?;
    for entry in &entries {
        copy_local_skill_dir(&cli_root.join(&entry.dir_name), &entries_root.join(&entry.dir_name))?;
    }
    let manifest = WorkspaceLocalSkillsManifest {
        schema_version: 1,
        operation_id: operation.operation_id().to_string(),
        cli_key: cli_key.to_string(),
        workspace_id: from_workspace_id,
        entries,
    };
    let bytes = manifest_bytes(&manifest)?;
    write_file_atomic(&root.join(WORKSPACE_ARTIFACT_MANIFEST), &bytes)?;
    let (_manifest, persisted_digest) =
        load_workspace_artifact(app, operation, cli_key, from_workspace_id)?;
    Ok(persisted_digest)
}

fn stash_bucket_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    workspace_id: Option<i64>,
) -> AppResult<PathBuf> {
    Ok(stash_root(app, cli_key)?.join(stash_bucket_name(workspace_id)))
}

fn migrate_legacy_stash(
    root: &Path,
    cli_key: &str,
    workspace_id: Option<i64>,
) -> AppResult<WorkspaceLocalSkillsManifest> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(root)
        .map_err(|_| recovery_error("RECOVERY_ARTIFACT_INVALID", "无法读取旧本地 Skills 暂存"))?
    {
        let entry = entry.map_err(|_| recovery_error("RECOVERY_ARTIFACT_INVALID", "无法读取旧本地 Skills 条目"))?;
        let path = entry.path();
        let name = entry.file_name().into_string().map_err(|_| {
            recovery_error("RECOVERY_ARTIFACT_INVALID", "旧本地 Skills 暂存包含非 UTF-8 文件名")
        })?;
        if name == WORKSPACE_STASH_OWNER || name == WORKSPACE_STASH_MANIFEST {
            continue;
        }
        if is_legacy_rotated_stash_name(&name) {
            return Err(recovery_error(
                "RECOVERY_ARTIFACT_INVALID",
                "旧本地 Skills 暂存包含轮换备份目录",
            ));
        }
        validate_dir_name(&name).map_err(|_| {
            recovery_error("RECOVERY_ARTIFACT_INVALID", "旧本地 Skills 暂存路径无效")
        })?;
        if is_symlink_or_junction(&path) || !path.is_dir() || !has_skill_md(&path) {
            return Err(recovery_error(
                "RECOVERY_ARTIFACT_INVALID",
                "旧本地 Skills 暂存包含不安全条目",
            ));
        }
        entries.push(WorkspaceLocalSkillEntry {
            dir_name: name,
            content_hash: local_skill_content_hash(&path)?,
        });
    }
    entries.sort_by(|left, right| left.dir_name.cmp(&right.dir_name));
    let manifest = WorkspaceLocalSkillsManifest {
        schema_version: 1,
        operation_id: "legacy".to_string(),
        cli_key: cli_key.to_string(),
        workspace_id,
        entries,
    };
    write_owner(&root.join(WORKSPACE_STASH_OWNER), "legacy")?;
    write_file_atomic(&root.join(WORKSPACE_STASH_MANIFEST), &manifest_bytes(&manifest)?)?;
    Ok(manifest)
}

fn is_legacy_rotated_stash_name(name: &str) -> bool {
    let Some(hidden_name) = name.strip_prefix('.') else {
        return false;
    };
    hidden_name.starts_with("old-") || hidden_name.contains(".old-")
}

fn load_stash<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    workspace_id: Option<i64>,
) -> AppResult<Option<WorkspaceLocalSkillsManifest>> {
    let root = stash_bucket_path(app, cli_key, workspace_id)?;
    if !root.exists() {
        return Ok(None);
    }
    ensure_safe_directory(&root)?;
    let manifest_path = root.join(WORKSPACE_STASH_MANIFEST);
    let owner_path = root.join(WORKSPACE_STASH_OWNER);
    let (manifest, _bytes) = if manifest_path.exists() || owner_path.exists() {
        if !manifest_path.exists() || !owner_path.exists() {
            return Err(recovery_error(
                "RECOVERY_ARTIFACT_INVALID",
                "本地 Skills 暂存所有者或清单缺失",
            ));
        }
        let (manifest, bytes) = read_manifest(&manifest_path)?;
        validate_snapshot_manifest(&manifest, cli_key, workspace_id, true)?;
        read_owner(&owner_path, &manifest.operation_id)?;
        (manifest, bytes)
    } else {
        let manifest = migrate_legacy_stash(&root, cli_key, workspace_id)?;
        (manifest.clone(), manifest_bytes(&manifest)?)
    };
    let mut allowed = BTreeSet::from([
        WORKSPACE_STASH_OWNER.to_string(),
        WORKSPACE_STASH_MANIFEST.to_string(),
    ]);
    allowed.extend(manifest.entries.iter().map(|entry| entry.dir_name.clone()));
    verify_payload(&root, &root, &manifest, allowed)?;
    Ok(Some(manifest))
}

fn replace_stash_from_artifact<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    operation: &RecoveryOperation,
    cli_key: &str,
    workspace_id: Option<i64>,
    manifest: &WorkspaceLocalSkillsManifest,
) -> AppResult<()> {
    let root = stash_bucket_path(app, cli_key, workspace_id)?;
    let parent = root.parent().ok_or_else(|| {
        recovery_error("RECOVERY_ARTIFACT_INVALID", "本地 Skills 暂存路径无效")
    })?;
    ensure_safe_directory(parent)?;
    let temp = parent.join(format!(".{}.{}", stash_bucket_name(workspace_id), operation.operation_id()));
    if temp.exists() {
        let removed = validate_owned_directory_or_remove_empty(
            &temp,
            WORKSPACE_STASH_OWNER,
            operation.operation_id(),
        )?;
        if !removed {
            std::fs::remove_dir_all(&temp).map_err(|_| {
                recovery_error("RECOVERY_ARTIFACT_INVALID", "无法清理本地 Skills 暂存临时目录")
            })?;
        }
    }
    std::fs::create_dir(&temp)
        .map_err(|_| recovery_error("RECOVERY_ARTIFACT_INVALID", "无法创建本地 Skills 暂存"))?;
    write_owner(&temp.join(WORKSPACE_STASH_OWNER), operation.operation_id())?;
    let artifact_entries = artifact_root(app, operation.operation_id())?.join(WORKSPACE_ARTIFACT_ENTRIES);
    for entry in &manifest.entries {
        copy_local_skill_dir(
            &artifact_entries.join(&entry.dir_name),
            &temp.join(&entry.dir_name),
        )?;
    }
    write_file_atomic(
        &temp.join(WORKSPACE_STASH_MANIFEST),
        &manifest_bytes(manifest)?,
    )?;
    if root.exists() {
        let _ = load_stash(app, cli_key, workspace_id)?;
        std::fs::remove_dir_all(&root).map_err(|_| {
            recovery_error("RECOVERY_ARTIFACT_INVALID", "无法替换本地 Skills 暂存")
        })?;
    }
    std::fs::rename(&temp, &root)
        .map_err(|_| recovery_error("RECOVERY_ARTIFACT_INVALID", "无法提升本地 Skills 暂存"))?;
    let _ = load_stash(app, cli_key, workspace_id)?;
    Ok(())
}

pub(crate) fn capture_staged_local_skills_for_workspace_switch<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    conn: &Connection,
    cli_key: &str,
    from_workspace_id: Option<i64>,
    operation: &RecoveryOperation,
) -> AppResult<()> {
    let (manifest, _digest) =
        load_workspace_artifact(app, operation, cli_key, from_workspace_id)?;
    replace_stash_from_artifact(app, operation, cli_key, from_workspace_id, &manifest)?;

    let cli_root = cli_skills_root(app, cli_key)?;
    let ssot_root = ssot_skills_root(app)?;
    let actual = scan_local_skill_entries(conn, &cli_root, &ssot_root)?;
    for actual_entry in &actual {
        if !manifest.entries.iter().any(|expected| {
            actual_entry.dir_name == expected.dir_name
                && actual_entry.content_hash == expected.content_hash
        }) {
            return Err(recovery_error(
                "RECOVERY_PROJECTION_CONFLICT",
                "本地 Skills 在工作区切换期间已变化",
            ));
        }
    }
    for entry in &manifest.entries {
        let path = cli_root.join(&entry.dir_name);
        if std::fs::symlink_metadata(&path).is_err() {
            continue;
        }
        if is_symlink_or_junction(&path) || local_skill_content_hash(&path)? != entry.content_hash {
            return Err(recovery_error(
                "RECOVERY_PROJECTION_CONFLICT",
                "本地 Skills 暂存前后不一致",
            ));
        }
        std::fs::remove_dir_all(&path).map_err(|_| {
            recovery_error("RECOVERY_PROJECTION_CONFLICT", "无法暂存本地 Skill")
        })?;
    }
    Ok(())
}

pub(crate) fn restore_staged_local_skills_for_workspace_switch<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    conn: &Connection,
    cli_key: &str,
    to_workspace_id: i64,
    _operation: &RecoveryOperation,
) -> AppResult<()> {
    let Some(manifest) = load_stash(app, cli_key, Some(to_workspace_id))? else {
        return Ok(());
    };
    let cli_root = cli_skills_root(app, cli_key)?;
    ensure_safe_directory(&cli_root)?;
    let ssot_root = ssot_skills_root(app)?;
    let stash_root = stash_bucket_path(app, cli_key, Some(to_workspace_id))?;
    for entry in &manifest.entries {
        let source = stash_root.join(&entry.dir_name);
        let destination = cli_root.join(&entry.dir_name);
        if std::fs::symlink_metadata(&destination).is_ok() {
            if !is_local_skill_dir(conn, &destination, &ssot_root)?
                || local_skill_content_hash(&destination)? != entry.content_hash
            {
                return Err(recovery_error(
                    "RECOVERY_PROJECTION_CONFLICT",
                    "本地 Skill 恢复目标冲突",
                ));
            }
            continue;
        }
        copy_local_skill_dir(&source, &destination)?;
    }
    Ok(())
}

pub(crate) fn cleanup_workspace_switch_local_skills_artifact<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    entry: &JournalEntry,
) -> AppResult<()> {
    let root = artifact_root(app, &entry.operation_id)?;
    if !root.exists() {
        return Ok(());
    }
    if validate_owned_directory_or_remove_empty(
        &root,
        WORKSPACE_ARTIFACT_OWNER,
        &entry.operation_id,
    )? {
        return Ok(());
    }
    std::fs::remove_dir_all(&root).map_err(|_| {
        recovery_error("RECOVERY_ARTIFACT_INVALID", "无法清理本地 Skills 快照")
    })
}

pub(crate) fn capture_local_skills_for_workspace_switch<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    conn: &Connection,
    cli_key: &str,
    from_workspace_id: Option<i64>,
) -> crate::shared::error::AppResult<()> {
    let cli_root = cli_skills_root(app, cli_key)?;
    let ssot_root = ssot_skills_root(app)?;

    let stash_root = stash_root(app, cli_key)?;
    let from_bucket = stash_root.join(stash_bucket_name(from_workspace_id));

    std::fs::create_dir_all(&from_bucket)
        .map_err(|e| format!("failed to create {}: {e}", from_bucket.display()))?;

    if cli_root.exists() {
        let entries = std::fs::read_dir(&cli_root)
            .map_err(|e| format!("failed to read dir {}: {e}", cli_root.display()))?;
        for entry in entries {
            let entry = entry
                .map_err(|e| format!("failed to read dir entry {}: {e}", cli_root.display()))?;
            let path = entry.path();
            if !is_local_skill_dir(conn, &path, &ssot_root)? {
                continue;
            }
            let dir_name = path
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or("")
                .to_string();
            if dir_name.is_empty() {
                continue;
            }
            let dst = from_bucket.join(&dir_name);
            move_dir(&path, &dst)?;
        }
    }

    Ok(())
}

pub(crate) fn restore_local_skills_for_workspace_switch<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    conn: &Connection,
    cli_key: &str,
    to_workspace_id: i64,
) -> crate::shared::error::AppResult<()> {
    let cli_root = cli_skills_root(app, cli_key)?;
    let ssot_root = ssot_skills_root(app)?;
    let to_bucket = stash_root(app, cli_key)?.join(to_workspace_id.to_string());

    std::fs::create_dir_all(&to_bucket)
        .map_err(|e| format!("failed to create {}: {e}", to_bucket.display()))?;

    if to_bucket.exists() {
        let entries = std::fs::read_dir(&to_bucket)
            .map_err(|e| format!("failed to read dir {}: {e}", to_bucket.display()))?;
        for entry in entries {
            let entry = entry
                .map_err(|e| format!("failed to read dir entry {}: {e}", to_bucket.display()))?;
            let path = entry.path();
            if !is_local_skill_dir(conn, &path, &ssot_root)? {
                continue;
            }
            let dir_name = path
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or("")
                .to_string();
            if dir_name.is_empty() {
                continue;
            }

            let dst = cli_root.join(&dir_name);
            if dst.exists() {
                tracing::warn!(
                    cli_key = %cli_key,
                    dir = %dir_name,
                    "本机 Skills 切换: 目标目录已存在，跳过恢复"
                );
                continue;
            }

            move_dir(&path, &dst)?;
        }
    }

    Ok(())
}

pub(crate) fn swap_local_skills_for_workspace_switch<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    conn: &Connection,
    cli_key: &str,
    from_workspace_id: Option<i64>,
    to_workspace_id: i64,
) -> crate::shared::error::AppResult<()> {
    capture_local_skills_for_workspace_switch(app, conn, cli_key, from_workspace_id)?;
    restore_local_skills_for_workspace_switch(app, conn, cli_key, to_workspace_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_unowned_snapshot_directory_is_removed() {
        let parent = tempfile::tempdir().expect("tempdir");
        let root = parent.path().join("artifact");
        std::fs::create_dir(&root).expect("create empty artifact");

        assert!(validate_owned_directory_or_remove_empty(&root, "owner", "expected")
            .expect("empty unowned artifact is recoverable"));
        assert!(!root.exists());
    }

    #[test]
    fn nonempty_unowned_snapshot_directory_is_preserved_and_rejected() {
        let parent = tempfile::tempdir().expect("tempdir");
        let root = parent.path().join("artifact");
        std::fs::create_dir(&root).expect("create artifact");
        std::fs::write(root.join("unknown"), b"content").expect("write unknown entry");

        let error = validate_owned_directory_or_remove_empty(&root, "owner", "expected")
            .expect_err("nonempty unowned artifact must fail closed");

        assert_eq!(error.code(), "RECOVERY_ARTIFACT_INVALID");
        assert!(root.join("unknown").exists());
    }

    #[test]
    fn legacy_rotation_names_are_never_restored_as_skills() {
        for name in [".demo.old-123", ".demo.old-disguised", ".old-123"] {
            assert!(is_legacy_rotated_stash_name(name), "accepted {name}");
        }
        assert!(!is_legacy_rotated_stash_name("demo"));
        assert!(!is_legacy_rotated_stash_name("old-demo"));
    }

    #[test]
    fn manifest_reader_preserves_original_bytes_for_journal_digest() {
        let root = tempfile::tempdir().expect("tempdir");
        let path = root.path().join("manifest.json");
        let bytes = br#"{
  "schema_version": 1,
  "operation_id": "legacy",
  "cli_key": "claude",
  "workspace_id": null,
  "entries": []
}
"#;
        std::fs::write(&path, bytes).expect("write manifest");

        let (_manifest, loaded_bytes) = read_manifest(&path).expect("read manifest");

        assert_eq!(loaded_bytes, bytes);
    }
}
