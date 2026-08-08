//! Usage: Strict Codex provider sync / backup / rollback core.

use crate::shared::error::AppResult;
use crate::shared::fs::{
    is_symlink, read_open_file_with_max_len, read_optional_file_with_max_len,
    write_file_atomic_if_changed,
};
use crate::shared::time::{now_unix_millis, now_unix_seconds};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{Read as _, Seek as _, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};

pub const PROVIDER_SYNC_LOCK_FILE: &str = "tmp/provider-sync.lock";
pub const PROVIDER_SYNC_BACKUP_ROOT: &str = "backups_state/provider-sync";
const PROVIDER_SYNC_MAX_BYTES: usize = 1024 * 1024;
const MANAGED_PROVIDER_AIO: &str = "aio";
const MANAGED_PROVIDER_OPENAI: &str = "OpenAI";
const PROVIDER_SYNC_MANAGED_BACKUP_MANIFEST: &str = "provider-sync.json";
const PROVIDER_SYNC_MANAGED_BY: &str = "Codex provider sync";
const PROVIDER_SYNC_BACKUP_VERSION: u8 = 2;
const PROVIDER_SYNC_BACKUP_SCOPE: &str = "active_sessions";
const CODEX_APP_RUNNING_OVERRIDE_NONE: u8 = 0;
const CODEX_APP_RUNNING_OVERRIDE_FALSE: u8 = 1;
const CODEX_APP_RUNNING_OVERRIDE_TRUE: u8 = 2;
const PROVIDER_SYNC_PRUNE_MAX_DEPTH: usize = 128;
const PROVIDER_SYNC_PRUNE_MAX_ENTRIES: usize = 100_000;
const PROVIDER_SYNC_PRUNE_MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
const PROVIDER_SYNC_PRUNE_MAX_TREE_HASHED_BYTES: u64 = 256 * 1024 * 1024;
// A single prune operation may enumerate the root, inspect a candidate, take
// one snapshot, validate it twice after isolation, and hash files at deletion
// boundaries. Keep that aggregate work bounded without shrinking the per-tree
// contract below 100,000 entries / 256 MiB.
const PROVIDER_SYNC_PRUNE_MAX_WORK_ENTRIES: usize = PROVIDER_SYNC_PRUNE_MAX_ENTRIES * 5;
const PROVIDER_SYNC_PRUNE_MAX_HASHED_BYTES: u64 =
    PROVIDER_SYNC_PRUNE_MAX_TREE_HASHED_BYTES * 6 + PROVIDER_SYNC_MAX_BYTES as u64 * 7;
const PROVIDER_SYNC_PRUNE_HASH_CHUNK_BYTES: usize = 64 * 1024;
const PROVIDER_SYNC_PRUNE_MAX_WARNINGS: usize = 32;

static CODEX_APP_RUNNING_OVERRIDE: AtomicU8 = AtomicU8::new(CODEX_APP_RUNNING_OVERRIDE_NONE);

fn codex_process_check_failed_message(command: &str, detail: impl AsRef<str>) -> String {
    format!(
        "CODEX_PROVIDER_SYNC_PROCESS_CHECK_FAILED: unable to verify whether Codex App is closed before syncing provider settings. Process check command `{command}` failed: {}. Please confirm Codex App is fully closed, then retry.",
        detail.as_ref()
    )
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub struct CodexProviderSyncResult {
    pub status: String,
    pub target_provider: String,
    pub trigger: String,
    pub backup_dir: Option<String>,
    pub changed_session_files: Vec<String>,
    pub sqlite_provider_rows_updated: usize,
    pub sqlite_user_event_rows_updated: usize,
    pub sqlite_cwd_rows_updated: usize,
    pub updated_workspace_roots: Vec<String>,
    pub warning: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CodexProviderSyncContext {
    pub trigger: String,
    pub target_provider: String,
    pub config_bytes: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
struct FileSnapshot {
    path: PathBuf,
    existed: bool,
    bytes: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
struct SyncChangeSet {
    config_bytes: Option<Vec<u8>>,
    session_changes: Vec<SessionChange>,
}

#[derive(Debug, Clone)]
struct SessionChange {
    path: PathBuf,
    original_text: Vec<u8>,
    next_text: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
struct BackupManifestV2 {
    version: u8,
    scope: &'static str,
    trigger: String,
    target_provider: String,
    created_at: String,
    managed_by: &'static str,
    config_path: Option<String>,
    session_files: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ManagedBackupVersion {
    V1,
    V2,
}

pub fn codex_provider_sync<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    context: CodexProviderSyncContext,
) -> AppResult<CodexProviderSyncResult> {
    let home = crate::codex_paths::codex_home_dir(app)?;
    let target_provider = resolve_target_provider(&context.target_provider)?;
    if codex_app_is_running()? {
        return Err("CODEX_PROVIDER_SYNC_PROCESS_RUNNING: Codex App is running".into());
    }
    let lock_path = home.join(PROVIDER_SYNC_LOCK_FILE);
    let _lock_guard = acquire_lock(&lock_path)?;

    if codex_app_is_running()? {
        return Err("CODEX_PROVIDER_SYNC_PROCESS_RUNNING: Codex App is running".into());
    }

    let config_path = crate::codex_paths::codex_config_toml_path(app)?;
    if config_path.exists() && is_symlink(&config_path)? {
        return Err(format!(
            "SEC_INVALID_INPUT: refusing to modify symlink path={}",
            config_path.display()
        )
        .into());
    }

    let current_config = read_optional_file_with_max_len(&config_path, PROVIDER_SYNC_MAX_BYTES)?;
    let current_config_text = optional_config_bytes_to_utf8(current_config)?;
    let _ = read_current_provider(&current_config_text)?;

    let change_set = build_change_set(&home, &context, &current_config_text)?;

    if change_set.session_changes.is_empty() && change_set.config_bytes.is_none() {
        return Ok(CodexProviderSyncResult {
            status: "up_to_date".to_string(),
            target_provider,
            trigger: context.trigger,
            backup_dir: None,
            changed_session_files: Vec::new(),
            sqlite_provider_rows_updated: 0,
            sqlite_user_event_rows_updated: 0,
            sqlite_cwd_rows_updated: 0,
            updated_workspace_roots: Vec::new(),
            warning: None,
        });
    }

    let backup_dir = create_backup(&home, &context, &change_set)?;
    apply_file_changes(&config_path, &change_set)?;

    let warning = match prune_managed_backups(&home, &backup_dir) {
        Ok(warning) => warning,
        Err(err) => Some(format!("provider sync backup prune failed: {err}")),
    };
    Ok(CodexProviderSyncResult {
        status: "synced".to_string(),
        target_provider,
        trigger: context.trigger,
        backup_dir: Some(backup_dir.to_string_lossy().to_string()),
        changed_session_files: change_set
            .session_changes
            .iter()
            .map(|change| change.path.to_string_lossy().to_string())
            .collect(),
        sqlite_provider_rows_updated: 0,
        sqlite_user_event_rows_updated: 0,
        sqlite_cwd_rows_updated: 0,
        updated_workspace_roots: Vec::new(),
        warning,
    })
}

pub fn codex_provider_sync_current<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    trigger: impl Into<String>,
) -> AppResult<CodexProviderSyncResult> {
    let config_path = crate::codex_paths::codex_config_toml_path(app)?;
    let current_config = read_optional_file_with_max_len(&config_path, PROVIDER_SYNC_MAX_BYTES)?;
    let current_config_text = optional_config_bytes_to_utf8(current_config)?;
    let target_provider = codex_provider_target_from_current_config_text(&current_config_text)?;
    codex_provider_sync(
        app,
        CodexProviderSyncContext {
            trigger: trigger.into(),
            target_provider,
            config_bytes: None,
        },
    )
}

pub fn codex_provider_sync_from_config_bytes<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    trigger: impl Into<String>,
    config_bytes: Vec<u8>,
) -> AppResult<CodexProviderSyncResult> {
    let config_text = String::from_utf8(config_bytes.clone())
        .map_err(|_| "SEC_INVALID_INPUT: codex config.toml must be valid UTF-8".to_string())?;
    let target_provider = codex_provider_target_from_config_text(&config_text)?;
    codex_provider_sync(
        app,
        CodexProviderSyncContext {
            trigger: trigger.into(),
            target_provider,
            config_bytes: Some(config_bytes),
        },
    )
}

pub fn codex_provider_target_from_config_text(config_text: &str) -> AppResult<String> {
    let current_provider = read_current_provider(config_text)?.ok_or_else(|| {
        "CODEX_PROVIDER_SYNC_INVALID_TARGET: unsupported provider target=(missing)".to_string()
    })?;
    resolve_target_provider(&current_provider)
}

pub(crate) fn codex_provider_target_from_patch_config_text(config_text: &str) -> AppResult<String> {
    match read_current_provider(config_text)? {
        Some(provider) => resolve_target_provider(&provider),
        None => Ok(MANAGED_PROVIDER_AIO.to_string()),
    }
}

pub fn codex_provider_target_from_current_config_text(config_text: &str) -> AppResult<String> {
    Ok(read_current_provider(config_text)?.unwrap_or_else(|| MANAGED_PROVIDER_AIO.to_string()))
}

fn resolve_target_provider(input: &str) -> AppResult<String> {
    let trimmed = input.trim();
    match trimmed {
        MANAGED_PROVIDER_AIO | MANAGED_PROVIDER_OPENAI => Ok(trimmed.to_string()),
        _ => Err(format!(
            "CODEX_PROVIDER_SYNC_INVALID_TARGET: unsupported provider target={trimmed}"
        )
        .into()),
    }
}

fn read_current_provider(text: &str) -> AppResult<Option<String>> {
    if text.trim().is_empty() {
        return Ok(None);
    }

    let value = toml::from_str::<toml::Value>(text)
        .map_err(|err| format!("CODEX_PROVIDER_SYNC_INVALID_CONFIG: invalid config.toml: {err}"))?;
    let provider = value
        .as_table()
        .and_then(|table| table.get("model_provider"))
        .and_then(toml::Value::as_str)
        .map(str::trim)
        .filter(|provider| !provider.is_empty())
        .map(ToString::to_string);
    Ok(provider)
}

fn optional_config_bytes_to_utf8(bytes: Option<Vec<u8>>) -> AppResult<String> {
    match bytes {
        Some(bytes) => String::from_utf8(bytes).map_err(|_| {
            "CODEX_PROVIDER_SYNC_INVALID_CONFIG: config.toml must be valid UTF-8".into()
        }),
        None => Ok(String::new()),
    }
}

fn acquire_lock(path: &Path) -> AppResult<LockGuard> {
    if path.exists() {
        return Err(format!("CODEX_PROVIDER_SYNC_LOCKED: {}", path.display()).into());
    }
    if let Some(parent) = path.parent() {
        ensure_safe_operational_dir(parent, "Codex provider sync lock parent")?;
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create lock dir {}: {e}", parent.display()))?;
    }
    fs::create_dir(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::AlreadyExists {
            format!("CODEX_PROVIDER_SYNC_LOCKED: {}", path.display())
        } else {
            format!("failed to acquire lock {}: {e}", path.display())
        }
    })?;
    fs::write(
        path.join("owner.json"),
        serde_json::json!({
            "pid": std::process::id(),
            "startedAt": now_unix_millis(),
        })
        .to_string(),
    )
    .map_err(|e| format!("failed to write lock owner {}: {e}", path.display()))?;
    Ok(LockGuard {
        path: path.to_path_buf(),
    })
}

struct LockGuard {
    path: PathBuf,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn codex_app_is_running() -> AppResult<bool> {
    match CODEX_APP_RUNNING_OVERRIDE.load(Ordering::SeqCst) {
        CODEX_APP_RUNNING_OVERRIDE_FALSE => return Ok(false),
        CODEX_APP_RUNNING_OVERRIDE_TRUE => return Ok(true),
        _ => {}
    }

    #[cfg(windows)]
    {
        let output = std::process::Command::new("tasklist")
            .args(["/FI", "IMAGENAME eq Codex.exe", "/NH"])
            .output()
            .map_err(|err| codex_process_check_failed_message("tasklist", err.to_string()))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let detail = if stderr.is_empty() {
                format!("exit status {}", output.status)
            } else {
                format!("exit status {}; stderr: {}", output.status, stderr)
            };
            return Err(codex_process_check_failed_message("tasklist", detail).into());
        }
        let text = String::from_utf8_lossy(&output.stdout).to_ascii_lowercase();
        Ok(text.contains("codex.exe"))
    }

    #[cfg(not(windows))]
    codex_app_is_running_from_ps()
}

#[cfg(not(windows))]
fn codex_app_is_running_from_ps() -> AppResult<bool> {
    let output = std::process::Command::new("ps")
        .args(["-axo", "comm="])
        .output()
        .map_err(|err| codex_process_check_failed_message("ps", err.to_string()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let detail = if stderr.is_empty() {
            format!("exit status {}", output.status)
        } else {
            format!("exit status {}; stderr: {}", output.status, stderr)
        };
        return Err(codex_process_check_failed_message("ps", detail).into());
    }

    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text.lines().any(process_name_is_codex_app))
}

#[cfg(not(windows))]
fn process_name_is_codex_app(name: &str) -> bool {
    let trimmed = name.trim();
    if trimmed == "Codex" || trimmed == "Codex.exe" {
        return true;
    }
    Path::new(trimmed)
        .file_stem()
        .and_then(|value| value.to_str())
        .is_some_and(|stem| stem == "Codex")
}

#[doc(hidden)]
pub(crate) fn set_codex_app_running_override_for_tests(running: Option<bool>) {
    let value = match running {
        Some(false) => CODEX_APP_RUNNING_OVERRIDE_FALSE,
        Some(true) => CODEX_APP_RUNNING_OVERRIDE_TRUE,
        None => CODEX_APP_RUNNING_OVERRIDE_NONE,
    };
    CODEX_APP_RUNNING_OVERRIDE.store(value, Ordering::SeqCst);
}

fn build_change_set(
    home: &Path,
    context: &CodexProviderSyncContext,
    current_config_text: &str,
) -> AppResult<SyncChangeSet> {
    let mut config_bytes = None;

    if let Some(bytes) = context.config_bytes.as_ref() {
        let next_config_text = String::from_utf8(bytes.clone())
            .map_err(|_| "SEC_INVALID_INPUT: codex config.toml must be valid UTF-8".to_string())?;
        ensure_within_codex_len(next_config_text.as_bytes(), "codex config.toml")?;
        if next_config_text != current_config_text {
            config_bytes = Some(next_config_text.into_bytes());
        }
    }

    let session_changes = collect_session_changes(home, &context.target_provider)?;

    Ok(SyncChangeSet {
        config_bytes,
        session_changes,
    })
}

fn ensure_within_codex_len(bytes: &[u8], label: &str) -> AppResult<()> {
    if bytes.len() > PROVIDER_SYNC_MAX_BYTES {
        return Err(format!(
            "SEC_INVALID_INPUT: {label} too large (max {PROVIDER_SYNC_MAX_BYTES} bytes)"
        )
        .into());
    }
    Ok(())
}

#[cfg(windows)]
fn normalize_path_for_prefix_match(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

fn candidate_within_codex_home(
    canonical_home: &Path,
    candidate: &Path,
    label: &str,
) -> AppResult<bool> {
    let Ok(canonical_candidate) = fs::canonicalize(candidate) else {
        return Ok(false);
    };

    #[cfg(windows)]
    {
        let candidate_s = normalize_path_for_prefix_match(&canonical_candidate);
        let home_s = normalize_path_for_prefix_match(canonical_home);
        if candidate_s == home_s || candidate_s.starts_with(&(home_s.clone() + "/")) {
            return Ok(true);
        }
    }

    #[cfg(not(windows))]
    {
        if canonical_candidate.starts_with(canonical_home) {
            return Ok(true);
        }
    }

    Err(format!(
        "SEC_INVALID_INPUT: {label} resolved outside Codex home path={}",
        candidate.display()
    )
    .into())
}

fn non_symlink_metadata(path: &Path, label: &str) -> AppResult<Option<fs::Metadata>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Err(format!(
                    "SEC_INVALID_INPUT: refusing to follow symlink {label} path={}",
                    path.display()
                )
                .into());
            }
            Ok(Some(metadata))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(format!(
            "failed to read metadata for {label} {}: {err}",
            path.display()
        )
        .into()),
    }
}

