# Provider Account-Usage Query Contract

## Scenario: Shared Demand-Driven Account-Usage Runtime

### 1. Scope / Trigger

Use this contract when changing account-usage cache ownership, desktop/TUI
consumer lifecycle, timed or manual refresh, provider edit/delete invalidation,
or Observer account-usage projection.

### 2. Signatures

The desktop query remains provider-keyed, but its interval is only a consumer
heartbeat. The backend owns remote fetch timing and completion ordering:

```ts
providerAccountUsageQueryOptions(providerId: number);
refreshProviderAccountUsage(queryClient, providerId);
useProviderAccountUsageQuery(provider, enabled?);
```

```rust
provider_account_usage_fetch(provider_id, force);
ProviderAccountUsageRuntimeState::touch_desktop(...);
ProviderAccountUsageRuntimeState::touch_tui(...);
ProviderAccountUsageRuntimeState::fetch(...);
```

### 3. Contracts

- The Tauri-managed runtime is the only process-wide owner of fetched results,
  remote refresh timing, and per-provider in-flight state. React Query mirrors
  the backend result for rendering; Observer reads the same backend result.
- Desktop queries send a five-second heartbeat while their account row is
  mounted. Observer snapshots with `include_providers=true` renew TUI leases.
  Leases last 15 seconds; when all relevant desktop and TUI leases expire, no
  new timed remote request may start.
- A configured provider performs an initial fetch when no reusable result
  exists. While a consumer remains active, timed refresh follows the persisted
  60-300 second interval. Disabling timed refresh suppresses interval refresh,
  but does not suppress the initial fetch, manual fetch, or 60-minute hard
  expiry refresh.
- The provider global `enabled` switch controls routing only. A configured,
  globally disabled provider is still eligible for account-usage refresh while
  it has an active balance consumer.
- At most one remote account-usage request per provider may be in flight.
  Manual and scheduled callers coalesce, up to four different providers may
  fetch concurrently, and a result from an invalidated configuration generation
  cannot commit.
- A successful result is displayable only when `last_fetched_at <= now` and its
  age is strictly less than 60 minutes. Loading, failed, expired, malformed, or
  future-dated states expose no previous amount.
- Provider save/delete invalidates only that provider's backend and frontend
  account cache. Enable/disable alone does not invalidate or stop account usage.
- Observer output is bounded and secret-free: configured providers expose only
  state, the preferred plan-remaining-or-balance amount, unit, and fetch time.
  It never exposes credentials, upstream bodies, or adapter error text.
- Account usage stays display-only. It never changes routing, availability,
  provider order, circuit/cooldown state, spend limits, or OAuth quota state.

### 4. Validation & Error Matrix

| Input / condition | Required result |
| --- | --- |
| Provider ID is not positive | Reject with `SEC_INVALID_INPUT` |
| Globally disabled but configured provider has consumer | Refresh normally; do not route it |
| No desktop/TUI consumer after lease timeout | Start no further timed remote requests |
| Desktop and TUI request the same due provider | One remote request and one shared result |
| Manual refresh overlaps scheduled refresh | Coalesce with the in-flight request |
| Provider config changes during fetch | Ignore the old-generation result; next consumer uses new config |
| Successful cache age is exactly 60 minutes | Treat as expired and expose no old amount |
| Successful timestamp is in the future | Fail closed and expose no amount |
| Refresh fails after an older success | Show failure, never the historical amount |
| Result has both plan remaining and cash balance | Project plan remaining |
| Result lacks plan remaining but has cash balance | Project cash balance |

### 5. Tests Required

- Cover lease establishment, renewal, timeout, re-entry, and no-consumer stop
  with a controllable clock.
- Cover initial, configured interval, timed-disabled, hard-expiry, failure
  recovery, and globally disabled provider refresh.
- Cover same-provider coalescing, four-provider concurrency, manual/scheduled
  overlap, invalidation generation, and independent provider schedules.
- Cover Observer omission for unconfigured providers and loading/available/
  failed projection, plan precedence, unit preservation, hard expiry, malformed
  values, and future timestamps.
- Keep the frontend exact-key cancellation tests: manual refresh uses the shared
  query function with `meta.force=true`, never a direct second cache writer.
- Assert refresh remains isolated from routing, availability, circuit, OAuth,
  mutations, reorder, and unrelated caches.

