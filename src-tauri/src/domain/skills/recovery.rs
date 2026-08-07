//! Usage: Journal-owned Skills recovery artifacts and replay projection.

use super::fs_ops::{
    copy_dir_recursive, exists_or_is_link, is_managed_dir, is_managed_link_to_ssot,
    is_symlink_or_junction, remove_managed_dir, remove_marker, skill_dir_content_hash,
    write_source_metadata, SkillSourceMetadata,
};
use super::ops::{remove_from_cli, remove_managed_targets_except, sync_one_cli};
use super::paths::{cli_skills_root, ssot_skills_root};
use crate::app_paths;
use crate::db;
use crate::infra::recovery_journal::{JournalEntry, RecoveryOperation};
use crate::shared::error::{AppError, AppResult};
use crate::shared::fs::{read_file_with_max_len, write_file_atomic};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const ARTIFACT_DIR_NAME: &str = "recovery-artifacts";
const ARTIFACT_MANIFEST_FILE: &str = "artifact.json";
const ARTIFACT_OWNER_FILE: &str = ".aio-recovery-owner";
const ARTIFACT_SOURCE_FILE: &str = "source.json";
const ARTIFACT_MAX_BYTES: usize = 128 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum SkillRecoveryContext {
    Install {
        workspace_id: i64,
        skill_key: String,
    },
    ImportLocal {
        workspace_id: i64,
        cli_key: String,
        skill_key: String,
        local_dir_name: String,
    },
    Update {
        workspace_id: i64,
        skill_id: i64,
        skill_key: String,
    },
    Uninstall {
        skill_id: i64,
        skill_key: String,
    },
    ReturnToLocal {
        workspace_id: i64,
        cli_key: String,
        skill_id: i64,
        skill_key: String,
    },
    InstallToLocal {
        workspace_id: i64,
        cli_key: String,
        dir_name: String,
    },
    DeleteLocal {
        workspace_id: i64,
        cli_key: String,
        dir_name: String,
    },
}