fn ensure_safe_operational_dir(path: &Path, label: &str) -> AppResult<()> {
    for ancestor in path.ancestors().collect::<Vec<_>>().into_iter().rev() {
        if !ancestor.exists() {
            continue;
        }
        let Some(metadata) = non_symlink_metadata(ancestor, label)? else {
            continue;
        };
        if !metadata.is_dir() {
            return Err(format!(
                "SEC_INVALID_INPUT: {label} is not a directory path={}",
                ancestor.display()
            )
            .into());
        }
    }
    Ok(())
}

fn collect_session_changes(home: &Path, target_provider: &str) -> AppResult<Vec<SessionChange>> {
    let mut changes = Vec::new();
    let canonical_home = fs::canonicalize(home)
        .map_err(|e| format!("failed to canonicalize Codex home {}: {e}", home.display()))?;
    let root = home.join("sessions");
    let Some(metadata) = non_symlink_metadata(&root, "Codex session root")? else {
        return Ok(changes);
    };
    if metadata.is_dir()
        && candidate_within_codex_home(&canonical_home, &root, "Codex session root")?
    {
        collect_rollout_changes(&canonical_home, &root, target_provider, &mut changes)?;
    }
    Ok(changes)
}

fn collect_rollout_changes(
    canonical_home: &Path,
    root: &Path,
    target_provider: &str,
    out: &mut Vec<SessionChange>,
) -> AppResult<()> {
    let Some(metadata) = non_symlink_metadata(root, "Codex session root")? else {
        return Ok(());
    };
    if !metadata.is_dir() {
        return Ok(());
    }
    if !candidate_within_codex_home(canonical_home, root, "Codex session root")? {
        return Ok(());
    }
    for entry in
        fs::read_dir(root).map_err(|e| format!("failed to read {}: {e}", root.display()))?
    {
        let entry =
            entry.map_err(|e| format!("failed to read dir entry {}: {e}", root.display()))?;
        let path = entry.path();
        let Some(metadata) = non_symlink_metadata(&path, "Codex session entry")? else {
            continue;
        };
        if !candidate_within_codex_home(canonical_home, &path, "Codex session entry")? {
            continue;
        }
        if metadata.is_dir() {
            collect_rollout_changes(canonical_home, &path, target_provider, out)?;
            continue;
        }
        if !metadata.is_file() {
            continue;
        }
        if !path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("rollout-") && name.ends_with(".jsonl"))
        {
            continue;
        }
        let bytes = fs::read(&path)
            .map_err(|e| format!("failed to read rollout file {}: {e}", path.display()))?;
        let next_bytes = rewrite_rollout_session_meta_providers(&bytes, target_provider)?;
        if next_bytes != bytes {
            out.push(SessionChange {
                path,
                original_text: bytes,
                next_text: next_bytes,
            });
        }
    }
    Ok(())
}

fn rewrite_rollout_session_meta_providers(
    bytes: &[u8],
    target_provider: &str,
) -> AppResult<Vec<u8>> {
    let text = String::from_utf8(bytes.to_vec())
        .map_err(|_| "SEC_INVALID_INPUT: rollout jsonl must be valid UTF-8".to_string())?;
    let mut out = String::with_capacity(text.len());
    for segment in text.split_inclusive('\n') {
        let (line, ending) = split_line_ending(segment);
        let next_line = match serde_json::from_str::<Value>(line) {
            Ok(mut value) if value.get("type").and_then(Value::as_str) == Some("session_meta") => {
                if let Some(payload) = value.get_mut("payload").and_then(Value::as_object_mut) {
                    payload.insert(
                        "model_provider".to_string(),
                        Value::String(target_provider.to_string()),
                    );
                    serde_json::to_string(&value)
                        .map_err(|e| format!("failed to rewrite rollout row: {e}"))?
                } else {
                    line.to_string()
                }
            }
            _ => line.to_string(),
        };
        out.push_str(&next_line);
        out.push_str(ending);
    }
    Ok(out.into_bytes())
}

fn split_line_ending(segment: &str) -> (&str, &str) {
    if let Some(line) = segment.strip_suffix("\r\n") {
        (line, "\r\n")
    } else if let Some(line) = segment.strip_suffix('\n') {
        (line, "\n")
    } else {
        (segment, "")
    }
}

fn create_backup(
    home: &Path,
    context: &CodexProviderSyncContext,
    change_set: &SyncChangeSet,
) -> AppResult<PathBuf> {
    let root = home.join(PROVIDER_SYNC_BACKUP_ROOT);
    ensure_safe_operational_dir(&root, "Codex provider sync backup root")?;
    fs::create_dir_all(&root)
        .map_err(|e| format!("failed to create backup root {}: {e}", root.display()))?;
    let mut backup_dir = root.join(format!("{}-{}", now_unix_seconds(), std::process::id()));
    let mut suffix = 0usize;
    while backup_dir.exists() {
        suffix += 1;
        backup_dir = root.join(format!(
            "{}-{}-{suffix}",
            now_unix_seconds(),
            std::process::id()
        ));
    }
    fs::create_dir_all(&backup_dir)
        .map_err(|e| format!("failed to create backup dir {}: {e}", backup_dir.display()))?;

    let mut manifest = BackupManifestV2 {
        version: PROVIDER_SYNC_BACKUP_VERSION,
        scope: PROVIDER_SYNC_BACKUP_SCOPE,
        trigger: context.trigger.clone(),
        target_provider: context.target_provider.clone(),
        created_at: now_unix_millis().to_string(),
        managed_by: PROVIDER_SYNC_MANAGED_BY,
        config_path: None,
        session_files: Vec::new(),
    };

    let config_path = home.join("config.toml");
    if change_set.config_bytes.is_some() {
        if let Some(metadata) =
            non_symlink_metadata(&config_path, "Codex config.toml backup source")?
        {
            if !metadata.is_file() {
                return Err(format!(
                    "SEC_INVALID_INPUT: Codex config.toml backup source is not a file path={}",
                    config_path.display()
                )
                .into());
            }
            let target = backup_dir.join("config.toml");
            fs::copy(&config_path, &target)
                .map_err(|e| format!("failed to backup {}: {e}", config_path.display()))?;
            manifest.config_path = Some(target.to_string_lossy().to_string());
        }
    }

    for change in &change_set.session_changes {
        let relative = change.path.strip_prefix(home).map_err(|_| {
            format!(
                "SEC_INVALID_INPUT: session backup source is outside Codex home path={}",
                change.path.display()
            )
        })?;
        let target = backup_dir.join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create backup parent {}: {e}", parent.display()))?;
        }
        fs::write(&target, &change.original_text).map_err(|e| {
            format!(
                "failed to backup session file {}: {e}",
                change.path.display()
            )
        })?;
        manifest
            .session_files
            .push(target.to_string_lossy().to_string());
    }

    fs::write(
        backup_dir.join(PROVIDER_SYNC_MANAGED_BACKUP_MANIFEST),
        serde_json::to_vec_pretty(&manifest)
            .map_err(|e| format!("failed to serialize backup manifest: {e}"))?,
    )
    .map_err(|e| format!("failed to write backup manifest: {e}"))?;

    Ok(backup_dir)
}

fn apply_file_changes(config_path: &Path, change_set: &SyncChangeSet) -> AppResult<()> {
    apply_file_changes_with(config_path, change_set, write_file_atomic_if_changed)
}

fn apply_file_changes_with<F>(
    config_path: &Path,
    change_set: &SyncChangeSet,
    mut writer: F,
) -> AppResult<()>
where
    F: FnMut(&Path, &[u8]) -> AppResult<bool>,
{
    let mut snapshots = snapshot_paths(config_path, change_set)?;
    let mut writes_started = false;
    let result = (|| -> AppResult<()> {
        if let Some(bytes) = change_set.config_bytes.as_ref() {
            writes_started = true;
            let _ = writer(config_path, bytes)?;
        }
        for change in &change_set.session_changes {
            writes_started = true;
            let _ = writer(&change.path, &change.next_text)?;
        }
        Ok(())
    })();

    if let Err(err) = result {
        if writes_started {
            if let Err(rollback_err) = restore_snapshots(&mut snapshots) {
                return Err(format!(
                    "CODEX_PROVIDER_SYNC_ROLLBACK_FAILED: failed to restore snapshots after {err}; rollback error: {rollback_err}"
                )
                .into());
            }
        }
        return Err(err);
    }
    Ok(())
}

