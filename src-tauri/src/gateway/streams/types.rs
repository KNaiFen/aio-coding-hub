//! Usage: Stream finalization context for gateway body relays.

use crate::gateway::active_requests::ActiveRequestRegistry;
use crate::gateway::plugins::pipeline::GatewayPluginPipeline;
use crate::{circuit_breaker, db, request_logs, session_manager};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::super::events::FailoverAttempt;

const ACTIVITY_FLUSH_INTERVAL_MS: i64 = 30_000;

pub(in crate::gateway) struct StreamActivityTracker {
    trace_id: String,
    cli_key: String,
    created_at_ms: i64,
    last_activity_ms: i64,
    last_flushed_activity_ms: i64,
    chunk_count: i64,
}

impl StreamActivityTracker {
    pub(in crate::gateway) fn new(trace_id: &str, cli_key: &str, created_at_ms: i64) -> Self {
        Self {
            trace_id: trace_id.to_string(),
            cli_key: cli_key.to_string(),
            created_at_ms,
            last_activity_ms: created_at_ms,
            last_flushed_activity_ms: created_at_ms,
            chunk_count: 0,
        }
    }

    pub(in crate::gateway) fn observe_chunk_at(&mut self, now_ms: i64) -> bool {
        self.chunk_count = self.chunk_count.saturating_add(1);
        self.last_activity_ms = now_ms.max(self.last_activity_ms).max(self.created_at_ms);
        if self
            .last_activity_ms
            .saturating_sub(self.last_flushed_activity_ms)
            < ACTIVITY_FLUSH_INTERVAL_MS
        {
            return false;
        }
        self.last_flushed_activity_ms = self.last_activity_ms;
        true
    }

    pub(in crate::gateway) fn last_activity_ms(&self) -> i64 {
        self.last_activity_ms
    }

    pub(in crate::gateway) fn details_json(&self, terminal_signal: Option<&str>) -> Option<String> {
        serde_json::to_string(&serde_json::json!({
            "trace_id": self.trace_id,
            "cli_key": self.cli_key,
            "chunk_count": self.chunk_count,
            "last_activity_ms": self.last_activity_ms,
            "terminal_signal": terminal_signal,
        }))
        .ok()
    }
}

#[derive(Debug, Default)]
struct UpstreamOutputTimingState {
    first_byte_ms: Option<u128>,
    first_output_ms: Option<u128>,
    last_output_ms: Option<u128>,
    final_attempt_duration_ms: Option<u128>,
    output_contaminated: bool,
    final_attempt_contaminated: bool,
}

#[derive(Clone, Debug, Default)]
pub(in crate::gateway) struct UpstreamOutputTiming {
    state: Arc<Mutex<UpstreamOutputTimingState>>,
}

impl UpstreamOutputTiming {
    pub(in crate::gateway) fn observe_first_byte_at(&self, elapsed_ms: u128) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.first_byte_ms.is_none() {
            state.first_byte_ms = Some(elapsed_ms);
        }
    }

    pub(in crate::gateway) fn first_byte_ms(&self) -> Option<u128> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .first_byte_ms
    }

    pub(in crate::gateway) fn from_buffered_prefix(
        first_output_ms: Option<u128>,
        last_output_ms: Option<u128>,
    ) -> Self {
        let timing = Self::default();
        if let Some(first_output_ms) = first_output_ms {
            let last_output_ms = last_output_ms
                .unwrap_or(first_output_ms)
                .max(first_output_ms);
            let mut state = timing
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.first_output_ms = Some(first_output_ms);
            state.last_output_ms = Some(last_output_ms);
        }
        timing
    }

    pub(in crate::gateway) fn observe_output_at(&self, elapsed_ms: u128) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.first_output_ms.is_none() {
            state.first_output_ms = Some(elapsed_ms);
        }
        state.last_output_ms = Some(
            state
                .last_output_ms
                .map_or(elapsed_ms, |last_output_ms| last_output_ms.max(elapsed_ms)),
        );
    }

    pub(in crate::gateway) fn invalidate_output(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.output_contaminated = true;
    }

    pub(in crate::gateway) fn invalidate_final_attempt(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.output_contaminated = true;
        state.final_attempt_contaminated = true;
    }

    pub(in crate::gateway) fn observe_protocol_completion_at(&self, elapsed_ms: u128) {
        self.observe_final_attempt_end_at(elapsed_ms);
    }

    pub(in crate::gateway) fn observe_clean_eof_at(&self, elapsed_ms: u128) {
        self.observe_final_attempt_end_at(elapsed_ms);
    }

    fn observe_final_attempt_end_at(&self, elapsed_ms: u128) {
        if elapsed_ms == 0 {
            return;
        }
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.final_attempt_duration_ms.is_none() {
            state.final_attempt_duration_ms = Some(elapsed_ms);
        }
    }

    pub(in crate::gateway) fn final_attempt_duration_ms(&self) -> Option<u128> {
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.final_attempt_contaminated {
            return None;
        }
        state
            .final_attempt_duration_ms
            .filter(|duration_ms| *duration_ms > 0)
    }

    pub(in crate::gateway) fn duration_ms(&self) -> Option<u128> {
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.output_contaminated {
            return None;
        }
        state.last_output_ms.zip(state.first_output_ms).and_then(
            |(last_output_ms, first_output_ms)| {
                last_output_ms
                    .checked_sub(first_output_ms)
                    .filter(|duration_ms| *duration_ms > 0)
            },
        )
    }
}

