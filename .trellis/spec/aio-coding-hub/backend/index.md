# AIO Coding Hub Backend Specs

Rules for the root application's Rust backend and local gateway runtime.

## Topics

- [Gateway attempt budget contract](./gateway-attempt-budget-contract.md):
  per-request provider attempts, reserved internal retries, strict model
  discovery, and cross-request circuit-breaker accounting.
- [Codex request content-encoding contract](./codex-request-content-encoding-contract.md):
  bounded decoding at the gateway boundary, supported HTTP encodings, identity
  forwarding, and local failure classification.
- [Codex context compaction observation contract](./codex-context-compaction-observation-contract.md):
  original-request signatures, fail-open classification, bounded markers, and
  strict separation from routing and HTTP request encoding.
- [Codex managed model route contract](../cross-layer/codex-managed-model-route-contract.md):
  readable profile aliases plus legacy UUID lookup, complete picker catalog
  lifecycle, one-provider routing, same-provider retry, and terminal
  wire-vs-observed route evidence.
- [Local observer and TUI contract](../cross-layer/local-observer-tui-contract.md):
  loopback-only authenticated snapshots and non-blocking read-only monitoring
  boundaries.
- [Upstream error response rule contract](./upstream-error-response-rule-contract.md):
  final-error matching, fail-open protocol rewriting, and separation from
  provider routing and health facts.

## Pre-Development Checklist

When changing gateway retry or circuit behavior:

1. Read [Gateway attempt budget contract](./gateway-attempt-budget-contract.md).
2. Identify whether each counter is request-scoped or persisted across requests.
3. Trace the effective provider retry policy, including provider overrides.
4. Keep strict helper routes explicit instead of relying on shared retry math.

When changing managed Codex alias routing or model-route detection:

1. Read [Codex managed model route contract](../cross-layer/codex-managed-model-route-contract.md).
2. Keep the managed provider as the only candidate while preserving common
   gates and same-provider retry.
3. Prove later terminal matched/unobserved evidence cannot leave a stale severe
   mapping from an earlier attempt.

When changing Codex request-body encoding:

1. Read [Codex request content-encoding contract](./codex-request-content-encoding-contract.md).
2. Keep semantic context compaction separate from HTTP request-body encoding.
3. Bound every decoded layer and preserve non-Codex transport behavior.
4. Keep decoding failures before provider selection and circuit accounting.

When changing Codex context-compaction observation:

1. Read [Codex context compaction observation contract](./codex-context-compaction-observation-contract.md).
2. Capture only bounded evidence from the original decoded request before
   plugins.
3. Keep every classifier outcome observational; malformed or future input must
   continue forwarding without a marker.
4. Test local, remote v1, remote v2, unknown, and conflicting metadata.

When changing the local observer runtime:

1. Read [Local observer and TUI contract](../cross-layer/local-observer-tui-contract.md).
2. Keep observer startup and snapshot reads best-effort; never await them on a
   request-forwarding path or hold the gateway database pool for observation.
3. Keep circuit inspection non-mutating and all projections bounded and
   secret-free.

When changing upstream error response rules:

1. Read [Upstream error response rule contract](./upstream-error-response-rule-contract.md).
2. Keep retry, failover, quota, cooldown, and circuit decisions on original
   upstream facts.
3. Apply a rewrite only at a terminal HTTP error response and fail open when
   bounded evidence is unavailable.
4. Keep attempt logs original and the request-level status client-visible.

## Quality Check

- Unit-test the attempt-budget calculation at its boundary values.
- Run route-level tests that exercise real provider retries and failover.
- Verify circuit failure counts across multiple requests.
- Run the full Rust suite after changing shared failover-loop inputs.
- Route-test managed and ordinary Codex requests together after changing
  provider selection, final wire-model tracking, or response observation.
- Verify supported Codex encodings arrive upstream as identity JSON, while
  invalid or oversized encoded bodies make zero upstream attempts.
- Prove context-compaction classification cannot change request bytes, headers,
  timeouts, provider selection, retries, circuit state, or response status.
- Prove error response rules cannot change provider selection, retry counts,
  circuit state, or attempt evidence, including a failure followed by success.