impl SkillRecoveryContext {
    fn operation_kind(&self) -> &'static str {
        match self {
            Self::Install { .. } => "skill.install",
            Self::ImportLocal { .. } => "skill.import_local",
            Self::Update { .. } => "skill.update",
            Self::Uninstall { .. } => "skill.uninstall",
            Self::ReturnToLocal { .. } => "skill.return_to_local",
            Self::InstallToLocal { .. } => "skill.install_to_local",
            Self::DeleteLocal { .. } => "skill.local_delete",
        }
    }

    fn matches_journal_scope(&self, entry: &JournalEntry) -> bool {
        match self {
            Self::Install { workspace_id, .. } => {
                entry.workspace_id == Some(*workspace_id) && entry.entity_id.is_none()
            }
            Self::ImportLocal {
                workspace_id,
                cli_key,
                ..
            }
            | Self::InstallToLocal {
                workspace_id,
                cli_key,
                ..
            }
            | Self::DeleteLocal {
                workspace_id,
                cli_key,
                ..
            } => {
                entry.workspace_id == Some(*workspace_id)
                    && entry.cli_key.as_deref() == Some(cli_key.as_str())
                    && entry.entity_id.is_none()
            }
            Self::Update {
                workspace_id,
                skill_id,
                ..
            } => entry.workspace_id == Some(*workspace_id) && entry.entity_id == Some(*skill_id),
            Self::Uninstall { skill_id, .. } => {
                entry.workspace_id.is_none() && entry.entity_id == Some(*skill_id)
            }
            Self::ReturnToLocal {
                workspace_id,
                cli_key,
                skill_id,
                ..
            } => {
                entry.workspace_id == Some(*workspace_id)
                    && entry.cli_key.as_deref() == Some(cli_key.as_str())
                    && entry.entity_id == Some(*skill_id)
            }
        }
    }

    fn validate_path_components(&self) -> AppResult<()> {
        fn validate(value: &str) -> AppResult<()> {
            super::util::validate_dir_name(value)
                .map(|_| ())
                .map_err(|_| {
                    AppError::new(
                        "RECOVERY_ARTIFACT_INVALID",
                        "Skill 恢复上下文包含不安全路径组件",
                    )
                })
        }

        match self {
            Self::Install { skill_key, .. }
            | Self::Update { skill_key, .. }
            | Self::Uninstall { skill_key, .. }
            | Self::ReturnToLocal { skill_key, .. } => validate(skill_key),
            Self::ImportLocal {
                skill_key,
                local_dir_name,
                ..
            } => {
                validate(skill_key)?;
                validate(local_dir_name)
            }
            Self::InstallToLocal { dir_name, .. } | Self::DeleteLocal { dir_name, .. } => {
                validate(dir_name)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArtifactRole {
    relative_path: String,
    content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArtifactManifest {
    schema_version: u8,
    operation_id: String,
    roles: BTreeMap<String, ArtifactRole>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source_sha256: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct StagedArtifact {
    roles: BTreeMap<String, ArtifactRole>,
}

impl StagedArtifact {
    pub(super) fn role_hash(&self, role: &str) -> AppResult<&str> {
        self.roles
            .get(role)
            .map(|value| value.content_hash.as_str())
            .ok_or_else(|| AppError::new("RECOVERY_ARTIFACT_INVALID", "恢复制品缺少内容角色"))
    }
}

#[derive(Debug)]
struct LoadedArtifact {
    root: PathBuf,
    manifest: ArtifactManifest,
    source_metadata: Option<SkillSourceMetadata>,
}

fn artifact_root<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> AppResult<PathBuf> {
    let root = app_paths::app_data_dir(app)?.join(ARTIFACT_DIR_NAME);
    ensure_safe_directory(&root)?;
    Ok(root)
}

fn ensure_safe_directory(path: &Path) -> AppResult<()> {
    let mut ancestors = path.ancestors().collect::<Vec<_>>();
    ancestors.reverse();
    for ancestor in ancestors {
        let Ok(metadata) = std::fs::symlink_metadata(ancestor) else {
            continue;
        };
        if metadata.file_type().is_symlink() || is_symlink_or_junction(ancestor) {
            return Err(format!("RECOVERY_ARTIFACT_UNSAFE_PATH: {}", ancestor.display()).into());
        }
        if !metadata.is_dir() {
            return Err(format!("RECOVERY_ARTIFACT_UNSAFE_PATH: {}", ancestor.display()).into());
        }
    }
    std::fs::create_dir_all(path)
        .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() || is_symlink_or_junction(path) || !metadata.is_dir() {
        return Err(format!("RECOVERY_ARTIFACT_UNSAFE_PATH: {}", path.display()).into());
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn artifact_context_json(context: &SkillRecoveryContext) -> AppResult<String> {
    serde_json::to_string(context)
        .map_err(|_| AppError::new("RECOVERY_ARTIFACT_INVALID", "无法序列化恢复上下文"))
}

fn artifact_role_path(root: &Path, role: &str) -> AppResult<PathBuf> {
    if !matches!(role, "desired" | "previous") {
        return Err(AppError::new(
            "RECOVERY_ARTIFACT_INVALID",
            "恢复制品角色无效",
        ));
    }
    Ok(root.join(role))
}

fn validate_artifact_role(role: &ArtifactRole, root: &Path) -> AppResult<PathBuf> {
    if role.relative_path != "desired" && role.relative_path != "previous" {
        return Err(AppError::new(
            "RECOVERY_ARTIFACT_INVALID",
            "恢复制品路径无效",
        ));
    }
    let path = root.join(&role.relative_path);
    if !path.starts_with(root) || is_symlink_or_junction(&path) {
        return Err(AppError::new(
            "RECOVERY_ARTIFACT_INVALID",
            "恢复制品路径不受信任",
        ));
    }
    Ok(path)
}

fn validate_root_entries(
    root: &Path,
    roles: &BTreeMap<String, ArtifactRole>,
    has_source_metadata: bool,
) -> AppResult<()> {
    let mut allowed = BTreeSet::from([
        ARTIFACT_MANIFEST_FILE.to_string(),
        ARTIFACT_OWNER_FILE.to_string(),
    ]);
    if has_source_metadata {
        allowed.insert(ARTIFACT_SOURCE_FILE.to_string());
    }
    allowed.extend(roles.values().map(|role| role.relative_path.clone()));
    let entries = std::fs::read_dir(root)
        .map_err(|error| format!("failed to read {}: {error}", root.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("failed to inspect artifact entry: {error}"))?;
        let name = entry.file_name().into_string().map_err(|_| {
            AppError::new("RECOVERY_ARTIFACT_INVALID", "恢复制品包含非 UTF-8 文件名")
        })?;
        if !allowed.contains(&name) || is_symlink_or_junction(&entry.path()) {
            return Err(AppError::new(
                "RECOVERY_ARTIFACT_INVALID",
                "恢复制品包含未受管文件",
            ));
        }
    }
    Ok(())
}

pub(super) fn stage_artifact<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    conn: &Connection,
    operation: &RecoveryOperation,
    context: &SkillRecoveryContext,
    role_sources: &[(&str, &Path)],
    source_metadata: Option<&SkillSourceMetadata>,
) -> AppResult<StagedArtifact> {
    operation.renew_lease_with_conn(conn)?;
    let root = artifact_root(app)?.join(operation.operation_id());
    if root.exists() {
        return Err(AppError::new(
            "RECOVERY_ARTIFACT_CONFLICT",
            "恢复操作的制品目录已存在",
        ));
    }
    std::fs::create_dir(&root)
        .map_err(|error| format!("failed to create artifact {}: {error}", root.display()))?;
    ensure_safe_directory(&root)?;

    let result = (|| {
        write_file_atomic(
            &root.join(ARTIFACT_OWNER_FILE),
            operation.operation_id().as_bytes(),
        )?;
        let mut roles = BTreeMap::new();
        for (role, source) in role_sources {
            let target = artifact_role_path(&root, role)?;
            copy_dir_recursive(source, &target)?;
            roles.insert(
                (*role).to_string(),
                ArtifactRole {
                    relative_path: (*role).to_string(),
                    content_hash: skill_dir_content_hash(&target)?,
                },
            );
        }
        if roles.is_empty() {
            return Err(AppError::new("RECOVERY_ARTIFACT_INVALID", "恢复制品为空"));
        }
        let source_sha256 = if let Some(metadata) = source_metadata {
            let bytes = serde_json::to_vec(metadata).map_err(|_| {
                AppError::new("RECOVERY_ARTIFACT_INVALID", "无法序列化 Skill 来源元数据")
            })?;
            write_file_atomic(&root.join(ARTIFACT_SOURCE_FILE), &bytes)?;
            Some(sha256_hex(&bytes))
        } else {
            None
        };
        let manifest = ArtifactManifest {
            schema_version: 1,
            operation_id: operation.operation_id().to_string(),
            roles: roles.clone(),
            source_sha256: source_sha256.clone(),
        };
        let bytes = serde_json::to_vec(&manifest)
            .map_err(|_| AppError::new("RECOVERY_ARTIFACT_INVALID", "无法序列化恢复制品清单"))?;
        let digest = sha256_hex(&bytes);
        write_file_atomic(&root.join(ARTIFACT_MANIFEST_FILE), &bytes)?;
        validate_root_entries(&root, &roles, source_sha256.is_some())?;
        let context_json = artifact_context_json(context)?;
        operation.configure_replay_with_conn(
            conn,
            &context_json,
            Some(operation.operation_id()),
            Some(&digest),
        )?;
        Ok(StagedArtifact { roles })
    })();

    match result {
        Ok(artifact) => Ok(artifact),
        Err(primary) => match remove_owned_artifact(app, operation.operation_id(), None) {
            Ok(()) => Err(primary),
            Err(cleanup) => Err(AppError::new(
                primary.code(),
                format!(
                    "{}; recovery_artifact_cleanup={}",
                    primary.code(),
                    cleanup.code()
                ),
            )),
        },
    }
}

fn load_artifact<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    entry: &JournalEntry,
    expected_context: Option<&SkillRecoveryContext>,
) -> AppResult<LoadedArtifact> {
    let Some(reference) = entry.artifact_ref.as_deref() else {
        return Err(AppError::new(
            "RECOVERY_ARTIFACT_MISSING",
            "恢复操作缺少制品引用",
        ));
    };
    if reference != entry.operation_id || !crate::shared::uuid::is_canonical_uuid_v4(reference) {
        return Err(AppError::new(
            "RECOVERY_ARTIFACT_INVALID",
            "恢复制品引用无效",
        ));
    }
    let root = artifact_root(app)?.join(reference);
    let metadata = std::fs::symlink_metadata(&root)
        .map_err(|_| AppError::new("RECOVERY_ARTIFACT_MISSING", "恢复制品不存在"))?;
    if metadata.file_type().is_symlink() || is_symlink_or_junction(&root) || !metadata.is_dir() {
        return Err(AppError::new(
            "RECOVERY_ARTIFACT_INVALID",
            "恢复制品目录不安全",
        ));
    }
    let owner = read_file_with_max_len(&root.join(ARTIFACT_OWNER_FILE), 128)?;
    if owner != entry.operation_id.as_bytes() {
        return Err(AppError::new(
            "RECOVERY_ARTIFACT_INVALID",
            "恢复制品所有权不匹配",
        ));
    }
    let manifest_bytes =
        read_file_with_max_len(&root.join(ARTIFACT_MANIFEST_FILE), ARTIFACT_MAX_BYTES)?;
    let actual_digest = sha256_hex(&manifest_bytes);
    let expected_digest = entry
        .artifact_sha256
        .as_deref()
        .ok_or_else(|| AppError::new("RECOVERY_ARTIFACT_INVALID", "恢复日志缺少制品摘要"))?;
    if expected_digest != actual_digest {
        return Err(AppError::new(
            "RECOVERY_ARTIFACT_INVALID",
            "恢复制品摘要不匹配",
        ));
    }
    let manifest: ArtifactManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|_| AppError::new("RECOVERY_ARTIFACT_INVALID", "恢复制品清单损坏"))?;
    if manifest.schema_version != 1 || manifest.operation_id != entry.operation_id {
        return Err(AppError::new(
            "RECOVERY_ARTIFACT_INVALID",
            "恢复制品清单不匹配",
        ));
    }
    validate_root_entries(&root, &manifest.roles, manifest.source_sha256.is_some())?;
    for role in manifest.roles.values() {
        let path = validate_artifact_role(role, &root)?;
        if skill_dir_content_hash(&path)? != role.content_hash {
            return Err(AppError::new(
                "RECOVERY_ARTIFACT_INVALID",
                "恢复制品内容摘要不匹配",
            ));
        }
    }
    let source_path = root.join(ARTIFACT_SOURCE_FILE);
    let source_metadata = match manifest.source_sha256.as_deref() {
        Some(expected_source_digest) => {
            let metadata = std::fs::symlink_metadata(&source_path)
                .map_err(|_| AppError::new("RECOVERY_ARTIFACT_INVALID", "制品来源元数据缺失"))?;
            if metadata.file_type().is_symlink()
                || is_symlink_or_junction(&source_path)
                || !metadata.is_file()
            {
                return Err(AppError::new(
                    "RECOVERY_ARTIFACT_INVALID",
                    "制品来源元数据不安全",
                ));
            }
            let bytes = read_file_with_max_len(&source_path, ARTIFACT_MAX_BYTES)?;
            if sha256_hex(&bytes) != expected_source_digest {
                return Err(AppError::new(
                    "RECOVERY_ARTIFACT_INVALID",
                    "制品来源元数据摘要不匹配",
                ));
            }
            Some(
                serde_json::from_slice(&bytes).map_err(|_| {
                    AppError::new("RECOVERY_ARTIFACT_INVALID", "制品来源元数据损坏")
                })?,
            )
        }
        None => match std::fs::symlink_metadata(&source_path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Ok(_) => {
                return Err(AppError::new(
                    "RECOVERY_ARTIFACT_INVALID",
                    "制品包含未绑定的来源元数据",
                ))
            }
            Err(_) => {
                return Err(AppError::new(
                    "RECOVERY_ARTIFACT_INVALID",
                    "无法检查制品来源元数据",
                ))
            }
        },
    };
    if expected_context.is_some() && entry.replay_context.is_none() {
        return Err(AppError::new("RECOVERY_ARTIFACT_INVALID", "恢复上下文缺失"));
    }
    Ok(LoadedArtifact {
        root,
        manifest,
        source_metadata,
    })
}

fn remove_owned_artifact<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    operation_id: &str,
    expected_digest: Option<&str>,
) -> AppResult<()> {
    let root = artifact_root(app)?.join(operation_id);
    if !exists_or_is_link(&root) {
        return Ok(());
    }
    let metadata = std::fs::symlink_metadata(&root)
        .map_err(|_| AppError::new("RECOVERY_ARTIFACT_INVALID", "无法检查恢复制品目录"))?;
    if metadata.file_type().is_symlink() || is_symlink_or_junction(&root) || !metadata.is_dir() {
        return Err(AppError::new(
            "RECOVERY_ARTIFACT_INVALID",
            "恢复制品目录不安全",
        ));
    }
    let owner_path = root.join(ARTIFACT_OWNER_FILE);
    if !exists_or_is_link(&owner_path) {
        let mut entries = std::fs::read_dir(&root)
            .map_err(|_| AppError::new("RECOVERY_ARTIFACT_INVALID", "无法检查恢复制品目录"))?;
        if entries.next().is_none() {
            std::fs::remove_dir(&root).map_err(|_| {
                AppError::new("RECOVERY_ARTIFACT_INVALID", "无法清理空恢复制品目录")
            })?;
            return Ok(());
        }
        return Err(AppError::new(
            "RECOVERY_ARTIFACT_INVALID",
            "恢复制品缺少所有权标记",
        ));
    }
    let owner = read_file_with_max_len(&owner_path, 128)?;
    if owner != operation_id.as_bytes() {
        return Err(AppError::new(
            "RECOVERY_ARTIFACT_INVALID",
            "恢复制品所有权不匹配",
        ));
    }
    let entry = JournalEntry {
        operation_id: operation_id.to_string(),
        parent_operation_id: None,
        operation_kind: "skill.artifact".to_string(),
        cli_key: None,
        workspace_id: None,
        entity_id: None,
        phase: "cleanup_pending".to_string(),
        status: "committed".to_string(),
        artifact_ref: Some(operation_id.to_string()),
        artifact_sha256: expected_digest.map(ToString::to_string),
        replay_context: None,
    };
    if root.join(ARTIFACT_MANIFEST_FILE).exists() {
        if expected_digest.is_some() {
            let _ = load_artifact(app, &entry, None)?;
        }
    } else if expected_digest.is_some() {
        return Err(AppError::new(
            "RECOVERY_ARTIFACT_INVALID",
            "恢复制品清单缺失",
        ));
    }
    validate_artifact_tree_for_removal(&root)?;
    std::fs::remove_dir_all(&root)
        .map_err(|error| format!("failed to remove artifact {}: {error}", root.display()))?;
    Ok(())
}

fn validate_artifact_tree_for_removal(path: &Path) -> AppResult<()> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|_| AppError::new("RECOVERY_ARTIFACT_INVALID", "无法检查恢复制品内容"))?;
    if metadata.file_type().is_symlink() || is_symlink_or_junction(path) {
        return Err(AppError::new(
            "RECOVERY_ARTIFACT_INVALID",
            "恢复制品包含不安全链接",
        ));
    }
    if metadata.is_file() {
        return Ok(());
    }
    if !metadata.is_dir() {
        return Err(AppError::new(
            "RECOVERY_ARTIFACT_INVALID",
            "恢复制品包含特殊文件",
        ));
    }
    for entry in std::fs::read_dir(path)
        .map_err(|_| AppError::new("RECOVERY_ARTIFACT_INVALID", "无法枚举恢复制品"))?
    {
        let entry = entry
            .map_err(|_| AppError::new("RECOVERY_ARTIFACT_INVALID", "无法检查恢复制品条目"))?;
        validate_artifact_tree_for_removal(&entry.path())?;
    }
    Ok(())
}

fn role_dir(artifact: &LoadedArtifact, role: &str) -> AppResult<PathBuf> {
    let role = artifact
        .manifest
        .roles
        .get(role)
        .ok_or_else(|| AppError::new("RECOVERY_ARTIFACT_INVALID", "恢复制品缺少内容角色"))?;
    validate_artifact_role(role, &artifact.root)
}

fn role_hash<'a>(artifact: &'a LoadedArtifact, role: &str) -> AppResult<&'a str> {
    artifact
        .manifest
        .roles
        .get(role)
        .map(|role| role.content_hash.as_str())
        .ok_or_else(|| AppError::new("RECOVERY_ARTIFACT_INVALID", "恢复制品缺少内容角色"))
}

