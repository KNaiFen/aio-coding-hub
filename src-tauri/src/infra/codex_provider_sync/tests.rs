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

#[test]
fn backup_prune_limits_accept_exact_boundaries_and_reject_overflow() {
    let exact_depth = ProviderSyncPruneBudget::default();
    exact_depth
        .ensure_depth(PROVIDER_SYNC_PRUNE_MAX_DEPTH)
        .expect("maximum supported depth should be accepted");
    assert!(
        exact_depth
            .ensure_depth(PROVIDER_SYNC_PRUNE_MAX_DEPTH + 1)
            .is_err(),
        "depth above the limit must fail closed"
    );

    let mut exact_entries = ProviderSyncPruneBudget {
        entries_seen: PROVIDER_SYNC_PRUNE_MAX_ENTRIES - 1,
        ..ProviderSyncPruneBudget::tree_limits()
    };
    exact_entries
        .record_entry()
        .expect("maximum supported entry count should be accepted");
    assert_eq!(exact_entries.entries_seen, PROVIDER_SYNC_PRUNE_MAX_ENTRIES);
    assert!(
        exact_entries.record_entry().is_err(),
        "entry count above the limit must fail closed"
    );

    let mut overflow = ProviderSyncPruneBudget {
        entries_seen: usize::MAX,
        ..Default::default()
    };
    assert!(
        overflow.record_entry().is_err(),
        "entry counter overflow must fail closed"
    );
}

#[test]
fn backup_tree_capture_enforces_real_depth_entry_and_hash_budgets() {
    let temp = tempfile::tempdir().expect("tempdir");

    let depth_root = temp.path().join("depth-root");
    std::fs::create_dir_all(depth_root.join("one/two")).expect("create depth tree");
    let depth_handle =
        open_provider_sync_backup_dir_no_follow(&depth_root).expect("open depth tree");
    let mut exact_depth = ProviderSyncPruneBudget::with_limits(2, 8, 1024, 4096);
    capture_provider_sync_backup_tree(&depth_handle, &mut exact_depth)
        .expect("exact recursive depth should be accepted");
    drop(depth_handle);
    let depth_handle =
        open_provider_sync_backup_dir_no_follow(&depth_root).expect("reopen depth tree");
    let mut overflow_depth = ProviderSyncPruneBudget::with_limits(1, 8, 1024, 4096);
    assert!(
        capture_provider_sync_backup_tree(&depth_handle, &mut overflow_depth).is_err(),
        "recursive depth above the configured limit must fail closed"
    );

    let entry_root = temp.path().join("entry-root");
    std::fs::create_dir_all(&entry_root).expect("create entry tree");
    for name in ["one", "two", "three"] {
        std::fs::write(entry_root.join(name), b"x").expect("write entry");
    }
    let entry_handle =
        open_provider_sync_backup_dir_no_follow(&entry_root).expect("open entry tree");
    let mut exact_entries = ProviderSyncPruneBudget::with_limits(1, 3, 1024, 4096);
    capture_provider_sync_backup_tree(&entry_handle, &mut exact_entries)
        .expect("exact entry count should be accepted");
    drop(entry_handle);
    let entry_handle =
        open_provider_sync_backup_dir_no_follow(&entry_root).expect("reopen entry tree");
    let mut overflow_entries = ProviderSyncPruneBudget::with_limits(1, 2, 1024, 4096);
    assert!(
        capture_provider_sync_backup_tree(&entry_handle, &mut overflow_entries).is_err(),
        "entry count above the configured limit must fail closed"
    );

    let hash_root = temp.path().join("hash-root");
    std::fs::create_dir_all(&hash_root).expect("create hash tree");
    std::fs::write(hash_root.join("one"), b"1234").expect("write first hash entry");
    std::fs::write(hash_root.join("two"), b"5678").expect("write second hash entry");
    let hash_handle = open_provider_sync_backup_dir_no_follow(&hash_root).expect("open hash tree");
    let mut exact_hash = ProviderSyncPruneBudget::with_limits(1, 2, 4, 8);
    capture_provider_sync_backup_tree(&hash_handle, &mut exact_hash)
        .expect("exact file and aggregate hash bytes should be accepted");
    drop(hash_handle);
    let hash_handle =
        open_provider_sync_backup_dir_no_follow(&hash_root).expect("reopen hash tree");
    let mut oversized_file = ProviderSyncPruneBudget::with_limits(1, 2, 3, 8);
    assert!(
        capture_provider_sync_backup_tree(&hash_handle, &mut oversized_file).is_err(),
        "a file above the configured hash size must fail closed"
    );
    drop(hash_handle);
    let hash_handle =
        open_provider_sync_backup_dir_no_follow(&hash_root).expect("reopen hash tree");
    let mut oversized_tree = ProviderSyncPruneBudget::with_limits(1, 2, 4, 7);
    assert!(
        capture_provider_sync_backup_tree(&hash_handle, &mut oversized_tree).is_err(),
        "aggregate hash bytes above the configured limit must fail closed"
    );
}

