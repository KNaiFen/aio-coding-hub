//! Usage: Read / patch Codex user-level `config.toml` ($CODEX_HOME/config.toml).

mod parsing;
mod patching;
mod types;

pub use types::{
    CodexConfigPatch, CodexConfigState, CodexConfigTomlState, CodexConfigTomlValidationError,
    CodexConfigTomlValidationResult,
};

use crate::codex_paths;
use crate::shared::fs::{
    is_symlink, read_optional_file_with_max_len, write_file_atomic, write_file_atomic_if_changed,
};
use parsing::{make_state_from_bytes, validate_codex_config_toml_raw};
use patching::patch_config_toml;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use types::CodexConfigStateMeta;

const CODEX_CONFIG_MAX_BYTES: usize = 1024 * 1024;
const CODEX_LIFECYCLE_JOURNAL_MAX_BYTES: usize = 512 * 1024;
const CODEX_LIFECYCLE_JOURNAL_SCHEMA_VERSION: u32 = 1;
const CODEX_LIFECYCLE_JOURNAL_FILE_NAME: &str = "lifecycle-journal.json";
const LIFECYCLE_FAILPOINT_NONE: u8 = 0;
const LIFECYCLE_FAILPOINT_PLANNED: u8 = 1;
const LIFECYCLE_FAILPOINT_CANONICAL_WRITTEN: u8 = 2;
const LIFECYCLE_FAILPOINT_LIVE_WRITTEN: u8 = 3;
const LIFECYCLE_FAILPOINT_CATALOG_WRITTEN: u8 = 4;
const LIFECYCLE_FAILPOINT_CATALOG_POLICY_WRITTEN: u8 = 5;

static CODEX_LIFECYCLE_FAILPOINT: AtomicU8 = AtomicU8::new(LIFECYCLE_FAILPOINT_NONE);

pub(crate) fn set_lifecycle_failpoint_for_tests(
    failpoint: Option<&str>,
) -> crate::shared::error::AppResult<()> {
    let value = match failpoint {
        None => LIFECYCLE_FAILPOINT_NONE,
        Some("planned") => LIFECYCLE_FAILPOINT_PLANNED,
        Some("canonical_written") => LIFECYCLE_FAILPOINT_CANONICAL_WRITTEN,
        Some("live_written") => LIFECYCLE_FAILPOINT_LIVE_WRITTEN,
        Some("catalog_written") => LIFECYCLE_FAILPOINT_CATALOG_WRITTEN,
        Some("catalog_policy_written") => LIFECYCLE_FAILPOINT_CATALOG_POLICY_WRITTEN,
        Some(_) => {
            return Err(crate::shared::error::AppError::new(
                "SEC_INVALID_INPUT",
                "unknown Codex lifecycle test failpoint",
            ));
        }
    };
    CODEX_LIFECYCLE_FAILPOINT.store(value, Ordering::SeqCst);
    Ok(())
}

