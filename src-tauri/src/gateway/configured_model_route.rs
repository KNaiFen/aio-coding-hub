//! User-configured, provider-aware model and reasoning-effort rewrites.

use axum::body::Bytes;
use serde_json::{json, Map, Value};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::gateway) struct ConfiguredModelRoute {
    pub(in crate::gateway) provider_id: i64,
    pub(in crate::gateway) provider_name: String,
    pub(in crate::gateway) policy_source: &'static str,
    pub(in crate::gateway) source_model: String,
    pub(in crate::gateway) target_model: Option<String>,
    pub(in crate::gateway) reasoning_effort: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::gateway) struct ConfiguredModelRouteOutcome {
    pub(in crate::gateway) body: Option<Bytes>,
    pub(in crate::gateway) effective_model: Option<String>,
    pub(in crate::gateway) model_applied: bool,
    pub(in crate::gateway) reasoning_effort_applied: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::gateway) struct CrossProviderRoutePlan {
    pub(in crate::gateway) target_provider_uuid: String,
    pub(in crate::gateway) target_model: Option<String>,
    pub(in crate::gateway) target_reasoning_effort: Option<String>,
}

impl ConfiguredModelRouteOutcome {
    fn applied(&self) -> bool {
        self.model_applied || self.reasoning_effort_applied
    }
}

#[allow(clippy::too_many_arguments)]
pub(in crate::gateway) fn resolve(
    cli_key: &str,
    method: &str,
    path: &str,
    requested_model: Option<&str>,
    source_reasoning_effort: Option<&str>,
    managed_model_route: bool,
    global_policy: &crate::settings::ModelRoutingPolicy,
    provider_policy: Option<&crate::settings::ModelRoutingPolicy>,
    provider_id: i64,
    provider_name: &str,
) -> Option<ConfiguredModelRoute> {
    if managed_model_route
        || !crate::gateway::observation::is_model_inference_request(cli_key, method, path)
    {
        return None;
    }

    let requested_model = requested_model
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    if requested_model.starts_with("aio/") {
        return None;
    }

    let (policy, policy_source) = provider_policy
        .map(|policy| (policy, "provider"))
        .unwrap_or((global_policy, "global"));
    if !policy.enabled {
        return None;
    }

    let rule = resolve_ordinary_rule(&policy.rules, requested_model, source_reasoning_effort)?;
    if rule.target_model.is_none() && rule.reasoning_effort.is_none() {
        return None;
    }

    Some(ConfiguredModelRoute {
        provider_id,
        provider_name: provider_name.to_string(),
        policy_source,
        source_model: requested_model.to_string(),
        target_model: rule.target_model.clone(),
        reasoning_effort: rule.reasoning_effort.clone(),
    })
}

/// Source-model matching is exact and case-sensitive. An explicit source effort
/// wins over the legacy model-only wildcard regardless of saved rule order.
pub(in crate::gateway) fn resolve_ordinary_rule<'a>(
    rules: &'a [crate::settings::ModelRoutingRule],
    requested_model: &str,
    source_reasoning_effort: Option<&str>,
) -> Option<&'a crate::settings::ModelRoutingRule> {
    source_reasoning_effort
        .and_then(|effort| {
            rules.iter().find(|rule| {
                rule.source_model == requested_model
                    && rule.source_reasoning_effort.as_deref() == Some(effort)
            })
        })
        .or_else(|| {
            rules.iter().find(|rule| {
                rule.source_model == requested_model && rule.source_reasoning_effort.is_none()
            })
        })
}

