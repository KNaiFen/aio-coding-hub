pub(crate) mod access_token;
pub(crate) mod active_requests;
mod background_tasks;
mod billing_header_rectifier;
mod binder;
mod claude_metadata_user_id_injection;
pub(crate) mod cli_auth;
mod codex_session_id;
mod configured_model_route;
pub(crate) mod control_service;
pub(crate) mod events;
pub(crate) mod http_client;
pub(crate) mod listen;
mod managed_model_route;
pub(crate) mod manager;
mod model_route_mapping;
pub(crate) mod oauth;
pub(crate) mod observation;
pub(crate) mod plugins;
mod proxy;
mod response_fixer;
mod routes;
pub(crate) mod runtime;
pub(crate) mod session_manager;
mod streams;
mod thinking_budget_rectifier;
mod thinking_signature_rectifier;
mod upstream_fingerprint;
mod upstream_identity;
pub(crate) mod util;
mod warmup;

use crate::settings;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, specta::Type, Default, PartialEq, Eq)]
pub struct GatewayStatus {
    pub running: bool,
    pub port: Option<u16>,
    pub base_url: Option<String>,
    pub listen_addr: Option<String>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct GatewayProviderCircuitStatus {
    pub provider_id: i64,
    pub state: String,
    pub failure_count: u32,
    pub failure_threshold: u32,
    pub open_until: Option<i64>,
    pub cooldown_until: Option<i64>,
}

pub(crate) fn planned_base_url(
    cfg: &settings::AppSettings,
) -> crate::shared::error::AppResult<String> {
    binder::planned_base_url(cfg)
}

pub(crate) fn listen_rebind_required(
    previous: &settings::AppSettings,
    next: &settings::AppSettings,
) -> bool {
    binder::listen_rebind_required(previous, next)
}

pub(crate) fn listener_accepts_non_loopback(
    cfg: &settings::AppSettings,
) -> crate::shared::error::AppResult<bool> {
    binder::listener_accepts_non_loopback(cfg)
}

pub(crate) fn resolve_transport_base_url(
    transport: &crate::providers::ProviderTransportContext,
    cli_key: &str,
) -> Result<String, String> {
    proxy::resolve_transport_base_url(transport, cli_key)
}

pub(crate) fn build_translated_bridge_probe(
    bridge_type: &str,
    model_mapping: crate::providers::ModelMapping,
    source_model: &str,
) -> Result<(String, serde_json::Value), String> {
    let bridge = proxy::protocol_bridge::get_bridge(bridge_type)
        .ok_or_else(|| format!("BRIDGE_UNSUPPORTED_TYPE: unsupported bridge_type={bridge_type}"))?;
    let body = serde_json::json!({
        "model": source_model,
        "input": "Reply with the single word OK.",
        "max_output_tokens": 100,
        "stream": false
    });
    let ctx = proxy::protocol_bridge::BridgeContext {
        claude_models: Default::default(),
        model_mapping,
        cx2cc_settings: Default::default(),
        requested_model: Some(source_model.to_string()),
        mapped_model: None,
        stream_requested: false,
        is_chatgpt_backend: false,
        responses_cache_namespace: None,
        responses_cache_input: None,
    };
    let mut translated = bridge
        .translate_request(body, &ctx)
        .map_err(|err| format!("BRIDGE_TRANSLATE_FAILED: {err}"))?;
    if let Some(body) = translated.body.as_object_mut() {
        body.remove("reasoning");
        body.remove("reasoning_effort");
        body.remove("thinking");
        if translated.target_path.ends_with("/responses") {
            body.insert("max_output_tokens".into(), serde_json::json!(100));
        } else if translated.target_path.ends_with("/messages")
            || translated.target_path.ends_with("/chat/completions")
        {
            body.insert("max_tokens".into(), serde_json::json!(100));
        }
    }
    Ok((translated.target_path, translated.body))
}

pub(crate) use proxy::protocol_bridge::cx2cc::codex_chatgpt_request_compat_value as probe_codex_oauth_body;
pub(crate) use proxy::protocol_bridge::probe::gemini_oauth_frame as probe_gemini_oauth_frame;
pub(crate) use proxy::protocol_bridge::probe::gemini_oauth_response as probe_gemini_oauth_response;
pub(crate) use proxy::protocol_bridge::probe::parse_codex_chatgpt_account_id as probe_codex_account_id;
pub(crate) use proxy::protocol_bridge::probe::{
    next_frame_end, validate_json as validate_probe_json, ProbeProtocol, ProbeStream,
};

pub(crate) async fn prepare_gemini_oauth_probe(
    client: &reqwest::Client,
    access_token: &str,
    path: &str,
    body: &serde_json::Value,
) -> Result<(String, serde_json::Value), String> {
    proxy::prepare_gemini_oauth_probe(client, access_token, path, body).await
}

pub(crate) fn cx2cc_probe_model(
    source_model: &str,
    models: &crate::providers::ClaudeModels,
    settings: &crate::settings::AppSettings,
) -> String {
    proxy::protocol_bridge::cx2cc::map_claude_to_openai(
        source_model,
        models,
        &proxy::cx2cc::settings::Cx2ccSettings::from_app_settings(settings),
    )
}
