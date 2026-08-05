//! Versioned, secret-free contract shared by AIO and the standalone terminal client.

use serde::{Deserialize, Serialize};

pub const OBSERVER_PROTOCOL_VERSION: u16 = 1;
pub const OBSERVER_DESCRIPTOR_FILE_NAME: &str = "observer-v1.json";
pub const OBSERVER_HISTORY_LIMIT_MAX: u16 = 50;
pub const OBSERVER_PROVIDER_PROBE_TIMEOUT_MS: u64 = 20_000;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum CliScope {
    Claude,
    Codex,
    Grok,
    Gemini,
    All,
}

impl CliScope {
    pub const VALUES: [Self; 5] = [
        Self::Codex,
        Self::Claude,
        Self::Grok,
        Self::Gemini,
        Self::All,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Grok => "grok",
            Self::Gemini => "gemini",
            Self::All => "all",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "claude" => Some(Self::Claude),
            "codex" => Some(Self::Codex),
            "grok" => Some(Self::Grok),
            "gemini" => Some(Self::Gemini),
            "all" => Some(Self::All),
            _ => None,
        }
    }

    pub fn matches(self, cli_key: &str) -> bool {
        self == Self::All || self.as_str() == cli_key.trim().to_ascii_lowercase()
    }
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ObserverDescriptorV1 {
    pub schema_version: u16,
    pub protocol_version: u16,
    pub app_version: String,
    pub pid: u32,
    pub port: u16,
    pub started_at_ms: i64,
    pub token: String,
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ObserverSection<T> {
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<T>,
}

impl<T> ObserverSection<T> {
    pub fn ready(value: T) -> Self {
        Self {
            available: true,
            value: Some(value),
        }
    }

    pub fn empty() -> Self {
        Self {
            available: true,
            value: None,
        }
    }

    pub fn unavailable() -> Self {
        Self {
            available: false,
            value: None,
        }
    }
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ObserverGatewayStatus {
    pub running: bool,
    pub port: Option<u16>,
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ObserverPreferredProvider {
    pub cli_key: String,
    pub provider_name: String,
    pub circuit_state: String,
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ObserverDominantProvider {
    pub provider_name: String,
    pub count: u8,
    pub sample_size: u8,
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ObserverTodayUsage {
    pub total_tokens: i64,
    pub cost_usd: Option<f64>,
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ObserverProviderSpendWindow {
    pub window: String,
    pub usage_usd: f64,
    pub limit_usd: f64,
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ObserverProviderOAuthQuota {
    pub short_label: Option<String>,
    pub five_hour_text: Option<String>,
    pub weekly_text: Option<String>,
    pub five_hour_reset_at_unix: Option<i64>,
    pub weekly_reset_at_unix: Option<i64>,
    pub checked_at_unix: i64,
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ObserverProviderAccountUsage {
    pub state: String,
    pub amount: Option<f64>,
    pub unit: Option<String>,
    pub last_fetched_at_unix: Option<i64>,
}

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObserverProviderAvailabilityState {
    Healthy,
    Unhealthy,
    NoData,
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ObserverProviderAvailabilityBucket {
    pub start_at_ms: i64,
    pub end_at_ms: i64,
    pub success_count: u32,
    pub failure_count: u32,
    pub state: ObserverProviderAvailabilityState,
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ObserverProviderAvailabilityTimeline {
    pub hours: u32,
    pub bucket_minutes: u32,
    pub success_count: u32,
    pub failure_count: u32,
    pub buckets: Vec<ObserverProviderAvailabilityBucket>,
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ObserverProviderAvailabilityTestResult {
    pub ok: bool,
    pub provider_id: i64,
    pub provider_name: String,
    pub base_url: String,
    pub status: Option<u16>,
    pub latency_ms: i64,
    pub error: Option<String>,
    pub response_preview: Option<String>,
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ObserverProviderStatus {
    pub provider_id: i64,
    pub cli_key: String,
    pub provider_name: String,
    pub route_rank: Option<i64>,
    pub provider_enabled: bool,
    pub route_enabled: bool,
    pub auth_kind: String,
    pub preferred: bool,
    pub eligibility: String,
    pub circuit_state: Option<String>,
    pub circuit_failure_count: Option<u32>,
    pub circuit_failure_threshold: Option<u32>,
    pub recover_at_unix: Option<i64>,
    pub spend_windows: Vec<ObserverProviderSpendWindow>,
    pub oauth_quota: Option<ObserverProviderOAuthQuota>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_usage: Option<ObserverProviderAccountUsage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub availability: Option<ObserverProviderAvailabilityTimeline>,
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ObserverProviderCollection {
    pub items: Vec<ObserverProviderStatus>,
    pub truncated: bool,
}

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObserverRequestState {
    Active,
    Terminal,
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ObserverRouteHop {
    pub provider_name: String,
    pub attempts: u32,
    pub skipped: bool,
    pub ok: bool,
    pub status: Option<i64>,
    pub error_code: Option<String>,
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ObserverRequestUsage {
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
    pub cache_read_tokens: Option<i64>,
    pub cache_creation_tokens: Option<i64>,
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ObserverContextCompaction {
    pub mode: String,
    pub implementation: String,
    pub trigger: String,
    pub reason: String,
    pub phase: String,
    pub strategy: String,
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ObserverConfiguredModelRoute {
    pub source_model: String,
    pub effective_model: String,
    pub reasoning_effort: Option<String>,
    pub policy_source: String,
    pub model_applied: bool,
    pub reasoning_effort_applied: bool,
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ObserverRequest {
    pub key: String,
    pub state: ObserverRequestState,
    pub cli_key: String,
    pub method: String,
    pub path: String,
    pub model: Option<String>,
    pub provider_name: Option<String>,
    pub status: Option<i64>,
    pub error_code: Option<String>,
    pub interrupted: bool,
    pub created_at_ms: i64,
    pub last_activity_ms: i64,
    pub duration_ms: Option<i64>,
    pub ttfb_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible_ttfb_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_stream_duration_ms: Option<i64>,
    #[serde(default)]
    pub upstream_stream_timing_version: i64,
    pub attempt_count: u32,
    pub retry_count: u32,
    pub provider_switch_count: u32,
    pub has_failover: bool,
    pub session_reuse: bool,
    pub session_id: Option<String>,
    pub folder_name: Option<String>,
    pub usage: Option<ObserverRequestUsage>,
    pub cost_usd: Option<f64>,
    pub route: Vec<ObserverRouteHop>,
    pub context_compaction: Option<ObserverContextCompaction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_reasoning_effort: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configured_model_route: Option<ObserverConfiguredModelRoute>,
}

#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ObserverSnapshotV1 {
    pub protocol_version: u16,
    pub app_version: String,
    pub generated_at_ms: i64,
    pub scope: CliScope,
    pub gateway: ObserverGatewayStatus,
    pub preferred_provider: ObserverSection<ObserverPreferredProvider>,
    pub last_request: ObserverSection<ObserverRequest>,
    pub dominant_provider: ObserverSection<ObserverDominantProvider>,
    pub active_inference_count: usize,
    pub today: ObserverSection<ObserverTodayUsage>,
    pub active_requests: ObserverSection<Vec<ObserverRequest>>,
    pub recent_requests: ObserverSection<Vec<ObserverRequest>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub providers: Option<ObserverSection<ObserverProviderCollection>>,
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ObserverHealthV1 {
    pub protocol_version: u16,
    pub app_version: String,
    pub pid: u32,
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ObserverApiError {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ObserverApiErrorResponse {
    pub error: ObserverApiError,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_scope_round_trips_fixed_values() {
        for scope in CliScope::VALUES {
            assert_eq!(CliScope::parse(scope.as_str()), Some(scope));
        }
        assert_eq!(CliScope::parse("future"), None);
    }

    #[test]
    fn descriptor_uses_camel_case_without_debug_contract() {
        let descriptor = ObserverDescriptorV1 {
            schema_version: 1,
            protocol_version: OBSERVER_PROTOCOL_VERSION,
            app_version: "0.60.39".to_string(),
            pid: 42,
            port: 37124,
            started_at_ms: 1_700_000_000_000,
            token: "secret".to_string(),
        };
        let value = serde_json::to_value(descriptor).expect("serialize descriptor");
        assert_eq!(value["schemaVersion"], 1);
        assert_eq!(value["protocolVersion"], OBSERVER_PROTOCOL_VERSION);
        assert!(value.get("schema_version").is_none());
    }

    #[test]
    fn provider_projection_is_optional_for_v1_compatibility() {
        let value = serde_json::json!({
            "protocolVersion": 1,
            "appVersion": "0.60.40",
            "generatedAtMs": 1,
            "scope": "codex",
            "gateway": { "running": true, "port": 37123 },
            "preferredProvider": { "available": true },
            "lastRequest": { "available": true },
            "dominantProvider": { "available": true },
            "activeInferenceCount": 0,
            "today": { "available": true },
            "activeRequests": { "available": true, "value": [] },
            "recentRequests": { "available": true, "value": [] }
        });
        let snapshot = serde_json::from_value::<ObserverSnapshotV1>(value)
            .expect("deserialize legacy v1 snapshot");
        assert!(snapshot.providers.is_none());
    }

    #[test]
    fn provider_probe_timeout_is_an_explicit_protocol_contract() {
        assert_eq!(OBSERVER_PROVIDER_PROBE_TIMEOUT_MS, 20_000);
    }
}