/// Cross-provider rules have the same exact-before-wildcard matching semantics
/// as ordinary rules. The caller resolves the target UUID against the request's
/// immutable named-mode member snapshot before constructing an execution route.
pub(in crate::gateway) fn resolve_cross_plan(
    cli_key: &str,
    method: &str,
    path: &str,
    requested_model: Option<&str>,
    source_reasoning_effort: Option<&str>,
    managed_model_route: bool,
    effective_sort_mode_uuid: Option<&str>,
    policy: Option<&crate::settings::CrossProviderModelRoutingPolicy>,
) -> Option<CrossProviderRoutePlan> {
    if managed_model_route
        || effective_sort_mode_uuid.is_none()
        || !crate::gateway::observation::is_model_inference_request(cli_key, method, path)
    {
        return None;
    }

    let requested_model = requested_model
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    if requested_model.starts_with("aio/") {
        return None;
    }

    let policy = policy.filter(|policy| policy.enabled)?;
    let rule = resolve_cross_rule(&policy.rules, requested_model, source_reasoning_effort)?;
    Some(CrossProviderRoutePlan {
        target_provider_uuid: rule.target_provider_uuid.clone(),
        target_model: rule.target_model.clone(),
        target_reasoning_effort: rule.target_reasoning_effort.clone(),
    })
}

pub(in crate::gateway) fn resolve_cross_rule<'a>(
    rules: &'a [crate::settings::CrossProviderModelRoutingRule],
    requested_model: &str,
    source_reasoning_effort: Option<&str>,
) -> Option<&'a crate::settings::CrossProviderModelRoutingRule> {
    source_reasoning_effort
        .and_then(|effort| {
            rules.iter().find(|rule| {
                rule.source_model == requested_model
                    && rule.source_reasoning_effort.as_deref() == Some(effort)
            })
        })
        .or_else(|| {
            rules.iter().find(|rule| {
                rule.source_model == requested_model && rule.source_reasoning_effort.is_none()
            })
        })
}

pub(in crate::gateway) fn cross_execution_route(
    target_provider_id: i64,
    target_provider_name: &str,
    source_model: &str,
    plan: &CrossProviderRoutePlan,
) -> ConfiguredModelRoute {
    ConfiguredModelRoute {
        provider_id: target_provider_id,
        provider_name: target_provider_name.to_string(),
        policy_source: "provider_cross",
        source_model: source_model.to_string(),
        target_model: plan.target_model.clone(),
        reasoning_effort: plan.target_reasoning_effort.clone(),
    }
}

pub(in crate::gateway) fn mark_pending(
    special_settings: &Arc<Mutex<Vec<Value>>>,
    route: &ConfiguredModelRoute,
) {
    upsert_route_setting(
        special_settings,
        route_setting(route, None, None, false, false, false),
    );
}

pub(in crate::gateway) fn mark_result(
    special_settings: &Arc<Mutex<Vec<Value>>>,
    route: &ConfiguredModelRoute,
    priced_cli_key: &str,
    outcome: &ConfiguredModelRouteOutcome,
) {
    upsert_route_setting(
        special_settings,
        route_setting(
            route,
            Some(priced_cli_key),
            outcome.effective_model.as_deref(),
            outcome.applied(),
            outcome.model_applied,
            outcome.reasoning_effort_applied,
        ),
    );
}

fn upsert_route_setting(special_settings: &Arc<Mutex<Vec<Value>>>, setting: serde_json::Value) {
    crate::gateway::response_fixer::upsert_configured_model_route(special_settings, setting);
}

fn route_setting(
    route: &ConfiguredModelRoute,
    priced_cli_key: Option<&str>,
    effective_model: Option<&str>,
    applied: bool,
    model_applied: bool,
    reasoning_effort_applied: bool,
) -> Value {
    json!({
        "type": "configured_model_route",
        "scope": "request",
        "providerId": route.provider_id,
        "providerName": route.provider_name,
        "policySource": route.policy_source,
        "sourceModel": route.source_model,
        "targetModel": route.target_model,
        "reasoningEffort": route.reasoning_effort,
        "effectiveModel": effective_model,
        "pricedCliKey": priced_cli_key,
        "pricedModel": effective_model,
        "applied": applied,
        "modelApplied": model_applied,
        "reasoningEffortApplied": reasoning_effort_applied,
    })
}

