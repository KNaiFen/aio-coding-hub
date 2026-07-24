# Add configurable session reuse switch

## Goal

Add a default-on global session-reuse switch that fully bypasses session routing state when disabled and clears active bindings when the setting changes.

## Requirements

- Add a global, user-configurable session-reuse switch in CLI Manager > General.
- Keep the switch enabled by default so existing installations retain the current behavior.
- When disabled, a request must not read, refresh, create, or update any session routing state:
  bound provider, bound sort mode, or bound provider order.
- When disabled, each request must use the currently active route and its configured provider order.
- Changing the switch must clear all in-memory session bindings without clearing recent-error state or requiring a gateway restart.
- Preserve existing circuit-gate behavior, attempt logging, route projection, and non-session failover behavior.
- Commit the completed change and produce macOS ARM64 plus portable Windows x64 build artifacts when the local build environment permits them.

## Acceptance Criteria

- [ ] `enable_session_reuse` persists with a default value of `true` for new and existing settings files.
- [ ] CLI Manager > General exposes a working switch with optimistic error recovery and generated IPC bindings.
- [ ] With the switch off, stale bindings cannot select, reorder, or pin a provider or sort mode.
- [ ] With the switch off, successful stream and non-stream requests do not recreate bindings.
- [ ] Toggling the setting clears all active bindings while retaining recent-error state.
- [ ] Existing session-reuse behavior is unchanged when the switch remains enabled.
- [ ] Focused Rust and frontend tests cover the setting, route selection, success finalization, and runtime clearing.
- [ ] The change is reviewed, committed, and build results are recorded accurately.

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
