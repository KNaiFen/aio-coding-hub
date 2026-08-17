//! Usage: Streaming tee wrappers that emit usage/cost and enqueue request logs.

use crate::gateway::response_fixer;
use crate::usage;
use axum::body::{Body, Bytes};
use futures_core::Stream;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use super::super::events::{emit_gateway_debug_log, emit_gateway_debug_log_lazy};
use super::super::model_route_mapping;
use super::super::proxy::{
    is_fake_200_non_stream_body, upstream_client_error_rules, GatewayErrorCode,
};
use super::super::util::{
    lossy_utf8_preview, now_unix_millis, now_unix_seconds, MAX_DEBUG_BODY_PREVIEW_BYTES,
};
use super::plugin_chunk::PLUGIN_STREAM_ERROR_MARKER;
use super::request_end::{emit_request_event_and_spawn_request_log, StreamRequestCompletion};
use super::{
    CodexResponsesOverloadErrorRewriter, RelayBodyStream, StreamFinalizeCtx, UpstreamOutputTiming,
};

pub(in crate::gateway) struct UpstreamModelObserverStream<S, B>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
{
    upstream: S,
    tracker: Arc<Mutex<usage::SseUsageTracker>>,
    observed_model: Arc<Mutex<Option<String>>>,
    observed_conflicting_model: Arc<Mutex<Option<String>>>,
    observed_reasoning_effort: Arc<Mutex<Option<String>>>,
    _marker: std::marker::PhantomData<B>,
}

impl<S, B> UpstreamModelObserverStream<S, B>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
{
    pub(in crate::gateway) fn new(
        upstream: S,
        tracker: Arc<Mutex<usage::SseUsageTracker>>,
        observed_model: Arc<Mutex<Option<String>>>,
        observed_conflicting_model: Arc<Mutex<Option<String>>>,
        observed_reasoning_effort: Arc<Mutex<Option<String>>>,
    ) -> Self {
        Self {
            upstream,
            tracker,
            observed_model,
            observed_conflicting_model,
            observed_reasoning_effort,
            _marker: std::marker::PhantomData,
        }
    }

    fn finalize_observed_route(&mut self) {
        let _ = finalize_upstream_route_observation(
            &self.tracker,
            &self.observed_model,
            &self.observed_conflicting_model,
            &self.observed_reasoning_effort,
        );
    }
}

fn finalize_upstream_route_observation(
    tracker: &Arc<Mutex<usage::SseUsageTracker>>,
    observed_model: &Arc<Mutex<Option<String>>>,
    observed_conflicting_model: &Arc<Mutex<Option<String>>>,
    observed_reasoning_effort: &Arc<Mutex<Option<String>>>,
) -> usage::ModelRouteEvidence {
    let evidence = {
        let mut tracker = match tracker.lock() {
            Ok(tracker) => tracker,
            Err(poisoned) => poisoned.into_inner(),
        };
        let _ = tracker.finalize();
        tracker.best_effort_route_evidence()
    };

    if let Some(model) = evidence.first_model.as_ref() {
        if let Ok(mut observed) = observed_model.lock() {
            *observed = Some(model.clone());
        }
    }
    if let Some(model) = evidence.first_conflicting_model.as_ref() {
        if let Ok(mut observed) = observed_conflicting_model.lock() {
            *observed = Some(model.clone());
        }
    }
    if let Some(effort) = evidence.reasoning_effort.as_ref() {
        if let Ok(mut observed) = observed_reasoning_effort.lock() {
            *observed = Some(effort.clone());
        }
    }
    evidence
}

impl<S, B> Stream for UpstreamModelObserverStream<S, B>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]> + Unpin,
{
    type Item = Result<B, reqwest::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().get_mut();
        match Pin::new(&mut this.upstream).poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                match this.tracker.lock() {
                    Ok(mut tracker) => tracker.ingest_chunk(chunk.as_ref()),
                    Err(poisoned) => poisoned.into_inner().ingest_chunk(chunk.as_ref()),
                }
                Poll::Ready(Some(Ok(chunk)))
            }
            Poll::Ready(Some(Err(err))) => {
                this.finalize_observed_route();
                Poll::Ready(Some(Err(err)))
            }
            Poll::Ready(None) => {
                this.finalize_observed_route();
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<S, B> Drop for UpstreamModelObserverStream<S, B>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
{
    fn drop(&mut self) {
        self.finalize_observed_route();
    }
}

const UPSTREAM_TIMING_RELAY_BUFFER_CAPACITY: usize = 32;

fn observe_new_output_events(
    tracker: &usage::SseUsageTracker,
    timing: &UpstreamOutputTiming,
    attempt_started: Instant,
    buffered_output_events_remaining: &mut u64,
    before: u64,
) {
    let observed = tracker.output_delta_event_count().saturating_sub(before);
    let buffered = observed.min(*buffered_output_events_remaining);
    *buffered_output_events_remaining =
        (*buffered_output_events_remaining).saturating_sub(buffered);
    if observed > buffered {
        timing.observe_output_at(attempt_started.elapsed().as_millis());
    }
}

fn invalidate_for_downstream_backpressure(timing: &UpstreamOutputTiming) {
    if timing.final_attempt_duration_ms().is_some() {
        timing.invalidate_output();
    } else {
        timing.invalidate_final_attempt();
    }
}

pub(in crate::gateway) fn spawn_upstream_output_timing_stream<S>(
    mut upstream: S,
    protocol_key: &str,
    timing: UpstreamOutputTiming,
    attempt_started: Instant,
    mut buffered_output_events_remaining: u64,
) -> RelayBodyStream
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin + Send + 'static,
{
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Bytes, reqwest::Error>>(
        UPSTREAM_TIMING_RELAY_BUFFER_CAPACITY,
    );
    let protocol_key = protocol_key.to_string();

    tokio::spawn(async move {
        let mut tracker = usage::SseUsageTracker::new(&protocol_key);
        let mut upstream_ended_normally = false;
        loop {
            let Some(item) = next_item(&mut upstream).await else {
                upstream_ended_normally = true;
                break;
            };
            if let Ok(chunk) = &item {
                let before = tracker.output_delta_event_count();
                let completion_before = tracker.protocol_completion_seen();
                tracker.ingest_chunk(chunk.as_ref());
                observe_new_output_events(
                    &tracker,
                    &timing,
                    attempt_started,
                    &mut buffered_output_events_remaining,
                    before,
                );
                if tracker.terminal_error_seen() {
                    timing.invalidate_final_attempt();
                } else if !completion_before && tracker.protocol_completion_seen() {
                    timing.observe_protocol_completion_at(attempt_started.elapsed().as_millis());
                }
            } else {
                timing.invalidate_final_attempt();
            }

            match tx.try_send(item) {
                Ok(()) => {}
                Err(tokio::sync::mpsc::error::TrySendError::Full(item)) => {
                    // A full queue means later observations would include downstream pacing.
                    invalidate_for_downstream_backpressure(&timing);
                    if tx.send(item).await.is_err() {
                        break;
                    }
                }
                Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                    timing.invalidate_output();
                    break;
                }
            }
        }

        let before = tracker.output_delta_event_count();
        let completion_before = tracker.protocol_completion_seen();
        let _ = tracker.finalize();
        observe_new_output_events(
            &tracker,
            &timing,
            attempt_started,
            &mut buffered_output_events_remaining,
            before,
        );
        if tracker.terminal_error_seen() {
            timing.invalidate_final_attempt();
        } else if !completion_before && tracker.protocol_completion_seen() {
            timing.observe_protocol_completion_at(attempt_started.elapsed().as_millis());
        } else if upstream_ended_normally {
            timing.observe_clean_eof_at(attempt_started.elapsed().as_millis());
        }
    });

    RelayBodyStream::new(rx)
}

pub(in crate::gateway) fn spawn_upstream_body_timing_stream<S>(
    mut upstream: S,
    timing: UpstreamOutputTiming,
    attempt_started: Instant,
) -> RelayBodyStream
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin + Send + 'static,
{
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Bytes, reqwest::Error>>(
        UPSTREAM_TIMING_RELAY_BUFFER_CAPACITY,
    );

    tokio::spawn(async move {
        loop {
            let Some(item) = next_item(&mut upstream).await else {
                timing.observe_clean_eof_at(attempt_started.elapsed().as_millis());
                break;
            };
            match &item {
                Ok(chunk) if !chunk.is_empty() => {
                    timing.observe_first_byte_at(attempt_started.elapsed().as_millis());
                }
                Ok(_) => {}
                Err(_) => timing.invalidate_final_attempt(),
            }

            match tx.try_send(item) {
                Ok(()) => {}
                Err(tokio::sync::mpsc::error::TrySendError::Full(item)) => {
                    timing.invalidate_final_attempt();
                    if tx.send(item).await.is_err() {
                        break;
                    }
                }
                Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                    timing.invalidate_output();
                    break;
                }
            }
        }
    });

    RelayBodyStream::new(rx)
}

fn is_codex_responses_path(cli_key: &str, path: &str) -> bool {
    if cli_key != "codex" {
        return false;
    }
    matches!(
        path.trim_end_matches('/'),
        "/v1/responses" | "/responses" | "/v1/codex/responses"
    )
}

#[allow(clippy::too_many_arguments)]
fn is_codex_client_abort_successish(
    cli_key: &str,
    path: &str,
    status: u16,
    saw_stream_output: bool,
    completion_seen: bool,
    usage_seen: bool,
    terminal_error_seen: bool,
    upstream_ended_normally: bool,
) -> bool {
    is_codex_responses_path(cli_key, path)
        && (200..300).contains(&status)
        && saw_stream_output
        && !terminal_error_seen
        // A clean upstream EOF is the fallback when no explicit completion marker
        // exists; otherwise completion or usage must have reached the drain.
        && (usage_seen || completion_seen || upstream_ended_normally)
}

fn is_codex_drop_successish(
    cli_key: &str,
    path: &str,
    status: u16,
    saw_stream_output: bool,
    completion_seen: bool,
    usage_seen: bool,
    terminal_error_seen: bool,
) -> bool {
    is_codex_responses_path(cli_key, path)
        && (200..300).contains(&status)
        && saw_stream_output
        // If completion/usage is observed, tolerate terminal markers during teardown.
        && (usage_seen || completion_seen || !terminal_error_seen)
}

fn is_codex_stream_terminal_error_successish(
    cli_key: &str,
    path: &str,
    status: u16,
    saw_stream_output: bool,
    completion_seen: bool,
    usage_seen: bool,
) -> bool {
    is_codex_stream_tail_error_successish(
        cli_key,
        path,
        status,
        saw_stream_output,
        completion_seen,
        usage_seen,
    )
}