pub(in crate::gateway) fn apply(
    route: &ConfiguredModelRoute,
    priced_cli_key: &str,
    path: &mut String,
    query: &mut Option<String>,
    body: &[u8],
) -> ConfiguredModelRouteOutcome {
    let mut body_json = serde_json::from_slice::<Value>(body).ok();
    let mut body_changed = false;
    let mut model_applied = false;

    if let Some(target_model) = route.target_model.as_deref() {
        let location = crate::gateway::util::infer_requested_model_info(
            path,
            query.as_deref(),
            body_json.as_ref(),
        )
        .location;
        model_applied = match location {
            Some(crate::gateway::util::RequestedModelLocation::Path) => {
                if let Some(next_path) =
                    crate::gateway::proxy::model_rewrite::replace_model_in_path(path, target_model)
                {
                    *path = next_path;
                    true
                } else {
                    false
                }
            }
            Some(crate::gateway::util::RequestedModelLocation::Query) => {
                if let Some(current_query) = query.as_deref() {
                    let next_query = crate::gateway::proxy::model_rewrite::replace_model_in_query(
                        current_query,
                        target_model,
                    );
                    *query = Some(next_query);
                    true
                } else {
                    false
                }
            }
            Some(crate::gateway::util::RequestedModelLocation::BodyJson) | None => {
                body_json.as_mut().is_some_and(|root| {
                    let changed = crate::gateway::proxy::model_rewrite::replace_model_in_body_json(
                        root,
                        target_model,
                    );
                    body_changed |= changed;
                    changed
                })
            }
        };

        if !model_applied {
            model_applied = body_json.as_mut().is_some_and(|root| {
                let changed = crate::gateway::proxy::model_rewrite::replace_model_in_body_json(
                    root,
                    target_model,
                );
                body_changed |= changed;
                changed
            });
        }
    }

    let mut reasoning_effort_applied = route
        .reasoning_effort
        .as_deref()
        .and_then(|effort| {
            body_json
                .as_mut()
                .map(|root| apply_reasoning_effort(priced_cli_key, path, root, effort))
        })
        .unwrap_or(false);
    body_changed |= reasoning_effort_applied;

    let serialized_body = body_changed
        .then(|| {
            body_json
                .as_ref()
                .and_then(|root| serde_json::to_vec(root).ok())
        })
        .flatten()
        .map(Bytes::from);
    if body_changed && serialized_body.is_none() {
        reasoning_effort_applied = false;
    }
    let final_body = serialized_body.as_deref().unwrap_or(body);
    let final_json = serde_json::from_slice::<Value>(final_body).ok();
    let effective_model = crate::gateway::util::infer_requested_model_info(
        path,
        query.as_deref(),
        final_json.as_ref(),
    )
    .model;
    if let Some(target_model) = route.target_model.as_deref() {
        model_applied = effective_model.as_deref() == Some(target_model);
    }

    ConfiguredModelRouteOutcome {
        body: serialized_body,
        effective_model,
        model_applied,
        reasoning_effort_applied,
    }
}

fn apply_reasoning_effort(
    priced_cli_key: &str,
    path: &str,
    root: &mut Value,
    effort: &str,
) -> bool {
    let Some(root) = root.as_object_mut() else {
        return false;
    };
    let normalized_path = path
        .split('?')
        .next()
        .unwrap_or(path)
        .trim_end_matches('/')
        .to_ascii_lowercase();

    if normalized_path.ends_with(":generatecontent")
        || normalized_path.ends_with(":streamgeneratecontent")
        || priced_cli_key == "gemini"
    {
        let Some(generation_config) = object_slot(root, "generationConfig") else {
            return false;
        };
        let Some(thinking_config) = object_slot(generation_config, "thinkingConfig") else {
            return false;
        };
        thinking_config.insert("thinkingLevel".to_string(), json!(effort));
        thinking_config.remove("thinkingBudget");
        return true;
    }

    if normalized_path.ends_with("/messages") || priced_cli_key == "claude" {
        let Some(output_config) = object_slot(root, "output_config") else {
            return false;
        };
        output_config.insert("effort".to_string(), json!(effort));
        return true;
    }

    if normalized_path.ends_with("/chat/completions") {
        root.insert("reasoning_effort".to_string(), json!(effort));
        return true;
    }

    if normalized_path.ends_with("/responses")
        || normalized_path.ends_with("/responses/compact")
        || priced_cli_key == "codex"
        || priced_cli_key == "grok"
    {
        let Some(reasoning) = object_slot(root, "reasoning") else {
            return false;
        };
        reasoning.insert("effort".to_string(), json!(effort));
        return true;
    }

    false
}