pub(crate) fn interrupt_lifecycle_for_tests(failpoint: &str) -> crate::shared::error::AppResult<()> {
    let expected = match failpoint {
        "planned" => LIFECYCLE_FAILPOINT_PLANNED,
        "canonical_written" => LIFECYCLE_FAILPOINT_CANONICAL_WRITTEN,
        "live_written" => LIFECYCLE_FAILPOINT_LIVE_WRITTEN,
        "catalog_written" => LIFECYCLE_FAILPOINT_CATALOG_WRITTEN,
        "catalog_policy_written" => LIFECYCLE_FAILPOINT_CATALOG_POLICY_WRITTEN,
        _ => return Ok(()),
    };
    if CODEX_LIFECYCLE_FAILPOINT
        .compare_exchange(
            expected,
            LIFECYCLE_FAILPOINT_NONE,
            Ordering::SeqCst,
            Ordering::SeqCst,
        )
        .is_ok()
    {
        return Err(crate::shared::error::AppError::new(
            "CODEX_LIFECYCLE_TEST_INTERRUPTED",
            format!("Codex lifecycle interrupted at {failpoint}"),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum CodexLifecycleJournalKind {
    CanonicalConfig,
    CatalogPolicy,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum CodexLifecycleJournalPhase {
    Planned,
    CanonicalWritten,
    LiveWritten,
    CatalogWritten,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CodexLifecycleJournal {
    schema_version: u32,
    kind: CodexLifecycleJournalKind,
    operation: String,
    phase: CodexLifecycleJournalPhase,
    canonical_sha256: Option<String>,
    projected_sha256: Option<String>,
    live_before_sha256: Option<String>,
    live_after_sha256: Option<String>,
    proxy_enabled: bool,
    provider_sync_target: Option<String>,
    mcp_manifest_after: Option<serde_json::Value>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn optional_sha256(bytes: Option<&[u8]>) -> Option<String> {
    bytes.map(sha256_hex)
}

fn lifecycle_journal_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    Ok(crate::app_paths::app_data_dir(app)?
        .join("cli-proxy")
        .join("codex")
        .join(CODEX_LIFECYCLE_JOURNAL_FILE_NAME))
}

fn write_lifecycle_journal(
    path: &Path,
    journal: &CodexLifecycleJournal,
) -> crate::shared::error::AppResult<()> {
    let mut bytes = serde_json::to_vec_pretty(journal).map_err(|_| {
        crate::shared::error::AppError::new(
            "CODEX_LIFECYCLE_JOURNAL_WRITE_FAILED",
            "failed to serialize the Codex lifecycle journal",
        )
    })?;
    bytes.push(b'\n');
    if bytes.len() > CODEX_LIFECYCLE_JOURNAL_MAX_BYTES {
        return Err(crate::shared::error::AppError::new(
            "CODEX_LIFECYCLE_JOURNAL_WRITE_FAILED",
            "the Codex lifecycle journal exceeds its size limit",
        ));
    }
    write_file_atomic(path, &bytes).map_err(|error| {
        crate::shared::error::AppError::new(
            "CODEX_LIFECYCLE_JOURNAL_WRITE_FAILED",
            format!("failed to persist the Codex lifecycle journal: {error}"),
        )
    })
}

fn read_lifecycle_journal(
    path: &Path,
) -> crate::shared::error::AppResult<Option<CodexLifecycleJournal>> {
    let Some(bytes) = read_optional_file_with_max_len(path, CODEX_LIFECYCLE_JOURNAL_MAX_BYTES)?
    else {
        return Ok(None);
    };
    let journal: CodexLifecycleJournal = serde_json::from_slice(&bytes).map_err(|_| {
        crate::shared::error::AppError::new(
            "CODEX_LIFECYCLE_RECOVERY_REQUIRED",
            "the Codex lifecycle journal is invalid",
        )
    })?;
    if journal.schema_version != CODEX_LIFECYCLE_JOURNAL_SCHEMA_VERSION {
        return Err(crate::shared::error::AppError::new(
            "CODEX_LIFECYCLE_RECOVERY_REQUIRED",
            "the Codex lifecycle journal schema is unsupported",
        ));
    }
    Ok(Some(journal))
}

fn clear_lifecycle_journal(path: &Path) -> crate::shared::error::AppResult<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(crate::shared::error::AppError::new(
            "CODEX_LIFECYCLE_JOURNAL_WRITE_FAILED",
            format!("failed to clear the Codex lifecycle journal: {error}"),
        )),
    }
}

fn ensure_codex_config_len(bytes: &[u8], label: &str) -> crate::shared::error::AppResult<()> {
    if bytes.len() > CODEX_CONFIG_MAX_BYTES {
        return Err(format!(
            "SEC_INVALID_INPUT: {label} too large (max {CODEX_CONFIG_MAX_BYTES} bytes)"
        )
        .into());
    }
    Ok(())
}

fn read_optional_codex_config_file(
    path: &Path,
) -> crate::shared::error::AppResult<Option<Vec<u8>>> {
    read_optional_file_with_max_len(path, CODEX_CONFIG_MAX_BYTES)
}

#[derive(Debug)]
pub(crate) struct CodexCliProxyBackupSnapshot {
    manifest_path: PathBuf,
    manifest_existed: bool,
    manifest_bytes: Option<Vec<u8>>,
    backup_path: PathBuf,
    backup_existed: bool,
    backup_bytes: Option<Vec<u8>>,
}

pub(crate) fn sync_codex_cli_proxy_backup_if_enabled<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    next_bytes: &[u8],
) -> crate::shared::error::AppResult<Option<CodexCliProxyBackupSnapshot>> {
    ensure_codex_config_len(next_bytes, "codex config backup")?;
    let manifest_path = crate::app_paths::app_data_dir(app)?
        .join("cli-proxy")
        .join("codex")
        .join("manifest.json");
    let manifest_snapshot = snapshot_optional_file(&manifest_path)?;
    let Some(backup_path) = super::cli_proxy::backup_file_path_for_enabled_manifest(
        app,
        "codex",
        "codex_config_toml",
        "config.toml",
    )
    .inspect_err(|_err| {
        let _ = restore_optional_file(&manifest_path, &manifest_snapshot);
    })?
    else {
        return Ok(None);
    };

    let backup_snapshot = match snapshot_optional_file(&backup_path) {
        Ok(snapshot) => snapshot,
        Err(err) => {
            let _ = restore_optional_file(&manifest_path, &manifest_snapshot);
            return Err(format!("CODEX_CONFIG_BACKUP_REFRESH_FAILED: {err}").into());
        }
    };
    let snapshot = CodexCliProxyBackupSnapshot {
        manifest_path,
        manifest_existed: manifest_snapshot.0,
        manifest_bytes: manifest_snapshot.1,
        backup_path,
        backup_existed: backup_snapshot.0,
        backup_bytes: backup_snapshot.1,
    };

    if let Err(err) = write_file_atomic_if_changed(&snapshot.backup_path, next_bytes)
        .map_err(|err| format!("CODEX_CONFIG_BACKUP_REFRESH_FAILED: {err}"))
    {
        let _ = restore_codex_cli_proxy_backup_snapshot(&snapshot);
        return Err(err.into());
    }

    Ok(Some(snapshot))
}

fn snapshot_optional_file(path: &Path) -> crate::shared::error::AppResult<(bool, Option<Vec<u8>>)> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if !metadata.is_file() {
                return Err(format!(
                    "SEC_INVALID_INPUT: backup target is not a file path={}",
                    path.display()
                )
                .into());
            }
            let bytes = fs::read(path).map_err(|err| {
                format!("failed to snapshot backup target {}: {err}", path.display())
            })?;
            Ok((true, Some(bytes)))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok((false, None)),
        Err(err) => Err(format!("failed to read backup target {}: {err}", path.display()).into()),
    }
}

fn restore_optional_file(
    path: &Path,
    snapshot: &(bool, Option<Vec<u8>>),
) -> crate::shared::error::AppResult<()> {
    match snapshot {
        (true, Some(bytes)) => {
            let _ = write_file_atomic_if_changed(path, bytes)?;
        }
        (false, _) => remove_path_if_exists(path)?,
        (true, None) => {}
    }
    Ok(())
}

pub(crate) fn restore_codex_cli_proxy_backup_snapshot(
    snapshot: &CodexCliProxyBackupSnapshot,
) -> crate::shared::error::AppResult<()> {
    restore_optional_file(
        &snapshot.backup_path,
        &(snapshot.backup_existed, snapshot.backup_bytes.clone()),
    )?;
    restore_optional_file(
        &snapshot.manifest_path,
        &(snapshot.manifest_existed, snapshot.manifest_bytes.clone()),
    )?;
    Ok(())
}

struct CodexBackupRollbackState {
    snapshot: CodexCliProxyBackupSnapshot,
    expected_manifest: (bool, Option<Vec<u8>>),
    expected_backup: (bool, Option<Vec<u8>>),
}

pub(crate) struct CanonicalConfigTransaction {
    config_path: PathBuf,
    live_before: (bool, Option<Vec<u8>>),
    live_written: Vec<u8>,
    backup: Option<CodexBackupRollbackState>,
    catalog: crate::codex_model_catalog::managed::AppliedManagedCatalog,
    journal_path: PathBuf,
}

impl CanonicalConfigTransaction {
    pub(crate) fn commit(&self) -> crate::shared::error::AppResult<()> {
        clear_lifecycle_journal(&self.journal_path)
    }

    pub(crate) fn rollback(self) -> crate::shared::error::AppResult<()> {
        self.catalog.rollback()?;
        rollback_canonical_files(
            &self.config_path,
            &self.live_before,
            &self.live_written,
            self.backup.as_ref(),
        )?;
        clear_lifecycle_journal(&self.journal_path)
    }
}

pub(crate) struct CatalogPolicyJournal {
    path: PathBuf,
    journal: CodexLifecycleJournal,
}

impl CatalogPolicyJournal {
    pub(crate) fn mark_catalog_written(&mut self) -> crate::shared::error::AppResult<()> {
        self.journal.phase = CodexLifecycleJournalPhase::CatalogWritten;
        write_lifecycle_journal(&self.path, &self.journal)
    }

    pub(crate) fn clear(&self) -> crate::shared::error::AppResult<()> {
        clear_lifecycle_journal(&self.path)
    }
}

pub(crate) fn begin_catalog_policy_journal_locked<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    operation: &str,
) -> crate::shared::error::AppResult<CatalogPolicyJournal> {
    let path = lifecycle_journal_path(app)?;
    if read_lifecycle_journal(&path)?.is_some() {
        return Err(crate::shared::error::AppError::new(
            "CODEX_LIFECYCLE_RECOVERY_REQUIRED",
            "an interrupted Codex lifecycle operation must be recovered first",
        ));
    }
    let config_path = codex_paths::codex_config_toml_path(app)?;
    let live_before = read_optional_codex_config_file(&config_path)?;
    let journal = CodexLifecycleJournal {
        schema_version: CODEX_LIFECYCLE_JOURNAL_SCHEMA_VERSION,
        kind: CodexLifecycleJournalKind::CatalogPolicy,
        operation: operation.to_string(),
        phase: CodexLifecycleJournalPhase::Planned,
        canonical_sha256: None,
        projected_sha256: None,
        live_before_sha256: optional_sha256(live_before.as_deref()),
        live_after_sha256: None,
        proxy_enabled: super::cli_proxy::codex_enabled_proxy_baseline(app)?.is_some(),
        provider_sync_target: None,
        mcp_manifest_after: None,
    };
    write_lifecycle_journal(&path, &journal)?;
    Ok(CatalogPolicyJournal { path, journal })
}