#[test]
fn backup_root_enumeration_is_bounded_before_candidate_collection() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join("root");
    for name in ["one", "two", "three"] {
        std::fs::create_dir_all(root.join(name)).expect("create root entry");
    }
    let root_handle = open_provider_sync_backup_dir_no_follow(&root).expect("open root");
    let mut exact = ProviderSyncPruneBudget::with_limits(1, 3, 1024, 4096);
    let names = provider_sync_backup_root_directory_names(&root_handle, &mut exact)
        .expect("exact root entry count should be accepted");
    assert_eq!(names.len(), 3);

    drop(root_handle);
    let root_handle = open_provider_sync_backup_dir_no_follow(&root).expect("reopen root");
    let mut overflow = ProviderSyncPruneBudget::with_limits(1, 2, 1024, 4096);
    assert!(
        provider_sync_backup_root_directory_names(&root_handle, &mut overflow).is_err(),
        "root enumeration must fail before collecting an unbounded candidate list"
    );
}

#[test]
fn full_candidate_work_fits_without_relaxing_single_tree_limits() {
    let mut operation_budget = ProviderSyncPruneBudget::default();
    let mut full_root_enumeration = ProviderSyncPruneBudget::tree_limits();
    full_root_enumeration.entries_seen = PROVIDER_SYNC_PRUNE_MAX_ENTRIES;
    operation_budget
        .consume(&full_root_enumeration)
        .expect("full root enumeration should fit");

    let mut full_candidate_classification = ProviderSyncPruneBudget::tree_limits();
    full_candidate_classification.entries_seen = PROVIDER_SYNC_PRUNE_MAX_ENTRIES;
    operation_budget
        .consume(&full_candidate_classification)
        .expect("full candidate classification should fit");
    operation_budget
        .reserve_file_hash(PROVIDER_SYNC_MAX_BYTES as u64)
        .expect("classification manifest should fit");

    operation_budget
        .reserve_file_hash(PROVIDER_SYNC_MAX_BYTES as u64)
        .expect("first ownership manifest should fit");
    let mut full_tree_snapshot = ProviderSyncPruneBudget::tree_limits();
    full_tree_snapshot.entries_seen = PROVIDER_SYNC_PRUNE_MAX_ENTRIES;
    for _ in 0..4 {
        full_tree_snapshot
            .reserve_file_hash(PROVIDER_SYNC_PRUNE_MAX_FILE_BYTES)
            .expect("four bounded files should fill the tree hash budget");
    }
    operation_budget
        .consume(&full_tree_snapshot)
        .expect("full initial tree snapshot should fit");
    operation_budget
        .reserve_file_hash(PROVIDER_SYNC_MAX_BYTES as u64)
        .expect("second ownership manifest should fit");

    let future_entries = PROVIDER_SYNC_PRUNE_MAX_ENTRIES * 2;
    let future_hashed_bytes =
        PROVIDER_SYNC_PRUNE_MAX_TREE_HASHED_BYTES * 5 + PROVIDER_SYNC_MAX_BYTES as u64 * 4;
    operation_budget
        .ensure_capacity(future_entries, future_hashed_bytes)
        .expect("a full legal tree must reserve all remaining work before isolation");
    assert!(
        operation_budget
            .ensure_capacity(future_entries + 1, future_hashed_bytes)
            .is_err(),
        "the aggregate work budget must remain bounded"
    );

    assert!(
        full_tree_snapshot.reserve_file_hash(1).is_err(),
        "aggregate operation capacity must not relax the single-tree byte limit"
    );
    assert!(
        full_tree_snapshot.record_entry().is_err(),
        "aggregate operation capacity must not relax the single-tree entry limit"
    );
}

