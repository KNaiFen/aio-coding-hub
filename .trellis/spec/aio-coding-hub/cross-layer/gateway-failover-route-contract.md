# Gateway Failover Route Contract

## Scenario: Change Provider Selection, Gates, Or Route Presentation

### 1. Scope / Trigger

Use this contract when changing session-bound provider selection, circuit or
rate-limit gates, `failover_max_providers_to_try`, persisted request attempts,
route projection, or the Home request-log route label. These layers share one
observable failover chain, but their counters have different meanings.

### 2. Signatures

The persisted provider limits are:

```rust
pub struct Settings {
    pub failover_max_attempts_per_provider: u32, // default 5, valid 1..=20
    pub failover_max_providers_to_try: u32,      // default 5, valid 1..=20
}
```

The frontend presentation receives the projected route and persisted attempt
count separately:

```ts
buildRequestRouteMeta({
  route: RequestLogRouteHop[] | null | undefined,
  status: number | null,
  hasFailover: boolean,
  attemptCount: number,
});
```

`RequestLogRouteHop` exposes `provider_id`, `provider_name`, `ok`, `attempts`,
`skipped`, and optional status/error/decision/reason fields.

### 3. Contracts

- Session binding owns reuse preference and ordering only. Filter providers
  with a known exhausted OAuth or spend-limit snapshot before resolving the
  binding. If the bound provider is removed by that eligibility filter, clear
  the binding; if it remains eligible but its circuit currently denies reuse,
  keep it in the list and let the later common gate decide.
- The provider-global `enabled` switch is the outer routing authority. Default
  routes, custom sort-mode membership, session reuse, forced candidate lists,
  and bridge source resolution must all exclude globally disabled providers;
  a route-member switch never overrides it. Route editors may preserve the
  member value while showing the global-disabled state, but must disable the
  member switch until the provider is globally enabled again.
- Candidate SQL filters the global switch when the request is selected. A
  running request also checks the shared provider-enable gate immediately
  before every upstream send, including same-provider retries and bridge source
  sends. A send already admitted before a successful disable command may
  finish; later retries or provider switches record a gate-only disabled skip
  and make no upstream call. Provider save, enable/disable, and delete paths
  update this gate before returning success.
- Route members may set `session_reuse_priority` in `0..=1000`; it is not the
  provider-global priority or ordinary route `sort_order`. A bound provider may
  be promoted only when no current eligible candidate has a higher reuse
  priority. Bound route-order snapshots may reorder candidates only within the
  same priority tier, so a lower-tier historical binding cannot bypass a
  higher-tier candidate. Equal tiers retain legacy session reuse behavior.
- Updating a route member's reuse priority clears only that CLI's session
  bindings after the durable write; circuit and recent-error runtime state must
  remain intact.
- A route switch becomes authoritative when its backend command succeeds. The
  command must durably write the new active route, then advance that CLI's
  in-memory route generation and clear its bindings before returning success.
  Advancing the generation is required even when zero bindings are removed.
- A request captures the current global and CLI route generation before reading
  session routing state. Binding reads, writes, and provider clears must verify
  that token under the same lock that owns generations and bindings. Requests
  that selected a provider before a switch may finish on that provider, but a
  stale request must not recreate, refresh, replace, or clear a binding after
  the switch.
- Active-route persistence must not expose a committed route change through an
  error result that skips runtime invalidation. Keep result projection inside
  the write transaction, reserve the WAL writer before validation reads, and
  make commit the final fallible database step.
- OAuth and spend limits are route-eligibility constraints. After forced-route
  narrowing and before session binding, evaluate every candidate with one
  shared database connection and remove known-limited providers. They create
  no attempt or route hop and consume no Ready-provider budget. A database or
  blocking-pool failure is fail-open so infrastructure trouble does not
  silently disable every route.
- The send-time provider gate repeats the same limit decision to cover a quota
  snapshot or spend total changing after selection. A limit denial there still
  creates no skipped attempt. Circuit, cooldown, and runtime global-switch
  denials remain common-gate decisions and create one `outcome="skipped"`
  attempt with stable error/reason data and zero upstream calls.
- `providers_tried` increments only after the common gates and preparation
  produce `Ready`. Therefore `failover_max_providers_to_try` caps Ready
  providers, not inspected candidates or skipped rows.
- Reaching the Ready-provider cap does not bypass the authoritative gate for
  later candidates. Later gate denials still emit skipped attempts/routes; the
  loop stops only when a later candidate itself becomes `Ready` beyond the cap.
- `attempt_count` is the number of persisted attempt rows. It may include
  retries and skipped rows, so it is not a provider count or switch count.
- The projected `route` is the source of provider-hop display. Derive
  `providerCount = route.length` for the complete audited chain. Derive
  `transitionCount` only from adjacent provider identity changes in
  `route.filter(!skipped)`: a gate-only circuit/cooldown hop made no upstream
  request and is not an effective supplier switch. Known limit exclusions do
  not appear in the projected route at all. Keep
  `attempt_count` as the raw audit-row count and never relabel it as the number
  of upstream requests.
