//! Usage: Strict Codex provider sync / backup / rollback core.

use crate::shared::error::AppResult;
use crate::shared::fs::{
    is_symlink, read_optional_file_with_max_len, write_file_atomic_if_changed,
};
use crate::shared::time::{now_unix_millis, now_unix_seconds};
use serde::Serialize;
use serde_json::Value;
use std::fs;
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
    let mut warnings = Vec::new();
    let mut candidates = Vec::new();
    for entry in fs::read_dir(&root)
        .map_err(|e| format!("failed to read backup root {}: {e}", root.display()))?
    {
        let entry =
            entry.map_err(|e| format!("failed to read backup entry {}: {e}", root.display()))?;
        let file_type = entry
            .file_type()
            .map_err(|e| format!("failed to inspect backup entry {}: {e}", root.display()))?;
        if file_type.is_symlink() || !file_type.is_dir() {
            continue;
        }
        let path = entry.path();
        let Some(version) = managed_backup_version(&path)? else {
            continue;
        };
        if version == ManagedBackupVersion::V2 && path == current_backup {
            continue;
        }
        candidates.push((path, version));
    }
    for (path, version) in candidates {
        if let Some(warning) = remove_managed_backup_candidate(&root, &path, version)? {
            warnings.push(warning);
        }
    }
    Ok((!warnings.is_empty()).then(|| warnings.join("; ")))
}

fn managed_backup_version(path: &Path) -> AppResult<Option<ManagedBackupVersion>> {
    let manifest_path = path.join(PROVIDER_SYNC_MANAGED_BACKUP_MANIFEST);
    let manifest_metadata = match fs::symlink_metadata(&manifest_path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(format!(
                "failed to inspect backup manifest {}: {err}",
                manifest_path.display()
            )
            .into());
        }
    };
    if manifest_metadata.file_type().is_symlink() || !manifest_metadata.is_file() {
        return Ok(None);
    }
    let Some(bytes) = read_optional_file_with_max_len(&manifest_path, PROVIDER_SYNC_MAX_BYTES)?
    else {
        return Ok(None);
    };
    let Ok(manifest) = serde_json::from_slice::<Value>(&bytes) else {
        return Ok(None);
    };
    if manifest.get("managed_by").and_then(Value::as_str) != Some(PROVIDER_SYNC_MANAGED_BY) {
        return Ok(None);
    }
    let Some(created_at) = manifest.get("created_at").and_then(Value::as_str) else {
        return Ok(None);
    };
    if created_at.parse::<u128>().is_err()
        || !manifest.get("trigger").is_some_and(Value::is_string)
        || !manifest
            .get("target_provider")
            .is_some_and(Value::is_string)
        || !manifest_path_field_is_valid(&manifest, "config_path")
        || !manifest_string_array_is_valid(&manifest, "session_files")
    {
        return Ok(None);
    }

    match manifest.get("version").and_then(Value::as_u64) {
        Some(1)
            if manifest_path_field_is_valid(&manifest, "global_state_path")
                && manifest_string_array_is_valid(&manifest, "sqlite_files") =>
        {
            Ok(Some(ManagedBackupVersion::V1))
        }
        Some(version) if version == u64::from(PROVIDER_SYNC_BACKUP_VERSION) => {
            if manifest.get("scope").and_then(Value::as_str) != Some(PROVIDER_SYNC_BACKUP_SCOPE)
                || manifest.get("sqlite_files").is_some()
                || manifest.get("global_state_path").is_some()
            {
                return Ok(None);
            }
            Ok(Some(ManagedBackupVersion::V2))
        }
        _ => Ok(None),
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

fn remove_managed_backup_candidate(
    root: &Path,
    path: &Path,
    expected_version: ManagedBackupVersion,
) -> AppResult<Option<String>> {
    if path.parent() != Some(root) {
        return Err(format!(
            "SEC_INVALID_INPUT: managed backup candidate is outside backup root path={}",
            path.display()
        )
        .into());
    }
    let quarantine = next_prune_quarantine_path(root)?;
    match fs::rename(path, &quarantine) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Ok(Some(format!(
                "provider sync backup prune failed to isolate {}: {err}",
                path.display()
            )));
        }
    }

    let validation = match fs::symlink_metadata(&quarantine) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            managed_backup_version(&quarantine).and_then(|version| {
                if version != Some(expected_version) {
                    return Ok(false);
                }
                backup_tree_is_safe_to_remove(&quarantine)
            })
        }
        Ok(_) => Ok(false),
        Err(err) => Err(format!(
            "failed to inspect isolated provider sync backup {}: {err}",
            quarantine.display()
        )
        .into()),
    };
    match validation {
        Ok(true) => {}
        Ok(false) => {
            return Ok(Some(restore_quarantined_backup(
                path,
                &quarantine,
                "ownership or symlink validation changed after isolation",
            )));
        }
        Err(err) => {
            return Ok(Some(restore_quarantined_backup(
                path,
                &quarantine,
                &format!("validation failed after isolation: {err}"),
            )));
        }
    }

    if let Err(err) = fs::remove_dir_all(&quarantine) {
        return Ok(Some(format!(
            "provider sync backup prune failed for {}: {err}",
            quarantine.display()
        )));
    }
    Ok(None)
}