## Scenario: NewAPI Model-Token Billing Adapter

### 1. Scope / Trigger

Use this scenario when changing the NewAPI account-usage adapter, its endpoint
construction, authentication, response parsing, unit normalization, body
limits, or error presentation. The production IPC entry remains
`provider_account_usage_fetch`; this scenario owns the Rust boundary between a
configured model API key and the display-only `ProviderAccountUsageResult`.

This scenario does not change the query-owner rules above and does not apply
account-usage results to routing, circuit state, provider availability, order,
or enablement.

### 2. Signatures

The production entry and NewAPI protocol helpers are:

```rust
pub(crate) async fn provider_account_usage_fetch(
    app: tauri::AppHandle,
    db_state: tauri::State<'_, DbInitState>,
    runtime_state: tauri::State<'_, ProviderAccountUsageRuntimeState>,
    provider_id: i64,
    force: Option<bool>,
) -> Result<ProviderAccountUsageResult, String>;

pub(crate) fn build_newapi_billing_urls(base_url: &str)
    -> Result<NewapiBillingUrls, String>;

pub(crate) async fn fetch_newapi_account_usage(
    base_url: &str,
    api_key: &str,
    fetched_at: i64,
    now_unix: i64,
) -> ProviderAccountUsageResult;

pub(crate) fn parse_newapi_billing_responses(
    status_body: &Value,
    subscription_body: &Value,
    usage_body: &Value,
    fetched_at: i64,
    now_unix: i64,
) -> ProviderAccountUsageResult;
```

`build_newapi_billing_urls` produces one same-origin public status URL and two
same-origin billing URLs. `fetch_newapi_account_usage` constructs its own
no-redirect client and owns the bounded GET sequence, so a caller cannot weaken
redirect policy. `parse_newapi_billing_responses` owns all-or-nothing validation
and normalization. The existing generated IPC DTO remains the frontend boundary.

### 3. Contracts

- Normalize a configured Base URL to its deployment root, accepting both a
  root URL and a URL with a trailing `/v1`. Derive only these same-origin paths:
  `/api/status`, `/v1/dashboard/billing/subscription`, and
  `/v1/dashboard/billing/usage`.
- The status GET is unauthenticated. The subscription and usage GETs use only
  the configured model key as Bearer authentication. Never send
  `New-Api-User` on this path.
- Disable redirects for the NewAPI client so credentials cannot follow a
  redirect target. Keep sub2api's existing redirect behavior unchanged.
- Read `quota_display_type` from status before interpreting billing values.
  The implemented unit is exactly `USD`; unknown and non-USD values fail
  closed and must not inherit a USD label.
- For USD, require finite, non-negative numeric `hard_limit_usd` and
  `total_usage`, then normalize `total = hard_limit_usd`,
  `used = total_usage / 100`, and `balance = total - used`. A finite negative
  derived balance is valid overage data and is passed to the shared status
  mapping; it is not a cross-endpoint inconsistency.
- Treat `hard_limit_usd == 100_000_000.0` as the one exact NewAPI
  model-token unlimited sentinel. Return `Available` with the local
  "model token unlimited" message and leave `total`, `used`, `balance`, unit,
  and expiry empty. Do not use a greater-than threshold: adjacent finite
  values remain ordinary model-token limits.
- `access_until` must be an exact JSON integer representable as `i64`.
  Floating-point values such as `1.0`, strings, and out-of-range integers fail
  closed. Values `<= 0` mean no expiry; values `> 0` map unchanged to
  `expires_at`.
- Select the usage date window from subscription payment-method semantics:
  month start through today when a payment method exists, otherwise the
  established inclusive 100-day window through today.
- On every response, recognize root `success=false` and a root `error` before
  required-field parsing. An authenticated `success=false` is an auth failure;
  other application errors are query failures. Never expose the upstream
  message.
- The snapshot is all-or-nothing. A transport, HTTP, application, JSON, size,
  type, unit, or required-field failure at any endpoint returns no partial
  total, used amount, balance, unit, or expiry.
- Enforce explicit response limits: status 16 KiB, subscription 8 KiB, usage
  8 KiB, and sub2api 32 KiB.
- Upstream bodies/messages, API keys, PII, hosts, token names, and actual
  account amounts must not enter logs, IPC errors, fixtures, research output,
  or this spec. Fixtures use synthetic values and reserved test hosts only.