- The Home compact route label derives operational counts from the projected
  hops: `skippedCount` counts skipped hops, `requestCount` sums `attempts` only
  for non-skipped hops, and `retryCount` sums `max(attempts - 1, 0)` only for
  non-skipped hops. Use `切N`, `跳N`, `重N`, and `请N` for those meanings while
  retaining a full natural-language summary for the rich tooltip and
  assistive technologies.
- Route presentation is observational and fail-open. Malformed or future hop
  fields may hide unsupported detail but must not break the Home feed. Rich
  route content uses the current theme's semantic popover surface, suppresses
  only known duplicate internal reasons, preserves unknown reasons as wrapped
  text, and scrolls within the collision-bounded viewport height.
- When limit eligibility removes every candidate, return
  `GW_NO_ENABLED_PROVIDER` / HTTP 503 with an empty attempts array and no
  `Retry-After`; this is semantically identical to a route with no enabled
  providers. When the remaining candidates are denied by circuit, cooldown,
  or the runtime global-switch gate, return
  `GW_ALL_PROVIDERS_UNAVAILABLE` / HTTP 503 and preserve those gate-denied
  providers in attempts and route. Do not manufacture an upstream call.
- Upstream 401 and 403 bodies are authentication material and must never enter
  console diagnostics, persisted attempt reasons, `attempts_json`, or
  `error_details_json`. The bounded body may remain in memory only as needed by
  existing failover/auth classification or an explicit configured HTTP retry
  rule. Serialization defensively strips a supplied 401/403 preview even when
  an earlier layer accidentally included it.
- HTTP retry content matching joins the existing error-body inspection path:
  consume the network body once, scan at most the decoded first 64 KiB, and use
  a separately bounded encoded input for gzip. A decode/read failure is an
  unmatched rule and compressed bytes must never be treated as text.
- Only an actual configured HTTP retry adds `retry_rule=<1-based index>` and an
  optional bounded single-line description to the attempt reason. Matcher
  contents, hit fragments, and response bodies are never added by this feature.
  Description `%`, `,`, and `=` delimiters are percent-escaped before joining
  the attempt-reason field format so they cannot impersonate another field.
- A native Codex Responses in-band terminal error may add one optional
  `stream_internal_error` object to its attempt. Persist only the recognized
  event/type/code/message, classification, matched keyword, disposition, and
  truncation flag. The message is one line and at most 2048 Unicode characters;
  other text fields are at most 512. Redact bearer credentials, common API-key
  forms, and token assignments before serialization. Never persist the raw SSE
  frame or ordinary model output.
- Before downstream commit, a positive stream-internal match belongs to the
  failover loop and its failed attempt remains visible even if a later retry
  succeeds. After commit, retry is unsafe: forward the original event and let
  stream completion update the optimistic success attempt with the same bounded
  evidence. Old or malformed evidence remains observational and fail-open.

### 4. Validation & Error Matrix

| Input / condition | Required result |
| --- | --- |
| `failover_max_providers_to_try == 0` | Reject with `SEC_INVALID_INPUT` |
| `failover_max_providers_to_try > 20` | Reject with `SEC_INVALID_INPUT` |
| attempts per provider x providers to try > 100 | Reject with `SEC_INVALID_INPUT` |
| Eligible session-bound provider is circuit-open | Keep candidate; common gate records one skipped row |
| Higher-priority provider has a known exhausted OAuth or spend limit | Remove it before session binding; the first usable provider is `provider_index=1` |
| Session is bound to a lower-priority provider while a higher-priority provider is limited | Reuse the lower provider directly; record no limit attempt |
| Candidate is gate-skipped | Zero upstream calls and no Ready-provider budget consumed |
| Provider or bridge source is disabled after selection | Current admitted send may finish; every later send is skipped |
| All candidates are limit-excluded | HTTP 503 `GW_NO_ENABLED_PROVIDER`, empty attempts, no `Retry-After` |
| All remaining candidates are circuit/cooldown/global-switch skipped | HTTP 503 `GW_ALL_PROVIDERS_UNAVAILABLE` with every denial in attempts and route |
| Ready-provider cap is reached | Stop before the next Ready provider |
| Two Ready providers consume cap 2, then a circuit-open candidate follows | Record the third skipped attempt/route; make no third upstream call |
| Route has 3 hops and 4 attempt rows | 3 providers, 2 transitions, 4 attempts |
| Two skipped hops followed by one sent request | Compact label `跳2·请1`; zero effective switches |
| A sent request, one skipped gate hop, then C sent | One effective A→C switch |
| One provider is sent 3 times | Compact label `重2·请3` |
| Upstream 401/403 body contains a credential-like value | Keep status and safe reason, but persist/log none of the body |
| Gzip body exceeds the decoded scan prefix | Match only decoded bytes within the first 64 KiB; never scan compressed fallback bytes |
| Native Codex `response.failed` contains a credential and a capacity phrase | Retry only before commit; persist the redacted structured evidence and no raw SSE |