pub(in crate::gateway) struct StreamFinalizeCtx<R: tauri::Runtime = tauri::Wry> {
    pub(in crate::gateway) app: tauri::AppHandle<R>,
    pub(in crate::gateway) db: db::Db,
    pub(in crate::gateway) log_tx: tokio::sync::mpsc::Sender<request_logs::RequestLogInsert>,
    pub(in crate::gateway) plugin_pipeline: Arc<GatewayPluginPipeline>,
    pub(in crate::gateway) circuit: Arc<circuit_breaker::CircuitBreaker>,
    pub(in crate::gateway) session: Arc<session_manager::SessionManager>,
    pub(in crate::gateway) route_generation: session_manager::SessionRouteGeneration,
    pub(in crate::gateway) session_id: Option<String>,
    pub(in crate::gateway) enable_session_reuse: bool,
    pub(in crate::gateway) sort_mode_id: Option<i64>,
    pub(in crate::gateway) trace_id: String,
    pub(in crate::gateway) cli_key: String,
    pub(in crate::gateway) method: String,
    pub(in crate::gateway) path: String,
    pub(in crate::gateway) observe: bool,
    pub(in crate::gateway) query: Option<String>,
    pub(in crate::gateway) excluded_from_stats: bool,
    pub(in crate::gateway) special_settings: Arc<Mutex<Vec<serde_json::Value>>>,
    pub(in crate::gateway) provider_health_neutral: bool,
    pub(in crate::gateway) status: u16,
    pub(in crate::gateway) error_category: Option<&'static str>,
    pub(in crate::gateway) error_code: Option<&'static str>,
    pub(in crate::gateway) started: Instant,
    pub(in crate::gateway) attempt_started: Instant,
    pub(in crate::gateway) attempts: Vec<FailoverAttempt>,
    pub(in crate::gateway) attempts_json: String,
    pub(in crate::gateway) requested_model: Option<String>,
    pub(in crate::gateway) requested_upstream_model: Option<String>,
    pub(in crate::gateway) managed_model_route: bool,
    pub(in crate::gateway) created_at_ms: i64,
    pub(in crate::gateway) created_at: i64,
    pub(in crate::gateway) provider_cooldown_secs: i64,
    pub(in crate::gateway) upstream_first_byte_timeout_secs: u32,
    pub(in crate::gateway) upstream_retry_policy: crate::settings::UpstreamRetryPolicy,
    pub(in crate::gateway) detect_stream_internal_errors: bool,
    pub(in crate::gateway) provider_id: i64,
    pub(in crate::gateway) provider_name: String,
    pub(in crate::gateway) base_url: String,
    pub(in crate::gateway) auth_mode: String,
    pub(in crate::gateway) upstream_route_tracker: Arc<Mutex<crate::usage::SseUsageTracker>>,
    pub(in crate::gateway) observed_upstream_model: Arc<Mutex<Option<String>>>,
    pub(in crate::gateway) observed_upstream_conflicting_model: Arc<Mutex<Option<String>>>,
    pub(in crate::gateway) observed_upstream_reasoning_effort: Arc<Mutex<Option<String>>>,
    pub(in crate::gateway) fake_200_detected: bool,
    pub(in crate::gateway) fake_200_quota_exhausted: bool,
    pub(in crate::gateway) activity: Arc<Mutex<StreamActivityTracker>>,
    pub(in crate::gateway) active_requests: Arc<ActiveRequestRegistry>,
    pub(in crate::gateway) upstream_output_timing: UpstreamOutputTiming,
}

#[cfg(test)]
mod tests {
    use super::UpstreamOutputTiming;

    #[test]
    fn upstream_output_timing_uses_buffered_prefix_and_latest_output() {
        let timing = UpstreamOutputTiming::from_buffered_prefix(Some(120), Some(420));
        assert_eq!(timing.duration_ms(), Some(300));

        timing.observe_output_at(700);
        assert_eq!(timing.duration_ms(), Some(580));
    }

    #[test]
    fn upstream_output_timing_requires_distinct_first_and_last_events() {
        let timing = UpstreamOutputTiming::default();
        timing.observe_output_at(120);
        assert_eq!(timing.duration_ms(), None);
    }

    #[test]
    fn downstream_contamination_preserves_a_frozen_final_attempt() {
        let timing = UpstreamOutputTiming::from_buffered_prefix(Some(120), Some(420));
        timing.observe_protocol_completion_at(900);
        timing.invalidate_output();
        assert_eq!(timing.duration_ms(), None);
        assert_eq!(timing.final_attempt_duration_ms(), Some(900));
    }

    #[test]
    fn upstream_failure_invalidates_output_and_final_attempt_timing() {
        let timing = UpstreamOutputTiming::from_buffered_prefix(Some(120), Some(420));
        timing.observe_protocol_completion_at(900);
        timing.invalidate_final_attempt();
        assert_eq!(timing.duration_ms(), None);
        assert_eq!(timing.final_attempt_duration_ms(), None);
    }

    #[test]
    fn final_attempt_timing_runs_from_send_until_clean_upstream_eof() {
        let timing = UpstreamOutputTiming::default();
        timing.observe_first_byte_at(120);
        timing.observe_first_byte_at(240);
        timing.observe_clean_eof_at(900);
        assert_eq!(timing.first_byte_ms(), Some(120));
        assert_eq!(timing.final_attempt_duration_ms(), Some(900));
    }

    #[test]
    fn first_protocol_completion_freezes_the_final_attempt_before_eof() {
        let timing = UpstreamOutputTiming::default();
        timing.observe_protocol_completion_at(700);
        timing.observe_clean_eof_at(900);
        assert_eq!(timing.final_attempt_duration_ms(), Some(700));
    }
}