fn rollback_canonical_files(
    config_path: &Path,
    live_before: &(bool, Option<Vec<u8>>),
    live_written: &[u8],
    backup: Option<&CodexBackupRollbackState>,
) -> crate::shared::error::AppResult<()> {
    if read_optional_codex_config_file(config_path)?.as_deref() != Some(live_written) {
        return Err(crate::shared::error::AppError::new(
            "CODEX_CONFIG_RECOVERY_REQUIRED",
            "Codex config changed after the lifecycle transaction; external bytes were preserved",
        ));
    }
    restore_optional_file(config_path, live_before)?;

    if let Some(backup) = backup {
        if snapshot_optional_file(&backup.snapshot.manifest_path)? != backup.expected_manifest
            || snapshot_optional_file(&backup.snapshot.backup_path)? != backup.expected_backup
        {
            return Err(crate::shared::error::AppError::new(
                "CODEX_CONFIG_RECOVERY_REQUIRED",
                "Codex lifecycle ownership changed before rollback; external bytes were preserved",
            ));
        }
        restore_codex_cli_proxy_backup_snapshot(&backup.snapshot)?;
    }
    Ok(())
}

fn remove_path_if_exists(path: &Path) -> crate::shared::error::AppResult<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => fs::remove_dir_all(path)
            .map_err(|err| format!("failed to remove dir {}: {err}", path.display()).into()),
        Ok(_) => fs::remove_file(path)
            .map_err(|err| format!("failed to remove file {}: {err}", path.display()).into()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("failed to inspect path {}: {err}", path.display()).into()),
    }
}