### 5. Good / Base / Bad Cases

- Good: two known-limited candidates are removed, then a third Ready candidate
  is sent as `provider_index=1`; the limit exclusions create no retries, route
  hops, or Ready-slot consumption.
- Good: two circuit-open candidates are skipped, then a third Ready candidate
  succeeds with `failover_max_providers_to_try = 2`; the skips do not consume
  either Ready slot and the effective route has zero switches.
- Base: one Ready provider and one attempt render as a direct request with zero
  provider transitions.
- Good: three gate-skipped candidates return 503, produce three route hops and
  three attempt rows, and call no upstream.
- Bad: removing a circuit-open session-bound provider before `run_gates`; the
  request loses the circuit denial from its audit trail.
- Bad: retaining a known-limited provider until `run_gates`; it creates a fake
  first attempt, turns the first real request into retry/provider index 2, and
  suppresses legitimate session-reuse presentation.
- Bad: rendering four attempt rows as "switched 4 times" when they include
  retries or gate-only candidates that never received an upstream request.

### 6. Tests Required

- Unit-test selection so a circuit-denied bound provider stays in the candidate
  list while reuse selection returns no bound provider.
- Route-test OAuth and spend-limit prefiltering: the limited provider gets zero
  upstream calls and no attempt; a lower eligible provider is the only attempt
  with `provider_index=1`.
- Route-test an existing lower-provider session binding with a higher-priority
  limited provider; require `session_reuse=true` and
  `selection_method="session_reuse"` on the first real attempt.
- Route-test all-limit behavior: HTTP 503 `GW_NO_ENABLED_PROVIDER`, empty
  response and persisted attempts, no `Retry-After`, and stale binding clear.
- Unit-test CLI and global route-generation invalidation, including clears that
  remove zero bindings, and prove stale reads/writes cannot affect a binding
  created under the current generation.
- Route-test a gated non-streaming A-to-Default switch and a gated streaming
  A-to-B switch. Release the old response only after the durable write and
  generation clear, then prove the next same-session request selects the new
  route and the late old success cannot overwrite it.
- Force active-route result projection to fail during A-to-B and A-to-Default
  updates; both operations must roll back to A instead of committing a change
  that would skip runtime invalidation.
- Unit-test that a lower-priority binding and a lower-priority bound-order
  fallback leave higher-priority candidates ahead, while equal-priority
  bindings retain their existing rotation behavior.
- Route-test all-gate-skip behavior: 503, one skipped row and route hop per
  candidate, preserved session binding, and zero upstream calls.
- Query-test default/custom routes and bridge source resolution with a globally
  disabled provider. Route-test disabling between two prepared attempts so the
  admitted attempt can finish while the next retry makes zero upstream calls;
  include the bridge-source variant.
- Frontend-test that a globally disabled custom-route member keeps its stored
  member value but renders off, carries the disabled label, and cannot invoke
  the route-member mutation.
- Route-test that skipped candidates do not consume the Ready-provider cap,
  plus a boundary where the cap stops before the next Ready provider.
- Route-test the reverse boundary `Ready, Ready, circuit-open/cooldown` at cap
  2; the third candidate must remain visible as skipped.
- Use `SYNTHETIC_SECRET` in 401 and 403 bodies; assert console output, attempt
  serialization, and error details omit it without changing failover/auth
  classification or the recorded status.
- Use `SYNTHETIC_SECRET` in pre-commit and post-commit Codex terminal events;
  assert attempts, error details, frontend copy text, and diagnostics omit it,
  while successful retry chains retain the failed attempt's bounded evidence.
- Keep model-discovery strict-attempt and health-neutral circuit tests passing;
  shared gate changes must not broaden those requests.
- Frontend-test provider, effective transition, skipped-hop, sent-request, and
  extra-retry counts together, including skipped gates between two actual
  suppliers, malformed/future values, and same-provider retries.
- Frontend-test the rich route panel in light/dark themes, long wrapped content,
  collision-bounded scrolling, known-reason deduplication, and preservation of
  unknown future reasons without changing the default short-tooltip surface.
- GitHub Actions must run the full Rust library suite after shared failover
  selection or gate changes, plus generated bindings, typecheck, lint, and Rust
  format checks; locally use only the cloud-only allowlist.

### 7. Wrong vs Correct

#### Wrong

```rust
for provider in providers {
    attempts.push(limit_gate(provider));
}
```

This turns a provider that was already known to be ineligible into a fake
request attempt and shifts retry/session-reuse indices.

#### Correct

```rust
let providers = filter_known_limits(providers);
let session_bound_provider = resolve_session_binding(&providers);
```

Filter known OAuth/spend exhaustion before session preference. Keep circuit,
cooldown, and runtime enable checks in the common gate because those denials
remain observable audit skips.
