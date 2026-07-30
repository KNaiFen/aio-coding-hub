//! Middleware and fail-open observation helpers for Codex Responses request
//! metadata.

use super::{MiddlewareAction, ProxyContext};
use crate::gateway::proxy::handler::early_error::push_special_setting;
use axum::http::{HeaderMap, Method};
use serde::Serialize;
use serde_json::{Map, Value};

const CODEX_TURN_METADATA_KEY: &str = "x-codex-turn-metadata";
const MAX_CODEX_TURN_METADATA_BYTES: usize = 16 * 1024;
const CODEX_SYSTEM_REQUEST_SETTING_TYPE: &str = "codex_system_request";
const CODEX_SYSTEM_REQUEST_THREAD_SOURCE: &str = "system";
const CODEX_CONTEXT_COMPACTION_SETTING_TYPE: &str = "codex_context_compaction";
const UNKNOWN_VALUE: &str = "unknown";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(in crate::gateway::proxy::handler) struct CodexContextCompactionMarker {
    #[serde(rename = "type")]
    marker_type: &'static str,
    mode: &'static str,
    implementation: &'static str,
    trigger: &'static str,
    reason: &'static str,
    phase: &'static str,
    strategy: &'static str,
}

impl CodexContextCompactionMarker {
    fn from_metadata(metadata: &Map<String, Value>) -> Self {
        let compaction = metadata.get("compaction").and_then(Value::as_object);
        let implementation = normalized_value(
            compaction,
            "implementation",
            &["responses", "responses_compact", "responses_compaction_v2"],
        );
        let mode = match implementation {
            "responses" => "local",
            "responses_compact" | "responses_compaction_v2" => "remote",
            _ => UNKNOWN_VALUE,
        };

        Self {
            marker_type: CODEX_CONTEXT_COMPACTION_SETTING_TYPE,
            mode,
            implementation,
            trigger: normalized_value(compaction, "trigger", &["manual", "auto"]),
            reason: normalized_value(
                compaction,
                "reason",
                &[
                    "user_requested",
                    "context_limit",
                    "model_downshift",
                    "comp_hash_changed",
                ],
            ),
            phase: normalized_value(
                compaction,
                "phase",
                &["standalone_turn", "pre_turn", "mid_turn"],
            ),
            strategy: normalized_value(compaction, "strategy", &["memento", "prefix_compaction"]),
        }
    }

    fn protocol_fallback(implementation: &'static str) -> Self {
        Self {
            marker_type: CODEX_CONTEXT_COMPACTION_SETTING_TYPE,
            mode: "remote",
            implementation,
            trigger: UNKNOWN_VALUE,
            reason: UNKNOWN_VALUE,
            phase: UNKNOWN_VALUE,
            strategy: UNKNOWN_VALUE,
        }
    }
}

pub(in crate::gateway::proxy::handler) struct CodexRequestClassifierMiddleware;

impl CodexRequestClassifierMiddleware {
    pub(in crate::gateway::proxy::handler) fn run<R: tauri::Runtime>(
        mut ctx: ProxyContext<R>,
    ) -> MiddlewareAction<R> {
        if let Some(setting) = codex_system_request_special_setting(
            &ctx.cli_key,
            &ctx.req_method,
            &ctx.forwarded_path,
            ctx.introspection_json.as_ref(),
        ) {
            push_special_setting(&ctx.special_settings, setting);
            ctx.provider_health_neutral = true;
        }

        MiddlewareAction::Continue(Box::new(ctx))
    }
}

pub(in crate::gateway::proxy::handler) fn classify_codex_context_compaction(
    cli_key: &str,
    method: &Method,
    forwarded_path: &str,
    headers: &HeaderMap,
    introspection_json: Option<&Value>,
) -> Option<CodexContextCompactionMarker> {
    if cli_key != "codex" || *method != Method::POST || !is_codex_responses_path(forwarded_path) {
        return None;
    }

    if let Some(metadata) = usable_body_turn_metadata(introspection_json) {
        return marker_from_explicit_metadata(&metadata);
    }
    if let Some(metadata) = usable_header_turn_metadata(headers) {
        return marker_from_explicit_metadata(&metadata);
    }

    if is_responses_compact_path(forwarded_path) {
        return Some(CodexContextCompactionMarker::protocol_fallback(
            "responses_compact",
        ));
    }
    top_level_input_has_compaction_trigger(introspection_json)
        .then(|| CodexContextCompactionMarker::protocol_fallback("responses_compaction_v2"))
}