pub(crate) fn codex_config_next_bytes(
    current: Option<Vec<u8>>,
    patch: CodexConfigPatch,
) -> crate::shared::error::AppResult<Vec<u8>> {
    patch_config_toml(current, patch)
}

pub(crate) fn codex_config_normalize_raw_toml(
    mut toml: String,
) -> crate::shared::error::AppResult<Vec<u8>> {
    ensure_codex_config_len(toml.as_bytes(), "codex config.toml")?;
    let validation = validate_codex_config_toml_raw(&toml);
    if !validation.ok {
        let err = validation.error.unwrap_or(CodexConfigTomlValidationError {
            message: "invalid TOML".to_string(),
            line: None,
            column: None,
        });

        let mut msg = format!("SEC_INVALID_INPUT: invalid config.toml: {}", err.message);
        match (err.line, err.column) {
            (Some(line), Some(column)) => msg.push_str(&format!(" (line {line}, column {column})")),
            (Some(line), None) => msg.push_str(&format!(" (line {line})")),
            _ => {}
        }
        return Err(msg.into());
    }

    if !toml.ends_with('\n') {
        toml.push('\n');
    }
    ensure_codex_config_len(toml.as_bytes(), "codex config.toml")?;
    Ok(toml.into_bytes())
}

pub(crate) fn codex_config_patch_target_provider(
    toml: &str,
) -> crate::shared::error::AppResult<String> {
    crate::infra::codex_provider_sync::codex_provider_target_from_patch_config_text(toml)
}

fn patch_requires_provider_sync(patch: &CodexConfigPatch) -> bool {
    patch.features_remote_compaction.is_some()
}

pub(crate) fn canonical_config_bytes_locked<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<Option<Vec<u8>>> {
    let current = match super::cli_proxy::codex_enabled_proxy_baseline(app)? {
        Some(baseline) => {
            let live = read_optional_codex_config_file(&baseline.config_path)?;
            super::cli_proxy::canonical_codex_config_from_live(
                live.as_deref(),
                baseline.config_bytes.as_deref(),
            )?
        }
        None => {
            let path = codex_paths::codex_config_toml_path(app)?;
            read_optional_codex_config_file(&path)?
        }
    };
    crate::codex_model_catalog::managed::canonicalize_config_bytes(app, current)
}