#[test]
fn prune_budget_exhaustion_preserves_current_and_existing_backups() {
    let temp = tempfile::tempdir().expect("tempdir");
    let home = temp.path();
    let root = home.join(PROVIDER_SYNC_BACKUP_ROOT);
    std::fs::create_dir_all(&root).expect("create backup root");
    let old = root.join("old");
    write_managed_manifest(&old, 1, "1", PROVIDER_SYNC_MANAGED_BY);
    let current = root.join("current");
    write_managed_manifest(
        &current,
        PROVIDER_SYNC_BACKUP_VERSION,
        "2",
        PROVIDER_SYNC_MANAGED_BY,
    );

    let mut budget = ProviderSyncPruneBudget::with_limits(8, 1, 1024, 4096);
    let warning = prune_managed_backups_with_budget(home, &current, &mut budget)
        .expect("budget exhaustion should remain a non-fatal prune warning");

    assert!(
        warning
            .as_deref()
            .is_some_and(|value| value.contains("root enumeration exhausted")),
        "{warning:?}"
    );
    assert!(old.exists(), "existing managed backup must be preserved");
    assert!(current.exists(), "current managed backup must be preserved");
}

#[test]
fn removal_budget_is_reserved_before_candidate_isolation() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join(PROVIDER_SYNC_BACKUP_ROOT);
    std::fs::create_dir_all(&root).expect("create backup root");
    let candidate = root.join("candidate");
    write_managed_manifest(&candidate, 1, "1", PROVIDER_SYNC_MANAGED_BY);
    std::fs::write(candidate.join("config.toml"), b"managed payload")
        .expect("write managed payload");
    let expected = managed_backup_version(&candidate)
        .expect("classify candidate")
        .expect("managed candidate");
    let root_handle = open_provider_sync_backup_dir_no_follow(&root).expect("open backup root");
    let mut budget = ProviderSyncPruneBudget::with_limits(
        8,
        100,
        PROVIDER_SYNC_MAX_BYTES as u64,
        (PROVIDER_SYNC_MAX_BYTES as u64) * 2,
    );

    let warning = remove_managed_backup_candidate_with_root(
        &root,
        &root_handle,
        &candidate,
        expected,
        &mut budget,
    )
    .expect("budget exhaustion should be a non-fatal prune warning");

    assert!(
        warning
            .as_deref()
            .is_some_and(|value| value.contains("would exhaust the prune budget")),
        "{warning:?}"
    );
    assert!(
        candidate.exists(),
        "candidate must remain at its original path"
    );
    assert!(
        std::fs::read_dir(&root)
            .expect("read backup root")
            .filter_map(Result::ok)
            .all(|entry| !entry.file_name().to_string_lossy().contains("prune")),
        "budget rejection must happen before a quarantine is created"
    );
}

#[test]
fn regular_file_fingerprint_binds_equal_length_contents() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join("root");
    std::fs::create_dir_all(&root).expect("create root");
    let payload = root.join("payload");
    std::fs::write(&payload, b"before").expect("write original payload");
    let root_handle = open_provider_sync_backup_dir_no_follow(&root).expect("open root");
    let original =
        open_provider_sync_backup_child_no_follow(&root_handle, OsStr::new("payload"), false)
            .expect("open original payload");
    let mut original_budget = ProviderSyncPruneBudget::with_limits(1, 1, 1024, 4096);
    let original_fingerprint =
        provider_sync_file_fingerprint_from_handle(&original, false, &mut original_budget)
            .expect("fingerprint original payload");
    drop(original);

    std::fs::write(&payload, b"after!").expect("rewrite payload at the same length");
    let changed =
        open_provider_sync_backup_child_no_follow(&root_handle, OsStr::new("payload"), false)
            .expect("open changed payload");
    let mut changed_budget = ProviderSyncPruneBudget::with_limits(1, 1, 1024, 4096);
    let changed_fingerprint =
        provider_sync_file_fingerprint_from_handle(&changed, false, &mut changed_budget)
            .expect("fingerprint changed payload");

    assert_eq!(original_fingerprint.identity, changed_fingerprint.identity);
    assert_eq!(original_fingerprint.size, changed_fingerprint.size);
    assert_ne!(
        original_fingerprint.content_sha256, changed_fingerprint.content_sha256,
        "equal-length in-place rewrites must change the content fingerprint"
    );
}