fn marker_from_explicit_metadata(
    metadata: &Map<String, Value>,
) -> Option<CodexContextCompactionMarker> {
    (metadata.get("request_kind").and_then(Value::as_str) == Some("compaction"))
        .then(|| CodexContextCompactionMarker::from_metadata(metadata))
}

fn usable_body_turn_metadata(introspection_json: Option<&Value>) -> Option<Map<String, Value>> {
    let raw = introspection_json?
        .get("client_metadata")?
        .as_object()?
        .get(CODEX_TURN_METADATA_KEY)?
        .as_str()?;
    parse_usable_turn_metadata(raw)
}

fn usable_header_turn_metadata(headers: &HeaderMap) -> Option<Map<String, Value>> {
    let all_values = headers.get_all(CODEX_TURN_METADATA_KEY);
    let mut values = all_values.iter();
    let value = values.next()?;
    if values.next().is_some() {
        return None;
    }
    value.to_str().ok().and_then(parse_usable_turn_metadata)
}

fn parse_usable_turn_metadata(raw: &str) -> Option<Map<String, Value>> {
    if raw.is_empty() || raw.len() > MAX_CODEX_TURN_METADATA_BYTES {
        return None;
    }
    let metadata = serde_json::from_str::<Value>(raw)
        .ok()?
        .as_object()?
        .clone();
    metadata.get("request_kind")?.as_str()?;
    Some(metadata)
}

fn normalized_value(
    values: Option<&Map<String, Value>>,
    key: &str,
    allowed: &[&'static str],
) -> &'static str {
    let Some(raw) = values
        .and_then(|values| values.get(key))
        .and_then(Value::as_str)
    else {
        return UNKNOWN_VALUE;
    };
    allowed
        .iter()
        .copied()
        .find(|candidate| *candidate == raw)
        .unwrap_or(UNKNOWN_VALUE)
}

fn is_codex_responses_path(forwarded_path: &str) -> bool {
    let segments = terminal_path_segments(forwarded_path);
    segments.ends_with(&["responses"]) || segments.ends_with(&["responses", "compact"])
}

fn is_responses_compact_path(forwarded_path: &str) -> bool {
    terminal_path_segments(forwarded_path).ends_with(&["responses", "compact"])
}

