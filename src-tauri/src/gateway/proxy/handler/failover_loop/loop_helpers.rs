//! Usage: Helper types and functions for the failover loop orchestrator.
//!
//! Contains `FinalizeOwnedCommon`, skip-attempt helpers, and the
//! "all providers unavailable" finalization predicate.

use super::*;

pub(super) struct FinalizeOwnedCommon {
    pub(super) cli_key: String,
    pub(super) method_hint: String,
    pub(super) forwarded_path: String,
    pub(super) query: Option<String>,
    pub(super) trace_id: String,
    pub(super) session_id: Option<String>,
    pub(super) requested_model: Option<String>,
    pub(super) special_settings: Arc<Mutex<Vec<serde_json::Value>>>,
}

pub(super) fn finalize_owned_from_input<R: tauri::Runtime>(
    input: &RequestContext<R>,
) -> FinalizeOwnedCommon {
    FinalizeOwnedCommon {
        cli_key: input.cli_key.clone(),
        method_hint: input.method_hint.clone(),
        forwarded_path: input.forwarded_path.clone(),
        query: input.query.clone(),
        trace_id: input.trace_id.clone(),
        session_id: input.session_id.clone(),
        requested_model: input.requested_model.clone(),
        special_settings: input.special_settings.clone(),
    }
}

pub(super) struct SkippedProviderAttempt<'a> {
    pub(super) provider_id: i64,
    pub(super) provider_name: &'a str,
    pub(super) base_url: &'a str,
    pub(super) error_category: &'static str,
    pub(super) error_code: &'static str,
    pub(super) reason: String,
    pub(super) reason_code: Option<&'static str>,
    pub(super) attempt_started_ms: u128,
    /// Circuit snapshot at gate-deny time; `Some` only for circuit-gate skips
    /// so non-circuit skip paths keep their serialized shape unchanged.
    pub(super) circuit: Option<crate::circuit_breaker::CircuitSnapshot>,
}

pub(super) fn push_skipped_provider_attempt(
    attempts: &mut Vec<FailoverAttempt>,
    skipped: SkippedProviderAttempt<'_>,
) {
    let circuit = skipped.circuit.as_ref();
    attempts.push(FailoverAttempt {
        provider_id: skipped.provider_id,
        provider_name: skipped.provider_name.to_string(),
        base_url: skipped.base_url.to_string(),
        outcome: "skipped".to_string(),
        upstream_sent: false,
        status: None,
        provider_index: None,
        retry_index: None,
        session_reuse: None,
        error_category: Some(skipped.error_category),
        error_code: Some(skipped.error_code),
        decision: Some("skip"),
        reason: Some(skipped.reason),
        selection_method: Some(dc::SELECTION_METHOD_FILTERED),
        reason_code: skipped.reason_code,
        attempt_started_ms: Some(skipped.attempt_started_ms),
        attempt_duration_ms: Some(0),
        // Gate skip did not change the circuit state; before == after.
        circuit_state_before: circuit.map(|s| s.state.as_str()),
        circuit_state_after: circuit.map(|s| s.state.as_str()),
        circuit_failure_count: circuit.map(|s| s.failure_count),
        circuit_failure_threshold: circuit.map(|s| s.failure_threshold),
        circuit_recover_at_unix: circuit.and_then(|s| s.open_until.or(s.cooldown_until)),
        circuit_trigger_error_code: circuit.and_then(|s| s.last_trigger_error_code),
        provider_bridged: None,
        timeout_secs: None,
        stream_internal_error: None,
        requested_upstream_model: None,
    });
}

pub(super) fn is_gate_only_skipped_attempt(attempt: &FailoverAttempt) -> bool {
    if attempt.decision != Some("skip") {
        return false;
    }

    if attempt.provider_index.is_some() || attempt.retry_index.is_some() {
        return false;
    }

    matches!(
        attempt.reason_code,
        Some(
            dc::REASON_CIRCUIT_OPEN
                | dc::REASON_CIRCUIT_COOLDOWN
                | dc::REASON_RATE_LIMITED
                | dc::REASON_PROVIDER_DISABLED
        )
    )
}

pub(super) fn should_finalize_as_all_providers_unavailable(attempts: &[FailoverAttempt]) -> bool {
    attempts.is_empty() || attempts.iter().all(is_gate_only_skipped_attempt)
}

pub(super) fn should_finalize_as_no_enabled_provider_after_limit_exclusions(
    attempts: &[FailoverAttempt],
    providers_tried: usize,
    limit_exclusions: usize,
    skipped_open: usize,
    skipped_cooldown: usize,
) -> bool {
    attempts.is_empty()
        && providers_tried == 0
        && limit_exclusions > 0
        && skipped_open == 0
        && skipped_cooldown == 0
}

pub(super) fn apply_cx2cc_request_settings(
    responses_body: &mut serde_json::Value,
    cx2cc_settings: &crate::gateway::proxy::cx2cc::settings::Cx2ccSettings,
) {
    if let Some(ref effort) = cx2cc_settings.model_reasoning_effort {
        responses_body["reasoning"] = serde_json::json!({ "effort": effort });
    }
    if let Some(ref tier) = cx2cc_settings.service_tier {
        responses_body["service_tier"] = serde_json::json!(tier);
    }
    if cx2cc_settings.disable_response_storage {
        responses_body["store"] = serde_json::json!(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pure_limit_exclusion_is_no_enabled_provider() {
        assert!(
            should_finalize_as_no_enabled_provider_after_limit_exclusions(&[], 0, 2, 0, 0)
        );
    }

    #[test]
    fn circuit_or_real_attempt_is_not_no_enabled_provider() {
        assert!(!should_finalize_as_no_enabled_provider_after_limit_exclusions(
            &[], 0, 1, 1, 0
        ));
        assert!(!should_finalize_as_no_enabled_provider_after_limit_exclusions(
            &[], 1, 1, 0, 0
        ));
    }
}
