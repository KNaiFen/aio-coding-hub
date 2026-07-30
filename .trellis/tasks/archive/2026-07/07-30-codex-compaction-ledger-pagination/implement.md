# Implementation

1. Add fail-open Codex compaction observation before request plugins, persist a
   fixed marker, propagate it through active snapshots, and render badges and
   tooltips in persisted and live request cards.
2. Add backend request-log page types, cursor validation, server-side filters,
   keyset query, IPC command and generated TypeScript bindings.
3. Replace the Logs page's fixed 200-row feed with page queries, cursor-stack
   navigation, page-size persistence, debounced filters, and stable live-card
   behavior without changing the Home feed.
4. Add schema v43, ledger/state/view, transactional dual-write, reconciliation,
   resumable background backfill, retention coverage checks, provider deletion
   and manual-clear semantics.
5. Move usage, cost, Session, folder, availability and provider-limit reads to
   the compatibility usage source; keep detail/replay reads on `request_logs`.
6. Update backend and cross-layer specs, regenerate bindings, run focused and
   full verification, then perform a findings-first review.

## Implementation Notes

- Compaction classification observes only the bounded, decoded original Codex
  request and is recorded before request plugins run. Classification has no
  error exit and never changes forwarding, retry, provider or health behavior.
- Request-log pages use a versioned opaque keyset cursor ordered by
  `(created_at_ms DESC, id DESC)`. Active requests are excluded before `LIMIT`
  and rendered separately so they do not consume persisted page capacity.
- Schema v43 dual-writes one compact `usage_ledger` row per trace in the same
  transaction as the canonical request-log UPSERT. A resumable background
  backfill keeps retention paused until coverage is audited and cut over.
- Usage consumers read the `usage_events` compatibility source. Cost totals use
  precise `i128` accumulation at the provider gate and rolling recovery
  buckets, availability is server-bucketed from the ledger, and provider
  leaderboards aggregate in SQL. Display aggregates may use floating point,
  but gate decisions never do.
- The incomplete compatibility view and the Rust projector both reject attempt
  provider IDs that cannot be represented as SQLite/Rust signed integers, so a
  malformed future attempt cannot change provider attribution at cutover.
- SQLite busy/locked during background reconciliation now retries with a
  bounded exponential delay while retention remains safely paused. Other
  backfill failures remain diagnostic-only and never affect request forwarding.
- Provider deletion checks managed-profile references before starting any
  historical usage projection and checks again in its final transaction. The
  compatibility view and the projector normalize provider-name ASCII whitespace
  with the same rules.
- Frontend verification passed with the Node 26 Web Storage workaround and
  excludes the intentionally retained `.local/codex-cli-reference` checkout.
  Rust verification remains pending CI because this host has no Cargo, Rustc or
  Rustfmt.
- The task remains `in_progress` until Rust formatting, generated bindings,
  tests and Clippy run in an environment with Cargo.
