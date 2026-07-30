# Request Log, Usage Ledger, And Pagination Contract

## Separate Lifetimes

`request_logs` owns request details, attempts, replay material, activity
diagnostics, paths, and large JSON. The existing
`request_log_retention_days` setting controls only this table.

`usage_ledger` owns one compact analytic row per logical `trace_id`. It remains
in the same SQLite database but is not deleted by request-log retention or the
manual "clear request logs" action. It is deleted only by an explicit matching
usage-clear operation, provider deletion with `clear_usage_stats=true`, or a
full application-data reset.

The Logs page's page size or old 200-row query window is never a retention
mechanism.

## Ledger Consistency

Request-log and ledger writes share the existing asynchronous writer and one
SQLite transaction. The ledger is projected from the canonical persisted
request-log row so a late pending placeholder cannot downgrade a terminal row.

The ledger stores normalized dimensions, timings, token buckets, provider name
snapshot, usage presence, and cost inputs. It does not store request/response
bodies, paths, attempts JSON, usage JSON, special-settings JSON, or error
details.

Schema upgrade creates a durable high-water backfill state but does not copy
unbounded history inside the migration transaction. The gateway starts
normally; a low-priority bounded job resumes from its committed cursor. New
requests dual-write immediately.

Until backfill is complete, the compatibility statistics source reads
`request_logs` and retention is paused. Backfill errors are diagnostic only and
must not fail gateway startup. Completion requires an anti-join coverage check
and the state transition in one transaction.

After completion, every aggregate reads the ledger source: usage summaries,
leaderboards, trends, cache rate, Session and folder views, cost backfill,
provider-limit display, and actual provider-limit gating. Detail, attempt, and
replay APIs continue reading `request_logs`.

## Retention And Clear Operations

Automatic retention verifies durable ledger coverage before deleting each
detail batch. Missing, incomplete, or unreadable ledger state deletes nothing.

Manual request-log clearing preserves the ledger and must fail without deleting
anything if an incomplete backfill still needs the detail table as its source.
Provider deletion with usage clearing removes both tables in the same
transaction; without usage clearing, the ledger name snapshot preserves
historical display.

## Cursor Pagination

The Logs page uses a versioned opaque Base64URL cursor over
`(created_at_ms, id)`, ordered descending. Server-side filters are applied
before the seek predicate, and `limit + 1` determines whether another page
exists. No total count is required.

Page size is 50, 100, or 200. The frontend keeps a cursor stack for previous
and next navigation, pauses persisted-history refresh while reading older
pages, and continues updating active requests. The Home page keeps its existing
bounded list/after-id realtime feed.

Array realtime-feed caches and page-object caches have distinct query-key
branches and reset shapes.

## Tests

Verify migration, resumable backfill, concurrent dual-write, pending
reconciliation, failure isolation, retention coverage, manual clear, provider
deletion, cost correction, aggregate equality before/after cutover, cursor
validation, equal-timestamp page boundaries, filter semantics, live overlays,
and generated binding compatibility.
