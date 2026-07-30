# Design: Count live inference requests

## Boundary

The backend registry already stores active requests by `trace_id`, so it
already exposes one item per live request. The incorrect session count is a
frontend-only derived-value bug.

## Data Flow

1. `useRequestLogsFeed` receives the active-request snapshot.
2. `HomeRequestLogsPanel` passes the visible snapshot to the gateway helper.
3. The helper filters with the existing `isActiveInferenceRequest` predicate.
4. The result is the number of matching array entries, with no Session or
   trace identity deduplication.

## Compatibility

- Keep the existing endpoint whitelist unchanged.
- Keep loading/error availability semantics unchanged.
- Keep the dynamic accessible name concise and expose the detailed counting
  rule through the existing tooltip/description surface.
- No generated bindings, Rust DTOs, database schema, gateway lifecycle, or
  request event changes are required.

## Failure Behavior

This is a pure frontend observation. Missing or failed snapshots continue to
render `--`; malformed or newly introduced auxiliary endpoints simply remain
subject to the existing inference predicate and cannot affect request
forwarding.

## Rollout and Rollback

- Release as v0.60.37 after PR and exact-merge `main` CI.
- Rollback is limited to restoring the previous derived-count helper and Home
  copy; no persisted data or backend migration is involved.