fn remove_directory_if_hash_matches(path: &Path, expected_hash: &str) -> AppResult<()> {
    if !exists_or_is_link(path) {
        return Ok(());
    }
    if is_symlink_or_junction(path) {
        return Err(format!("RECOVERY_PROJECTION_UNSAFE_TARGET: {}", path.display()).into());
    }
    if !path.is_dir() || skill_dir_content_hash(path)? != expected_hash {
        return Err(format!("RECOVERY_PROJECTION_CONFLICT: {}", path.display()).into());
    }
    std::fs::remove_dir_all(path)
        .map_err(|error| format!("failed to remove {}: {error}", path.display()))?;
    Ok(())
}

fn project_ssot_from_role<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    skill_key: &str,
    artifact: &LoadedArtifact,
    role: &str,
    replace_hash: Option<&str>,
) -> AppResult<()> {
    let ssot_dir = ssot_skills_root(app)?.join(skill_key);
    let desired_hash = role_hash(artifact, role)?;
    if exists_or_is_link(&ssot_dir) {
        if !is_symlink_or_junction(&ssot_dir)
            && ssot_dir.is_dir()
            && skill_dir_content_hash(&ssot_dir)? == desired_hash
        {
            return Ok(());
        }
        let Some(previous_hash) = replace_hash else {
            return Err(format!("RECOVERY_PROJECTION_CONFLICT: {}", ssot_dir.display()).into());
        };
        remove_directory_if_hash_matches(&ssot_dir, previous_hash)?;
    }
    copy_dir_recursive(&role_dir(artifact, role)?, &ssot_dir)
}

