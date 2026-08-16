mod support;

use serde_json::{json, Value};

fn user_catalog() -> Value {
    json!({
        "model_auto_compact_token_limit": 999_999,
        "unknown_root": {"preserved": true},
        "models": [
            {
                "slug": "gpt-5.6-sol",
                "visibility": "list",
                "context_window": 272_000,
                "max_context_window": 273_000,
                "effective_context_window_percent": 95,
                "auto_compact_token_limit": 250_000
            },
            {
                "slug": "gpt-5.6-terra",
                "visibility": "list",
                "context_window": 274_000,
                "max_context_window": 275_000,
                "effective_context_window_percent": 94,
                "auto_compact_token_limit": 251_000
            },
            {
                "slug": "gpt-5.6-luna",
                "visibility": "list",
                "context_window": 276_000,
                "max_context_window": 277_000,
                "effective_context_window_percent": 93,
                "auto_compact_token_limit": 252_000
            },
            {
                "slug": "gpt-other",
                "visibility": "list",
                "context_window": 128_000,
                "max_context_window": 128_000,
                "effective_context_window_percent": 87
            }
        ]
    })
}

fn config_catalog_path(config: &str) -> String {
    config
        .parse::<toml_edit::DocumentMut>()
        .expect("valid config")
        .get("model_catalog_json")
        .and_then(toml_edit::Item::as_str)
        .expect("model catalog pointer")
        .to_string()
}

#[test]
fn canonical_config_journal_recovers_each_persisted_phase() {
    for (phase, expected_model) in [
        ("planned", "old"),
        ("canonical_written", "old"),
        ("live_written", "new"),
        ("catalog_written", "new"),
    ] {
        let app = support::TestApp::new();
        let handle = app.handle();
        aio_coding_hub_lib::test_support::init_db(&handle).expect("init db");
        let config_path = aio_coding_hub_lib::test_support::codex_config_toml_path(&handle)
            .expect("config path");
        std::fs::create_dir_all(config_path.parent().expect("config parent"))
            .expect("create config parent");
        std::fs::write(&config_path, "model = \"old\"\n").expect("write old config");

        aio_coding_hub_lib::test_support::set_codex_lifecycle_failpoint_for_tests(Some(phase))
            .expect("set lifecycle failpoint");
        let error = aio_coding_hub_lib::test_support::codex_config_toml_raw_set(
            &handle,
            "model = \"new\"\n".to_string(),
        )
        .expect_err("failpoint must interrupt the transaction");
        assert_eq!(error.code(), "CODEX_LIFECYCLE_TEST_INTERRUPTED");

        aio_coding_hub_lib::test_support::recover_codex_lifecycle(&handle)
            .expect("recover lifecycle");
        let recovered = std::fs::read_to_string(&config_path).expect("recovered config");
        assert_eq!(
            recovered
                .parse::<toml_edit::DocumentMut>()
                .expect("valid recovered config")["model"]
                .as_str(),
            Some(expected_model),
            "phase={phase}"
        );
        aio_coding_hub_lib::test_support::codex_config_toml_raw_set(
            &handle,
            format!("model = \"after-{phase}\"\n"),
        )
        .expect("journal must be cleared after recovery");
    }
}

