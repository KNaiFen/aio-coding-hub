mod support;

#[test]
fn prompt_default_sync_preserves_source_bytes_and_resolves_journal() {
    let app = support::TestApp::new();
    let handle = app.handle();
    let original = b"\xEF\xBB\xBFsource prompt with trailing spaces  \r\n".to_vec();
    aio_coding_hub_lib::test_support::prompt_restore_target_bytes(
        &handle,
        "claude",
        Some(original.clone()),
    )
    .expect("write source prompt");

    let report = aio_coding_hub_lib::test_support::prompts_default_sync_from_files_json(&handle)
        .expect("import default prompts");
    let claude = report
        .get("items")
        .and_then(|items| items.as_array())
        .and_then(|items| {
            items
                .iter()
                .find(|item| item.get("cli_key").and_then(|value| value.as_str()) == Some("claude"))
        })
        .expect("claude sync result");
    assert_eq!(
        claude.get("action").and_then(|value| value.as_str()),
        Some("created")
    );
    assert_eq!(
        aio_coding_hub_lib::test_support::prompt_read_target_bytes(&handle, "claude")
            .expect("read source prompt"),
        Some(original)
    );
    assert_eq!(
        aio_coding_hub_lib::test_support::recovery_journal_statuses_for_kind(
            &handle,
            "prompt.default_sync",
        )
        .expect("read journal statuses"),
        vec!["resolved"]
    );
}