fn snapshot_paths(config_path: &Path, change_set: &SyncChangeSet) -> AppResult<Vec<FileSnapshot>> {
    let mut snapshots = Vec::new();
    if change_set.config_bytes.is_some() {
        snapshots.push(snapshot_path(config_path)?);
    }
    for change in &change_set.session_changes {
        snapshots.push(snapshot_path(&change.path)?);
    }
    Ok(snapshots)
}

fn snapshot_path(path: &Path) -> AppResult<FileSnapshot> {
    let Some(metadata) = non_symlink_metadata(path, "Codex provider sync snapshot")? else {
        return Ok(FileSnapshot {
            path: path.to_path_buf(),
            existed: false,
            bytes: None,
        });
    };
    if !metadata.is_file() {
        return Err(format!(
            "SEC_INVALID_INPUT: snapshot target is not a file path={}",
            path.display()
        )
        .into());
    };
    let bytes =
        fs::read(path).map_err(|e| format!("failed to snapshot {}: {e}", path.display()))?;
    Ok(FileSnapshot {
        path: path.to_path_buf(),
        existed: true,
        bytes: Some(bytes),
    })
}

fn restore_snapshots(snapshots: &mut [FileSnapshot]) -> AppResult<()> {
    for snapshot in snapshots.iter().rev() {
        if snapshot.existed {
            if let Some(bytes) = snapshot.bytes.as_ref() {
                fs::write(&snapshot.path, bytes)
                    .map_err(|e| format!("failed to restore {}: {e}", snapshot.path.display()))?;
            }
        } else if snapshot.path.exists() {
            fs::remove_file(&snapshot.path).map_err(|e| {
                format!("failed to remove restored {}: {e}", snapshot.path.display())
            })?;
        }
    }
    Ok(())
}

fn prune_managed_backups(home: &Path, current_backup: &Path) -> AppResult<Option<String>> {
    let mut budget = ProviderSyncPruneBudget::default();
    prune_managed_backups_with_budget(home, current_backup, &mut budget)
}

fn prune_managed_backups_with_budget(
    home: &Path,
    current_backup: &Path,
    budget: &mut ProviderSyncPruneBudget,
) -> AppResult<Option<String>> {
    let root = home.join(PROVIDER_SYNC_BACKUP_ROOT);
    let Some(root_metadata) = non_symlink_metadata(&root, "Codex provider sync backup root")?
    else {
        return Ok(None);
    };
    if !root_metadata.is_dir() {
        return Err(format!(
            "SEC_INVALID_INPUT: Codex provider sync backup root is not a directory path={}",
            root.display()
        )
        .into());
    }
    let root_handle = open_provider_sync_backup_dir_no_follow(&root)?;

    #[cfg(test)]
    run_after_provider_sync_backup_root_open_test_hook();

    if current_backup.parent() != Some(root.as_path()) {
        return Err(format!(
            "SEC_INVALID_INPUT: current provider sync backup is outside backup root path={}",
            current_backup.display()
        )
        .into());
    }
    let current_name = current_backup.file_name().ok_or_else(|| {
        "SEC_INVALID_INPUT: invalid current provider sync backup name".to_string()
    })?;
    let mut warnings = Vec::new();
    let mut suppressed_warnings = 0usize;
    let mut root_enumeration_budget = ProviderSyncPruneBudget::tree_limits();
    let names_result =
        provider_sync_backup_root_directory_names(&root_handle, &mut root_enumeration_budget);
    if let Err(err) = budget.consume(&root_enumeration_budget) {
        return Ok(Some(format!(
            "provider sync backup prune preserved all existing backups because root enumeration exhausted the prune budget: {err}"
        )));
    }
    let names = match names_result {
        Ok(names) => names,
        Err(err) => {
            return Ok(Some(format!(
                "provider sync backup prune preserved all existing backups because root enumeration failed closed: {err}"
            )));
        }
    };
    for name in names {
        if name.as_os_str() == current_name {
            continue;
        }
        if budget.is_exhausted() {
            push_provider_sync_prune_warning(
                &mut warnings,
                &mut suppressed_warnings,
                "provider sync backup prune budget exhausted; remaining backups were preserved"
                    .to_string(),
            );
            break;
        }
        let path = root.join(&name);
        let candidate = match open_provider_sync_backup_child_no_follow(&root_handle, &name, true) {
            Ok(candidate) => candidate,
            Err(err) => {
                push_provider_sync_prune_warning(
                    &mut warnings,
                    &mut suppressed_warnings,
                    provider_sync_backup_preserved_warning(
                        &path,
                        &format!("failed to bind candidate during classification: {err}"),
                    ),
                );
                continue;
            }
        };
        let mut classification_budget = ProviderSyncPruneBudget::tree_limits();
        let manifest_result =
            provider_sync_backup_dir_has_regular_manifest(&candidate, &mut classification_budget);
        if let Err(err) = budget.consume(&classification_budget) {
            push_provider_sync_prune_warning(
                &mut warnings,
                &mut suppressed_warnings,
                provider_sync_backup_preserved_warning(
                    &path,
                    &format!("candidate classification exhausted the prune budget: {err}"),
                ),
            );
            break;
        }
        match manifest_result {
            Ok(true) => {}
            Ok(false) => continue,
            Err(err) => {
                push_provider_sync_prune_warning(
                    &mut warnings,
                    &mut suppressed_warnings,
                    provider_sync_backup_preserved_warning(
                        &path,
                        &format!("failed to inspect candidate manifest entry: {err}"),
                    ),
                );
                if budget.is_exhausted() {
                    break;
                }
                continue;
            }
        }
        let version = match managed_backup_version_from_dir_handle(&candidate, budget) {
            Ok(Some(version)) => version,
            Ok(None) => continue,
            Err(err) => {
                push_provider_sync_prune_warning(
                    &mut warnings,
                    &mut suppressed_warnings,
                    provider_sync_backup_preserved_warning(
                        &path,
                        &format!("failed to classify candidate from trusted handle: {err}"),
                    ),
                );
                if budget.is_exhausted() {
                    break;
                }
                continue;
            }
        };
        if let Some(warning) =
            remove_managed_backup_candidate_with_root(&root, &root_handle, &path, version, budget)?
        {
            push_provider_sync_prune_warning(&mut warnings, &mut suppressed_warnings, warning);
        }
    }
    if suppressed_warnings > 0 {
        warnings.push(format!(
            "provider sync backup prune omitted {suppressed_warnings} additional warning(s)"
        ));
    }
    Ok((!warnings.is_empty()).then(|| warnings.join("; ")))
}

fn push_provider_sync_prune_warning(
    warnings: &mut Vec<String>,
    suppressed: &mut usize,
    warning: String,
) {
    if warnings.len() < PROVIDER_SYNC_PRUNE_MAX_WARNINGS {
        warnings.push(warning);
    } else {
        *suppressed = suppressed.saturating_add(1);
    }
}

#[cfg(test)]
fn managed_backup_version(path: &Path) -> AppResult<Option<ManagedBackupVersion>> {
    let root = path
        .parent()
        .ok_or_else(|| "SEC_INVALID_INPUT: managed backup has no parent directory".to_string())?;
    let name = path
        .file_name()
        .ok_or_else(|| "SEC_INVALID_INPUT: invalid managed backup name".to_string())?;
    let root_handle = open_provider_sync_backup_dir_no_follow(root)?;
    let candidate = open_provider_sync_backup_child_no_follow(&root_handle, name, true)?;
    let mut budget = ProviderSyncPruneBudget::default();
    if !provider_sync_backup_dir_has_regular_manifest(&candidate, &mut budget)? {
        return Ok(None);
    }
    managed_backup_version_from_dir_handle(&candidate, &mut budget)
}

fn managed_backup_version_from_bytes(bytes: &[u8]) -> Option<ManagedBackupVersion> {
    let Ok(manifest) = serde_json::from_slice::<Value>(bytes) else {
        return None;
    };
    if manifest.get("managed_by").and_then(Value::as_str) != Some(PROVIDER_SYNC_MANAGED_BY) {
        return None;
    }
    let created_at = manifest.get("created_at").and_then(Value::as_str)?;
    if created_at.parse::<u128>().is_err()
        || !manifest.get("trigger").is_some_and(Value::is_string)
        || !manifest
            .get("target_provider")
            .is_some_and(Value::is_string)
        || !manifest_path_field_is_valid(&manifest, "config_path")
        || !manifest_string_array_is_valid(&manifest, "session_files")
    {
        return None;
    }

    match manifest.get("version").and_then(Value::as_u64) {
        Some(1)
            if manifest_path_field_is_valid(&manifest, "global_state_path")
                && manifest_string_array_is_valid(&manifest, "sqlite_files") =>
        {
            Some(ManagedBackupVersion::V1)
        }
        Some(version) if version == u64::from(PROVIDER_SYNC_BACKUP_VERSION) => {
            if manifest.get("scope").and_then(Value::as_str) != Some(PROVIDER_SYNC_BACKUP_SCOPE)
                || manifest.get("sqlite_files").is_some()
                || manifest.get("global_state_path").is_some()
            {
                return None;
            }
            Some(ManagedBackupVersion::V2)
        }
        _ => None,
    }
}

fn manifest_path_field_is_valid(manifest: &Value, field: &str) -> bool {
    manifest
        .get(field)
        .is_some_and(|value| value.is_null() || value.is_string())
}

fn manifest_string_array_is_valid(manifest: &Value, field: &str) -> bool {
    manifest
        .get(field)
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().all(Value::is_string))
}

#[cfg(test)]
fn remove_managed_backup_candidate(
    root: &Path,
    path: &Path,
    expected_version: ManagedBackupVersion,
) -> AppResult<Option<String>> {
    let root_handle = open_provider_sync_backup_dir_no_follow(root)?;
    let mut budget = ProviderSyncPruneBudget::default();
    remove_managed_backup_candidate_with_root(
        root,
        &root_handle,
        path,
        expected_version,
        &mut budget,
    )
}

fn remove_managed_backup_candidate_with_root(
    root: &Path,
    root_handle: &std::fs::File,
    path: &Path,
    expected_version: ManagedBackupVersion,
    budget: &mut ProviderSyncPruneBudget,
) -> AppResult<Option<String>> {
    if path.parent() != Some(root) {
        return Err(format!(
            "SEC_INVALID_INPUT: managed backup candidate is outside backup root path={}",
            path.display()
        )
        .into());
    }
    let Some(original_name) = path.file_name() else {
        return Err("SEC_INVALID_INPUT: invalid managed backup candidate name".into());
    };
    let delete_handle =
        match open_provider_sync_backup_child_no_follow(root_handle, original_name, true) {
            Ok(handle) => handle,
            Err(err) => {
                return Ok(Some(provider_sync_backup_preserved_warning(
                    path,
                    &format!("failed to bind candidate directory: {err}"),
                )));
            }
        };
    let candidate_identity = provider_sync_file_identity_from_handle(&delete_handle)?;
    let validation_handle =
        match open_provider_sync_backup_child_no_follow(root_handle, original_name, true) {
            Ok(handle) => handle,
            Err(err) => {
                return Ok(Some(provider_sync_backup_preserved_warning(
                    path,
                    &format!("failed to reopen candidate directory: {err}"),
                )));
            }
        };
    if provider_sync_file_identity_from_handle(&validation_handle)? != candidate_identity {
        return Ok(Some(provider_sync_backup_preserved_warning(
            path,
            "candidate identity changed before validation",
        )));
    }
    let snapshot_result =
        validated_provider_sync_backup_snapshot(&validation_handle, expected_version, budget);
    let expected_snapshot = match snapshot_result {
        Ok(snapshot) => snapshot,
        Err(err) => {
            return Ok(Some(provider_sync_backup_preserved_warning(
                path,
                &format!("ownership or tree validation changed before isolation: {err}"),
            )));
        }
    };
    drop(validation_handle);

    let (future_entries, future_hashed_bytes) =
        expected_snapshot.remaining_removal_work_upper_bound()?;
    if let Err(err) = budget.ensure_capacity(future_entries, future_hashed_bytes) {
        return Ok(Some(provider_sync_backup_preserved_warning(
            path,
            &format!("candidate removal would exhaust the prune budget: {err}"),
        )));
    }

    #[cfg(test)]
    run_before_provider_sync_backup_isolation_test_hook();

    if let Err(err) = ensure_provider_sync_backup_name_identity(
        root_handle,
        original_name,
        candidate_identity,
        true,
    ) {
        return Ok(Some(provider_sync_backup_preserved_warning(
            path,
            &format!("candidate identity changed before isolation: {err}"),
        )));
    }

    let quarantine_name =
        match isolate_provider_sync_backup_candidate(root_handle, &delete_handle, original_name) {
            Ok(name) => name,
            Err(err) => {
                return Ok(Some(provider_sync_backup_preserved_warning(
                    path,
                    &format!("failed to isolate candidate: {err}"),
                )));
            }
        };
    let quarantine = root.join(&quarantine_name);
    if let Err(err) = validate_isolated_provider_sync_backup(
        root_handle,
        &quarantine_name,
        candidate_identity,
        expected_version,
        &expected_snapshot,
        budget,
    ) {
        return Ok(Some(provider_sync_backup_isolated_warning(
            path,
            &quarantine,
            &format!("validation changed after isolation: {err}"),
        )));
    }

    #[cfg(test)]
    run_after_provider_sync_backup_validation_test_hook(&quarantine);

    if let Err(err) = validate_isolated_provider_sync_backup(
        root_handle,
        &quarantine_name,
        candidate_identity,
        expected_version,
        &expected_snapshot,
        budget,
    ) {
        return Ok(Some(provider_sync_backup_isolated_warning(
            path,
            &quarantine,
            &format!("validation changed before removal: {err}"),
        )));
    }
    if let Err(err) = remove_quarantined_provider_sync_backup(
        root_handle,
        &quarantine_name,
        &delete_handle,
        candidate_identity,
        &expected_snapshot,
        budget,
    ) {
        return Ok(Some(provider_sync_backup_isolated_warning(
            path,
            &quarantine,
            &format!("removal failed: {err}"),
        )));
    }
    Ok(None)
}

