# Design

## Codex context compaction observation

After the existing bounded Codex request decoding succeeds, capture the
original decoded JSON and request metadata before `RequestAfterBodyRead`.
Classification returns only `Option<CompactionMarker>` and cannot produce a
gateway error. Canonical body metadata wins over the direct compatibility
header. Explicit usable non-compaction metadata prevents protocol fallbacks.

Known implementation mapping:

- `responses` -> local
- `responses_compact` -> remote v1
- `responses_compaction_v2` -> remote v2

When explicit metadata is absent or unusable, a terminal
`responses/compact` path identifies remote v1 and a top-level
`input` item with `type=compaction_trigger` identifies remote v2. Unknown enum
values are normalized to fixed `unknown` values; no untrusted strings or
request content are persisted.

The marker is stored in bounded `special_settings_json` and copied to the
active request registry. It never sets `is_compact_request`,
`provider_health_neutral`, or any routing field.

## Usage ledger

Schema v43 adds `usage_ledger`, one row per `trace_id`, plus a singleton
`usage_ledger_backfill_state` and a compatibility `usage_events` view. The
ledger contains only analytic dimensions, token buckets, timings, provider
name snapshot, and normalized cost inputs.

Fresh installs start complete. Upgrades record the current maximum
`request_logs.id`, start incomplete, and run a bounded background backfill
after normal startup. New request-log writes update both tables in one
transaction. While incomplete, `usage_events` reads `request_logs`; once every
row through the recorded high-water is covered, one transaction marks the
backfill complete and the view reads `usage_ledger`.

Retention is paused while incomplete. After completion, each deletion batch
verifies ledger coverage before deleting details. Backfill failure is
diagnostic only: the gateway keeps forwarding and statistics keep reading
`request_logs`.

## Cursor pagination

`request_logs_page_all` accepts bounded filters, an opaque versioned cursor,
and a limit. The cursor encodes `(created_at_ms, id)` and the query uses
descending keyset pagination with `limit + 1`. No total count is calculated.
The Logs page owns a cursor stack for previous/next navigation; Home keeps the
existing list/after-id feed.

## Compatibility and failure behavior

- No settings, database-file, provider, authentication, or retention-day
  contract changes.
- Invalid compaction metadata never returns an error.
- Invalid pagination input uses the existing `SEC_INVALID_INPUT`.
- Existing log detail, replay, and attempts remain limited by request-log
  retention; aggregate usage does not.