fn object_slot<'a>(
    root: &'a mut Map<String, Value>,
    key: &str,
) -> Option<&'a mut Map<String, Value>> {
    let value = root.entry(key.to_string()).or_insert_with(|| json!({}));
    if !value.is_object() {
        *value = json!({});
    }
    value.as_object_mut()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(rule: crate::settings::ModelRoutingRule) -> crate::settings::ModelRoutingPolicy {
        crate::settings::ModelRoutingPolicy {
            enabled: true,
            rules: vec![rule],
        }
    }

    #[test]
    fn provider_policy_overrides_global_policy_as_a_whole() {
        let global = policy(crate::settings::ModelRoutingRule {
            source_model: "fable5".to_string(),
            source_reasoning_effort: None,
            target_model: Some("opus4.8".to_string()),
            reasoning_effort: None,
        });
        let provider = crate::settings::ModelRoutingPolicy::default();

        assert!(resolve(
            "claude",
            "POST",
            "/v1/messages",
            Some("fable5"),
            None,
            false,
            &global,
            Some(&provider),
            7,
            "backup",
        )
        .is_none());
    }

    #[test]
    fn exact_case_sensitive_matching_and_managed_alias_exclusion() {
        let global = policy(crate::settings::ModelRoutingRule {
            source_model: "fable5".to_string(),
            source_reasoning_effort: None,
            target_model: Some("opus4.8".to_string()),
            reasoning_effort: None,
        });

        assert!(resolve(
            "claude",
            "POST",
            "/v1/messages",
            Some("Fable5"),
            None,
            false,
            &global,
            None,
            7,
            "backup",
        )
        .is_none());
        assert!(resolve(
            "codex",
            "POST",
            "/v1/responses",
            Some("aio/11111111-1111-4111-8111-111111111111"),
            None,
            false,
            &global,
            None,
            7,
            "backup",
        )
        .is_none());
    }

    #[test]
    fn source_effort_exact_rule_wins_over_model_only_wildcard() {
        let rules = vec![
            crate::settings::ModelRoutingRule {
                source_model: "fable5".to_string(),
                source_reasoning_effort: None,
                target_model: Some("fallback".to_string()),
                reasoning_effort: None,
            },
            crate::settings::ModelRoutingRule {
                source_model: "fable5".to_string(),
                source_reasoning_effort: Some("high".to_string()),
                target_model: Some("precise".to_string()),
                reasoning_effort: None,
            },
        ];

        assert_eq!(
            resolve_ordinary_rule(&rules, "fable5", Some("high"))
                .and_then(|rule| rule.target_model.as_deref()),
            Some("precise")
        );
        assert_eq!(
            resolve_ordinary_rule(&rules, "fable5", Some("low"))
                .and_then(|rule| rule.target_model.as_deref()),
            Some("fallback")
        );
    }

    fn cross_policy(
        rules: Vec<crate::settings::CrossProviderModelRoutingRule>,
    ) -> crate::settings::CrossProviderModelRoutingPolicy {
        crate::settings::CrossProviderModelRoutingPolicy {
            enabled: true,
            rules,
        }
    }

    #[test]
    fn cross_exact_effort_wins_over_wildcard_and_builds_target_route() {
        let policy = cross_policy(vec![
            crate::settings::CrossProviderModelRoutingRule {
                source_model: "fable5".to_string(),
                source_reasoning_effort: None,
                target_provider_uuid: "00000000-0000-4000-8000-000000000002".to_string(),
                target_model: Some("fallback".to_string()),
                target_reasoning_effort: None,
            },
            crate::settings::CrossProviderModelRoutingRule {
                source_model: "fable5".to_string(),
                source_reasoning_effort: Some("high".to_string()),
                target_provider_uuid: "00000000-0000-4000-8000-000000000003".to_string(),
                target_model: Some("precise".to_string()),
                target_reasoning_effort: Some("low".to_string()),
            },
        ]);

        let plan = resolve_cross_plan(
            "claude",
            "POST",
            "/v1/messages",
            Some("fable5"),
            Some("high"),
            false,
            Some("10000000-0000-4000-8000-000000000001"),
            Some(&policy),
        )
        .expect("cross plan");
        assert_eq!(
            plan.target_provider_uuid,
            "00000000-0000-4000-8000-000000000003"
        );

        let route = cross_execution_route(3, "target", "fable5", &plan);
        assert_eq!(route.provider_id, 3);
        assert_eq!(route.provider_name, "target");
        assert_eq!(route.policy_source, "provider_cross");
        assert_eq!(route.target_model.as_deref(), Some("precise"));
        assert_eq!(route.reasoning_effort.as_deref(), Some("low"));
    }

    #[test]
    fn cross_plan_requires_named_inference_and_excludes_managed_aliases() {
        let policy = cross_policy(vec![crate::settings::CrossProviderModelRoutingRule {
            source_model: "fable5".to_string(),
            source_reasoning_effort: None,
            target_provider_uuid: "00000000-0000-4000-8000-000000000002".to_string(),
            target_model: None,
            target_reasoning_effort: None,
        }]);

        for (method, path, model, managed, mode_uuid) in [
            ("POST", "/v1/messages", "fable5", false, None),
            (
                "POST",
                "/v1/messages",
                "fable5",
                true,
                Some("10000000-0000-4000-8000-000000000001"),
            ),
            (
                "POST",
                "/v1/messages/count_tokens",
                "fable5",
                false,
                Some("10000000-0000-4000-8000-000000000001"),
            ),
            (
                "GET",
                "/v1/messages",
                "fable5",
                false,
                Some("10000000-0000-4000-8000-000000000001"),
            ),
            (
                "POST",
                "/v1/messages",
                "aio/00000000-0000-4000-8000-000000000001",
                false,
                Some("10000000-0000-4000-8000-000000000001"),
            ),
            (
                "POST",
                "/v1/messages",
                "Fable5",
                false,
                Some("10000000-0000-4000-8000-000000000001"),
            ),
        ] {
            assert!(resolve_cross_plan(
                "claude",
                method,
                path,
                Some(model),
                None,
                managed,
                mode_uuid,
                Some(&policy),
            )
            .is_none());
        }
    }

    #[test]
    fn rewrites_claude_model_and_effort() {
        let route = ConfiguredModelRoute {
            provider_id: 7,
            provider_name: "backup".to_string(),
            policy_source: "provider",
            source_model: "fable5".to_string(),
            target_model: Some("opus4.8".to_string()),
            reasoning_effort: Some("low".to_string()),
        };
        let mut path = "/v1/messages".to_string();
        let mut query = None;
        let outcome = apply(
            &route,
            "claude",
            &mut path,
            &mut query,
            br#"{"model":"fable5","output_config":{"effort":"high"}}"#,
        );
        let body: Value = serde_json::from_slice(outcome.body.as_deref().unwrap()).unwrap();

        assert_eq!(body["model"], "opus4.8");
        assert_eq!(body["output_config"]["effort"], "low");
        assert_eq!(outcome.effective_model.as_deref(), Some("opus4.8"));
    }

    #[test]
    fn rewrites_gemini_path_and_thinking_level() {
        let route = ConfiguredModelRoute {
            provider_id: 7,
            provider_name: "backup".to_string(),
            policy_source: "provider",
            source_model: "gemini-pro".to_string(),
            target_model: Some("publisher/gemini-flash".to_string()),
            reasoning_effort: Some("high".to_string()),
        };
        let mut path = "/v1beta/models/gemini-pro:streamGenerateContent".to_string();
        let mut query = None;
        let outcome = apply(&route, "gemini", &mut path, &mut query, br#"{}"#);
        let body: Value = serde_json::from_slice(outcome.body.as_deref().unwrap()).unwrap();

        assert!(path.contains("/models/publisher%2Fgemini-flash:streamGenerateContent"));
        assert_eq!(
            body["generationConfig"]["thinkingConfig"]["thinkingLevel"],
            "high"
        );
        assert!(body["generationConfig"]["thinkingConfig"]
            .get("thinkingBudget")
            .is_none());
        assert_eq!(
            outcome.effective_model.as_deref(),
            Some("publisher/gemini-flash")
        );
    }

    #[test]
    fn rewrites_codex_compact_model_and_responses_effort() {
        let route = ConfiguredModelRoute {
            provider_id: 7,
            provider_name: "backup".to_string(),
            policy_source: "global",
            source_model: "gpt-expensive".to_string(),
            target_model: Some("gpt-cheap".to_string()),
            reasoning_effort: Some("low".to_string()),
        };
        let mut path = "/nested/v1/responses/compact/".to_string();
        let mut query = None;
        let outcome = apply(
            &route,
            "codex",
            &mut path,
            &mut query,
            br#"{"model":"gpt-expensive","reasoning":{"effort":"high"}}"#,
        );
        let body: Value = serde_json::from_slice(outcome.body.as_deref().unwrap()).unwrap();

        assert_eq!(body["model"], "gpt-cheap");
        assert_eq!(body["reasoning"]["effort"], "low");
        assert!(outcome.model_applied);
        assert!(outcome.reasoning_effort_applied);
    }

    #[test]
    fn rewrites_grok_chat_completions_effort_field() {
        let route = ConfiguredModelRoute {
            provider_id: 7,
            provider_name: "backup".to_string(),
            policy_source: "provider",
            source_model: "grok-expensive".to_string(),
            target_model: None,
            reasoning_effort: Some("minimal".to_string()),
        };
        let mut path = "/v1/chat/completions".to_string();
        let mut query = None;
        let outcome = apply(
            &route,
            "grok",
            &mut path,
            &mut query,
            br#"{"model":"grok-expensive","reasoning_effort":"high"}"#,
        );
        let body: Value = serde_json::from_slice(outcome.body.as_deref().unwrap()).unwrap();

        assert_eq!(body["model"], "grok-expensive");
        assert_eq!(body["reasoning_effort"], "minimal");
        assert!(!outcome.model_applied);
        assert!(outcome.reasoning_effort_applied);
    }

    #[test]
    fn gemini_non_numeric_effort_uses_level_and_removes_budget() {
        let route = ConfiguredModelRoute {
            provider_id: 7,
            provider_name: "backup".to_string(),
            policy_source: "global",
            source_model: "gemini-pro".to_string(),
            target_model: None,
            reasoning_effort: Some("HIGH".to_string()),
        };
        let mut path = "/v1beta/models/gemini-pro:generateContent".to_string();
        let mut query = None;
        let outcome = apply(
            &route,
            "gemini",
            &mut path,
            &mut query,
            br#"{"generationConfig":{"thinkingConfig":{"thinkingBudget":512}}}"#,
        );
        let body: Value = serde_json::from_slice(outcome.body.as_deref().unwrap()).unwrap();
        let thinking = &body["generationConfig"]["thinkingConfig"];

        assert_eq!(thinking["thinkingLevel"], "HIGH");
        assert!(thinking.get("thinkingBudget").is_none());
    }

    #[test]
    fn resolve_excludes_auxiliary_and_non_post_requests() {
        let global = policy(crate::settings::ModelRoutingRule {
            source_model: "fable5".to_string(),
            source_reasoning_effort: None,
            target_model: Some("opus4.8".to_string()),
            reasoning_effort: None,
        });

        for (method, path) in [
            ("GET", "/v1/responses"),
            ("POST", "/v1/models"),
            ("POST", "/v1/messages/count_tokens"),
        ] {
            assert!(resolve(
                "claude",
                method,
                path,
                Some("fable5"),
                None,
                false,
                &global,
                None,
                7,
                "backup",
            )
            .is_none());
        }
    }

    #[test]
    fn malformed_body_fails_open_for_effort_only_rule() {
        let route = ConfiguredModelRoute {
            provider_id: 7,
            provider_name: "backup".to_string(),
            policy_source: "provider",
            source_model: "fable5".to_string(),
            target_model: None,
            reasoning_effort: Some("low".to_string()),
        };
        let mut path = "/v1/messages".to_string();
        let mut query = None;
        let outcome = apply(&route, "claude", &mut path, &mut query, b"not-json");

        assert!(outcome.body.is_none());
        assert!(!outcome.applied());
    }
}