fn provider_sync_backup_preserved_warning(path: &Path, reason: &str) -> String {
    format!(
        "provider sync backup prune preserved changed entry at {} because {reason}",
        path.display()
    )
}

fn provider_sync_backup_isolated_warning(
    original: &Path,
    quarantine: &Path,
    reason: &str,
) -> String {
    format!(
        "provider sync backup prune preserved isolated data for {} because {reason}; isolated path is {}",
        original.display(),
        quarantine.display()
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProviderSyncFileIdentity {
    volume: u64,
    file: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProviderSyncFileFingerprint {
    identity: ProviderSyncFileIdentity,
    size: u64,
    modified: i128,
    changed: i128,
    links: u64,
    content_sha256: Option<[u8; 32]>,
}

#[cfg(unix)]
impl ProviderSyncFileFingerprint {
    fn matches_after_unix_rename(&self, observed: &Self) -> bool {
        // renameat updates ctime for the same inode; every other bound property must remain stable.
        self.identity == observed.identity
            && self.size == observed.size
            && self.modified == observed.modified
            && self.links == observed.links
            && self.content_sha256 == observed.content_sha256
    }
}

#[derive(Debug)]
struct ProviderSyncPruneBudget {
    max_depth: usize,
    max_entries: usize,
    max_file_bytes: u64,
    max_hashed_bytes: u64,
    entries_seen: usize,
    hashed_bytes: u64,
}

impl Default for ProviderSyncPruneBudget {
    fn default() -> Self {
        Self {
            max_depth: PROVIDER_SYNC_PRUNE_MAX_DEPTH,
            max_entries: PROVIDER_SYNC_PRUNE_MAX_WORK_ENTRIES,
            max_file_bytes: PROVIDER_SYNC_PRUNE_MAX_FILE_BYTES,
            max_hashed_bytes: PROVIDER_SYNC_PRUNE_MAX_HASHED_BYTES,
            entries_seen: 0,
            hashed_bytes: 0,
        }
    }
}

impl ProviderSyncPruneBudget {
    fn ensure_depth(&self, depth: usize) -> AppResult<()> {
        if depth > self.max_depth {
            return Err("SEC_INVALID_INPUT: provider sync backup tree is too deep".into());
        }
        Ok(())
    }

    fn record_entry(&mut self) -> AppResult<()> {
        self.entries_seen = self.entries_seen.checked_add(1).ok_or_else(|| {
            "SEC_INVALID_INPUT: provider sync backup entry count overflow".to_string()
        })?;
        if self.entries_seen > self.max_entries {
            return Err("SEC_INVALID_INPUT: provider sync backup tree has too many entries".into());
        }
        Ok(())
    }

    fn reserve_file_hash(&mut self, size: u64) -> AppResult<()> {
        if size > self.max_file_bytes {
            return Err(
                "SEC_INVALID_INPUT: provider sync backup file is too large to verify safely".into(),
            );
        }
        self.hashed_bytes = self.hashed_bytes.checked_add(size).ok_or_else(|| {
            "SEC_INVALID_INPUT: provider sync backup hash byte count overflow".to_string()
        })?;
        if self.hashed_bytes > self.max_hashed_bytes {
            return Err(
                "SEC_INVALID_INPUT: provider sync backup tree is too large to verify safely".into(),
            );
        }
        Ok(())
    }

    fn is_exhausted(&self) -> bool {
        self.entries_seen >= self.max_entries || self.hashed_bytes >= self.max_hashed_bytes
    }

    fn with_limits(
        max_depth: usize,
        max_entries: usize,
        max_file_bytes: u64,
        max_hashed_bytes: u64,
    ) -> Self {
        Self {
            max_depth,
            max_entries,
            max_file_bytes,
            max_hashed_bytes,
            entries_seen: 0,
            hashed_bytes: 0,
        }
    }

    fn tree_limits() -> Self {
        Self::with_limits(
            PROVIDER_SYNC_PRUNE_MAX_DEPTH,
            PROVIDER_SYNC_PRUNE_MAX_ENTRIES,
            PROVIDER_SYNC_PRUNE_MAX_FILE_BYTES,
            PROVIDER_SYNC_PRUNE_MAX_TREE_HASHED_BYTES,
        )
    }

    fn consume(&mut self, consumed: &Self) -> AppResult<()> {
        self.ensure_capacity(consumed.entries_seen, consumed.hashed_bytes)?;
        self.entries_seen = self
            .entries_seen
            .checked_add(consumed.entries_seen)
            .ok_or_else(|| {
                "SEC_INVALID_INPUT: provider sync backup entry count overflow".to_string()
            })?;
        self.hashed_bytes = self
            .hashed_bytes
            .checked_add(consumed.hashed_bytes)
            .ok_or_else(|| {
                "SEC_INVALID_INPUT: provider sync backup hash byte count overflow".to_string()
            })?;
        Ok(())
    }

    fn ensure_capacity(
        &self,
        additional_entries: usize,
        additional_hashed_bytes: u64,
    ) -> AppResult<()> {
        let entries = self
            .entries_seen
            .checked_add(additional_entries)
            .ok_or_else(|| {
                "SEC_INVALID_INPUT: provider sync backup entry count overflow".to_string()
            })?;
        if entries > self.max_entries {
            return Err("SEC_INVALID_INPUT: provider sync backup tree has too many entries".into());
        }
        let hashed_bytes = self
            .hashed_bytes
            .checked_add(additional_hashed_bytes)
            .ok_or_else(|| {
                "SEC_INVALID_INPUT: provider sync backup hash byte count overflow".to_string()
            })?;
        if hashed_bytes > self.max_hashed_bytes {
            return Err(
                "SEC_INVALID_INPUT: provider sync backup tree is too large to verify safely".into(),
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProviderSyncBackupEntryKind {
    File,
    Directory,
}

#[cfg(unix)]
type ProviderSyncBackupEntryName = OsString;

#[cfg(windows)]
type ProviderSyncBackupEntryName = Vec<u16>;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProviderSyncBackupEntrySnapshot {
    name: ProviderSyncBackupEntryName,
    fingerprint: ProviderSyncFileFingerprint,
    kind: ProviderSyncBackupEntryKind,
    children: Vec<ProviderSyncBackupEntrySnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProviderSyncBackupTreeSnapshot {
    entries: Vec<ProviderSyncBackupEntrySnapshot>,
}

impl ProviderSyncBackupTreeSnapshot {
    fn remaining_removal_work_upper_bound(&self) -> AppResult<(usize, u64)> {
        let (entries, hashed_bytes) = provider_sync_backup_snapshot_work(&self.entries)?;
        // Two full isolated-tree validations remain. Deletion then hashes every
        // file at three handle-bound boundaries. Each validation also reads the
        // manifest twice outside the tree walk.
        let future_entries = entries.checked_mul(2).ok_or_else(|| {
            "SEC_INVALID_INPUT: provider sync backup entry count overflow".to_string()
        })?;
        let future_hashed_bytes = hashed_bytes
            .checked_mul(5)
            .and_then(|value| {
                (PROVIDER_SYNC_MAX_BYTES as u64)
                    .checked_mul(4)
                    .and_then(|manifest_bytes| value.checked_add(manifest_bytes))
            })
            .ok_or_else(|| {
                "SEC_INVALID_INPUT: provider sync backup hash byte count overflow".to_string()
            })?;
        Ok((future_entries, future_hashed_bytes))
    }
}

fn provider_sync_backup_snapshot_work(
    entries: &[ProviderSyncBackupEntrySnapshot],
) -> AppResult<(usize, u64)> {
    let mut entry_count = 0usize;
    let mut hashed_bytes = 0u64;
    for entry in entries {
        entry_count = entry_count.checked_add(1).ok_or_else(|| {
            "SEC_INVALID_INPUT: provider sync backup entry count overflow".to_string()
        })?;
        if entry.kind == ProviderSyncBackupEntryKind::File {
            hashed_bytes = hashed_bytes
                .checked_add(entry.fingerprint.size)
                .ok_or_else(|| {
                    "SEC_INVALID_INPUT: provider sync backup hash byte count overflow".to_string()
                })?;
        }
        let (child_entries, child_hashed_bytes) =
            provider_sync_backup_snapshot_work(&entry.children)?;
        entry_count = entry_count.checked_add(child_entries).ok_or_else(|| {
            "SEC_INVALID_INPUT: provider sync backup entry count overflow".to_string()
        })?;
        hashed_bytes = hashed_bytes
            .checked_add(child_hashed_bytes)
            .ok_or_else(|| {
                "SEC_INVALID_INPUT: provider sync backup hash byte count overflow".to_string()
            })?;
    }
    Ok((entry_count, hashed_bytes))
}

fn validated_provider_sync_backup_snapshot(
    dir: &std::fs::File,
    expected_version: ManagedBackupVersion,
    budget: &mut ProviderSyncPruneBudget,
) -> AppResult<ProviderSyncBackupTreeSnapshot> {
    if managed_backup_version_from_dir_handle(dir, budget)? != Some(expected_version) {
        return Err("SEC_INVALID_INPUT: provider sync backup ownership changed".into());
    }
    let snapshot = capture_provider_sync_backup_tree_bounded(dir, budget)?;
    if managed_backup_version_from_dir_handle(dir, budget)? != Some(expected_version) {
        return Err("SEC_INVALID_INPUT: provider sync backup ownership changed".into());
    }
    Ok(snapshot)
}

fn capture_provider_sync_backup_tree_bounded(
    dir: &std::fs::File,
    budget: &mut ProviderSyncPruneBudget,
) -> AppResult<ProviderSyncBackupTreeSnapshot> {
    let mut tree_budget = ProviderSyncPruneBudget::tree_limits();
    let snapshot_result = capture_provider_sync_backup_tree(dir, &mut tree_budget);
    budget.consume(&tree_budget)?;
    snapshot_result
}

fn validate_isolated_provider_sync_backup(
    root: &std::fs::File,
    quarantine_name: &OsStr,
    identity: ProviderSyncFileIdentity,
    expected_version: ManagedBackupVersion,
    expected_snapshot: &ProviderSyncBackupTreeSnapshot,
    budget: &mut ProviderSyncPruneBudget,
) -> AppResult<()> {
    let handle = open_provider_sync_backup_child_no_follow(root, quarantine_name, true)?;
    if provider_sync_file_identity_from_handle(&handle)? != identity {
        return Err("SEC_INVALID_INPUT: provider sync quarantine identity changed".into());
    }
    let current = validated_provider_sync_backup_snapshot(&handle, expected_version, budget)?;
    if &current != expected_snapshot {
        return Err("SEC_INVALID_INPUT: provider sync quarantine tree changed".into());
    }
    Ok(())
}

fn ensure_provider_sync_backup_name_identity(
    root: &std::fs::File,
    name: &OsStr,
    expected: ProviderSyncFileIdentity,
    is_directory: bool,
) -> AppResult<()> {
    let current = open_provider_sync_backup_child_no_follow(root, name, is_directory)?;
    if provider_sync_file_identity_from_handle(&current)? != expected {
        return Err("SEC_INVALID_INPUT: provider sync backup entry identity changed".into());
    }
    Ok(())
}

fn isolate_provider_sync_backup_candidate(
    root: &std::fs::File,
    candidate: &std::fs::File,
    original_name: &OsStr,
) -> AppResult<OsString> {
    use rand::RngCore as _;

    for _ in 0..32 {
        let random = rand::thread_rng().next_u64();
        let quarantine_name = OsString::from(format!(
            ".provider-sync-prune-{}-{random:016x}",
            std::process::id()
        ));
        match rename_provider_sync_backup_no_replace(
            root,
            candidate,
            original_name,
            &quarantine_name,
        ) {
            Ok(()) => return Ok(quarantine_name),
            Err(err) if provider_sync_rename_target_exists(&err) => continue,
            Err(err) => return Err(format!("failed to rename managed backup: {err}").into()),
        }
    }
    Err("failed to allocate provider sync prune quarantine path".into())
}

fn provider_sync_rename_target_exists(error: &std::io::Error) -> bool {
    error.kind() == std::io::ErrorKind::AlreadyExists
        || matches!(error.raw_os_error(), Some(80) | Some(183))
}

#[cfg(unix)]
fn rename_provider_sync_backup_no_replace(
    root: &std::fs::File,
    _candidate: &std::fs::File,
    original_name: &OsStr,
    quarantine_name: &OsStr,
) -> std::io::Result<()> {
    rustix::fs::renameat_with(
        root,
        original_name,
        root,
        quarantine_name,
        rustix::fs::RenameFlags::NOREPLACE,
    )
    .map_err(Into::into)
}

#[cfg(windows)]
fn rename_provider_sync_backup_no_replace(
    root: &std::fs::File,
    candidate: &std::fs::File,
    _original_name: &OsStr,
    quarantine_name: &OsStr,
) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt as _;
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Storage::FileSystem::{
        FileRenameInfo, SetFileInformationByHandle, FILE_RENAME_INFO,
    };

    let name = quarantine_name.encode_wide().collect::<Vec<_>>();
    let name_bytes = name
        .len()
        .checked_mul(std::mem::size_of::<u16>())
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "provider sync quarantine name is too long",
            )
        })?;
    let header = std::mem::offset_of!(FILE_RENAME_INFO, FileName);
    let byte_len = header
        .checked_add(name_bytes as usize)
        .ok_or_else(|| std::io::Error::other("provider sync rename buffer overflow"))?;
    let mut buffer = vec![0_u64; byte_len.div_ceil(std::mem::size_of::<u64>())];
    let info = buffer.as_mut_ptr().cast::<FILE_RENAME_INFO>();
    unsafe {
        (*info).Anonymous.ReplaceIfExists = 0;
        (*info).RootDirectory = root.as_raw_handle() as _;
        (*info).FileNameLength = name_bytes;
        std::ptr::copy_nonoverlapping(
            name.as_ptr().cast::<u8>(),
            (*info).FileName.as_mut_ptr().cast::<u8>(),
            name_bytes as usize,
        );
    }
    let ok = unsafe {
        SetFileInformationByHandle(
            candidate.as_raw_handle() as _,
            FileRenameInfo,
            buffer.as_ptr().cast(),
            byte_len as u32,
        )
    };
    if ok == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(unix)]
fn open_provider_sync_backup_dir_no_follow(path: &Path) -> AppResult<std::fs::File> {
    let fd = rustix::fs::open(
        path,
        rustix::fs::OFlags::RDONLY
            | rustix::fs::OFlags::DIRECTORY
            | rustix::fs::OFlags::NOFOLLOW
            | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    )
    .map_err(|err| {
        format!(
            "SEC_INVALID_INPUT: failed to open provider sync backup directory {} without following links: {err}",
            path.display()
        )
    })?;
    Ok(fd.into())
}

#[cfg(windows)]
fn open_provider_sync_backup_dir_no_follow(path: &Path) -> AppResult<std::fs::File> {
    use std::os::windows::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_LIST_DIRECTORY,
        FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_TRAVERSE,
        SYNCHRONIZE,
    };

    let file = std::fs::OpenOptions::new()
        .access_mode(FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES | FILE_TRAVERSE | SYNCHRONIZE)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
        .map_err(|err| {
            format!(
                "SEC_INVALID_INPUT: failed to open provider sync backup directory {}: {err}",
                path.display()
            )
        })?;
    let attributes = windows_provider_sync_backup_handle_attributes(&file)?;
    if attributes & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT != 0
        || attributes & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_DIRECTORY == 0
    {
        return Err(format!(
            "SEC_INVALID_INPUT: provider sync backup directory is not a non-reparse directory path={}",
            path.display()
        )
        .into());
    }
    Ok(file)
}

#[cfg(unix)]
fn provider_sync_backup_root_directory_names(
    root: &std::fs::File,
    budget: &mut ProviderSyncPruneBudget,
) -> AppResult<Vec<OsString>> {
    use std::os::unix::ffi::{OsStrExt as _, OsStringExt as _};

    let entries = rustix::fs::Dir::read_from(root)
        .map_err(|err| format!("failed to enumerate provider sync backup root: {err}"))?;
    let mut names = Vec::new();
    for entry in entries {
        let entry =
            entry.map_err(|err| format!("failed to enumerate provider sync backup root: {err}"))?;
        let raw_name = entry.file_name();
        if raw_name.to_bytes() == b"." || raw_name.to_bytes() == b".." {
            continue;
        }
        budget.record_entry()?;
        let name = OsString::from_vec(raw_name.to_bytes().to_vec());
        let stat = rustix::fs::statat(root, &name, rustix::fs::AtFlags::SYMLINK_NOFOLLOW)
            .map_err(|err| format!("failed to inspect provider sync backup root entry: {err}"))?;
        if rustix::fs::FileType::from_raw_mode(stat.st_mode) == rustix::fs::FileType::Directory {
            names.push(name);
        }
    }
    names.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    Ok(names)
}

#[cfg(windows)]
fn provider_sync_backup_root_directory_names(
    root: &std::fs::File,
    budget: &mut ProviderSyncPruneBudget,
) -> AppResult<Vec<OsString>> {
    use std::os::windows::ffi::OsStringExt as _;

    let mut names = windows_provider_sync_backup_entries(root, budget)?
        .into_iter()
        .filter(|entry| entry.is_directory && !entry.is_reparse)
        .map(|entry| OsString::from_wide(&entry.name))
        .collect::<Vec<_>>();
    names.sort();
    Ok(names)
}

#[cfg(unix)]
fn provider_sync_backup_dir_has_regular_manifest(
    dir: &std::fs::File,
    budget: &mut ProviderSyncPruneBudget,
) -> AppResult<bool> {
    let entries = rustix::fs::Dir::read_from(dir)
        .map_err(|err| format!("failed to enumerate provider sync backup candidate: {err}"))?;
    for entry in entries {
        let entry = entry
            .map_err(|err| format!("failed to enumerate provider sync backup candidate: {err}"))?;
        let name = entry.file_name();
        if name.to_bytes() != b"." && name.to_bytes() != b".." {
            budget.record_entry()?;
        }
        if name.to_bytes() != PROVIDER_SYNC_MANAGED_BACKUP_MANIFEST.as_bytes() {
            continue;
        }
        let stat = rustix::fs::statat(dir, name, rustix::fs::AtFlags::SYMLINK_NOFOLLOW)
            .map_err(|err| format!("failed to inspect provider sync backup manifest: {err}"))?;
        return Ok(
            rustix::fs::FileType::from_raw_mode(stat.st_mode) == rustix::fs::FileType::RegularFile
        );
    }
    Ok(false)
}

#[cfg(windows)]
fn provider_sync_backup_dir_has_regular_manifest(
    dir: &std::fs::File,
    budget: &mut ProviderSyncPruneBudget,
) -> AppResult<bool> {
    use std::os::windows::ffi::OsStrExt as _;

    let manifest_name = OsStr::new(PROVIDER_SYNC_MANAGED_BACKUP_MANIFEST)
        .encode_wide()
        .collect::<Vec<_>>();
    Ok(windows_provider_sync_backup_entries(dir, budget)?
        .into_iter()
        .any(|entry| entry.name == manifest_name && !entry.is_directory && !entry.is_reparse))
}

#[cfg(unix)]
fn open_provider_sync_backup_child_no_follow(
    parent: &std::fs::File,
    name: &OsStr,
    is_directory: bool,
) -> AppResult<std::fs::File> {
    let flags = rustix::fs::OFlags::RDONLY
        | rustix::fs::OFlags::NOFOLLOW
        | rustix::fs::OFlags::CLOEXEC
        | if is_directory {
            rustix::fs::OFlags::DIRECTORY
        } else {
            rustix::fs::OFlags::NONBLOCK
        };
    let fd = rustix::fs::openat(parent, name, flags, rustix::fs::Mode::empty()).map_err(|err| {
        format!(
            "SEC_INVALID_INPUT: failed to open provider sync backup entry without following links: {err}"
        )
    })?;
    let file: std::fs::File = fd.into();
    let stat = rustix::fs::fstat(&file)
        .map_err(|err| format!("failed to inspect provider sync backup entry: {err}"))?;
    let file_type = rustix::fs::FileType::from_raw_mode(stat.st_mode);
    let expected_type = if is_directory {
        rustix::fs::FileType::Directory
    } else {
        rustix::fs::FileType::RegularFile
    };
    if file_type != expected_type {
        return Err("SEC_INVALID_INPUT: provider sync backup entry type changed".into());
    }
    Ok(file)
}

#[cfg(windows)]
fn open_provider_sync_backup_child_no_follow(
    parent: &std::fs::File,
    name: &OsStr,
    is_directory: bool,
) -> AppResult<std::fs::File> {
    use std::os::windows::ffi::OsStrExt as _;
    let name = name.encode_wide().collect::<Vec<_>>();
    open_windows_provider_sync_backup_child_no_follow(parent, &name, is_directory)
}

#[cfg(windows)]
fn open_windows_provider_sync_backup_child_no_follow(
    parent: &std::fs::File,
    name: &[u16],
    is_directory: bool,
) -> AppResult<std::fs::File> {
    use std::os::windows::io::{AsRawHandle as _, FromRawHandle as _};
    use windows_sys::Wdk::Foundation::OBJECT_ATTRIBUTES;
    use windows_sys::Wdk::Storage::FileSystem::{
        NtCreateFile, FILE_DIRECTORY_FILE, FILE_NON_DIRECTORY_FILE, FILE_OPEN,
        FILE_OPEN_REPARSE_POINT, FILE_SYNCHRONOUS_IO_NONALERT,
    };
    use windows_sys::Win32::Foundation::{GENERIC_READ, HANDLE, UNICODE_STRING};
    use windows_sys::Win32::Storage::FileSystem::{
        DELETE, FILE_LIST_DIRECTORY, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ,
        FILE_SHARE_WRITE, FILE_TRAVERSE, SYNCHRONIZE,
    };
    use windows_sys::Win32::System::IO::IO_STATUS_BLOCK;

    let mut name = name.to_vec();
    let byte_len = name
        .len()
        .checked_mul(std::mem::size_of::<u16>())
        .and_then(|value| u16::try_from(value).ok())
        .ok_or_else(|| {
            "SEC_INVALID_INPUT: provider sync backup entry name is too long".to_string()
        })?;
    let unicode = UNICODE_STRING {
        Length: byte_len,
        MaximumLength: byte_len,
        Buffer: name.as_mut_ptr(),
    };
    let attributes = OBJECT_ATTRIBUTES {
        Length: std::mem::size_of::<OBJECT_ATTRIBUTES>() as u32,
        RootDirectory: parent.as_raw_handle() as _,
        ObjectName: &unicode,
        Attributes: 0,
        SecurityDescriptor: std::ptr::null(),
        SecurityQualityOfService: std::ptr::null(),
    };
    let mut io_status = std::mem::MaybeUninit::<IO_STATUS_BLOCK>::zeroed();
    let mut handle: HANDLE = std::ptr::null_mut();
    let desired_access = DELETE
        | FILE_READ_ATTRIBUTES
        | SYNCHRONIZE
        | if is_directory {
            FILE_LIST_DIRECTORY | FILE_TRAVERSE
        } else {
            GENERIC_READ
        };
    let create_options = FILE_OPEN_REPARSE_POINT
        | FILE_SYNCHRONOUS_IO_NONALERT
        | if is_directory {
            FILE_DIRECTORY_FILE
        } else {
            FILE_NON_DIRECTORY_FILE
        };
    let status = unsafe {
        NtCreateFile(
            &mut handle,
            desired_access,
            &attributes,
            io_status.as_mut_ptr(),
            std::ptr::null(),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            FILE_OPEN,
            create_options,
            std::ptr::null(),
            0,
        )
    };
    if status < 0 || handle.is_null() {
        return Err(format!(
            "SEC_INVALID_INPUT: failed to open provider sync backup entry relative to trusted handle: ntstatus {status:#x}"
        )
        .into());
    }
    let file = unsafe { std::fs::File::from_raw_handle(handle as _) };
    let attributes = windows_provider_sync_backup_handle_attributes(&file)?;
    if attributes & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT != 0
        || (attributes & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_DIRECTORY != 0)
            != is_directory
    {
        return Err("SEC_INVALID_INPUT: provider sync backup entry type changed".into());
    }
    Ok(file)
}

#[cfg(unix)]
fn provider_sync_file_identity_from_handle(
    file: &std::fs::File,
) -> AppResult<ProviderSyncFileIdentity> {
    let stat = rustix::fs::fstat(file)
        .map_err(|err| format!("failed to identify provider sync backup handle: {err}"))?;
    Ok(ProviderSyncFileIdentity {
        volume: stat.st_dev as u64,
        file: stat.st_ino as u64,
    })
}

#[cfg(windows)]
fn provider_sync_file_identity_from_handle(
    file: &std::fs::File,
) -> AppResult<ProviderSyncFileIdentity> {
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    };

    let mut info = std::mem::MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::zeroed();
    if unsafe { GetFileInformationByHandle(file.as_raw_handle() as _, info.as_mut_ptr()) } == 0 {
        return Err("failed to identify provider sync backup handle".into());
    }
    let info = unsafe { info.assume_init() };
    Ok(ProviderSyncFileIdentity {
        volume: u64::from(info.dwVolumeSerialNumber),
        file: (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
    })
}

#[cfg(unix)]
fn provider_sync_file_metadata_fingerprint_from_handle(
    file: &std::fs::File,
) -> AppResult<ProviderSyncFileFingerprint> {
    use std::os::unix::fs::MetadataExt as _;

    let metadata = file
        .metadata()
        .map_err(|err| format!("failed to fingerprint provider sync backup handle: {err}"))?;
    Ok(ProviderSyncFileFingerprint {
        identity: ProviderSyncFileIdentity {
            volume: metadata.dev(),
            file: metadata.ino(),
        },
        size: metadata.size(),
        modified: i128::from(metadata.mtime()) * 1_000_000_000 + i128::from(metadata.mtime_nsec()),
        changed: i128::from(metadata.ctime()) * 1_000_000_000 + i128::from(metadata.ctime_nsec()),
        links: metadata.nlink(),
        content_sha256: None,
    })
}

#[cfg(windows)]
fn provider_sync_file_metadata_fingerprint_from_handle(
    file: &std::fs::File,
) -> AppResult<ProviderSyncFileFingerprint> {
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Storage::FileSystem::{
        FileBasicInfo, GetFileInformationByHandle, GetFileInformationByHandleEx,
        BY_HANDLE_FILE_INFORMATION, FILE_BASIC_INFO,
    };

    let mut info = std::mem::MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::zeroed();
    if unsafe { GetFileInformationByHandle(file.as_raw_handle() as _, info.as_mut_ptr()) } == 0 {
        return Err("failed to fingerprint provider sync backup handle".into());
    }
    let info = unsafe { info.assume_init() };
    let mut basic = std::mem::MaybeUninit::<FILE_BASIC_INFO>::zeroed();
    if unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle() as _,
            FileBasicInfo,
            basic.as_mut_ptr().cast(),
            std::mem::size_of::<FILE_BASIC_INFO>() as u32,
        )
    } == 0
    {
        return Err("failed to read provider sync backup change time".into());
    }
    let basic = unsafe { basic.assume_init() };
    let file_time = |value: windows_sys::Win32::Foundation::FILETIME| {
        i128::from((u64::from(value.dwHighDateTime) << 32) | u64::from(value.dwLowDateTime))
    };
    Ok(ProviderSyncFileFingerprint {
        identity: ProviderSyncFileIdentity {
            volume: u64::from(info.dwVolumeSerialNumber),
            file: (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
        },
        size: (u64::from(info.nFileSizeHigh) << 32) | u64::from(info.nFileSizeLow),
        modified: file_time(info.ftLastWriteTime),
        changed: i128::from(basic.ChangeTime),
        links: u64::from(info.nNumberOfLinks),
        content_sha256: None,
    })
}

fn provider_sync_file_content_sha256(
    file: &std::fs::File,
    expected_size: u64,
    budget: &mut ProviderSyncPruneBudget,
) -> AppResult<[u8; 32]> {
    budget.reserve_file_hash(expected_size)?;
    let mut reader = file
        .try_clone()
        .map_err(|err| format!("failed to clone provider sync backup file for hashing: {err}"))?;
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|err| format!("failed to seek provider sync backup file for hashing: {err}"))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; PROVIDER_SYNC_PRUNE_HASH_CHUNK_BYTES];
    let mut bytes_read = 0_u64;
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|err| format!("failed to hash provider sync backup file: {err}"))?;
        if read == 0 {
            break;
        }
        bytes_read = bytes_read.checked_add(read as u64).ok_or_else(|| {
            "SEC_INVALID_INPUT: provider sync backup hash byte count overflow".to_string()
        })?;
        if bytes_read > expected_size {
            return Err(
                "SEC_INVALID_INPUT: provider sync backup file grew while being hashed".into(),
            );
        }
        digest.update(&buffer[..read]);
    }
    if bytes_read != expected_size {
        return Err(
            "SEC_INVALID_INPUT: provider sync backup file size changed while being hashed".into(),
        );
    }
    Ok(digest.finalize().into())
}