fn is_codex_stream_tail_error_successish(
    cli_key: &str,
    path: &str,
    status: u16,
    saw_stream_output: bool,
    completion_seen: bool,
    usage_seen: bool,
) -> bool {
    is_codex_responses_path(cli_key, path)
        && (200..300).contains(&status)
        && saw_stream_output
        && (usage_seen || completion_seen)
}

fn is_codex_body_buffer_drop_successish(
    cli_key: &str,
    path: &str,
    status: u16,
    saw_stream_output: bool,
    usage_seen: bool,
) -> bool {
    is_codex_responses_path(cli_key, path)
        && (200..300).contains(&status)
        && saw_stream_output
        && usage_seen
}

fn is_plugin_stream_error_chunk(chunk: &[u8]) -> bool {
    chunk
        .windows(PLUGIN_STREAM_ERROR_MARKER.len())
        .any(|window| window == PLUGIN_STREAM_ERROR_MARKER.as_bytes())
}

fn spawn_touch_activity<R: tauri::Runtime>(
    ctx: &StreamFinalizeCtx<R>,
    last_activity_ms: i64,
    details: Option<String>,
) {
    if ctx.observe {
        ctx.active_requests
            .touch(ctx.trace_id.as_str(), last_activity_ms);
    }

    let db = ctx.db.clone();
    let trace_id = ctx.trace_id.clone();
    let cli_key = ctx.cli_key.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(err) =
            crate::request_logs::touch_activity(&db, &trace_id, &cli_key, last_activity_ms, details)
        {
            tracing::warn!(
                trace_id = %trace_id,
                cli = %cli_key,
                error = %err,
                "request log activity touch failed"
            );
        }
    });
}

struct NextFuture<'a, S: Stream + Unpin>(&'a mut S);

impl<'a, S: Stream + Unpin> Future for NextFuture<'a, S> {
    type Output = Option<S::Item>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut *self.0).poll_next(cx)
    }
}

async fn next_item<S: Stream + Unpin>(stream: &mut S) -> Option<S::Item> {
    NextFuture(stream).await
}

pub(in crate::gateway) struct UsageSseTeeStream<S, B, R = tauri::Wry>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
    R: tauri::Runtime,
    R::Handle: Unpin,
{
    upstream: S,
    tracker: usage::SseUsageTracker,
    ctx: StreamFinalizeCtx<R>,
    first_byte_ms: Option<u128>,
    idle_timeout: Option<Duration>,
    idle_sleep: Option<Pin<Box<tokio::time::Sleep>>>,
    finalized: bool,
    defer_terminal_error: bool,
    stop_after_terminal_error: bool,
}

impl<S, B, R> UsageSseTeeStream<S, B, R>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
    R: tauri::Runtime,
    R::Handle: Unpin,
{
    pub(in crate::gateway) fn new(
        upstream: S,
        ctx: StreamFinalizeCtx<R>,
        idle_timeout: Option<Duration>,
        initial_first_byte_ms: Option<u128>,
    ) -> Self {
        let tracker = usage::SseUsageTracker::new(&ctx.cli_key);
        let tracker = if ctx.detect_stream_internal_errors {
            tracker.with_stream_internal_error_classifier(
                ctx.upstream_retry_policy.enabled
                    && ctx.upstream_retry_policy.stream_internal_errors.enabled,
                &ctx.upstream_retry_policy
                    .stream_internal_errors
                    .retry_keywords,
                &ctx.upstream_retry_policy
                    .stream_internal_errors
                    .non_retry_keywords,
            )
        } else {
            tracker
        };
        Self {
            upstream,
            tracker,
            ctx,
            first_byte_ms: initial_first_byte_ms,
            idle_timeout,
            idle_sleep: idle_timeout.map(|d| Box::pin(tokio::time::sleep(d))),
            finalized: false,
            defer_terminal_error: false,
            stop_after_terminal_error: false,
        }
    }

    pub(in crate::gateway) fn with_defer_terminal_error(mut self) -> Self {
        self.defer_terminal_error = true;
        self
    }

    fn poll_next_inner(
        &mut self,
        cx: &mut Context<'_>,
        enforce_idle_timeout: bool,
        defer_finalization: bool,
    ) -> Poll<Option<Result<B, reqwest::Error>>> {
        if self.stop_after_terminal_error {
            return Poll::Ready(None);
        }

        let next = Pin::new(&mut self.upstream).poll_next(cx);

        match next {
            Poll::Pending => {
                // Upstream has no data right now; only then check the idle timer.
                // Drain path (enforce_idle_timeout=false) keeps its own deadline.
                if enforce_idle_timeout {
                    if let Some(sleep) = self.idle_sleep.as_mut() {
                        if sleep.as_mut().poll(cx).is_ready() {
                            self.finalize(Some(GatewayErrorCode::StreamIdleTimeout.as_str()));
                            return Poll::Ready(None);
                        }
                    }
                }
                Poll::Pending
            }
            Poll::Ready(None) => {
                // The relay owns finalization while draining after a downstream
                // disconnect so it can persist complete client-abort evidence.
                if !(defer_finalization
                    || self.defer_terminal_error && self.tracker.terminal_error_seen())
                {
                    self.finalize(self.ctx.error_code);
                }
                Poll::Ready(None)
            }
            Poll::Ready(Some(Ok(chunk))) => {
                if self.first_byte_ms.is_none() {
                    self.first_byte_ms = Some(self.ctx.attempt_started.elapsed().as_millis());
                }
                // Reuse existing Box allocation via Sleep::reset() to avoid heap churn per chunk
                if let Some(d) = self.idle_timeout {
                    if let Some(ref mut sleep) = self.idle_sleep {
                        sleep.as_mut().reset(tokio::time::Instant::now() + d);
                    } else {
                        self.idle_sleep = Some(Box::pin(tokio::time::sleep(d)));
                    }
                }
                emit_gateway_debug_log_lazy(&self.ctx.app, || {
                    format!(
                        "[SSE_CHUNK] trace_id={} len={}\n  {}",
                        self.ctx.trace_id,
                        chunk.as_ref().len(),
                        lossy_utf8_preview(chunk.as_ref(), MAX_DEBUG_BODY_PREVIEW_BYTES),
                    )
                });
                let was_terminal_error = self.tracker.terminal_error_seen();
                self.tracker.ingest_chunk(chunk.as_ref());
                if let Ok(mut activity) = self.ctx.activity.lock() {
                    if activity.observe_chunk_at(now_unix_millis().min(i64::MAX as u64) as i64) {
                        spawn_touch_activity(
                            &self.ctx,
                            activity.last_activity_ms(),
                            activity.details_json(None),
                        );
                    }
                }
                if self.tracker.terminal_error_seen() {
                    if !was_terminal_error {
                        emit_gateway_debug_log(
                            &self.ctx.app,
                            format!(
                                "[SSE] terminal_error_seen triggered — trace_id={} cli_key={} path={} fake_200={}",
                                self.ctx.trace_id,
                                self.ctx.cli_key,
                                self.ctx.path,
                                self.tracker.fake_200_detected(),
                            ),
                        );
                    }
                    if !self.defer_terminal_error {
                        let code = if self.tracker.fake_200_detected() {
                            GatewayErrorCode::Fake200.as_str()
                        } else {
                            GatewayErrorCode::StreamError.as_str()
                        };
                        self.finalize(Some(code));
                        if is_plugin_stream_error_chunk(chunk.as_ref()) {
                            self.stop_after_terminal_error = true;
                            return Poll::Ready(Some(Ok(chunk)));
                        }
                        return Poll::Ready(None);
                    }
                }
                Poll::Ready(Some(Ok(chunk)))
            }
            Poll::Ready(Some(Err(err))) => {
                if defer_finalization {
                    return Poll::Ready(Some(Err(err)));
                }
                let completion_seen = self.tracker.completion_seen();
                let codex_successish = is_codex_stream_tail_error_successish(
                    &self.ctx.cli_key,
                    &self.ctx.path,
                    self.ctx.status,
                    self.first_byte_ms.is_some(),
                    completion_seen,
                    completion_seen,
                );
                if codex_successish {
                    emit_gateway_debug_log(
                        &self.ctx.app,
                        format!(
                            "[SSE] tolerating late stream read error after completion trace_id={} cli_key={} path={} err={}",
                            self.ctx.trace_id, self.ctx.cli_key, self.ctx.path, err
                        ),
                    );
                    self.finalize(None);
                    Poll::Ready(None)
                } else {
                    self.finalize(Some(GatewayErrorCode::StreamError.as_str()));
                    Poll::Ready(Some(Err(err)))
                }
            }
        }
    }

    fn finalize(&mut self, error_code: Option<&'static str>) {
        if self.finalized {
            return;
        }
        self.finalized = true;

        let usage = self.tracker.finalize();
        let terminal_signal = if error_code.is_some() {
            Some("error")
        } else if self.tracker.completion_seen() {
            Some("completed")
        } else {
            None
        };
        if let Ok(activity) = self.ctx.activity.lock() {
            spawn_touch_activity(
                &self.ctx,
                activity.last_activity_ms(),
                activity.details_json(terminal_signal),
            );
        }

        // Propagate fake 200 detection from tracker to finalize context.
        if self.tracker.fake_200_detected() {
            self.ctx.fake_200_detected = true;
        }
        let effective_error_code = if error_code.is_none()
            && self
                .tracker
                .is_empty_success(&self.ctx.path, self.ctx.status, usage.as_ref())
        {
            Some(GatewayErrorCode::EmptyResponse.as_str())
        } else {
            error_code
        };
        let usage_metrics = usage.as_ref().map(|u| u.metrics.clone());
        let route_evidence = finalize_upstream_route_observation(
            &self.ctx.upstream_route_tracker,
            &self.ctx.observed_upstream_model,
            &self.ctx.observed_upstream_conflicting_model,
            &self.ctx.observed_upstream_reasoning_effort,
        );
        if let Some(setting) = model_route_mapping::build_model_route_mapping_setting_from_shared(
            model_route_mapping::SharedModelRouteSettingInput {
                cli_key: &self.ctx.cli_key,
                requested_model: self.ctx.requested_upstream_model.as_deref(),
                actual_model: route_evidence.first_model.as_deref(),
                conflicting_actual_model: route_evidence.first_conflicting_model.as_deref(),
                actual_reasoning_effort: route_evidence.reasoning_effort.as_deref(),
                special_settings: &self.ctx.special_settings,
                provider_id: self.ctx.provider_id,
                provider_name: &self.ctx.provider_name,
            },
        ) {
            response_fixer::push_model_route_mapping_special_setting(
                &self.ctx.special_settings,
                setting,
            );
        }
        let requested_model = self
            .ctx
            .requested_model
            .clone()
            .or(route_evidence.first_model);

        let stream_error_seen = self.tracker.terminal_error_seen()
            || self.tracker.stream_internal_error_evidence().is_some();
        let upstream_stream_duration_ms = self.ctx.upstream_output_timing.duration_ms();
        let upstream_stream_timing_version = i64::from(
            effective_error_code.is_none()
                && !stream_error_seen
                && self.tracker.completion_seen()
                && upstream_stream_duration_ms.is_some(),
        );
        let final_upstream_attempt_duration_ms =
            self.ctx.upstream_output_timing.final_attempt_duration_ms();
        let final_upstream_attempt_timing_version = i64::from(
            effective_error_code.is_none()
                && !stream_error_seen
                && final_upstream_attempt_duration_ms.is_some(),
        );
        emit_request_event_and_spawn_request_log(
            &self.ctx,
            StreamRequestCompletion::from_error_code(
                effective_error_code,
                self.first_byte_ms,
                self.first_byte_ms,
                requested_model,
                usage_metrics,
                usage,
            )
            .with_upstream_stream_timing(
                upstream_stream_duration_ms,
                upstream_stream_timing_version,
            )
            .with_final_upstream_attempt_timing(
                final_upstream_attempt_duration_ms,
                final_upstream_attempt_timing_version,
            )
            .with_terminal_signal(terminal_signal)
            .with_stream_internal_error(self.tracker.stream_internal_error_evidence().cloned()),
        );
    }
}