#[cfg(windows)]
#[test]
fn windows_backup_fingerprint_uses_change_time_for_metadata_changes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join("root");
    std::fs::create_dir_all(&root).expect("create root");
    let payload = root.join("payload");
    std::fs::write(&payload, b"before").expect("write original payload");
    let root_handle = open_provider_sync_backup_dir_no_follow(&root).expect("open root");
    let original =
        open_provider_sync_backup_child_no_follow(&root_handle, OsStr::new("payload"), false)
            .expect("open original payload");
    let original_metadata = provider_sync_file_metadata_fingerprint_from_handle(&original)
        .expect("read original change time");
    drop(original);

    let mut permissions = std::fs::metadata(&payload)
        .expect("read payload metadata")
        .permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&payload, permissions.clone())
        .expect("set readonly payload attribute");
    let changed =
        open_provider_sync_backup_child_no_follow(&root_handle, OsStr::new("payload"), false)
            .expect("open changed payload");
    let changed_metadata = provider_sync_file_metadata_fingerprint_from_handle(&changed)
        .expect("read changed change time");

    assert_eq!(original_metadata.identity, changed_metadata.identity);
    assert_eq!(original_metadata.size, changed_metadata.size);
    assert_eq!(
        original_metadata.modified, changed_metadata.modified,
        "changing a Windows file attribute must not require LastWriteTime to move"
    );
    assert_ne!(
        original_metadata.changed, changed_metadata.changed,
        "Windows ChangeTime must advance when file metadata changes"
    );
    drop(changed);
    permissions.set_readonly(false);
    std::fs::set_permissions(&payload, permissions).expect("restore payload permissions");
}

#[cfg(windows)]
#[test]
fn windows_backup_directory_enumeration_restarts_for_each_snapshot() {
    use std::os::windows::ffi::OsStrExt as _;

    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join("root");
    std::fs::create_dir_all(&root).expect("create root");
    std::fs::write(root.join("first.txt"), b"1").expect("write first entry");
    let root_handle = open_provider_sync_backup_dir_no_follow(&root).expect("open root");

    let mut first_budget = ProviderSyncPruneBudget::with_limits(4, 8, 1024, 4096);
    let first = capture_provider_sync_backup_tree(&root_handle, &mut first_budget)
        .expect("capture first snapshot");
    assert_eq!(
        first.entries.len(),
        1,
        "first snapshot should see one entry"
    );

    std::fs::write(root.join("second.txt"), b"2").expect("write second entry");
    let mut second_budget = ProviderSyncPruneBudget::with_limits(4, 8, 1024, 4096);
    let second = capture_provider_sync_backup_tree(&root_handle, &mut second_budget)
        .expect("capture second snapshot from the same handle");
    assert_eq!(
        second.entries.len(),
        2,
        "each snapshot must restart the directory cursor and observe new entries"
    );
    let second_name = OsStr::new("second.txt").encode_wide().collect::<Vec<_>>();
    assert!(
        second.entries.iter().any(|entry| entry.name == second_name),
        "the restarted snapshot must include the entry added after the first enumeration"
    );
}