- Account usage is display-only. Fetching and parsing must not test or mutate
  routing, circuit/cooldown state, availability, provider order/enablement,
  OAuth quota, or local request usage.

### 4. Validation & Error Matrix

| Input / condition | Required result |
| --- | --- |
| Base URL is the deployment root | Derive the three documented same-origin paths |
| Base URL ends in `/v1` | Remove only that trailing API segment before deriving paths |
| Base URL contains credentials, lacks an HTTP(S) origin, or cannot be parsed | Configuration-required result; send no request |
| Status request | No Authorization or `New-Api-User` header |
| Subscription or usage request | Bearer model key only; no `New-Api-User` |
| NewAPI response redirects | Do not follow; fail the snapshot without forwarding credentials |
| `quota_display_type` is `USD` | Apply the documented billing formulas and label USD |
| Unit is missing, unknown, non-USD, or differently cased | Query failure with no unit or amounts |
| Raw total/usage is missing, negative, non-numeric, NaN, or infinite | Query failure with no partial snapshot |
| `hard_limit_usd` equals the exact unlimited sentinel | `Available`; no amount, unit, or expiry fields |
| `hard_limit_usd` is merely near or above the sentinel | Parse as an ordinary finite model-token limit |
| Used amount exceeds total | Preserve the finite negative derived balance and use shared status mapping |
| `access_until` is an exact in-range JSON integer | Preserve it exactly; map positive values to `expires_at` |
| `access_until` is `<= 0` | Successful snapshot with no expiry |
| `access_until` is a float, string, or outside `i64` | Query failure with no partial snapshot |
| Root `success=false` on an authenticated response | `AuthFailed` before required-field validation |
| Root `error` or unauthenticated application error | `QueryFailed` before required-field validation |
| Any one endpoint returns non-success HTTP, invalid JSON, or exceeds its cap | Fail the whole snapshot; never reuse values from another endpoint |
| Account usage fetch completes | No routing, circuit, availability, or provider mutation side effect |

### 5. Good / Base / Bad Cases

- Good: a Base URL ending in `/v1` yields one public status request and two
  same-origin Bearer billing requests; the model key never reaches status or a
  redirect target.
- Good: finite non-negative raw values produce total, used, and their exact
  finite difference; an overage produces a negative balance rather than a
  fabricated zero or query failure.
- Good: a large exact `i64` JSON expiry is preserved without passing through
  floating-point conversion.
- Good: the exact official unlimited sentinel produces an amount-free
  available result, while adjacent finite totals still use the normal formula.
- Base: USD status, a valid subscription, and valid usage produce one complete
  display snapshot with optional expiry.
- Bad: parse quota-like fields before checking `success=false`, causing an auth
  error to appear as a missing-field error.
- Bad: accept a partial snapshot when status or one billing endpoint fails, or
  hard-label an unknown unit as USD.
- Bad: feed account usage into availability, provider selection, circuit reset,
  or automatic provider disablement.

### 6. Tests Required

- Cover Base URL root and trailing `/v1` forms, exact derived paths, same-origin
  enforcement, disabled redirects, and rejection of credentialed/invalid URLs.
- Capture requests and assert status has no authentication, billing requests
  have Bearer authentication, no request has `New-Api-User`, and usage dates
  follow both payment-method branches.
- Assert the exact USD formula, finite overage/negative balance behavior,
  zero/expired/available status mapping, and unchanged positive expiry mapping.
- Assert exact-sentinel equality, empty amount/unit/expiry fields for unlimited
  model tokens, and ordinary finite parsing immediately below and above it.
- Assert `access_until` preserves exact integers above `2^53`, while floats,
  strings, and integers outside `i64` fail closed.
- Cover root `success=false`, root error, missing required fields, negative raw
  values, non-finite representations, unknown/non-USD units, invalid JSON,
  body caps, and each partial-endpoint failure.
- Assert errors contain no upstream message, body, key, host, PII, token name,
  or actual account amount; fixtures must remain wholly synthetic.
- Keep all sub2api fixtures green and prove its parser and redirect behavior do
  not change. Keep the query-owner, manual-refresh race, display rendering, and
  routing/circuit/availability isolation tests green.
- Run focused Rust and frontend tests, generated-binding validation, typecheck,
  lint, Rust format/check/Clippy, secret/PII diff audit, and `git diff --check`
  in proportion to the change.