impl<S, B, R> Stream for UsageSseTeeStream<S, B, R>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
    R: tauri::Runtime,
    R::Handle: Unpin,
{
    type Item = Result<B, reqwest::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().get_mut();
        this.poll_next_inner(cx, true, false)
    }
}

struct DrainNextFuture<'a, S, B, R>(&'a mut UsageSseTeeStream<S, B, R>)
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
    R: tauri::Runtime,
    R::Handle: Unpin;

impl<'a, S, B, R> Future for DrainNextFuture<'a, S, B, R>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
    R: tauri::Runtime,
    R::Handle: Unpin,
{
    type Output = Option<Result<B, reqwest::Error>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.poll_next_inner(cx, false, true)
    }
}

async fn next_drain_item<S, B, R>(
    stream: &mut UsageSseTeeStream<S, B, R>,
) -> Option<Result<B, reqwest::Error>>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
    R: tauri::Runtime,
    R::Handle: Unpin,
{
    DrainNextFuture(stream).await
}

impl<S, B, R> Drop for UsageSseTeeStream<S, B, R>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
    R: tauri::Runtime,
    R::Handle: Unpin,
{
    fn drop(&mut self) {
        if !self.finalized {
            self.ctx.upstream_output_timing.invalidate_output();
            // Best-effort flush for trailing partial SSE data before deciding abort/success.
            let usage = self.tracker.finalize();
            let usage_seen = usage.is_some();
            let completion_seen = self.tracker.completion_seen();
            let terminal_error_seen = self.tracker.terminal_error_seen();

            let codex_successish = is_codex_drop_successish(
                &self.ctx.cli_key,
                &self.ctx.path,
                self.ctx.status,
                self.first_byte_ms.is_some(),
                completion_seen,
                usage_seen,
                terminal_error_seen,
            );

            if codex_successish {
                self.finalize(None);
            } else {
                self.finalize(Some(GatewayErrorCode::StreamAborted.as_str()));
            }
        }
    }
}

const SSE_RELAY_BUFFER_CAPACITY: usize = 32;

async fn send_sse_relay_chunk(
    tx: &tokio::sync::mpsc::Sender<Result<Bytes, reqwest::Error>>,
    timing: &UpstreamOutputTiming,
    chunk: Bytes,
) -> bool {
    match tx.try_send(Ok(chunk)) {
        Ok(()) => true,
        Err(tokio::sync::mpsc::error::TrySendError::Full(item)) => {
            invalidate_for_downstream_backpressure(timing);
            tx.send(item).await.is_ok()
        }
        Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
            timing.invalidate_output();
            false
        }
    }
}

pub(in crate::gateway) fn spawn_usage_sse_relay_body<S, R>(
    upstream: S,
    ctx: StreamFinalizeCtx<R>,
    idle_timeout: Option<Duration>,
    initial_first_byte_ms: Option<u128>,
    rewrite_codex_responses_overload_errors: bool,
) -> Body
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin + Send + 'static,
    R: tauri::Runtime + 'static,
    R::Handle: Unpin,
{
    let (tx, rx) =
        tokio::sync::mpsc::channel::<Result<Bytes, reqwest::Error>>(SSE_RELAY_BUFFER_CAPACITY);

    let mut tee = UsageSseTeeStream::new(upstream, ctx, idle_timeout, initial_first_byte_ms)
        .with_defer_terminal_error();

    tokio::spawn(async move {
        let mut forwarded_chunks: i64 = 0;
        let mut forwarded_bytes: i64 = 0;
        let mut drained_chunks: i64 = 0;
        let mut drained_bytes: i64 = 0;
        let mut client_abort_detected_by: Option<&'static str> = None;
        let mut downstream_closed = false;
        let mut upstream_ended_normally = false;
        let mut upstream_transport_error_seen = false;
        let mut overload_rewriter =
            rewrite_codex_responses_overload_errors.then(CodexResponsesOverloadErrorRewriter::new);

        let is_codex_responses = is_codex_responses_path(&tee.ctx.cli_key, &tee.ctx.path);
        let mut drain_deadline: Option<tokio::time::Instant> = None;
        let drain_grace = {
            // Codex ChatGPT backend: response.completed (carrying usage) may arrive
            // several seconds after the last output chunk; use a longer drain window.
            let cap = if is_codex_responses {
                Duration::from_secs(15)
            } else {
                Duration::from_secs(5)
            };
            let floor = Duration::from_millis(500);
            match idle_timeout {
                Some(d) if d < floor => floor,
                Some(d) if d > cap => cap,
                Some(d) => d,
                None => {
                    if is_codex_responses {
                        Duration::from_secs(10)
                    } else {
                        Duration::from_secs(2)
                    }
                }
            }
        };

        loop {
            if downstream_closed {
                if !is_codex_responses {
                    break;
                }
                // A completion frame freezes the attempt timestamp but is not
                // sufficient to stop draining: a queued terminal error must
                // still be able to invalidate that timestamp before logging.
                // Stop only at EOF, an upstream error, or the bounded deadline.
                // Some Codex backend flows can emit transient error-like markers before
                // `response.completed` (with usage) arrives.
                let Some(deadline) = drain_deadline else {
                    break;
                };
                let now = tokio::time::Instant::now();
                if now >= deadline {
                    break;
                }

                let remaining = deadline.saturating_duration_since(now);
                match tokio::time::timeout(remaining, next_drain_item(&mut tee)).await {
                    Ok(Some(Ok(chunk))) => {
                        let chunk_len = chunk.len().min(i64::MAX as usize) as i64;
                        drained_chunks = drained_chunks.saturating_add(1);
                        drained_bytes = drained_bytes.saturating_add(chunk_len);
                    }
                    Ok(Some(Err(_))) => {
                        upstream_transport_error_seen = true;
                        break;
                    }
                    Ok(None) => {
                        upstream_ended_normally = true;
                        break;
                    }
                    Err(_) => {
                        break;
                    }
                }
                continue;
            }

            tokio::select! {
                biased;
                // 如果客户端提前断开，但上游短时间没有新 chunk，就会卡在 next_item().await。
                // 这里通过监听 rx 端被 drop 来更早感知断开，避免误记 GW_STREAM_ABORTED。
                // 如果断开和 idle timeout 同时 ready，断开应优先进入 Codex drain。
                _ = tx.closed() => {
                    tee.ctx.upstream_output_timing.invalidate_output();
                    client_abort_detected_by = Some("rx_closed");
                    downstream_closed = true;
                    if is_codex_responses {
                        drain_deadline = Some(tokio::time::Instant::now() + drain_grace);
                        continue;
                    }
                    break;
                }
                item = next_item(&mut tee) => {
                    let Some(item) = item else {
                        if let Some(tail) = overload_rewriter
                            .as_mut()
                            .and_then(CodexResponsesOverloadErrorRewriter::finish)
                        {
                            let tail_len = tail.len().min(i64::MAX as usize) as i64;
                            if !send_sse_relay_chunk(
                                &tx,
                                &tee.ctx.upstream_output_timing,
                                tail,
                            )
                            .await
                            {
                                client_abort_detected_by = Some("send_failed");
                            } else {
                                forwarded_chunks = forwarded_chunks.saturating_add(1);
                                forwarded_bytes = forwarded_bytes.saturating_add(tail_len);
                            }
                        }
                        upstream_ended_normally = true;
                        break;
                    };

                    match item {
                        Ok(chunk) => {
                            let chunks = match overload_rewriter.as_mut() {
                                Some(rewriter) => rewriter.ingest(chunk),
                                None => vec![chunk],
                            };
                            let mut send_failed = false;
                            for chunk in chunks {
                                let chunk_len = chunk.len().min(i64::MAX as usize) as i64;
                                if !send_sse_relay_chunk(
                                    &tx,
                                    &tee.ctx.upstream_output_timing,
                                    chunk,
                                )
                                .await
                                {
                                    client_abort_detected_by = Some("send_failed");
                                    downstream_closed = true;
                                    send_failed = true;
                                    break;
                                }
                                forwarded_chunks = forwarded_chunks.saturating_add(1);
                                forwarded_bytes = forwarded_bytes.saturating_add(chunk_len);
                            }
                            if send_failed {
                                if is_codex_responses {
                                    drain_deadline = Some(tokio::time::Instant::now() + drain_grace);
                                    continue;
                                }
                                break;
                            }
                        }
                        Err(err) => {
                            tee.ctx.upstream_output_timing.invalidate_final_attempt();
                            if let Some(tail) = overload_rewriter
                                .as_mut()
                                .and_then(CodexResponsesOverloadErrorRewriter::finish)
                            {
                                let tail_len = tail.len().min(i64::MAX as usize) as i64;
                                if send_sse_relay_chunk(
                                    &tx,
                                    &tee.ctx.upstream_output_timing,
                                    tail,
                                )
                                .await
                                {
                                    forwarded_chunks = forwarded_chunks.saturating_add(1);
                                    forwarded_bytes = forwarded_bytes.saturating_add(tail_len);
                                }
                            }
                            // 尽力把流错误透传给客户端
                            let _ = tx.send(Err(err)).await;
                            break;
                        }
                    }
                }
            }
        }

        // When the stream ended normally (no client abort) but the tracker
        // detected a terminal-error-like SSE event, apply Codex-specific
        // tolerance: if we already saw output AND completion/usage, treat the
        // request as successful instead of marking it GW_STREAM_ERROR.
        if client_abort_detected_by.is_none() && tee.tracker.terminal_error_seen() {
            let completion_seen = tee.tracker.completion_seen();
            let saw_stream_output = tee.first_byte_ms.is_some()
                || forwarded_chunks > 0
                || forwarded_bytes > 0
                || drained_chunks > 0
                || drained_bytes > 0;

            // For Codex /v1/responses, completion_seen implies response.completed
            // was received (which carries usage). We treat this as a proxy for
            // usage_seen to avoid consuming tracker state before tee.finalize().
            let codex_successish = is_codex_stream_terminal_error_successish(
                &tee.ctx.cli_key,
                &tee.ctx.path,
                tee.ctx.status,
                saw_stream_output,
                completion_seen,
                completion_seen,
            );

            if codex_successish {
                tee.finalize(None);
            } else {
                let code = if tee.tracker.fake_200_detected() {
                    GatewayErrorCode::Fake200.as_str()
                } else {
                    GatewayErrorCode::StreamError.as_str()
                };
                tee.finalize(Some(code));
            }
        }

        if let Some(detected_by) = client_abort_detected_by {
            let duration_ms = tee.ctx.started.elapsed().as_millis().min(i64::MAX as u128) as i64;
            let ttfb_ms = tee.first_byte_ms.and_then(|v| {
                if v >= duration_ms as u128 {
                    return None;
                }
                Some(v.min(i64::MAX as u128) as i64)
            });
            // Flush pending partial SSE data before deciding abort/success.
            let usage = tee.tracker.finalize();
            let usage_seen = usage.is_some();
            let completion_seen = tee.tracker.completion_seen();
            let terminal_error_seen = tee.tracker.terminal_error_seen();
            let saw_stream_output = tee.first_byte_ms.is_some()
                || forwarded_chunks > 0
                || forwarded_bytes > 0
                || drained_chunks > 0
                || drained_bytes > 0;

            response_fixer::push_special_setting(
                &tee.ctx.special_settings,
                serde_json::json!({
                    "type": "client_abort",
                    "scope": "stream",
                    "reason": "client_disconnected",
                    "detected_by": detected_by,
                    "duration_ms": duration_ms,
                    "ttfb_ms": ttfb_ms,
                    "forwarded_chunks": forwarded_chunks,
                    "forwarded_bytes": forwarded_bytes,
                    "drained_chunks": drained_chunks,
                    "drained_bytes": drained_bytes,
                    "upstream_ended_normally": upstream_ended_normally,
                    "upstream_transport_error_seen": upstream_transport_error_seen,
                    "completion_seen": completion_seen,
                    "terminal_error_seen": terminal_error_seen,
                    "saw_stream_output": saw_stream_output,
                    "ts": now_unix_seconds() as i64,
                }),
            );

            // Codex SSE: only a completed/usage-bearing or cleanly ended upstream
            // drain may turn a downstream disconnect into a successful request.
            let codex_successish = !upstream_transport_error_seen
                && is_codex_client_abort_successish(
                    &tee.ctx.cli_key,
                    &tee.ctx.path,
                    tee.ctx.status,
                    saw_stream_output,
                    completion_seen,
                    usage_seen,
                    terminal_error_seen,
                    upstream_ended_normally,
                );
            if codex_successish {
                tee.finalize(None);
            } else {
                tee.finalize(Some(GatewayErrorCode::StreamAborted.as_str()));
            }
        }
    });

    Body::from_stream(RelayBodyStream::new(rx))
}