fn provider_sync_file_fingerprint_from_handle(
    file: &std::fs::File,
    is_directory: bool,
    budget: &mut ProviderSyncPruneBudget,
) -> AppResult<ProviderSyncFileFingerprint> {
    let before = provider_sync_file_metadata_fingerprint_from_handle(file)?;
    if is_directory {
        return Ok(before);
    }
    let content_sha256 = provider_sync_file_content_sha256(file, before.size, budget)?;
    let after = provider_sync_file_metadata_fingerprint_from_handle(file)?;
    if after != before {
        return Err(
            "SEC_INVALID_INPUT: provider sync backup file changed while being fingerprinted".into(),
        );
    }
    Ok(ProviderSyncFileFingerprint {
        content_sha256: Some(content_sha256),
        ..before
    })
}

fn managed_backup_version_from_dir_handle(
    dir: &std::fs::File,
    budget: &mut ProviderSyncPruneBudget,
) -> AppResult<Option<ManagedBackupVersion>> {
    let mut manifest = open_provider_sync_backup_child_no_follow(
        dir,
        OsStr::new(PROVIDER_SYNC_MANAGED_BACKUP_MANIFEST),
        false,
    )?;
    let bytes = read_open_file_with_max_len(&mut manifest, PROVIDER_SYNC_MAX_BYTES)?;
    budget.reserve_file_hash(bytes.len() as u64)?;
    Ok(managed_backup_version_from_bytes(&bytes))
}

