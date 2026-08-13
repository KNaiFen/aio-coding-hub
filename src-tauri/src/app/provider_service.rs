use crate::app::provider_availability_probe_runtime::{
    ProviderAvailabilityProbeMutationGuard, ProviderAvailabilityProbeRuntimeState,
};
use crate::app_state::{ensure_db_ready, DbInitState};
use crate::gateway_control::{
    app_gateway_clear_cli_route_runtime_state, app_gateway_clear_cli_session_bindings,
    app_gateway_set_provider_enabled,
};
use crate::{blocking, providers};
use tauri::Manager;

#[derive(serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProviderUpsertInput {
    pub provider_id: Option<i64>,
    pub cli_key: String,
    pub name: String,
    pub base_urls: Vec<String>,
    pub base_url_mode: providers::ProviderBaseUrlMode,
    pub auth_mode: Option<providers::ProviderAuthMode>,
    pub api_key: Option<String>,
    pub enabled: bool,
    pub cost_multiplier: f64,
    pub priority: Option<i64>,
    pub claude_models: Option<providers::ClaudeModels>,
    pub model_mapping: Option<providers::ModelMapping>,
    pub availability_test_model: Option<String>,
    #[serde(default)]
    pub availability_probe_enabled: Option<bool>,
    #[serde(default)]
    pub availability_probe_interval_minutes: Option<u32>,
    #[serde(rename = "limit5hUsd", alias = "limit5HUsd")]
    #[specta(rename = "limit5hUsd")]
    pub limit_5h_usd: Option<f64>,
    pub limit_daily_usd: Option<f64>,
    pub daily_reset_mode: Option<providers::DailyResetMode>,
    pub daily_reset_time: Option<String>,
    pub limit_weekly_usd: Option<f64>,
    pub limit_monthly_usd: Option<f64>,
    pub limit_total_usd: Option<f64>,
    pub tags: Option<Vec<String>>,
    pub note: Option<String>,
    pub source_provider_id: Option<i64>,
    pub bridge_type: Option<String>,
    pub stream_idle_timeout_seconds: Option<u32>,
    pub extension_values: Option<Vec<providers::ProviderExtensionValuesInput>>,
    #[serde(default)]
    pub account_usage_credentials:
        Option<crate::domain::provider_account_usage::ProviderAccountUsageCredentialsPatch>,
    pub upstream_retry_policy_override: Option<crate::settings::UpstreamRetryPolicy>,
    #[serde(default)]
    pub upstream_retry_policy_override_specified: bool,
    pub model_routing_policy_override: Option<crate::settings::ModelRoutingPolicy>,
    #[serde(default)]
    pub model_routing_policy_override_specified: bool,
}

pub(crate) async fn begin_provider_availability_probe_mutation<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    provider_id: i64,
) -> Option<ProviderAvailabilityProbeMutationGuard> {
    let runtime = ProviderAvailabilityProbeRuntimeState::from_app(app)?;
    runtime.begin_mutation(provider_id).await
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ProviderRuntimeResetDecision {
    clear_route_runtime_state: bool,
}

fn normalize_provider_name(name: &str) -> String {
    name.trim().to_lowercase()
}

fn build_duplicated_provider_name(
    source_name: &str,
    existing_providers: &[providers::ProviderSummary],
) -> String {
    let base_name = format!("{} 副本", source_name.trim());
    let used_names: std::collections::HashSet<String> = existing_providers
        .iter()
        .map(|provider| normalize_provider_name(&provider.name))
        .collect();

    if !used_names.contains(&normalize_provider_name(&base_name)) {
        return base_name;
    }

    let mut index = 2;
    loop {
        let candidate = format!("{base_name} {index}");
        if !used_names.contains(&normalize_provider_name(&candidate)) {
            return candidate;
        }
        index += 1;
    }
}

fn submitted_api_key_changed(
    previous_api_key: Option<&str>,
    submitted_api_key: Option<&str>,
) -> bool {
    let Some(submitted) = submitted_api_key
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
    else {
        return false;
    };

    previous_api_key
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        != Some(submitted)
}

fn provider_runtime_reset_decision(
    previous: Option<&providers::ProviderSummary>,
    previous_api_key: Option<&str>,
    next: &providers::ProviderSummary,
    submitted_api_key: Option<&str>,
) -> ProviderRuntimeResetDecision {
    let Some(previous) = previous else {
        return ProviderRuntimeResetDecision {
            clear_route_runtime_state: next.enabled,
        };
    };

    let sensitive_config_changed = previous.base_urls != next.base_urls
        || previous.base_url_mode != next.base_url_mode
        || previous.enabled != next.enabled
        || previous.auth_mode != next.auth_mode
        || submitted_api_key_changed(previous_api_key, submitted_api_key)
        || previous.source_provider_id != next.source_provider_id
        || previous.bridge_type != next.bridge_type
        || previous.model_mapping != next.model_mapping
        || previous.upstream_retry_policy_override != next.upstream_retry_policy_override
        || previous.model_routing_policy_override != next.model_routing_policy_override;

    ProviderRuntimeResetDecision {
        clear_route_runtime_state: sensitive_config_changed,
    }
}

fn custom_account_usage_permission_request(
    values: Option<&[providers::ProviderExtensionValuesInput]>,
    base_url: &str,
) -> Result<
    Option<crate::domain::provider_account_usage::ProviderAccountUsageCustomPermissionRequest>,
    String,
> {
    crate::domain::provider_account_usage::custom_account_usage_permission_request(values, base_url)
        .map_err(|message| format!("SEC_INVALID_INPUT: {message}"))
}

pub(crate) async fn providers_list(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
) -> Result<Vec<providers::ProviderSummary>, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("providers_list", move || {
        providers::list_by_cli(&db, &cli_key)
    })
    .await
    .map_err(Into::into)
}

