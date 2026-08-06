use super::*;

#[test]
fn target_provider_rejects_unmanaged_raw_toml() {
    let err = codex_provider_target_from_config_text(
        "model_provider = \"Anthropic\"\n[model_providers.Anthropic]\nname = \"Anthropic\"\n",
    )
    .expect_err("unsupported raw config should fail");

    assert!(
        err.to_string()
            .contains("CODEX_PROVIDER_SYNC_INVALID_TARGET"),
        "{err}"
    );
}

#[test]
fn target_provider_parses_toml_comments() {
    assert_eq!(
        codex_provider_target_from_config_text(
            "model_provider = \"OpenAI\" # keep remote compaction provider\n\
             [model_providers.OpenAI]\n\
             name = \"OpenAI\"\n",
        )
        .expect("commented model_provider should parse"),
        "OpenAI"
    );
}

#[test]
fn current_config_provider_defaults_to_aio_when_missing() {
    assert_eq!(
        codex_provider_target_from_current_config_text("approval_policy = \"on-request\"\n")
            .expect("valid missing-provider config should default"),
        "aio"
    );
}

#[test]
fn current_config_provider_rejects_invalid_toml() {
    let err = codex_provider_target_from_current_config_text("model_provider =")
        .expect_err("invalid TOML should fail closed");
    assert!(
        err.to_string()
            .contains("CODEX_PROVIDER_SYNC_INVALID_CONFIG"),
        "{err}"
    );
}

fn write_managed_manifest(dir: &Path, version: u8, created_at: &str, managed_by: &str) {
    std::fs::create_dir_all(dir).expect("create backup dir");
    let mut manifest = serde_json::json!({
        "version": version,
        "trigger": "test",
        "target_provider": "OpenAI",
        "created_at": created_at,
        "managed_by": managed_by,
        "config_path": null,
        "session_files": [],
    });
    if version == 1 {
        manifest["sqlite_files"] = serde_json::json!([]);
        manifest["global_state_path"] = serde_json::Value::Null;
    } else if version == PROVIDER_SYNC_BACKUP_VERSION {
        manifest["scope"] = serde_json::json!(PROVIDER_SYNC_BACKUP_SCOPE);
    }
    std::fs::write(
        dir.join(PROVIDER_SYNC_MANAGED_BACKUP_MANIFEST),
        serde_json::to_vec(&manifest).expect("serialize manifest"),
    )
    .expect("write manifest");
}

#[cfg(windows)]
fn symlink_test_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(src, dst)
}

#[cfg(not(windows))]
fn symlink_test_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(src, dst)
}

#[cfg(windows)]
fn symlink_test_file(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(src, dst)
}

#[cfg(not(windows))]
fn symlink_test_file(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(src, dst)
}

