//! Usage: Gateway proxy module facade (exports the proxy handler + shared types).

use axum::http::Method;

mod abort_guard;
mod caches;
mod cli_proxy_guard;
pub(super) mod cx2cc;
mod error_code;
mod errors;
mod failover;
mod fake_200;
mod forwarder;
mod gemini_oauth;
mod handler;
mod http_util;
mod logging;
pub(in crate::gateway) mod model_rewrite;
pub(in crate::gateway) mod protocol_bridge;
pub(in crate::gateway) mod provider_router;
mod request_body;
mod request_context;
mod request_end;
mod sse;
pub(in crate::gateway) use sse::{find_sse_event_end, parse_sse_frame};
pub(in crate::gateway) mod status_override;
mod types;
pub(in crate::gateway) mod upstream_client_error_rules;
mod upstream_error_response_rules;

pub(super) use caches::{ProviderBaseUrlPingCache, RecentErrorCache};
pub(super) use error_code::GatewayErrorCode;
pub(crate) use failover::resolve_transport_base_url;
pub(in crate::gateway) use fake_200::is_fake_200_non_stream_body;
pub(in crate::gateway) use logging::spawn_enqueue_request_log_with_backpressure;
pub(super) use types::ErrorCategory;

pub(super) use handler::proxy_impl;

pub(super) async fn prepare_gemini_oauth_probe(
    client: &reqwest::Client,
    access_token: &str,
    path: &str,
    body: &serde_json::Value,
) -> Result<(String, serde_json::Value), String> {
    #[cfg(test)]
    if let Ok(project) = std::env::var("AIO_CODING_HUB_TEST_PROBE_GEMINI_PROJECT") {
        let request = gemini_oauth::prepare_upstream_request_with_project(
            path,
            None,
            body.clone(),
            None,
            Some(&project),
        )?;
        let base_url = std::env::var("AIO_CODING_HUB_TEST_PROBE_OAUTH_BASE_URL")
            .expect("synthetic project requires a mock upstream");
        let url = crate::gateway::util::build_target_url(
            &base_url,
            &request.forwarded_path,
            request.query.as_deref(),
        )?;
        let body = serde_json::from_slice(&request.body_bytes)
            .map_err(|_| "PROBE_STRUCTURE".to_string())?;
        return Ok((url.to_string(), body));
    }
    let request = gemini_oauth::prepare_upstream_request(
        client,
        access_token,
        path,
        None,
        Some(body),
        &bytes::Bytes::new(),
        None,
    )
    .await?;
    let url = crate::gateway::util::build_target_url(
        &request.base_url,
        &request.forwarded_path,
        request.query.as_deref(),
    )?;
    let body =
        serde_json::from_slice(&request.body_bytes).map_err(|_| "PROBE_STRUCTURE".to_string())?;
    Ok((url.to_string(), body))
}

const CLAUDE_COUNT_TOKENS_PATH: &str = "/v1/messages/count_tokens";
const CLAUDE_LOGGED_MESSAGES_PATH: &str = "/v1/messages";

fn is_claude_count_tokens_request(cli_key: &str, forwarded_path: &str) -> bool {
    cli_key == "claude" && forwarded_path == CLAUDE_COUNT_TOKENS_PATH
}

fn should_observe_request(cli_key: &str, method: &Method, forwarded_path: &str) -> bool {
    if is_codex_model_discovery_request(cli_key, method, forwarded_path) {
        return false;
    }

    cli_key != "claude" || forwarded_path == CLAUDE_LOGGED_MESSAGES_PATH
}

fn is_codex_model_discovery_request(cli_key: &str, method: &Method, forwarded_path: &str) -> bool {
    cli_key == "codex"
        && method == Method::GET
        && matches!(
            forwarded_path.trim_end_matches('/'),
            "/v1/models" | "/models"
        )
}

fn is_claude_probe_request(
    forwarded_path: &str,
    introspection_json: Option<&serde_json::Value>,
) -> bool {
    if forwarded_path != CLAUDE_LOGGED_MESSAGES_PATH {
        return false;
    }

    let Some(root) = introspection_json else {
        return false;
    };
    let Some(messages) = root.get("messages").and_then(|value| value.as_array()) else {
        return false;
    };
    if messages.len() != 1 {
        return false;
    }

    let Some(first_message) = messages.first().and_then(|value| value.as_object()) else {
        return false;
    };
    if first_message.get("role").and_then(|value| value.as_str()) != Some("user") {
        return false;
    }

    let Some(content) = first_message
        .get("content")
        .and_then(|value| value.as_str())
    else {
        return false;
    };
    let normalized = content.trim().to_ascii_lowercase();
    normalized == "foo" || normalized == "count"
}

fn compute_observe_request(
    cli_key: &str,
    method: &Method,
    forwarded_path: &str,
    introspection_json: Option<&serde_json::Value>,
) -> bool {
    if !should_observe_request(cli_key, method, forwarded_path) {
        return false;
    }

    if cli_key != "claude" {
        return true;
    }

    !is_claude_probe_request(forwarded_path, introspection_json)
}

fn should_seed_in_progress_request_log(cli_key: &str, forwarded_path: &str, observe: bool) -> bool {
    observe && cli_key == "claude" && forwarded_path == CLAUDE_LOGGED_MESSAGES_PATH
}

fn build_claude_probe_response_body() -> serde_json::Value {
    serde_json::json!({
        "input_tokens": 0
    })
}

pub(super) struct RequestLogEnqueueArgs {
    pub(super) trace_id: String,
    pub(super) cli_key: String,
    pub(super) session_id: Option<String>,
    pub(super) method: String,
    pub(super) path: String,
    pub(super) query: Option<String>,
    pub(super) excluded_from_stats: bool,
    pub(super) special_settings_json: Option<String>,
    pub(super) status: Option<u16>,
    pub(super) error_code: Option<&'static str>,
    pub(super) duration_ms: u128,
    pub(super) ttfb_ms: Option<u128>,
    pub(super) visible_ttfb_ms: Option<u128>,
    pub(super) upstream_stream_duration_ms: Option<u128>,
    pub(super) upstream_stream_timing_version: i64,
    pub(super) final_upstream_attempt_duration_ms: Option<u128>,
    pub(super) final_upstream_attempt_timing_version: i64,
    pub(super) estimated_final_upstream_attempt_duration_ms: Option<u128>,
    pub(super) attempts_json: String,
    pub(super) requested_model: Option<String>,
    pub(super) created_at_ms: i64,
    pub(super) last_activity_ms: Option<i64>,
    pub(super) activity_details_json: Option<String>,
    pub(super) created_at: i64,
    pub(super) usage_metrics: Option<crate::usage::UsageMetrics>,
    pub(super) usage: Option<crate::usage::UsageExtract>,
    pub(super) provider_chain_json: Option<String>,
    pub(super) error_details_json: Option<String>,
}

#[cfg(test)]
mod tests;
