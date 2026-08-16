# Codex 配置事务与代理恢复设计

## Context

Codex configuration currently has two meanings but not two explicit layers:

- the user's direct configuration that must remain valid when AIO's proxy is off;
- the live configuration projected for the AIO proxy while it is on.

The current setter reads the live file and refreshes the direct backup from those live bytes. Proxy disable/exit later restores selected keys from that backup. A save during proxy operation can therefore make proxy-only values look canonical. Structured config, raw TOML, MCP sync, proxy lifecycle, managed catalog, and exit cleanup also lack one shared transaction coordinator.

## State Model

Define three explicit states:

1. `CanonicalCodexConfig`: the semantic direct baseline, including legitimate user changes.
2. `LiveCodexConfig`: a deterministic projection of the canonical baseline plus active AIO proxy/catalog overlays.
3. `CodexLifecycleManifest`: ownership, expected hashes, backup references, projection generation, and interrupted-transaction phase.

The canonical state may use the existing direct backup location and manifest, but it must no longer be refreshed from an already projected live file. The exact storage format is an implementation decision inside the existing private Rust boundary; the semantic roles are fixed.

## Coordinator And Mutation Flow

Introduce one backend-owned Codex lifecycle coordinator. Callers submit semantic operations rather than writing files directly:

- structured config patch;
- validated raw TOML replacement;
- MCP-table replacement;
- proxy enable, re-sync, disable, or exit restore;
- managed catalog policy/profile change.

For an ordinary mutation:

1. Acquire the lifecycle lock and reject the operation if exit has begun.
2. Resolve and validate Codex paths once.
3. Read canonical, live, manifest, backup, and catalog pre-images with bounded readers.
4. Derive the next canonical bytes from the canonical pre-image.
5. Derive the next live bytes by applying current proxy/catalog projection to the next canonical bytes.
6. Record the transaction intent and expected hashes.
7. Atomically write the canonical/backup state, owned catalog state if applicable, live projection, and committed manifest in the documented order.
8. Clear the transaction intent only after all read-back ownership/hash checks pass.

If the proxy is disabled, canonical and live converge and no proxy projection is added.

## Locking, Drift, And Rollback

- The lifecycle lock is outermost. Existing narrower locks may remain only if they cannot be acquired independently by participating writers and their order is documented.
- Before each write, compare the current file with the captured expected pre-image. Drift aborts before overwrite.
- Before rollback, compare the current file with the bytes written by this transaction. Roll back only an owned value; never overwrite a later external edit.
- Return stable, bounded error categories for drift, invalid input, partial recovery required, and ordinary I/O failure.
- Do not include file contents or secrets in errors or logs.

Concurrency tests use barriers and failpoints, not timing sleeps.

## Crash Recovery

The lifecycle manifest records a monotonically increasing generation and the current transaction phase. Startup recovery runs before proxy startup or managed-catalog projection:

- no intent: validate ordinary ownership and continue;
- intent with only canonical state written: either finish the derived projection or restore the owned canonical pre-image;
- intent with live/catalog state written: finish manifest commit when hashes match, otherwise restore only owned writes;
- any external drift: preserve external state, mark recovery required, and keep the proxy from starting with an ambiguous config.

Exit first closes the mutation gate, then acquires the same coordinator and restores the direct projection. The existing timeout remains a lifecycle concern; it must not allow a new writer to race after restoration begins.

## Participating Modules

Expected primary change surface:

- `src-tauri/src/infra/codex_config/mod.rs` and its parsing/patching/tests;
- `src-tauri/src/infra/cli_proxy/mod.rs`, `codex.rs`, and proxy tests;
- `src-tauri/src/infra/mcp_sync/sync.rs`;
- `src-tauri/src/infra/codex_model_catalog/managed.rs`;
- Codex application service/startup/cleanup/resident lifecycle modules;
- `.trellis/spec/aio-coding-hub/cross-layer/codex-config-contract.md`;
- `.trellis/spec/aio-coding-hub/cross-layer/codex-managed-model-route-contract.md`.

Exact public command signatures should remain stable unless the implementation proves a private command split is necessary. Changes outside this surface require main-session review against the stop conditions.

## Compatibility And Security

- Continue using structured TOML/JSON parsers and atomic same-directory replacement.
- Preserve current file-size limits and strengthen path validation by reusing the managed-catalog ancestor symlink/reparse checks where the config path is less protected.
- Preserve unknown TOML and JSON fields.
- Test only in temporary homes with serialized environment mutation where required.
- Never read or write the developer's real Codex home during tests or acceptance.

## Verification Design

Pure tests cover canonical mutation and live projection. Temp-home integration tests cover enable/save/disable, enable/MCP/exit, failure at every transaction phase, external drift, startup recovery, and the concurrency matrix in the PRD. Repository policy leaves Rust, integration, and platform coverage to GitHub Actions; local evidence is limited to the fixed `$gkd-local-verify` contract.