#[cfg(unix)]
fn capture_provider_sync_backup_tree(
    dir: &std::fs::File,
    budget: &mut ProviderSyncPruneBudget,
) -> AppResult<ProviderSyncBackupTreeSnapshot> {
    let entries = capture_provider_sync_backup_tree_unix(dir, 0, budget)?;
    Ok(ProviderSyncBackupTreeSnapshot { entries })
}

#[cfg(unix)]
fn capture_provider_sync_backup_tree_unix(
    dir: &std::fs::File,
    depth: usize,
    budget: &mut ProviderSyncPruneBudget,
) -> AppResult<Vec<ProviderSyncBackupEntrySnapshot>> {
    use std::os::unix::ffi::{OsStrExt as _, OsStringExt as _};

    budget.ensure_depth(depth)?;
    let entries = rustix::fs::Dir::read_from(dir)
        .map_err(|err| format!("failed to enumerate provider sync backup: {err}"))?;
    let mut snapshot = Vec::new();
    for entry in entries {
        let entry =
            entry.map_err(|err| format!("failed to enumerate provider sync backup: {err}"))?;
        let raw_name = entry.file_name();
        if raw_name.to_bytes() == b"." || raw_name.to_bytes() == b".." {
            continue;
        }
        budget.record_entry()?;
        let name = OsString::from_vec(raw_name.to_bytes().to_vec());
        let stat = rustix::fs::statat(dir, &name, rustix::fs::AtFlags::SYMLINK_NOFOLLOW)
            .map_err(|err| format!("failed to inspect provider sync backup entry: {err}"))?;
        let stat_identity = ProviderSyncFileIdentity {
            volume: stat.st_dev as u64,
            file: stat.st_ino as u64,
        };
        let file_type = rustix::fs::FileType::from_raw_mode(stat.st_mode);
        let (fingerprint, kind, children) = if file_type == rustix::fs::FileType::Directory {
            let child = open_provider_sync_backup_child_no_follow(dir, &name, true)?;
            let fingerprint = provider_sync_file_fingerprint_from_handle(&child, true, budget)?;
            if fingerprint.identity != stat_identity {
                return Err(
                    "SEC_INVALID_INPUT: provider sync backup directory identity changed".into(),
                );
            }
            (
                fingerprint,
                ProviderSyncBackupEntryKind::Directory,
                capture_provider_sync_backup_tree_unix(&child, depth + 1, budget)?,
            )
        } else if file_type == rustix::fs::FileType::RegularFile {
            let child = open_provider_sync_backup_child_no_follow(dir, &name, false)?;
            let fingerprint = provider_sync_file_fingerprint_from_handle(&child, false, budget)?;
            if fingerprint.identity != stat_identity {
                return Err("SEC_INVALID_INPUT: provider sync backup file identity changed".into());
            }
            (fingerprint, ProviderSyncBackupEntryKind::File, Vec::new())
        } else {
            return Err(
                "SEC_INVALID_INPUT: provider sync backup contains a link or special entry".into(),
            );
        };
        snapshot.push(ProviderSyncBackupEntrySnapshot {
            name,
            fingerprint,
            kind,
            children,
        });
    }
    snapshot.sort_by(|left, right| left.name.as_bytes().cmp(right.name.as_bytes()));
    Ok(snapshot)
}

#[cfg(windows)]
fn capture_provider_sync_backup_tree(
    dir: &std::fs::File,
    budget: &mut ProviderSyncPruneBudget,
) -> AppResult<ProviderSyncBackupTreeSnapshot> {
    let entries = capture_provider_sync_backup_tree_windows(dir, 0, budget)?;
    Ok(ProviderSyncBackupTreeSnapshot { entries })
}

