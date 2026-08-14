# Gateway Listen And Token Contract

## Scenario: Change Gateway Listener Or Access-Token Presentation

### 1. Scope / Trigger

Use this contract when changing gateway listen modes or custom addresses,
runtime rebinding, lifecycle locking, CLI proxy synchronization, non-loopback
request authentication, bearer-token generation/reveal/acknowledgement, or the
CLI Manager UI that saves listener settings and presents the token.

This contract complements the
[settings ownership and rollback contract](./settings-ownership-rollback-contract.md).
The settings writer still owns its ordinary fields and CAS rollback; this
contract defines the runtime critical section and the one-shot credential
boundary that follow a listener change.

### 2. Signatures And Owners

The backend listener transaction has two explicit lock paths:

```rust
async fn sync_cli_proxy_for_settings(...);
async fn sync_cli_proxy_for_settings_unlocked(...);

async fn sync_cli_proxy_for_settings_with_lifecycle_guard(
    ...,
    lifecycle_guard: Option<&GatewayLifecycleGuard>,
) -> bool;
```

The public token IPC remains:

```text
gateway_bearer_token_reveal() -> GatewayBearerTokenReveal | null
gateway_bearer_token_rotate() -> GatewayBearerTokenReveal
gateway_bearer_token_acknowledge() -> boolean
```

`CliManagerPage` owns `useGatewayTokenController` for the lifetime of the page.
`NetworkSettingsCard` owns only listener draft state, validation, save progress,
rollback, and the callbacks that request reveal or rotation from that owner.

### 3. Contracts

#### Listener transaction and lock ownership

- Listener planning uses the resolved binding. A preferred-port, listen-mode,
  custom-address, or relevant WSL host-address change requires rebinding and
  CLI proxy synchronization according to the existing runtime plan.
- When the gateway was running, `settings_set` acquires the gateway lifecycle
  guard once and holds it across stop/start rebinding, effective runtime-state
  resolution, and CLI proxy synchronization. Every operation below that owner
  uses its `_unlocked` path; no helper may wait for the same non-reentrant lock.
- A caller without a lifecycle guard uses the locked wrapper. Passing
  `Option<&GatewayLifecycleGuard>` is ownership evidence, not an optional
  synchronization preference: `Some` selects the unlocked core and `None`
  selects the wrapper that acquires the guard.
- Runtime rollback remains governed by the settings owned-field CAS result.
  Only `Restored` may restore the previous runtime. If the previous gateway was
  running, restoration uses the caller-held lifecycle guard and unlocked
  stop/start path; a concurrent settings winner is preserved and resynchronized.
- Removing the nested acquisition must not weaken serialization. Listener
  rebinding and CLI proxy sync remain one lifecycle transaction, and unrelated
  callers continue to acquire the wrapper lock.

#### Non-loopback authentication and token storage

- Listener mode is not the authorization decision. The gateway checks the
  actual peer address: loopback peers retain the existing internal exception;
  every non-loopback peer must send exactly one valid `Authorization: Bearer
  <token>` header. Missing, duplicate, malformed, wrong, or stale credentials
  receive the existing empty `401` response with `WWW-Authenticate: Bearer`.
- After admission, client-controlled authorization, provider identity, and
  forwarding headers are removed before proxy dispatch. Gateway access-token
  changes must not alter upstream provider authentication.
- Tokens keep the existing 32-byte random / 43-character URL-safe shape and
  constant-time digest verification. The AIO private sidecar stores only the
  schema version, SHA-256 digest, generation, and confirmation flag; it never
  stores plaintext and retains its existing atomic write and Unix `0600`
  permissions.
- Plaintext exists only in the process-owned pending generation, the bounded
  reveal response, the page-lifetime React state, and existing internal managed
  client synchronization. It must not enter logs, URLs, query keys, TanStack
  Query cache, browser storage, task records, fixtures presented as real
  credentials, or AIO's persisted token state.

#### One-shot reveal and frontend state

- `reveal` consumes `pending` with take-once semantics and records the revealed
  generation. `acknowledge` confirms only that same current generation.
  `rotate` creates and reveals a new generation. Closing the dialog without
  acknowledgement clears the frontend plaintext but does not recreate backend
  pending state; the user must rotate to obtain plaintext again.
- Initial pending-token recovery and post-save presentation call the same
  page-level serialized/deduplicated reveal operation. While one reveal Promise
  is in flight, later callers reuse it and cannot concurrently consume the
  backend one-shot value.
- The controller and dialog remain mounted when General tab unmounts. A reveal
  result that arrives after a tab switch must still be shown, copied,
  acknowledged, or closed from the page owner.