#[cfg(windows)]
#[test]
fn windows_backup_removal_revalidates_immediately_before_handle_delete() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join(PROVIDER_SYNC_BACKUP_ROOT);
    std::fs::create_dir_all(&root).expect("create backup root");
    let candidate = root.join("candidate");
    write_managed_manifest(&candidate, 1, "1", PROVIDER_SYNC_MANAGED_BY);
    std::fs::write(candidate.join("config.toml"), b"before").expect("write managed payload");
    let expected = managed_backup_version(&candidate)
        .expect("classify candidate")
        .expect("managed candidate");

    let root_for_hook = root.clone();
    set_before_windows_provider_sync_backup_handle_delete_test_hook(Box::new(move || {
        let quarantine = find_prune_quarantine(&root_for_hook);
        std::fs::write(quarantine.join("config.toml"), b"after!")
            .expect("rewrite isolated payload at handle-delete boundary");
    }));

    let warning = remove_managed_backup_candidate(&root, &candidate, expected)
        .expect("safe candidate removal");

    assert!(warning.is_some(), "late file change should be reported");
    let isolated = find_prune_quarantine(&root);
    assert_eq!(
        std::fs::read(isolated.join("config.toml")).expect("changed payload must survive"),
        b"after!".to_vec()
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

fn find_prune_quarantine(root: &Path) -> PathBuf {
    std::fs::read_dir(root)
        .expect("read backup root")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(".provider-sync-prune-"))
        })
        .expect("find provider sync prune quarantine")
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

    #[cfg(windows)]
    assert!(
        manifest_symlink_created && child_symlink_created && root_symlink_created,
        "Windows CI must permit reparse-point test setup; a skipped link would leave the no-follow boundary unverified"
    );

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
fn backup_pruning_classifies_from_the_open_root_after_path_replacement() {
    let temp = tempfile::tempdir().expect("tempdir");
    let home = temp.path();
    let root = home.join(PROVIDER_SYNC_BACKUP_ROOT);
    std::fs::create_dir_all(&root).expect("create backup root");

    let legacy_v1 = root.join("legacy-v1");
    write_managed_manifest(&legacy_v1, 1, "1", PROVIDER_SYNC_MANAGED_BY);
    let current_v2 = root.join("current-v2");
    write_managed_manifest(
        &current_v2,
        PROVIDER_SYNC_BACKUP_VERSION,
        "2",
        PROVIDER_SYNC_MANAGED_BY,
    );

    let moved_root = home.join("moved-provider-sync-root");
    let root_for_hook = root.clone();
    let moved_root_for_hook = moved_root.clone();
    set_after_provider_sync_backup_root_open_test_hook(Box::new(move || {
        std::fs::rename(&root_for_hook, &moved_root_for_hook).expect("move trusted root");
        let replacement_legacy = root_for_hook.join("legacy-v1");
        write_managed_manifest(&replacement_legacy, 1, "3", PROVIDER_SYNC_MANAGED_BY);
        std::fs::write(
            replacement_legacy.join("user-notes.txt"),
            b"keep replacement",
        )
        .expect("write replacement root data");
    }));

    let warning = prune_managed_backups(home, &current_v2).expect("prune bound root");

    assert!(warning.is_none(), "{warning:?}");
    assert!(
        !moved_root.join("legacy-v1").exists(),
        "legacy backup in the bound root should be removed"
    );
    assert!(
        moved_root.join("current-v2").exists(),
        "current backup in the bound root should remain"
    );
    assert_eq!(
        std::fs::read(root.join("legacy-v1/user-notes.txt"))
            .expect("replacement root data must survive"),
        b"keep replacement".to_vec()
    );
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

#[test]
fn backup_removal_preserves_a_symlink_replaced_after_classification() {
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
    let external_managed = temp.path().join("external-managed");
    write_managed_manifest(&external_managed, 1, "2", PROVIDER_SYNC_MANAGED_BY);
    std::fs::write(external_managed.join("user-notes.txt"), b"keep me")
        .expect("write external data");
    if symlink_test_dir(&external_managed, &candidate).is_err() {
        return;
    }

    let warning = remove_managed_backup_candidate(&root, &candidate, expected)
        .expect("safe candidate removal");

    assert!(warning.is_some(), "replacement should be reported");
    assert!(
        std::fs::symlink_metadata(&candidate)
            .expect("replacement symlink must survive")
            .file_type()
            .is_symlink(),
        "replacement symlink must be restored"
    );
    assert_eq!(
        std::fs::read(external_managed.join("user-notes.txt"))
            .expect("symlink target data must survive"),
        b"keep me".to_vec()
    );
    assert!(
        displaced_managed.exists(),
        "the separately displaced managed backup is outside this removal"
    );
}

#[test]
fn backup_removal_stays_bound_to_open_root_after_path_replacement() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join(PROVIDER_SYNC_BACKUP_ROOT);
    std::fs::create_dir_all(&root).expect("create backup root");
    let candidate = root.join("candidate");
    write_managed_manifest(&candidate, 1, "1", PROVIDER_SYNC_MANAGED_BY);
    let expected = managed_backup_version(&candidate)
        .expect("classify candidate")
        .expect("managed candidate");

    let moved_root = temp.path().join("moved-provider-sync-root");
    let root_for_hook = root.clone();
    let moved_root_for_hook = moved_root.clone();
    set_before_provider_sync_backup_isolation_test_hook(Box::new(move || {
        std::fs::rename(&root_for_hook, &moved_root_for_hook).expect("move trusted root");
        let replacement = root_for_hook.join("candidate");
        std::fs::create_dir_all(&replacement).expect("create replacement root candidate");
        std::fs::write(replacement.join("user-notes.txt"), b"keep replacement")
            .expect("write replacement root data");
    }));

    let warning = remove_managed_backup_candidate(&root, &candidate, expected)
        .expect("safe candidate removal");

    assert!(warning.is_none(), "{warning:?}");
    assert_eq!(
        std::fs::read(root.join("candidate/user-notes.txt"))
            .expect("replacement root data must survive"),
        b"keep replacement".to_vec()
    );
    assert!(
        !moved_root.join("candidate").exists(),
        "the managed candidate in the bound root should be removed"
    );
}