#[test]
fn backup_pruning_keeps_current_v2_and_preserves_unmanaged_entries() {
    let temp = tempfile::tempdir().expect("tempdir");
    let home = temp.path();
    let root = home.join(PROVIDER_SYNC_BACKUP_ROOT);
    std::fs::create_dir_all(&root).expect("create backup root");

    let legacy_v1 = root.join("legacy-v1");
    write_managed_manifest(&legacy_v1, 1, "1", PROVIDER_SYNC_MANAGED_BY);
    let old_v2 = root.join("old-v2");
    write_managed_manifest(
        &old_v2,
        PROVIDER_SYNC_BACKUP_VERSION,
        "2",
        PROVIDER_SYNC_MANAGED_BY,
    );
    let current_v2 = root.join("current-v2");
    write_managed_manifest(
        &current_v2,
        PROVIDER_SYNC_BACKUP_VERSION,
        "3",
        PROVIDER_SYNC_MANAGED_BY,
    );

    let no_manifest = root.join("no-manifest");
    std::fs::create_dir_all(&no_manifest).expect("create unmanaged dir");
    let corrupt = root.join("corrupt");
    std::fs::create_dir_all(&corrupt).expect("create corrupt dir");
    std::fs::write(
        corrupt.join(PROVIDER_SYNC_MANAGED_BACKUP_MANIFEST),
        b"{not-json",
    )
    .expect("write corrupt manifest");
    let wrong_marker = root.join("wrong-marker");
    write_managed_manifest(&wrong_marker, 1, "4", "someone else");
    let invalid_created_at = root.join("invalid-created-at");
    write_managed_manifest(
        &invalid_created_at,
        1,
        "not-a-number",
        PROVIDER_SYNC_MANAGED_BY,
    );
    let future_version = root.join("future-version");
    write_managed_manifest(&future_version, 3, "6", PROVIDER_SYNC_MANAGED_BY);

    let manifest_symlink = root.join("manifest-symlink");
    std::fs::create_dir_all(&manifest_symlink).expect("create manifest symlink dir");
    let external_manifest = home.join("external-manifest.json");
    std::fs::write(
        &external_manifest,
        serde_json::json!({
            "version": 1,
            "trigger": "test",
            "target_provider": "OpenAI",
            "created_at": "7",
            "managed_by": PROVIDER_SYNC_MANAGED_BY,
            "config_path": null,
            "session_files": [],
            "sqlite_files": [],
            "global_state_path": null,
        })
        .to_string(),
    )
    .expect("write external manifest");
    let manifest_symlink_created = symlink_test_file(
        &external_manifest,
        &manifest_symlink.join(PROVIDER_SYNC_MANAGED_BACKUP_MANIFEST),
    )
    .map(|_| true)
    .unwrap_or(false);

    let managed_with_symlink = root.join("managed-with-symlink");
    write_managed_manifest(&managed_with_symlink, 1, "5", PROVIDER_SYNC_MANAGED_BY);
    let outside = home.join("outside");
    std::fs::create_dir_all(&outside).expect("create outside dir");
    let child_symlink_created = symlink_test_dir(&outside, &managed_with_symlink.join("external"))
        .map(|_| true)
        .unwrap_or(false);

    let root_symlink = root.join("root-symlink");
    let root_symlink_created = symlink_test_dir(&outside, &root_symlink)
        .map(|_| true)
        .unwrap_or(false);

    let warning = prune_managed_backups(home, &current_v2).expect("prune");
    if child_symlink_created {
        assert!(warning.is_some(), "symlink preservation should be reported");
    } else {
        assert!(warning.is_none(), "{warning:?}");
    }

    assert!(!legacy_v1.exists(), "legacy v1 should be migrated away");
    assert!(!old_v2.exists(), "older v2 should be replaced");
    assert!(current_v2.exists(), "current v2 must be retained");
    assert!(no_manifest.exists(), "manifest-less directory is unmanaged");
    assert!(corrupt.exists(), "corrupt manifest is unmanaged");
    assert!(wrong_marker.exists(), "marker mismatch is unmanaged");
    assert!(
        invalid_created_at.exists(),
        "invalid ownership metadata is unmanaged"
    );
    assert!(
        future_version.exists(),
        "future manifest version is unmanaged"
    );
    if manifest_symlink_created {
        assert!(
            manifest_symlink.exists(),
            "symlinked manifest must be preserved"
        );
        assert!(
            external_manifest.exists(),
            "manifest symlink target must be preserved"
        );
    }
    if child_symlink_created {
        assert!(
            managed_with_symlink.exists(),
            "managed trees containing symlinks must be preserved"
        );
    }
    if root_symlink_created {
        assert!(
            std::fs::symlink_metadata(&root_symlink).is_ok(),
            "symlink backup entry must be preserved"
        );
    }
    assert!(outside.exists(), "pruning must not touch symlink targets");
}

#[test]
fn backup_removal_revalidates_an_entry_replaced_after_classification() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join(PROVIDER_SYNC_BACKUP_ROOT);
    std::fs::create_dir_all(&root).expect("create backup root");
    let candidate = root.join("candidate");
    write_managed_manifest(&candidate, 1, "1", PROVIDER_SYNC_MANAGED_BY);
    let expected = managed_backup_version(&candidate)
        .expect("classify candidate")
        .expect("managed candidate");

    let displaced_managed = root.join("displaced-managed");
    std::fs::rename(&candidate, &displaced_managed).expect("replace candidate");
    std::fs::create_dir_all(&candidate).expect("create replacement directory");
    std::fs::write(candidate.join("user-notes.txt"), b"keep me").expect("write replacement data");

    let warning = remove_managed_backup_candidate(&root, &candidate, expected)
        .expect("safe candidate removal");

    assert!(warning.is_some(), "replacement should be reported");
    assert_eq!(
        std::fs::read(candidate.join("user-notes.txt")).expect("replacement must survive"),
        b"keep me".to_vec()
    );
    assert!(
        displaced_managed.exists(),
        "the separately displaced managed backup is outside this removal"
    );
}

fn session_change(path: &Path, original: &[u8], next: &[u8]) -> SessionChange {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create session parent");
    }
    std::fs::write(path, original).expect("write original session");
    SessionChange {
        path: path.to_path_buf(),
        original_text: original.to_vec(),
        next_text: next.to_vec(),
    }
}