fn begin_canonical_lifecycle_journal<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    operation: &str,
    canonical: &[u8],
    projected: &[u8],
    live_before: Option<&[u8]>,
    provider_sync_target: Option<String>,
    mcp_manifest_after: Option<serde_json::Value>,
) -> crate::shared::error::AppResult<(PathBuf, CodexLifecycleJournal)> {
    let path = lifecycle_journal_path(app)?;
    if read_lifecycle_journal(&path)?.is_some() {
        return Err(crate::shared::error::AppError::new(
            "CODEX_LIFECYCLE_RECOVERY_REQUIRED",
            "an interrupted Codex lifecycle operation must be recovered first",
        ));
    }
    let journal = CodexLifecycleJournal {
        schema_version: CODEX_LIFECYCLE_JOURNAL_SCHEMA_VERSION,
        kind: CodexLifecycleJournalKind::CanonicalConfig,
        operation: operation.to_string(),
        phase: CodexLifecycleJournalPhase::Planned,
        canonical_sha256: Some(sha256_hex(canonical)),
        projected_sha256: Some(sha256_hex(projected)),
        live_before_sha256: optional_sha256(live_before),
        live_after_sha256: None,
        proxy_enabled: super::cli_proxy::codex_enabled_proxy_baseline(app)?.is_some(),
        provider_sync_target,
        mcp_manifest_after,
    };
    write_lifecycle_journal(&path, &journal)?;
    Ok((path, journal))
}

fn update_lifecycle_journal_phase(
    path: &Path,
    journal: &mut CodexLifecycleJournal,
    phase: CodexLifecycleJournalPhase,
) -> crate::shared::error::AppResult<()> {
    journal.phase = phase;
    write_lifecycle_journal(path, journal)
}

pub(crate) fn recover_interrupted_lifecycle<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<()> {
    let _guard = crate::codex_managed_profiles::lock_profile_lifecycle();
    crate::codex_managed_profiles::ensure_lifecycle_open()?;
    let path = lifecycle_journal_path(app)?;
    let Some(journal) = read_lifecycle_journal(&path)? else {
        return crate::codex_model_catalog::managed::sync_current_locked(app);
    };

    if journal.kind == CodexLifecycleJournalKind::CatalogPolicy {
        crate::codex_model_catalog::managed::sync_current_locked(app)?;
        clear_lifecycle_journal(&path)?;
        return Ok(());
    }

    let config_path = codex_paths::codex_config_toml_path(app)?;
    let current = read_optional_codex_config_file(&config_path)?;
    let current_without_catalog =
        crate::codex_model_catalog::managed::canonicalize_config_bytes(app, current.clone())?;
    let current_hash = optional_sha256(current.as_deref());
    let current_without_catalog_hash = optional_sha256(current_without_catalog.as_deref());
    let current_is_prior = current_hash.as_deref() == journal.live_before_sha256.as_deref();
    let current_is_projected =
        current_without_catalog_hash.as_deref() == journal.projected_sha256.as_deref();
    let current_is_catalog_written =
        current_hash.as_deref() == journal.live_after_sha256.as_deref();
    let expected_canonical = journal.canonical_sha256.as_deref().ok_or_else(|| {
        crate::shared::error::AppError::new(
            "CODEX_LIFECYCLE_RECOVERY_REQUIRED",
            "the Codex lifecycle journal is missing the canonical hash",
        )
    })?;
    let proxy_baseline = super::cli_proxy::codex_enabled_proxy_baseline(app)?;
    if proxy_baseline.is_some() != journal.proxy_enabled {
        return Err(crate::shared::error::AppError::new(
            "CODEX_LIFECYCLE_RECOVERY_REQUIRED",
            "the Codex proxy state changed during interrupted-operation recovery",
        ));
    }

    let canonical_source_hash = match proxy_baseline.as_ref() {
        Some(baseline) => baseline.config_bytes.as_deref().map(sha256_hex),
        None => current_without_catalog_hash.clone(),
    };
    let canonical_source_matches = canonical_source_hash.as_deref() == Some(expected_canonical);
    if current_is_prior && !canonical_source_matches {
        clear_lifecycle_journal(&path)?;
        return crate::codex_model_catalog::managed::sync_current_locked(app);
    }
    if journal.phase == CodexLifecycleJournalPhase::Planned && current_is_prior {
        let canonical_was_written = proxy_baseline
            .as_ref()
            .and_then(|baseline| baseline.config_bytes.as_deref())
            .is_some_and(|bytes| sha256_hex(bytes) == expected_canonical);
        if !canonical_was_written {
            clear_lifecycle_journal(&path)?;
            return crate::codex_model_catalog::managed::sync_current_locked(app);
        }
    }

    let canonical = if let Some(baseline) = proxy_baseline {
        let canonical = baseline.config_bytes.ok_or_else(|| {
            crate::shared::error::AppError::new(
                "CODEX_LIFECYCLE_RECOVERY_REQUIRED",
                "the enabled Codex proxy has no canonical config backup",
            )
        })?;
        if sha256_hex(&canonical) != expected_canonical
            || (!current_is_prior && !current_is_projected && !current_is_catalog_written)
        {
            return Err(crate::shared::error::AppError::new(
                "CODEX_LIFECYCLE_RECOVERY_REQUIRED",
                "Codex lifecycle bytes drifted during interrupted-operation recovery",
            ));
        }
        canonical
    } else {
        let canonical = current_without_catalog.ok_or_else(|| {
            crate::shared::error::AppError::new(
                "CODEX_LIFECYCLE_RECOVERY_REQUIRED",
                "the interrupted Codex canonical config is unavailable",
            )
        })?;
        if sha256_hex(&canonical) != expected_canonical || !current_is_projected {
            return Err(crate::shared::error::AppError::new(
                "CODEX_LIFECYCLE_RECOVERY_REQUIRED",
                "Codex lifecycle bytes drifted during interrupted-operation recovery",
            ));
        }
        canonical
    };

    let projected = super::cli_proxy::project_codex_config_if_enabled(app, canonical)?;
    if sha256_hex(&projected) != journal.projected_sha256.as_deref().unwrap_or_default() {
        return Err(crate::shared::error::AppError::new(
            "CODEX_LIFECYCLE_RECOVERY_REQUIRED",
            "the Codex live projection no longer matches the interrupted operation",
        ));
    }
    if current_without_catalog.as_deref() != Some(projected.as_slice()) {
        if read_optional_codex_config_file(&config_path)? != current {
            return Err(crate::shared::error::AppError::new(
                "CODEX_LIFECYCLE_RECOVERY_REQUIRED",
                "Codex config changed during interrupted-operation recovery",
            ));
        }
        write_file_atomic_if_changed(&config_path, &projected)?;
    }
    crate::codex_model_catalog::managed::sync_current_locked(app)?;

    if let Some(target_provider) = journal.provider_sync_target.as_ref() {
        let final_config = read_optional_codex_config_file(&config_path)?.unwrap_or_default();
        crate::infra::codex_provider_sync::codex_provider_sync(
            app,
            crate::infra::codex_provider_sync::CodexProviderSyncContext {
                trigger: "codex_lifecycle_recovery".to_string(),
                target_provider: target_provider.clone(),
                config_bytes: Some(final_config),
            },
        )?;
    }

    if let Some(manifest) = journal.mcp_manifest_after {
        let mut bytes = serde_json::to_vec_pretty(&manifest).map_err(|_| {
            crate::shared::error::AppError::new(
                "CODEX_LIFECYCLE_RECOVERY_REQUIRED",
                "failed to serialize the recovered Codex MCP manifest",
            )
        })?;
        bytes.push(b'\n');
        crate::mcp_sync::restore_manifest_bytes(app, "codex", Some(bytes))?;
    }
    clear_lifecycle_journal(&path)
}