fn sync_all_skills<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    conn: &rusqlite::Connection,
) -> AppResult<()> {
    for cli_key in
        crate::shared::cli_key::cli_keys_with(crate::shared::cli_key::CliCapability::Skills)
    {
        sync_one_cli(app, conn, cli_key)?;
    }
    Ok(())
}

fn context_for_entry(entry: &JournalEntry) -> AppResult<SkillRecoveryContext> {
    let Some(raw) = entry.replay_context.as_deref() else {
        return Err(AppError::new(
            "RECOVERY_ARTIFACT_INVALID",
            "Skill 恢复上下文缺失",
        ));
    };
    let context: SkillRecoveryContext = serde_json::from_str(raw)
        .map_err(|_| AppError::new("RECOVERY_ARTIFACT_INVALID", "Skill 恢复上下文损坏"))?;
    context.validate_path_components()?;
    if context.operation_kind() != entry.operation_kind {
        return Err(AppError::new(
            "RECOVERY_ARTIFACT_INVALID",
            "Skill 恢复操作类型不匹配",
        ));
    }
    if !context.matches_journal_scope(entry) {
        return Err(AppError::new(
            "RECOVERY_ARTIFACT_INVALID",
            "Skill 恢复上下文范围不匹配",
        ));
    }
    Ok(context)
}

