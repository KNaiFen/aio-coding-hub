# AIO Local Observer and TUI Contract

## Scope

The desktop application exposes an observational, authenticated observer for a
standalone `aio-tui` process running on the same machine. This surface is for
SSH and narrow-terminal observability; it is not a second gateway and is not a
remote administration API.

## Endpoint and descriptor

- The observer binds only to `127.0.0.1` on an ephemeral port.
- The descriptor is `~/.aio-coding-hub/observer-v1.json` (the same home and
  dot-directory overrides used by AIO).
- The descriptor is written atomically and contains only protocol/app version,
  PID, port, start time, and a random bearer token. Unix permissions are
  `0600`; shutdown removes it only when PID and token still belong to the
  current process.
- Supported resources are authenticated `GET` requests to
  `/api/observer/v1/health` and
  `/api/observer/v1/snapshot?cli=<scope>&history_limit=0..50`. The snapshot
  accepts optional `include_providers=true`; its response field is additive and
  optional so old clients and old observers can fail open independently.
- The sole active operation is authenticated `POST`
  `/api/observer/v1/providers/<provider_id>/test-availability`. It is bounded by
  a separate concurrency limit and timeout, returns a bounded fixed-shape result
  with credential-stripped URL and preview fields, and records the same bounded
  availability observation as the desktop and scheduled full-probe entry
  points. It never changes routing, limits, or circuit state.
- Responses are `no-store` and `nosniff`. Invalid input, authentication, busy,
  and internal failures use fixed structured messages without body, URL,
  credentials, or decoder details.

## Isolation and fail-open behavior

- The observer never listens on the gateway port, mutates request state, or
  sends IPC commands. Only the explicit manual availability-test POST may call
  one upstream provider; snapshot and health requests remain read-only.
- Snapshot database reads use a separate single-connection SQLite read-only,
  `query_only` pool with short timeouts. Concurrent cache misses wait fairly for
  that query lane within a bounded deadline; contention timeout returns
  `OBS_BUSY` and is never cached as an unavailable projection. A genuinely
  missing or slow database read marks only the affected sections unavailable.
- Circuit status is read through a non-mutating peek. It must not reserve a
  half-open probe, persist state, emit events, or alter provider health.
- Observer startup, refresh, serialization, authentication, and TUI parsing
  failures must not change forwarding, retries, provider selection, circuit
  accounting, request-log writes, or application shutdown.

## Snapshot semantics

- `active_inference_count` is global and counts every active model-inference
  request individually, including parallel requests in one Session and CLI
  sub-agents. Auxiliary model-list, search, token-count, probe, and discovery
  requests are excluded.
- `last_request` and `dominant_provider` use terminal model-inference records;
  failures and client interruptions remain eligible. `dominant_provider` samples
  the newest ten records and resolves ties by recency.
- `today` is global and uses the existing usage-ledger aggregate semantics:
  known cost is returned without a partial marker, and `null` means no known
  cost. Total tokens include input, output, and cache buckets according to the
  existing effective-token expression.
- The preferred provider is the first enabled provider in the active gateway
  order that has not reached a configured spend limit, has no active exhausted
  OAuth quota snapshot, whose peeked circuit is not `OPEN`, and whose cooldown
  is not active. Spend state comes from the existing provider-limit read model;
  OAuth state comes from the same snapshot gate used by forwarding. This is a
  local eligibility read, not a remote account/model-quota guess. If any
  eligibility read fails, only `preferred_provider` becomes unavailable.
  `all` selects the CLI from the newest terminal inference record.
- Terminal route counters retain every skipped audit hop in `route`, but
  `provider_switch_count` compares only adjacent non-skipped suppliers and
  `retry_count` sums only extra attempts on non-skipped suppliers. TUI route
  summaries display both counters whenever both are non-zero; neither counter
  is inferred from the other.
- The logs view contains active requests first and up to fifty terminal proxy
  records. Active requests do not consume the fifty-record allowance. Each
  request has a stable opaque key so terminal completion does not move the
  selected detail unexpectedly.
- Request projections are bounded and never contain body text, upstream URLs,
  credentials, raw error JSON, or other large configuration blobs.
- A request projection may include a bounded optional configured-model-route
  observation only when an applied provider-scoped marker is valid. Unknown,
  malformed, future, or mismatched-provider markers are omitted without making
  the request or snapshot unavailable.
- `cross_provider_model_route` is additive explanatory metadata in the bounded
  `special_settings` channel. Desktop projection renders compact
  `A / source -> B / target` audit text only for an HTTP-success record whose
  marker is `matched`, `singleHop=true`, and target provider equals the final
  provider. Failed/skipped, oversized, malformed, future, self-target, or
  provider-mismatched markers are omitted from summaries while bounded raw
  metadata and the complete attempts remain available in detail.
- The configured-route projection accepts `policy_source=provider_cross` only
  when the applied marker matches the final provider. It remains authoritative
  for effective model/effort. The cross marker never affects pricing, usage,
  preferred provider, route hops, retry count, or `provider_switch_count`.
