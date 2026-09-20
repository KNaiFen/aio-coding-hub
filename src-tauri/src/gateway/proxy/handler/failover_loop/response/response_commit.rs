//! Required response checks at the single, still-uncommitted success boundary.

use super::*;
use crate::gateway::events::PluginDecision;
use crate::gateway::plugins::before_commit::{
    GatewayCommitAttempt, GatewayCommitDecision, GatewayCommitOutboundRequest,
    GatewayCommitRequest, GatewayResponseCommitInput, GatewayResponseCommitPlan,
};
use futures_core::Stream;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

// Complete responses carry RPC/JS copies as well as decoded bytes. Bound the
// number of complete deliveries, including slow downstream consumers.
const MAX_CONCURRENT_COMPLETE_RESPONSES: usize = 2;
const COMPLETE_REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
static COMPLETE_RESPONSE_SLOTS: OnceLock<Arc<Semaphore>> = OnceLock::new();

pub(in crate::gateway::proxy) struct ResponseCommitGuard {
    pub(super) plan: GatewayResponseCommitPlan,
    pub(super) deadline: Instant,
    replay_safe: AtomicBool,
    reservation: Arc<OwnedSemaphorePermit>,
}

impl ResponseCommitGuard {
    pub(super) fn initialize<R: tauri::Runtime>(
        input: &RequestContext<R>,
    ) -> Result<Option<Self>, GatewayErrorCode> {
        let plan = input
            .state
            .plugin_pipeline
            .freeze_response_commit_plan(&input.cli_key, &input.req_method, &input.forwarded_path)
            .map_err(|_| GatewayErrorCode::ResponseValidationFailed)?;
        let Some(plan) = plan else {
            return Ok(None);
        };
        let reservation = COMPLETE_RESPONSE_SLOTS
            .get_or_init(|| Arc::new(Semaphore::new(MAX_CONCURRENT_COMPLETE_RESPONSES)))
            .clone()
            .try_acquire_owned()
            .map_err(|_| GatewayErrorCode::ResponseBufferLimit)?;
        let duration = input
            .upstream_request_timeout_non_streaming
            .map(|configured| configured.min(COMPLETE_REQUEST_TIMEOUT))
            .unwrap_or(COMPLETE_REQUEST_TIMEOUT);
        Ok(Some(Self {
            plan,
            deadline: input.started + duration,
            replay_safe: AtomicBool::new(!has_private_continuation(
                &input.base_headers,
                &input.request_body_state.decoded_clone(),
            )),
            reservation: Arc::new(reservation),
        }))
    }

    pub(super) fn observe_outbound(&self, headers: &HeaderMap, body: &[u8]) {
        if has_private_continuation(headers, body) {
            self.replay_safe.store(false, Ordering::Relaxed);
        }
    }

    pub(super) fn replay_safe(&self) -> bool {
        self.replay_safe.load(Ordering::Relaxed)
    }

    pub(super) fn execution_lease(&self) -> Arc<OwnedSemaphorePermit> {
        self.reservation.clone()
    }

    pub(super) fn reserve_delivery(&self, response: Response) -> Response {
        response.map(|body| {
            Body::from_stream(ReservedBody {
                inner: body.into_data_stream(),
                _reservation: self.reservation.clone(),
            })
        })
    }
}

struct ReservedBody {
    inner: axum::body::BodyDataStream,
    _reservation: Arc<OwnedSemaphorePermit>,
}

impl Stream for ReservedBody {
    type Item = Result<Bytes, axum::Error>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}

// Validation already established UTF-8. Preserve that boundary when replaying
// complete SSE through the existing bounded chunk hooks, without copying the body.
struct ReplaySseStream {
    body: Bytes,
    offset: usize,
    chunk_limit: usize,
}

impl Stream for ReplaySseStream {
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let start = self.offset;
        if start == self.body.len() {
            return Poll::Ready(None);
        }
        let mut end = start.saturating_add(self.chunk_limit).min(self.body.len());
        while end > start && end < self.body.len() && self.body[end] & 0xc0 == 0x80 {
            end -= 1;
        }
        if end == start {
            // A scalar cannot fit into the configured context budget.
            self.offset = self.body.len();
            return Poll::Ready(Some(Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "stream context budget cannot hold a UTF-8 scalar",
            ))));
        }
        if end < self.body.len() {
            let window = &self.body[start..end];
            let lf = window
                .windows(2)
                .rposition(|pair| pair == b"\n\n")
                .map(|i| i + 2);
            let crlf = window
                .windows(4)
                .rposition(|pair| pair == b"\r\n\r\n")
                .map(|i| i + 4);
            if let Some(boundary) = lf.into_iter().chain(crlf).max() {
                end = start + boundary;
            }
        }
        let chunk = self.body.slice(start..end);
        self.offset = end;
        Poll::Ready(Some(Ok(chunk)))
    }
}

