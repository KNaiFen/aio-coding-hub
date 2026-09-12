# 增加 Codex 372K 上下文开关

## Plan Status

- Implementation authorization: confirmed
- Confirmation date and summary: 2026-08-17; the user confirmed no further requirements and authorized worktree creation, handoff, execution documentation, and implementation
- Confirmed coverage: Codex configuration transaction hardening, the exact 372K catalog policy, UI, backup/restore, no-runtime-verification rule, and acceptance criteria below
- Planning revision: the formerly separate config-hardening plan is consolidated into this task by user decision; the full planning SHA is recorded by `task.py delegate`
- Execution route: one delegated Trellis worktree and one unique writer for the complete Codex config/catalog lifecycle
- Migrated from direct-main record: none; the earlier `08-17-codex-config-transaction-hardening` planning record was consolidated before implementation and remains recoverable from Git history

## Material Facts, Assumptions, and Open Questions

| Item | Source | Status / closure condition |
|---|---|---|
| Structured/raw Codex saves currently derive from live `config.toml` and can copy proxy-only bytes into the direct backup | `infra/codex_config/mod.rs`, `infra/cli_proxy/codex.rs` | Confirmed defect; fix inside this task before applying the 372K overlay through that lifecycle |
| Config save, MCP sync, proxy lifecycle, managed catalog, startup, and exit can write related Codex state without one shared transaction boundary | Current Rust modules | Confirmed; one lifecycle coordinator and lock are required |
| The switch targets only `gpt-5.6-sol`, `gpt-5.6-terra`, and `gpt-5.6-luna` | User requirement | Locked |
| Exactly `context_window` and `max_context_window` change to `372000` for each target; percent is preserved rather than rewritten | User requirement and correction | Locked; six numeric values change in total |
| AIO must not touch `models_cache.json`, the global model window, or automatic compaction thresholds | User requirement | Locked |
| The complete base is the canonical user's absolute `model_catalog_json` when present, otherwise installed Codex `debug models --bundled` | Existing managed-catalog lifecycle | Locked implementation interpretation |
| Turning the switch off restores only this overlay and preserves unrelated user edits and any managed-profile catalog policy | User requirement and safe merge-restore semantics | Locked |
| Neither toggling nor delivery verifies or claims that Codex actually uses 372K at runtime | Final user correction | Locked; real-use feedback remains with the user |
| Config hardening and 372K share config/catalog ownership and should not be separate worktrees | User decision after dependency review | Locked; one combined task and writer |

No material product question remains open. Implementation is authorized.

## Goal

First make direct Codex configuration the canonical semantic baseline and the proxy-facing file a deterministic, recoverable projection. Then add one authoritative `开启上下文 372K` switch that builds a complete owned model catalog, changes only the six requested values, and restores only its own overlay when disabled.

## Requirements

### R1. Establish canonical and live configuration layers

- Treat the direct, non-proxy Codex configuration as the source of truth for semantic user edits.
- Treat live proxy configuration as a derived overlay for AIO provider/auth/catalog values.
- Never refresh the direct baseline from already projected live bytes.
- Preserve unrelated TOML keys, comments, ordering, and supported external edits.
- Structured save, raw TOML save, and MCP sync all mutate canonical state before regenerating any live projection.

### R2. Use one recoverable Codex lifecycle transaction

- Serialize config saves, MCP sync, proxy enable/re-sync/disable, managed catalog changes, 372K preference changes, startup repair, and exit restore through one coordinator and documented lock order.
- Capture pre-images, compare before every write, and compare transaction ownership before rollback. External drift wins and returns a bounded recovery-required error.
- Record enough journal/manifest state to repair every partial multi-file phase before proxy startup/reprojection.
- Close the mutation gate when exit begins and restore direct semantics under the same coordinator.
- Keep atomic-write, bounded-read, symlink/reparse, backup-path, size, and sensitive-log protections.

### R3. Add one backend-owned 372K toggle

- Place `开启上下文 372K` in the existing CLI settings Codex section.
- The frontend submits only `enabled: bool`; the backend owns exact slugs, constants, paths, transformation, persistence, and side effects.
- Display authoritative persisted/manifest state, disable while saving, serialize repeated mutations, and reuse existing feedback behavior.
- The preference works whether or not CLI proxy or managed profiles are active.

### R4. Build and transform the current complete catalog

- Read the canonical user's safe absolute `model_catalog_json`; otherwise invoke installed Codex with structured `debug models --bundled` arguments.
- Parse JSON structurally and preserve every unknown root/model field, model order, and non-target model.
- Require exactly one structurally valid entry for each target before committing feature state.
- For each exact slug, set only `context_window` and `max_context_window` to `372000`.
- Never assign, normalize, delete, or synthesize `effective_context_window_percent`; a source fixture value of `95` remains `95` only because it is preserved.
- Never read, copy, edit, delete, or depend on `models_cache.json`; never change per-model or root auto-compaction values or global `model_context_window`.

### R5. Maintain one managed catalog owner and exact restore semantics

- Compose one complete catalog as base -> optional 372K transform -> optional managed-profile rows -> ownership metadata/hash.
- Generalize activation from “profiles exist” to “one or more catalog policies are active”; do not add a competing writer or generated path.
- Before enable, preserve the prior canonical catalog pointer or absence through the lifecycle transaction.
- Apply preference, generated catalog, config pointer, and owner manifest as one recoverable operation.
- Disable with profiles by rebuilding without the 372K transform; disable without other consumers by restoring the prior pointer/absence and removing only ownership-verified generated data.
- Include base identity and active policies in drift/ownership metadata; repeated transitions and restart are idempotent.