- Codex request projections may include a bounded optional requested reasoning
  effort parsed from the request-scoped `codex_reasoning_effort` marker. Only
  the fixed supported effort vocabulary is exposed; missing, malformed, or
  future values are omitted and old snapshots remain valid.
- The optional provider projection is capped at 512 rows. It contains only
  provider/CLI names, route rank and enable flags, authentication kind, fixed
  eligibility labels, non-mutating circuit snapshots, spend-window totals, and
  bounded locally cached OAuth quota text/reset times. It excludes endpoints,
  credentials, tokens, email, notes, tags, extensions, and arbitrary/error JSON.
- Eligible active-route providers follow actual route order. Disabled or
  out-of-route rows follow in stable pool order; `all` groups Codex, Claude,
  Grok, and Gemini. A custom-route membership never makes a globally disabled
  provider eligible or preferred. Provider projection failures make only that
  optional section unavailable and never enter routing or health accounting.
- Each provider may include a 12-bucket, 3/6/12-hour availability timeline
  derived from terminal real-upstream attempts and completed full Provider
  probes. Desktop manual, Observer/TUI manual, and scheduled probes write equal
  success/failure facts after a bounded probe returns `Ok`; a bounded
  `ok=false` result is a failure fact. Base URL Ping, internal probe errors,
  local preflight failures without a probe result, skipped attempts, and
  aborted requests never contribute facts.
- Provider configuration and credential writers acquire a per-Provider probe
  mutation gate, advance the generation, and hold the gate through their
  durable write. New manual, Observer/TUI, and scheduled probes wait for that
  boundary before reading configuration. An older flight either records before
  the mutation begins or becomes stale; no probe may start in the invalidation
  to commit window and later publish an observation for the wrong generation.

## TUI behavior

- `aio-tui` defaults to a dashboard whose request and provider views switch with
  Left/Right. Request and provider selections are independent. List navigation
  selects an item for five seconds; inactivity clears the selection and scroll
  offset so request snapshots resume following the newest rows. Detail view
  suspends expiry and returning to the list starts a new five-second window.
  Snapshot refreshes never extend the deadline. Both views share the same two-line
  summary: concurrency plus preferred provider, then CLI plus today's cost and
  tokens. View names and connection-state text stay out of this header. A dim
  rule separates the shared summary from either list.
- Provider cards use four semantic base lines plus a dim separator. A fifth
  OAuth line is present only when locally cached label, five-hour, or weekly
  quota text is displayable. Labels and values are separate styled spans:
  available/on/closed is green, limits/cooldown/half-open is yellow, open circuit
  is red, and disabled/missing/unknown is dim gray. Enter opens bounded read-only
  detail, where unavailable OAuth placeholders are likewise omitted.
- `aio-tui status` is the continuously refreshed status line and reserves a
  two-column left gutter on every wrapped row. Its fallback palette uses ordinary
  cyan, green, and magenta with dim separators; errors and warnings retain red
  and yellow. It does not use bright ANSI colors or bold status segments.
  `aio-tui status --once` remains unindented pipe-friendly output.
- `--cli` accepts `claude`, `codex`, `grok`, `gemini`, or `all`; the default is
  `codex`. Concurrency and today usage remain global in every scope.
- The client never starts AIO. When the observer is unavailable it keeps the
  last successful snapshot, shows a stale/offline label, and retries every two
  seconds. `OBS_BUSY` is shown as transient observer contention and likewise
  retains the last successful snapshot. Interactive mode restores raw mode,
  alternate screen, and cursor on normal exit and panic.
- The client disables HTTP proxies and redirects for the loopback request and
  bounds descriptor and snapshot sizes. Protocol or JSON failures hide the
  affected view rather than crashing the process.
- Status/request polling omits the provider projection. Entering the provider
  view requests it; if an older observer rejects the additive query, the client
  retries the legacy snapshot and marks only the provider view unsupported.
- Request cards use semantic status, model, target-model, provider, route, and
  metrics lines so dynamic model routing does not change color ownership. When
  an applied configured route changes a Codex model, the card renders
  `Codex / source[-requested-effort] →` followed by a
  display-width-right-aligned target line containing
  `effective[-effective-effort][ 压缩·模式]`; the source arrow remains visible at
  every nonzero width. Codex requests without a model change retain one line and
  show the final effective effort. Statusline and request detail use the same
  hyphenated source/effective evidence. Missing optional effort evidence omits
  the suffix. Non-Codex route formatting remains unchanged. Old observers or
  invalid optional route fields continue to render the ordinary model safely.
- Cross success continues to show final B provider/model through existing
  fields. Desktop Home adds the bounded cross audit text above; this task does
  not change the TUI formatter, TTFB, or switch/retry wording. B failure
  followed by A/C success therefore keeps A/C as the card/TUI terminal provider
  while detail retains the full chain.
- Provider availability detail converts bucket timestamps to the host system's
  local timezone at render time and displays `HH:MM-HH:MM` without a hard-coded
  timezone suffix.

## Release boundary

Standalone TUI archives are published for Windows x64, macOS Intel, macOS
Apple Silicon, and Linux x64. They share the desktop version and checksums but
are not included in the desktop updater `latest.json` or desktop installers.