#[test]
fn backup_removal_preserves_replacements_after_quarantine_validation() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join(PROVIDER_SYNC_BACKUP_ROOT);
    std::fs::create_dir_all(&root).expect("create backup root");
    let candidate = root.join("candidate");
    write_managed_manifest(&candidate, 1, "1", PROVIDER_SYNC_MANAGED_BY);
    let expected = managed_backup_version(&candidate)
        .expect("classify candidate")
        .expect("managed candidate");

    let moved_quarantine = root.join("moved-quarantine");
    let moved_quarantine_for_hook = moved_quarantine.clone();
    let candidate_for_hook = candidate.clone();
    set_after_provider_sync_backup_validation_test_hook(Box::new(move |quarantine| {
        std::fs::rename(quarantine, &moved_quarantine_for_hook).expect("move isolated backup");
        std::fs::create_dir_all(quarantine).expect("create quarantine replacement");
        std::fs::write(quarantine.join("replacement-sentinel"), b"keep quarantine")
            .expect("write quarantine sentinel");
        std::fs::create_dir_all(&candidate_for_hook).expect("create candidate replacement");
        std::fs::write(candidate_for_hook.join("user-notes.txt"), b"keep candidate")
            .expect("write candidate replacement");
    }));

    let warning = remove_managed_backup_candidate(&root, &candidate, expected)
        .expect("safe candidate removal");

    assert!(warning.is_some(), "replacement should be reported");
    assert_eq!(
        std::fs::read(candidate.join("user-notes.txt"))
            .expect("candidate replacement must survive"),
        b"keep candidate".to_vec()
    );
    let replacement_quarantine = std::fs::read_dir(&root)
        .expect("read backup root")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.join("replacement-sentinel").exists())
        .expect("quarantine replacement must survive");
    assert_eq!(
        std::fs::read(replacement_quarantine.join("replacement-sentinel"))
            .expect("read quarantine replacement"),
        b"keep quarantine".to_vec()
    );
    assert!(
        moved_quarantine
            .join(PROVIDER_SYNC_MANAGED_BACKUP_MANIFEST)
            .exists(),
        "the validated managed backup must remain at its attacker-moved path"
    );
}

#[test]
fn backup_removal_preserves_tree_entries_added_after_validation() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join(PROVIDER_SYNC_BACKUP_ROOT);
    std::fs::create_dir_all(&root).expect("create backup root");
    let candidate = root.join("candidate");
    write_managed_manifest(&candidate, 1, "1", PROVIDER_SYNC_MANAGED_BY);
    let expected = managed_backup_version(&candidate)
        .expect("classify candidate")
        .expect("managed candidate");

    set_after_provider_sync_backup_validation_test_hook(Box::new(move |quarantine| {
        std::fs::write(quarantine.join("late-user-file.txt"), b"keep me")
            .expect("inject late file");
    }));

    let warning = remove_managed_backup_candidate(&root, &candidate, expected)
        .expect("safe candidate removal");

    assert!(warning.is_some(), "tree change should be reported");
    let isolated = std::fs::read_dir(&root)
        .expect("read backup root")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.join("late-user-file.txt").exists())
        .expect("changed quarantine must remain");
    assert_eq!(
        std::fs::read(isolated.join("late-user-file.txt")).expect("late file must survive"),
        b"keep me".to_vec()
    );
    assert!(
        isolated
            .join(PROVIDER_SYNC_MANAGED_BACKUP_MANIFEST)
            .exists(),
        "managed backup data must remain after a late tree change"
    );
}