fn has_private_continuation(headers: &HeaderMap, body: &[u8]) -> bool {
    if ["x-codex-turn-state", "x-openai-turn-state"]
        .iter()
        .any(|name| headers.contains_key(*name))
    {
        return true;
    }
    serde_json::from_slice::<serde_json::Value>(body)
        .ok()
        .is_some_and(|value| {
            ["previous_response_id", "conversation"]
                .iter()
                .any(|key| value.get(*key).is_some_and(|value| !value.is_null()))
        })
}

pub(super) async fn finish_failure<R: tauri::Runtime>(
    ctx: CommonCtx<'_, R>,
    abort_guard: &mut crate::gateway::proxy::abort_guard::RequestAbortGuard<R>,
    attempts: &[FailoverAttempt],
    code: GatewayErrorCode,
    message: String,
) -> Response {
    finish_failure_with_status(
        ctx,
        abort_guard,
        attempts,
        code,
        message,
        StatusCode::BAD_GATEWAY,
    )
    .await
}

pub(super) async fn finish_failure_with_status<R: tauri::Runtime>(
    ctx: CommonCtx<'_, R>,
    abort_guard: &mut crate::gateway::proxy::abort_guard::RequestAbortGuard<R>,
    attempts: &[FailoverAttempt],
    code: GatewayErrorCode,
    message: String,
    status: StatusCode,
) -> Response {
    emit_request_event_and_spawn_request_log(
        RequestEndArgs::from_context(RequestEndContextArgs {
            deps: RequestEndDeps::new(
                &ctx.state.app,
                &ctx.state.db,
                &ctx.state.log_tx,
                &ctx.state.plugin_pipeline,
                &ctx.state.active_requests,
            ),
            trace_id: ctx.trace_id,
            cli_key: ctx.cli_key,
            method: ctx.method_hint,
            path: ctx.forwarded_path,
            observe: ctx.observe,
            query: ctx.query.as_deref(),
            excluded_from_stats: false,
            duration_ms: ctx.started.elapsed().as_millis(),
            attempts,
            special_settings_json: response_fixer::special_settings_json(ctx.special_settings),
            session_id: ctx.session_id.clone(),
            requested_model: ctx.requested_model.clone(),
            created_at_ms: ctx.created_at_ms,
            created_at: ctx.created_at,
        })
        .with_completion(RequestCompletion::failure(
            status.as_u16(),
            Some(ErrorCategory::SystemError.as_str()),
            code.as_str(),
        )),
    );
    abort_guard.disarm();
    error_response(
        status,
        ctx.trace_id.clone(),
        code.as_str(),
        message,
        if ctx.verbose_provider_error {
            attempts.to_vec()
        } else {
            Vec::new()
        },
    )
}

pub(super) enum CommitOutcome {
    Accepted(reqwest::Response, complete_response::CollectedTiming),
    Control(LoopControl),
}