pub(crate) async fn provider_upsert(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    mut input: ProviderUpsertInput,
) -> Result<providers::ProviderSummary, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    crate::domain::provider_account_usage::strip_custom_account_usage_permission_proofs(
        &mut input.extension_values,
    );
    let permission_base_url = input
        .base_urls
        .iter()
        .map(|value| value.trim())
        .find(|value| !value.is_empty())
        .unwrap_or_default();
    if let Some(permission) = custom_account_usage_permission_request(
        input.extension_values.as_deref(),
        permission_base_url,
    )? {
        let already_confirmed = if let Some(provider_id) = input.provider_id {
            let db = db.clone();
            let fingerprint = permission.fingerprint.clone();
            let base_origin = permission.base_origin.clone();
            blocking::run(
                "provider_upsert_check_custom_account_usage_permission",
                move || {
                    let conn = db.open_connection()?;
                    let provider = providers::get_by_id(&conn, provider_id)?;
                    Ok::<_, crate::shared::error::AppError>(
                    crate::domain::provider_account_usage::
                        custom_account_usage_saved_permission_matches(
                            &provider.extension_values,
                            &fingerprint,
                            &base_origin,
                        ),
                )
                },
            )
            .await
            .map_err(Into::<String>::into)?
        } else {
            false
        };
        if !already_confirmed {
            let confirmed = crate::app::provider_account_usage_confirmation::
                confirm_custom_account_usage_network_access(
                    &app,
                    crate::app::provider_account_usage_confirmation::
                        CustomAccountUsageConfirmationKind::Enable,
                    &permission.network_origins,
                    &permission.fingerprint,
                )
                .await?;
            if !confirmed {
                return Err(
                    "SEC_CONFIRM_REQUIRED: custom account usage permission was not confirmed"
                        .to_string(),
                );
            }
            crate::domain::provider_account_usage::add_custom_account_usage_permission_proof(
                &mut input.extension_values,
                &permission.fingerprint,
                &permission.base_origin,
            )?;
        }
    }

    let ProviderUpsertInput {
        provider_id,
        cli_key,
        name,
        base_urls,
        base_url_mode,
        auth_mode,
        api_key,
        enabled,
        cost_multiplier,
        priority,
        claude_models,
        model_mapping,
        availability_test_model,
        availability_probe_enabled,
        availability_probe_interval_minutes,
        limit_5h_usd,
        limit_daily_usd,
        daily_reset_mode,
        daily_reset_time,
        limit_weekly_usd,
        limit_monthly_usd,
        limit_total_usd,
        tags,
        note,
        source_provider_id,
        bridge_type,
        stream_idle_timeout_seconds,
        extension_values,
        account_usage_credentials,
        upstream_retry_policy_override,
        upstream_retry_policy_override_specified,
        model_routing_policy_override,
        model_routing_policy_override_specified,
    } = input;

    let is_create = provider_id.is_none();
    let probe_mutation_guard = match provider_id {
        Some(provider_id) => begin_provider_availability_probe_mutation(&app, provider_id).await,
        None => None,
    };
    let name_for_log = name.clone();
    let cli_key_for_log = cli_key.clone();
    let submitted_api_key = api_key.clone();
    let result = blocking::run("provider_upsert", move || {
        let _probe_mutation_guard = probe_mutation_guard;
        let previous = match provider_id {
            Some(id) => {
                let conn = db.open_connection()?;
                Some(providers::get_by_id(&conn, id)?)
            }
            None => None,
        };
        let previous_api_key = match provider_id {
            Some(id) => Some(providers::get_api_key_plaintext(&db, id)?),
            None => None,
        };
        let availability_probe_enabled = availability_probe_enabled
            .or_else(|| {
                previous
                    .as_ref()
                    .map(|provider| provider.availability_probe_enabled)
            })
            .unwrap_or(false);
        let availability_probe_interval_minutes = availability_probe_interval_minutes
            .or_else(|| {
                previous
                    .as_ref()
                    .map(|provider| provider.availability_probe_interval_minutes)
            })
            .unwrap_or(providers::DEFAULT_AVAILABILITY_PROBE_INTERVAL_MINUTES);

        let saved = providers::upsert(
            &db,
            providers::ProviderUpsertParams {
                provider_id,
                cli_key,
                name,
                base_urls,
                base_url_mode,
                auth_mode,
                api_key,
                enabled,
                cost_multiplier,
                priority,
                claude_models,
                model_mapping,
                availability_test_model,
                availability_probe_enabled,
                availability_probe_interval_minutes,
                limit_5h_usd,
                limit_daily_usd,
                daily_reset_mode,
                daily_reset_time,
                limit_weekly_usd,
                limit_monthly_usd,
                limit_total_usd,
                tags,
                note,
                source_provider_id,
                bridge_type,
                stream_idle_timeout_seconds,
                extension_values,
                account_usage_credentials_patch: account_usage_credentials,
                account_usage_credentials_copy_from_provider_id: None,
                upstream_retry_policy_override,
                upstream_retry_policy_override_specified,
                model_routing_policy_override,
                model_routing_policy_override_specified,
            },
        )?;

        let decision = provider_runtime_reset_decision(
            previous.as_ref(),
            previous_api_key.as_deref(),
            &saved,
            submitted_api_key.as_deref(),
        );

        Ok::<_, crate::shared::error::AppError>((saved, decision))
    })
    .await
    .map_err(Into::into);

    if let Ok((ref provider, decision)) = result {
        if let Some(runtime) = app.try_state::<
            crate::app::provider_account_usage_runtime::ProviderAccountUsageRuntimeState,
        >() {
            runtime.invalidate(provider.id).await;
        }
        app_gateway_set_provider_enabled(&app, provider.id, provider.enabled);
        if is_create {
            tracing::info!(
                provider_id = provider.id,
                provider_name = %name_for_log,
                cli_key = %cli_key_for_log,
                "provider created"
            );
        } else {
            tracing::info!(
                provider_id = provider.id,
                provider_name = %name_for_log,
                cli_key = %cli_key_for_log,
                "provider updated"
            );
        }

        if decision.clear_route_runtime_state {
            let cleared = app_gateway_clear_cli_route_runtime_state(&app, &provider.cli_key);
            tracing::info!(
                provider_id = provider.id,
                cli_key = %provider.cli_key,
                cleared_sessions = cleared.cleared_sessions,
                cleared_recent_errors = cleared.cleared_recent_errors,
                "provider route runtime state cleared after provider save"
            );
        }
    }

    result.map(|(provider, _)| provider)
}

