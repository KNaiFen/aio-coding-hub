//! Usage: Re-export request-end helpers for `failover_loop`.

pub(super) use crate::gateway::proxy::request_end::{
    emit_request_event_and_spawn_request_log, RequestCompletion, RequestEndArgs,
    RequestEndContextArgs, RequestEndDeps,
};
