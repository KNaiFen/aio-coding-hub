# Implementation Plan

1. Add the default-on setting, view projection, dedicated command, generated
   binding contract, frontend service, and query mutation.
2. Add an all-session-binding runtime clear operation and invoke it after a
   successful setting change.
3. Thread the resolved setting through gateway runtime settings and provider
   resolution; skip all session reads, writes, sort-mode persistence, and order
   persistence when disabled.
4. Gate stream and non-stream success binding with the same request-scoped
   decision.
5. Add focused Rust and frontend regressions, regenerate bindings, then run
   formatting, type checks, lint, tests, review, and build packaging.

## Rollback

Re-enable the switch. The setting mutation clears bindings, so the next
request starts a fresh legacy session-reuse record.

## Phase 2 Plan

1. [x] Add migration v41->v42 for `session_reuse_priority` on default-route and
   sort-mode route members, with a checked `0..=1000` range and default `0`.
2. [x] Carry the priority through route read/write models and gateway provider
   projections. Preserve it when either route order is rewritten.
3. [x] Modify only session-binding promotion: retain same-tier reuse, but leave the
   configured route order unchanged when a higher-tier candidate exists.
4. [x] Add dedicated default-route and sort-mode priority commands that clear only
   the affected CLI's session bindings after successful writes.
5. [x] Add generated IPC bindings, service/query mutations, and a local-draft
   numeric priority editor in the route-order panel that persists on blur or
   Enter.
6. [x] Add migration, persistence, selection, IPC/frontend adapter, and UI
   regressions. Regenerate bindings, run focused checks, then complete the
   wider verification and commit.

## Phase 2 Verification

- Passed: Rust format and Clippy, generated bindings, TypeScript typecheck,
  lint, production frontend build, and the full frontend unit suite
  (`2602` tests).
- Passed: route-priority persistence/import, migration, provider-order, and
  lower-priority binding selection regressions.
- `cargo test --locked --lib` reached `2305` passing tests but has `42`
  environment-specific macOS filesystem-security failures in Codex-managed
  profile, image history, and unrelated skill-filesystem tests. The same
  failures occur outside the sandbox; they do not exercise this task's
  route-priority paths.
