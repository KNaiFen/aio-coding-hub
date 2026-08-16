# Codex Config Contract

## Scenario: Mutate Codex Configuration While AIO Projections Are Active

### Canonical And Live Layers

The direct Codex configuration is the canonical semantic document. The file at
`$CODEX_HOME/config.toml` is the live document and may additionally contain two
AIO-owned projections:

```text
canonical direct config
  -> optional CLI-proxy provider/auth projection
  -> optional managed model_catalog_json pointer
  -> live config.toml
```

Reads, structured saves, raw TOML saves, and Codex MCP sync operate on the
canonical document. When the proxy is enabled, its manifest backup stores that
canonical document and the live file is rebuilt from it. A managed catalog
pointer is then composed onto the live projection. Proxy-only provider/auth
keys and the AIO-generated catalog path must never become canonical bytes.

### Lifecycle Coordinator

- Structured/raw config writes, Codex MCP sync, proxy enable/sync/disable,
  managed profile/catalog changes, the 372K preference, startup repair, and
  exit restoration share `lock_profile_lifecycle` as their outer Codex lock.
- A mutation checks the shutdown admission gate after acquiring that lock.
  Exit closes admission before restoring proxy and catalog projections.
- The fixed order is lifecycle lock -> proxy/MCP metadata -> canonical bytes ->
  live projection -> managed catalog. Code inside this sequence must not
  reacquire the lifecycle lock.
- Before writing, validate proxy/catalog ownership and snapshot every affected
  file. On failure, compensate only a file whose current bytes still equal the
  bytes written by that transaction. External drift is preserved and reported
  as recovery required.
- Startup repairs proxy manifest state first and then re-applies the catalog
  policy. Exit restores direct bytes without changing the persisted policies.

### Required Regression Evidence

- Proxy enable -> structured/raw/MCP save -> proxy disable or exit preserves
  the semantic edit while restoring direct provider, auth, provider-table, and
  catalog-pointer semantics.
- Repeated projection/sync is idempotent, preserves comments and unknown keys,
  and leaves direct backups free of AIO projection-only values.
- Failure and drift tests cover live config, proxy backup/manifest, MCP
  backup/manifest, generated catalog, and preference persistence.
- Concurrent config/MCP/proxy/catalog/exit operations serialize without lost
  updates or competing lock order.

## Scenario: Add Or Change A Structured Codex Config Field

### 1. Scope / Trigger

Use this contract when a root `config.toml` field is exposed through AIO's
structured Codex settings. The field crosses these owners:

```text
config.toml
  -> src-tauri/src/infra/codex_config
  -> src/generated/bindings.ts
  -> src/services/cli/cliManager.ts
  -> src/components/cli-manager/tabs/CodexTab.tsx
```

This contract prevents a field from being readable in one layer but silently
cleared, rejected, or misrepresented in another.

### 2. Signatures

The Rust source of truth is in
`src-tauri/src/infra/codex_config/types.rs`:

```rust
pub struct CodexConfigState {
    pub approvals_reviewer: Option<String>,
}

pub struct CodexConfigPatch {
    pub approvals_reviewer: Option<String>,
}
```

Specta generates both TypeScript fields as `string | null` in
`src/generated/bindings.ts`. The public frontend patch type is
`Partial<GeneratedCodexConfigPatch>`; `DEFAULT_CODEX_CONFIG_PATCH` in
`src/services/cli/cliManager.ts` supplies `null` for omitted generated fields.

### 3. Contracts

- Read: `make_state_from_bytes` reads root string values verbatim. Supported
  and future strings remain observable through `CodexConfigState`.
- Structured patch omitted / `null`: do not modify the existing TOML key.
- Structured patch empty string: delete the root key.
- Structured patch supported string: upsert exactly one root key through the
  existing patch helpers.
- Structured patch unsupported string: fail before output bytes are produced.
- Full raw TOML save: validate the complete file before atomic write. Fields
  with exact enums must reject empty, non-string, padded, and unknown values.
- Generated boundary: edit Rust types and let GitHub Actions generate and
  format the TypeScript bindings. Apply only the reviewed bounded drift patch;
  do not generate locally or hand-maintain generated types as the source of truth.
- Frontend adapter: a `null` default must deserialize to Rust `None`; it is not
  the same as the empty-string deletion signal.
- UI: preserve unknown current values with a synthetic option. Passive render
  and changes to a companion setting must never clean up that value.

For `approvals_reviewer`, `approval_policy` decides whether a request exists
and the reviewer decides who evaluates an eligible request. The UI may warn
about an ineffective combination, but only an explicit user action may patch
the companion field.

### 4. Validation & Error Matrix

| Boundary | Input | Result |
| --- | --- | --- |
| Structured patch | field omitted / `null` | Preserve current key |
| Structured patch | `""` | Delete current key |
| Structured patch | `user` / `auto_review` | Upsert root string |
| Structured patch | other non-empty string | Return validation error |
| Raw TOML | exact `"user"` / `"auto_review"` | Accept |
| Raw TOML | empty or padded string | Reject before write |
| Raw TOML | non-string or unknown string | Reject before write |
| Structured read | unknown string already on disk | Return it verbatim |
| Unrelated structured patch | unknown reviewer on disk | Preserve it verbatim |

The raw reviewer enum uses exact comparison in
`validate_root_exact_string_enum`. Do not use the trim-tolerant generic enum
validator for a field whose raw contract requires exact values.

### 5. Good / Base / Bad Cases

- Good: select `auto_review`; write one
  `approvals_reviewer = "auto_review"` root key and preserve comments/tables.
- Base: patch only `model`; an existing future reviewer value remains intact.
- Good: render `auto_review + never` as ineffective and offer an explicit
  `approval_policy = "on-request"` action.
- Bad: map an unknown reviewer to the unset option; this hides external state
  and encourages accidental cleanup.
- Bad: silently rewrite `approval_policy` when the reviewer selector changes.
- Bad: validate a full raw save after writing; invalid input must leave the
  previous file bytes unchanged.

### 6. Tests Required

- `src-tauri/src/infra/codex_config/tests.rs`
  - Parse supported and unknown strings exactly.
  - Write supported values once, delete on empty, and reject unsupported
    structured values.
  - Preserve unknown values during unrelated patches.
  - Cover empty, padded, non-string, and unknown raw values.
- `src-tauri/tests/codex_config_toml_raw.rs`
  - Assert each invalid full-file save leaves existing bytes unchanged.
- `src/services/cli/__tests__/cliManager.service.test.ts`
  - Assert omitted frontend fields normalize to `null` and explicit values
    cross the generated command boundary.
- `src/components/cli-manager/tabs/__tests__/`
  - Exhaust the pure policy/reviewer matrix.
  - Assert unknown-value display, direct selector patches, and companion-field
    changes only from the explicit action.

GitHub Actions must run the focused frontend/Rust tests, generate bindings,
format, lint, type-check, run Clippy, and complete the affected Rust suite.
Locally, use only the dependency-free repository contract and diff check:

```bash
node scripts/check-local-verification.mjs --base <full-task-base-sha>
```

### 7. Wrong vs Correct

#### Wrong

```typescript
// Changing reviewer silently changes when approvals are requested.
persistCodexConfig({
  approvals_reviewer: "auto_review",
  approval_policy: "on-request",
});
```

#### Correct

```typescript
// Selector changes only its own field.
persistCodexConfig({ approvals_reviewer: "auto_review" });

// A separate user-invoked action changes only the policy.
persistCodexConfig({ approval_policy: "on-request" });
```