### 7. Wrong vs Correct

#### Wrong

```rust
// A model key is not a user access token.
client
    .get(format!("{origin}/api/user/self"))
    .bearer_auth(model_key)
    .header("New-Api-User", user_id);

let balance_usd = quota / 500_000.0;
```

This combines incompatible authentication contracts, may mask
`success=false` as a missing quota field, and assumes a deployment-specific
divisor and USD unit.

#### Correct

```text
GET {origin}/api/status
GET {origin}/v1/dashboard/billing/subscription  Authorization: Bearer <model-key>
GET {origin}/v1/dashboard/billing/usage         Authorization: Bearer <model-key>
```

```rust
// Internally normalizes three same-origin URLs, performs bounded requests,
// and returns a snapshot only after complete response validation.
fetch_newapi_account_usage(base_url, model_key, fetched_at, now).await
```

The NewAPI client refuses redirects, every URL remains on the normalized
origin, status selects the display unit, and the billing parser applies the
documented formula only after application-error and field validation.

## Scenario: NewAPI User-Account Mode And Private Credentials

### 1. Scope / Trigger

Use this scenario when changing the NewAPI billing/account mode selector,
account credentials, provider summaries or save patches, the v38 private
credential table, the status plus user-account HTTP contract, or the editor's
configured/missing/clear behavior.

The account path is an explicitly selected alternative to model-token billing.
It is never inferred from a provider name, host, response amount, or a legacy
User ID.

### 2. Signatures

```rust
pub(crate) enum NewapiQueryMode { Billing, Account }

pub(crate) struct ProviderAccountUsageCredentialsPatch {
    pub new_api_user_id: Option<String>,
    pub new_api_access_token: Option<String>,
    pub clear_new_api_access_token: bool,
}

pub(crate) async fn fetch_newapi_user_account_usage(
    base_url: &str,
    access_token: &str,
    user_id: &str,
    fetched_at: i64,
    now_unix: i64,
) -> ProviderAccountUsageResult;

pub(crate) fn parse_newapi_account_responses(
    status_body: &Value,
    account_body: &Value,
    expected_user_id: &str,
    fetched_at: i64,
    now_unix: i64,
) -> ProviderAccountUsageResult;
```

The private SQLite owner is:

```sql
CREATE TABLE provider_account_usage_credentials (
  provider_id INTEGER PRIMARY KEY,
  newapi_user_id TEXT,
  newapi_access_token_plaintext TEXT,
  updated_at INTEGER NOT NULL,
  FOREIGN KEY(provider_id) REFERENCES providers(id) ON DELETE CASCADE
);
```

`ProviderSummary` may expose the normalized User ID and
`newapi_account_access_token_configured: bool`; it must never expose the
access token.

### 3. Contracts

- The built-in extension stores only `adapterKind`, `newApiQueryMode`,
  timed-refresh enablement, and refresh interval. Missing or unknown
  `newApiQueryMode` defaults to `billing`.
- v37-to-v38 migrates only a valid legacy `newApiUserId` into the private
  table, removes every private account field from extension JSON through the
  shared sanitizer, and cascades credential deletion with the provider.
- `None` for the entire credential patch preserves private credentials. In a
  supplied patch, a normalized positive User ID replaces the stored ID;
  absent/blank User ID clears it. A non-empty token replaces the token,
  absent/blank token preserves it, and the explicit clear flag removes it.
  Setting and clearing a token in one patch is invalid.
- User IDs contain ASCII digits only, canonicalize without leading zeroes, and
  must be in `1..=i64::MAX`, matching the signed Go `int` compatibility
  boundary used by supported NewAPI deployments. Tokens are at most 64 KiB
  and must form a valid Bearer header value.
- Switching query mode or adapter only changes extension config. It does not
  send a credential-clear patch. The explicit clear action removes both the
  User ID and token.
- Account mode loads only the private User ID/token. Missing either returns
  `ConfigurationRequired` before client creation or network I/O. It never
  reads or sends the provider model API key.
- Billing and sub2api paths load only the model API key and never read or send
  private account credentials.
- Normalize the Base URL exactly as the billing scenario does. Account mode
  derives only same-origin `/api/status` and `/api/user/self`, disables
  redirects, caps status and account bodies at 16 KiB, and uses a 15-second
  client timeout.
