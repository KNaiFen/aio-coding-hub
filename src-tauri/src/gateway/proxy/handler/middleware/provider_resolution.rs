//! Middleware: resolves session routing and selects providers with session binding.

use super::{MiddlewareAction, ProxyContext};
use crate::gateway::proxy::forwarder::{filter_routing_candidates, needs_limit_evaluation};
use crate::gateway::proxy::handler::early_error::{
    build_early_error_log_ctx, early_error_contract, push_special_setting,
    respond_early_error_with_enqueue, respond_invalid_cli_key_with_spawn,
    respond_provider_selection_failed_with_spawn, EarlyErrorKind,
};
use crate::gateway::proxy::handler::provider_selection::{
    resolve_session_bound_provider_id, resolve_session_routing_decision,
    select_providers_with_session_binding, ProviderSelection,
};
use crate::gateway::response_fixer;

pub(in crate::gateway::proxy::handler) struct ProviderResolutionMiddleware;

const SESSION_ID_DIAGNOSTIC_SUFFIX_LEN: usize = 8;

impl ProviderResolutionMiddleware {
    pub(in crate::gateway::proxy::handler) async fn run<R: tauri::Runtime>(
        mut ctx: ProxyContext<R>,
    ) -> MiddlewareAction<R> {
        // --- session routing decision ---
        let decision = resolve_session_routing_decision(
            &ctx.headers,
            ctx.introspection_json.as_ref(),
            ctx.is_claude_count_tokens,
        );
        ctx.session_id = decision.session_id;
        ctx.allow_session_reuse = decision.allow_session_reuse && ctx.managed_model_route.is_none();
        let enable_session_reuse = ctx
            .runtime_settings
            .as_ref()
            .expect("runtime_settings must be populated before provider resolution")
            .enable_session_reuse;

        // --- provider selection ---
        // Runs rusqlite queries; keep them off the async worker via the bounded
        // blocking pool (pool.get can block up to 5s under DB contention).
        let selection_result = {
            let state = ctx.state.clone();
            let cli_key = ctx.cli_key.clone();
            let session_id = ctx.session_id.clone();
            let created_at = ctx.created_at;
            let managed_provider_identity = ctx
                .managed_model_route
                .as_ref()
                .map(|route| (route.provider_id, route.provider_uuid.clone()));
            crate::blocking::run("gateway_provider_selection", move || {
                if let Some((provider_id, provider_uuid)) = managed_provider_identity {
                    let route_generation = state.session.capture_route_generation(&cli_key);
                    let providers =
                        crate::providers::get_enabled_direct_codex_for_gateway_by_identity(
                            &state.db,
                            provider_id,
                            &provider_uuid,
                        )?
                        .into_iter()
                        .collect();
                    Ok(ProviderSelection {
                        route_generation,
                        effective_sort_mode_id: None,
                        effective_sort_mode_uuid: None,
                        providers,
                        sort_mode_members: Vec::new(),
                        bound_provider_order: None,
                        active_sort_mode_id: None,
                        session_bound_sort_mode_id: None,
                    })
                } else {
                    select_providers_with_session_binding(
                        &state,
                        &cli_key,
                        session_id.as_deref(),
                        created_at,
                        enable_session_reuse,
                    )
                }
            })
            .await
        };
        let selection = match selection_result {
            Ok(s) => s,
            Err(err) => {
                let log_ctx = build_early_error_log_ctx(&ctx);
                let special_settings_json =
                    response_fixer::special_settings_json(&ctx.special_settings);
                // A rejected cli key is the caller's fault (400); everything
                // else here is infrastructure (DB pool / blocking pool) and
                // must not be misfiled as a client error.
                let resp = if err.code() == "SEC_INVALID_INPUT" {
                    respond_invalid_cli_key_with_spawn(
                        &log_ctx,
                        special_settings_json,
                        ctx.session_id.clone(),
                        ctx.requested_model.clone(),
                        err.to_string(),
                    )
                } else {
                    respond_provider_selection_failed_with_spawn(
                        &log_ctx,
                        special_settings_json,
                        ctx.session_id.clone(),
                        ctx.requested_model.clone(),
                        err.to_string(),
                    )
                };
                return MiddlewareAction::ShortCircuit(resp);
            }
        };

        let initial_provider_ids = provider_ids(&selection.providers);
        ctx.route_generation = Some(selection.route_generation);
        ctx.effective_sort_mode_id = selection.effective_sort_mode_id;
        ctx.effective_sort_mode_uuid = selection.effective_sort_mode_uuid.clone();
        ctx.sort_mode_members = selection.sort_mode_members.clone();
        ctx.providers = selection.providers;

        // A known OAuth or spend-limit exhaustion is route eligibility, not a
        // failed attempt. Filter it before session binding so the first
        // actually usable provider remains provider_index=1 and can reuse an
        // existing session binding.
        let limit_excluded_provider_ids = if !ctx.providers.iter().any(needs_limit_evaluation) {
            Vec::new()
        } else {
            let state = ctx.state.clone();
            let created_at = ctx.created_at;
            let providers_to_filter = std::mem::take(&mut ctx.providers);
            let fallback_providers = providers_to_filter.clone();
            let filter_result = crate::blocking::run(
                "gateway_provider_limit_filter",
                move || -> crate::shared::error::AppResult<_> {
                    let conn = match state.db.open_connection() {
                        Ok(conn) => conn,
                        Err(err) => {
                            tracing::warn!(
                                "failed to open database for provider limit prefilter; allowing candidates: {err}"
                            );
                            return Ok((providers_to_filter, Vec::new()));
                        }
                    };
                    let (eligible, excluded_provider_ids) =
                        filter_routing_candidates(&conn, providers_to_filter, created_at);
                    Ok((eligible, excluded_provider_ids))
                },
            )
            .await;

            match filter_result {
                Ok((eligible, excluded_provider_ids)) => {
                    ctx.providers = eligible;
                    excluded_provider_ids
                }
                Err(err) => {
                    tracing::warn!(
                        trace_id = %ctx.trace_id,
                        cli_key = %ctx.cli_key,
                        "provider limit prefilter failed; allowing candidates: {err}"
                    );
                    ctx.providers = fallback_providers;
                    Vec::new()
                }
            }
        };

        // --- session bound provider ---
        ctx.session_bound_provider_id = resolve_session_bound_provider_id(
            ctx.state.session.as_ref(),
            ctx.state.circuit.as_ref(),
            &ctx.cli_key,
            ctx.session_id.as_deref(),
            selection.route_generation,
            ctx.created_at,
            enable_session_reuse,
            ctx.allow_session_reuse,
            &mut ctx.providers,
            selection.bound_provider_order.as_deref(),
        );

        // --- no enabled provider guard ---
        if ctx.providers.is_empty() {
            let final_provider_ids = provider_ids(&ctx.providers);

            push_special_setting(
                &ctx.special_settings,
                no_enabled_provider_diagnostic(&NoEnabledProviderDiagnosticArgs {
                    cli_key: &ctx.cli_key,
                    active_sort_mode_id: selection.active_sort_mode_id,
                    effective_sort_mode_id: ctx.effective_sort_mode_id,
                    session_bound_sort_mode_id: selection.session_bound_sort_mode_id,
                    session_id: ctx.session_id.as_deref(),
                    session_bound_provider_id: ctx.session_bound_provider_id,
                    initial_provider_ids: &initial_provider_ids,
                    final_provider_ids: &final_provider_ids,
                    limit_excluded_provider_ids: &limit_excluded_provider_ids,
                }),
            );
            let contract = early_error_contract(EarlyErrorKind::NoEnabledProvider);
            let message = no_enabled_provider_message(&ctx.cli_key);
            let session_id = ctx.session_id.take();
            let requested_model = ctx.requested_model.take();
            let special_settings_json =
                response_fixer::special_settings_json(&ctx.special_settings);
            let log_ctx = build_early_error_log_ctx(&ctx);

            let resp = respond_early_error_with_enqueue(
                &log_ctx,
                contract,
                message,
                special_settings_json,
                session_id,
                requested_model,
            )
            .await;
            return MiddlewareAction::ShortCircuit(resp);
        }

        MiddlewareAction::Continue(Box::new(ctx))
    }
}