fn installed_hash_for_replay<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    conn: &rusqlite::Connection,
    expected_skill_id: Option<i64>,
    skill_key: &str,
    accepted_hashes: &[&str],
) -> AppResult<Option<String>> {
    let stored: Option<Option<String>> = match expected_skill_id {
        Some(skill_id) => conn
            .query_row(
                "SELECT installed_content_hash FROM skills WHERE id = ?1 AND skill_key = ?2",
                rusqlite::params![skill_id, skill_key],
                |row| row.get(0),
            )
            .optional(),
        None => conn
            .query_row(
                "SELECT installed_content_hash FROM skills WHERE skill_key = ?1",
                [skill_key],
                |row| row.get(0),
            )
            .optional(),
    }
    .map_err(|_| AppError::new("RECOVERY_JOURNAL_DB_FAILED", "无法读取 Skill 内容摘要"))?;
    let Some(stored) = stored else {
        if let Some(expected_skill_id) = expected_skill_id {
            let current_skill_id = conn
                .query_row(
                    "SELECT id FROM skills WHERE skill_key = ?1",
                    [skill_key],
                    |row| row.get::<_, i64>(0),
                )
                .optional()
                .map_err(|_| {
                    AppError::new("RECOVERY_JOURNAL_DB_FAILED", "无法核对 Skill 恢复实体")
                })?;
            if current_skill_id.is_some_and(|value| value != expected_skill_id) {
                return Err(AppError::new(
                    "RECOVERY_JOURNAL_STATE_CONFLICT",
                    "Skill 标识已被其他实体占用",
                ));
            }
        }
        return Ok(None);
    };
    match stored {
        Some(hash) => Ok(Some(hash)),
        None => {
            let ssot_dir = ssot_skills_root(app)?.join(skill_key);
            if is_symlink_or_junction(&ssot_dir) || !ssot_dir.is_dir() {
                return Err(AppError::new(
                    "RECOVERY_ARTIFACT_INVALID",
                    "旧 Skill 的 SSOT 目录不安全",
                ));
            }
            let actual = skill_dir_content_hash(&ssot_dir)?;
            if !accepted_hashes.contains(&actual.as_str()) {
                return Err(AppError::new(
                    "RECOVERY_ARTIFACT_INVALID",
                    "旧 Skill 内容无法与恢复制品核对",
                ));
            }
            let changed = match expected_skill_id {
                Some(skill_id) => conn.execute(
                    "UPDATE skills SET installed_content_hash = ?1 WHERE id = ?2 AND skill_key = ?3 AND installed_content_hash IS NULL",
                    rusqlite::params![actual, skill_id, skill_key],
                ),
                None => conn.execute(
                    "UPDATE skills SET installed_content_hash = ?1 WHERE skill_key = ?2 AND installed_content_hash IS NULL",
                    rusqlite::params![actual, skill_key],
                ),
            }
                .map_err(|_| {
                    AppError::new("RECOVERY_JOURNAL_DB_FAILED", "无法回填 Skill 内容摘要")
                })?;
            if changed != 1 {
                return Err(AppError::new(
                    "RECOVERY_JOURNAL_STATE_CONFLICT",
                    "Skill 内容摘要已被其他操作更新",
                ));
            }
            Ok(Some(actual))
        }
    }
}

fn replay_install<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    conn: &rusqlite::Connection,
    context: &SkillRecoveryContext,
    artifact: &LoadedArtifact,
) -> AppResult<()> {
    let SkillRecoveryContext::Install {
        workspace_id,
        skill_key,
    } = context
    else {
        return Err(AppError::new(
            "RECOVERY_ARTIFACT_INVALID",
            "Skill 安装恢复上下文不匹配",
        ));
    };
    let desired_hash = role_hash(artifact, "desired")?;
    let Some(installed_hash) =
        installed_hash_for_replay(app, conn, None, skill_key, &[desired_hash])?
    else {
        return Ok(());
    };
    if installed_hash != desired_hash {
        return Err(AppError::new(
            "RECOVERY_ARTIFACT_INVALID",
            "Skill 安装内容与 SQLite 不一致",
        ));
    }
    project_ssot_from_role(app, skill_key, artifact, "desired", None)?;
    let _ = workspace_id;
    sync_all_skills(app, conn)
}

fn replay_import_local<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    conn: &rusqlite::Connection,
    context: &SkillRecoveryContext,
    artifact: &LoadedArtifact,
) -> AppResult<()> {
    let SkillRecoveryContext::ImportLocal {
        workspace_id,
        cli_key,
        skill_key,
        local_dir_name,
    } = context
    else {
        return Err(AppError::new(
            "RECOVERY_ARTIFACT_INVALID",
            "本地导入恢复上下文不匹配",
        ));
    };
    let desired_hash = role_hash(artifact, "desired")?;
    let Some(installed_hash) =
        installed_hash_for_replay(app, conn, None, skill_key, &[desired_hash])?
    else {
        return Ok(());
    };
    if installed_hash != desired_hash {
        return Err(AppError::new(
            "RECOVERY_ARTIFACT_INVALID",
            "Skill 导入内容与 SQLite 不一致",
        ));
    }
    project_ssot_from_role(app, skill_key, artifact, "desired", None)?;
    let local_dir = cli_skills_root(app, cli_key)?.join(local_dir_name);
    if exists_or_is_link(&local_dir)
        && !is_managed_link_to_ssot(&local_dir, &ssot_skills_root(app)?)
    {
        if is_symlink_or_junction(&local_dir)
            || !local_dir.is_dir()
            || skill_dir_content_hash(&local_dir)? != role_hash(artifact, "desired")?
        {
            return Err(format!("RECOVERY_PROJECTION_CONFLICT: {}", local_dir.display()).into());
        }
        super::fs_ops::write_marker(&local_dir)?;
    }
    let _ = workspace_id;
    sync_all_skills(app, conn)
}

fn replay_update<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    conn: &rusqlite::Connection,
    context: &SkillRecoveryContext,
    artifact: &LoadedArtifact,
) -> AppResult<()> {
    let SkillRecoveryContext::Update {
        workspace_id,
        skill_id,
        skill_key,
    } = context
    else {
        return Err(AppError::new(
            "RECOVERY_ARTIFACT_INVALID",
            "Skill 更新恢复上下文不匹配",
        ));
    };
    let desired_hash = role_hash(artifact, "desired")?;
    let previous_hash = role_hash(artifact, "previous")?;
    let Some(hash) = installed_hash_for_replay(
        app,
        conn,
        Some(*skill_id),
        skill_key,
        &[desired_hash, previous_hash],
    )?
    else {
        let ssot_dir = ssot_skills_root(app)?.join(skill_key);
        if exists_or_is_link(&ssot_dir) {
            if is_symlink_or_junction(&ssot_dir) || !ssot_dir.is_dir() {
                return Err(
                    format!("RECOVERY_PROJECTION_UNSAFE_TARGET: {}", ssot_dir.display()).into(),
                );
            }
            let actual_hash = skill_dir_content_hash(&ssot_dir)?;
            if actual_hash != desired_hash && actual_hash != previous_hash {
                return Err(format!("RECOVERY_PROJECTION_CONFLICT: {}", ssot_dir.display()).into());
            }
            remove_directory_if_hash_matches(&ssot_dir, &actual_hash)?;
        }
        return sync_all_skills(app, conn);
    };
    if hash == desired_hash {
        project_ssot_from_role(app, skill_key, artifact, "desired", Some(previous_hash))?;
        let _ = workspace_id;
        return sync_all_skills(app, conn);
    }
    if hash == previous_hash {
        return project_ssot_from_role(app, skill_key, artifact, "previous", Some(desired_hash));
    }
    Err(AppError::new(
        "RECOVERY_ARTIFACT_INVALID",
        "Skill 更新状态与制品不一致",
    ))
}