- Status is public. User-account GET authentication is exactly Bearer system
  access token plus `New-Api-User: <canonical-id>`.
- Account success must be the exact root boolean `true`; missing, false, or
  non-boolean success fails closed. Status requires exact `USD` and a finite
  positive `quota_per_unit`. Account identity must be a positive JSON
  integer representable as `i64` and exactly match the configured identity.
- `quota` may be any finite number and maps to `balance = quota /
  quota_per_unit`. `used_quota` must be finite and non-negative and maps to
  historical `used`. Account mode never fabricates `total`.
- Upstream messages, bodies, hosts, token names, credentials, identities, PII,
  and live amounts do not enter logs or IPC errors. Save diagnostics redact
  both nested token and User ID fields before logging.

### 4. Validation & Error Matrix

| Input / condition | Required result |
| --- | --- |
| Mode missing or unknown | Use billing; do not load account credentials |
| Account mode has only one credential | `ConfigurationRequired`; send no request |
| User ID is zero, signed, non-digit, or above `i64::MAX` | `SEC_INVALID_INPUT`; do not persist or import |
| Token is empty during an ordinary edit | Preserve the stored token |
| Explicit clear is selected | Delete User ID and token together |
| Set and clear token are both requested | `SEC_INVALID_INPUT`; preserve stored credentials |
| Status request | No Authorization or `New-Api-User` |
| Account request | Bearer account token and matching canonical `New-Api-User` |
| Redirect, non-success HTTP, invalid JSON, or body cap failure | Fail all-or-nothing; forward no credential |
| Account root `success` is not exactly `true` | `QueryFailed`; expose no amounts |
| Account ID is invalid or does not match | `QueryFailed` or `AuthFailed`; expose no amounts |
| Unit/divisor/value is missing, non-finite, or unsupported | `QueryFailed`; expose no partial result |
| Valid negative quota and non-negative historical use | Preserve negative balance; leave total empty |

### 5. Good / Base / Bad Cases

- Good: explicit account mode with complete private credentials performs one
  public status request and one private user request, then returns balance and
  historical use without a total.
- Good: mode and adapter switches preserve an unsaved secret draft and stored
  credentials; returning to account mode restores the same draft.
- Base: legacy NewAPI config without a mode continues through model-token
  billing even if a migrated User ID exists.
- Bad: infer account mode because a User ID exists, use the model API key as a
  system token, or accept an identity above the signed range.
- Bad: log a failed upsert payload before replacing both account identity and
  token fields with a redacted placeholder.

### 6. Tests Required

- Migration: valid/invalid legacy User IDs, sanitized extension JSON, v38
  idempotence, private table creation, and delete cascade.
- Persistence: whole-patch preserve, User ID set/clear, token preserve/set/
  clear, set-plus-clear rejection, local copy, and summaries without token.
- Protocol: root and trailing-`/v1` URLs, public/private headers, no redirects,
  both 16 KiB caps, exact success boolean, signed identity bounds and match,
  unit/divisor/value matrices, and all-or-nothing failures.
- Isolation: prove account mode never loads the model key and billing/sub2api
  never load private credentials; prove missing credentials send no request.
- Frontend: explicit selector, legacy billing default, partial-save allowed,
  configured/missing state, masked token, explicit clear, mode/adapter draft
  preservation, and nested diagnostic redaction.
- Run generated-binding validation, focused and full frontend/Rust tests,
  typecheck, lint, format, Clippy, secret/PII diff audit, and diff check.

### 7. Wrong vs Correct

#### Wrong

```rust
// Credential presence must not select the protocol.
if credentials.new_api_user_id.is_some() {
    fetch_user_self(base_url, model_api_key).await
}
```

#### Correct

```text
mode=billing -> model key -> status + subscription + usage
mode=account -> private User ID/token -> status + user/self
missing account credential -> local ConfigurationRequired, zero requests
```

Selection is explicit, each branch loads only its own credential class, and
every account response is identity- and unit-validated before projection.

## Scenario: sub2api Daily Rate-Limit Projection

### 1. Scope / Trigger

Use this scenario when changing sub2api `rate_limits` parsing or the daily
usage fields shown by `ProviderAccountUsageResult`. The parser may project a
proved daily window; it must not reinterpret periodic limits as wallet balance.

### 2. Signatures