pub(in crate::gateway::proxy::handler) fn no_enabled_provider_message(cli_key: &str) -> String {
    format!("no enabled provider for cli_key={cli_key}")
}

struct NoEnabledProviderDiagnosticArgs<'a> {
    cli_key: &'a str,
    active_sort_mode_id: Option<i64>,
    effective_sort_mode_id: Option<i64>,
    session_bound_sort_mode_id: Option<Option<i64>>,
    session_id: Option<&'a str>,
    session_bound_provider_id: Option<i64>,
    initial_provider_ids: &'a [i64],
    final_provider_ids: &'a [i64],
    limit_excluded_provider_ids: &'a [i64],
}

fn no_enabled_provider_diagnostic(args: &NoEnabledProviderDiagnosticArgs<'_>) -> serde_json::Value {
    let sort_mode = match args.effective_sort_mode_id {
        Some(id) => serde_json::json!({"kind": "custom", "modeId": id}),
        None => serde_json::json!({"kind": "default", "modeId": serde_json::Value::Null}),
    };
    let cleared_reason =
        if !args.limit_excluded_provider_ids.is_empty() && args.final_provider_ids.is_empty() {
            "all_candidates_limit_excluded"
        } else if args.effective_sort_mode_id.is_some() {
            "empty_sort_mode_candidates"
        } else {
            "empty_default_candidates"
        };

    serde_json::json!({
        "type": "provider_selection_diagnostic",
        "scope": "request",
        "hit": true,
        "reason": "no_enabled_provider",
        "clearedReason": cleared_reason,
        "cliKey": args.cli_key,
        "sortMode": sort_mode,
        "activeSortModeId": args.active_sort_mode_id,
        "effectiveSortModeId": args.effective_sort_mode_id,
        "sessionBoundSortModeId": args.session_bound_sort_mode_id,
        "sortModeSource": if args.session_bound_sort_mode_id.is_some() {
            "session_bound"
        } else {
            "active"
        },
        "sessionIdPresent": args.session_id.is_some(),
        "sessionIdSuffix": args.session_id.map(diagnostic_session_suffix),
        "sessionBoundProviderId": args.session_bound_provider_id,
        "candidateProviderIds": args.initial_provider_ids,
        "candidateProviderCount": args.initial_provider_ids.len(),
        "candidateProviderIdsAfterLimit": args.final_provider_ids,
        "candidateProviderCountAfterLimit": args.final_provider_ids.len(),
        "limitExcludedProviderIds": args.limit_excluded_provider_ids,
        "limitExcludedProviderCount": args.limit_excluded_provider_ids.len(),
    })
}