pub(super) async fn check<R: tauri::Runtime>(
    ctx: CommonCtx<'_, R>,
    input: &RequestContext<R>,
    provider: ProviderCtx<'_>,
    attempt: AttemptCtx<'_>,
    mut state: LoopState<'_, R>,
    response: reqwest::Response,
    model: Option<String>,
) -> CommitOutcome {
    let guard = input
        .response_commit
        .as_ref()
        .expect("selected complete response plan");
    let status = response.status();
    let idle_timeout = match provider.stream_idle_timeout_seconds {
        Some(0) => None,
        Some(seconds) => Some(Duration::from_secs(seconds.into())),
        None => ctx.upstream_stream_idle_timeout,
    };
    let complete = match complete_response::collect(
        response,
        ctx.started,
        attempt.attempt_started,
        guard.deadline,
        ctx.upstream_first_byte_timeout,
        idle_timeout,
    )
    .await
    {
        Ok(complete) => complete,
        Err(error) => {
            use complete_response::CollectError;
            let code = match error {
                CollectError::TooLarge => GatewayErrorCode::ResponseBufferLimit,
                CollectError::Deadline => GatewayErrorCode::ResponseCommitTimeout,
                CollectError::UnsupportedEncoding => GatewayErrorCode::ResponseValidationFailed,
                CollectError::Read | CollectError::Decode => {
                    GatewayErrorCode::UpstreamBodyReadError
                }
                CollectError::FirstByteTimeout => GatewayErrorCode::UpstreamTimeout,
                CollectError::IdleTimeout => GatewayErrorCode::StreamIdleTimeout,
            };
            if matches!(
                error,
                CollectError::Read
                    | CollectError::Decode
                    | CollectError::FirstByteTimeout
                    | CollectError::IdleTimeout
            ) {
                let decision = if attempt.retry_index < attempt.provider_max_attempts
                    && matches!(error, CollectError::Read | CollectError::Decode)
                {
                    FailoverDecision::RetrySameProvider
                } else {
                    FailoverDecision::SwitchProvider
                };
                return CommitOutcome::Control(
                    record_system_failure_and_decide(RecordSystemFailureArgs {
                        ctx,
                        provider_ctx: provider,
                        attempt_ctx: attempt,
                        loop_state: state,
                        status: Some(status.as_u16()),
                        error_code: code.as_str(),
                        decision,
                        outcome: format!(
                            "response_collection_failed: code={} decision={}",
                            code.as_str(),
                            decision.as_str()
                        ),
                        reason: format!("complete upstream response collection failed: {error:?}"),
                        timeout_secs: matches!(error, CollectError::FirstByteTimeout)
                            .then_some(ctx.upstream_first_byte_timeout_secs),
                    })
                    .await,
                );
            }
            record_rejection(
                ctx,
                provider,
                attempt,
                &mut state,
                ResponseRejection {
                    status: status.as_u16(),
                    code,
                    diagnostic: None,
                    switch_provider: false,
                },
            )
            .await;
            return CommitOutcome::Control(LoopControl::Return(
                finish_failure(
                    ctx,
                    state.abort_guard,
                    state.attempts,
                    code,
                    format!("complete response validation failed: {error:?}"),
                )
                .await,
            ));
        }
    };
    let output = ctx
        .state
        .plugin_pipeline
        .run_response_commit_hook(
            &guard.plan,
            GatewayResponseCommitInput {
                execution_lease: Some(guard.reservation.clone()),
                trace_id: ctx.trace_id.clone(),
                request: GatewayCommitRequest {
                    cli_key: ctx.cli_key.clone(),
                    method: ctx.method_hint.clone(),
                    path: ctx.forwarded_path.clone(),
                    requested_model: ctx.requested_model.clone(),
                },
                outbound_request: GatewayCommitOutboundRequest { model },
                attempt: GatewayCommitAttempt {
                    provider_id: provider.provider_id,
                    provider_index: provider.provider_index as usize,
                    retry_index: attempt.retry_index as usize,
                },
                status: status.as_u16(),
                headers: complete.headers.clone(),
                body: complete.body.clone(),
            },
            guard.deadline,
        )
        .await;
    let decision = match output {
        Ok(output) => {
            crate::gateway::plugins::audit::persist_gateway_plugin_diagnostics(
                &ctx.state.db,
                ctx.trace_id,
                output.audit_events,
                output.execution_reports,
            );
            output.decision
        }
        Err(mut error) => {
            crate::gateway::plugins::audit::persist_gateway_plugin_error_audit_events(
                &ctx.state.db,
                ctx.trace_id,
                &mut error,
            );
            let code = if Instant::now() >= guard.deadline {
                GatewayErrorCode::ResponseCommitTimeout
            } else {
                GatewayErrorCode::ResponseValidationFailed
            };
            record_rejection(
                ctx,
                provider,
                attempt,
                &mut state,
                ResponseRejection {
                    status: status.as_u16(),
                    code,
                    diagnostic: None,
                    switch_provider: false,
                },
            )
            .await;
            return CommitOutcome::Control(LoopControl::Return(
                finish_failure(
                    ctx,
                    state.abort_guard,
                    state.attempts,
                    code,
                    "required response validation could not complete".to_string(),
                )
                .await,
            ));
        }
    };
    match decision {
        GatewayCommitDecision::Pass => {
            response_fixer::push_special_setting(
                ctx.special_settings,
                serde_json::json!({
                    "type": "response_commit", "scope": "attempt", "hit": true,
                    "providerId": provider.provider_id, "upstreamFirstByteMs": complete.timing.first_byte_ms,
                    "upstreamCompletedMs": complete.timing.completed_ms, "validatedMs": ctx.started.elapsed().as_millis(),
                    "decodedBytes": complete.body.len(),
                }),
            );
            let body = if is_event_stream(&complete.headers) {
                reqwest::Body::wrap_stream(ReplaySseStream {
                    body: complete.body,
                    offset: 0,
                    chunk_limit: ctx.state.plugin_pipeline.stream_context_limit_bytes(),
                })
            } else {
                reqwest::Body::from(complete.body)
            };
            let mut replay = axum::http::Response::new(body);
            *replay.status_mut() = status;
            *replay.headers_mut() = complete.headers;
            CommitOutcome::Accepted(reqwest::Response::from(replay), complete.timing)
        }
        decision @ (GatewayCommitDecision::Block(_) | GatewayCommitDecision::SwitchProvider(_)) => {
            // Only switch requests can continue; block always wins in the public pipeline.
            let switch = matches!(&decision, GatewayCommitDecision::SwitchProvider(_));
            let rejection = match decision {
                GatewayCommitDecision::Block(rejection)
                | GatewayCommitDecision::SwitchProvider(rejection) => rejection,
                GatewayCommitDecision::Pass => unreachable!(),
            };
            let code = if switch && !guard.replay_safe() {
                GatewayErrorCode::RequestReplayUnsafe
            } else {
                GatewayErrorCode::ResponseRejected
            };
            let diagnostic = PluginDecision {
                plugin_id: rejection.plugin_id,
                reason_code: rejection.reason_code,
                message: None,
            };
            record_rejection(
                ctx,
                provider,
                attempt,
                &mut state,
                ResponseRejection {
                    status: status.as_u16(),
                    code,
                    diagnostic: Some(diagnostic),
                    switch_provider: switch && guard.replay_safe(),
                },
            )
            .await;
            if switch && guard.replay_safe() {
                state.failed_provider_ids.insert(provider.provider_id);
                CommitOutcome::Control(LoopControl::BreakRetry)
            } else {
                CommitOutcome::Control(LoopControl::Return(finish_failure(ctx, state.abort_guard, state.attempts, code, if code == GatewayErrorCode::RequestReplayUnsafe { "request uses private continuation state and cannot safely switch providers" } else { "response rejected by required validation" }.to_string()).await))
            }
        }
    }
}