```rust
pub(crate) fn parse_account_usage_response(
    adapter_kind: ProviderAccountUsageAdapterKind,
    body: &Value,
    fetched_at: i64,
    now_unix: i64,
) -> ProviderAccountUsageResult;
```

The existing result fields are `daily_used` and `daily_total`. No new
balance field or window-specific credential is introduced.

### 3. Contracts

- If `rate_limits` is present, it must be an array. Recognize only an exact
  string `window == "1d"`; unknown windows are ignored rather than guessed as
  day, week, or month.
- At most one `1d` entry is valid. Its `limit`, `used`, and `remaining`
  are finite and non-negative, `used <= limit`, and `remaining` equals
  `limit - used` within `max(1e-9, abs(limit) * 1e-9)`.
- `window_start` and `reset_at` must parse as timestamps and
  `reset_at > window_start`.
- A valid entry maps only `limit -> daily_total` and `used -> daily_used`.
  It does not populate `balance`, `plan_remaining`, weekly, or monthly
  fields.
- Existing root balance/remaining, plan remaining, subscription, expiry, and
  validity behavior remains unchanged. A successful validity-only payload with
  no recognized period remains `Available`.
- A malformed or duplicate known `1d` entry fails the result closed; the
  parser must not silently drop it and show a partially valid snapshot.

### 4. Validation & Error Matrix

| Input / condition | Required result |
| --- | --- |
| No `rate_limits` | Preserve legacy sub2api parsing |
| `rate_limits` is not an array | `QueryFailed`; no partial daily values |
| Only unknown windows exist | Ignore them; do not invent periodic fields |
| One valid `1d` entry | Populate daily total/used only |
| Duplicate `1d` entries | `QueryFailed` |
| Negative/non-finite amount or `used > limit` | `QueryFailed` |
| Remaining arithmetic exceeds tolerance | `QueryFailed` |
| Missing/invalid timestamps or reset not after start | `QueryFailed` |

### 5. Good / Base / Bad Cases

- Good: an otherwise validity-only response with one consistent `1d` entry
  displays daily used/total and no wallet balance.
- Base: a legacy root balance or subscription payload parses exactly as before.
- Base: an unknown periodic window is ignored and does not become daily data.
- Bad: map `remaining` from a `1d` window into account `balance`.
- Bad: accept the first of two `1d` entries or silently ignore malformed
  known-window arithmetic.

### 6. Tests Required

- Cover one valid `1d` entry, absent limits, unknown-only windows, duplicate
  known windows, non-array input, every amount validation, tolerance boundary,
  timestamp parsing, and reset ordering.
- Assert daily projection leaves balance, plan remaining, weekly, and monthly
  fields untouched.
- Keep all legacy root-balance, plan, subscription, expiry, validity-only,
  authentication, body-cap, and display regressions green.
- Assert account usage remains display-only and never affects provider routing,
  availability, circuit state, order, or enablement.

### 7. Wrong vs Correct

#### Wrong

```rust
result.balance = rate_limit["remaining"].as_f64();
```

#### Correct

```rust
result.daily_total = Some(validated_limit);
result.daily_used = Some(validated_used);
// Account balance remains absent unless a legacy balance contract proves it.
```

Window semantics stay in their matching DTO fields; field-name similarity does
not create a wallet-balance contract.

## Scenario: Local Custom JavaScript Account-Usage Adapter

### 1. Scope / Trigger

Use this scenario when changing the `custom` account-usage adapter, its local
extension fields, native confirmation, QuickJS boundary, draft-test IPC,
credential loading, HTTPS request policy, portable configuration sanitizers,
or normalized result mapping.

The adapter is available only to direct API-key providers. It performs one
display-only JSON request and reuses the existing provider-scoped query owner;
it never affects routing, availability, circuit state, provider order, or
enablement.

### 2. Signatures And Script Contract

The JavaScript source evaluates to one object with two synchronous functions:

```javascript
({
  request: (ctx) => ({
    url: ctx.baseUrl + "/v1/usage",
    method: "GET",
    headers: { Authorization: `Bearer ${ctx.apiKey}` }
  }),
  parse: (response) => ({
    status: "available",
    balance: response.data.balance,
    unit: "USD"
  })
})
```

