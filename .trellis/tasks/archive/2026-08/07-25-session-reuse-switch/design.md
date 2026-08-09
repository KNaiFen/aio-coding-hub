# Design: Configurable Session Reuse Policy

## Decision

Add a dedicated global `enable_session_reuse: bool` setting, defaulting to
`true`. It is owned by a dedicated settings writer rather than the broad
ordinary settings patch, matching other gateway feature toggles.

## Runtime Semantics

When enabled, current legacy behavior is preserved. When disabled, the gateway
uses the current active route for every request and does not consult or mutate
the in-memory session routing record. This applies to all three sticky fields:

- last successful provider
- sort-mode identity
- provider ordering snapshot

The global setting is resolved at the beginning of a request and carried as a
separate request snapshot into provider selection and success finalization.
The existing request-level `allow_session_reuse` flag continues to decide
whether a multi-turn request may put a bound provider first; it must not be
repurposed as the global switch because that would change existing behavior for
single-turn requests. Already-running requests retain their snapshot; the next
request observes a setting change.

## State Invalidation

After the durable setting update succeeds, the application clears all session
bindings from the running gateway. A dedicated all-bindings operation is used
instead of `clear_cli_route_runtime_state`, because the latter also clears the
unrelated recent-error cache.

## Boundaries

The setting is transported as follows:

`AppSettings -> SettingsView / IPC -> generated bindings -> settings service ->
React Query mutation -> CLI Manager General tab`.

The runtime value flows through:

`RuntimeSettingsMiddleware -> HandlerRuntimeSettings ->
ProviderResolutionMiddleware -> provider selection / finalization`.

## Non-goals

- No provider-weight or same-tier reuse model in this change.
- No database migration: the setting lives in the existing settings file.
- No proactive external health probing. The existing common circuit gate remains
  the authoritative source of skipped attempts and half-open recovery.

## Compatibility

Missing serialized fields deserialize to `true`; existing users retain the
legacy reuse behavior. The future per-route priority-group feature will be a
separate change and must not reuse the provider-global `priority` field.

## Phase 2: Per-Route Reuse Priority

Each member of `default_route_providers` and `sort_mode_providers` gains a
`session_reuse_priority` integer in the inclusive range `0..=1000`. A larger
number is more preferred for deciding whether a session binding may be
reused. The migration gives every existing row `0`, so all existing candidates
remain one reuse group by default.

This is deliberately separate from provider-global `priority` and route
`sort_order`:

- `sort_order` remains the normal candidate and failover sequence.
- `session_reuse_priority` only prevents a lower-priority session binding from
  being promoted above a higher-priority candidate.
- Equal priorities preserve the existing binding rotation behavior.

At request time, the gateway keeps the current route candidate list intact. If
the bound provider is present but its priority is lower than the maximum among
the current candidates, it does not rotate that provider to the front. The
existing common gate then owns all circuit, cooldown, quota, and other
availability decisions: a temporarily unavailable higher-priority provider is
recorded as skipped and the route naturally falls back; when it recovers, it
is first in the existing route order again. No proactive health probe or second
availability gate is introduced.

Updating a route-member priority uses dedicated IPC commands. After a durable
write succeeds, they clear only that CLI's session bindings. Reordering keeps
the stored priorities, and sort-mode configuration export/import carries the
new field with a serde default of `0` for older bundles. The default route is
not added to configuration bundles because that would broaden their existing
schema and ownership boundary.
