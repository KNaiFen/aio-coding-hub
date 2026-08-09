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

When an applied configured-model-route marker matches the final provider, its
target CLI/model is the authoritative cost basis for both the request log and
ledger. A missing target-model price remains unknown; costing must never fall
back to the original client model. Malformed, future, pending, or
provider-mismatched markers are ignored.

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

## Bounded Provider Trends

Provider cache-rate and performance trends query `usage_events`, never
`request_logs` or request/attempt JSON. This keeps trend history aligned with
the ledger lifetime and preserves the same result after covered request-detail
rows or the live Provider record are deleted.

Without a Provider filter, both trends rank at most ten `(cli_key,
provider_id)` series by successful-request count over the complete filtered
range. A Provider filter returns only that Provider. Missing and zero limits
must never mean unlimited; caller-provided limits are bounded to `1..=10`.

The shared planner selects the finest local-calendar bucket among hour, day,
week, month, and year that fits at most 120 buckets. Queries preserve the full
filtered range and enforce a second response check of at most 1,200 rows; a
single-Provider response is at most 120 rows. Week buckets start on Monday.

Performance trend formulas are identical to the usage summary formulas:

- average duration is successful duration divided by successful requests;
- average TTFB includes only successful rows where `ttfb_ms < duration_ms`;
- each valid output-rate sample is
  `output_tokens * 1000.0 / final_upstream_attempt_duration_ms`, using the
  complete trusted final successful upstream attempt including TTFB;
- summary, leaderboard, folder, day-detail, trend, and daily-rollup output rate
  is the arithmetic mean of those per-request samples (`rate_sum / sample_count`),
  never `SUM(output_tokens) / SUM(duration)` and never a duration with TTFB
  subtracted.

The final-attempt timestamp freezes at the first trustworthy protocol
completion event, or at a clean EOF for protocols without such an event. After
a downstream disconnect, the Codex Responses stream keeps draining until EOF,
an upstream error, or its bounded deadline even after completion is seen. A
later terminal failure/incomplete event or transport error invalidates the
frozen attempt before persistence; a clean completion is not allowed to hide a
queued terminal error. The disconnect itself invalidates downstream output
timing, but does not discard a final-attempt timestamp established by a
trustworthy completion or clean drain.

Errors, statistics-excluded rows, missing or non-positive output usage,
untrusted timing versions, and missing or non-positive final-attempt duration
never enter the output-rate denominator. Invalid TTFB remains excluded only
from the TTFB metric; a valid final-attempt output-rate sample does not require
TTFB. Trend ranges use local natural calendar boundaries and intentionally
ignore the configurable statistics day-start offset.

Each performance row exposes the valid sample count for duration, TTFB, and
output rate separately. Consumers must display the sample count for the active
metric rather than treating the total successful-request count as every
metric's denominator.

## Provider Daily Trend Projection

Provider daily rollups are a rebuildable performance projection over
`usage_ledger`; they are not a replacement fact source and never authorize
automatic ledger deletion. Model, Session, folder, cost repair, provider-limit,
hourly, and partial-day consumers continue reading event-level ledger rows.

The projection stores one additive row per local natural day, CLI, and Provider,
plus a separate day-coverage record. Only a `complete` day whose stored local
midnight boundaries still match the current system time-zone rules is trusted.
The current day, partial query boundaries, dirty or missing days, invalid legacy
timestamps, and every hour-granularity query use raw ledger data. Day and coarser
queries combine trusted rollups with the exact raw complement inside one SQLite
read snapshot, then perform Top Provider selection and final division after the
two sources are re-aggregated.

Ledger inserts, deletes, and real changes to trend-relevant fields atomically
mark both the old and new local days dirty. Rebuilding a closed day uses one
`IMMEDIATE` transaction, independently compares the eligible raw row count with
the projected request sum, and marks the day complete only after they match.
Request-log retention runs first; rollup maintenance rebuilds at most 32 days per
batch and releases the maintenance mutex between delayed continuation batches.

A time-zone boundary change invalidates the derived projection so it can be
rebuilt from the retained ledger. If ensure detects a missing rollup table or
dirty-day trigger, it restores the schema, clears all potentially stale derived
rows, and resets the projection cursor before queries may trust coverage again.

Provider deletion with `clear_usage_stats=true` removes matching request logs,
ledger rows, and Provider rollup rows in the same transaction. Default Provider
deletion retains both ledger history and rollup name snapshots.

## Home Realtime Concurrency

The Home current-concurrency value counts active model inference requests, not
distinct Sessions. Every active snapshot entry that matches the existing
inference endpoint classifier counts as one, including parallel requests in
the same Session and subagent requests.

Auxiliary requests such as model lists, search, token counting, probes, and
non-POST traffic remain excluded. An available empty snapshot displays `0`;
an unavailable snapshot displays `--`.

This is a frontend-only observation over the existing per-trace active-request
registry. It must not change gateway forwarding, retries, provider health,
request logging, historical pagination, or IPC contracts.

## Tests

Verify migration, resumable backfill, concurrent dual-write, pending
reconciliation, failure isolation, retention coverage, manual clear, provider
deletion, cost correction, aggregate equality before/after cutover, bounded
provider trend formulas/ranking/buckets/rows and detail-retention invariance,
per-request output-rate arithmetic averaging versus token/duration weighting,
raw/rollup/hybrid equality after projection rebuild,
cursor validation, equal-timestamp page boundaries, filter semantics, live
overlays, and generated binding compatibility.
