//! Complete, immutable response validation before the gateway commits an attempt.

use super::context::GatewayPluginHookName;
use super::permissions::GatewayPluginError;
use super::pipeline::{
    attach_plugin_diagnostics, audit_event, duration_ms_i64, plugin_hook, GatewayPluginAuditEvent,
    GatewayPluginHookExecutionReport, GatewayPluginPipeline, HookReportOutcome,
};
use crate::domain::plugins::PluginDetail;
use crate::shared::time::now_unix_millis;
use axum::body::Bytes;
use axum::http::{HeaderMap, Method};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::time::Instant;

pub(crate) const MAX_COMPLETE_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const HOOK: GatewayPluginHookName = GatewayPluginHookName::ResponseBeforeCommit;

pub(crate) type GatewayCommitFuture =
    Pin<Box<dyn Future<Output = Result<GatewayCommitResult, GatewayPluginError>> + Send>>;

#[derive(Debug, Clone)]
pub(crate) struct GatewayResponseCommitPlan {
    plugins: Vec<PluginDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GatewayCommitRequest {
    pub(crate) cli_key: String,
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) requested_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GatewayCommitOutboundRequest {
    pub(crate) model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GatewayCommitAttempt {
    pub(crate) provider_id: i64,
    pub(crate) provider_index: usize,
    pub(crate) retry_index: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct GatewayResponseCommitInput {
    pub(crate) trace_id: String,
    pub(crate) request: GatewayCommitRequest,
    pub(crate) outbound_request: GatewayCommitOutboundRequest,
    pub(crate) attempt: GatewayCommitAttempt,
    pub(crate) status: u16,
    pub(crate) headers: HeaderMap,
    pub(crate) body: Bytes,
    pub(crate) execution_lease: Option<std::sync::Arc<tokio::sync::OwnedSemaphorePermit>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GatewayCommitContext {
    pub(crate) hook_name: String,
    pub(crate) trace_id: String,
    pub(crate) request: GatewayCommitRequest,
    pub(crate) outbound_request: GatewayCommitOutboundRequest,
    pub(crate) attempt: GatewayCommitAttempt,
    pub(crate) response: GatewayCommitResponse,
    #[serde(skip)]
    pub(crate) execution_lease: Option<std::sync::Arc<tokio::sync::OwnedSemaphorePermit>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GatewayCommitResponse {
    pub(crate) status: u16,
    pub(crate) headers: BTreeMap<String, String>,
    pub(crate) content_type: Option<String>,
    pub(crate) body: String,
    pub(crate) complete: bool,
    pub(crate) decoded_bytes: usize,
}

impl GatewayResponseCommitInput {
    pub(crate) fn into_context(self) -> Result<GatewayCommitContext, GatewayPluginError> {
        if self.body.len() > MAX_COMPLETE_RESPONSE_BYTES {
            return Err(GatewayPluginError::new(
                "PLUGIN_COMMIT_RESPONSE_TOO_LARGE",
                "complete response exceeds the validation budget",
            ));
        }
        let body = std::str::from_utf8(&self.body)
            .map_err(|_| {
                GatewayPluginError::new(
                    "PLUGIN_COMMIT_INVALID_UTF8",
                    "complete response must be valid UTF-8",
                )
            })?
            .to_owned();
        let headers = ["content-type", "content-language", "cache-control"]
            .into_iter()
            .filter_map(|name| {
                self.headers
                    .get(name)
                    .and_then(|value| value.to_str().ok())
                    .map(|value| (name.to_owned(), value.to_owned()))
            })
            .collect::<BTreeMap<_, _>>();
        Ok(GatewayCommitContext {
            execution_lease: self.execution_lease,
            hook_name: HOOK.as_str().to_owned(),
            trace_id: self.trace_id,
            request: self.request,
            outbound_request: self.outbound_request,
            attempt: self.attempt,
            response: GatewayCommitResponse {
                status: self.status,
                content_type: headers.get("content-type").cloned(),
                headers,
                body,
                complete: true,
                decoded_bytes: self.body.len(),
            },
        })
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(tag = "action", deny_unknown_fields)]
pub(crate) enum GatewayCommitResult {
    #[serde(rename = "pass")]
    Pass,
    #[serde(rename = "block")]
    Block {
        #[serde(rename = "reasonCode")]
        reason_code: String,
        message: Option<String>,
    },
    #[serde(rename = "switchProvider")]
    SwitchProvider {
        #[serde(rename = "reasonCode")]
        reason_code: String,
        message: Option<String>,
    },
}

impl GatewayCommitResult {
    pub(crate) fn from_value(value: serde_json::Value) -> Result<Self, GatewayPluginError> {
        // serde's unit variant accepts unknown fields; validate pass explicitly too.
        if value.get("action").and_then(serde_json::Value::as_str) == Some("pass")
            && value.as_object().is_some_and(|object| object.len() != 1)
        {
            return Err(invalid_result());
        }
        let result: Self = serde_json::from_value(value).map_err(|_| invalid_result())?;
        result.validate()?;
        Ok(result)
    }

    fn validate(&self) -> Result<(), GatewayPluginError> {
        let (Self::Block {
            reason_code,
            message,
        }
        | Self::SwitchProvider {
            reason_code,
            message,
        }) = self
        else {
            return Ok(());
        };
        if reason_code.is_empty()
            || reason_code.len() > 96
            || !reason_code
                .bytes()
                .all(|ch| ch.is_ascii_alphanumeric() || b"_.-".contains(&ch))
            || message
                .as_ref()
                .is_some_and(|message| message.len() > 256 || message.chars().any(char::is_control))
        {
            return Err(invalid_result());
        }
        Ok(())
    }
}

fn invalid_result() -> GatewayPluginError {
    GatewayPluginError::new("PLUGIN_COMMIT_INVALID_OUTPUT", "beforeCommit requires pass, block or switchProvider with a bounded reasonCode and optional message")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GatewayCommitRejection {
    pub(crate) plugin_id: String,
    pub(crate) reason_code: String,
    pub(crate) message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GatewayCommitDecision {
    Pass,
    Block(GatewayCommitRejection),
    SwitchProvider(GatewayCommitRejection),
}

#[derive(Debug, Clone)]
pub(crate) struct GatewayResponseCommitOutput {
    pub(crate) decision: GatewayCommitDecision,
    pub(crate) audit_events: Vec<GatewayPluginAuditEvent>,
    pub(crate) execution_reports: Vec<GatewayPluginHookExecutionReport>,
}

impl GatewayPluginPipeline {
    pub(crate) fn freeze_response_commit_plan(
        &self,
        cli_key: &str,
        method: &Method,
        path: &str,
    ) -> Result<Option<GatewayResponseCommitPlan>, GatewayPluginError> {
        let plugins = self
            .plugins_for_hook(HOOK)
            .iter()
            .filter(|plugin| {
                plugin_hook(plugin, HOOK)
                    .and_then(|hook| hook.request_match.as_ref())
                    .is_some_and(|scope| {
                        scope.cli_keys.iter().any(|value| value == cli_key)
                            && scope.methods.iter().any(|value| value == method.as_str())
                            && scope.paths.iter().any(|value| value == path)
                    })
            })
            .cloned()
            .collect::<Vec<_>>();
        if plugins.is_empty() {
            return Ok(None);
        }
        for plugin in &plugins {
            self.ensure_commit_participant(plugin)?;
            if self.commit_circuit_unavailable(&plugin.summary.plugin_id) {
                return Err(GatewayPluginError::new(
                    "PLUGIN_COMMIT_UNAVAILABLE",
                    "required response validation circuit is open",
                ));
            }
        }
        Ok(Some(GatewayResponseCommitPlan { plugins }))
    }

    fn ensure_commit_participant(&self, frozen: &PluginDetail) -> Result<(), GatewayPluginError> {
        let available = self.plugins_for_hook(HOOK).iter().any(|current| {
            current.summary.plugin_id == frozen.summary.plugin_id
                && current.manifest == frozen.manifest
                && current.installed_dir == frozen.installed_dir
                && current
                    .manifest
                    .capabilities
                    .iter()
                    .any(|cap| cap == "gateway.hooks")
        });
        if available {
            Ok(())
        } else {
            Err(GatewayPluginError::new(
                "PLUGIN_COMMIT_UNAVAILABLE",
                "required response validation plugin was disabled or replaced",
            ))
        }
    }

    pub(crate) async fn run_response_commit_hook(
        &self,
        plan: &GatewayResponseCommitPlan,
        input: GatewayResponseCommitInput,
        deadline: Instant,
    ) -> Result<GatewayResponseCommitOutput, GatewayPluginError> {
        let context = input.into_context()?;
        let mut output = GatewayResponseCommitOutput {
            decision: GatewayCommitDecision::Pass,
            audit_events: vec![],
            execution_reports: vec![],
        };
        for plugin in &plan.plugins {
            let started = Instant::now();
            let started_at_ms = now_unix_millis();
            let mut executor_called = false;
            let mut circuit_lease = None;
            let result = async {
                self.ensure_commit_participant(plugin)?;
                circuit_lease = self.acquire_circuit(&plugin.summary.plugin_id);
                if circuit_lease.is_none() {
                    return Err(GatewayPluginError::new(
                        "PLUGIN_COMMIT_UNAVAILABLE",
                        "required response validation circuit is open",
                    ));
                }
                let remaining = deadline
                    .checked_duration_since(Instant::now())
                    .filter(|value| !value.is_zero())
                    .ok_or_else(|| {
                        GatewayPluginError::new(
                            "PLUGIN_HOOK_TIMEOUT",
                            "response validation deadline expired",
                        )
                    })?;
                executor_called = true;
                let result = self
                    .executor
                    .execute_response_commit_hook(
                        plugin,
                        context.clone(),
                        remaining.min(self.hook_timeout(plugin, HOOK)),
                    )
                    .await?;
                result.validate()?;
                if matches!(result, GatewayCommitResult::SwitchProvider { .. })
                    && !plugin
                        .manifest
                        .capabilities
                        .iter()
                        .any(|cap| cap == "gateway.provider.switch")
                {
                    return Err(GatewayPluginError::new(
                        "PLUGIN_PERMISSION_DENIED",
                        "switchProvider requires gateway.provider.switch",
                    ));
                }
                Ok(result)
            }
            .await;
            let result = match result {
                Ok(result) => result,
                Err(error) => {
                    if executor_called {
                        self.record_failure(&plugin.summary.plugin_id);
                    }
                    output.audit_events.push(audit_event(
                        plugin,
                        HOOK,
                        "plugin.hook.failed",
                        "high",
                        "Required response validation failed",
                        serde_json::json!({"errorCode": error.code_for_logging()}),
                    ));
                    output.execution_reports.push(self.commit_report(
                        plugin,
                        &context.trace_id,
                        started,
                        started_at_ms,
                        Err(error.code_for_logging()),
                        serde_json::json!({"changed":false}),
                    ));
                    return Err(attach_plugin_diagnostics(
                        error,
                        output.audit_events,
                        output.execution_reports,
                    ));
                }
            };
            self.record_success(&plugin.summary.plugin_id);
            let (status, reason_code, message) = match result {
                GatewayCommitResult::Pass => ("completed", None, None),
                GatewayCommitResult::Block {
                    reason_code,
                    message,
                } => ("blocked", Some(reason_code), message),
                GatewayCommitResult::SwitchProvider {
                    reason_code,
                    message,
                } => ("switchProvider", Some(reason_code), message),
            };
            // Store identifiers only. A third-party message can contain response secrets.
            let details = serde_json::json!({"decision":status,"reasonCode":reason_code,"providerId":context.attempt.provider_id,"providerIndex":context.attempt.provider_index,"retryIndex":context.attempt.retry_index});
            output.audit_events.push(audit_event(
                plugin,
                HOOK,
                "plugin.hook.completed",
                if status == "completed" {
                    "low"
                } else {
                    "medium"
                },
                "Required response validation completed",
                details.clone(),
            ));
            output.execution_reports.push(self.commit_report(
                plugin,
                &context.trace_id,
                started,
                started_at_ms,
                Ok(status),
                details,
            ));
            if let Some(reason_code) = reason_code {
                let rejection = GatewayCommitRejection {
                    plugin_id: plugin.summary.plugin_id.clone(),
                    reason_code,
                    message,
                };
                if status == "blocked" {
                    output.decision = GatewayCommitDecision::Block(rejection);
                    return Ok(output);
                }
                if matches!(output.decision, GatewayCommitDecision::Pass) {
                    output.decision = GatewayCommitDecision::SwitchProvider(rejection);
                }
            }
        }
        Ok(output)
    }

    fn commit_report(
        &self,
        plugin: &PluginDetail,
        trace_id: &str,
        started: Instant,
        started_at_ms: i64,
        outcome: Result<&'static str, &'static str>,
        details: serde_json::Value,
    ) -> GatewayPluginHookExecutionReport {
        let (status, error_code) = match outcome {
            Ok(status) => (status, None),
            Err(error_code) => ("failedClosed", Some(error_code)),
        };
        let mut report = self.hook_execution_report(
            plugin,
            HOOK,
            trace_id,
            HookReportOutcome {
                started_at_ms,
                duration_ms: duration_ms_i64(started),
                status,
                failure_kind: error_code.map(|_| "required_validation"),
                error_code,
                mutation_summary: details,
                replayable: false,
                replay_export_reason: Some("complete response contents are not retained"),
            },
        );
        report.context_budget = serde_json::json!({"bodyBytes":MAX_COMPLETE_RESPONSE_BYTES});
        report.output_budget = serde_json::json!({"reasonCodeBytes":96,"messageBytes":256});
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::plugins::PluginHookMatch;
    use crate::gateway::plugins::pipeline::{
        GatewayPluginPipelineConfig, InMemoryGatewayPluginExecutor,
    };
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    fn plugin(id: &str, priority: i32) -> PluginDetail {
        let mut plugin = super::super::pipeline::tests::plugin(id, priority, vec![]);
        let hook = &mut plugin.manifest.contributes.as_mut().unwrap().gateway_hooks[0];
        hook.name = HOOK.as_str().to_owned();
        hook.failure_policy = None;
        hook.request_match = Some(PluginHookMatch {
            cli_keys: vec!["codex".into()],
            methods: vec!["POST".into()],
            paths: vec!["/responses".into(), "/v1/responses".into()],
        });
        plugin
            .manifest
            .capabilities
            .push("gateway.provider.switch".into());
        plugin
    }

    pub(super) fn input() -> GatewayResponseCommitInput {
        GatewayResponseCommitInput {
            trace_id: "trace-commit".into(),
            request: GatewayCommitRequest {
                cli_key: "codex".into(),
                method: "POST".into(),
                path: "/responses".into(),
                requested_model: Some("original".into()),
            },
            outbound_request: GatewayCommitOutboundRequest {
                model: Some("mapped".into()),
            },
            attempt: GatewayCommitAttempt {
                provider_id: 42,
                provider_index: 0,
                retry_index: 0,
            },
            status: 200,
            headers: HeaderMap::new(),
            body: Bytes::from_static(b"tail-evidence"),
            execution_lease: None,
        }
    }

    #[test]
    fn complete_response_budget_matches_public_contract() {
        let contract: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../docs/plugins/plugin-api-v1-contract.json"
        ))
        .unwrap();
        assert_eq!(
            contract["responseCommit"]["maxDecodedBytes"].as_u64(),
            Some(MAX_COMPLETE_RESPONSE_BYTES as u64)
        );
    }

    #[test]
    fn complete_context_rejects_truncation_and_projects_only_safe_headers() {
        let mut value = input();
        value
            .headers
            .insert("authorization", "Bearer secret".parse().unwrap());
        value
            .headers
            .insert("set-cookie", "secret=1".parse().unwrap());
        value
            .headers
            .insert("x-response-id", "secret".parse().unwrap());
        value
            .headers
            .insert("content-type", "text/event-stream".parse().unwrap());
        let context = value.clone().into_context().unwrap();
        let json = serde_json::to_value(context).unwrap();
        assert_eq!(json["request"]["cliKey"], "codex");
        assert_eq!(json["outboundRequest"]["model"], "mapped");
        assert_eq!(json["response"]["complete"], true);
        assert_eq!(json["response"]["decodedBytes"], 13);
        assert_eq!(json["response"]["headers"].as_object().unwrap().len(), 1);
        assert!(json["request"].get("body").is_none());
        value.body = Bytes::from(vec![b'x'; MAX_COMPLETE_RESPONSE_BYTES + 1]);
        assert_eq!(
            value.clone().into_context().unwrap_err().code(),
            "PLUGIN_COMMIT_RESPONSE_TOO_LARGE"
        );
        value.body = Bytes::from_static(&[0xff]);
        assert_eq!(
            value.into_context().unwrap_err().code(),
            "PLUGIN_COMMIT_INVALID_UTF8"
        );
    }

    #[test]
    fn commit_results_reject_mutations_unknown_actions_and_unbounded_diagnostics() {
        for result in [
            serde_json::json!({"action":"replace","responseBody":"bad"}),
            serde_json::json!({"action":"warn","message":"bad"}),
            serde_json::json!({"action":"pass","responseBody":"bad"}),
            serde_json::json!({"action":"switchProvider","reasonCode":"ok","providerId":2}),
            serde_json::json!({"action":"switchProvider","reasonCode":"bad reason"}),
            serde_json::json!({"action":"block","reasonCode":"ok","message":"x".repeat(257)}),
        ] {
            assert!(GatewayCommitResult::from_value(result).is_err());
        }
        assert_eq!(
            GatewayCommitResult::from_value(serde_json::json!({"action":"pass"})).unwrap(),
            GatewayCommitResult::Pass
        );
    }

    #[tokio::test]
    async fn commit_switch_continues_validation_and_block_or_failure_wins() {
        for (second, expected) in [("pass", "switch"), ("block", "block"), ("error", "error")] {
            let seen = Arc::new(Mutex::new(Vec::new()));
            let first_seen = seen.clone();
            let second_seen = seen.clone();
            let executor = InMemoryGatewayPluginExecutor::new()
                .with_response_commit_handler("one", move |_, context| {
                    first_seen.lock().unwrap().push(context.response.body);
                    Ok(GatewayCommitResult::SwitchProvider {
                        reason_code: "tail.rejected".into(),
                        message: None,
                    })
                })
                .with_response_commit_handler("two", move |_, context| {
                    second_seen.lock().unwrap().push(context.response.body);
                    match second {
                        "block" => Ok(GatewayCommitResult::Block {
                            reason_code: "policy.block".into(),
                            message: None,
                        }),
                        "error" => Err(invalid_result()),
                        _ => Ok(GatewayCommitResult::Pass),
                    }
                });
            let pipeline = GatewayPluginPipeline::for_tests(
                vec![plugin("one", 0), plugin("two", 1)],
                Arc::new(executor),
                GatewayPluginPipelineConfig::default(),
            );
            let plan = pipeline
                .freeze_response_commit_plan("codex", &Method::POST, "/responses")
                .unwrap()
                .unwrap();
            let result = pipeline
                .run_response_commit_hook(&plan, input(), Instant::now() + Duration::from_secs(1))
                .await;
            match expected {
                "block" => assert!(matches!(
                    result.unwrap().decision,
                    GatewayCommitDecision::Block(_)
                )),
                "error" => assert!(result.is_err()),
                _ => assert!(matches!(
                    result.unwrap().decision,
                    GatewayCommitDecision::SwitchProvider(_)
                )),
            }
            assert_eq!(
                *seen.lock().unwrap(),
                vec!["tail-evidence", "tail-evidence"]
            );
            assert_eq!(pipeline.circuit_snapshot("one").failure_count, 0);
            if second != "error" {
                assert_eq!(pipeline.circuit_snapshot("two").failure_count, 0);
            }
        }
    }

    #[tokio::test]
    async fn commit_snapshot_preserves_config_but_disabled_or_updated_plugins_fail_closed() {
        let mut original = plugin("one", 0);
        original.config = serde_json::json!({"frozen":true});
        let executor = InMemoryGatewayPluginExecutor::new().with_response_commit_handler(
            "one",
            |detail, _| {
                assert_eq!(detail.config["frozen"], true);
                Ok(GatewayCommitResult::Pass)
            },
        );
        let pipeline = GatewayPluginPipeline::for_tests(
            vec![original.clone()],
            Arc::new(executor),
            GatewayPluginPipelineConfig::default(),
        );
        assert!(pipeline
            .freeze_response_commit_plan("claude", &Method::POST, "/responses")
            .unwrap()
            .is_none());
        assert!(pipeline
            .freeze_response_commit_plan("codex", &Method::GET, "/responses")
            .unwrap()
            .is_none());
        assert!(pipeline
            .freeze_response_commit_plan("codex", &Method::POST, "/responses/compact")
            .unwrap()
            .is_none());
        let plan = pipeline
            .freeze_response_commit_plan("codex", &Method::POST, "/responses")
            .unwrap()
            .unwrap();
        let mut changed = original.clone();
        changed.config = serde_json::json!({"frozen":false});
        pipeline.replace_plugins(vec![changed]);
        assert!(matches!(
            pipeline
                .run_response_commit_hook(&plan, input(), Instant::now() + Duration::from_secs(1))
                .await
                .unwrap()
                .decision,
            GatewayCommitDecision::Pass
        ));
        let mut updated = original.clone();
        updated.manifest.version = "2.0.0".into();
        pipeline.replace_plugins(vec![updated]);
        assert_eq!(
            pipeline
                .run_response_commit_hook(&plan, input(), Instant::now() + Duration::from_secs(1))
                .await
                .unwrap_err()
                .code(),
            "PLUGIN_COMMIT_UNAVAILABLE"
        );
        pipeline.replace_plugins(vec![]);
        assert!(pipeline
            .run_response_commit_hook(&plan, input(), Instant::now() + Duration::from_secs(1))
            .await
            .is_err());
        pipeline.replace_plugins(vec![original]);
        pipeline.force_open_circuit_for_tests("one");
        assert!(pipeline
            .freeze_response_commit_plan("codex", &Method::POST, "/responses")
            .is_err());
        assert_eq!(
            pipeline
                .run_response_commit_hook(&plan, input(), Instant::now() + Duration::from_secs(1))
                .await
                .unwrap_err()
                .code(),
            "PLUGIN_COMMIT_UNAVAILABLE"
        );
    }

    #[tokio::test]
    async fn commit_requires_switch_capability_and_enforces_deadline_before_execution() {
        let mut detail = plugin("one", 0);
        detail
            .manifest
            .capabilities
            .retain(|cap| cap != "gateway.provider.switch");
        let executor =
            InMemoryGatewayPluginExecutor::new().with_response_commit_handler("one", |_, _| {
                Ok(GatewayCommitResult::SwitchProvider {
                    reason_code: "reject".into(),
                    message: None,
                })
            });
        let pipeline = GatewayPluginPipeline::for_tests(
            vec![detail],
            Arc::new(executor),
            GatewayPluginPipelineConfig::default(),
        );
        let plan = pipeline
            .freeze_response_commit_plan("codex", &Method::POST, "/v1/responses")
            .unwrap()
            .unwrap();
        assert_eq!(
            pipeline
                .run_response_commit_hook(&plan, input(), Instant::now())
                .await
                .unwrap_err()
                .code(),
            "PLUGIN_HOOK_TIMEOUT"
        );
        assert_eq!(
            pipeline
                .run_response_commit_hook(&plan, input(), Instant::now() + Duration::from_secs(1))
                .await
                .unwrap_err()
                .code(),
            "PLUGIN_PERMISSION_DENIED"
        );
    }
    #[tokio::test]
    async fn commit_expired_deadline_releases_half_open_probe() {
        let executor = InMemoryGatewayPluginExecutor::new()
            .with_response_commit_handler("one", |_, _| Ok(GatewayCommitResult::Pass));
        let pipeline = GatewayPluginPipeline::for_tests(
            vec![plugin("one", 0)],
            Arc::new(executor),
            GatewayPluginPipelineConfig {
                circuit_cooldown: Duration::ZERO,
                ..GatewayPluginPipelineConfig::default()
            },
        );
        pipeline.force_open_circuit_for_tests("one");
        let plan = pipeline
            .freeze_response_commit_plan("codex", &Method::POST, "/responses")
            .unwrap()
            .unwrap();
        let failures_before = pipeline.circuit_snapshot("one").failure_count;
        let error = pipeline
            .run_response_commit_hook(&plan, input(), Instant::now())
            .await
            .unwrap_err();
        assert_eq!(error.code(), "PLUGIN_HOOK_TIMEOUT");
        assert!(!pipeline.circuit_snapshot("one").half_open);
        assert_eq!(
            pipeline.circuit_snapshot("one").failure_count,
            failures_before
        );
        let next_plan = pipeline
            .freeze_response_commit_plan("codex", &Method::POST, "/responses")
            .unwrap()
            .unwrap();
        assert_eq!(
            pipeline
                .run_response_commit_hook(
                    &next_plan,
                    input(),
                    Instant::now() + Duration::from_secs(1)
                )
                .await
                .unwrap()
                .decision,
            GatewayCommitDecision::Pass,
        );
    }

    #[tokio::test]
    async fn commit_cancelled_half_open_probe_allows_next_request() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let started = Arc::new(tokio::sync::Notify::new());
        let calls = Arc::new(AtomicUsize::new(0));
        let executor =
            InMemoryGatewayPluginExecutor::new().with_response_commit_async_handler("one", {
                let started = started.clone();
                let calls = calls.clone();
                move |_| {
                    let started = started.clone();
                    let first = calls.fetch_add(1, Ordering::SeqCst) == 0;
                    async move {
                        if first {
                            started.notify_one();
                            std::future::pending::<()>().await;
                        }
                        Ok(GatewayCommitResult::Pass)
                    }
                }
            });
        let pipeline = GatewayPluginPipeline::for_tests_shared(
            vec![plugin("one", 0)],
            Arc::new(executor),
            GatewayPluginPipelineConfig {
                circuit_cooldown: Duration::ZERO,
                ..GatewayPluginPipelineConfig::default()
            },
        );
        pipeline.force_open_circuit_for_tests("one");
        let plan = pipeline
            .freeze_response_commit_plan("codex", &Method::POST, "/responses")
            .unwrap()
            .unwrap();
        let task_pipeline = pipeline.clone();
        let task = tokio::spawn(async move {
            task_pipeline
                .run_response_commit_hook(&plan, input(), Instant::now() + Duration::from_secs(5))
                .await
        });
        tokio::time::timeout(Duration::from_secs(1), started.notified())
            .await
            .unwrap();
        assert!(pipeline.circuit_snapshot("one").half_open);
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert!(!pipeline.circuit_snapshot("one").half_open);
        let next_plan = pipeline
            .freeze_response_commit_plan("codex", &Method::POST, "/responses")
            .unwrap()
            .unwrap();
        assert_eq!(
            pipeline
                .run_response_commit_hook(
                    &next_plan,
                    input(),
                    Instant::now() + Duration::from_secs(1)
                )
                .await
                .unwrap()
                .decision,
            GatewayCommitDecision::Pass,
        );
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }
}