#[cfg(windows)]
fn capture_provider_sync_backup_tree_windows(
    dir: &std::fs::File,
    depth: usize,
    budget: &mut ProviderSyncPruneBudget,
) -> AppResult<Vec<ProviderSyncBackupEntrySnapshot>> {
    budget.ensure_depth(depth)?;
    let mut snapshot = Vec::new();
    for entry in windows_provider_sync_backup_entries(dir, budget)? {
        if entry.is_reparse {
            return Err(
                "SEC_INVALID_INPUT: provider sync backup contains a reparse-point entry".into(),
            );
        }
        let child = open_windows_provider_sync_backup_child_no_follow(
            dir,
            &entry.name,
            entry.is_directory,
        )?;
        let fingerprint =
            provider_sync_file_fingerprint_from_handle(&child, entry.is_directory, budget)?;
        if fingerprint.identity.file != entry.file_id {
            return Err("SEC_INVALID_INPUT: provider sync backup entry identity changed".into());
        }
        let kind = if entry.is_directory {
            ProviderSyncBackupEntryKind::Directory
        } else {
            ProviderSyncBackupEntryKind::File
        };
        let children = if entry.is_directory {
            capture_provider_sync_backup_tree_windows(&child, depth + 1, budget)?
        } else {
            Vec::new()
        };
        snapshot.push(ProviderSyncBackupEntrySnapshot {
            name: entry.name,
            fingerprint,
            kind,
            children,
        });
    }
    snapshot.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(snapshot)
}

#[cfg(windows)]
#[derive(Debug)]
struct WindowsProviderSyncBackupEntry {
    name: Vec<u16>,
    is_directory: bool,
    is_reparse: bool,
    file_id: u64,
}

#[cfg(windows)]
fn windows_provider_sync_backup_entries(
    dir: &std::fs::File,
    budget: &mut ProviderSyncPruneBudget,
) -> AppResult<Vec<WindowsProviderSyncBackupEntry>> {
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_NO_MORE_FILES};
    use windows_sys::Win32::Storage::FileSystem::{
        FileIdBothDirectoryInfo, FileIdBothDirectoryRestartInfo, GetFileInformationByHandleEx,
        FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT, FILE_ID_BOTH_DIR_INFO,
    };

    let mut entries = Vec::new();
    let mut restart_scan = true;
    loop {
        let mut buffer = vec![0_u64; (64 * 1024) / std::mem::size_of::<u64>()];
        let buffer_bytes = buffer.len() * std::mem::size_of::<u64>();
        let info_class = if restart_scan {
            FileIdBothDirectoryRestartInfo
        } else {
            FileIdBothDirectoryInfo
        };
        let ok = unsafe {
            GetFileInformationByHandleEx(
                dir.as_raw_handle() as _,
                info_class,
                buffer.as_mut_ptr().cast(),
                buffer_bytes as u32,
            )
        };
        if ok == 0 {
            let error = unsafe { GetLastError() };
            if error == ERROR_NO_MORE_FILES {
                break;
            }
            return Err(format!(
                "failed to enumerate quarantined provider sync backup: os error {error}"
            )
            .into());
        }
        restart_scan = false;

        let header = std::mem::offset_of!(FILE_ID_BOTH_DIR_INFO, FileName);
        let alignment = std::mem::align_of::<FILE_ID_BOTH_DIR_INFO>();
        let mut offset = 0usize;
        loop {
            let fixed_end = offset
                .checked_add(std::mem::size_of::<FILE_ID_BOTH_DIR_INFO>())
                .ok_or_else(|| {
                    "SEC_INVALID_INPUT: provider sync backup entry header overflow".to_string()
                })?;
            if offset % alignment != 0 || fixed_end > buffer_bytes {
                return Err(
                    "SEC_INVALID_INPUT: invalid provider sync backup directory entry alignment"
                        .into(),
                );
            }
            let info = unsafe {
                &*buffer
                    .as_ptr()
                    .cast::<u8>()
                    .add(offset)
                    .cast::<FILE_ID_BOTH_DIR_INFO>()
            };
            let name_bytes = info.FileNameLength as usize;
            if name_bytes % std::mem::size_of::<u16>() != 0 {
                return Err(
                    "SEC_INVALID_INPUT: invalid provider sync backup entry name length".into(),
                );
            }
            let minimum_record = header.checked_add(name_bytes).ok_or_else(|| {
                "SEC_INVALID_INPUT: provider sync backup entry length overflow".to_string()
            })?;
            let record_len = if info.NextEntryOffset == 0 {
                minimum_record
            } else {
                info.NextEntryOffset as usize
            };
            let record_end = offset.checked_add(record_len).ok_or_else(|| {
                "SEC_INVALID_INPUT: provider sync backup entry bounds overflow".to_string()
            })?;
            if record_len < minimum_record
                || info.NextEntryOffset != 0 && record_len % alignment != 0
                || record_end > buffer_bytes
            {
                return Err(
                    "SEC_INVALID_INPUT: invalid provider sync backup directory entry bounds".into(),
                );
            }
            let name_len = name_bytes / std::mem::size_of::<u16>();
            let name = unsafe { std::slice::from_raw_parts(info.FileName.as_ptr(), name_len) };
            if name != [b'.' as u16] && name != [b'.' as u16, b'.' as u16] {
                budget.record_entry()?;
                entries.push(WindowsProviderSyncBackupEntry {
                    name: name.to_vec(),
                    is_directory: info.FileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0,
                    is_reparse: info.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0,
                    file_id: info.FileId as u64,
                });
            }
            if info.NextEntryOffset == 0 {
                break;
            }
            offset = offset.checked_add(record_len).ok_or_else(|| {
                "SEC_INVALID_INPUT: provider sync backup entry offset overflow".to_string()
            })?;
        }
    }
    Ok(entries)
}

#[cfg(windows)]
fn windows_provider_sync_backup_handle_attributes(file: &std::fs::File) -> AppResult<u32> {
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    };

    let mut info = std::mem::MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::zeroed();
    if unsafe { GetFileInformationByHandle(file.as_raw_handle() as _, info.as_mut_ptr()) } == 0 {
        return Err("failed to inspect provider sync backup handle".into());
    }
    Ok(unsafe { info.assume_init() }.dwFileAttributes)
}

#[cfg(unix)]
fn isolate_unix_provider_sync_backup_entry(
    parent: &std::fs::File,
    original_name: &OsStr,
) -> AppResult<OsString> {
    use rand::RngCore as _;

    for _ in 0..32 {
        let random = rand::thread_rng().next_u64();
        let tombstone = OsString::from(format!(
            ".provider-sync-delete-{}-{random:016x}",
            std::process::id()
        ));
        match rustix::fs::renameat_with(
            parent,
            original_name,
            parent,
            &tombstone,
            rustix::fs::RenameFlags::NOREPLACE,
        ) {
            Ok(()) => return Ok(tombstone),
            Err(err) => {
                let io_error: std::io::Error = err.into();
                if provider_sync_rename_target_exists(&io_error) {
                    continue;
                }
                return Err(format!(
                    "failed to isolate provider sync backup entry for deletion: {io_error}"
                )
                .into());
            }
        }
    }
    Err("failed to allocate provider sync delete tombstone".into())
}

#[cfg(unix)]
fn remove_quarantined_provider_sync_backup(
    root: &std::fs::File,
    quarantine_name: &OsStr,
    handle: &std::fs::File,
    identity: ProviderSyncFileIdentity,
    snapshot: &ProviderSyncBackupTreeSnapshot,
    budget: &mut ProviderSyncPruneBudget,
) -> AppResult<()> {
    if provider_sync_file_identity_from_handle(handle)? != identity {
        return Err("SEC_INVALID_INPUT: provider sync quarantine handle identity changed".into());
    }
    remove_provider_sync_backup_snapshot_contents_at(handle, snapshot, budget)?;

    #[cfg(test)]
    run_before_unix_provider_sync_backup_root_final_isolation_test_hook();

    let final_name = isolate_unix_provider_sync_backup_entry(root, quarantine_name)?;
    let final_handle = open_provider_sync_backup_child_no_follow(root, &final_name, true)?;
    if provider_sync_file_identity_from_handle(&final_handle)? != identity {
        return Err(format!(
            "SEC_INVALID_INPUT: provider sync quarantine identity changed at final tombstone {}",
            final_name.to_string_lossy()
        )
        .into());
    }
    if !capture_provider_sync_backup_tree_bounded(&final_handle, budget)?
        .entries
        .is_empty()
    {
        return Err(format!(
            "SEC_INVALID_INPUT: provider sync quarantine gained entries at final tombstone {}",
            final_name.to_string_lossy()
        )
        .into());
    }
    ensure_provider_sync_backup_name_identity(root, &final_name, identity, true)?;
    rustix::fs::unlinkat(root, &final_name, rustix::fs::AtFlags::REMOVEDIR)
        .map_err(|err| format!("failed to remove quarantined provider sync backup: {err}").into())
}

#[cfg(unix)]
fn remove_provider_sync_backup_snapshot_contents_at(
    dir: &std::fs::File,
    snapshot: &ProviderSyncBackupTreeSnapshot,
    budget: &mut ProviderSyncPruneBudget,
) -> AppResult<()> {
    remove_provider_sync_backup_snapshot_entries_at(dir, &snapshot.entries, budget)
}

#[cfg(unix)]
fn remove_provider_sync_backup_snapshot_entries_at(
    dir: &std::fs::File,
    entries: &[ProviderSyncBackupEntrySnapshot],
    budget: &mut ProviderSyncPruneBudget,
) -> AppResult<()> {
    for entry in entries {
        let is_directory = entry.kind == ProviderSyncBackupEntryKind::Directory;
        let child = open_provider_sync_backup_child_no_follow(dir, &entry.name, is_directory)?;
        if provider_sync_file_fingerprint_from_handle(&child, is_directory, budget)?
            != entry.fingerprint
        {
            return Err(
                "SEC_INVALID_INPUT: provider sync backup entry changed during removal".into(),
            );
        }

        #[cfg(test)]
        run_before_unix_provider_sync_backup_entry_isolation_test_hook();

        let isolated_name = isolate_unix_provider_sync_backup_entry(dir, &entry.name)?;
        let isolated =
            open_provider_sync_backup_child_no_follow(dir, &isolated_name, is_directory)?;
        let isolated_fingerprint =
            provider_sync_file_fingerprint_from_handle(&isolated, is_directory, budget)?;
        if !entry
            .fingerprint
            .matches_after_unix_rename(&isolated_fingerprint)
        {
            return Err(format!(
                "SEC_INVALID_INPUT: provider sync backup entry identity changed at tombstone {}",
                isolated_name.to_string_lossy()
            )
            .into());
        }
        if is_directory {
            remove_provider_sync_backup_snapshot_entries_at(&isolated, &entry.children, budget)?;
        }

        let final_name = isolate_unix_provider_sync_backup_entry(dir, &isolated_name)?;
        let final_handle =
            open_provider_sync_backup_child_no_follow(dir, &final_name, is_directory)?;
        if is_directory {
            if provider_sync_file_identity_from_handle(&final_handle)? != entry.fingerprint.identity
            {
                return Err(format!(
                    "SEC_INVALID_INPUT: provider sync backup directory identity changed at final tombstone {}",
                    final_name.to_string_lossy()
                )
                .into());
            }
            if !capture_provider_sync_backup_tree_bounded(&final_handle, budget)?
                .entries
                .is_empty()
            {
                return Err(format!(
                    "SEC_INVALID_INPUT: provider sync backup directory gained entries at final tombstone {}",
                    final_name.to_string_lossy()
                )
                .into());
            }
        } else {
            let final_fingerprint =
                provider_sync_file_fingerprint_from_handle(&final_handle, false, budget)?;
            if !entry
                .fingerprint
                .matches_after_unix_rename(&final_fingerprint)
            {
                return Err(format!(
                    "SEC_INVALID_INPUT: provider sync backup file changed at final tombstone {}",
                    final_name.to_string_lossy()
                )
                .into());
            }
        }
        ensure_provider_sync_backup_name_identity(
            dir,
            &final_name,
            entry.fingerprint.identity,
            is_directory,
        )?;
        rustix::fs::unlinkat(
            dir,
            &final_name,
            if is_directory {
                rustix::fs::AtFlags::REMOVEDIR
            } else {
                rustix::fs::AtFlags::empty()
            },
        )
        .map_err(|err| format!("failed to remove provider sync backup entry: {err}"))?;
    }
    if !capture_provider_sync_backup_tree_bounded(dir, budget)?
        .entries
        .is_empty()
    {
        return Err("SEC_INVALID_INPUT: provider sync backup gained entries during removal".into());
    }
    Ok(())
}