pub(crate) fn apply_canonical_bytes_locked<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    canonical: Vec<u8>,
    requires_provider_sync: bool,
) -> crate::shared::error::AppResult<CanonicalConfigTransaction> {
    apply_canonical_bytes_with_completion_locked(
        app,
        canonical,
        requires_provider_sync,
        "codex_config",
        None,
    )
}

pub(crate) fn apply_canonical_bytes_with_completion_locked<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    canonical: Vec<u8>,
    requires_provider_sync: bool,
    operation: &str,
    mcp_manifest_after: Option<serde_json::Value>,
) -> crate::shared::error::AppResult<CanonicalConfigTransaction> {
    ensure_codex_config_len(&canonical, "canonical Codex config.toml")?;
    if let Some(baseline) = super::cli_proxy::codex_enabled_proxy_baseline(app)? {
        if !super::cli_proxy::codex_proxy_config_is_applied(app, &baseline.base_origin) {
            return Err(crate::shared::error::AppError::new(
                "CODEX_CONFIG_RECOVERY_REQUIRED",
                "the Codex proxy projection changed externally; refusing to overwrite it",
            ));
        }
    }
    let _preflight = crate::codex_model_catalog::managed::prepare_current_locked(app)?;
    let config_path = codex_paths::codex_config_toml_path(app)?;
    let live_before = snapshot_optional_file(&config_path)?;
    let live_written = super::cli_proxy::project_codex_config_if_enabled(app, canonical.clone())?;
    ensure_codex_config_len(&live_written, "projected Codex config.toml")?;
    let provider_sync_target = if requires_provider_sync {
        Some(codex_config_patch_target_provider(
            std::str::from_utf8(&canonical)
                .map_err(|_| "SEC_INVALID_INPUT: codex config.toml must be valid UTF-8")?,
        )?)
    } else {
        None
    };
    let (journal_path, mut journal) = begin_canonical_lifecycle_journal(
        app,
        operation,
        &canonical,
        &live_written,
        live_before.1.as_deref(),
        provider_sync_target.clone(),
        mcp_manifest_after,
    )?;
    interrupt_lifecycle_for_tests("planned")?;
    let backup_snapshot = sync_codex_cli_proxy_backup_if_enabled(app, &canonical)?;
    let backup = match backup_snapshot
        .map(|snapshot| {
            Ok(CodexBackupRollbackState {
                expected_manifest: snapshot_optional_file(&snapshot.manifest_path)?,
                expected_backup: snapshot_optional_file(&snapshot.backup_path)?,
                snapshot,
            })
        })
        .transpose()
    {
        Ok(backup) => backup,
        Err(error) => return Err(error),
    };
    if let Err(error) = update_lifecycle_journal_phase(
        &journal_path,
        &mut journal,
        CodexLifecycleJournalPhase::CanonicalWritten,
    ) {
        if let Some(backup) = backup.as_ref() {
            restore_codex_cli_proxy_backup_snapshot(&backup.snapshot)?;
        }
        clear_lifecycle_journal(&journal_path)?;
        return Err(error);
    }
    interrupt_lifecycle_for_tests("canonical_written")?;
    if let Err(error) = write_file_atomic_if_changed(&config_path, &live_written) {
        if let Some(backup) = backup.as_ref() {
            restore_codex_cli_proxy_backup_snapshot(&backup.snapshot)?;
        }
        clear_lifecycle_journal(&journal_path)?;
        return Err(error);
    }
    if let Err(error) = update_lifecycle_journal_phase(
        &journal_path,
        &mut journal,
        CodexLifecycleJournalPhase::LiveWritten,
    ) {
        rollback_canonical_files(
            &config_path,
            &live_before,
            &live_written,
            backup.as_ref(),
        )?;
        clear_lifecycle_journal(&journal_path)?;
        return Err(error);
    }
    interrupt_lifecycle_for_tests("live_written")?;

    let plan = match crate::codex_model_catalog::managed::prepare_current_locked(app) {
        Ok(plan) => plan,
        Err(error) => {
            rollback_canonical_files(
                &config_path,
                &live_before,
                &live_written,
                backup.as_ref(),
            )?;
            clear_lifecycle_journal(&journal_path)?;
            return Err(error);
        }
    };
    let catalog = match plan.apply(app) {
        Ok(catalog) => catalog,
        Err(error) => {
            rollback_canonical_files(
                &config_path,
                &live_before,
                &live_written,
                backup.as_ref(),
            )?;
            clear_lifecycle_journal(&journal_path)?;
            return Err(error);
        }
    };
    let live_after = match read_optional_codex_config_file(&config_path) {
        Ok(live_after) => live_after,
        Err(error) => {
            catalog.rollback()?;
            rollback_canonical_files(
                &config_path,
                &live_before,
                &live_written,
                backup.as_ref(),
            )?;
            clear_lifecycle_journal(&journal_path)?;
            return Err(error);
        }
    };
    journal.live_after_sha256 = optional_sha256(live_after.as_deref());
    if let Err(error) = update_lifecycle_journal_phase(
        &journal_path,
        &mut journal,
        CodexLifecycleJournalPhase::CatalogWritten,
    ) {
        catalog.rollback()?;
        rollback_canonical_files(
            &config_path,
            &live_before,
            &live_written,
            backup.as_ref(),
        )?;
        clear_lifecycle_journal(&journal_path)?;
        return Err(error);
    }
    interrupt_lifecycle_for_tests("catalog_written")?;
    let transaction = CanonicalConfigTransaction {
        config_path: config_path.clone(),
        live_before,
        live_written,
        backup,
        catalog,
        journal_path,
    };

    if let Some(target_provider) = provider_sync_target {
        let final_config = read_optional_codex_config_file(&config_path)?.unwrap_or_default();
        if let Err(error) = crate::infra::codex_provider_sync::codex_provider_sync(
            app,
            crate::infra::codex_provider_sync::CodexProviderSyncContext {
                trigger: "codex_config_set".to_string(),
                target_provider,
                config_bytes: Some(final_config),
            },
        ) {
            transaction.rollback()?;
            return Err(error);
        }
    }

    Ok(transaction)
}