pub(in crate::gateway) struct UsageBodyBufferTeeStream<S, B, R = tauri::Wry>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
    R: tauri::Runtime,
    R::Handle: Unpin,
{
    upstream: S,
    ctx: StreamFinalizeCtx<R>,
    first_byte_ms: Option<u128>,
    buffer: Vec<u8>,
    max_bytes: usize,
    truncated: bool,
    total_timeout: Option<Duration>,
    total_sleep: Option<Pin<Box<tokio::time::Sleep>>>,
    finalized: bool,
}

impl<S, B, R> UsageBodyBufferTeeStream<S, B, R>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
    R: tauri::Runtime,
    R::Handle: Unpin,
{
    pub(in crate::gateway) fn new(
        upstream: S,
        ctx: StreamFinalizeCtx<R>,
        max_bytes: usize,
        total_timeout: Option<Duration>,
    ) -> Self {
        let remaining = total_timeout.and_then(|d| d.checked_sub(ctx.started.elapsed()));
        Self {
            upstream,
            ctx,
            first_byte_ms: None,
            buffer: Vec::new(),
            max_bytes,
            truncated: false,
            total_timeout,
            total_sleep: remaining.map(|d| Box::pin(tokio::time::sleep(d))),
            finalized: false,
        }
    }

    fn finalize(&mut self, error_code: Option<&'static str>) {
        if self.finalized {
            return;
        }
        self.finalized = true;

        let effective_error_code = if error_code.is_none()
            && !self.truncated
            && !self.buffer.is_empty()
            && is_fake_200_non_stream_body(&self.buffer)
        {
            Some(GatewayErrorCode::Fake200.as_str())
        } else {
            error_code
        };
        if effective_error_code == Some(GatewayErrorCode::Fake200.as_str()) {
            self.ctx.fake_200_detected = true;
            self.ctx.fake_200_quota_exhausted =
                upstream_client_error_rules::match_quota_exhausted(&self.buffer);
        }

        let usage = if self.truncated || self.buffer.is_empty() {
            None
        } else {
            usage::parse_usage_from_json_or_sse_bytes(&self.ctx.cli_key, &self.buffer)
        };
        let usage_metrics = usage.as_ref().map(|u| u.metrics.clone());
        let route_evidence = if self.truncated || self.buffer.is_empty() {
            usage::ModelRouteEvidence::default()
        } else {
            usage::parse_model_route_evidence_from_json_or_sse_bytes(
                &self.ctx.cli_key,
                &self.buffer,
            )
        };
        if let Some(setting) = model_route_mapping::build_model_route_mapping_setting_from_shared(
            model_route_mapping::SharedModelRouteSettingInput {
                cli_key: &self.ctx.cli_key,
                requested_model: self.ctx.requested_upstream_model.as_deref(),
                actual_model: route_evidence.first_model.as_deref(),
                conflicting_actual_model: route_evidence.first_conflicting_model.as_deref(),
                actual_reasoning_effort: route_evidence.reasoning_effort.as_deref(),
                special_settings: &self.ctx.special_settings,
                provider_id: self.ctx.provider_id,
                provider_name: &self.ctx.provider_name,
            },
        ) {
            response_fixer::push_model_route_mapping_special_setting(
                &self.ctx.special_settings,
                setting,
            );
        }
        let requested_model = self.ctx.requested_model.clone().or_else(|| {
            if self.truncated || self.buffer.is_empty() {
                None
            } else {
                route_evidence.first_model.clone()
            }
        });

        let first_byte_ms = self
            .ctx
            .upstream_output_timing
            .first_byte_ms()
            .or(self.first_byte_ms);
        let final_attempt_duration_ms = self.ctx.upstream_output_timing.final_attempt_duration_ms();
        emit_request_event_and_spawn_request_log(
            &self.ctx,
            StreamRequestCompletion::from_error_code(
                effective_error_code,
                first_byte_ms,
                first_byte_ms,
                requested_model,
                usage_metrics,
                usage,
            )
            .with_final_upstream_attempt_timing(
                final_attempt_duration_ms,
                i64::from(effective_error_code.is_none() && final_attempt_duration_ms.is_some()),
            ),
        );
    }
}

impl<S, B, R> Stream for UsageBodyBufferTeeStream<S, B, R>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
    R: tauri::Runtime,
    R::Handle: Unpin,
{
    type Item = Result<B, reqwest::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().get_mut();
        if let Some(total) = this.total_timeout {
            if this.ctx.started.elapsed() >= total {
                this.finalize(Some(GatewayErrorCode::UpstreamTimeout.as_str()));
                return Poll::Ready(None);
            }
        }

        let next = Pin::new(&mut this.upstream).poll_next(cx);

        match next {
            Poll::Pending => {
                if let Some(timer) = this.total_sleep.as_mut() {
                    if timer.as_mut().poll(cx).is_ready() {
                        this.finalize(Some(GatewayErrorCode::UpstreamTimeout.as_str()));
                        return Poll::Ready(None);
                    }
                }
                Poll::Pending
            }
            Poll::Ready(None) => {
                this.finalize(this.ctx.error_code);
                Poll::Ready(None)
            }
            Poll::Ready(Some(Ok(chunk))) => {
                if this.first_byte_ms.is_none() {
                    this.first_byte_ms = Some(this.ctx.attempt_started.elapsed().as_millis());
                }
                if !this.truncated {
                    let bytes = chunk.as_ref();
                    if this.buffer.len().saturating_add(bytes.len()) <= this.max_bytes {
                        this.buffer.extend_from_slice(bytes);
                    } else {
                        this.truncated = true;
                        this.buffer.clear();
                    }
                }
                Poll::Ready(Some(Ok(chunk)))
            }
            Poll::Ready(Some(Err(err))) => {
                this.finalize(Some(GatewayErrorCode::StreamError.as_str()));
                Poll::Ready(Some(Err(err)))
            }
        }
    }
}