fn replay_uninstall<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    conn: &rusqlite::Connection,
    context: &SkillRecoveryContext,
    artifact: &LoadedArtifact,
) -> AppResult<()> {
    let SkillRecoveryContext::Uninstall {
        skill_id,
        skill_key,
    } = context
    else {
        return Err(AppError::new(
            "RECOVERY_ARTIFACT_INVALID",
            "Skill 卸载恢复上下文不匹配",
        ));
    };
    let previous_hash = role_hash(artifact, "previous")?;
    let installed_hash =
        installed_hash_for_replay(app, conn, Some(*skill_id), skill_key, &[previous_hash])?;
    if let Some(installed_hash) = installed_hash {
        if installed_hash != previous_hash {
            return Err(AppError::new(
                "RECOVERY_ARTIFACT_INVALID",
                "Skill 卸载前态与 SQLite 不一致",
            ));
        }
        return project_ssot_from_role(app, skill_key, artifact, "previous", None);
    }
    for cli_key in
        crate::shared::cli_key::cli_keys_with(crate::shared::cli_key::CliCapability::Skills)
    {
        remove_from_cli(app, cli_key, skill_key)?;
    }
    remove_directory_if_hash_matches(
        &ssot_skills_root(app)?.join(skill_key),
        role_hash(artifact, "previous")?,
    )
}

fn replay_return_to_local<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    conn: &rusqlite::Connection,
    context: &SkillRecoveryContext,
    artifact: &LoadedArtifact,
) -> AppResult<()> {
    let SkillRecoveryContext::ReturnToLocal {
        workspace_id,
        cli_key,
        skill_id,
        skill_key,
    } = context
    else {
        return Err(AppError::new(
            "RECOVERY_ARTIFACT_INVALID",
            "返回本地恢复上下文不匹配",
        ));
    };
    let previous_hash = role_hash(artifact, "previous")?;
    let installed_hash =
        installed_hash_for_replay(app, conn, Some(*skill_id), skill_key, &[previous_hash])?;
    if let Some(installed_hash) = installed_hash {
        if installed_hash != previous_hash {
            return Err(AppError::new(
                "RECOVERY_ARTIFACT_INVALID",
                "返回本地前态与 SQLite 不一致",
            ));
        }
        return project_ssot_from_role(app, skill_key, artifact, "previous", None);
    }
    let local_target = cli_skills_root(app, cli_key)?.join(skill_key);
    if exists_or_is_link(&local_target) {
        if is_managed_dir(&local_target)
            || is_managed_link_to_ssot(&local_target, &ssot_skills_root(app)?)
        {
            remove_managed_dir(&local_target)?;
        } else if is_symlink_or_junction(&local_target)
            || !local_target.is_dir()
            || skill_dir_content_hash(&local_target)? != role_hash(artifact, "previous")?
        {
            return Err(format!("RECOVERY_PROJECTION_CONFLICT: {}", local_target.display()).into());
        }
    }
    if !exists_or_is_link(&local_target) {
        copy_dir_recursive(&role_dir(artifact, "previous")?, &local_target)?;
    }
    if let Some(metadata) = artifact.source_metadata.as_ref() {
        write_source_metadata(&local_target, metadata)?;
    }
    remove_marker(&local_target)?;
    remove_managed_targets_except(app, skill_key, &local_target)?;
    remove_directory_if_hash_matches(
        &ssot_skills_root(app)?.join(skill_key),
        role_hash(artifact, "previous")?,
    )?;
    let _ = workspace_id;
    Ok(())
}

fn replay_install_to_local<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    context: &SkillRecoveryContext,
    artifact: &LoadedArtifact,
) -> AppResult<()> {
    let SkillRecoveryContext::InstallToLocal {
        workspace_id,
        cli_key,
        dir_name,
    } = context
    else {
        return Err(AppError::new(
            "RECOVERY_ARTIFACT_INVALID",
            "本地安装恢复上下文不匹配",
        ));
    };
    let local_dir = cli_skills_root(app, cli_key)?.join(dir_name);
    if exists_or_is_link(&local_dir) {
        if is_symlink_or_junction(&local_dir)
            || !local_dir.is_dir()
            || skill_dir_content_hash(&local_dir)? != role_hash(artifact, "desired")?
        {
            return Err(format!("RECOVERY_PROJECTION_CONFLICT: {}", local_dir.display()).into());
        }
    } else {
        copy_dir_recursive(&role_dir(artifact, "desired")?, &local_dir)?;
    }
    if let Some(metadata) = artifact.source_metadata.as_ref() {
        write_source_metadata(&local_dir, metadata)?;
    }
    let _ = workspace_id;
    Ok(())
}

fn replay_delete_local<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    context: &SkillRecoveryContext,
    artifact: &LoadedArtifact,
) -> AppResult<()> {
    let SkillRecoveryContext::DeleteLocal {
        workspace_id,
        cli_key,
        dir_name,
    } = context
    else {
        return Err(AppError::new(
            "RECOVERY_ARTIFACT_INVALID",
            "本地删除恢复上下文不匹配",
        ));
    };
    let local_dir = cli_skills_root(app, cli_key)?.join(dir_name);
    remove_directory_if_hash_matches(&local_dir, role_hash(artifact, "previous")?)?;
    let _ = workspace_id;
    Ok(())
}

pub(crate) fn replay_recovery_operation<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: &db::Db,
    operation: &RecoveryOperation,
) -> AppResult<()> {
    operation.renew_lease()?;
    let entry = operation.entry();
    if entry.replay_context.is_none() {
        let conn = db.open_connection()?;
        if let Some(cli_key) = entry.cli_key.as_deref() {
            return sync_one_cli(app, &conn, cli_key);
        }
        return sync_all_skills(app, &conn);
    }
    let context = context_for_entry(entry)?;
    let artifact = load_artifact(app, entry, Some(&context))?;
    let conn = db.open_connection()?;
    match &context {
        SkillRecoveryContext::Install { .. } => replay_install(app, &conn, &context, &artifact),
        SkillRecoveryContext::ImportLocal { .. } => {
            replay_import_local(app, &conn, &context, &artifact)
        }
        SkillRecoveryContext::Update { .. } => replay_update(app, &conn, &context, &artifact),
        SkillRecoveryContext::Uninstall { .. } => replay_uninstall(app, &conn, &context, &artifact),
        SkillRecoveryContext::ReturnToLocal { .. } => {
            replay_return_to_local(app, &conn, &context, &artifact)
        }
        SkillRecoveryContext::InstallToLocal { .. } => {
            replay_install_to_local(app, &context, &artifact)
        }
        SkillRecoveryContext::DeleteLocal { .. } => replay_delete_local(app, &context, &artifact),
    }
}

