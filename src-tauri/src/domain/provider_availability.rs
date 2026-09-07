//! Usage: Lightweight provider availability probe.
//!
//! Sends a minimal API request to verify that a provider's base URL + credentials
//! are reachable and functional. Supports all recognized provider CLI types.

use crate::providers::{is_supported_bridge_type, ModelMapping, CX2CC_BRIDGE_TYPE};
use crate::shared::error::{db_err, AppError, AppResult};
use crate::{blocking, db};
use reqwest::header::{HeaderMap, HeaderValue};
use rusqlite::{params, params_from_iter, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(45);
const WORK_TIMEOUT: Duration = Duration::from_secs(60);
const PROBE_PROMPT: &str = "Reply with the single word OK.";
const PROBE_RESPONSE_BODY_LIMIT: usize = 64 * 1024;
const PROBE_RESPONSE_PREVIEW_LIMIT: usize = 500;
const AVAILABILITY_RETENTION_MS: i64 = 24 * 60 * 60 * 1_000;
const AVAILABILITY_RETENTION_BATCH_SIZE: usize = 1_000;
const AVAILABILITY_RETENTION_INTERVAL: Duration = Duration::from_secs(60 * 60);
pub const TUI_PROVIDER_AVAILABILITY_BUCKETS: u16 = 12;
pub const TRAY_PROVIDER_AVAILABILITY_BUCKETS: u16 = 18;
pub const DESKTOP_PROVIDER_AVAILABILITY_BUCKETS: u16 = 36;

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ProviderAvailabilityResult {
    pub ok: bool,
    pub provider_id: i64,
    pub provider_name: String,
    pub base_url: String,
    pub status: Option<u16>,
    pub latency_ms: i64,
    pub error: Option<String>,
    pub response_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[specta(optional)]
    pub requested_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[specta(optional)]
    pub tested_model: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderAvailabilityState {
    Healthy,
    Degraded,
    Unhealthy,
    NoData,
}

#[derive(Debug, Clone, Serialize, specta::Type, PartialEq, Eq)]
pub struct ProviderAvailabilityBucket {
    pub start_at_ms: i64,
    pub end_at_ms: i64,
    pub success_count: u32,
    pub failure_count: u32,
    pub state: ProviderAvailabilityState,
}

#[derive(Debug, Clone, Serialize, specta::Type, PartialEq, Eq)]
pub struct ProviderAvailabilityTimeline {
    pub provider_id: i64,
    pub hours: u32,
    pub bucket_count: u16,
    pub bucket_minutes: u32,
    pub success_count: u32,
    pub failure_count: u32,
    pub buckets: Vec<ProviderAvailabilityBucket>,
}

#[derive(Debug, Deserialize)]
struct AvailabilityAttempt {
    provider_id: i64,
    outcome: String,
    error_category: Option<String>,
    #[serde(default)]
    upstream_sent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AvailabilityObservation {
    trace_id: String,
    cli_key: String,
    provider_id: i64,
    observed_at_ms: i64,
    success: bool,
}

struct LoadedProvider {
    id: i64,
    transport_provider_id: i64,
    cli_key: String,
    name: String,
    base_urls: Vec<String>,
    api_key_plaintext: String,
    availability_test_model: Option<String>,
    model_mapping: ModelMapping,
    claude_models: crate::providers::ClaudeModels,
    auth_mode: String,
    oauth_provider_type: Option<String>,
    source_provider_id: Option<i64>,
    bridge_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProbeResponseBody {
    bytes: Vec<u8>,
    truncated: bool,
    limit: usize,
}

fn append_probe_response_chunk(bytes: &mut Vec<u8>, chunk: &[u8], limit: usize) -> bool {
    let remaining = limit.saturating_sub(bytes.len());
    if remaining == 0 {
        return !chunk.is_empty();
    }

    let keep = chunk.len().min(remaining);
    bytes.extend_from_slice(&chunk[..keep]);
    keep < chunk.len()
}

async fn read_probe_response_body_with_limit(
    mut resp: reqwest::Response,
    limit: usize,
) -> Result<ProbeResponseBody, &'static str> {
    let content_length = resp.content_length();
    let mut truncated = content_length.is_some_and(|len| len > limit as u64);
    let capacity = content_length
        .and_then(|len| usize::try_from(len).ok())
        .unwrap_or_default()
        .min(limit);
    let mut bytes = Vec::with_capacity(capacity);

    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|error| if error.is_timeout() { "PROBE_TIMEOUT" } else { "PROBE_READ" })?
    {
        if append_probe_response_chunk(&mut bytes, chunk.as_ref(), limit) {
            truncated = true;
            break;
        }
    }

    Ok(ProbeResponseBody {
        bytes,
        truncated,
        limit,
    })
}

fn probe_response_preview(body: &ProbeResponseBody) -> String {
    let preview_len = body.bytes.len().min(PROBE_RESPONSE_PREVIEW_LIMIT);
    let mut preview = String::from_utf8_lossy(&body.bytes[..preview_len]).to_string();
    if body.truncated {
        if !preview.is_empty() {
            preview.push('\n');
        }
        preview.push_str(&format!(
            "[probe response truncated after {} bytes]",
            body.limit
        ));
    }
    preview
}

async fn load_provider_for_test(db: db::Db, provider_id: i64) -> AppResult<LoadedProvider> {
    blocking::run("provider_availability_load", move || -> AppResult<LoadedProvider> {
        if provider_id <= 0 {
            return Err(format!("SEC_INVALID_INPUT: invalid provider_id={provider_id}").into());
        }

        let conn = db.open_connection()?;
        #[allow(clippy::type_complexity)]
        let row: Option<(
            i64,
            String,
            String,
            String,
            String,
            String,
            Option<String>,
            String,
            String,
            Option<String>,
            Option<i64>,
            Option<String>,
        )> = conn
            .query_row(
                r#"
SELECT id, cli_key, name, base_url, base_urls_json, api_key_plaintext, availability_test_model, model_mapping_json, auth_mode, oauth_provider_type, source_provider_id, bridge_type
FROM providers
WHERE id = ?1
"#,
                rusqlite::params![provider_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                        row.get(9)?,
                        row.get(10)?,
                        row.get(11)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| format!("DB_ERROR: {e}"))?;

        let Some((id, cli_key, name, base_url_fallback, base_urls_json, api_key_plaintext, availability_test_model, model_mapping_json, auth_mode, oauth_provider_type, source_provider_id, bridge_type)) = row else {
            return Err("DB_NOT_FOUND: provider not found".into());
        };

        let mut base_urls: Vec<String> = serde_json::from_str::<Vec<String>>(&base_urls_json)
            .ok()
            .unwrap_or_default()
            .into_iter()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .collect();

        if base_urls.is_empty() {
            let fallback = base_url_fallback.trim().to_string();
            if !fallback.is_empty() {
                base_urls.push(fallback);
            }
        }

        let claude_models = conn.query_row(
            "SELECT claude_models_json FROM providers WHERE id = ?1", [id],
            |row| row.get::<_, String>(0),
        ).map_err(|e| db_err!("failed to load probe bridge models: {e}"))?;
        Ok(LoadedProvider {
            id,
            transport_provider_id: id,
            cli_key,
            name,
            base_urls,
            api_key_plaintext,
            availability_test_model: normalize_probe_model(availability_test_model.as_deref()),
            model_mapping: model_mapping_from_json(&model_mapping_json),
            claude_models: serde_json::from_str(&claude_models).unwrap_or_default(),
            auth_mode,
            oauth_provider_type,
            source_provider_id,
            bridge_type,
        })
    })
    .await
}

async fn load_effective_provider_for_test(
    db: db::Db,
    provider_id: i64,
) -> AppResult<LoadedProvider> {
    let provider = load_provider_for_test(db.clone(), provider_id).await?;
    let Some(bridge_type) = provider.bridge_type.as_deref() else {
        return Ok(provider);
    };

    if bridge_type == CX2CC_BRIDGE_TYPE && provider.source_provider_id.is_none() {
        return Ok(provider);
    }

    let Some(source_provider_id) = provider.source_provider_id else {
        return Ok(provider);
    };

    let bridge_type = bridge_type.to_string();
    let (source, source_cli_key) = blocking::run("provider_availability_source", move || {
        crate::providers::get_source_provider_for_availability(&db, source_provider_id, &bridge_type)
    }).await?;

    Ok(LoadedProvider {
        id: provider.id,
        transport_provider_id: source.id,
        cli_key: source_cli_key,
        name: provider.name,
        base_urls: source.base_urls,
        api_key_plaintext: source.api_key_plaintext,
        availability_test_model: provider.availability_test_model,
        model_mapping: provider.model_mapping,
        claude_models: provider.claude_models,
        auth_mode: source.auth_mode,
        oauth_provider_type: source.oauth_provider_type,
        source_provider_id: provider.source_provider_id,
        bridge_type: provider.bridge_type,
    })
}

impl LoadedProvider {
    fn transport_context(&self) -> crate::providers::ProviderTransportContext {
        crate::providers::ProviderTransportContext {
            provider_id: self.transport_provider_id,
            base_urls: self.base_urls.clone(),
            api_key_plaintext: self.api_key_plaintext.clone(),
            auth_mode: self.auth_mode.clone(),
            oauth_provider_type: self.oauth_provider_type.clone(),
        }
    }

    fn resolved_base_url(&self) -> AppResult<String> {
        crate::gateway::resolve_transport_base_url(&self.transport_context(), &self.cli_key)
            .map_err(Into::into)
    }
}

fn normalize_probe_model(value: Option<&str>) -> Option<String> {
    let trimmed = value?.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

fn model_mapping_from_json(raw: &str) -> ModelMapping {
    let mapping = serde_json::from_str::<ModelMapping>(raw)
        .ok()
        .unwrap_or_default();
    ModelMapping {
        default_model: normalize_probe_model(mapping.default_model.as_deref()),
        exact: mapping
            .exact
            .into_iter()
            .filter_map(|(key, value)| {
                let key = normalize_probe_model(Some(&key))?;
                let value = normalize_probe_model(Some(&value))?;
                Some((key, value))
            })
            .collect(),
    }
}

fn resolve_codex_probe_model_from_sources(
    provider_override: Option<&str>,
    global_setting: Option<&str>,
) -> String {
    normalize_probe_model(provider_override)
        .or_else(|| normalize_probe_model(global_setting))
        .unwrap_or_else(|| crate::settings::DEFAULT_CODEX_PROVIDER_TEST_MODEL.to_string())
}

fn build_probe_request(
    cli_key: &str,
    base_url: &str,
    api_key: &str,
    model_override: Option<&str>,
    grok_preferences: Option<&crate::grok_config::GrokProxyPreferences>,
) -> AppResult<(String, HeaderMap, serde_json::Value)> {
    match cli_key {
        "claude" => {
            let url = build_probe_url(base_url, "/v1/messages", None)?;
            let mut headers = HeaderMap::new();
            if let Ok(v) = HeaderValue::from_str(api_key) {
                headers.insert("x-api-key", v);
            }
            headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
            headers.insert("content-type", HeaderValue::from_static("application/json"));
            let body = serde_json::json!({
                "model": model_override.unwrap_or("claude-sonnet-4-6"),
                "max_tokens": 100,
                "messages": [{"role": "user", "content": PROBE_PROMPT}]
            });
            Ok((url, headers, body))
        }
        "codex" => {
            let url = build_probe_url(base_url, "/v1/responses", None)?;
            let mut headers = HeaderMap::new();
            let bearer = format!("Bearer {api_key}");
            if let Ok(v) = HeaderValue::from_str(&bearer) {
                headers.insert("authorization", v);
            }
            headers.insert("content-type", HeaderValue::from_static("application/json"));
            let body = serde_json::json!({
                "model": model_override.unwrap_or(crate::settings::DEFAULT_CODEX_PROVIDER_TEST_MODEL),
                "max_output_tokens": 100,
                "input": [{"role": "user", "content": PROBE_PROMPT}],
                "store": false,
                "stream": false
            });
            Ok((url, headers, body))
        }
        "grok" => {
            let preferences = crate::grok_config::validate_preferences(
                grok_preferences.cloned().unwrap_or_default(),
            )?;
            let mut headers = HeaderMap::new();
            let bearer = format!("Bearer {api_key}");
            if let Ok(v) = HeaderValue::from_str(&bearer) {
                headers.insert("authorization", v);
            }
            headers.insert("content-type", HeaderValue::from_static("application/json"));
            let (url, body) = match preferences.api_backend {
                crate::grok_config::GrokApiBackend::Responses => (
                    build_probe_url(base_url, "/v1/responses", None)?,
                    serde_json::json!({
                        "model": preferences.model_id,
                        "input": PROBE_PROMPT,
                        "max_output_tokens": 100,
                        "store": false,
                        "stream": false
                    }),
                ),
                crate::grok_config::GrokApiBackend::ChatCompletions => (
                    build_probe_url(base_url, "/v1/chat/completions", None)?,
                    serde_json::json!({
                        "model": preferences.model_id,
                        "messages": [{"role": "user", "content": PROBE_PROMPT}],
                        "max_tokens": 100,
                        "stream": false
                    }),
                ),
            };
            Ok((url, headers, body))
        }
        "gemini" => {
            let query = format!("key={api_key}");
            let url = build_probe_url(
                base_url,
                "/v1beta/models/gemini-2.0-flash:generateContent",
                Some(&query),
            )?;
            let mut headers = HeaderMap::new();
            headers.insert("content-type", HeaderValue::from_static("application/json"));
            let body = serde_json::json!({
                "contents": [{"role": "user", "parts": [{"text": PROBE_PROMPT}]}],
                "generationConfig": {"maxOutputTokens": 100}
            });
            Ok((url, headers, body))
        }
        _ => Err(format!("UNSUPPORTED_CLI_KEY: {cli_key}").into()),
    }
}

fn build_probe_request_with_body(
    cli_key: &str,
    base_url: &str,
    api_key: &str,
    target_path: &str,
    body: serde_json::Value,
) -> AppResult<(String, HeaderMap, serde_json::Value)> {
    let path = if target_path.starts_with('/') {
        target_path.to_string()
    } else {
        format!("/{target_path}")
    };
    let mut url = build_probe_url(base_url, &path, None)?;
    let mut headers = HeaderMap::new();

    match cli_key {
        "claude" => {
            if let Ok(v) = HeaderValue::from_str(api_key) {
                headers.insert("x-api-key", v);
            }
            headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        }
        "codex" | "grok" => {
            let bearer = format!("Bearer {api_key}");
            if let Ok(v) = HeaderValue::from_str(&bearer) {
                headers.insert("authorization", v);
            }
        }
        "gemini" => {
            let separator = if url.contains('?') { '&' } else { '?' };
            url.push(separator);
            url.push_str("key=");
            url.push_str(api_key);
        }
        _ => return Err(format!("UNSUPPORTED_CLI_KEY: {cli_key}").into()),
    }

    headers.insert("content-type", HeaderValue::from_static("application/json"));
    Ok((url, headers, body))
}

fn build_bridge_probe_request(
    provider: &LoadedProvider,
    base_url: &str,
    api_key: &str,
    source_model: &str,
) -> AppResult<(String, HeaderMap, serde_json::Value)> {
    let bridge_type = provider
        .bridge_type
        .as_deref()
        .ok_or_else(|| "BRIDGE_MISSING_TYPE: bridge provider missing bridge_type".to_string())?;
    let (target_path, translated_body) = crate::gateway::build_translated_bridge_probe(
        bridge_type,
        provider.model_mapping.clone(),
        source_model,
    )?;
    build_probe_request_with_body(
        &provider.cli_key,
        base_url,
        api_key,
        &target_path,
        translated_body,
    )
}

fn build_probe_url(base_url: &str, path: &str, query: Option<&str>) -> AppResult<String> {
    Ok(crate::gateway::util::build_target_url(base_url, path, query)?.to_string())
}

#[cfg(test)]
fn redact_key_param(msg: &str) -> String {
    regex::Regex::new(r"([?&])key=[^&\s]*")
        .map(|re| re.replace_all(msg, "${1}key=***").to_string())
        .unwrap_or_else(|_| msg.to_string())
}

fn redact_probe_credential(input: &str, credential: &str) -> String {
    crate::domain::provider_account_usage::redact_secret(input, credential)
}

fn looks_like_auth_failure(status: u16, response_text: &str) -> bool {
    if matches!(status, 401 | 403) {
        return true;
    }

    let lower = response_text.to_ascii_lowercase();
    [
        "api key not valid",
        "invalid api key",
        "invalid_api_key",
        "invalid x-api-key",
        "authentication",
        "unauthorized",
        "permission denied",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn should_map_bridge_probe_model(bridge_type: Option<&str>) -> bool {
    matches!(bridge_type, Some(value) if value != CX2CC_BRIDGE_TYPE && is_supported_bridge_type(value))
}

fn probe_failure(error: &AppError) -> AppError {
    let raw = error.to_string();
    let code = if error.code() == "INTERNAL_ERROR" {
        raw.strip_prefix("INTERNAL_ERROR: ").unwrap_or_default()
    } else {
        error.code()
    };
    let (code, message) = match code {
        "PROBE_NO_TEXT" => ("PROBE_NO_TEXT", "本次探测未获得有效回答；仅推理内容不算回答，100 token 预算可能已耗尽"),
        "PROBE_UNFINISHED" => ("PROBE_UNFINISHED", "响应提前结束，未收到完整结束标记"),
        "PROBE_TERMINATION" => ("PROBE_TERMINATION", "上游以不支持的原因结束了生成"),
        "PROBE_READ" => ("PROBE_READ", "读取上游响应失败"),
        "PROBE_TOO_LARGE" => ("PROBE_TOO_LARGE", "上游响应超过 64 KiB 限制"),
        "PROBE_TIMEOUT" => ("PROBE_TIMEOUT", "本次探测超时，请稍后重试"),
        "PROBE_HTTP" => ("PROBE_HTTP", "上游 HTTP 请求失败"),
        "PROBE_AUTH" | "AUTH_RELOGIN_REQUIRED" | "OAUTH_REFRESH_FAILED" => ("PROBE_AUTH", "认证失败，请检查凭据或重新登录"),
        "PROBE_MODEL_QUOTA" => ("PROBE_MODEL_QUOTA", "上游拒绝请求，请检查模型、配额或频率限制"),
        "PROBE_STRUCTURE" => ("PROBE_STRUCTURE", "上游响应格式不符合预期协议"),
        "PROBE_UPSTREAM_ERROR" => ("PROBE_UPSTREAM_ERROR", "上游返回错误，本次生成失败"),
        "PROBE_OAUTH_PROTOCOL" => ("PROBE_OAUTH_PROTOCOL", "OAuth 认证不支持当前请求协议"),
        "PROBE_INVALID_URL" => ("PROBE_INVALID_URL", "供应商地址格式无效"),
        _ => ("PROBE_PREPARATION", "探测准备失败，请检查供应商配置"),
    };
    AppError::new(code, message)
}

pub async fn test_provider_availability<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    db: db::Db,
    provider_id: i64,
) -> AppResult<ProviderAvailabilityResult> {
    let started = Instant::now();
    let deadline = tokio::time::Instant::now() + WORK_TIMEOUT;
    let provider = tokio::time::timeout_at(deadline, load_effective_provider_for_test(db.clone(), provider_id))
        .await.map_err(|_| probe_failure(&AppError::from("PROBE_TIMEOUT")))??;

    if let Some(bridge_type) = provider.bridge_type.as_deref() {
        let bridge_label = if bridge_type == CX2CC_BRIDGE_TYPE {
            "CX2CC"
        } else if is_supported_bridge_type(bridge_type) {
            "转译桥接"
        } else {
            "未知桥接"
        };
        if bridge_type == CX2CC_BRIDGE_TYPE && provider.source_provider_id.is_none() {
            return Ok(ProviderAvailabilityResult {
                ok: false,
                provider_id: provider.id,
                provider_name: provider.name,
                base_url: provider.base_urls.first().cloned().unwrap_or_default(),
                status: None,
                latency_ms: 0,
                error: Some(format!("{bridge_label}供应商需通过其源供应商测试可用性")),
                response_preview: None,
                requested_model: None,
                tested_model: None,
            });
        }
    }

    let base_url = provider.resolved_base_url()?;
    if base_url.is_empty() {
        return Ok(ProviderAvailabilityResult {
            ok: false,
            provider_id: provider.id,
            provider_name: provider.name,
            base_url,
            status: None,
            latency_ms: 0,
            error: Some("供应商未配置 Base URL".into()),
            response_preview: None,
            requested_model: None,
            tested_model: None,
        });
    }

    if provider.auth_mode != "oauth" && provider.api_key_plaintext.trim().is_empty() {
        return Ok(ProviderAvailabilityResult {
            ok: false,
            provider_id: provider.id,
            provider_name: provider.name,
            base_url,
            status: None,
            latency_ms: 0,
            error: Some("供应商未配置 API Key".into()),
            response_preview: None,
            requested_model: None,
            tested_model: None,
        });
    }

    let bridge_probe_source_model =
        if should_map_bridge_probe_model(provider.bridge_type.as_deref()) {
            let settings = crate::settings::read(app)?;
            Some(resolve_codex_probe_model_from_sources(
                provider.availability_test_model.as_deref(),
                Some(settings.codex_provider_test_model.as_str()),
            ))
        } else {
            None
        };
    let regular_probe_model = if provider.bridge_type.as_deref() == Some(CX2CC_BRIDGE_TYPE) {
        Some(crate::gateway::cx2cc_probe_model("claude-sonnet-4-6", &provider.claude_models, &crate::settings::read(app)?))
    } else if bridge_probe_source_model.is_none() && provider.cli_key == "codex"
    {
        match normalize_probe_model(provider.availability_test_model.as_deref()) {
            Some(model) => Some(model),
            None => {
                let settings = crate::settings::read(app)?;
                Some(resolve_codex_probe_model_from_sources(
                    None,
                    Some(settings.codex_provider_test_model.as_str()),
                ))
            }
        }
    } else {
        None
    };
    let grok_preferences = if provider.cli_key == "grok" {
        Some(crate::grok_config::get(app)?.effective_preferences)
    } else {
        None
    };
    let (mut url, mut headers, mut body) = if let Some(source_model) = bridge_probe_source_model.as_deref() {
        build_bridge_probe_request(&provider, &base_url, "", source_model)?
    } else {
        build_probe_request(
            &provider.cli_key,
            &base_url,
            "",
            regular_probe_model.as_deref(),
            grok_preferences.as_ref(),
        )?
    };

    let mut parsed_url = reqwest::Url::parse(&url).map_err(|_| "PROBE_INVALID_URL")?;
    let mut protocol = if parsed_url.path().ends_with("/responses") {
        crate::gateway::ProbeProtocol::Responses
    } else if parsed_url.path().ends_with("/chat/completions") {
        crate::gateway::ProbeProtocol::Chat
    } else if parsed_url.path().ends_with("/messages") {
        crate::gateway::ProbeProtocol::Anthropic
    } else {
        crate::gateway::ProbeProtocol::Gemini
    };
    let tested_model = body.get("model").and_then(serde_json::Value::as_str).map(str::to_string)
        .or_else(|| parsed_url.path().split("/models/").nth(1)?.split(':').next().map(str::to_string));
    let requested_model = if provider.bridge_type.as_deref() == Some(CX2CC_BRIDGE_TYPE) {
        Some("claude-sonnet-4-6".to_string())
    } else { bridge_probe_source_model.clone().or_else(|| tested_model.clone()) };
    let mut result = ProviderAvailabilityResult {
        ok: false, provider_id: provider.id, provider_name: provider.name.clone(), base_url,
        status: None, latency_ms: 0, error: None, response_preview: None,
        requested_model, tested_model,
    };

    let client = reqwest::Client::builder()
        .user_agent(format!(
            "aio-coding-hub-probe/{}",
            env!("CARGO_PKG_VERSION")
        ))
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| format!("HTTP_CLIENT_INIT: {e}"))?;

    let mut effective_credential = String::new();
    let execution: AppResult<()> = match tokio::time::timeout_at(deadline, async {
        effective_credential = crate::providers::resolve_effective_transport_credential(
            &db, &client, &provider.cli_key, &provider.transport_context(),
        ).await?;
        let gemini_oauth = provider.auth_mode == "oauth" && provider.cli_key == "gemini";
        if provider.auth_mode == "oauth" {
            headers.remove("x-api-key");
            let adapter = crate::gateway::oauth::registry::resolve_oauth_adapter(
                &provider.cli_key, provider.transport_provider_id, provider.oauth_provider_type.as_deref(),
            )?;
            adapter.inject_upstream_headers(&mut headers, &effective_credential)?;
            if provider.cli_key == "codex" {
                if protocol == crate::gateway::ProbeProtocol::Chat {
                    body = serde_json::json!({
                        "model": result.tested_model, "max_output_tokens": 100,
                        "input": [{"role": "user", "content": PROBE_PROMPT}],
                        "store": false, "stream": false,
                    });
                    protocol = crate::gateway::ProbeProtocol::Responses;
                } else if protocol != crate::gateway::ProbeProtocol::Responses {
                    return Err("PROBE_OAUTH_PROTOCOL: ChatGPT requires Responses".into());
                }
                let details = crate::providers::get_oauth_details(&db, provider.transport_provider_id)?;
                if let Some(account_id) = crate::gateway::probe_codex_account_id(details.oauth_id_token.as_deref())
                    .or_else(|| crate::gateway::probe_codex_account_id(Some(&details.oauth_access_token))) {
                    headers.insert("chatgpt-account-id", HeaderValue::from_str(&account_id).map_err(|_| "PROBE_AUTH")?);
                }
                body = crate::gateway::probe_codex_oauth_body(&body);
                url = build_probe_url(&result.base_url, "/responses", None)?;
            } else if gemini_oauth {
                let prepared = crate::gateway::prepare_gemini_oauth_probe(&client, &effective_credential, parsed_url.path(), &body).await?;
                url = prepared.0;
                body = prepared.1;
            }
        } else if provider.cli_key == "gemini" {
            parsed_url.query_pairs_mut().clear().append_pair("key", &effective_credential);
            url = parsed_url.to_string();
        } else if provider.cli_key == "claude" {
            headers.insert("x-api-key", HeaderValue::from_str(&effective_credential).map_err(|_| "PROBE_AUTH")?);
        } else {
            headers.insert("authorization", HeaderValue::from_str(&format!("Bearer {effective_credential}")).map_err(|_| "PROBE_AUTH")?);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("PROBE_TIMEOUT: probe preparation exhausted the work budget".into());
        }
        let response = client.post(&url).headers(headers).json(&body).send().await
            .map_err(|error| if error.is_timeout() { "PROBE_TIMEOUT" } else { "PROBE_HTTP" })?;
        let status = response.status().as_u16();
        result.status = Some(status);
        if !response.status().is_success() {
            let body = read_probe_response_body_with_limit(response, PROBE_RESPONSE_BODY_LIMIT).await
                .map_err(|code| crate::shared::error::AppError::new(code, "probe response read failed"))?;
            let preview = redact_probe_credential(&probe_response_preview(&body), &effective_credential);
            let code = if looks_like_auth_failure(status, &preview) { "PROBE_AUTH" }
                else if matches!(status, 400 | 404 | 429) { "PROBE_MODEL_QUOTA" } else { "PROBE_HTTP" };
            result.response_preview = Some(preview);
            return Err(format!("{code}: HTTP {status}").into());
        }
        let is_sse = response.headers().get("content-type").and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.split(';').next().is_some_and(|mime| mime.trim() == "text/event-stream"));
        if body.get("stream") == Some(&serde_json::Value::Bool(true)) && !is_sse {
            return Err("PROBE_STRUCTURE: expected SSE".into());
        }
        if is_sse {
            read_probe_stream(response, protocol, gemini_oauth).await?;
        } else {
            let response_body = read_probe_response_body_with_limit(response, PROBE_RESPONSE_BODY_LIMIT).await
                .map_err(|code| crate::shared::error::AppError::new(code, "probe response read failed"))?;
            if response_body.truncated { return Err("PROBE_TOO_LARGE".into()); }
            let value: serde_json::Value = serde_json::from_slice(&response_body.bytes).map_err(|_| "PROBE_STRUCTURE")?;
            let value = if gemini_oauth {
                crate::gateway::probe_gemini_oauth_response(&value)?
            } else {
                &value
            };
            if let Err(error) = crate::gateway::validate_probe_json(protocol, value) {
                result.response_preview = Some(redact_probe_credential(&probe_response_preview(&response_body), &effective_credential));
                return Err(error.into());
            }
        }
        Ok(())
    }).await {
        Ok(result) => result,
        Err(_) => Err(crate::shared::error::AppError::new("PROBE_TIMEOUT", "probe work exceeded 60 seconds")),
    };
    result.latency_ms = started.elapsed().as_millis().min(i64::MAX as u128) as i64;
    match execution {
        Ok(()) => result.ok = true,
        Err(error) => {
            result.error = Some(probe_failure(&error).to_string());
        },
    }
    Ok(result)
}

async fn read_probe_stream(mut response: reqwest::Response, protocol: crate::gateway::ProbeProtocol, gemini_oauth: bool) -> AppResult<()> {
    let mut stream = crate::gateway::ProbeStream::new(protocol);
    let mut buffer = Vec::new();
    let mut received = 0usize;
    while let Some(chunk) = response.chunk().await.map_err(|error|
        if error.is_timeout() { "PROBE_TIMEOUT" } else { "PROBE_READ" })? {
        received += chunk.len();
        if received > PROBE_RESPONSE_BODY_LIMIT { return Err("PROBE_TOO_LARGE".into()); }
        buffer.extend_from_slice(&chunk);
        while let Some(end) = crate::gateway::next_frame_end(&buffer) {
            if gemini_oauth {
                crate::gateway::probe_gemini_oauth_frame(&mut stream, &buffer[..end])?;
            } else {
                stream.frame(&buffer[..end])?;
            }
            buffer.drain(..end);
        }
        if stream.terminal() {
            return Ok(());
        }
    }
    Err("PROBE_UNFINISHED".into())
}

pub fn is_valid_availability_hours(hours: u32) -> bool {
    matches!(hours, 3 | 6 | 12)
}

pub fn normalized_availability_hours(hours: u32) -> u32 {
    if is_valid_availability_hours(hours) {
        hours
    } else {
        crate::settings::DEFAULT_PROVIDER_AVAILABILITY_HOURS
    }
}

fn is_request_level_abort(error_code: Option<&str>) -> bool {
    matches!(
        error_code,
        Some(
            "GW_REQUEST_ABORTED"
                | "GW_STREAM_ABORTED"
                | "GW_REQUEST_INTERRUPTED_BY_RESTART"
                | "GW_REQUEST_INTERRUPTED_BY_GATEWAY_STOP"
        )
    )
}

fn is_provider_attributed_failure(attempt: &AvailabilityAttempt) -> bool {
    if attempt
        .outcome
        .starts_with("cx2cc_event_stream_aggregate_error:")
    {
        return false;
    }
    matches!(
        attempt.error_category.as_deref(),
        Some("SYSTEM_ERROR" | "PROVIDER_ERROR" | "RESOURCE_NOT_FOUND")
    )
}

fn observations_from_attempts(
    trace_id: &str,
    cli_key: &str,
    observed_at_ms: i64,
    request_error_code: Option<&str>,
    attempts_json: &str,
) -> Vec<AvailabilityObservation> {
    if is_request_level_abort(request_error_code) {
        return Vec::new();
    }
    let attempts =
        serde_json::from_str::<Vec<AvailabilityAttempt>>(attempts_json).unwrap_or_default();
    let mut outcomes = HashMap::<i64, bool>::new();
    for attempt in attempts {
        if attempt.provider_id <= 0 || !attempt.upstream_sent {
            continue;
        }
        if attempt.outcome == "success" {
            outcomes.insert(attempt.provider_id, true);
        } else if is_provider_attributed_failure(&attempt) {
            outcomes.entry(attempt.provider_id).or_insert(false);
        }
    }
    outcomes
        .into_iter()
        .map(|(provider_id, success)| AvailabilityObservation {
            trace_id: trace_id.to_string(),
            cli_key: cli_key.to_string(),
            provider_id,
            observed_at_ms: observed_at_ms.max(0),
            success,
        })
        .collect()
}

/// Projects terminal request attempts inside the request-log transaction so
/// both records become visible together. Failures are diagnostic-only.
pub(crate) fn record_request_observations_best_effort(
    tx: &rusqlite::Transaction<'_>,
    items: &[crate::request_logs::RequestLogInsert],
) {
    let observations = items
        .iter()
        .flat_map(|item| {
            let observed_at_ms = if item.created_at_ms > 0 {
                item.created_at_ms
            } else {
                item.created_at.saturating_mul(1_000)
            };
            observations_from_attempts(
                &item.trace_id,
                &item.cli_key,
                observed_at_ms,
                item.error_code.as_deref(),
                &item.attempts_json,
            )
        })
        .collect::<Vec<_>>();
    if observations.is_empty() {
        return;
    }

    let result = (|| -> AppResult<()> {
        {
            let mut statement = tx
                .prepare_cached(
                    r#"
INSERT INTO provider_availability_observations(
  trace_id, cli_key, provider_id, observed_at_ms, success
)
SELECT ?1, ?2, ?3, ?4, ?5
WHERE EXISTS (SELECT 1 FROM providers WHERE id = ?3)
ON CONFLICT(trace_id, provider_id) DO UPDATE SET
  cli_key = excluded.cli_key,
  observed_at_ms = excluded.observed_at_ms,
  success = CASE
    WHEN provider_availability_observations.success = 1 OR excluded.success = 1 THEN 1
    ELSE 0
  END
"#,
                )
                .map_err(|error| db_err!("failed to prepare availability projection: {error}"))?;
            for observation in observations {
                statement
                    .execute(params![
                        observation.trace_id,
                        observation.cli_key,
                        observation.provider_id,
                        observation.observed_at_ms,
                        i64::from(observation.success),
                    ])
                    .map_err(|error| db_err!("failed to write availability fact: {error}"))?;
            }
        }
        Ok(())
    })();
    if let Err(error) = result {
        tracing::warn!(
            error = %error.code(),
            "provider availability observation projection failed"
        );
    }
}

/// Persists one completed manual or scheduled probe as a normal availability
/// fact. The runtime owns generation checks; this function owns the durable,
/// provider-backed projection boundary and deliberately has no error/detail
/// parameters to keep probe diagnostics out of the status timeline.
pub(crate) fn record_probe_observation(
    db: &db::Db,
    trace_id: &str,
    provider_id: i64,
    observed_at_ms: i64,
    success: bool,
) -> AppResult<()> {
    if provider_id <= 0 || trace_id.trim().is_empty() {
        return Err("SEC_INVALID_INPUT: invalid provider availability observation".into());
    }
    let conn = db.open_connection()?;
    conn.execute(
        r#"
INSERT INTO provider_availability_observations(
  trace_id, cli_key, provider_id, observed_at_ms, success
)
SELECT ?1, p.cli_key, p.id, ?3, ?4
FROM providers p
WHERE p.id = ?2
ON CONFLICT(trace_id, provider_id) DO NOTHING
"#,
        params![
            trace_id,
            provider_id,
            observed_at_ms.max(0),
            i64::from(success),
        ],
    )
    .map_err(|error| db_err!("failed to write provider probe availability fact: {error}"))?;
    Ok(())
}

fn bucket_state(success_count: u32, failure_count: u32) -> ProviderAvailabilityState {
    let success_count = u64::from(success_count);
    let total = success_count.saturating_add(u64::from(failure_count));
    if total == 0 {
        ProviderAvailabilityState::NoData
    } else if success_count.saturating_mul(100) < total.saturating_mul(50) {
        ProviderAvailabilityState::Unhealthy
    } else if success_count.saturating_mul(100) < total.saturating_mul(90) {
        ProviderAvailabilityState::Degraded
    } else {
        ProviderAvailabilityState::Healthy
    }
}

pub fn timelines(
    db: &db::Db,
    provider_ids: &[i64],
    hours: u32,
    bucket_count: u16,
    now_ms: i64,
) -> AppResult<Vec<ProviderAvailabilityTimeline>> {
    let hours = normalized_availability_hours(hours);
    if !matches!(
        bucket_count,
        TUI_PROVIDER_AVAILABILITY_BUCKETS
            | TRAY_PROVIDER_AVAILABILITY_BUCKETS
            | DESKTOP_PROVIDER_AVAILABILITY_BUCKETS
    ) {
        return Err("SEC_INVALID_INPUT: bucket_count must be 12, 18, or 36".into());
    }
    let mut seen = HashSet::new();
    let provider_ids = provider_ids
        .iter()
        .copied()
        .filter(|provider_id| *provider_id > 0 && seen.insert(*provider_id))
        .take(512)
        .collect::<Vec<_>>();
    if provider_ids.is_empty() {
        return Ok(Vec::new());
    }

    let bucket_count_i64 = i64::from(bucket_count);
    let range_ms = i64::from(hours).saturating_mul(60 * 60 * 1_000);
    let bucket_ms = range_ms
        .checked_div(bucket_count_i64)
        .ok_or_else(|| "SEC_INVALID_INPUT: invalid availability range".to_string())?;
    let now_ms = now_ms.max(0);
    let end_at_ms = now_ms
        .div_euclid(bucket_ms)
        .saturating_add(1)
        .saturating_mul(bucket_ms);
    let start_at_ms = end_at_ms.saturating_sub(bucket_ms.saturating_mul(bucket_count_i64));
    let current_period_start_ms = end_at_ms.saturating_sub(bucket_ms);

    let empty_buckets = || {
        (0..bucket_count_i64)
            .map(|index| {
                let start = start_at_ms.saturating_add(index.saturating_mul(bucket_ms));
                ProviderAvailabilityBucket {
                    start_at_ms: start,
                    end_at_ms: start.saturating_add(bucket_ms),
                    success_count: 0,
                    failure_count: 0,
                    state: ProviderAvailabilityState::NoData,
                }
            })
            .collect::<Vec<_>>()
    };
    let mut output = provider_ids
        .iter()
        .map(|provider_id| ProviderAvailabilityTimeline {
            provider_id: *provider_id,
            hours,
            bucket_count,
            bucket_minutes: u32::try_from(bucket_ms / 60_000).unwrap_or_default(),
            success_count: 0,
            failure_count: 0,
            buckets: empty_buckets(),
        })
        .collect::<Vec<_>>();
    let positions = output
        .iter()
        .enumerate()
        .map(|(index, timeline)| (timeline.provider_id, index))
        .collect::<HashMap<_, _>>();
    let mut current_last_success = vec![None; output.len()];

    let placeholders = std::iter::repeat_n("?", provider_ids.len())
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        r#"
SELECT provider_id, observed_at_ms, success, rowid
FROM provider_availability_observations
WHERE observed_at_ms >= ?
  AND observed_at_ms < ?
  AND provider_id IN ({placeholders})
ORDER BY observed_at_ms ASC, success DESC, rowid ASC
"#
    );
    let mut values = Vec::<rusqlite::types::Value>::with_capacity(provider_ids.len() + 2);
    values.push(start_at_ms.into());
    values.push(end_at_ms.into());
    values.extend(provider_ids.iter().copied().map(Into::into));
    let conn = db.open_connection()?;
    let mut statement = conn
        .prepare(&sql)
        .map_err(|error| db_err!("failed to prepare availability timeline: {error}"))?;
    let rows = statement
        .query_map(params_from_iter(values), |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)? != 0,
                row.get::<_, i64>(3)?,
            ))
        })
        .map_err(|error| db_err!("failed to query availability timeline: {error}"))?;
    for row in rows {
        let (provider_id, observed_at_ms, success, _rowid) =
            row.map_err(|error| db_err!("failed to read availability timeline row: {error}"))?;
        let Some(position) = positions.get(&provider_id).copied() else {
            continue;
        };
        let bucket_index = observed_at_ms
            .saturating_sub(start_at_ms)
            .div_euclid(bucket_ms);
        let Ok(bucket_index) = usize::try_from(bucket_index) else {
            continue;
        };
        let timeline = &mut output[position];
        let Some(bucket) = timeline.buckets.get_mut(bucket_index) else {
            continue;
        };
        if success {
            bucket.success_count = bucket.success_count.saturating_add(1);
            timeline.success_count = timeline.success_count.saturating_add(1);
        } else {
            bucket.failure_count = bucket.failure_count.saturating_add(1);
            timeline.failure_count = timeline.failure_count.saturating_add(1);
        }
        if observed_at_ms >= current_period_start_ms {
            current_last_success[position] = Some(success);
        }
    }
    for (position, timeline) in output.iter_mut().enumerate() {
        for bucket in &mut timeline.buckets {
            bucket.state = bucket_state(bucket.success_count, bucket.failure_count);
        }
        if let Some(current) = timeline.buckets.last_mut() {
            current.state = match current_last_success[position] {
                Some(true) if current.state == ProviderAvailabilityState::Healthy => {
                    ProviderAvailabilityState::Healthy
                }
                Some(true) => ProviderAvailabilityState::Degraded,
                Some(false) => ProviderAvailabilityState::Unhealthy,
                None => ProviderAvailabilityState::NoData,
            };
        }
    }
    Ok(output)
}