pub(crate) async fn provider_duplicate(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
) -> Result<providers::ProviderSummary, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let result = blocking::run("provider_duplicate", move || {
        duplicate_provider_in_db(&db, provider_id)
    })
    .await
    .map_err(Into::into);

    if let Ok(ref provider) = result {
        app_gateway_set_provider_enabled(&app, provider.id, provider.enabled);
        if provider.enabled {
            let cleared = app_gateway_clear_cli_route_runtime_state(&app, &provider.cli_key);
            tracing::info!(
                provider_id = provider.id,
                cli_key = %provider.cli_key,
                cleared_sessions = cleared.cleared_sessions,
                cleared_recent_errors = cleared.cleared_recent_errors,
                "provider route runtime state cleared after duplicate"
            );
        }

        tracing::info!(
            provider_id = provider.id,
            cli_key = %provider.cli_key,
            provider_name = %provider.name,
            "provider duplicated"
        );
    }

    result
}

fn duplicate_provider_in_db(
    db: &crate::db::Db,
    provider_id: i64,
) -> crate::shared::error::AppResult<providers::ProviderSummary> {
    let conn = db.open_connection()?;
    let source = providers::get_by_id(&conn, provider_id)?;
    let siblings = providers::list_by_cli(db, &source.cli_key)?;
    let api_key = if source.auth_mode == "api_key" && source.source_provider_id.is_none() {
        Some(providers::get_api_key_plaintext(db, provider_id)?)
    } else {
        None
    };
    let extension_values = Some(
        source
            .extension_values
            .iter()
            .map(|value| providers::ProviderExtensionValuesInput {
                plugin_id: value.plugin_id.clone(),
                namespace: value.namespace.clone(),
                values: value.values.clone(),
            })
            .collect(),
    );

    providers::upsert(
        db,
        providers::ProviderUpsertParams {
            provider_id: None,
            cli_key: source.cli_key.clone(),
            name: build_duplicated_provider_name(&source.name, &siblings),
            base_urls: source.base_urls.clone(),
            base_url_mode: source.base_url_mode,
            auth_mode: match source.auth_mode.as_str() {
                "oauth" => Some(providers::ProviderAuthMode::Oauth),
                _ => Some(providers::ProviderAuthMode::ApiKey),
            },
            api_key,
            enabled: source.enabled,
            cost_multiplier: source.cost_multiplier,
            priority: None,
            claude_models: Some(source.claude_models.clone()),
            model_mapping: Some(source.model_mapping.clone()),
            availability_test_model: source.availability_test_model.clone(),
            availability_probe_enabled: source.availability_probe_enabled,
            availability_probe_interval_minutes: source.availability_probe_interval_minutes,
            limit_5h_usd: source.limit_5h_usd,
            limit_daily_usd: source.limit_daily_usd,
            daily_reset_mode: Some(source.daily_reset_mode),
            daily_reset_time: Some(source.daily_reset_time.clone()),
            limit_weekly_usd: source.limit_weekly_usd,
            limit_monthly_usd: source.limit_monthly_usd,
            limit_total_usd: source.limit_total_usd,
            tags: Some(source.tags.clone()),
            note: Some(source.note.clone()),
            source_provider_id: source.source_provider_id,
            bridge_type: source.bridge_type.clone(),
            stream_idle_timeout_seconds: source.stream_idle_timeout_seconds,
            extension_values,
            account_usage_credentials_patch: None,
            account_usage_credentials_copy_from_provider_id: Some(provider_id),
            upstream_retry_policy_override: source.upstream_retry_policy_override.clone(),
            upstream_retry_policy_override_specified: true,
            model_routing_policy_override: source.model_routing_policy_override.clone(),
            model_routing_policy_override_specified: true,
        },
    )
}

