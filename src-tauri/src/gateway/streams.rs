//! Usage: Gateway stream adapters (gunzip, relays, usage/timing tees).

mod types;
pub(super) use types::{StreamActivityTracker, StreamFinalizeCtx, UpstreamOutputTiming};

mod finalize;
mod request_end;

mod relay;
pub(super) use relay::{FirstChunkStream, RelayBodyStream};

mod gunzip;
pub(super) use gunzip::GunzipStream;

mod plugin_chunk;
pub(super) use plugin_chunk::MaybePluginChunkStream;

mod codex_responses_overload_rewrite;
pub(super) use codex_responses_overload_rewrite::CodexResponsesOverloadErrorRewriter;

mod usage_tee;
pub(super) use usage_tee::{
    spawn_upstream_body_timing_stream, spawn_upstream_output_timing_stream,
    spawn_usage_sse_relay_body, UpstreamModelObserverStream, UsageBodyBufferTeeStream,
    UsageSseTeeStream,
};

mod timing;
pub(super) use timing::TimingOnlyTeeStream;