pub fn purge_expired_observations(db: &db::Db, now_ms: i64) -> AppResult<u64> {
    let cutoff = now_ms.max(0).saturating_sub(AVAILABILITY_RETENTION_MS);
    let mut deleted = 0_u64;
    loop {
        let mut conn = db.open_connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| db_err!("failed to begin availability retention: {error}"))?;
        let affected = tx
            .execute(
                r#"
DELETE FROM provider_availability_observations
WHERE rowid IN (
  SELECT rowid
  FROM provider_availability_observations
  WHERE observed_at_ms < ?1
  ORDER BY observed_at_ms ASC
  LIMIT ?2
)
"#,
                params![cutoff, AVAILABILITY_RETENTION_BATCH_SIZE as i64],
            )
            .map_err(|error| db_err!("failed to purge availability facts: {error}"))?;
        tx.commit()
            .map_err(|error| db_err!("failed to commit availability retention: {error}"))?;
        deleted = deleted.saturating_add(affected as u64);
        if affected < AVAILABILITY_RETENTION_BATCH_SIZE {
            break;
        }
        std::thread::yield_now();
    }
    Ok(deleted)
}

pub(crate) fn spawn_retention_task(db: db::Db) {
    tauri::async_runtime::spawn(async move {
        run_retention_once(db.clone()).await;
        let mut interval = tokio::time::interval(AVAILABILITY_RETENTION_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await;
        loop {
            interval.tick().await;
            run_retention_once(db.clone()).await;
        }
    });
}

async fn run_retention_once(db: db::Db) {
    let result = blocking::run("provider_availability_retention", move || {
        purge_expired_observations(&db, crate::shared::time::now_unix_millis())
    })
    .await;
    match result {
        Ok(deleted) if deleted > 0 => {
            tracing::info!(deleted, "purged expired provider availability observations");
        }
        Ok(_) => {}
        Err(error) => {
            tracing::warn!(
                error = %error.code(),
                "provider availability retention task failed"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::{
        upsert, DailyResetMode, ProviderAuthMode, ProviderBaseUrlMode, ProviderUpsertParams,
        CODEX_TO_ANTHROPIC_MESSAGES_BRIDGE_TYPE, CODEX_TO_OPENAI_CHAT_BRIDGE_TYPE,
        CODEX_TO_OPENAI_RESPONSES_BRIDGE_TYPE,
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn request_observations_merge_retries_and_keep_failover_providers() {
        let attempts = serde_json::json!([
            {"provider_id": 1, "outcome": "request_error", "error_category": "SYSTEM_ERROR", "upstream_sent": true},
            {"provider_id": 1, "outcome": "upstream_error", "error_category": "PROVIDER_ERROR", "upstream_sent": true},
            {"provider_id": 2, "outcome": "success", "upstream_sent": true},
            {"provider_id": 2, "outcome": "skipped", "error_category": "PROVIDER_ERROR", "upstream_sent": false}
        ]);

        let mut observations =
            observations_from_attempts("trace", "codex", 1_000, None, &attempts.to_string());
        observations.sort_by_key(|item| item.provider_id);

        assert_eq!(observations.len(), 2);
        assert_eq!(
            (observations[0].provider_id, observations[0].success),
            (1, false)
        );
        assert_eq!(
            (observations[1].provider_id, observations[1].success),
            (2, true)
        );
    }

    #[test]
    fn request_observations_prefer_eventual_success_and_ignore_local_failures() {
        let attempts = serde_json::json!([
            {"provider_id": 1, "outcome": "request_error", "error_category": "SYSTEM_ERROR", "upstream_sent": true},
            {"provider_id": 1, "outcome": "success", "upstream_sent": true},
            {"provider_id": 2, "outcome": "managed_model_invalid", "error_category": "NON_RETRYABLE_CLIENT_ERROR", "upstream_sent": false},
            {"provider_id": 3, "outcome": "bridge_response_translate_error", "error_category": "NON_RETRYABLE_CLIENT_ERROR", "upstream_sent": true}
        ]);

        let observations =
            observations_from_attempts("trace", "codex", 1_000, None, &attempts.to_string());

        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].provider_id, 1);
        assert!(observations[0].success);
        assert!(observations_from_attempts(
            "trace",
            "codex",
            1_000,
            Some("GW_REQUEST_ABORTED"),
            &attempts.to_string(),
        )
        .is_empty());
    }

    #[test]
    fn probe_observations_are_idempotent_provider_backed_and_secret_free() {
        let temp = tempfile::tempdir().expect("tempdir");
        let db = crate::db::init_for_tests(&temp.path().join("probe-observations.sqlite3"))
            .expect("init db");
        let provider =
            upsert(&db, default_provider_params("probe-provider")).expect("insert provider");

        record_probe_observation(&db, "availability-probe:success", provider.id, 1_000, true)
            .expect("record successful probe");
        record_probe_observation(&db, "availability-probe:failure", provider.id, 2_000, false)
            .expect("record failed probe");
        record_probe_observation(&db, "availability-probe:success", provider.id, 9_999, false)
            .expect("duplicate probe remains idempotent");

        let conn = db.open_connection().expect("open db");
        let success = conn
            .query_row(
                "SELECT cli_key, observed_at_ms, success FROM provider_availability_observations WHERE trace_id = ?1 AND provider_id = ?2",
                params!["availability-probe:success", provider.id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?)),
            )
            .expect("read successful probe");
        let failure: i64 = conn
            .query_row(
                "SELECT success FROM provider_availability_observations WHERE trace_id = ?1 AND provider_id = ?2",
                params!["availability-probe:failure", provider.id],
                |row| row.get(0),
            )
            .expect("read failed probe");
        assert_eq!(success, ("codex".to_string(), 1_000, 1));
        assert_eq!(failure, 0);

        let mut columns_statement = conn
            .prepare("SELECT name FROM pragma_table_info('provider_availability_observations') ORDER BY cid")
            .expect("prepare observation schema");
        let columns = columns_statement
            .query_map([], |row| row.get::<_, String>(0))
            .expect("read observation schema")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect observation schema");
        assert_eq!(
            columns,
            vec![
                "trace_id",
                "cli_key",
                "provider_id",
                "observed_at_ms",
                "success",
            ],
            "availability facts may not carry probe URLs, credentials, response bodies, or errors"
        );
        drop(columns_statement);
        drop(conn);

        crate::providers::delete(&db, provider.id, false).expect("delete provider");
        record_probe_observation(&db, "availability-probe:deleted", provider.id, 3_000, true)
            .expect("deleted provider is ignored");
        let deleted_rows: i64 = db
            .open_connection()
            .expect("open db")
            .query_row(
                "SELECT COUNT(1) FROM provider_availability_observations WHERE trace_id = 'availability-probe:deleted'",
                [],
                |row| row.get(0),
            )
            .expect("count deleted provider observations");
        assert_eq!(deleted_rows, 0);
    }

    #[test]
    fn availability_state_uses_fifty_and_ninety_percent_boundaries() {
        assert_eq!(bucket_state(0, 0), ProviderAvailabilityState::NoData);
        assert_eq!(bucket_state(49, 51), ProviderAvailabilityState::Unhealthy);
        assert_eq!(bucket_state(1, 1), ProviderAvailabilityState::Degraded);
        assert_eq!(bucket_state(89, 11), ProviderAvailabilityState::Degraded);
        assert_eq!(bucket_state(9, 1), ProviderAvailabilityState::Healthy);
    }

    fn default_provider_params(name: &str) -> ProviderUpsertParams {
        ProviderUpsertParams {
            provider_id: None,
            cli_key: "codex".to_string(),
            name: name.to_string(),
            base_urls: vec!["https://api.example.com/v1".to_string()],
            base_url_mode: ProviderBaseUrlMode::Order,
            auth_mode: Some(ProviderAuthMode::ApiKey),
            api_key: Some("sk-test".to_string()),
            enabled: true,
            cost_multiplier: 1.0,
            priority: Some(100),
            claude_models: None,
            availability_test_model: None,
            availability_probe_enabled: false,
            availability_probe_interval_minutes: 10,
            limit_5h_usd: None,
            limit_daily_usd: None,
            daily_reset_mode: Some(DailyResetMode::Fixed),
            daily_reset_time: Some("00:00:00".to_string()),
            limit_weekly_usd: None,
            limit_monthly_usd: None,
            limit_total_usd: None,
            tags: None,
            note: None,
            source_provider_id: None,
            bridge_type: None,
            stream_idle_timeout_seconds: None,
            model_mapping: None,
            extension_values: None,
            account_usage_credentials_patch: None,
            account_usage_credentials_copy_from_provider_id: None,
            upstream_retry_policy_override: None,
            upstream_retry_policy_override_specified: false,
            model_routing_policy_override: None,
            model_routing_policy_override_specified: false,
        }
    }

    #[test]
    fn timelines_align_natural_buckets_and_retention_keeps_cutoff() {
        let temp = tempfile::tempdir().expect("tempdir");
        let db_path = temp.path().join("provider-availability-facts.sqlite3");
        let db = crate::db::init_for_tests(&db_path).expect("init db");
        let provider =
            upsert(&db, default_provider_params("timeline-provider")).expect("insert provider");
        let now_ms: i64 = 10 * 60 * 60 * 1_000 + 17 * 60 * 1_000;
        let tui_bucket_ms = 30 * 60 * 1_000;
        let tray_bucket_ms = 20 * 60 * 1_000;
        let desktop_bucket_ms = 10 * 60 * 1_000;
        let current_bucket_start = now_ms.div_euclid(tui_bucket_ms) * tui_bucket_ms;
        let tray_current_bucket_start = now_ms.div_euclid(tray_bucket_ms) * tray_bucket_ms;
        let desktop_current_bucket_start = now_ms.div_euclid(desktop_bucket_ms) * desktop_bucket_ms;
        let cutoff = now_ms - AVAILABILITY_RETENTION_MS;
        let conn = db.open_connection().expect("open db");
        for (trace, observed_at_ms, success) in [
            ("success-1", current_bucket_start + 1, 1_i64),
            ("success-2", current_bucket_start + 2, 1),
            ("success-3", current_bucket_start + 3, 1),
            ("failure-1", current_bucket_start + 4, 0),
            (
                "desktop-current-success",
                desktop_current_bucket_start + 1,
                1,
            ),
            ("expired", cutoff - 1, 0),
            ("at-cutoff", cutoff, 1),
        ] {
            conn.execute(
                "INSERT INTO provider_availability_observations(trace_id, cli_key, provider_id, observed_at_ms, success) VALUES (?1, 'codex', ?2, ?3, ?4)",
                params![trace, provider.id, observed_at_ms, success],
            )
            .expect("insert observation");
        }
        drop(conn);

        let timeline = timelines(&db, &[provider.id], 6, 12, now_ms)
            .expect("load timeline")
            .pop()
            .expect("provider timeline");
        let desktop_timeline = timelines(&db, &[provider.id], 6, 36, now_ms)
            .expect("load desktop timeline")
            .pop()
            .expect("desktop provider timeline");
        let tray_timeline = timelines(&db, &[provider.id], 6, 18, now_ms)
            .expect("load tray timeline")
            .pop()
            .expect("tray provider timeline");
        let current = timeline.buckets.last().expect("current bucket");
        assert_eq!(timeline.bucket_minutes, 30);
        assert_eq!(tray_timeline.bucket_minutes, 20);
        assert_eq!(desktop_timeline.bucket_minutes, 10);
        assert_eq!(current.start_at_ms, current_bucket_start);
        assert_eq!(
            tray_timeline
                .buckets
                .last()
                .expect("tray current bucket")
                .start_at_ms,
            tray_current_bucket_start
        );
        assert_eq!(
            desktop_timeline
                .buckets
                .last()
                .expect("desktop current bucket")
                .start_at_ms,
            desktop_current_bucket_start
        );
        assert_eq!((current.success_count, current.failure_count), (4, 1));
        assert_eq!(current.state, ProviderAvailabilityState::Degraded);
        assert_eq!(
            desktop_timeline.buckets.last().map(|bucket| (
                bucket.success_count,
                bucket.failure_count,
                bucket.state
            )),
            Some((1, 0, ProviderAvailabilityState::Healthy))
        );
        assert_eq!(
            tray_timeline.buckets.last().map(|bucket| (
                bucket.success_count,
                bucket.failure_count,
                bucket.state
            )),
            Some((4, 1, ProviderAvailabilityState::Degraded))
        );
        assert_eq!(
            desktop_timeline
                .buckets
                .iter()
                .find(|bucket| bucket.start_at_ms == current_bucket_start)
                .map(|bucket| (bucket.success_count, bucket.failure_count, bucket.state)),
            Some((3, 1, ProviderAvailabilityState::Degraded))
        );

        let rolled_now_ms = 10 * 60 * 60 * 1_000 + 31 * 60 * 1_000;
        for (bucket_count, expected_current_start) in [
            (12, 10 * 60 * 60 * 1_000 + 30 * 60 * 1_000),
            (18, 10 * 60 * 60 * 1_000 + 20 * 60 * 1_000),
            (36, 10 * 60 * 60 * 1_000 + 30 * 60 * 1_000),
        ] {
            let rolled_timeline = timelines(&db, &[provider.id], 6, bucket_count, rolled_now_ms)
                .expect("load rolled timeline")
                .pop()
                .expect("rolled provider timeline");
            let rolled_historical = rolled_timeline
                .buckets
                .iter()
                .find(|bucket| bucket.start_at_ms == current_bucket_start)
                .expect("previous current period becomes historical");
            assert_eq!(
                rolled_historical.state,
                ProviderAvailabilityState::Degraded,
                "historical state must use the success ratio for {bucket_count} buckets"
            );
            let rolled_current = rolled_timeline.buckets.last().expect("new current bucket");
            assert_eq!(rolled_current.start_at_ms, expected_current_start);
            assert_eq!(
                rolled_current.state,
                ProviderAvailabilityState::NoData,
                "new current state must stay empty for {bucket_count} buckets"
            );
        }

        assert_eq!(purge_expired_observations(&db, now_ms).expect("purge"), 1);
        let remaining: i64 = db
            .open_connection()
            .expect("open db")
            .query_row(
                "SELECT COUNT(1) FROM provider_availability_observations WHERE trace_id = 'at-cutoff'",
                [],
                |row| row.get(0),
            )
            .expect("count cutoff row");
        assert_eq!(remaining, 1);
    }

    #[test]
    fn current_status_cell_uses_degraded_when_last_observation_succeeds_below_ninety_percent() {
        let temp = tempfile::tempdir().expect("tempdir");
        let db_path = temp
            .path()
            .join("provider-availability-last-success.sqlite3");
        let db = crate::db::init_for_tests(&db_path).expect("init db");
        let provider =
            upsert(&db, default_provider_params("last-success")).expect("insert provider");
        let now_ms: i64 = 10 * 60 * 60 * 1_000 + 17 * 60 * 1_000;
        let current_period_start = 10 * 60 * 60 * 1_000 + 10 * 60 * 1_000;
        let conn = db.open_connection().expect("open db");
        for (trace, observed_at_ms, success) in [
            ("failure-1", current_period_start + 1, 0_i64),
            ("failure-2", current_period_start + 2, 0),
            ("failure-3", current_period_start + 3, 0),
            ("success-last", current_period_start + 4, 1),
        ] {
            conn.execute(
                "INSERT INTO provider_availability_observations(trace_id, cli_key, provider_id, observed_at_ms, success) VALUES (?1, 'codex', ?2, ?3, ?4)",
                params![trace, provider.id, observed_at_ms, success],
            )
            .expect("insert observation");
        }
        drop(conn);

        for bucket_count in [12, 18, 36] {
            let timeline = timelines(&db, &[provider.id], 6, bucket_count, now_ms)
                .expect("load timeline")
                .pop()
                .expect("provider timeline");
            assert_eq!(
                timeline.buckets.last().expect("current status cell").state,
                ProviderAvailabilityState::Degraded,
                "the last successful observation must produce degraded below 90% for {bucket_count} buckets"
            );
        }
    }

    #[test]
    fn current_status_cell_prefers_failure_for_equal_timestamp_ties() {
        let temp = tempfile::tempdir().expect("tempdir");
        let db_path = temp.path().join("provider-availability-rowid.sqlite3");
        let db = crate::db::init_for_tests(&db_path).expect("init db");
        let success_last =
            upsert(&db, default_provider_params("success-last")).expect("insert provider");
        let failure_last =
            upsert(&db, default_provider_params("failure-last")).expect("insert provider");
        let now_ms: i64 = 10 * 60 * 60 * 1_000 + 17 * 60 * 1_000;
        let observed_at_ms = 10 * 60 * 60 * 1_000 + 10 * 60 * 1_000 + 1;
        let conn = db.open_connection().expect("open db");
        for (trace, provider_id, success) in [
            ("success-last-failure", success_last.id, 0_i64),
            ("success-last-success", success_last.id, 1),
            ("failure-last-success", failure_last.id, 1),
            ("failure-last-failure", failure_last.id, 0),
        ] {
            conn.execute(
                "INSERT INTO provider_availability_observations(trace_id, cli_key, provider_id, observed_at_ms, success) VALUES (?1, 'codex', ?2, ?3, ?4)",
                params![trace, provider_id, observed_at_ms, success],
            )
            .expect("insert tied observation");
        }
        drop(conn);

        for bucket_count in [12, 18, 36] {
            let timelines = timelines(
                &db,
                &[success_last.id, failure_last.id],
                6,
                bucket_count,
                now_ms,
            )
            .expect("load timelines");
            let state_for = |provider_id| {
                timelines
                    .iter()
                    .find(|timeline| timeline.provider_id == provider_id)
                    .and_then(|timeline| timeline.buckets.last())
                    .map(|bucket| bucket.state)
                    .expect("current status")
            };
            assert_eq!(
                state_for(success_last.id),
                ProviderAvailabilityState::Unhealthy,
                "failure must win equal timestamps for {bucket_count} buckets"
            );
            assert_eq!(
                state_for(failure_last.id),
                ProviderAvailabilityState::Unhealthy,
                "failure must win equal timestamps for {bucket_count} buckets"
            );
        }
    }

    async fn response_from_request_capture(
        expected_path: &'static str,
        response_status: u16,
        response_body: &'static str,
    ) -> (String, tokio::task::JoinHandle<String>) {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("bind test server");
        let addr = listener.local_addr().expect("test server addr");
        let task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept request");
            let mut buf = Vec::new();
            let mut chunk = [0_u8; 1024];
            loop {
                let read = stream.read(&mut chunk).await.expect("read request");
                if read == 0 {
                    break;
                }
                buf.extend_from_slice(&chunk[..read]);
                if buf.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            if let Some(header_end) = buf.windows(4).position(|window| window == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&buf[..header_end]).to_ascii_lowercase();
                let content_length = headers
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length:"))
                    .and_then(|value| value.trim().parse::<usize>().ok())
                    .unwrap_or_default();
                let body_start = header_end + 4;
                while buf.len().saturating_sub(body_start) < content_length {
                    let read = stream.read(&mut chunk).await.expect("read request body");
                    if read == 0 {
                        break;
                    }
                    buf.extend_from_slice(&chunk[..read]);
                }
            }

            let request = String::from_utf8_lossy(&buf).to_string();
            assert!(
                request.contains(&format!("POST {expected_path} ")),
                "unexpected request path: {request}"
            );

            let response = format!(
                "HTTP/1.1 {response_status} OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            stream
                .write_all(response.as_bytes())
                .await
                .expect("write response");
            let _ = stream.shutdown().await;
            request
        });

        (format!("http://{addr}"), task)
    }

    fn header_value(headers: &HeaderMap, key: &str) -> String {
        headers
            .get(key)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string()
    }

    #[test]
    fn build_probe_request_for_claude_uses_messages_endpoint_and_x_api_key() {
        let (url, headers, body) = build_probe_request(
            "claude",
            "https://api.example.com/",
            "sk-claude",
            None,
            None,
        )
        .expect("claude request");

        assert_eq!(url, "https://api.example.com/v1/messages");
        assert_eq!(header_value(&headers, "x-api-key"), "sk-claude");
        assert_eq!(header_value(&headers, "anthropic-version"), "2023-06-01");
        assert_eq!(body["model"], "claude-sonnet-4-6");
        assert_eq!(body["messages"][0]["content"], PROBE_PROMPT);
        assert_eq!(body["max_tokens"], 100);
    }

    #[test]
    fn build_probe_request_for_claude_uses_model_override() {
        let (_, _, body) = build_probe_request(
            "claude",
            "https://api.example.com/",
            "sk-claude",
            Some("claude-test-model"),
            None,
        )
        .expect("claude request");

        assert_eq!(body["model"], "claude-test-model");
    }

    #[test]
    fn build_probe_request_for_codex_uses_responses_and_bearer_auth() {
        let (url, headers, body) = build_probe_request(
            "codex",
            "https://api.example.com",
            "sk-openai",
            Some("gpt-test"),
            None,
        )
        .expect("codex request");

        assert_eq!(url, "https://api.example.com/v1/responses");
        assert_eq!(header_value(&headers, "authorization"), "Bearer sk-openai");
        assert_eq!(body["input"][0]["content"], PROBE_PROMPT);
        assert_eq!(body["max_output_tokens"], 100);
        assert_eq!(body["model"], "gpt-test");
    }

    #[test]
    fn build_probe_request_for_grok_uses_effective_responses_model_and_bearer_auth() {
        let preferences = crate::grok_config::GrokProxyPreferences {
            model_id: "grok-responses-custom".to_string(),
            api_backend: crate::grok_config::GrokApiBackend::Responses,
            ..Default::default()
        };
        let (url, headers, body) = build_probe_request(
            "grok",
            "https://api.example.com/",
            "test-grok-key",
            None,
            Some(&preferences),
        )
        .expect("Grok request");

        assert_eq!(url, "https://api.example.com/v1/responses");
        assert_eq!(
            header_value(&headers, "authorization"),
            "Bearer test-grok-key"
        );
        assert_eq!(body["model"], preferences.model_id);
        assert_eq!(body["input"], PROBE_PROMPT);
        assert_eq!(body["max_output_tokens"], 100);
        assert_eq!(body["store"], false);
        assert_eq!(body["stream"], false);
    }

    #[test]
    fn build_probe_request_for_grok_uses_effective_chat_completions_model_and_body() {
        let preferences = crate::grok_config::GrokProxyPreferences {
            model_id: "grok-chat-custom".to_string(),
            api_backend: crate::grok_config::GrokApiBackend::ChatCompletions,
            ..Default::default()
        };

        let (url, headers, body) = build_probe_request(
            "grok",
            "https://api.example.com/v1",
            "test-grok-key",
            None,
            Some(&preferences),
        )
        .expect("Grok Chat request");

        assert_eq!(url, "https://api.example.com/v1/chat/completions");
        assert_eq!(
            header_value(&headers, "authorization"),
            "Bearer test-grok-key"
        );
        assert_eq!(body["model"], preferences.model_id);
        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["messages"][0]["content"], PROBE_PROMPT);
        assert_eq!(body["max_tokens"], 100);
        assert_eq!(body["stream"], false);
    }

    #[test]
    fn build_probe_request_deduplicates_versioned_base_paths_for_all_clis() {
        let cases = [
            (
                "claude",
                "https://api.example.com/v1/",
                "https://api.example.com/v1/messages",
            ),
            (
                "codex",
                "https://api.example.com/v1",
                "https://api.example.com/v1/responses",
            ),
            (
                "grok",
                "https://api.example.com/v1/",
                "https://api.example.com/v1/responses",
            ),
            (
                "gemini",
                "https://api.example.com/v1beta/",
                "https://api.example.com/v1beta/models/gemini-2.0-flash:generateContent?key=test-key",
            ),
        ];

        for (cli_key, base_url, expected_url) in cases {
            let (url, _, _) = build_probe_request(cli_key, base_url, "test-key", None, None)
                .unwrap_or_else(|err| panic!("{cli_key} probe request failed: {err}"));

            assert_eq!(url, expected_url, "unexpected {cli_key} probe URL");
        }
    }

    #[test]
    fn build_probe_request_for_gemini_uses_generate_content_key_param() {
        let (url, headers, body) = build_probe_request(
            "gemini",
            "https://generativelanguage.googleapis.com/",
            "sk-google",
            None,
            None,
        )
        .expect("gemini request");

        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key=sk-google"
        );
        assert_eq!(header_value(&headers, "content-type"), "application/json");
        assert_eq!(body["contents"][0]["parts"][0]["text"], PROBE_PROMPT);
        assert_eq!(body["generationConfig"]["maxOutputTokens"], 100);
    }

    #[test]
    fn build_probe_request_rejects_unsupported_cli_key() {
        let err = build_probe_request("unknown", "https://api.example.com", "secret", None, None)
            .unwrap_err()
            .to_string();

        assert_eq!(err, "UNSUPPORTED_CLI_KEY: unknown");
    }

    #[test]
    fn resolve_codex_probe_model_from_sources_prefers_provider_override_then_global_then_default() {
        assert_eq!(
            resolve_codex_probe_model_from_sources(Some("gpt-provider"), Some("gpt-global")),
            "gpt-provider"
        );
        assert_eq!(
            resolve_codex_probe_model_from_sources(Some("   "), Some("gpt-global")),
            "gpt-global"
        );
        assert_eq!(
            resolve_codex_probe_model_from_sources(None, Some("   ")),
            crate::settings::DEFAULT_CODEX_PROVIDER_TEST_MODEL
        );
    }

    #[test]
    fn redact_key_param_preserves_delimiters_and_hides_gemini_key() {
        let redacted =
            redact_key_param("连接失败: https://host/v1beta/models?alt=sse&key=sk-secret&other=1");

        assert_eq!(
            redacted,
            "连接失败: https://host/v1beta/models?alt=sse&key=***&other=1"
        );
        assert!(!redacted.contains("sk-secret"));
    }

    #[test]
    fn probe_output_redacts_an_echoed_effective_credential() {
        let redacted = redact_probe_credential(
            r#"{"error":"credential sk-secret was rejected"}"#,
            "sk-secret",
        );

        assert_eq!(
            redacted,
            r#"{"error":"credential [REDACTED] was rejected"}"#
        );
    }

    #[test]
    fn append_probe_response_chunk_keeps_bounded_prefix() {
        let mut bytes = b"abcd".to_vec();
        let truncated = append_probe_response_chunk(&mut bytes, b"efgh", 6);

        assert_eq!(bytes, b"abcdef");
        assert!(truncated);
    }

    #[test]
    fn probe_response_preview_marks_truncated_payloads() {
        let preview = probe_response_preview(&ProbeResponseBody {
            bytes: b"upstream error".to_vec(),
            truncated: true,
            limit: 12,
        });

        assert_eq!(
            preview,
            "upstream error\n[probe response truncated after 12 bytes]"
        );
    }

    #[tokio::test]
    async fn probe_rejects_http_errors_even_with_an_answer_body() {
        for status in [400, 401, 403, 404, 429, 500, 503] {
            let temp = tempfile::tempdir().unwrap();
            let db = crate::db::init_for_tests(&temp.path().join("status.db")).unwrap();
            let (url, server) = response_from_request_capture("/v1/responses", status,
                r#"{"object":"response","status":"completed","output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":"OK"}]}]}"#).await;
            let mut params = default_provider_params("status");
            params.base_urls = vec![url];
            params.availability_test_model = Some("selected-model".into());
            let provider = upsert(&db, params).unwrap();
            let app = tauri::test::mock_app();
            let result = test_provider_availability(app.handle(), db, provider.id).await.unwrap();
            assert!(!result.ok, "status {status}");
            assert_eq!(result.status, Some(status));
            assert_eq!(result.tested_model.as_deref(), Some("selected-model"));
            let expected = match status {
                401 | 403 => "PROBE_AUTH: 认证失败",
                400 | 404 | 429 => "PROBE_MODEL_QUOTA: 上游拒绝请求",
                _ => "PROBE_HTTP: 上游 HTTP 请求失败",
            };
            assert!(result.error.as_deref().unwrap().starts_with(expected));
            server.await.unwrap();
        }
    }

    #[tokio::test]
    async fn disabled_provider_generates_one_selected_model_without_routing() {
        let temp = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&temp.path().join("success.db")).unwrap();
        let (url, server) = response_from_request_capture("/v1/responses", 200,
            r#"{"object":"response","status":"incomplete","incomplete_details":{"reason":"max_output_tokens"},"output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":"an answer"}]}]}"#).await;
        let mut params = default_provider_params("disabled");
        params.base_urls = vec![url];
        params.availability_test_model = Some("selected-model".into());
        params.enabled = false;
        let provider = upsert(&db, params).unwrap();
        let app = tauri::test::mock_app();
        let result = test_provider_availability(app.handle(), db, provider.id).await.unwrap();
        assert!(result.ok);
        assert_eq!(result.requested_model, result.tested_model);
        let request = server.await.unwrap();
        let body: serde_json::Value = serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap()).unwrap();
        assert_eq!(body["model"], "selected-model");
        assert_eq!(body["input"][0]["content"], PROBE_PROMPT);
        assert_eq!(body["max_output_tokens"], 100);
    }

    #[tokio::test]
    async fn successful_http_does_not_accept_non_answers() {
        for body in [
            "<html>OK</html>", "", "{}", r#"{"error":{"message":"quota"}}"#,
            r#"{"object":"response","status":"completed","output":[]}"#,
            r#"{"object":"response","status":"in_progress","output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":"OK"}]}]}"#,
        ] {
            let temp = tempfile::tempdir().unwrap();
            let db = crate::db::init_for_tests(&temp.path().join("non-answer.db")).unwrap();
            let (url, server) = response_from_request_capture("/v1/responses", 200, body).await;
            let mut params = default_provider_params("non-answer");
            params.base_urls = vec![url];
            params.availability_test_model = Some("selected-model".into());
            let provider = upsert(&db, params).unwrap();
            let app = tauri::test::mock_app();
            let result = test_provider_availability(app.handle(), db, provider.id).await.unwrap();
            assert!(!result.ok, "{body}");
            assert_eq!(result.tested_model.as_deref(), Some("selected-model"));
            assert!(result.error.as_deref().unwrap().starts_with("PROBE_"));
            server.await.unwrap();
        }
    }

    #[test]
    fn probe_failures_expose_fixed_chinese_reasons_without_upstream_details() {
        for (code, reason) in [
            ("PROBE_NO_TEXT", "本次探测未获得有效回答"),
            ("PROBE_UNFINISHED", "未收到完整结束标记"),
            ("PROBE_TERMINATION", "不支持的原因"),
            ("PROBE_READ", "读取上游响应失败"),
            ("PROBE_TOO_LARGE", "64 KiB"),
            ("PROBE_TIMEOUT", "本次探测超时"),
            ("PROBE_HTTP", "HTTP 请求失败"),
            ("PROBE_AUTH", "认证失败"),
            ("PROBE_MODEL_QUOTA", "模型、配额或频率限制"),
            ("PROBE_STRUCTURE", "响应格式"),
            ("PROBE_UPSTREAM_ERROR", "上游返回错误"),
        ] {
            for error in [AppError::from(code.to_string()), AppError::new(code, "SYNTHETIC_PRIVATE_DETAIL")] {
                let display = probe_failure(&error).to_string();
                assert!(display.starts_with(&format!("{code}: ")));
                assert!(display.contains(reason));
                assert!(!display.contains("SYNTHETIC_PRIVATE_DETAIL"));
                assert!(display.chars().count() <= 128);
            }
        }
        let no_text = probe_failure(&AppError::from("PROBE_NO_TEXT")).to_string();
        assert!(no_text.contains("仅推理内容不算回答"));
        assert!(no_text.contains("100 token"));
        assert!(!no_text.contains("永久"));
        assert_eq!(probe_failure(&AppError::new("UPSTREAM_PRIVATE_CODE", "SYNTHETIC_PRIVATE_DETAIL")).code(), "PROBE_PREPARATION");
    }

    #[tokio::test]
    async fn reasoning_only_token_limit_returns_the_chinese_no_answer_reason() {
        let temp = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&temp.path().join("reasoning-only.db")).unwrap();
        let (url, server) = response_from_request_capture("/v1/responses", 200,
            r#"{"object":"response","status":"incomplete","incomplete_details":{"reason":"max_output_tokens"},"output":[{"type":"reasoning","summary":[{"text":"OK"}]}],"usage":{"output_tokens":100}}"#).await;
        let mut params = default_provider_params("reasoning-only");
        params.base_urls = vec![url];
        params.availability_test_model = Some("selected-model".into());
        let provider = upsert(&db, params).unwrap();
        let app = tauri::test::mock_app();
        let result = test_provider_availability(app.handle(), db, provider.id).await.unwrap();
        assert!(!result.ok);
        assert_eq!(result.tested_model.as_deref(), Some("selected-model"));
        assert_eq!(result.error, Some(probe_failure(&AppError::from("PROBE_NO_TEXT")).to_string()));
        let request = server.await.unwrap();
        let body: serde_json::Value = serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap()).unwrap();
        assert_eq!(body["max_output_tokens"], 100);
    }

    #[test]
    fn fixed_bridge_wire_budgets_and_models_match_target_protocols() {
        for (bridge_type, path, budget_key) in [
            (CODEX_TO_OPENAI_RESPONSES_BRIDGE_TYPE, "/v1/responses", "max_output_tokens"),
            (CODEX_TO_OPENAI_CHAT_BRIDGE_TYPE, "/v1/chat/completions", "max_tokens"),
            (CODEX_TO_ANTHROPIC_MESSAGES_BRIDGE_TYPE, "/v1/messages", "max_tokens"),
        ] {
            let mapping = ModelMapping { default_model: Some("wire-model".into()), ..Default::default() };
            let (target, body) = crate::gateway::build_translated_bridge_probe(bridge_type, mapping, "selected-model").unwrap();
            assert_eq!(target, path);
            assert_eq!(body["model"], "wire-model");
            assert_eq!(body[budget_key], 100);
            assert!(body.get("reasoning").is_none());
            assert!(body.get("thinking").is_none());
            assert!(body.get("tools").is_none_or(|tools| tools.as_array().is_some_and(Vec::is_empty)));
            assert_eq!(body["stream"], false);
        }
        let (_, headers, _) = build_probe_request_with_body(
            "grok", "https://example.test", "synthetic", "/v1/chat/completions", serde_json::json!({})
        ).unwrap();
        assert_eq!(header_value(&headers, "authorization"), "Bearer synthetic");
    }

    #[test]
    fn oauth_probe_headers_use_the_existing_provider_adapters() {
        for (cli, kind) in [("claude", "claude_oauth"), ("codex", "codex_oauth"), ("gemini", "gemini_oauth")] {
            let adapter = crate::gateway::oauth::registry::resolve_oauth_adapter(cli, 1, Some(kind)).unwrap();
            let (_, mut headers, body) = build_probe_request(cli, "https://example.test", "", None, None).unwrap();
            headers.remove("x-api-key");
            adapter.inject_upstream_headers(&mut headers, "synthetic-token").unwrap();
            assert_eq!(header_value(&headers, "authorization"), "Bearer synthetic-token");
            assert!(!headers.contains_key("x-api-key"));
            match cli {
                "claude" => {
                    assert!(header_value(&headers, "anthropic-beta").contains("oauth-2025-04-20"));
                    assert_eq!(body["max_tokens"], 100);
                },
                "gemini" => {
                    assert!(!header_value(&headers, "x-goog-api-client").is_empty());
                    assert_eq!(body["generationConfig"]["maxOutputTokens"], 100);
                },
                _ => {
                    let body = crate::gateway::probe_codex_oauth_body(&body);
                    assert_eq!(body["stream"], true);
                    assert_eq!(body["store"], false);
                    assert!(body.get("max_output_tokens").is_none());
                },
            }
        }
        let models = crate::providers::ClaudeModels { sonnet_model: Some("mapped-codex".into()), ..Default::default() };
        assert_eq!(crate::gateway::cx2cc_probe_model("claude-sonnet-4-6", &models, &crate::settings::AppSettings::default()), "mapped-codex");
    }

    #[tokio::test]
    async fn bounded_body_distinguishes_exact_complete_overflow_and_read_failure() {
        for (raw, expected) in [
            ("HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\n4\r\nabcd\r\n0\r\n\r\n", Some(false)),
            ("HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\n5\r\nabcde\r\n0\r\n\r\n", Some(true)),
            ("HTTP/1.1 200 OK\r\ncontent-length: 9\r\n\r\nabcd", None),
        ] {
            let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
            let url = format!("http://{}", listener.local_addr().unwrap());
            let server = tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0u8; 1024];
                socket.read(&mut request).await.unwrap();
                socket.write_all(raw.as_bytes()).await.unwrap();
                socket.shutdown().await.unwrap();
            });
            let response = reqwest::Client::builder().no_proxy().build().unwrap().get(url).send().await.unwrap();
            let body = read_probe_response_body_with_limit(response, 4).await;
            match expected {
                Some(truncated) => {
                    let body = body.unwrap();
                    assert_eq!(body.bytes, b"abcd");
                    assert_eq!(body.truncated, truncated);
                },
                None => assert_eq!(body.unwrap_err(), "PROBE_READ"),
            }
            server.await.unwrap();
        }
    }

    #[tokio::test]
    async fn sse_terminal_returns_before_eof_and_latency_includes_body() {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let (release, released) = tokio::sync::oneshot::channel::<()>();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0u8; 1024];
            socket.read(&mut request).await.unwrap();
            socket.write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\n").await.unwrap();
            tokio::time::sleep(Duration::from_millis(30)).await;
            socket.write_all(b"event: response.completed\ndata: {\"response\":{\"id\":\"resp_1\",\"object\":\"response\",\"status\":\"completed\",\"output\":[{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"OK\"}]}]}}\n\ndata: [DO").await.unwrap();
            let _ = released.await;
        });
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let started = Instant::now();
        let response = client.get(url).send().await.unwrap();
        tokio::time::timeout(Duration::from_secs(2), read_probe_stream(response, crate::gateway::ProbeProtocol::Responses, false)).await.unwrap().unwrap();
        assert!(started.elapsed() >= Duration::from_millis(30));
        release.send(()).unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn sse_body_timeout_and_early_eof_never_succeed() {
        for stall in [false, true] {
            let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
            let url = format!("http://{}", listener.local_addr().unwrap());
            let (release, released) = tokio::sync::oneshot::channel::<()>();
            let server = tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0u8; 1024];
                socket.read(&mut request).await.unwrap();
                socket.write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\r\ndata: {\"type\":\"response.output_text.delta\",\"response_id\":\"resp_1\",\"delta\":\"OK\"}\n\n").await.unwrap();
                if stall { let _ = released.await; }
                socket.shutdown().await.unwrap();
            });
            let client = reqwest::Client::builder().no_proxy().timeout(Duration::from_millis(100)).build().unwrap();
            let response = client.get(url).send().await.unwrap();
            let error = read_probe_stream(response, crate::gateway::ProbeProtocol::Responses, false).await.unwrap_err().to_string();
            assert!(error.contains(if stall { "PROBE_TIMEOUT" } else { "PROBE_UNFINISHED" }), "{error}");
            let _ = release.send(());
            server.await.unwrap();
        }
    }

    #[tokio::test]
    async fn dynamic_cx2cc_probe_fails_without_contacting_a_gateway() {
        let temp = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&temp.path().join("dynamic-bridge.db")).unwrap();
        let mut params = default_provider_params("dynamic");
        params.cli_key = "claude".into();
        params.base_urls = vec![];
        params.api_key = None;
        params.bridge_type = Some(CX2CC_BRIDGE_TYPE.into());
        let provider = upsert(&db, params).unwrap();
        let app = tauri::test::mock_app();
        let result = test_provider_availability(app.handle(), db, provider.id).await.unwrap();
        assert!(!result.ok);
        assert!(result.status.is_none());
        assert!(result.tested_model.is_none());
        assert!(result.error.as_deref().unwrap().contains("源供应商"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn codex_bridge_availability_uses_source_provider_transport() {
        let temp = tempfile::tempdir().expect("tempdir");
        let db_path = temp.path().join("provider-availability.sqlite3");
        let db = crate::db::init_for_tests(&db_path).expect("init db");
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let (source_base_url, server_task) = response_from_request_capture(
            "/v1/messages",
            400,
            r#"{"error":{"message":"model not found"}}"#,
        )
        .await;

        let mut source_params = default_provider_params("Claude source");
        source_params.cli_key = "claude".to_string();
        source_params.base_urls = vec![source_base_url.clone()];
        source_params.api_key = Some("sk-claude".to_string());
        let source = upsert(&db, source_params).expect("insert source");

        let mut bridge_params = default_provider_params("Codex bridge");
        bridge_params.base_urls = vec![];
        bridge_params.api_key = None;
        bridge_params.source_provider_id = Some(source.id);
        bridge_params.bridge_type = Some(CODEX_TO_ANTHROPIC_MESSAGES_BRIDGE_TYPE.to_string());
        let bridge = upsert(&db, bridge_params).expect("insert bridge");

        let result = test_provider_availability(&app_handle, db, bridge.id)
            .await
            .expect("availability result");

        assert!(!result.ok);
        assert_eq!(result.provider_id, bridge.id);
        assert_eq!(result.provider_name, "Codex bridge");
        assert_eq!(result.base_url, source_base_url);
        assert_eq!(result.status, Some(400));
        assert!(result.error.as_deref().unwrap().contains("PROBE_MODEL_QUOTA"));

        let request = server_task.await.expect("server task");
        assert!(request
            .to_ascii_lowercase()
            .contains("x-api-key: sk-claude"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn codex_bridge_availability_maps_configured_test_model() {
        let temp = tempfile::tempdir().expect("tempdir");
        let db_path = temp
            .path()
            .join("provider-availability-mapped-model.sqlite3");
        let db = crate::db::init_for_tests(&db_path).expect("init db");
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let (source_base_url, server_task) = response_from_request_capture(
            "/v1/messages",
            400,
            r#"{"error":{"message":"model not found"}}"#,
        )
        .await;

        let mut source_params = default_provider_params("Claude source");
        source_params.cli_key = "claude".to_string();
        source_params.base_urls = vec![source_base_url.clone()];
        source_params.api_key = Some("sk-claude".to_string());
        let source = upsert(&db, source_params).expect("insert source");

        let mut bridge_params = default_provider_params("Codex bridge");
        bridge_params.base_urls = vec![];
        bridge_params.api_key = None;
        bridge_params.availability_test_model = Some("gpt-5.5".to_string());
        bridge_params.model_mapping = Some(crate::providers::ModelMapping {
            default_model: Some("claude-default".to_string()),
            exact: std::collections::BTreeMap::from([(
                "gpt-5.5".to_string(),
                "claude-opus-test".to_string(),
            )]),
        });
        bridge_params.source_provider_id = Some(source.id);
        bridge_params.bridge_type = Some(CODEX_TO_ANTHROPIC_MESSAGES_BRIDGE_TYPE.to_string());
        let bridge = upsert(&db, bridge_params).expect("insert bridge");

        let result = test_provider_availability(&app_handle, db, bridge.id)
            .await
            .expect("availability result");

        assert!(!result.ok);
        assert_eq!(result.requested_model.as_deref(), Some("gpt-5.5"));
        assert_eq!(result.tested_model.as_deref(), Some("claude-opus-test"));
        let request = server_task.await.expect("server task");
        assert!(
            request.contains(r#""model":"claude-opus-test""#),
            "mapped test model was not used: {request}"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn codex_bridge_availability_uses_disabled_source_transport() {
        let temp = tempfile::tempdir().expect("tempdir");
        let db_path = temp
            .path()
            .join("provider-availability-disabled-source.sqlite3");
        let db = crate::db::init_for_tests(&db_path).expect("init db");
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let (source_base_url, server_task) = response_from_request_capture(
            "/v1/responses",
            400,
            r#"{"error":{"message":"model not found"}}"#,
        )
        .await;

        let mut source_params = default_provider_params("Disabled source");
        source_params.cli_key = "codex".to_string();
        source_params.base_urls = vec![source_base_url.clone()];
        source_params.api_key = Some("sk-disabled-source".to_string());
        let source = upsert(&db, source_params).expect("insert source");

        let mut bridge_params = default_provider_params("Codex bridge");
        bridge_params.base_urls = vec![];
        bridge_params.api_key = None;
        bridge_params.source_provider_id = Some(source.id);
        bridge_params.bridge_type = Some(CODEX_TO_OPENAI_RESPONSES_BRIDGE_TYPE.to_string());
        let bridge = upsert(&db, bridge_params).expect("insert bridge");
        crate::providers::set_enabled(&db, source.id, false).expect("disable source");

        let result = test_provider_availability(&app_handle, db, bridge.id)
            .await
            .expect("availability result");

        assert!(!result.ok);
        assert_eq!(result.provider_id, bridge.id);
        assert_eq!(result.provider_name, "Codex bridge");
        assert_eq!(result.base_url, source_base_url);
        assert_eq!(result.status, Some(400));
        assert!(result.error.as_deref().unwrap().contains("PROBE_MODEL_QUOTA"));

        let request = server_task.await.expect("server task");
        assert!(request
            .to_ascii_lowercase()
            .contains("authorization: bearer sk-disabled-source"));
    }

    #[allow(clippy::await_holding_lock)]
    #[tokio::test(flavor = "current_thread")]
    async fn codex_bridge_availability_uses_oauth_source_credential() {
        let _env_lock = crate::test_support::test_env_lock();
        let temp = tempfile::tempdir().expect("tempdir");
        let db_path = temp
            .path()
            .join("provider-availability-oauth-source.sqlite3");
        let db = crate::db::init_for_tests(&db_path).expect("init db");
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let (source_base_url, server_task) = response_from_request_capture(
            "/responses",
            400,
            r#"{"error":{"message":"model not found"}}"#,
        )
        .await;
        let _oauth_base_url_override = crate::test_support::ScopedTestEnvVar::set(
            "AIO_CODING_HUB_TEST_CODEX_OAUTH_BASE_URL",
            source_base_url.clone(),
        );

        let mut source_params = default_provider_params("OAuth codex source");
        source_params.base_urls = vec![source_base_url.clone()];
        source_params.auth_mode = Some(ProviderAuthMode::Oauth);
        source_params.api_key = None;
        let source = upsert(&db, source_params).expect("insert oauth source");

        crate::providers::update_oauth_tokens(
            &db,
            source.id,
            "oauth",
            "codex_oauth",
            "oauth-access-token",
            Some("oauth-refresh-token"),
            None,
            "https://auth.openai.com/oauth/token",
            "test-client-id",
            None,
            Some(crate::shared::time::now_unix_seconds() + 3_600),
            Some("oauth@example.com"),
        )
        .expect("seed oauth token");

        let mut bridge_params = default_provider_params("Codex bridge");
        bridge_params.base_urls = vec![];
        bridge_params.api_key = None;
        bridge_params.source_provider_id = Some(source.id);
        bridge_params.bridge_type = Some(CODEX_TO_OPENAI_CHAT_BRIDGE_TYPE.to_string());
        let bridge = upsert(&db, bridge_params).expect("insert bridge");

        let result = test_provider_availability(&app_handle, db, bridge.id)
            .await
            .expect("availability result");

        assert!(!result.ok);
        assert_eq!(result.provider_id, bridge.id);
        assert_eq!(result.provider_name, "Codex bridge");
        assert_eq!(result.base_url, source_base_url);
        assert_eq!(result.status, Some(400));
        assert!(result.error.as_deref().unwrap().contains("PROBE_MODEL_QUOTA"));

        let request = server_task.await.expect("server task");
        assert!(request
            .to_ascii_lowercase()
            .contains("authorization: bearer oauth-access-token"));
        let payload: serde_json::Value = serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap()).unwrap();
        assert_eq!(payload["stream"], true);
        assert_eq!(payload["store"], false);
        assert!(payload.get("max_output_tokens").is_none());
    }
}