pub(crate) async fn provider_set_enabled(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
    enabled: bool,
) -> Result<providers::ProviderSummary, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let probe_mutation_guard = begin_provider_availability_probe_mutation(&app, provider_id).await;
    let result = blocking::run("provider_set_enabled", move || {
        let _probe_mutation_guard = probe_mutation_guard;
        providers::set_enabled(&db, provider_id, enabled)
    })
    .await
    .map_err(Into::into);

    if let Ok(ref provider) = result {
        app_gateway_set_provider_enabled(&app, provider.id, provider.enabled);
        let cleared = app_gateway_clear_cli_route_runtime_state(&app, &provider.cli_key);
        tracing::info!(
            provider_id = provider.id,
            enabled = provider.enabled,
            cleared_sessions = cleared.cleared_sessions,
            cleared_recent_errors = cleared.cleared_recent_errors,
            "provider enabled state changed"
        );
    }

    result
}

pub(crate) async fn provider_delete(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    provider_id: i64,
    clear_usage_stats: bool,
) -> Result<bool, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let probe_mutation_guard = begin_provider_availability_probe_mutation(&app, provider_id).await;
    let result = blocking::run(
        "provider_delete",
        move || -> crate::shared::error::AppResult<(bool, String)> {
            let _probe_mutation_guard = probe_mutation_guard;
            let cli_key = providers::cli_key_by_id(&db, provider_id)?.ok_or_else(|| {
                crate::shared::error::AppError::from("DB_NOT_FOUND: provider not found")
            })?;
            providers::delete(&db, provider_id, clear_usage_stats)?;
            Ok((true, cli_key))
        },
    )
    .await
    .map_err(Into::into);

    if let Ok((true, ref cli_key)) = result {
        if let Some(runtime) = app.try_state::<
            crate::app::provider_account_usage_runtime::ProviderAccountUsageRuntimeState,
        >() {
            runtime.invalidate(provider_id).await;
        }
        app_gateway_set_provider_enabled(&app, provider_id, false);
        let cleared = app_gateway_clear_cli_route_runtime_state(&app, cli_key);
        tracing::info!(
            provider_id = provider_id,
            cli_key = %cli_key,
            clear_usage_stats = clear_usage_stats,
            cleared_sessions = cleared.cleared_sessions,
            cleared_recent_errors = cleared.cleared_recent_errors,
            "provider deleted"
        );
    }

    result.map(|(deleted, _)| deleted)
}