- Listener save enters an explicit applying state and disables conflicting
  listener/token controls. A successful response resets the draft from the
  returned canonical settings and immediately requests reveal when that
  canonical listener is non-loopback. A `null` response or error resets from
  the latest real settings, does not reveal, and always leaves applying state.
- External settings-to-draft synchronization happens in an effect or an
  equivalent post-render boundary. It must not dispatch during render and must
  not overwrite a user choice while its save is still applying.

### 4. Validation And Transition Matrix

| Condition | Required result |
| --- | --- |
| Running gateway changes `localhost -> lan` | One lifecycle guard covers rebind and CLI proxy sync; mutation completes within the bounded test timeout |
| Running gateway changes non-loopback -> `localhost` | Same single-guard transaction completes; no token reveal is requested for canonical localhost |
| Caller has no lifecycle guard | Locked wrapper acquires the guard before CLI proxy sync |
| Runtime side effect fails and owned-field CAS restores | Previous runtime is restored under the existing guard without reacquisition |
| Runtime side effect fails after a concurrent winner | Preserve and resynchronize the winner; do not restore stale runtime |
| Non-loopback peer lacks one exact valid Bearer header | Empty `401` with the existing Bearer challenge; no proxy dispatch |
| Loopback peer omits the gateway token | Preserve the existing loopback exception |
| First pending reveal | Return plaintext once and bind acknowledgement to that generation |
| Concurrent initial/save reveal calls | Reuse one in-flight operation; at most one backend reveal call for that flight |
| User closes without acknowledgement | Clear frontend plaintext; a later plaintext display requires rotate |
| Listener save returns canonical non-loopback settings | Reset draft, request reveal immediately, then return controls to idle |
| Listener save returns `null` or throws | Reset to latest canonical settings, do not reveal, and return controls to idle |
| General tab unmounts while reveal is pending | Page owner receives and presents the eventual result |

### 5. Good / Base / Bad Cases

- **Good:** a running localhost gateway switches to LAN while one caller-owned
  lifecycle guard covers stop, start, and CLI proxy sync; the canonical response
  triggers the page controller's one-shot reveal before the interaction ends.
- **Good:** two reveal triggers during the same in-flight request share one
  Promise. Switching tabs does not unmount the owner or lose the plaintext.
- **Base:** a localhost-only listener stays usable without a token for loopback
  peers; a persisted digest may remain private but does not change that peer
  exception.
- **Bad:** a helper called under `settings_set` acquires the lifecycle lock a
  second time, causing the running-gateway mutation or its rollback to hang.
- **Bad:** `NetworkSettingsCard` mounts its own reveal effect, allowing tab
  remount and save completion to race for `pending.take()`.
- **Bad:** a rejected or `null` save leaves the optimistic listener selected,
  or stores the plaintext in cache, logs, browser storage, or task evidence.

### 6. Tests Required

- Use behavior tests with `tokio::time::timeout` for both localhost-to-LAN and
  non-loopback-to-localhost branches while a lifecycle guard is already held.
  Source-string assertions do not prove the deadlock is removed.
- Keep gateway route tests for missing, malformed, duplicate, wrong, stale, and
  valid Bearer credentials; keep loopback admission and credential/header
  stripping coverage.
- Keep token tests proving strict random shape, digest-only AIO persistence,
  one-shot reveal, generation-bound acknowledgement, rotation, and secret-free
  diagnostics.
- Frontend tests must cover applying feedback, canonical success, `null`, error,
  LAN-to-localhost recovery, effect-based external synchronization, immediate
  post-save reveal, one in-flight reveal, General-tab unmount, copy,
  acknowledge, rotate, and close-without-ack.
- GitHub Actions must run full-scope frontend and Rust jobs for listener/token
  changes. Locally use only the cloud-only verification allowlist.

### 7. Wrong Vs Correct

```rust
// Wrong: settings_set already owns the lifecycle guard.
let _guard = gateway_lifecycle_lock::lock().await;
sync_cli_proxy_for_settings(app, origin, true).await; // reacquires and hangs

// Correct: the owner passes explicit evidence to the shared coordinator.
let guard = gateway_lifecycle_lock::lock().await;
sync_cli_proxy_for_settings_with_lifecycle_guard(
    app,
    origin,
    true,
    Some(&guard),
)
.await;
```

```text
Wrong: General mount reveal ----\
                                 +--> concurrent pending.take() / lost UI owner
       listener-save reveal ----/

Correct: initial recovery ------\
                                  +--> one page-level in-flight Promise --> dialog
         listener-save success --/
```