#[cfg(windows)]
fn remove_quarantined_provider_sync_backup(
    root: &std::fs::File,
    quarantine_name: &OsStr,
    handle: &std::fs::File,
    identity: ProviderSyncFileIdentity,
    snapshot: &ProviderSyncBackupTreeSnapshot,
    budget: &mut ProviderSyncPruneBudget,
) -> AppResult<()> {
    if provider_sync_file_identity_from_handle(handle)? != identity {
        return Err("SEC_INVALID_INPUT: provider sync quarantine handle identity changed".into());
    }
    remove_windows_provider_sync_backup_snapshot_contents(handle, snapshot, budget)?;
    ensure_provider_sync_backup_name_identity(root, quarantine_name, identity, true)?;
    delete_windows_provider_sync_backup_handle(handle)
}

#[cfg(windows)]
fn remove_windows_provider_sync_backup_snapshot_contents(
    dir: &std::fs::File,
    snapshot: &ProviderSyncBackupTreeSnapshot,
    budget: &mut ProviderSyncPruneBudget,
) -> AppResult<()> {
    remove_windows_provider_sync_backup_snapshot_entries(dir, &snapshot.entries, budget)
}

#[cfg(windows)]
fn remove_windows_provider_sync_backup_snapshot_entries(
    dir: &std::fs::File,
    entries: &[ProviderSyncBackupEntrySnapshot],
    budget: &mut ProviderSyncPruneBudget,
) -> AppResult<()> {
    for entry in entries {
        let is_directory = entry.kind == ProviderSyncBackupEntryKind::Directory;
        let child =
            open_windows_provider_sync_backup_child_no_follow(dir, &entry.name, is_directory)?;
        if provider_sync_file_fingerprint_from_handle(&child, is_directory, budget)?
            != entry.fingerprint
        {
            return Err(
                "SEC_INVALID_INPUT: provider sync backup entry changed during removal".into(),
            );
        }
        if is_directory {
            remove_windows_provider_sync_backup_snapshot_entries(&child, &entry.children, budget)?;
        }

        #[cfg(test)]
        run_before_windows_provider_sync_backup_handle_delete_test_hook();

        if is_directory {
            if provider_sync_file_identity_from_handle(&child)? != entry.fingerprint.identity {
                return Err(
                    "SEC_INVALID_INPUT: provider sync backup directory identity changed before handle deletion"
                        .into(),
                );
            }
            if !capture_provider_sync_backup_tree_bounded(&child, budget)?
                .entries
                .is_empty()
            {
                return Err(
                    "SEC_INVALID_INPUT: provider sync backup directory gained entries before handle deletion"
                        .into(),
                );
            }
        } else if provider_sync_file_fingerprint_from_handle(&child, false, budget)?
            != entry.fingerprint
        {
            return Err(
                "SEC_INVALID_INPUT: provider sync backup file changed before handle deletion"
                    .into(),
            );
        }
        delete_windows_provider_sync_backup_handle(&child)?;
        drop(child);
    }
    if !capture_provider_sync_backup_tree_bounded(dir, budget)?
        .entries
        .is_empty()
    {
        return Err("SEC_INVALID_INPUT: provider sync backup gained entries during removal".into());
    }
    Ok(())
}

#[cfg(windows)]
fn delete_windows_provider_sync_backup_handle(file: &std::fs::File) -> AppResult<()> {
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::Storage::FileSystem::{
        FileDispositionInfo, FileDispositionInfoEx, SetFileInformationByHandle,
        FILE_DISPOSITION_FLAG_DELETE, FILE_DISPOSITION_FLAG_IGNORE_READONLY_ATTRIBUTE,
        FILE_DISPOSITION_FLAG_POSIX_SEMANTICS, FILE_DISPOSITION_INFO, FILE_DISPOSITION_INFO_EX,
    };

    let disposition_ex = FILE_DISPOSITION_INFO_EX {
        Flags: FILE_DISPOSITION_FLAG_DELETE
            | FILE_DISPOSITION_FLAG_POSIX_SEMANTICS
            | FILE_DISPOSITION_FLAG_IGNORE_READONLY_ATTRIBUTE,
    };
    let ex_ok = unsafe {
        SetFileInformationByHandle(
            file.as_raw_handle() as _,
            FileDispositionInfoEx,
            (&disposition_ex as *const FILE_DISPOSITION_INFO_EX).cast(),
            std::mem::size_of::<FILE_DISPOSITION_INFO_EX>() as u32,
        )
    };
    if ex_ok != 0 {
        return Ok(());
    }
    let ex_error = unsafe { GetLastError() };
    let disposition = FILE_DISPOSITION_INFO { DeleteFile: 1 };
    let fallback_ok = unsafe {
        SetFileInformationByHandle(
            file.as_raw_handle() as _,
            FileDispositionInfo,
            (&disposition as *const FILE_DISPOSITION_INFO).cast(),
            std::mem::size_of::<FILE_DISPOSITION_INFO>() as u32,
        )
    };
    if fallback_ok == 0 {
        let fallback_error = unsafe { GetLastError() };
        return Err(format!(
            "failed to delete provider sync backup handle: extended error {ex_error}; fallback error {fallback_error}"
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
type AfterProviderSyncBackupValidationHook = Box<dyn FnOnce(&Path) + Send>;

#[cfg(test)]
type BeforeProviderSyncBackupIsolationHook = Box<dyn FnOnce() + Send>;

#[cfg(test)]
type AfterProviderSyncBackupRootOpenHook = Box<dyn FnOnce() + Send>;

#[cfg(all(test, unix))]
type BeforeUnixProviderSyncBackupEntryIsolationHook = Box<dyn FnOnce() + Send>;

#[cfg(all(test, unix))]
type BeforeUnixProviderSyncBackupRootFinalIsolationHook = Box<dyn FnOnce() + Send>;

#[cfg(all(test, windows))]
type BeforeWindowsProviderSyncBackupHandleDeleteHook = Box<dyn FnOnce() + Send>;

#[cfg(test)]
thread_local! {
    static BEFORE_PROVIDER_SYNC_BACKUP_ISOLATION_TEST_HOOK: std::cell::RefCell<Option<BeforeProviderSyncBackupIsolationHook>> = const { std::cell::RefCell::new(None) };
    static AFTER_PROVIDER_SYNC_BACKUP_ROOT_OPEN_TEST_HOOK: std::cell::RefCell<Option<AfterProviderSyncBackupRootOpenHook>> = const { std::cell::RefCell::new(None) };
}

#[cfg(all(test, unix))]
thread_local! {
    static BEFORE_UNIX_PROVIDER_SYNC_BACKUP_ENTRY_ISOLATION_TEST_HOOK: std::cell::RefCell<Option<BeforeUnixProviderSyncBackupEntryIsolationHook>> = const { std::cell::RefCell::new(None) };
    static BEFORE_UNIX_PROVIDER_SYNC_BACKUP_ROOT_FINAL_ISOLATION_TEST_HOOK: std::cell::RefCell<Option<BeforeUnixProviderSyncBackupRootFinalIsolationHook>> = const { std::cell::RefCell::new(None) };
}

#[cfg(all(test, windows))]
thread_local! {
    static BEFORE_WINDOWS_PROVIDER_SYNC_BACKUP_HANDLE_DELETE_TEST_HOOK: std::cell::RefCell<Option<BeforeWindowsProviderSyncBackupHandleDeleteHook>> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub(super) fn set_before_provider_sync_backup_isolation_test_hook(
    hook: BeforeProviderSyncBackupIsolationHook,
) {
    BEFORE_PROVIDER_SYNC_BACKUP_ISOLATION_TEST_HOOK.with(|current| {
        assert!(
            current.borrow().is_none(),
            "provider sync isolation test hook is already set"
        );
        current.replace(Some(hook));
    });
}

#[cfg(test)]
fn run_before_provider_sync_backup_isolation_test_hook() {
    let hook =
        BEFORE_PROVIDER_SYNC_BACKUP_ISOLATION_TEST_HOOK.with(|current| current.borrow_mut().take());
    if let Some(hook) = hook {
        hook();
    }
}

#[cfg(test)]
pub(super) fn set_after_provider_sync_backup_root_open_test_hook(
    hook: AfterProviderSyncBackupRootOpenHook,
) {
    AFTER_PROVIDER_SYNC_BACKUP_ROOT_OPEN_TEST_HOOK.with(|current| {
        assert!(
            current.borrow().is_none(),
            "provider sync root-open test hook is already set"
        );
        current.replace(Some(hook));
    });
}

#[cfg(test)]
fn run_after_provider_sync_backup_root_open_test_hook() {
    let hook =
        AFTER_PROVIDER_SYNC_BACKUP_ROOT_OPEN_TEST_HOOK.with(|current| current.borrow_mut().take());
    if let Some(hook) = hook {
        hook();
    }
}

#[cfg(all(test, unix))]
pub(super) fn set_before_unix_provider_sync_backup_entry_isolation_test_hook(
    hook: BeforeUnixProviderSyncBackupEntryIsolationHook,
) {
    BEFORE_UNIX_PROVIDER_SYNC_BACKUP_ENTRY_ISOLATION_TEST_HOOK.with(|current| {
        assert!(
            current.borrow().is_none(),
            "provider sync entry isolation test hook is already set"
        );
        current.replace(Some(hook));
    });
}

#[cfg(all(test, unix))]
fn run_before_unix_provider_sync_backup_entry_isolation_test_hook() {
    let hook = BEFORE_UNIX_PROVIDER_SYNC_BACKUP_ENTRY_ISOLATION_TEST_HOOK
        .with(|current| current.borrow_mut().take());
    if let Some(hook) = hook {
        hook();
    }
}

#[cfg(all(test, unix))]
pub(super) fn set_before_unix_provider_sync_backup_root_final_isolation_test_hook(
    hook: BeforeUnixProviderSyncBackupRootFinalIsolationHook,
) {
    BEFORE_UNIX_PROVIDER_SYNC_BACKUP_ROOT_FINAL_ISOLATION_TEST_HOOK.with(|current| {
        assert!(
            current.borrow().is_none(),
            "provider sync root final isolation test hook is already set"
        );
        current.replace(Some(hook));
    });
}

#[cfg(all(test, unix))]
fn run_before_unix_provider_sync_backup_root_final_isolation_test_hook() {
    let hook = BEFORE_UNIX_PROVIDER_SYNC_BACKUP_ROOT_FINAL_ISOLATION_TEST_HOOK
        .with(|current| current.borrow_mut().take());
    if let Some(hook) = hook {
        hook();
    }
}

#[cfg(all(test, windows))]
pub(super) fn set_before_windows_provider_sync_backup_handle_delete_test_hook(
    hook: BeforeWindowsProviderSyncBackupHandleDeleteHook,
) {
    BEFORE_WINDOWS_PROVIDER_SYNC_BACKUP_HANDLE_DELETE_TEST_HOOK.with(|current| {
        assert!(
            current.borrow().is_none(),
            "provider sync Windows handle-delete test hook is already set"
        );
        current.replace(Some(hook));
    });
}

#[cfg(all(test, windows))]
fn run_before_windows_provider_sync_backup_handle_delete_test_hook() {
    let hook = BEFORE_WINDOWS_PROVIDER_SYNC_BACKUP_HANDLE_DELETE_TEST_HOOK
        .with(|current| current.borrow_mut().take());
    if let Some(hook) = hook {
        hook();
    }
}

#[cfg(test)]
thread_local! {
    static AFTER_PROVIDER_SYNC_BACKUP_VALIDATION_TEST_HOOK: std::cell::RefCell<Option<AfterProviderSyncBackupValidationHook>> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub(super) fn set_after_provider_sync_backup_validation_test_hook(
    hook: AfterProviderSyncBackupValidationHook,
) {
    AFTER_PROVIDER_SYNC_BACKUP_VALIDATION_TEST_HOOK.with(|current| {
        assert!(
            current.borrow().is_none(),
            "provider sync validation test hook is already set"
        );
        current.replace(Some(hook));
    });
}

#[cfg(test)]
fn run_after_provider_sync_backup_validation_test_hook(quarantine: &Path) {
    let hook =
        AFTER_PROVIDER_SYNC_BACKUP_VALIDATION_TEST_HOOK.with(|current| current.borrow_mut().take());
    if let Some(hook) = hook {
        hook(quarantine);
    }
}

#[cfg(test)]
mod tests;