#[test]
fn context_window_372k_is_idempotent_and_composes_with_proxy_startup_and_exit() {
    let app = support::TestApp::new();
    let handle = app.handle();
    aio_coding_hub_lib::test_support::init_db(&handle).expect("init db");

    let config_path =
        aio_coding_hub_lib::test_support::codex_config_toml_path(&handle).expect("config path");
    let codex_home = config_path.parent().expect("codex home");
    std::fs::create_dir_all(codex_home).expect("create Codex home");
    let user_catalog_path = codex_home.join("user-model-catalog.json");
    std::fs::write(
        &user_catalog_path,
        serde_json::to_vec_pretty(&user_catalog()).expect("serialize user catalog"),
    )
    .expect("write user catalog");
    let user_catalog_path_text = user_catalog_path.to_string_lossy().to_string();
    let quoted_user_catalog = serde_json::to_string(&user_catalog_path_text).expect("quote path");
    std::fs::write(
        &config_path,
        format!(
            "# direct config\nmodel_provider = \"direct\"\nmodel_catalog_json = {quoted_user_catalog}\n\n[model_providers.direct]\nbase_url = \"https://api.openai.com/v1\"\n\n[user_section]\nkeep = true\n"
        ),
    )
    .expect("write direct config");
    let cache_path = codex_home.join("models_cache.json");
    let cache_sentinel = b"cache-sentinel-do-not-touch\n";
    std::fs::write(&cache_path, cache_sentinel).expect("write cache sentinel");

    aio_coding_hub_lib::test_support::set_codex_lifecycle_failpoint_for_tests(Some(
        "catalog_policy_written",
    ))
    .expect("set catalog policy failpoint");
    let interrupted = aio_coding_hub_lib::test_support::codex_context_window_372k_set_json(
        &handle, true,
    )
    .expect_err("catalog policy failpoint must interrupt enable");
    assert_eq!(interrupted.code(), "CODEX_LIFECYCLE_TEST_INTERRUPTED");
    aio_coding_hub_lib::test_support::recover_codex_lifecycle(&handle)
        .expect("recover catalog policy");
    assert_eq!(
        config_catalog_path(&std::fs::read_to_string(&config_path).expect("recovered config")),
        user_catalog_path_text
    );
    assert!(!aio_coding_hub_lib::test_support::settings_get_json(&handle)
        .expect("settings after recovery")["enable_codex_context_window_372k"]
        .as_bool()
        .expect("372K setting"));

    let enabled = aio_coding_hub_lib::test_support::codex_context_window_372k_set_json(
        &handle, true,
    )
    .expect("enable 372K");
    assert_eq!(enabled["enabled"], json!(true));
    let app_data = aio_coding_hub_lib::test_support::app_data_dir(&handle).expect("app data");
    let generated_path = app_data
        .join("cli-proxy")
        .join("codex")
        .join("managed-model-catalog.json");
    let generated_path_text = generated_path.to_string_lossy().to_string();
    let generated_once = std::fs::read(&generated_path).expect("generated catalog");
    assert_eq!(
        config_catalog_path(&std::fs::read_to_string(&config_path).expect("live config")),
        generated_path_text
    );
    assert_eq!(std::fs::read(&cache_path).expect("cache sentinel"), cache_sentinel);

    aio_coding_hub_lib::test_support::codex_context_window_372k_set_json(&handle, true)
        .expect("repeat enable");
    assert_eq!(
        std::fs::read(&generated_path).expect("idempotent generated catalog"),
        generated_once
    );

    aio_coding_hub_lib::test_support::codex_catalog_restore_direct_on_exit(&handle)
        .expect("restore direct on exit");
    assert_eq!(
        config_catalog_path(&std::fs::read_to_string(&config_path).expect("exit config")),
        user_catalog_path_text
    );
    let settings =
        aio_coding_hub_lib::test_support::settings_get_json(&handle).expect("settings after exit");
    assert_eq!(settings["enable_codex_context_window_372k"], json!(true));

    aio_coding_hub_lib::test_support::codex_catalog_sync_current(&handle)
        .expect("startup catalog recovery");
    assert_eq!(
        config_catalog_path(&std::fs::read_to_string(&config_path).expect("startup config")),
        generated_path_text
    );

    let base_origin = "http://127.0.0.1:37123";
    let proxy_enabled = aio_coding_hub_lib::test_support::cli_proxy_set_enabled_json(
        &handle,
        "codex",
        true,
        base_origin,
    )
    .expect("enable proxy");
    assert_eq!(proxy_enabled["ok"], json!(true));
    let projected = std::fs::read_to_string(&config_path).expect("projected config");
    assert!(projected.contains("model_provider = \"aio\""), "{projected}");
    assert_eq!(config_catalog_path(&projected), generated_path_text);

    let proxy_disabled = aio_coding_hub_lib::test_support::cli_proxy_set_enabled_json(
        &handle,
        "codex",
        false,
        base_origin,
    )
    .expect("disable proxy");
    assert_eq!(proxy_disabled["ok"], json!(true));
    let direct_with_policy = std::fs::read_to_string(&config_path).expect("direct config");
    assert!(
        direct_with_policy.contains("model_provider = \"direct\""),
        "{direct_with_policy}"
    );
    assert_eq!(
        config_catalog_path(&direct_with_policy),
        generated_path_text
    );

    let disabled = aio_coding_hub_lib::test_support::codex_context_window_372k_set_json(
        &handle, false,
    )
    .expect("disable 372K");
    assert_eq!(disabled["enabled"], json!(false));
    let restored = std::fs::read_to_string(&config_path).expect("restored config");
    assert_eq!(config_catalog_path(&restored), user_catalog_path_text);
    assert!(restored.contains("[user_section]"), "{restored}");
    assert!(!generated_path.exists());
    assert_eq!(std::fs::read(&cache_path).expect("cache sentinel"), cache_sentinel);

    let before_repeat_disable = std::fs::read(&config_path).expect("config before disable");
    aio_coding_hub_lib::test_support::codex_context_window_372k_set_json(&handle, false)
        .expect("repeat disable");
    assert_eq!(
        std::fs::read(&config_path).expect("config after repeat disable"),
        before_repeat_disable
    );
}