impl<S, B, R> Drop for UsageBodyBufferTeeStream<S, B, R>
where
    S: Stream<Item = Result<B, reqwest::Error>> + Unpin,
    B: AsRef<[u8]>,
    R: tauri::Runtime,
    R::Handle: Unpin,
{
    fn drop(&mut self) {
        if !self.finalized {
            self.ctx.upstream_output_timing.invalidate_output();
            let usage_seen = !self.truncated
                && !self.buffer.is_empty()
                && usage::parse_usage_from_json_or_sse_bytes(&self.ctx.cli_key, &self.buffer)
                    .is_some();

            let codex_successish = is_codex_body_buffer_drop_successish(
                &self.ctx.cli_key,
                &self.ctx.path,
                self.ctx.status,
                self.first_byte_ms.is_some(),
                usage_seen,
            );

            if codex_successish {
                self.finalize(None);
            } else {
                self.finalize(Some(GatewayErrorCode::StreamAborted.as_str()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        invalidate_for_downstream_backpressure, is_codex_body_buffer_drop_successish,
        is_codex_client_abort_successish, is_codex_drop_successish, is_codex_responses_path,
        is_codex_stream_tail_error_successish, is_codex_stream_terminal_error_successish,
        is_plugin_stream_error_chunk, next_item, spawn_touch_activity,
        spawn_upstream_output_timing_stream, spawn_usage_sse_relay_body, RelayBodyStream,
        StreamFinalizeCtx, UpstreamModelObserverStream, UsageSseTeeStream,
    };
    use crate::gateway::active_requests::{ActiveRequestRegistry, ActiveRequestStart};
    use crate::gateway::proxy::GatewayErrorCode;
    use crate::gateway::streams::{StreamActivityTracker, UpstreamOutputTiming};
    use crate::{circuit_breaker, db, request_logs, session_manager, usage};
    use axum::body::Bytes;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    fn test_stream_finalize_ctx(
        app: tauri::AppHandle<tauri::test::MockRuntime>,
        db: db::Db,
        log_tx: tokio::sync::mpsc::Sender<request_logs::RequestLogInsert>,
        active_requests: Arc<ActiveRequestRegistry>,
    ) -> StreamFinalizeCtx<tauri::test::MockRuntime> {
        let session = Arc::new(session_manager::SessionManager::new());
        let route_generation = session.capture_route_generation("codex");
        StreamFinalizeCtx {
            app,
            db,
            log_tx,
            plugin_pipeline: crate::gateway::plugins::pipeline::GatewayPluginPipeline::empty_shared(
            ),
            circuit: Arc::new(circuit_breaker::CircuitBreaker::new(
                circuit_breaker::CircuitBreakerConfig::default(),
                HashMap::new(),
                None,
            )),
            session,
            route_generation,
            session_id: Some("sess-usage-tee-drain".to_string()),
            enable_session_reuse: true,
            session_binding_allowed: true,
            sort_mode_id: None,
            trace_id: "trace-usage-tee-drain".to_string(),
            cli_key: "codex".to_string(),
            method: "POST".to_string(),
            path: "/v1/responses".to_string(),
            observe: true,
            query: None,
            excluded_from_stats: false,
            special_settings: Arc::new(Mutex::new(Vec::new())),
            provider_health_neutral: false,
            status: 200,
            error_category: None,
            error_code: None,
            started: Instant::now(),
            attempt_started: Instant::now(),
            attempts: Vec::new(),
            attempts_json: "[]".to_string(),
            requested_model: None,
            requested_upstream_model: None,
            managed_model_route: false,
            created_at_ms: 1_700_000_000_000,
            created_at: 1_700_000_000,
            provider_cooldown_secs: 0,
            upstream_first_byte_timeout_secs: 300,
            upstream_retry_policy: crate::settings::UpstreamRetryPolicy::default(),
            detect_stream_internal_errors: true,
            provider_id: 1,
            provider_name: "test-provider".to_string(),
            base_url: "https://upstream.example".to_string(),
            auth_mode: "api_key".to_string(),
            upstream_route_tracker: Arc::new(Mutex::new(usage::SseUsageTracker::new("codex"))),
            observed_upstream_model: Arc::new(Mutex::new(None)),
            observed_upstream_conflicting_model: Arc::new(Mutex::new(None)),
            observed_upstream_reasoning_effort: Arc::new(Mutex::new(None)),
            fake_200_detected: false,
            fake_200_quota_exhausted: false,
            activity: Arc::new(Mutex::new(StreamActivityTracker::new(
                "trace-usage-tee-drain",
                "codex",
                1_700_000_000_000,
            ))),
            active_requests,
            upstream_output_timing: Default::default(),
        }
    }

    fn active_request_start(trace_id: &str) -> ActiveRequestStart {
        ActiveRequestStart {
            trace_id: trace_id.to_string(),
            cli_key: "codex".to_string(),
            method: "POST".to_string(),
            path: "/v1/responses".to_string(),
            query: None,
            session_id: Some("sess-usage-tee-drain".to_string()),
            requested_model: Some("gpt-5".to_string()),
            special_settings_json: None,
            created_at_ms: 1_700_000_000_000,
        }
    }

    #[test]
    fn codex_responses_path_accepts_v1_and_backend_style_paths() {
        assert!(is_codex_responses_path("codex", "/v1/responses"));
        assert!(is_codex_responses_path("codex", "/responses"));
        assert!(is_codex_responses_path("codex", "/v1/responses/"));
        assert!(is_codex_responses_path("codex", "/v1/codex/responses"));
        assert!(!is_codex_responses_path("claude", "/v1/responses"));
        assert!(!is_codex_responses_path("codex", "/v1/chat/completions"));
    }

    #[test]
    fn stream_activity_tracker_flushes_at_most_every_30_seconds() {
        let mut tracker = StreamActivityTracker::new("trace-a", "codex", 1_000);
        assert!(!tracker.observe_chunk_at(10_000));
        assert!(!tracker.observe_chunk_at(30_999));
        assert!(tracker.observe_chunk_at(31_000));
        assert!(!tracker.observe_chunk_at(45_000));
        assert!(tracker.observe_chunk_at(61_000));
    }

    #[test]
    fn spawn_touch_activity_updates_active_registry_immediately() {
        let app = tauri::test::mock_app();
        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(&db_dir.path().join("usage-tee-touch.sqlite"))
            .expect("init test db");
        let (log_tx, _log_rx) = tokio::sync::mpsc::channel(4);
        let active_requests = Arc::new(ActiveRequestRegistry::default());
        active_requests.register(active_request_start("trace-usage-tee-drain"));
        let ctx =
            test_stream_finalize_ctx(app.handle().clone(), db, log_tx, active_requests.clone());

        spawn_touch_activity(&ctx, 1_700_000_045_000, None);

        assert_eq!(
            active_requests.snapshot()[0].last_activity_ms,
            1_700_000_045_000
        );
    }

    #[test]
    fn plugin_stream_error_chunk_is_still_detected_without_rewriting_marker() {
        let chunk = Bytes::from_static(
            b": aio-plugin-error\nevent: error\ndata: {\"error\":\"plugin_failed\"}\n\n",
        );
        assert!(is_plugin_stream_error_chunk(chunk.as_ref()));
        assert_eq!(
            std::str::from_utf8(chunk.as_ref()).expect("utf8"),
            ": aio-plugin-error\nevent: error\ndata: {\"error\":\"plugin_failed\"}\n\n"
        );
    }

    #[test]
    fn timing_freeze_requires_exact_protocol_completion_or_clean_eof() {
        let mut tracker = usage::SseUsageTracker::new("codex");
        tracker.ingest_chunk(b"data: {\"status\":\"completed\"}\n\n");
        assert!(tracker.completion_seen());
        assert!(!tracker.protocol_completion_seen());

        tracker.ingest_chunk(b"data: {\"type\":\"response.item.completed\"}\n\n");
        assert!(tracker.completion_seen());
        assert!(!tracker.protocol_completion_seen());

        tracker.ingest_chunk(
            b"event: response.completed\ndata: {\"type\":\"response.completed\"}\n\n",
        );
        assert!(tracker.protocol_completion_seen());
    }

    #[test]
    fn downstream_backpressure_only_invalidates_an_unfinished_attempt() {
        let before_completion = UpstreamOutputTiming::default();
        invalidate_for_downstream_backpressure(&before_completion);
        before_completion.observe_protocol_completion_at(10);
        assert_eq!(before_completion.final_attempt_duration_ms(), None);

        let after_completion = UpstreamOutputTiming::from_buffered_prefix(Some(1), Some(5));
        after_completion.observe_protocol_completion_at(10);
        invalidate_for_downstream_backpressure(&after_completion);
        assert_eq!(after_completion.final_attempt_duration_ms(), Some(10));
        assert_eq!(after_completion.duration_ms(), None);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn upstream_route_observer_tracks_chunked_model_and_reasoning_effort() {
        let observed_model = Arc::new(Mutex::new(None));
        let observed_conflict = Arc::new(Mutex::new(None));
        let observed_effort = Arc::new(Mutex::new(None));
        let route_tracker = Arc::new(Mutex::new(usage::SseUsageTracker::new("codex")));
        let (upstream_tx, upstream_rx) =
            tokio::sync::mpsc::channel::<Result<Bytes, reqwest::Error>>(4);
        upstream_tx
            .send(Ok(Bytes::from_static(
                b"event: response.completed\ndata: {\"type\":\"response.compl",
            )))
            .await
            .expect("send first chunk");
        upstream_tx
            .send(Ok(Bytes::from_static(
                b"eted\",\"response\":{\"model\":\"gpt-5.5\",\"reasoning\":{\"effort\":\"high\"}}}\n\n",
            )))
            .await
            .expect("send second chunk");
        drop(upstream_tx);

        let mut observer = UpstreamModelObserverStream::new(
            RelayBodyStream::new(upstream_rx),
            route_tracker,
            observed_model.clone(),
            observed_conflict,
            observed_effort.clone(),
        );
        while let Some(item) = next_item(&mut observer).await {
            item.expect("observed chunk");
        }

        assert_eq!(
            observed_model.lock().expect("model lock").as_deref(),
            Some("gpt-5.5")
        );
        assert_eq!(
            observed_effort.lock().expect("effort lock").as_deref(),
            Some("high")
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn upstream_route_observer_finalizes_unterminated_tail_before_error() {
        let observed_model = Arc::new(Mutex::new(None));
        let observed_conflict = Arc::new(Mutex::new(None));
        let observed_effort = Arc::new(Mutex::new(None));
        let route_tracker = Arc::new(Mutex::new(usage::SseUsageTracker::new("codex")));
        let (upstream_tx, upstream_rx) =
            tokio::sync::mpsc::channel::<Result<Bytes, reqwest::Error>>(4);
        upstream_tx
            .send(Ok(Bytes::from_static(
                b"event: response.completed\ndata: {\"response\":{\"model\":\"gpt-5.5\",\"reasoning_effort\":\"xhigh\"}}",
            )))
            .await
            .expect("send unterminated event");
        let read_error = reqwest::Client::new()
            .get("://invalid-url")
            .build()
            .expect_err("invalid URL should produce a reqwest error");
        upstream_tx
            .send(Err(read_error))
            .await
            .expect("send read error");
        drop(upstream_tx);

        let mut observer = UpstreamModelObserverStream::new(
            RelayBodyStream::new(upstream_rx),
            route_tracker,
            observed_model.clone(),
            observed_conflict,
            observed_effort.clone(),
        );
        next_item(&mut observer)
            .await
            .expect("body chunk")
            .expect("body chunk should succeed");
        assert!(next_item(&mut observer).await.expect("read error").is_err());

        assert_eq!(
            observed_model.lock().expect("model lock").as_deref(),
            Some("gpt-5.5")
        );
        assert_eq!(
            observed_effort.lock().expect("effort lock").as_deref(),
            Some("xhigh")
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn upstream_route_observer_does_not_snapshot_parse_small_pending_fragments() {
        let observed_model = Arc::new(Mutex::new(None));
        let observed_conflict = Arc::new(Mutex::new(None));
        let observed_effort = Arc::new(Mutex::new(None));
        let route_tracker = Arc::new(Mutex::new(usage::SseUsageTracker::new("codex")));
        let payload = format!(
            "event: response.completed\ndata: {{\"type\":\"response.completed\",\"response\":{{\"model\":\"gpt-5.5\",\"reasoning\":{{\"effort\":\"high\"}},\"padding\":\"{}\"}}}}",
            "x".repeat(1024 * 1024 - 4096)
        );
        let chunks = payload
            .as_bytes()
            .chunks(1024)
            .map(Bytes::copy_from_slice)
            .collect::<Vec<_>>();
        let chunk_count = chunks.len();
        let (upstream_tx, upstream_rx) =
            tokio::sync::mpsc::channel::<Result<Bytes, reqwest::Error>>(chunk_count + 1);
        for chunk in chunks {
            upstream_tx.send(Ok(chunk)).await.expect("send fragment");
        }

        let mut observer = UpstreamModelObserverStream::new(
            RelayBodyStream::new(upstream_rx),
            route_tracker.clone(),
            observed_model.clone(),
            observed_conflict,
            observed_effort.clone(),
        );
        for _ in 0..chunk_count {
            next_item(&mut observer)
                .await
                .expect("fragment")
                .expect("fragment should succeed");
        }

        assert_eq!(
            route_tracker
                .lock()
                .expect("route tracker lock")
                .event_json_parse_attempts(),
            0
        );
        assert!(observed_model.lock().expect("model lock").is_none());
        assert!(observed_effort.lock().expect("effort lock").is_none());

        drop(upstream_tx);
        assert!(next_item(&mut observer).await.is_none());
        assert_eq!(
            observed_model.lock().expect("model lock").as_deref(),
            Some("gpt-5.5")
        );
        assert_eq!(
            observed_effort.lock().expect("effort lock").as_deref(),
            Some("high")
        );
        assert_eq!(
            route_tracker
                .lock()
                .expect("route tracker lock")
                .event_json_parse_attempts(),
            1
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn outer_tee_drop_logs_unterminated_route_before_observer_drop() {
        let app = tauri::test::mock_app();
        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(&db_dir.path().join("usage-tee-observer-drop.sqlite"))
            .expect("init test db");
        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
        let active_requests = Arc::new(ActiveRequestRegistry::default());
        active_requests.register(active_request_start("trace-usage-tee-drain"));
        let mut ctx = test_stream_finalize_ctx(app.handle().clone(), db, log_tx, active_requests);
        ctx.requested_model = Some("gpt-5.5".to_string());
        ctx.requested_upstream_model = Some("gpt-5.5".to_string());
        let observed_model = ctx.observed_upstream_model.clone();
        let observed_conflict = ctx.observed_upstream_conflicting_model.clone();
        let observed_effort = ctx.observed_upstream_reasoning_effort.clone();
        let route_tracker = ctx.upstream_route_tracker.clone();

        let (upstream_tx, upstream_rx) =
            tokio::sync::mpsc::channel::<Result<Bytes, reqwest::Error>>(4);
        upstream_tx
            .send(Ok(Bytes::from_static(
                b"event: response.completed\ndata: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\",\"model\":\"gpt-5.4-mini\",\"reasoning\":{\"effort\":\"high\"},\"usage\":{\"input_tokens\":1,\"output_tokens\":2,\"total_tokens\":3}}}",
            )))
            .await
            .expect("send unterminated completion");

        let observer = UpstreamModelObserverStream::new(
            RelayBodyStream::new(upstream_rx),
            route_tracker.clone(),
            observed_model.clone(),
            observed_conflict,
            observed_effort.clone(),
        );
        let mut stream = UsageSseTeeStream::new(observer, ctx, None, None);
        next_item(&mut stream)
            .await
            .expect("body chunk")
            .expect("body chunk should succeed");

        assert!(observed_model.lock().expect("model lock").is_none());
        assert!(observed_effort.lock().expect("effort lock").is_none());
        assert_eq!(
            route_tracker
                .lock()
                .expect("route tracker lock")
                .event_json_parse_attempts(),
            0
        );

        drop(stream);
        drop(upstream_tx);

        assert_eq!(
            observed_model.lock().expect("model lock").as_deref(),
            Some("gpt-5.4-mini")
        );
        assert_eq!(
            observed_effort.lock().expect("effort lock").as_deref(),
            Some("high")
        );

        let log = tokio::time::timeout(Duration::from_secs(2), log_rx.recv())
            .await
            .expect("request log should be enqueued")
            .expect("request log channel should stay open");
        let special_settings = log
            .special_settings_json
            .as_deref()
            .expect("route setting json");
        assert!(special_settings.contains("\"actualModel\":\"gpt-5.4-mini\""));
        assert!(special_settings.contains("\"actualReasoningEffort\":\"high\""));
    }

    #[test]
    fn codex_client_abort_successish_rejects_terminal_marker_without_completion_or_usage() {
        assert!(!is_codex_client_abort_successish(
            "codex",
            "/v1/responses",
            200,
            true,
            false,
            false,
            true,
            false
        ));
    }

    #[test]
    fn codex_client_abort_successish_requires_completion_or_usage() {
        assert!(!is_codex_client_abort_successish(
            "codex",
            "/v1/responses",
            200,
            true,
            false,
            false,
            false,
            false
        ));
        assert!(is_codex_client_abort_successish(
            "codex",
            "/v1/responses",
            200,
            true,
            true,
            false,
            false,
            false
        ));
    }

    #[test]
    fn codex_client_abort_successish_accepts_completion_usage_or_clean_eof() {
        assert!(is_codex_client_abort_successish(
            "codex",
            "/v1/responses",
            200,
            true,
            true,
            false,
            false,
            false
        ));
        assert!(is_codex_client_abort_successish(
            "codex",
            "/v1/responses",
            200,
            true,
            false,
            true,
            false,
            false
        ));
        assert!(is_codex_client_abort_successish(
            "codex",
            "/v1/responses",
            200,
            true,
            false,
            false,
            false,
            true
        ));
        assert!(!is_codex_client_abort_successish(
            "codex",
            "/v1/responses",
            200,
            true,
            true,
            false,
            true,
            true
        ));
    }

    #[test]
    fn codex_drop_successish_allows_completion_or_usage_with_terminal_marker() {
        assert!(is_codex_drop_successish(
            "codex",
            "/v1/responses",
            200,
            true,
            true,
            false,
            true
        ));
        assert!(is_codex_drop_successish(
            "codex",
            "/v1/responses",
            200,
            true,
            false,
            true,
            true
        ));
        assert!(!is_codex_drop_successish(
            "codex",
            "/v1/responses",
            200,
            true,
            false,
            false,
            true
        ));
    }

    #[test]
    fn codex_body_buffer_drop_successish_when_usage_seen() {
        assert!(is_codex_body_buffer_drop_successish(
            "codex",
            "/v1/responses",
            200,
            true,
            true
        ));
        assert!(is_codex_body_buffer_drop_successish(
            "codex",
            "/responses",
            204,
            true,
            true
        ));
    }

    #[test]
    fn codex_body_buffer_drop_successish_requires_usage_and_stream_output() {
        assert!(!is_codex_body_buffer_drop_successish(
            "codex",
            "/v1/responses",
            200,
            true,
            false
        ));
        assert!(!is_codex_body_buffer_drop_successish(
            "codex",
            "/v1/responses",
            200,
            false,
            true
        ));
        assert!(!is_codex_body_buffer_drop_successish(
            "claude",
            "/v1/responses",
            200,
            true,
            true
        ));
        assert!(!is_codex_body_buffer_drop_successish(
            "codex",
            "/v1/chat/completions",
            200,
            true,
            true
        ));
    }

    #[test]
    fn codex_stream_terminal_error_successish_with_completion_and_usage() {
        assert!(is_codex_stream_terminal_error_successish(
            "codex",
            "/v1/responses",
            200,
            true,
            true,
            true
        ));
        assert!(is_codex_stream_terminal_error_successish(
            "codex",
            "/responses",
            200,
            true,
            true,
            false
        ));
        assert!(is_codex_stream_terminal_error_successish(
            "codex",
            "/v1/responses",
            200,
            true,
            false,
            true
        ));
    }

    #[test]
    fn codex_stream_terminal_error_not_successish_without_completion_or_usage() {
        assert!(!is_codex_stream_terminal_error_successish(
            "codex",
            "/v1/responses",
            200,
            true,
            false,
            false
        ));
    }

    #[test]
    fn codex_stream_terminal_error_not_successish_for_non_codex() {
        assert!(!is_codex_stream_terminal_error_successish(
            "claude",
            "/v1/responses",
            200,
            true,
            true,
            true
        ));
    }

    #[test]
    fn codex_stream_terminal_error_not_successish_without_stream_output() {
        assert!(!is_codex_stream_terminal_error_successish(
            "codex",
            "/v1/responses",
            200,
            false,
            true,
            true
        ));
    }

    #[test]
    fn codex_stream_terminal_error_not_successish_on_error_status() {
        assert!(!is_codex_stream_terminal_error_successish(
            "codex",
            "/v1/responses",
            500,
            true,
            true,
            true
        ));
    }

    #[test]
    fn codex_stream_tail_error_successish_after_completion() {
        assert!(is_codex_stream_tail_error_successish(
            "codex",
            "/v1/responses",
            200,
            true,
            true,
            false
        ));
        assert!(!is_codex_stream_tail_error_successish(
            "codex",
            "/v1/responses",
            200,
            true,
            false,
            false
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn upstream_timing_freezes_at_protocol_completion_before_delayed_eof() {
        let timing = UpstreamOutputTiming::default();
        let attempt_started = Instant::now();
        let (upstream_tx, upstream_rx) =
            tokio::sync::mpsc::channel::<Result<Bytes, reqwest::Error>>(4);
        let mut stream = spawn_upstream_output_timing_stream(
            RelayBodyStream::new(upstream_rx),
            "codex",
            timing.clone(),
            attempt_started,
            0,
        );

        tokio::time::sleep(Duration::from_millis(2)).await;
        upstream_tx
            .send(Ok(Bytes::from_static(
                b"event: response.completed\ndata: {\"type\":\"response.completed\"}\n\n",
            )))
            .await
            .expect("send completion");
        next_item(&mut stream)
            .await
            .expect("completion chunk")
            .expect("completion chunk should succeed");
        let frozen = timing
            .final_attempt_duration_ms()
            .expect("completion must freeze timing");

        tokio::time::sleep(Duration::from_millis(20)).await;
        drop(upstream_tx);
        assert!(next_item(&mut stream).await.is_none());
        assert_eq!(timing.final_attempt_duration_ms(), Some(frozen));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn upstream_timing_uses_clean_eof_without_exact_completion() {
        let timing = UpstreamOutputTiming::default();
        let attempt_started = Instant::now();
        let (upstream_tx, upstream_rx) =
            tokio::sync::mpsc::channel::<Result<Bytes, reqwest::Error>>(4);
        let mut stream = spawn_upstream_output_timing_stream(
            RelayBodyStream::new(upstream_rx),
            "codex",
            timing.clone(),
            attempt_started,
            0,
        );

        tokio::time::sleep(Duration::from_millis(2)).await;
        upstream_tx
            .send(Ok(Bytes::from_static(
                b"data: {\"type\":\"response.output_text.delta\",\"delta\":\"ok\"}\n\n",
            )))
            .await
            .expect("send output");
        drop(upstream_tx);
        while let Some(item) = next_item(&mut stream).await {
            item.expect("output chunk should succeed");
        }
        assert!(timing.final_attempt_duration_ms().is_some());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn upstream_read_error_invalidates_a_previously_frozen_attempt() {
        let timing = UpstreamOutputTiming::default();
        let attempt_started = Instant::now();
        let (upstream_tx, upstream_rx) =
            tokio::sync::mpsc::channel::<Result<Bytes, reqwest::Error>>(4);
        let mut stream = spawn_upstream_output_timing_stream(
            RelayBodyStream::new(upstream_rx),
            "codex",
            timing.clone(),
            attempt_started,
            0,
        );

        tokio::time::sleep(Duration::from_millis(2)).await;
        upstream_tx
            .send(Ok(Bytes::from_static(
                b"event: response.completed\ndata: {\"type\":\"response.completed\"}\n\n",
            )))
            .await
            .expect("send completion");
        next_item(&mut stream)
            .await
            .expect("completion chunk")
            .expect("completion chunk should succeed");
        assert!(timing.final_attempt_duration_ms().is_some());

        let read_error = reqwest::Client::new()
            .get("://invalid-url")
            .build()
            .expect_err("invalid URL should produce a reqwest error");
        upstream_tx
            .send(Err(read_error))
            .await
            .expect("send read error");
        drop(upstream_tx);
        let mut saw_error = false;
        while let Some(item) = next_item(&mut stream).await {
            saw_error |= item.is_err();
        }
        assert!(saw_error);
        assert_eq!(timing.final_attempt_duration_ms(), None);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn upstream_terminal_failure_invalidates_a_previously_frozen_attempt() {
        let timing = UpstreamOutputTiming::default();
        let attempt_started = Instant::now();
        let (upstream_tx, upstream_rx) =
            tokio::sync::mpsc::channel::<Result<Bytes, reqwest::Error>>(4);
        let mut stream = spawn_upstream_output_timing_stream(
            RelayBodyStream::new(upstream_rx),
            "codex",
            timing.clone(),
            attempt_started,
            0,
        );

        tokio::time::sleep(Duration::from_millis(2)).await;
        upstream_tx
            .send(Ok(Bytes::from_static(
                b"event: response.completed\ndata: {\"type\":\"response.completed\"}\n\n",
            )))
            .await
            .expect("send completion");
        next_item(&mut stream)
            .await
            .expect("completion chunk")
            .expect("completion chunk should succeed");
        assert!(timing.final_attempt_duration_ms().is_some());

        upstream_tx
            .send(Ok(Bytes::from_static(
                b"event: response.failed\ndata: {\"type\":\"response.failed\"}\n\n",
            )))
            .await
            .expect("send terminal failure");
        drop(upstream_tx);
        while let Some(item) = next_item(&mut stream).await {
            item.expect("terminal failure chunk should remain transport-valid");
        }
        assert_eq!(timing.final_attempt_duration_ms(), None);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn codex_disconnect_drain_ignores_stream_idle_timeout_until_completion() {
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();
        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(&db_dir.path().join("usage-tee-drain.sqlite"))
            .expect("init test db");
        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
        let active_requests = Arc::new(ActiveRequestRegistry::default());
        active_requests.register(active_request_start("trace-usage-tee-drain"));
        let ctx = test_stream_finalize_ctx(app_handle, db, log_tx, active_requests.clone());
        let upstream_timing = ctx.upstream_output_timing.clone();
        let attempt_started = ctx.attempt_started;
        let (upstream_tx, upstream_rx) =
            tokio::sync::mpsc::channel::<Result<Bytes, reqwest::Error>>(4);

        let timed_upstream = spawn_upstream_output_timing_stream(
            RelayBodyStream::new(upstream_rx),
            "codex",
            upstream_timing,
            attempt_started,
            0,
        );

        let body = spawn_usage_sse_relay_body(
            timed_upstream,
            ctx,
            Some(Duration::from_millis(10)),
            None,
            false,
        );

        upstream_tx
            .send(Ok(Bytes::from_static(
                b"data: {\"type\":\"response.output_text.delta\",\"delta\":\"hello\"}\n\n",
            )))
            .await
            .expect("send first output chunk");

        let mut body_stream = body.into_data_stream();
        let first = tokio::time::timeout(Duration::from_secs(1), next_item(&mut body_stream))
            .await
            .expect("first output chunk should arrive")
            .expect("body should yield first output")
            .expect("first output should be ok");
        assert!(first.as_ref().starts_with(b"data:"));

        drop(body_stream);

        let completion_tx = upstream_tx;
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            let _ = completion_tx
                .send(Ok(Bytes::from_static(
                    b"event: response.completed\n\
                      data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\",\"model\":\"gpt-5\",\"usage\":{\"input_tokens\":1,\"output_tokens\":2,\"total_tokens\":3}}}\n\n",
                )))
                .await;
        });

        let log = tokio::time::timeout(Duration::from_secs(2), log_rx.recv())
            .await
            .expect("request log should be enqueued")
            .expect("request log channel should stay open");

        assert_eq!(log.error_code, None);
        assert_eq!(log.status, Some(200));
        assert_eq!(log.input_tokens, Some(1));
        assert_eq!(log.output_tokens, Some(2));
        assert_eq!(log.total_tokens, Some(3));
        assert!(log.final_upstream_attempt_duration_ms.is_some());
        assert_eq!(log.final_upstream_attempt_timing_version, 1);
        assert!(log
            .special_settings_json
            .as_deref()
            .is_some_and(|value| value.contains("\"client_abort\"")));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn codex_disconnect_drain_invalidates_completion_followed_by_terminal_failure() {
        let app = tauri::test::mock_app();
        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(&db_dir.path().join("usage-tee-drain-failed.sqlite"))
            .expect("init test db");
        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
        let active_requests = Arc::new(ActiveRequestRegistry::default());
        active_requests.register(active_request_start("trace-usage-tee-drain"));
        let ctx = test_stream_finalize_ctx(app.handle().clone(), db, log_tx, active_requests);
        let upstream_timing = ctx.upstream_output_timing.clone();
        let attempt_started = ctx.attempt_started;
        let (upstream_tx, upstream_rx) =
            tokio::sync::mpsc::channel::<Result<Bytes, reqwest::Error>>(4);

        let timed_upstream = spawn_upstream_output_timing_stream(
            RelayBodyStream::new(upstream_rx),
            "codex",
            upstream_timing,
            attempt_started,
            0,
        );
        let body = spawn_usage_sse_relay_body(
            timed_upstream,
            ctx,
            Some(Duration::from_millis(500)),
            None,
            false,
        );

        upstream_tx
            .send(Ok(Bytes::from_static(
                b"data: {\"type\":\"response.output_text.delta\",\"delta\":\"hello\"}\n\n",
            )))
            .await
            .expect("send first output chunk");
        let mut body_stream = body.into_data_stream();
        next_item(&mut body_stream)
            .await
            .expect("body should yield first output")
            .expect("first output should be ok");
        drop(body_stream);

        upstream_tx
            .send(Ok(Bytes::from_static(
                b"event: response.completed\n\
                  data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\",\"usage\":{\"input_tokens\":1,\"output_tokens\":2,\"total_tokens\":3}}}\n\n",
            )))
            .await
            .expect("send completion");
        upstream_tx
            .send(Ok(Bytes::from_static(
                b"event: response.failed\n\
                  data: {\"type\":\"response.failed\",\"response\":{\"status\":\"failed\"}}\n\n",
            )))
            .await
            .expect("send terminal failure");
        drop(upstream_tx);

        let log = tokio::time::timeout(Duration::from_secs(2), log_rx.recv())
            .await
            .expect("request log should be enqueued")
            .expect("request log channel should stay open");
        assert_eq!(
            log.error_code,
            Some(GatewayErrorCode::StreamAborted.as_str().to_string())
        );
        assert_eq!(log.final_upstream_attempt_duration_ms, None);
        assert_eq!(log.final_upstream_attempt_timing_version, 0);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn codex_disconnect_drain_transport_error_remains_aborted() {
        let app = tauri::test::mock_app();
        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(&db_dir.path().join("usage-tee-drain-read-error.sqlite"))
            .expect("init test db");
        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
        let active_requests = Arc::new(ActiveRequestRegistry::default());
        active_requests.register(active_request_start("trace-usage-tee-drain"));
        let ctx = test_stream_finalize_ctx(app.handle().clone(), db, log_tx, active_requests);
        let upstream_timing = ctx.upstream_output_timing.clone();
        let attempt_started = ctx.attempt_started;
        let (upstream_tx, upstream_rx) =
            tokio::sync::mpsc::channel::<Result<Bytes, reqwest::Error>>(4);

        let timed_upstream = spawn_upstream_output_timing_stream(
            RelayBodyStream::new(upstream_rx),
            "codex",
            upstream_timing,
            attempt_started,
            0,
        );
        let body = spawn_usage_sse_relay_body(
            timed_upstream,
            ctx,
            Some(Duration::from_millis(500)),
            None,
            false,
        );

        upstream_tx
            .send(Ok(Bytes::from_static(
                b"data: {\"type\":\"response.output_text.delta\",\"delta\":\"hello\"}\n\n",
            )))
            .await
            .expect("send first output chunk");
        let mut body_stream = body.into_data_stream();
        next_item(&mut body_stream)
            .await
            .expect("body should yield first output")
            .expect("first output should be ok");
        drop(body_stream);

        upstream_tx
            .send(Ok(Bytes::from_static(
                b"event: response.completed\n\
                  data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\",\"usage\":{\"input_tokens\":1,\"output_tokens\":2,\"total_tokens\":3}}}\n\n",
            )))
            .await
            .expect("send completion");
        let read_error = reqwest::Client::new()
            .get("://invalid-url")
            .build()
            .expect_err("invalid URL should produce a reqwest error");
        upstream_tx
            .send(Err(read_error))
            .await
            .expect("send read error");
        drop(upstream_tx);

        let log = tokio::time::timeout(Duration::from_secs(2), log_rx.recv())
            .await
            .expect("request log should be enqueued")
            .expect("request log channel should stay open");
        assert_eq!(
            log.error_code,
            Some(GatewayErrorCode::StreamAborted.as_str().to_string())
        );
        assert_eq!(log.final_upstream_attempt_duration_ms, None);
        assert_eq!(log.final_upstream_attempt_timing_version, 0);
        assert!(log
            .special_settings_json
            .as_deref()
            .is_some_and(|value| value.contains("\"type\":\"client_abort\"")
                && value.contains("\"upstream_transport_error_seen\":true")));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn stream_idle_timeout_fires_after_configured_silence() {
        let app = tauri::test::mock_app();
        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(&db_dir.path().join("usage-tee-idle-timeout.sqlite"))
            .expect("init test db");
        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
        let active_requests = Arc::new(ActiveRequestRegistry::default());
        active_requests.register(active_request_start("trace-usage-tee-drain"));
        let ctx = test_stream_finalize_ctx(app.handle().clone(), db, log_tx, active_requests);
        let (upstream_tx, upstream_rx) =
            tokio::sync::mpsc::channel::<Result<Bytes, reqwest::Error>>(4);

        let body = spawn_usage_sse_relay_body(
            RelayBodyStream::new(upstream_rx),
            ctx,
            Some(Duration::from_millis(500)),
            None,
            false,
        );
        let mut body_stream = body.into_data_stream();

        // Chunk gaps (100ms) are below the idle window (500ms) but sum above
        // it: all chunks arriving proves each chunk resets the timer. The 5x
        // margin absorbs scheduler hiccups on loaded CI runners; tokio::time
        // pause is not an option because the request log is delivered from
        // tauri's separate real-time runtime.
        for _ in 0..3 {
            upstream_tx
                .send(Ok(Bytes::from_static(
                    b"data: {\"type\":\"response.output_text.delta\",\"delta\":\"hello\"}\n\n",
                )))
                .await
                .expect("send output chunk");
            let chunk = tokio::time::timeout(Duration::from_secs(1), next_item(&mut body_stream))
                .await
                .expect("output chunk should arrive")
                .expect("body should yield output chunk")
                .expect("output chunk should be ok");
            assert!(chunk.as_ref().starts_with(b"data:"));
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // Upstream goes silent without closing; downstream stays connected.
        // The idle timeout must end the body stream.
        let end = tokio::time::timeout(Duration::from_secs(2), next_item(&mut body_stream))
            .await
            .expect("idle timeout should end the body stream");
        assert!(end.is_none());

        let log = tokio::time::timeout(Duration::from_secs(2), log_rx.recv())
            .await
            .expect("request log should be enqueued")
            .expect("request log channel should stay open");
        assert_eq!(
            log.error_code,
            Some(GatewayErrorCode::StreamIdleTimeout.as_str().to_string())
        );
        drop(upstream_tx);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn stream_idle_timeout_disabled_keeps_stream_open() {
        let app = tauri::test::mock_app();
        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(&db_dir.path().join("usage-tee-idle-disabled.sqlite"))
            .expect("init test db");
        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
        let active_requests = Arc::new(ActiveRequestRegistry::default());
        active_requests.register(active_request_start("trace-usage-tee-drain"));
        let ctx = test_stream_finalize_ctx(app.handle().clone(), db, log_tx, active_requests);
        let upstream_timing = ctx.upstream_output_timing.clone();
        let attempt_started = ctx.attempt_started;
        let (upstream_tx, upstream_rx) =
            tokio::sync::mpsc::channel::<Result<Bytes, reqwest::Error>>(4);

        let timed_upstream = spawn_upstream_output_timing_stream(
            RelayBodyStream::new(upstream_rx),
            "codex",
            upstream_timing,
            attempt_started,
            0,
        );
        let body = spawn_usage_sse_relay_body(timed_upstream, ctx, None, None, false);
        let mut body_stream = body.into_data_stream();

        upstream_tx
            .send(Ok(Bytes::from_static(
                b"data: {\"type\":\"response.output_text.delta\",\"delta\":\"hello\"}\n\n",
            )))
            .await
            .expect("send first output chunk");
        let first = tokio::time::timeout(Duration::from_secs(1), next_item(&mut body_stream))
            .await
            .expect("first output chunk should arrive")
            .expect("body should yield first output")
            .expect("first output should be ok");
        assert!(first.as_ref().starts_with(b"data:"));

        // Silence longer than any small idle window; disabled timeout must not
        // interrupt the stream.
        tokio::time::sleep(Duration::from_millis(50)).await;

        upstream_tx
            .send(Ok(Bytes::from_static(
                b"data: {\"type\":\"response.output_text.delta\",\"delta\":\"world\"}\n\n",
            )))
            .await
            .expect("send second output chunk");
        let second = tokio::time::timeout(Duration::from_secs(1), next_item(&mut body_stream))
            .await
            .expect("second output chunk should arrive")
            .expect("body should yield second output")
            .expect("second output should be ok");
        assert!(second.as_ref().starts_with(b"data:"));

        // Let the stream end normally.
        drop(upstream_tx);
        let end = tokio::time::timeout(Duration::from_secs(1), next_item(&mut body_stream))
            .await
            .expect("body stream should end after upstream closes");
        assert!(end.is_none());

        let log = tokio::time::timeout(Duration::from_secs(2), log_rx.recv())
            .await
            .expect("request log should be enqueued")
            .expect("request log channel should stay open");
        assert_eq!(log.error_code, None);
        assert!(log.final_upstream_attempt_duration_ms.is_some());
        assert_eq!(log.final_upstream_attempt_timing_version, 1);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn sse_route_mapping_prefers_route_observed_before_bridge() {
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();
        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(&db_dir.path().join("usage-tee-route.sqlite"))
            .expect("init test db");
        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
        let active_requests = Arc::new(ActiveRequestRegistry::default());
        active_requests.register(active_request_start("trace-usage-tee-drain"));
        let mut ctx = test_stream_finalize_ctx(app_handle, db, log_tx, active_requests);
        ctx.requested_model = Some("gpt-5.5".to_string());
        ctx.requested_upstream_model = Some("gpt-5.5".to_string());

        let (raw_tx, raw_rx) = tokio::sync::mpsc::channel::<Result<Bytes, reqwest::Error>>(4);
        raw_tx
            .send(Ok(Bytes::from_static(
                b"event: response.completed\n\
                  data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\",\"model\":\"gpt-5.4-mini\",\"reasoning\":{\"effort\":\"low\"}}}\n\n",
            )))
            .await
            .expect("send raw upstream completion");
        drop(raw_tx);
        let mut observer = UpstreamModelObserverStream::new(
            RelayBodyStream::new(raw_rx),
            ctx.upstream_route_tracker.clone(),
            ctx.observed_upstream_model.clone(),
            ctx.observed_upstream_conflicting_model.clone(),
            ctx.observed_upstream_reasoning_effort.clone(),
        );
        while let Some(chunk) = next_item(&mut observer).await {
            chunk.expect("raw upstream chunk");
        }

        // Simulate a bridge rewriting the response route after the observer.
        let (upstream_tx, upstream_rx) =
            tokio::sync::mpsc::channel::<Result<Bytes, reqwest::Error>>(4);
        upstream_tx
            .send(Ok(Bytes::from_static(
                b"event: response.completed\n\
                  data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\",\"model\":\"gpt-5.5\",\"reasoning\":{\"effort\":\"high\"},\"usage\":{\"input_tokens\":1,\"output_tokens\":2,\"total_tokens\":3}}}\n\n",
            )))
            .await
            .expect("send completion");
        drop(upstream_tx);
        let mut stream = UsageSseTeeStream::new(RelayBodyStream::new(upstream_rx), ctx, None, None);

        while let Some(chunk) = next_item(&mut stream).await {
            chunk.expect("stream chunk");
        }

        let log = tokio::time::timeout(Duration::from_secs(2), log_rx.recv())
            .await
            .expect("request log should be enqueued")
            .expect("request log channel should stay open");
        let special_settings = log
            .special_settings_json
            .as_deref()
            .expect("route setting json");
        assert!(special_settings.contains("\"type\":\"model_route_mapping\""));
        assert!(special_settings.contains("\"requestedModel\":\"gpt-5.5\""));
        assert!(special_settings.contains("\"actualModel\":\"gpt-5.4-mini\""));
        assert!(special_settings.contains("\"actualReasoningEffort\":\"low\""));
        assert!(special_settings.contains("\"actualReasoningEffortSource\":\"response\""));
        assert!(!special_settings.contains("\"actualModel\":\"gpt-5.5\""));
        assert!(!special_settings.contains("\"actualReasoningEffort\":\"high\""));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn sse_route_mapping_does_not_use_bridge_injected_effort() {
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();
        let db_dir = tempfile::tempdir().expect("db dir");
        let db = db::init_for_tests(&db_dir.path().join("usage-tee-route-injection.sqlite"))
            .expect("init test db");
        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel(4);
        let active_requests = Arc::new(ActiveRequestRegistry::default());
        active_requests.register(active_request_start("trace-usage-tee-drain"));
        let mut ctx = test_stream_finalize_ctx(app_handle, db, log_tx, active_requests);
        ctx.requested_model = Some("gpt-5.5".to_string());
        ctx.requested_upstream_model = Some("gpt-5.5".to_string());
        ctx.special_settings
            .lock()
            .expect("special settings lock")
            .push(serde_json::json!({
                "type": "codex_reasoning_effort",
                "effort": "high"
            }));

        let (raw_tx, raw_rx) = tokio::sync::mpsc::channel::<Result<Bytes, reqwest::Error>>(4);
        raw_tx
            .send(Ok(Bytes::from_static(
                b"event: response.completed\n\
                  data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\",\"model\":\"gpt-5.5\"}}\n\n",
            )))
            .await
            .expect("send raw upstream completion");
        drop(raw_tx);
        let mut observer = UpstreamModelObserverStream::new(
            RelayBodyStream::new(raw_rx),
            ctx.upstream_route_tracker.clone(),
            ctx.observed_upstream_model.clone(),
            ctx.observed_upstream_conflicting_model.clone(),
            ctx.observed_upstream_reasoning_effort.clone(),
        );
        while let Some(chunk) = next_item(&mut observer).await {
            chunk.expect("raw upstream chunk");
        }

        // Simulate a bridge adding an effort that was absent from the upstream payload.
        let (upstream_tx, upstream_rx) =
            tokio::sync::mpsc::channel::<Result<Bytes, reqwest::Error>>(4);
        upstream_tx
            .send(Ok(Bytes::from_static(
                b"event: response.completed\n\
                  data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\",\"model\":\"gpt-5.4-mini\",\"reasoning\":{\"effort\":\"medium\"},\"usage\":{\"input_tokens\":1,\"output_tokens\":2,\"total_tokens\":3}}}\n\n",
            )))
            .await
            .expect("send bridged completion");
        drop(upstream_tx);
        let mut stream = UsageSseTeeStream::new(RelayBodyStream::new(upstream_rx), ctx, None, None);

        while let Some(chunk) = next_item(&mut stream).await {
            chunk.expect("stream chunk");
        }

        let log = tokio::time::timeout(Duration::from_secs(2), log_rx.recv())
            .await
            .expect("request log should be enqueued")
            .expect("request log channel should stay open");
        assert!(!log
            .special_settings_json
            .as_deref()
            .is_some_and(|settings| settings.contains("\"type\":\"model_route_mapping\"")));
    }
}