pub(crate) async fn providers_reorder(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
    ordered_provider_ids: Vec<i64>,
) -> Result<Vec<providers::ProviderSummary>, String> {
    let cli_key_for_log = cli_key.clone();
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let result = blocking::run("providers_reorder", move || {
        providers::reorder(&db, &cli_key, ordered_provider_ids)
    })
    .await
    .map_err(Into::into);

    if let Ok(ref providers) = result {
        tracing::info!(
            cli_key = %cli_key_for_log,
            count = providers.len(),
            "provider pool display order updated"
        );
    }

    result
}

pub(crate) async fn default_route_providers_list(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
) -> Result<Vec<providers::ProviderRouteRow>, String> {
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    blocking::run("default_route_providers_list", move || {
        providers::default_route_list(&db, &cli_key)
    })
    .await
    .map_err(Into::into)
}

pub(crate) async fn default_route_providers_set_order(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
    ordered_provider_ids: Vec<i64>,
) -> Result<Vec<providers::ProviderRouteRow>, String> {
    let cli_key_for_log = cli_key.clone();
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let result = blocking::run("default_route_providers_set_order", move || {
        providers::default_route_set_order(&db, &cli_key, ordered_provider_ids)
    })
    .await
    .map_err(Into::into);

    if let Ok(ref rows) = result {
        let cleared = app_gateway_clear_cli_route_runtime_state(&app, &cli_key_for_log);
        tracing::info!(
            cli_key = %cli_key_for_log,
            count = rows.len(),
            cleared_sessions = cleared.cleared_sessions,
            cleared_recent_errors = cleared.cleared_recent_errors,
            "default route provider order updated"
        );
    }

    result
}