#[cfg(windows)]
fn normalize_path_for_prefix_match(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_lowercase()
}

#[cfg(windows)]
fn path_is_under_allowed_root(dir: &Path, allowed_root: &Path) -> bool {
    let dir_s = normalize_path_for_prefix_match(dir);
    let root_s = normalize_path_for_prefix_match(allowed_root);
    dir_s == root_s || dir_s.starts_with(&(root_s + "/"))
}

#[cfg(not(windows))]
fn path_is_under_allowed_root(dir: &Path, allowed_root: &Path) -> bool {
    dir.starts_with(allowed_root)
}

fn codex_config_get_locked<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<CodexConfigState> {
    let path = codex_paths::codex_config_toml_path(app)?;
    let dir = path.parent().unwrap_or(Path::new("")).to_path_buf();
    let user_default_path = codex_paths::codex_home_dir_user_default(app)?.join("config.toml");
    let user_default_dir = user_default_path
        .parent()
        .unwrap_or(Path::new(""))
        .to_path_buf();
    let follow_path = codex_paths::codex_home_dir_follow_env_or_default(app)?.join("config.toml");
    let follow_dir = follow_path.parent().unwrap_or(Path::new("")).to_path_buf();
    let bytes = canonical_config_bytes_locked(app)?;

    let can_open_config_dir = crate::app_paths::home_dir(app)
        .ok()
        .map(|home| {
            let allowed_root = home.join(".codex");
            path_is_under_allowed_root(&dir, &allowed_root)
                || follow_dir == dir
                || codex_paths::configured_codex_home_dir(app)
                    .as_ref()
                    .is_some_and(|configured_dir| configured_dir == &dir)
        })
        .unwrap_or(false);

    make_state_from_bytes(
        CodexConfigStateMeta {
            config_dir: dir.to_string_lossy().to_string(),
            config_path: path.to_string_lossy().to_string(),
            user_home_default_dir: user_default_dir.to_string_lossy().to_string(),
            user_home_default_path: user_default_path.to_string_lossy().to_string(),
            follow_codex_home_dir: follow_dir.to_string_lossy().to_string(),
            follow_codex_home_path: follow_path.to_string_lossy().to_string(),
            can_open_config_dir,
        },
        bytes,
    )
}