`request(ctx)` sees opaque internal placeholders for `ctx.apiKey` and
`ctx.baseUrl`, not the plaintext credential values. `ctx.baseUrl` represents a
Base URL whose trailing slashes are removed by the backend, so scripts append
paths directly to it. String transforms applied in JavaScript affect only the
opaque placeholder and must not be used to transform the later plaintext
replacement. Rust serializes and validates the returned
request plan, then replaces each placeholder exactly once while enforcing the
final URL/header/body size limits. Replacement values are never scanned again,
so placeholder-looking text inside a real key or Base URL cannot trigger a
second substitution.

`parse(response)` receives only `{ status, data }`, where `data` is the bounded
JSON response after exact plaintext credential/Base-URL redaction. It receives
no `ctx`, API key, Base URL, request headers, request URL, or raw response body.
It returns the existing normalized `ProviderAccountUsageResult` fields.

The draft entry point is:

```rust
provider_account_usage_test_custom_script(
    provider_id: i64,
    draft: ProviderAccountUsageCustomScriptDraft,
) -> Result<ProviderAccountUsageResult, String>;
```

The renderer sends only the provider ID, source, extra origins, and timeout.
The backend owns native confirmation and credential loading. Draft tests do not
persist configuration or write the TanStack Query cache.

### 3. Local Configuration And Permission Contracts

- Custom source is at most 32 KiB in UTF-8, has one bounded timeout in 2-15
  seconds, and has at most 16 normalized extra HTTPS origins of at most 512
  bytes each. Origin entries must be origin-only URLs without user info, path,
  query, or fragment.
- `customEnabled` defaults to false. Enabling or testing requires a native
  confirmation that lists every effective HTTPS origin and the full SHA-256
  permission fingerprint. The confirmation states that the script and all
  target services are trusted to read or forward the API key.
- Only one native custom-usage confirmation may be open. A concurrent request
  fails immediately with `SEC_CONFIRM_BUSY`; it does not queue another dialog.
- The permission fingerprint binds the exact script bytes and normalized extra
  origins. The confirmed Base URL origin is stored and checked separately.
  Changing the script, extra-origin set, or Base URL origin revokes automatic
  execution until another native confirmation succeeds.
- Renderer-supplied proof fields are stripped before permission evaluation.
  Only the backend may inject the transient proof after confirmation. Proof
  fields are never persisted, returned, shared, exported, backed up, or
  imported; only the derived local fingerprint/Base-origin binding may persist.
- Saved execution loads Base URLs, auth/source identity, extension values, and
  plaintext key from one credential-context query, then rejects a stale mix if
  it differs from the configuration snapshot that authorized execution. Draft
  execution confirms first, then loads the same credential snapshot and rejects
  provider identity or Base URL changes before using the key.
- An API-key-only rotation does not revoke a confirmation: permission binds the
  exact script and target origins, not one secret version. The post-confirmation
  snapshot deliberately uses the provider's current key. Base URL, auth mode,
  or source-provider identity changes still fail as stale.
- No API key is read before the draft native confirmation succeeds. Canceling,
  closing, or failing confirmation performs no credentialed request.

### 4. Runtime, Network, And Result Contracts

- Each `request` and `parse` phase uses a fresh one-shot child process hosting a
  QuickJS runtime with an 8 MiB memory limit, 256 KiB stack, and 100 ms engine
  interrupt deadline. After a bounded startup handshake, the parent applies a
  separate 100 ms call deadline and kills and reaps the child on timeout. The
  process deadline is the hard boundary for native QuickJS built-ins that do
  not poll the engine interrupt handler.
- The script worker receives only source, a fixed `request`/`parse` selector,
  opaque request placeholders, or the already-redacted response JSON over a
  bounded stdio protocol. Plaintext credentials and the materialized Base URL
  remain in the parent process and never enter worker arguments, environment,
  configuration files, or stdio.
- The runtime exposes only the required JavaScript/JSON/RegExp intrinsics. It
  has no `fetch`, modules, filesystem, process, environment, Tauri IPC, timers,
  WebSocket, or other host/network primitive.
- At most four complete custom workflows may run concurrently. A non-waiting
  semaphore covers request evaluation, credentialed HTTP, and response parsing;
  excess work returns a stable busy result instead of creating an unbounded
  wait or HTTP queue.