fn provider_ids(providers: &[crate::providers::ProviderForGateway]) -> Vec<i64> {
    providers.iter().map(|provider| provider.id).collect()
}

fn diagnostic_session_suffix(session_id: &str) -> String {
    let suffix: Vec<char> = session_id
        .chars()
        .rev()
        .take(SESSION_ID_DIAGNOSTIC_SUFFIX_LEN)
        .collect();
    suffix.into_iter().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_enabled_provider_message_preserves_cli_key() {
        assert_eq!(
            no_enabled_provider_message("codex"),
            "no enabled provider for cli_key=codex"
        );
    }

    #[test]
    fn no_enabled_provider_diagnostic_marks_empty_active_candidates() {
        let value = no_enabled_provider_diagnostic(&NoEnabledProviderDiagnosticArgs {
            cli_key: "claude",
            active_sort_mode_id: Some(6),
            effective_sort_mode_id: Some(6),
            session_bound_sort_mode_id: None,
            session_id: Some("01234567-89ab-cdef-0123-456789abcdef"),
            session_bound_provider_id: None,
            initial_provider_ids: &[],
            final_provider_ids: &[],
            limit_excluded_provider_ids: &[],
        });

        assert_eq!(
            value.get("type").and_then(|v| v.as_str()),
            Some("provider_selection_diagnostic")
        );
        assert_eq!(
            value.get("clearedReason").and_then(|v| v.as_str()),
            Some("empty_sort_mode_candidates")
        );
        assert_eq!(
            value.pointer("/sortMode/kind").and_then(|v| v.as_str()),
            Some("custom")
        );
        assert_eq!(
            value.get("activeSortModeId").and_then(|v| v.as_i64()),
            Some(6)
        );
        assert_eq!(
            value.get("effectiveSortModeId").and_then(|v| v.as_i64()),
            Some(6)
        );
        assert_eq!(
            value.get("sortModeSource").and_then(|v| v.as_str()),
            Some("active")
        );
        assert_eq!(
            value.get("sessionIdPresent").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(
            value.get("sessionIdSuffix").and_then(|v| v.as_str()),
            Some("89abcdef")
        );
        assert!(!value.to_string().contains("01234567-89ab-cdef"));
        assert_eq!(
            value.get("candidateProviderCount").and_then(|v| v.as_u64()),
            Some(0)
        );
    }

    #[test]
    fn no_enabled_provider_diagnostic_marks_limit_exclusions() {
        let value = no_enabled_provider_diagnostic(&NoEnabledProviderDiagnosticArgs {
            cli_key: "codex",
            active_sort_mode_id: None,
            effective_sort_mode_id: None,
            session_bound_sort_mode_id: None,
            session_id: Some("session-limit-test"),
            session_bound_provider_id: None,
            initial_provider_ids: &[11, 22],
            final_provider_ids: &[],
            limit_excluded_provider_ids: &[11, 22],
        });

        assert_eq!(
            value.get("clearedReason").and_then(|v| v.as_str()),
            Some("all_candidates_limit_excluded")
        );
        assert_eq!(
            value
                .get("candidateProviderIds")
                .and_then(|v| v.as_array())
                .map(|items| items.iter().filter_map(|v| v.as_i64()).collect::<Vec<_>>()),
            Some(vec![11, 22])
        );
        assert_eq!(
            value
                .get("limitExcludedProviderIds")
                .and_then(|v| v.as_array())
                .map(|items| items.iter().filter_map(|v| v.as_i64()).collect::<Vec<_>>()),
            Some(vec![11, 22])
        );
        assert_eq!(
            value
                .get("candidateProviderCountAfterLimit")
                .and_then(|v| v.as_u64()),
            Some(0)
        );
    }
}