struct ResponseRejection {
    status: u16,
    code: GatewayErrorCode,
    diagnostic: Option<PluginDecision>,
    switch_provider: bool,
}

async fn record_rejection<R: tauri::Runtime>(
    ctx: CommonCtx<'_, R>,
    provider: ProviderCtx<'_>,
    attempt: AttemptCtx<'_>,
    state: &mut LoopState<'_, R>,
    rejection: ResponseRejection,
) {
    let ResponseRejection {
        status,
        code,
        diagnostic,
        switch_provider: switch,
    } = rejection;
    let policy_rejection = diagnostic.is_some();
    let outcome = if policy_rejection {
        "response_rejected"
    } else {
        "response_validation_failed"
    }
    .to_string();
    state.attempts.push(FailoverAttempt {
        provider_id: provider.provider_id,
        provider_name: provider.provider_name_base.clone(),
        base_url: provider.provider_base_url_base.clone(),
        outcome: outcome.clone(),
        status: Some(status),
        provider_index: Some(provider.provider_index),
        retry_index: Some(attempt.retry_index),
        session_reuse: provider.session_reuse,
        error_category: Some(ErrorCategory::SystemError.as_str()),
        error_code: Some(code.as_str()),
        decision: Some(if switch { "switch" } else { "abort" }),
        reason: Some(
            if switch {
                "required response validation requested the next eligible provider"
            } else {
                "required response validation stopped delivery"
            }
            .to_string(),
        ),
        selection_method: dc::selection_method(
            provider.provider_index,
            attempt.retry_index,
            provider.session_reuse,
        ),
        reason_code: Some(if policy_rejection {
            "response_rejected"
        } else {
            "response_validation_failed"
        }),
        plugin_decision: diagnostic,
        attempt_started_ms: Some(attempt.attempt_started_ms),
        attempt_duration_ms: Some(attempt.attempt_started.elapsed().as_millis()),
        circuit_state_before: Some(attempt.circuit_before.state.as_str()),
        circuit_state_after: Some(attempt.circuit_before.state.as_str()),
        circuit_failure_count: Some(attempt.circuit_before.failure_count),
        circuit_failure_threshold: Some(attempt.circuit_before.failure_threshold),
        circuit_recover_at_unix: None,
        circuit_trigger_error_code: None,
        provider_bridged: Some(provider.provider_bridged),
        timeout_secs: None,
        reasoning_effort: attempt.reasoning_effort.map(str::to_string),
        upstream_sent: attempt.upstream_sent,
        claude_model_mapping: provider.claude_model_mapping.cloned(),
        model_redirect: provider.model_redirect.cloned(),
    });
    *state.last_outcome = Some(AttemptOutcome::new(
        ErrorCategory::SystemError.as_str(),
        code.as_str(),
    ));
    state.abort_guard.capture_completed_attempts(state.attempts);
    emit_attempt_event_and_log_with_circuit_before(ctx, provider, attempt, outcome, Some(status))
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn continuation_requires_explicit_replay_safety_not_input_length() {
        assert!(has_private_continuation(&HeaderMap::new(), br#"{"previous_response_id":"resp_private","input":[{"role":"user","content":"increment"}]}"#));
        assert!(has_private_continuation(
            &HeaderMap::new(),
            br#"{"conversation":"conv_private","input":"increment"}"#
        ));
        assert!(!has_private_continuation(
            &HeaderMap::new(),
            br#"{"input":[{"role":"user","content":"full history"}]}"#
        ));
    }
}