pub(crate) async fn default_route_provider_set_session_reuse_priority(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    cli_key: String,
    provider_id: i64,
    session_reuse_priority: i64,
) -> Result<providers::ProviderRouteRow, String> {
    let cli_key_for_db = cli_key.clone();
    let db = ensure_db_ready(app.clone(), db_state.inner()).await?;
    let result = blocking::run(
        "default_route_provider_set_session_reuse_priority",
        move || {
            providers::default_route_set_session_reuse_priority(
                &db,
                &cli_key_for_db,
                provider_id,
                session_reuse_priority,
            )
        },
    )
    .await
    .map_err(Into::into);

    if result.is_ok() {
        let cleared = app_gateway_clear_cli_session_bindings(&app, &cli_key);
        tracing::info!(
            cli_key = %cli_key,
            provider_id,
            session_reuse_priority,
            cleared_session_bindings = cleared,
            "default route session reuse priority updated"
        );
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn insert_duplicate_test_provider(
        db: &crate::db::Db,
        name: &str,
        ordinary_policy: Option<crate::settings::ModelRoutingPolicy>,
    ) -> providers::ProviderSummary {
        providers::upsert(
            db,
            providers::ProviderUpsertParams {
                provider_id: None,
                cli_key: "claude".to_string(),
                name: name.to_string(),
                base_urls: vec!["https://example.invalid/v1".to_string()],
                base_url_mode: providers::ProviderBaseUrlMode::Order,
                auth_mode: Some(providers::ProviderAuthMode::ApiKey),
                api_key: Some("test-key".to_string()),
                enabled: true,
                cost_multiplier: 1.0,
                priority: Some(100),
                claude_models: None,
                model_mapping: None,
                availability_test_model: None,
                availability_probe_enabled: false,
                availability_probe_interval_minutes: 10,
                limit_5h_usd: None,
                limit_daily_usd: None,
                daily_reset_mode: Some(providers::DailyResetMode::Fixed),
                daily_reset_time: Some("00:00:00".to_string()),
                limit_weekly_usd: None,
                limit_monthly_usd: None,
                limit_total_usd: None,
                tags: None,
                note: None,
                source_provider_id: None,
                bridge_type: None,
                stream_idle_timeout_seconds: None,
                extension_values: None,
                account_usage_credentials_patch: None,
                account_usage_credentials_copy_from_provider_id: None,
                upstream_retry_policy_override: None,
                upstream_retry_policy_override_specified: false,
                model_routing_policy_override: ordinary_policy,
                model_routing_policy_override_specified: true,
            },
        )
        .expect("insert duplicate test provider")
    }

    #[test]
    fn custom_account_usage_permission_precheck_classifies_invalid_input() {
        let values = vec![providers::ProviderExtensionValuesInput {
            plugin_id: crate::domain::provider_account_usage::ACCOUNT_USAGE_PLUGIN_ID.to_string(),
            namespace: crate::domain::provider_account_usage::ACCOUNT_USAGE_NAMESPACE.to_string(),
            values: serde_json::json!({
                "adapterKind": "custom",
                "customAllowedOrigins": [],
                "customTimeoutSeconds": 5,
                "customEnabled": true
            }),
        }];

        let error =
            custom_account_usage_permission_request(Some(&values), "https://api.example.test/v1")
                .expect_err("invalid custom config must fail precheck");

        assert_eq!(error, "SEC_INVALID_INPUT: 自定义账户用量脚本不能为空");
    }

    #[test]
    fn provider_upsert_input_deserializes_runtime_camel_case_shape() {
        let input: ProviderUpsertInput = serde_json::from_value(serde_json::json!({
            "providerId": 1,
            "cliKey": "claude",
            "name": "P1",
            "baseUrls": ["https://example.com"],
            "baseUrlMode": "order",
            "authMode": "api_key",
            "apiKey": "k1",
            "enabled": true,
            "costMultiplier": 1.0,
            "priority": 10,
            "claudeModels": null,
            "availabilityProbeEnabled": true,
            "availabilityProbeIntervalMinutes": 30,
            "limit5hUsd": 5.0,
            "limitDailyUsd": 10.0,
            "dailyResetMode": "fixed",
            "dailyResetTime": "00:00:00",
            "limitWeeklyUsd": null,
            "limitMonthlyUsd": null,
            "limitTotalUsd": null,
            "tags": ["x"],
            "note": "n",
            "streamIdleTimeoutSeconds": 90
        }))
        .expect("deserialize provider input");

        assert_eq!(input.base_url_mode, providers::ProviderBaseUrlMode::Order);
        assert_eq!(input.auth_mode, Some(providers::ProviderAuthMode::ApiKey));
        assert_eq!(input.availability_probe_enabled, Some(true));
        assert_eq!(input.availability_probe_interval_minutes, Some(30));
        assert_eq!(input.limit_5h_usd, Some(5.0));
        assert_eq!(
            input.daily_reset_mode,
            Some(providers::DailyResetMode::Fixed)
        );
        assert_eq!(input.stream_idle_timeout_seconds, Some(90));
    }

    #[test]
    fn provider_upsert_input_accepts_legacy_generated_limit_alias() {
        let input: ProviderUpsertInput = serde_json::from_value(serde_json::json!({
            "providerId": 1,
            "cliKey": "claude",
            "name": "P1",
            "baseUrls": ["https://example.com"],
            "baseUrlMode": "ping",
            "enabled": true,
            "costMultiplier": 1.0,
            "limit5HUsd": 7.0,
            "limitDailyUsd": null,
            "dailyResetMode": "rolling",
            "dailyResetTime": "00:00:00",
            "limitWeeklyUsd": null,
            "limitMonthlyUsd": null,
            "limitTotalUsd": null
        }))
        .expect("deserialize provider input legacy alias");

        assert_eq!(input.base_url_mode, providers::ProviderBaseUrlMode::Ping);
        assert_eq!(input.availability_probe_enabled, None);
        assert_eq!(input.availability_probe_interval_minutes, None);
        assert_eq!(input.limit_5h_usd, Some(7.0));
        assert_eq!(
            input.daily_reset_mode,
            Some(providers::DailyResetMode::Rolling)
        );
    }

    #[test]
    fn provider_duplicate_copies_ordinary_policy_without_mode_membership_or_cross_policy() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = crate::db::init_for_tests(&dir.path().join("provider-duplicate-routing.db"))
            .expect("init db");
        let ordinary_policy = crate::settings::ModelRoutingPolicy {
            enabled: true,
            rules: vec![crate::settings::ModelRoutingRule {
                source_model: "source-model".to_string(),
                source_reasoning_effort: None,
                target_model: Some("ordinary-target".to_string()),
                reasoning_effort: None,
            }],
        };
        let source = insert_duplicate_test_provider(&db, "Source", Some(ordinary_policy.clone()));
        let target = insert_duplicate_test_provider(&db, "Target", None);
        let mode = crate::sort_modes::create_mode(&db, "Mode").expect("create mode");
        crate::sort_modes::set_mode_providers_order(
            &db,
            mode.id,
            "claude",
            vec![source.id, target.id],
        )
        .expect("set mode members");
        let cross_policy = crate::settings::CrossProviderModelRoutingPolicy {
            enabled: true,
            rules: vec![crate::settings::CrossProviderModelRoutingRule {
                source_model: "source-model".to_string(),
                source_reasoning_effort: None,
                target_provider_uuid: target.provider_uuid.clone(),
                target_model: Some("cross-target".to_string()),
                target_reasoning_effort: None,
            }],
        };
        let conn = db.open_connection().expect("open db");
        conn.execute(
            "UPDATE sort_mode_providers SET cross_provider_model_routing_policy_json = ?1 WHERE mode_id = ?2 AND provider_id = ?3",
            rusqlite::params![
                serde_json::to_string(&cross_policy).expect("serialize cross policy"),
                mode.id,
                source.id
            ],
        )
        .expect("seed source cross policy");
        drop(conn);

        let duplicate = duplicate_provider_in_db(&db, source.id).expect("duplicate provider");
        assert_ne!(duplicate.provider_uuid, source.provider_uuid);
        assert_eq!(
            duplicate.model_routing_policy_override.as_ref(),
            Some(&ordinary_policy)
        );

        let conn = db.open_connection().expect("reopen db");
        let duplicate_memberships: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sort_mode_providers WHERE provider_id = ?1",
                [duplicate.id],
                |row| row.get(0),
            )
            .expect("count duplicate memberships");
        assert_eq!(duplicate_memberships, 0);
        let source_cross: Option<String> = conn
            .query_row(
                "SELECT cross_provider_model_routing_policy_json FROM sort_mode_providers WHERE mode_id = ?1 AND provider_id = ?2",
                rusqlite::params![mode.id, source.id],
                |row| row.get(0),
            )
            .expect("read source cross policy");
        assert!(source_cross
            .as_deref()
            .is_some_and(|raw| raw.contains(&target.provider_uuid)));
    }

    #[test]
    fn provider_runtime_reset_decision_handles_create_and_non_sensitive_edits() {
        let next = providers::ProviderSummary {
            id: 1,
            provider_uuid: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            cli_key: "claude".to_string(),
            name: "Provider A".to_string(),
            base_urls: vec!["https://api.example.com".to_string()],
            base_url_mode: providers::ProviderBaseUrlMode::Order,
            claude_models: Default::default(),
            model_mapping: Default::default(),
            availability_test_model: None,
            availability_probe_enabled: false,
            availability_probe_interval_minutes: 10,
            enabled: true,
            priority: 1,
            cost_multiplier: 1.0,
            limit_5h_usd: None,
            limit_daily_usd: None,
            daily_reset_mode: providers::DailyResetMode::Fixed,
            daily_reset_time: "00:00:00".to_string(),
            limit_weekly_usd: None,
            limit_monthly_usd: None,
            limit_total_usd: None,
            tags: vec![],
            note: String::new(),
            created_at: 1,
            updated_at: 1,
            auth_mode: "api_key".to_string(),
            oauth_provider_type: None,
            oauth_email: None,
            oauth_expires_at: None,
            oauth_last_error: None,
            source_provider_id: None,
            bridge_type: None,
            stream_idle_timeout_seconds: None,
            extension_values: vec![],
            upstream_retry_policy_override: None,
            model_routing_policy_override: None,
            api_key_configured: true,
            newapi_account_user_id: None,
            newapi_account_access_token_configured: false,
        };

        assert_eq!(
            provider_runtime_reset_decision(None, None, &next, None),
            ProviderRuntimeResetDecision {
                clear_route_runtime_state: true,
            }
        );

        let mut disabled_create = next.clone();
        disabled_create.enabled = false;
        assert_eq!(
            provider_runtime_reset_decision(None, None, &disabled_create, None),
            ProviderRuntimeResetDecision::default()
        );

        let mut previous = next.clone();
        previous.name = "Old Name".to_string();
        previous.note = "old".to_string();
        previous.updated_at = 0;

        assert_eq!(
            provider_runtime_reset_decision(
                Some(&previous),
                Some("sk-existing"),
                &next,
                Some("   ")
            ),
            ProviderRuntimeResetDecision::default()
        );

        assert_eq!(
            provider_runtime_reset_decision(
                Some(&previous),
                Some("sk-existing"),
                &next,
                Some("sk-existing")
            ),
            ProviderRuntimeResetDecision::default()
        );

        let mut disabled = next.clone();
        disabled.enabled = false;

        assert_eq!(
            provider_runtime_reset_decision(Some(&next), Some("sk-existing"), &disabled, None),
            ProviderRuntimeResetDecision {
                clear_route_runtime_state: true,
            }
        );
    }

    #[test]
    fn provider_runtime_reset_decision_detects_sensitive_claude_changes() {
        let previous = providers::ProviderSummary {
            id: 1,
            provider_uuid: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            cli_key: "claude".to_string(),
            name: "Provider A".to_string(),
            base_urls: vec!["https://api.old.example.com".to_string()],
            base_url_mode: providers::ProviderBaseUrlMode::Order,
            claude_models: Default::default(),
            model_mapping: Default::default(),
            availability_test_model: None,
            availability_probe_enabled: false,
            availability_probe_interval_minutes: 10,
            enabled: true,
            priority: 1,
            cost_multiplier: 1.0,
            limit_5h_usd: None,
            limit_daily_usd: None,
            daily_reset_mode: providers::DailyResetMode::Fixed,
            daily_reset_time: "00:00:00".to_string(),
            limit_weekly_usd: None,
            limit_monthly_usd: None,
            limit_total_usd: None,
            tags: vec![],
            note: String::new(),
            created_at: 1,
            updated_at: 1,
            auth_mode: "api_key".to_string(),
            oauth_provider_type: None,
            oauth_email: None,
            oauth_expires_at: None,
            oauth_last_error: None,
            source_provider_id: None,
            bridge_type: None,
            stream_idle_timeout_seconds: None,
            extension_values: vec![],
            upstream_retry_policy_override: None,
            model_routing_policy_override: None,
            api_key_configured: true,
            newapi_account_user_id: None,
            newapi_account_access_token_configured: false,
        };

        let mut next = previous.clone();
        next.base_urls = vec!["https://api.new.example.com".to_string()];

        assert_eq!(
            provider_runtime_reset_decision(Some(&previous), Some("sk-old"), &next, None),
            ProviderRuntimeResetDecision {
                clear_route_runtime_state: true,
            }
        );

        let mut next_non_claude = previous.clone();
        next_non_claude.cli_key = "codex".to_string();

        assert_eq!(
            provider_runtime_reset_decision(
                Some(&next_non_claude),
                Some("sk-old"),
                &next_non_claude,
                Some("sk-new")
            ),
            ProviderRuntimeResetDecision {
                clear_route_runtime_state: true,
            }
        );
    }
}