fn terminal_path_segments(forwarded_path: &str) -> Vec<&str> {
    forwarded_path
        .split('?')
        .next()
        .unwrap_or(forwarded_path)
        .trim_end_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn top_level_input_has_compaction_trigger(introspection_json: Option<&Value>) -> bool {
    introspection_json
        .and_then(|body| body.get("input"))
        .and_then(Value::as_array)
        .is_some_and(|items| {
            items
                .iter()
                .any(|item| item.get("type").and_then(Value::as_str) == Some("compaction_trigger"))
        })
}

fn codex_system_request_special_setting(
    cli_key: &str,
    method: &Method,
    forwarded_path: &str,
    introspection_json: Option<&Value>,
) -> Option<Value> {
    if cli_key != "codex"
        || *method != Method::POST
        || !matches!(
            forwarded_path,
            "/responses" | "/responses/" | "/v1/responses" | "/v1/responses/"
        )
    {
        return None;
    }

    let turn_metadata = introspection_json?
        .get("client_metadata")?
        .as_object()?
        .get(CODEX_TURN_METADATA_KEY)?
        .as_str()?;
    if turn_metadata.is_empty() || turn_metadata.len() > MAX_CODEX_TURN_METADATA_BYTES {
        return None;
    }

    let turn_metadata = serde_json::from_str::<Value>(turn_metadata).ok()?;
    let turn_metadata = turn_metadata.as_object()?;
    (turn_metadata.get("thread_source").and_then(Value::as_str)
        == Some(CODEX_SYSTEM_REQUEST_THREAD_SOURCE))
    .then(|| {
        serde_json::json!({
            "type": CODEX_SYSTEM_REQUEST_SETTING_TYPE,
            "threadSource": CODEX_SYSTEM_REQUEST_THREAD_SOURCE,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Map};

    fn compaction_metadata(
        implementation: &str,
        trigger: &str,
        reason: &str,
        phase: &str,
        strategy: &str,
    ) -> String {
        json!({
            "request_kind": "compaction",
            "compaction": {
                "trigger": trigger,
                "reason": reason,
                "implementation": implementation,
                "phase": phase,
                "strategy": strategy,
            }
        })
        .to_string()
    }

    fn compaction_request_body(turn_metadata: Value, input: Value) -> Value {
        let mut client_metadata = Map::new();
        client_metadata.insert(CODEX_TURN_METADATA_KEY.to_string(), turn_metadata);
        json!({
            "model": "gpt-5.4-mini",
            "client_metadata": client_metadata,
            "input": input,
        })
    }

    fn compaction_marker(
        body: Option<&Value>,
        header: Option<&str>,
        path: &str,
    ) -> Option<CodexContextCompactionMarker> {
        let mut headers = HeaderMap::new();
        if let Some(header) = header {
            headers.insert(
                CODEX_TURN_METADATA_KEY,
                header.parse().expect("valid test header"),
            );
        }
        classify_codex_context_compaction("codex", &Method::POST, path, &headers, body)
    }

    fn marker_json(marker: Option<CodexContextCompactionMarker>) -> Option<Value> {
        marker.map(|marker| serde_json::to_value(marker).expect("serialize marker"))
    }

    fn request_body_with_turn_metadata(value: Value) -> Value {
        let mut client_metadata = Map::new();
        client_metadata.insert(CODEX_TURN_METADATA_KEY.to_string(), value);
        json!({
            "model": "gpt-5.4-mini",
            "client_metadata": client_metadata,
        })
    }

    fn classify(body: Option<&Value>) -> Option<Value> {
        codex_system_request_special_setting("codex", &Method::POST, "/v1/responses", body)
    }

    fn system_turn_metadata() -> String {
        json!({ "thread_source": "system" }).to_string()
    }

    #[test]
    fn classifies_all_known_explicit_compaction_implementations() {
        for (implementation, mode) in [
            ("responses", "local"),
            ("responses_compact", "remote"),
            ("responses_compaction_v2", "remote"),
        ] {
            let body = compaction_request_body(
                Value::String(compaction_metadata(
                    implementation,
                    "manual",
                    "user_requested",
                    "standalone_turn",
                    "memento",
                )),
                json!([]),
            );

            assert_eq!(
                marker_json(compaction_marker(Some(&body), None, "/v1/responses")),
                Some(json!({
                    "type": CODEX_CONTEXT_COMPACTION_SETTING_TYPE,
                    "mode": mode,
                    "implementation": implementation,
                    "trigger": "manual",
                    "reason": "user_requested",
                    "phase": "standalone_turn",
                    "strategy": "memento",
                }))
            );
        }
    }

    #[test]
    fn normalizes_untrusted_compaction_values_to_fixed_unknowns() {
        let body = compaction_request_body(
            Value::String(compaction_metadata(
                "future_impl",
                "scheduled",
                "future_reason",
                "future_phase",
                "future_strategy",
            )),
            json!([]),
        );

        assert_eq!(
            marker_json(compaction_marker(Some(&body), None, "/v1/responses")),
            Some(json!({
                "type": CODEX_CONTEXT_COMPACTION_SETTING_TYPE,
                "mode": UNKNOWN_VALUE,
                "implementation": UNKNOWN_VALUE,
                "trigger": UNKNOWN_VALUE,
                "reason": UNKNOWN_VALUE,
                "phase": UNKNOWN_VALUE,
                "strategy": UNKNOWN_VALUE,
            }))
        );
    }

    #[test]
    fn compaction_request_kind_with_missing_details_is_unknown() {
        let body = compaction_request_body(
            Value::String(json!({ "request_kind": "compaction" }).to_string()),
            json!([]),
        );

        assert_eq!(
            marker_json(compaction_marker(Some(&body), None, "/v1/responses")),
            Some(json!({
                "type": CODEX_CONTEXT_COMPACTION_SETTING_TYPE,
                "mode": UNKNOWN_VALUE,
                "implementation": UNKNOWN_VALUE,
                "trigger": UNKNOWN_VALUE,
                "reason": UNKNOWN_VALUE,
                "phase": UNKNOWN_VALUE,
                "strategy": UNKNOWN_VALUE,
            }))
        );
    }

    #[test]
    fn canonical_body_metadata_wins_over_compatibility_header() {
        let body = compaction_request_body(
            Value::String(compaction_metadata(
                "responses",
                "auto",
                "context_limit",
                "pre_turn",
                "prefix_compaction",
            )),
            json!([]),
        );
        let header = compaction_metadata(
            "responses_compact",
            "manual",
            "user_requested",
            "standalone_turn",
            "memento",
        );

        let marker = marker_json(compaction_marker(
            Some(&body),
            Some(&header),
            "/v1/responses/compact",
        ))
        .expect("body marker");
        assert_eq!(marker.get("mode").and_then(Value::as_str), Some("local"));
        assert_eq!(
            marker.get("implementation").and_then(Value::as_str),
            Some("responses")
        );
        assert_eq!(marker.get("trigger").and_then(Value::as_str), Some("auto"));
    }

    #[test]
    fn usable_non_compaction_body_metadata_blocks_header_and_protocol_fallbacks() {
        let body = compaction_request_body(
            Value::String(json!({ "request_kind": "turn" }).to_string()),
            json!([{ "type": "compaction_trigger" }]),
        );
        let header = compaction_metadata(
            "responses_compact",
            "manual",
            "user_requested",
            "standalone_turn",
            "memento",
        );

        assert!(compaction_marker(Some(&body), Some(&header), "/v1/responses/compact").is_none());
    }

    #[test]
    fn malformed_body_metadata_yields_to_usable_header() {
        let body = compaction_request_body(Value::String("not-json".to_string()), json!([]));
        let header = compaction_metadata(
            "responses_compact",
            "manual",
            "user_requested",
            "standalone_turn",
            "memento",
        );

        let marker = marker_json(compaction_marker(
            Some(&body),
            Some(&header),
            "/v1/responses",
        ))
        .expect("header marker");
        assert_eq!(
            marker.get("implementation").and_then(Value::as_str),
            Some("responses_compact")
        );
    }

    #[test]
    fn duplicate_headers_are_unusable_and_allow_protocol_fallbacks() {
        let metadata = compaction_metadata(
            "responses_compact",
            "manual",
            "user_requested",
            "standalone_turn",
            "memento",
        );
        let mut headers = HeaderMap::new();
        headers.append(
            CODEX_TURN_METADATA_KEY,
            "not-json".parse().expect("malformed test metadata header"),
        );
        headers.append(
            CODEX_TURN_METADATA_KEY,
            metadata.parse().expect("valid test metadata header"),
        );

        assert!(classify_codex_context_compaction(
            "codex",
            &Method::POST,
            "/v1/responses",
            &headers,
            Some(&json!({ "input": [] })),
        )
        .is_none());

        let mut malformed_headers = HeaderMap::new();
        malformed_headers.append(
            CODEX_TURN_METADATA_KEY,
            "not-json".parse().expect("malformed test metadata header"),
        );
        malformed_headers.append(
            CODEX_TURN_METADATA_KEY,
            "[]".parse().expect("wrong-shape test metadata header"),
        );
        let fallback = marker_json(classify_codex_context_compaction(
            "codex",
            &Method::POST,
            "/v1/responses/compact",
            &malformed_headers,
            Some(&json!({ "input": [] })),
        ))
        .expect("protocol fallback marker");
        assert_eq!(
            fallback.get("implementation").and_then(Value::as_str),
            Some("responses_compact")
        );

        let mut conflicting_headers = HeaderMap::new();
        conflicting_headers.append(
            CODEX_TURN_METADATA_KEY,
            json!({ "request_kind": "turn" })
                .to_string()
                .parse()
                .expect("non-compaction test metadata header"),
        );
        conflicting_headers.append(
            CODEX_TURN_METADATA_KEY,
            metadata.parse().expect("later compaction metadata header"),
        );
        let fallback = marker_json(classify_codex_context_compaction(
            "codex",
            &Method::POST,
            "/v1/responses/compact",
            &conflicting_headers,
            Some(&json!({ "input": [] })),
        ))
        .expect("duplicate headers must yield protocol fallback");
        assert_eq!(
            fallback.get("implementation").and_then(Value::as_str),
            Some("responses_compact")
        );
        assert_eq!(
            fallback.get("trigger").and_then(Value::as_str),
            Some(UNKNOWN_VALUE)
        );
    }

    #[test]
    fn usable_non_compaction_header_blocks_protocol_fallbacks() {
        let body = json!({
            "model": "gpt-5.4-mini",
            "input": [{ "type": "compaction_trigger" }],
        });
        let header = json!({ "request_kind": "turn" }).to_string();

        assert!(compaction_marker(Some(&body), Some(&header), "/v1/responses/compact").is_none());
    }

    #[test]
    fn protocol_fallbacks_identify_remote_v1_and_v2() {
        let v1 = marker_json(compaction_marker(
            Some(&json!({ "input": [] })),
            None,
            "/nested/openai/v1/responses/compact/",
        ))
        .expect("v1 marker");
        assert_eq!(v1.get("mode").and_then(Value::as_str), Some("remote"));
        assert_eq!(
            v1.get("implementation").and_then(Value::as_str),
            Some("responses_compact")
        );
        assert_eq!(
            v1.get("trigger").and_then(Value::as_str),
            Some(UNKNOWN_VALUE)
        );

        let v2 = marker_json(compaction_marker(
            Some(&json!({
                "input": [
                    { "type": "message" },
                    { "type": "compaction_trigger" }
                ]
            })),
            None,
            "/nested/openai/v1/responses/",
        ))
        .expect("v2 marker");
        assert_eq!(v2.get("mode").and_then(Value::as_str), Some("remote"));
        assert_eq!(
            v2.get("implementation").and_then(Value::as_str),
            Some("responses_compaction_v2")
        );
    }

    #[test]
    fn malformed_explicit_metadata_allows_protocol_fallback() {
        let body = compaction_request_body(
            Value::String("not-json".to_string()),
            json!([{ "type": "compaction_trigger" }]),
        );

        let marker = marker_json(compaction_marker(Some(&body), Some("[]"), "/v1/responses"))
            .expect("v2 fallback marker");
        assert_eq!(
            marker.get("implementation").and_then(Value::as_str),
            Some("responses_compaction_v2")
        );
    }

    #[test]
    fn ignores_trigger_outside_top_level_input_array() {
        for body in [
            json!({ "input": { "type": "compaction_trigger" } }),
            json!({ "input": "compaction_trigger" }),
            json!({ "nested": { "input": [{ "type": "compaction_trigger" }] } }),
            json!({ "input": [{ "type": "COMPaction_trigger" }] }),
            json!({ "input": [{ "type": 1 }] }),
        ] {
            assert!(compaction_marker(Some(&body), None, "/v1/responses").is_none());
        }
    }

    #[test]
    fn compaction_classifier_is_scoped_to_codex_post_responses_paths() {
        let body = json!({ "input": [{ "type": "compaction_trigger" }] });
        let headers = HeaderMap::new();

        assert!(classify_codex_context_compaction(
            "claude",
            &Method::POST,
            "/v1/responses",
            &headers,
            Some(&body)
        )
        .is_none());
        assert!(classify_codex_context_compaction(
            "codex",
            &Method::GET,
            "/v1/responses",
            &headers,
            Some(&body)
        )
        .is_none());
        assert!(classify_codex_context_compaction(
            "codex",
            &Method::POST,
            "/v1/chat/completions",
            &headers,
            Some(&body)
        )
        .is_none());
        assert!(classify_codex_context_compaction(
            "codex",
            &Method::POST,
            "/v1/responses/extra",
            &headers,
            Some(&body)
        )
        .is_none());
    }

    #[test]
    fn oversized_metadata_is_unusable_and_does_not_persist_content() {
        let oversized = format!(
            r#"{{"request_kind":"compaction","padding":"{}"}}"#,
            "secret".repeat(MAX_CODEX_TURN_METADATA_BYTES)
        );
        let body = compaction_request_body(Value::String(oversized), json!([]));

        assert!(compaction_marker(Some(&body), None, "/v1/responses").is_none());
    }

    #[test]
    fn classifies_system_turn_metadata() {
        let body = request_body_with_turn_metadata(Value::String(system_turn_metadata()));

        assert_eq!(
            classify(Some(&body)),
            Some(json!({
                "type": CODEX_SYSTEM_REQUEST_SETTING_TYPE,
                "threadSource": CODEX_SYSTEM_REQUEST_THREAD_SOURCE,
            }))
        );
    }

    #[test]
    fn accepts_only_exact_responses_paths() {
        let body = request_body_with_turn_metadata(Value::String(system_turn_metadata()));

        for path in [
            "/responses",
            "/responses/",
            "/v1/responses",
            "/v1/responses/",
        ] {
            assert!(codex_system_request_special_setting(
                "codex",
                &Method::POST,
                path,
                Some(&body),
            )
            .is_some());
        }

        for path in ["responses", "/v1/responses//", "/v1/responses/extra"] {
            assert!(codex_system_request_special_setting(
                "codex",
                &Method::POST,
                path,
                Some(&body),
            )
            .is_none());
        }
    }

    #[test]
    fn rejects_non_codex_or_non_post_requests() {
        let body = request_body_with_turn_metadata(Value::String(system_turn_metadata()));

        assert!(codex_system_request_special_setting(
            "claude",
            &Method::POST,
            "/v1/responses",
            Some(&body),
        )
        .is_none());
        assert!(codex_system_request_special_setting(
            "codex",
            &Method::GET,
            "/v1/responses",
            Some(&body),
        )
        .is_none());
    }

    #[test]
    fn rejects_missing_or_invalid_outer_metadata_shapes() {
        let bodies = [
            Value::Null,
            json!([]),
            json!({}),
            json!({ "client_metadata": null }),
            json!({ "client_metadata": [] }),
            json!({ "client_metadata": {} }),
        ];

        assert!(classify(None).is_none());
        for body in &bodies {
            assert!(classify(Some(body)).is_none());
        }
    }

    #[test]
    fn rejects_non_string_turn_metadata() {
        for value in [Value::Null, json!({}), json!([]), json!(1), json!(true)] {
            let body = request_body_with_turn_metadata(value);
            assert!(classify(Some(&body)).is_none());
        }
    }

    #[test]
    fn rejects_empty_malformed_or_non_object_turn_metadata() {
        for raw in ["", "not-json", "null", "[]", "true"] {
            let body = request_body_with_turn_metadata(Value::String(raw.to_string()));
            assert!(classify(Some(&body)).is_none());
        }
    }

    #[test]
    fn rejects_missing_non_string_or_non_system_thread_source() {
        for metadata in [
            json!({}),
            json!({ "thread_source": null }),
            json!({ "thread_source": 1 }),
            json!({ "thread_source": true }),
            json!({ "thread_source": {} }),
            json!({ "thread_source": "user" }),
            json!({ "thread_source": "SYSTEM" }),
        ] {
            let body = request_body_with_turn_metadata(Value::String(metadata.to_string()));
            assert!(classify(Some(&body)).is_none());
        }
    }

    #[test]
    fn enforces_nested_metadata_byte_limit() {
        let prefix = r#"{"thread_source":"system","padding":""#;
        let suffix = r#""}"#;
        let padding = "x".repeat(MAX_CODEX_TURN_METADATA_BYTES - prefix.len() - suffix.len());
        let at_limit = format!("{prefix}{padding}{suffix}");
        assert_eq!(at_limit.len(), MAX_CODEX_TURN_METADATA_BYTES);

        let at_limit_body = request_body_with_turn_metadata(Value::String(at_limit.clone()));
        assert!(classify(Some(&at_limit_body)).is_some());

        let over_limit_body =
            request_body_with_turn_metadata(Value::String(format!("{at_limit} ")));
        assert!(classify(Some(&over_limit_body)).is_none());
    }
}
