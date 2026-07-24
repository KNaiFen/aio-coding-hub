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