#[test]
fn second_session_write_failure_restores_config_and_first_session() {
    let temp = tempfile::tempdir().expect("tempdir");
    let home = temp.path();
    let config_path = home.join("config.toml");
    let original_config = b"model_provider = \"aio\"\n";
    let next_config = b"model_provider = \"OpenAI\"\n";
    std::fs::write(&config_path, original_config).expect("write config");
    let first_path = home.join("sessions/rollout-first.jsonl");
    let second_path = home.join("sessions/rollout-second.jsonl");
    let first_original = b"first original\n";
    let second_original = b"second original\n";
    let change_set = SyncChangeSet {
        config_bytes: Some(next_config.to_vec()),
        session_changes: vec![
            session_change(&first_path, first_original, b"first changed\n"),
            session_change(&second_path, second_original, b"second changed\n"),
        ],
    };
    let context = CodexProviderSyncContext {
        trigger: "test".to_string(),
        target_provider: "OpenAI".to_string(),
        config_bytes: Some(next_config.to_vec()),
    };
    let backup_dir = create_backup(home, &context, &change_set).expect("create diagnostic backup");

    let mut writes = 0usize;
    let err = apply_file_changes_with(
        &config_path,
        &change_set,
        |path, bytes| -> AppResult<bool> {
            writes += 1;
            if writes == 3 {
                return Err("injected second session write failure".into());
            }
            std::fs::write(path, bytes).map_err(|error| format!("test write failed: {error}"))?;
            Ok(true)
        },
    )
    .expect_err("second session write should fail");

    assert!(err.to_string().contains("injected second session"), "{err}");
    assert_eq!(writes, 3);
    assert_eq!(
        std::fs::read(&config_path).expect("read config"),
        original_config.to_vec()
    );
    assert_eq!(
        std::fs::read(&first_path).expect("read first"),
        first_original.to_vec()
    );
    assert_eq!(
        std::fs::read(&second_path).expect("read second"),
        second_original.to_vec()
    );
    assert!(backup_dir.join("provider-sync.json").exists());
    assert_eq!(
        std::fs::read(backup_dir.join("sessions/rollout-first.jsonl")).expect("read first backup"),
        first_original.to_vec()
    );
}

#[test]
fn config_write_failure_restores_snapshot_and_keeps_diagnostic_backup() {
    let temp = tempfile::tempdir().expect("tempdir");
    let home = temp.path();
    let config_path = home.join("config.toml");
    let original_config = b"model_provider = \"aio\"\n";
    let next_config = b"model_provider = \"OpenAI\"\n";
    std::fs::write(&config_path, original_config).expect("write config");
    let session_path = home.join("sessions/rollout-config-failure.jsonl");
    let session_original = b"session original\n";
    let change_set = SyncChangeSet {
        config_bytes: Some(next_config.to_vec()),
        session_changes: vec![session_change(
            &session_path,
            session_original,
            b"session changed\n",
        )],
    };
    let context = CodexProviderSyncContext {
        trigger: "test".to_string(),
        target_provider: "OpenAI".to_string(),
        config_bytes: Some(next_config.to_vec()),
    };
    let backup_dir = create_backup(home, &context, &change_set).expect("create diagnostic backup");

    let err = apply_file_changes_with(
        &config_path,
        &change_set,
        |_path, _bytes| -> AppResult<bool> { Err("injected config write failure".into()) },
    )
    .expect_err("config write should fail");

    assert!(err.to_string().contains("injected config"), "{err}");
    assert_eq!(
        std::fs::read(&config_path).expect("read config"),
        original_config.to_vec()
    );
    assert_eq!(
        std::fs::read(&session_path).expect("read session"),
        session_original.to_vec()
    );
    assert!(backup_dir.join("provider-sync.json").exists());
    assert_eq!(
        std::fs::read(backup_dir.join("config.toml")).expect("read config backup"),
        original_config.to_vec()
    );
}

#[test]
fn running_app_override_blocks_sync() {
    let temp = tempfile::tempdir().expect("tempdir");
    let home = temp.path();
    std::fs::create_dir_all(home.join("tmp")).expect("create tmp");

    crate::test_support::codex_provider_sync_set_running_override_for_tests(Some(true));
    let is_running = codex_app_is_running().expect("override should not query process list");
    crate::test_support::codex_provider_sync_set_running_override_for_tests(None);

    assert!(is_running, "override should force running state");
}

#[test]
fn process_check_failed_message_explains_next_step() {
    let message = codex_process_check_failed_message("tasklist", "exit status 1");

    assert!(
        message.contains("CODEX_PROVIDER_SYNC_PROCESS_CHECK_FAILED"),
        "{message}"
    );
    assert!(
        message.contains("unable to verify whether Codex App is closed"),
        "{message}"
    );
    assert!(message.contains("tasklist"), "{message}");
    assert!(
        message.contains("Please confirm Codex App is fully closed, then retry."),
        "{message}"
    );
}
