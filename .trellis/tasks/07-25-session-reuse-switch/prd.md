# Add configurable session reuse policy

## Goal

Phase 1 adds a default-on global session-reuse switch that fully bypasses
session routing state when disabled and clears active bindings when the setting
changes.

Phase 2 adds a per-route-member session-reuse priority. Providers at the same
priority may retain the existing session binding behavior. A bound provider at
a lower priority must not be moved ahead of a currently eligible higher-priority
candidate.

## Requirements

- Add a global, user-configurable session-reuse switch in CLI Manager > General.
- Keep the switch enabled by default so existing installations retain the current behavior.
- When disabled, a request must not read, refresh, create, or update any session routing state:
  bound provider, bound sort mode, or bound provider order.
- When disabled, each request must use the currently active route and its configured provider order.
- Changing the switch must clear all in-memory session bindings without clearing recent-error state or requiring a gateway restart.
- Preserve existing circuit-gate behavior, attempt logging, route projection, and non-session failover behavior.
- Commit the completed change and produce macOS ARM64 plus portable Windows x64 build artifacts when the local build environment permits them.
- Store the priority on each default-route or sort-mode route member, rather
  than on the provider globally.
- Default existing and new members to the same priority so old routes retain
  their behavior until configured otherwise.
- A priority policy change must clear session bindings for the affected CLI,
  without clearing recent-error / circuit-related runtime state.
- Preserve priority values when a route is reordered, and include custom sort
  mode priorities in configuration import/export with backward-compatible
  defaults.

## Acceptance Criteria

- [x] `enable_session_reuse` persists with a default value of `true` for new and existing settings files.
- [x] CLI Manager > General exposes a working switch with optimistic error recovery and generated IPC bindings.
- [x] With the switch off, stale bindings cannot select, reorder, or pin a provider or sort mode.
- [x] With the switch off, successful stream and non-stream requests do not recreate bindings.
- [x] Toggling the setting clears all active bindings while retaining recent-error state.
- [x] Existing session-reuse behavior is unchanged when the switch remains enabled.
- [x] Focused Rust and frontend tests cover the setting, route selection, success finalization, and runtime clearing.
- [x] The change is reviewed, committed, and build results are recorded accurately.
- [x] Default routes and sort-mode routes can persist and expose a bounded
  session-reuse priority for every member.
- [x] A low-priority bound provider does not suppress a higher-priority route
  candidate; same-priority bindings continue to reuse the bound provider.
- [x] Priority updates clear only the affected CLI's session bindings.
- [x] Reordering, configuration import, and old configuration bundles preserve
  or default priorities correctly.

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.

## Delivery Status

Phase 1 was delivered in commit `fc41acf0` with the default-on global switch,
runtime binding invalidation, focused tests, and requested local build
artifacts. Phase 2 is delivered with the route-priority implementation and its
validation recorded in `implement.md`.