- One request may use only `GET` or `POST` and only an HTTPS URL whose exact
  origin is the configured Base URL origin or a normalized extra origin.
  Redirects are disabled and every 3xx response fails. Reject user info and
  forbidden hop-by-hop, host, length, or proxy-authorization headers.
- Enforce final materialized limits: URL 16 KiB, at most 32 headers, header name
  128 bytes, header value 8 KiB, request body 64 KiB, response body 64 KiB, and
  serialized JavaScript output 64 KiB.
- Require a successful HTTP status and valid JSON. Reject `NaN`, infinity,
  invalid statuses, invalid field types, non-integer expiry, and strings longer
  than the result text cap. Unknown output fields are ignored.
- Error/auth/configuration statuses are all-or-nothing: no balance, usage,
  total, unit, expiry, or other partial account fields survive. Errors are
  stable local categories and never include source, URL, request headers,
  response body, upstream message, or plaintext credentials.

### 5. Explicit Trust Boundary

Arbitrary secret-bearing requests plus arbitrary response parsing create an
inherent information channel. A trusted endpoint can echo the key after an
encoding or transformation, and a trusted script can map that value into a
normalized result field. Exact recursive plaintext redaction is defense in
depth against accidental reflection; it is not a claim that the application
can prove what arbitrary code or a remote service intends.

Therefore native confirmation means the user explicitly trusts both the exact
fingerprinted script and every displayed target origin with the provider API
key. Do not describe this adapter as safe for untrusted scripts or endpoints.
If a future product requirement needs a hard guarantee that transformed key
material cannot reach the renderer, replace arbitrary JavaScript/request
construction with a backend-owned declarative credential and field-mapping
protocol; additional redaction heuristics cannot establish that guarantee.

### 6. Portability Contract

Custom source, extra origins, enabled state, permission fingerprint, confirmed
Base origin, and all proof fields are local-only. Provider sharing,
configuration export, and configuration backup must omit them and serialize a
disabled account-usage adapter while preserving canonical portable refresh
settings. Import sanitization must drop crafted custom fields and cannot
activate a script. Native Sub2API/NewAPI adapter settings and their existing
portability rules remain unchanged.

### 7. Validation Matrix

| Input / condition | Required result |
| --- | --- |
| Valid direct API-key provider, confirmed local config | Run one bounded request and return one normalized snapshot |
| Script or normalized extra origins change | Clear enabled permission; require native confirmation |
| Base URL changes to another origin | Reject saved execution and require native confirmation |
| Concurrent native confirmation | Reject immediately with `SEC_CONFIRM_BUSY` |
| Concurrent custom workflow beyond four | Return busy result without waiting or sending HTTP |
| URL is HTTP, credentialed, redirected, or outside the exact allowlist | Send no credential or reject the response |
| Request/output exceeds a final materialized limit | Fail closed before the oversized allocation/request crosses the boundary |
| Base URL, auth mode, or source-provider identity changes while draft confirmation is open | Reject as stale; do not send a key under old network authorization |
| Only the provider API key rotates while draft confirmation is open | Use the current key; script and target permission remain unchanged |
| Parser returns a failure status with account values | Drop every partial account value |
| Crafted share/export/backup/import contains custom fields | Strip and disable; never execute |

### 8. Tests Required

- Cover UTF-8 source limits, origin normalization/count/length limits including
  duplicate canonical origins, timeout bounds, permission invalidation,
  Base-origin rebinding, proof stripping, and proof/fingerprint non-portability.
- Cover absent host capabilities, missing functions, syntax/runtime failure,
  interpreted infinite execution, a native built-in that does not poll the
  QuickJS interrupt handler, child timeout/reaping, non-finite output after
  mutable-global tampering, strict field normalization, exact credential
  redaction, single-pass placeholder replacement, and expansion limits using
  synthetic secrets and reserved test origins.
- Cover method, HTTPS origin, user-info, redirect, header, request body,
  response body, strict UTF-8/JSON, HTTP/auth, timeout, percent-encoded final
  URL expansion, and all-or-nothing failure paths. Script-provided failure
  messages and upstream response content must not survive those failures.
- Cover saved and draft credential-snapshot races, native confirmation content,
  confirmation single-flight, full-workflow concurrency, cancellation before
  credential loading, and local-only share/export/backup/import behavior.
- Keep generated bindings, TypeScript config normalization, editor reset/test
  races, query ownership, native adapters, and routing/circuit isolation green.