pub fn codex_config_get<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<CodexConfigState> {
    let _guard = crate::codex_managed_profiles::lock_profile_lifecycle();
    codex_config_get_locked(app)
}

fn codex_config_toml_get_raw_locked<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<CodexConfigTomlState> {
    let path = codex_paths::codex_config_toml_path(app)?;
    let bytes = canonical_config_bytes_locked(app)?;
    let exists = bytes.is_some();

    let toml = match bytes {
        Some(bytes) => String::from_utf8(bytes)
            .map_err(|_| "SEC_INVALID_INPUT: codex config.toml must be valid UTF-8".to_string())?,
        None => String::new(),
    };

    Ok(CodexConfigTomlState {
        config_path: path.to_string_lossy().to_string(),
        exists,
        toml,
    })
}

pub fn codex_config_toml_get_raw<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<CodexConfigTomlState> {
    let _guard = crate::codex_managed_profiles::lock_profile_lifecycle();
    codex_config_toml_get_raw_locked(app)
}

pub fn codex_config_toml_validate_raw(
    toml: String,
) -> crate::shared::error::AppResult<CodexConfigTomlValidationResult> {
    Ok(validate_codex_config_toml_raw(&toml))
}

pub fn codex_config_toml_set_raw<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    toml: String,
) -> crate::shared::error::AppResult<CodexConfigState> {
    let _guard = crate::codex_managed_profiles::lock_profile_lifecycle();
    crate::codex_managed_profiles::ensure_lifecycle_open()?;
    let path = codex_paths::codex_config_toml_path(app)?;
    if path.exists() && is_symlink(&path)? {
        return Err(format!(
            "SEC_INVALID_INPUT: refusing to modify symlink path={}",
            path.display()
        )
        .into());
    }

    let bytes = codex_config_normalize_raw_toml(toml)?;
    let transaction = apply_canonical_bytes_locked(app, bytes, false)?;
    if let Err(error) = transaction.commit() {
        transaction.rollback()?;
        return Err(error);
    }
    codex_config_get_locked(app)
}

pub fn codex_config_set<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    patch: CodexConfigPatch,
) -> crate::shared::error::AppResult<CodexConfigState> {
    let _guard = crate::codex_managed_profiles::lock_profile_lifecycle();
    crate::codex_managed_profiles::ensure_lifecycle_open()?;
    let path = codex_paths::codex_config_toml_path(app)?;
    if path.exists() && is_symlink(&path)? {
        return Err(format!(
            "SEC_INVALID_INPUT: refusing to modify symlink path={}",
            path.display()
        )
        .into());
    }

    let current = canonical_config_bytes_locked(app)?;
    let requires_provider_sync = patch_requires_provider_sync(&patch);
    let next = codex_config_next_bytes(current, patch)?;
    ensure_codex_config_len(&next, "codex config.toml")?;
    let transaction = apply_canonical_bytes_locked(app, next, requires_provider_sync)?;
    if let Err(error) = transaction.commit() {
        transaction.rollback()?;
        return Err(error);
    }
    codex_config_get_locked(app)
}

#[cfg(test)]
mod tests;