fn next_prune_quarantine_path(root: &Path) -> AppResult<PathBuf> {
    use rand::RngCore as _;

    for _ in 0..32 {
        let random = rand::thread_rng().next_u64();
        let candidate = root.join(format!(
            ".provider-sync-prune-{}-{random:016x}",
            std::process::id()
        ));
        match fs::symlink_metadata(&candidate) {
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(candidate),
            Ok(_) => continue,
            Err(err) => {
                return Err(format!(
                    "failed to inspect provider sync prune path {}: {err}",
                    candidate.display()
                )
                .into());
            }
        }
    }
    Err("failed to allocate provider sync prune quarantine path".into())
}

fn restore_quarantined_backup(original: &Path, quarantine: &Path, reason: &str) -> String {
    let restored = match fs::symlink_metadata(original) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => fs::rename(quarantine, original),
        Ok(_) => {
            return format!(
                "provider sync backup prune preserved changed entry at {} because {reason}; isolated data remains at {}",
                original.display(),
                quarantine.display()
            );
        }
        Err(err) => {
            return format!(
                "provider sync backup prune preserved {} because {reason}; failed to inspect restore path: {err}; isolated data remains at {}",
                original.display(),
                quarantine.display()
            );
        }
    };
    match restored {
        Ok(()) => format!(
            "provider sync backup prune preserved {} because {reason}",
            original.display()
        ),
        Err(err) => format!(
            "provider sync backup prune preserved isolated data at {} because {reason}; failed to restore {}: {err}",
            quarantine.display(),
            original.display()
        ),
    }
}

fn backup_tree_is_safe_to_remove(root: &Path) -> AppResult<bool> {
    let mut pending = vec![(root.to_path_buf(), 0usize)];
    let mut entries_seen = 0usize;
    while let Some((dir, depth)) = pending.pop() {
        if depth > PROVIDER_SYNC_PRUNE_MAX_DEPTH {
            return Ok(false);
        }
        for entry in fs::read_dir(&dir)
            .map_err(|e| format!("failed to inspect managed backup {}: {e}", dir.display()))?
        {
            let entry = entry
                .map_err(|e| format!("failed to inspect managed backup {}: {e}", dir.display()))?;
            let path = entry.path();
            let file_type = entry.file_type().map_err(|e| {
                format!(
                    "failed to inspect managed backup entry {}: {e}",
                    path.display()
                )
            })?;
            entries_seen += 1;
            if entries_seen > PROVIDER_SYNC_PRUNE_MAX_ENTRIES {
                return Ok(false);
            }
            if file_type.is_symlink() {
                return Ok(false);
            }
            if file_type.is_dir() {
                pending.push((path, depth + 1));
            }
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests;
