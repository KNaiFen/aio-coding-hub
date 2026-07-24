# Design: Configurable Session Reuse

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