pub(crate) fn cleanup_recovery_operation<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    entry: &JournalEntry,
) -> AppResult<()> {
    let reference = entry.artifact_ref.as_deref().unwrap_or(&entry.operation_id);
    remove_owned_artifact(app, reference, entry.artifact_sha256.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;
    use std::ffi::OsString;
    use std::sync::MutexGuard;

    const OPERATION_ID: &str = "11111111-1111-4111-8111-111111111111";

    struct RecoveryTestApp {
        _lock: MutexGuard<'static, ()>,
        previous_test_home: Option<OsString>,
        _home: tempfile::TempDir,
        app: tauri::App<tauri::test::MockRuntime>,
        db: db::Db,
    }

    impl RecoveryTestApp {
        fn new() -> Self {
            let lock = crate::test_support::test_env_lock();
            let previous_test_home = std::env::var_os("AIO_CODING_HUB_TEST_HOME");
            let home = tempfile::tempdir().expect("home tempdir");
            std::env::set_var("AIO_CODING_HUB_TEST_HOME", home.path());
            crate::test_support::clear_settings_cache();
            let app = tauri::test::mock_app();
            let db =
                db::init_for_tests(&home.path().join("recovery.sqlite")).expect("init recovery db");
            Self {
                _lock: lock,
                previous_test_home,
                _home: home,
                app,
                db,
            }
        }

        fn handle(&self) -> tauri::AppHandle<tauri::test::MockRuntime> {
            self.app.handle().clone()
        }
    }

    impl Drop for RecoveryTestApp {
        fn drop(&mut self) {
            match self.previous_test_home.take() {
                Some(value) => std::env::set_var("AIO_CODING_HUB_TEST_HOME", value),
                None => std::env::remove_var("AIO_CODING_HUB_TEST_HOME"),
            }
            crate::test_support::clear_settings_cache();
        }
    }

    fn write_skill(path: &Path, name: &str) {
        std::fs::create_dir_all(path).expect("create skill dir");
        std::fs::write(path.join("SKILL.md"), format!("---\nname: {name}\n---\n"))
            .expect("write skill");
    }

    fn loaded_update_artifact(root: &Path) -> LoadedArtifact {
        let desired = root.join("desired");
        let previous = root.join("previous");
        write_skill(&desired, "Desired");
        write_skill(&previous, "Previous");
        let roles = BTreeMap::from([
            (
                "desired".to_string(),
                ArtifactRole {
                    relative_path: "desired".to_string(),
                    content_hash: skill_dir_content_hash(&desired).expect("hash desired"),
                },
            ),
            (
                "previous".to_string(),
                ArtifactRole {
                    relative_path: "previous".to_string(),
                    content_hash: skill_dir_content_hash(&previous).expect("hash previous"),
                },
            ),
        ]);
        LoadedArtifact {
            root: root.to_path_buf(),
            manifest: ArtifactManifest {
                schema_version: 1,
                operation_id: OPERATION_ID.to_string(),
                roles,
                source_sha256: None,
            },
            source_metadata: None,
        }
    }

    fn insert_legacy_skill(conn: &rusqlite::Connection, skill_id: i64, skill_key: &str) {
        conn.execute(
            r#"
INSERT INTO skills(
  id, skill_key, name, normalized_name, description, source_git_url,
  source_branch, source_subdir, installed_commit, installed_content_hash,
  created_at, updated_at
) VALUES (?1, ?2, 'Demo', 'demo', '', 'https://example.test/demo.git',
          'main', '.', NULL, NULL, 1, 1)
"#,
            params![skill_id, skill_key],
        )
        .expect("insert legacy skill");
    }

    fn write_owned_artifact(
        test: &RecoveryTestApp,
    ) -> (JournalEntry, SkillRecoveryContext, PathBuf) {
        let root = artifact_root(&test.handle())
            .expect("artifact root")
            .join(OPERATION_ID);
        std::fs::create_dir(&root).expect("create artifact dir");
        std::fs::write(root.join(ARTIFACT_OWNER_FILE), OPERATION_ID).expect("write owner");
        let desired = root.join("desired");
        write_skill(&desired, "Desired");
        let source_metadata = SkillSourceMetadata {
            source_git_url: "https://example.test/demo.git".to_string(),
            source_branch: "main".to_string(),
            source_subdir: ".".to_string(),
        };
        let source_bytes = serde_json::to_vec(&source_metadata).expect("serialize source");
        std::fs::write(root.join(ARTIFACT_SOURCE_FILE), &source_bytes).expect("write source");
        let roles = BTreeMap::from([(
            "desired".to_string(),
            ArtifactRole {
                relative_path: "desired".to_string(),
                content_hash: skill_dir_content_hash(&desired).expect("hash desired"),
            },
        )]);
        let manifest = ArtifactManifest {
            schema_version: 1,
            operation_id: OPERATION_ID.to_string(),
            roles,
            source_sha256: Some(sha256_hex(&source_bytes)),
        };
        let manifest_bytes = serde_json::to_vec(&manifest).expect("serialize manifest");
        std::fs::write(root.join(ARTIFACT_MANIFEST_FILE), &manifest_bytes).expect("write manifest");
        let context = SkillRecoveryContext::Install {
            workspace_id: 1,
            skill_key: "demo".to_string(),
        };
        let entry = JournalEntry {
            operation_id: OPERATION_ID.to_string(),
            parent_operation_id: None,
            operation_kind: "skill.install".to_string(),
            cli_key: None,
            workspace_id: Some(1),
            entity_id: None,
            phase: "committed".to_string(),
            status: "committed".to_string(),
            artifact_ref: Some(OPERATION_ID.to_string()),
            artifact_sha256: Some(sha256_hex(&manifest_bytes)),
            replay_context: Some(artifact_context_json(&context).expect("serialize context")),
        };
        (entry, context, root)
    }

    #[test]
    fn update_with_missing_entity_does_not_restore_previous_ssot() {
        let test = RecoveryTestApp::new();
        let artifact_dir = tempfile::tempdir().expect("artifact tempdir");
        let artifact = loaded_update_artifact(artifact_dir.path());
        let ssot_dir = ssot_skills_root(&test.handle())
            .expect("ssot root")
            .join("demo");
        write_skill(&ssot_dir, "Previous");
        let context = SkillRecoveryContext::Update {
            workspace_id: 1,
            skill_id: 42,
            skill_key: "demo".to_string(),
        };
        let conn = test.db.open_connection().expect("open db");

        replay_update(&test.handle(), &conn, &context, &artifact)
            .expect("reconcile missing entity");

        assert!(!ssot_dir.exists());
    }

    #[test]
    fn update_backfills_legacy_null_hash_before_projection() {
        let test = RecoveryTestApp::new();
        let artifact_dir = tempfile::tempdir().expect("artifact tempdir");
        let artifact = loaded_update_artifact(artifact_dir.path());
        let ssot_dir = ssot_skills_root(&test.handle())
            .expect("ssot root")
            .join("demo");
        write_skill(&ssot_dir, "Previous");
        let context = SkillRecoveryContext::Update {
            workspace_id: 1,
            skill_id: 42,
            skill_key: "demo".to_string(),
        };
        let conn = test.db.open_connection().expect("open db");
        insert_legacy_skill(&conn, 42, "demo");

        replay_update(&test.handle(), &conn, &context, &artifact).expect("replay legacy update");

        let stored: String = conn
            .query_row(
                "SELECT installed_content_hash FROM skills WHERE id = 42",
                [],
                |row| row.get(0),
            )
            .expect("read backfilled hash");
        assert_eq!(
            stored.as_str(),
            role_hash(&artifact, "previous").expect("previous hash")
        );
        assert_eq!(
            std::fs::read_to_string(ssot_dir.join("SKILL.md")).expect("read SSOT"),
            "---\nname: Previous\n---\n"
        );
    }

    #[test]
    fn recovery_context_rejects_mismatched_skill_entity() {
        let context = SkillRecoveryContext::Update {
            workspace_id: 7,
            skill_id: 41,
            skill_key: "demo".to_string(),
        };
        let entry = JournalEntry {
            operation_id: OPERATION_ID.to_string(),
            parent_operation_id: None,
            operation_kind: "skill.update".to_string(),
            cli_key: Some("codex".to_string()),
            workspace_id: Some(7),
            entity_id: Some(42),
            phase: "committed".to_string(),
            status: "committed".to_string(),
            artifact_ref: Some(OPERATION_ID.to_string()),
            artifact_sha256: Some("0".repeat(64)),
            replay_context: None,
        };

        assert!(!context.matches_journal_scope(&entry));
    }

    #[test]
    fn recovery_context_rejects_path_escape_components() {
        for context in [
            SkillRecoveryContext::Install {
                workspace_id: 1,
                skill_key: "../escape".to_string(),
            },
            SkillRecoveryContext::ImportLocal {
                workspace_id: 1,
                cli_key: "codex".to_string(),
                skill_key: "safe".to_string(),
                local_dir_name: "nested/name".to_string(),
            },
            SkillRecoveryContext::InstallToLocal {
                workspace_id: 1,
                cli_key: "codex".to_string(),
                dir_name: "nested\\name".to_string(),
            },
        ] {
            let error = context
                .validate_path_components()
                .expect_err("unsafe recovery path must fail");
            assert_eq!(error.code(), "RECOVERY_ARTIFACT_INVALID");
        }
    }

    #[test]
    fn artifact_manifest_tampering_is_rejected() {
        let test = RecoveryTestApp::new();
        let (entry, context, root) = write_owned_artifact(&test);
        std::fs::write(root.join(ARTIFACT_MANIFEST_FILE), b"{}").expect("tamper manifest");

        let error = load_artifact(&test.handle(), &entry, Some(&context))
            .expect_err("manifest tampering must fail");

        assert_eq!(error.code(), "RECOVERY_ARTIFACT_INVALID");
    }

    #[test]
    fn artifact_role_tampering_is_rejected() {
        let test = RecoveryTestApp::new();
        let (entry, context, root) = write_owned_artifact(&test);
        std::fs::write(root.join("desired").join("SKILL.md"), "tampered").expect("tamper role");

        let error = load_artifact(&test.handle(), &entry, Some(&context))
            .expect_err("role tampering must fail");

        assert_eq!(error.code(), "RECOVERY_ARTIFACT_INVALID");
    }

    #[test]
    fn artifact_source_tampering_is_rejected() {
        let test = RecoveryTestApp::new();
        let (entry, context, root) = write_owned_artifact(&test);
        std::fs::write(root.join(ARTIFACT_SOURCE_FILE), b"{}").expect("tamper source");

        let error = load_artifact(&test.handle(), &entry, Some(&context))
            .expect_err("source tampering must fail");

        assert_eq!(error.code(), "RECOVERY_ARTIFACT_INVALID");
    }

    #[test]
    fn cleanup_preserves_registered_artifact_after_content_tampering() {
        let test = RecoveryTestApp::new();
        let (entry, _context, root) = write_owned_artifact(&test);
        std::fs::write(root.join("desired").join("SKILL.md"), "tampered").expect("tamper role");

        let error = cleanup_recovery_operation(&test.handle(), &entry)
            .expect_err("tampered artifact cleanup must fail closed");

        assert_eq!(error.code(), "RECOVERY_ARTIFACT_INVALID");
        assert!(root.exists());
    }

    #[test]
    fn cleanup_removes_owned_artifact_without_registered_reference() {
        let test = RecoveryTestApp::new();
        let root = artifact_root(&test.handle())
            .expect("artifact root")
            .join(OPERATION_ID);
        std::fs::create_dir(&root).expect("create orphan artifact");
        std::fs::write(root.join(ARTIFACT_OWNER_FILE), OPERATION_ID).expect("write owner");
        std::fs::write(root.join("partial"), b"staged").expect("write partial artifact");
        let entry = JournalEntry {
            operation_id: OPERATION_ID.to_string(),
            parent_operation_id: None,
            operation_kind: "skill.install".to_string(),
            cli_key: None,
            workspace_id: Some(1),
            entity_id: None,
            phase: "cleanup_pending".to_string(),
            status: "committed".to_string(),
            artifact_ref: None,
            artifact_sha256: None,
            replay_context: None,
        };

        cleanup_recovery_operation(&test.handle(), &entry).expect("cleanup orphan artifact");

        assert!(!root.exists());
    }
}