#[test]
fn backup_removal_preserves_in_place_file_changes_after_validation() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join(PROVIDER_SYNC_BACKUP_ROOT);
    std::fs::create_dir_all(&root).expect("create backup root");
    let candidate = root.join("candidate");
    write_managed_manifest(&candidate, 1, "1", PROVIDER_SYNC_MANAGED_BY);
    let payload = candidate.join("config.toml");
    std::fs::write(&payload, b"before").expect("write managed payload");
    let expected = managed_backup_version(&candidate)
        .expect("classify candidate")
        .expect("managed candidate");

    set_after_provider_sync_backup_validation_test_hook(Box::new(move |quarantine| {
        std::fs::write(quarantine.join("config.toml"), b"after!")
            .expect("rewrite isolated payload");
    }));

    let warning = remove_managed_backup_candidate(&root, &candidate, expected)
        .expect("safe candidate removal");

    assert!(warning.is_some(), "file change should be reported");
    let isolated = find_prune_quarantine(&root);
    assert_eq!(
        std::fs::read(isolated.join("config.toml")).expect("changed payload must survive"),
        b"after!".to_vec()
    );
}

#[cfg(unix)]
#[test]
fn backup_removal_preserves_entry_replaced_at_unix_delete_boundary() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join(PROVIDER_SYNC_BACKUP_ROOT);
    std::fs::create_dir_all(&root).expect("create backup root");
    let candidate = root.join("candidate");
    write_managed_manifest(&candidate, 1, "1", PROVIDER_SYNC_MANAGED_BY);
    let expected = managed_backup_version(&candidate)
        .expect("classify candidate")
        .expect("managed candidate");

    let root_for_hook = root.clone();
    set_before_unix_provider_sync_backup_entry_isolation_test_hook(Box::new(move || {
        let quarantine = find_prune_quarantine(&root_for_hook);
        let manifest = quarantine.join(PROVIDER_SYNC_MANAGED_BACKUP_MANIFEST);
        std::fs::rename(&manifest, quarantine.join("displaced-manifest.json"))
            .expect("displace validated manifest");
        std::fs::write(&manifest, b"keep replacement").expect("write replacement manifest entry");
    }));

    let warning = remove_managed_backup_candidate(&root, &candidate, expected)
        .expect("safe candidate removal");

    assert!(warning.is_some(), "entry replacement should be reported");
    let isolated = find_prune_quarantine(&root);
    assert!(
        isolated.join("displaced-manifest.json").exists(),
        "the validated manifest must remain displaced inside quarantine"
    );
    let replacement = std::fs::read_dir(&isolated)
        .expect("read isolated backup")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(".provider-sync-delete-"))
        })
        .expect("replacement tombstone must remain");
    assert_eq!(
        std::fs::read(replacement).expect("read replacement tombstone"),
        b"keep replacement".to_vec()
    );
}

#[cfg(unix)]
#[test]
fn backup_removal_preserves_root_replaced_at_unix_final_delete_boundary() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join(PROVIDER_SYNC_BACKUP_ROOT);
    std::fs::create_dir_all(&root).expect("create backup root");
    let candidate = root.join("candidate");
    write_managed_manifest(&candidate, 1, "1", PROVIDER_SYNC_MANAGED_BY);
    let expected = managed_backup_version(&candidate)
        .expect("classify candidate")
        .expect("managed candidate");

    let root_for_hook = root.clone();
    let moved_quarantine = root.join("moved-final-quarantine");
    let moved_quarantine_for_hook = moved_quarantine.clone();
    set_before_unix_provider_sync_backup_root_final_isolation_test_hook(Box::new(move || {
        let quarantine = find_prune_quarantine(&root_for_hook);
        std::fs::rename(&quarantine, &moved_quarantine_for_hook)
            .expect("move emptied managed quarantine");
        std::fs::create_dir_all(&quarantine).expect("create quarantine replacement");
        std::fs::write(quarantine.join("replacement-sentinel"), b"keep replacement")
            .expect("write replacement sentinel");
    }));

    let warning = remove_managed_backup_candidate(&root, &candidate, expected)
        .expect("safe candidate removal");

    assert!(warning.is_some(), "root replacement should be reported");
    let replacement = std::fs::read_dir(&root)
        .expect("read backup root")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.join("replacement-sentinel").exists())
        .expect("replacement root tombstone must remain");
    assert_eq!(
        std::fs::read(replacement.join("replacement-sentinel")).expect("read replacement sentinel"),
        b"keep replacement".to_vec()
    );
    assert!(
        moved_quarantine.exists(),
        "the emptied managed quarantine should remain at its moved path"
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