### R6. Do not perform or claim runtime verification

- Toggling does not call `refreshCodex`, launch a model request, scan for post-toggle proof, restart Codex, or show a `372000/372000/95` verification/effectiveness status.
- Automated tests prove only AIO's transformation, owned files, transaction, and state. They must not claim a Codex process adopted the values.
- Actual Codex behavior remains outside acceptance; the user will report real-use problems.

### R7. Keep contracts, bindings, and tests authoritative

- Update the Codex config contract with canonical/live/coordinator/drift/recovery semantics.
- Extend the managed model-catalog contract from profile-only activation to shared catalog policies and the exact six-value transform.
- Regenerate checked-in bindings through the repository's normal CI-owned workflow.
- Add deterministic temp-home, barrier, failpoint, restart, path, pure JSON, frontend, and cross-layer regression coverage.

## Non-Goals

- Do not modify Codex source, gateway request bodies/routing, provider selection policy, MCP payload semantics, model capabilities, pricing, or persistence outside this config/catalog lifecycle.
- Do not make all models use 372K, use global `model_context_window`, edit refreshable cache, or change automatic compression thresholds.
- Do not inspect a real user's Codex home in tests, launch validation requests, add runtime monitoring, or automatically restart/refresh Codex.
- Do not add a release, change package versions, or alter release configuration.

## Acceptance Criteria

- [ ] AC1: Direct config -> proxy enable -> structured save -> proxy disable/exit restores direct provider/auth/catalog/provider-table semantics and retains the semantic edit.
- [ ] AC2: Raw TOML save and Codex MCP sync use the same canonical-then-project flow; re-sync is idempotent and direct backup contains no AIO projection-only values.
- [ ] AC3: Barrier tests cover config vs MCP/proxy/catalog/372K/exit without lost updates, competing locks, or deadlock.
- [ ] AC4: Failure at each baseline, live, backup, catalog, preference, and manifest phase either commits the whole operation or rolls back only bytes still owned by that transaction.
- [ ] AC5: External drift is preserved with a recovery-required result; restart repairs every owned partial phase before proxy projection, and exit rejects new mutations after shutdown begins.
- [ ] AC6: The Codex settings page exposes `开启上下文 372K` with authoritative state, saving disablement, serialized mutations, and existing feedback behavior.
- [ ] AC7: Enabling from a complete fixture produces one complete owned catalog with exactly six target differences, all equal to `372000`.
- [ ] AC8: Target percent values, non-target models, unknown fields, ordering, all auto-compaction values, and root `model_auto_compact_token_limit` remain semantically unchanged.
- [ ] AC9: User-catalog and installed-bundled base paths are covered; a sentinel `models_cache.json` remains byte-for-byte untouched.
- [ ] AC10: Missing/duplicate/invalid/oversized/unsafe/drifted catalog input fails without partial preference, catalog, config, or manifest state.
- [ ] AC11: Disable restores the prior pointer/absence when no other policy exists and rebuilds one shared catalog without 372K when profiles remain.
- [ ] AC12: Enable/enable, disable/disable, enable/disable, restart, proxy on/off, and zero/one/multiple-profile combinations are idempotent and preserve unrelated config edits.
- [ ] AC13: The toggle path does not refresh, restart, execute a model request, scan for proof, change auto-compaction, or expose a runtime-effectiveness status.
- [ ] AC14: Temp-home/platform tests preserve comments and unknown keys, enforce supported path/size protections, emit no sensitive contents, and never access a real user home.
- [ ] AC15: Applicable specifications, generated bindings, service/query state, and implementation agree; evidence explicitly limits its claim to AIO artifacts and state.
- [ ] AC16: Fixed local verification passes against the registered full base SHA, and required GitHub checks pass on the exact final PR head.

## Stop Conditions

- The design would require a public API compatibility break, destructive settings migration, credential-handling change, weakened path protection, or unavoidable user-data loss.
- Any participating writer cannot enter the shared coordinator without changing its product semantics.
- The catalog cannot be structurally transformed while preserving unknown/non-target semantics or cannot share one owner with managed profiles.
- Implementation would need to touch `models_cache.json`, global model window, auto-compaction, real user config, or runtime model requests.
- Upstream drift invalidates the config backup, catalog command/schema, proxy, MCP, startup, or exit contracts assumed here.

## Scope and Decision Changes

| Date | Old / new decision | Affected acceptance criteria | Decision owner / resume condition |
|---|---|---|---|
| 2026-08-17 | Split the pre-existing config defect from the 372K feature during initial risk analysis | All | Superseded before implementation |
| 2026-08-17 | Remove all runtime-effectiveness verification; user real-use feedback is external | AC8, AC13, AC15 | Final user correction |
| 2026-08-17 | Consolidate transaction hardening and 372K into this task because they share owners, files, rollback, and final integration | AC1-AC16 | User decision; one Codex worktree and unique writer |
| 2026-08-17 | Authorize worktree creation, handoff, execution documentation, and implementation | AC1-AC16 | User confirmed no further requirements |

## PENDING Review

- `PENDING.md` was reviewed on 2026-08-17 and contains no unresolved entries.

## Notes

- The removed standalone hardening task had no worktree, branch, or implementation; its planning history remains in Git.
- File/state tests are required safety evidence but deliberately do not prove Codex runtime adoption.
