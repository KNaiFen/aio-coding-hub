# 增加 Codex 372K 上下文开关

## Plan Status

- Implementation authorization: pending explicit user confirmation
- Confirmation date and summary: 2026-08-17; the user supplied and corrected the requirement, but did not authorize implementation
- Confirmed coverage: UI switch, exact catalog transformation, backup/restore semantics, no-runtime-verification rule, dependency, and acceptance criteria recorded below
- Planning revision: pending planning commit
- Execution route: delegated Trellis worktree after the prerequisite is accepted and the user explicitly authorizes implementation
- Migrated from direct-main record: none; this is a new complex cross-layer task

## Material Facts, Assumptions, and Open Questions

| Item | Source | Status / closure condition |
|---|---|---|
| The switch targets only `gpt-5.6-sol`, `gpt-5.6-terra`, and `gpt-5.6-luna` | User requirement | Locked |
| Exactly `context_window` and `max_context_window` change to `372000` for each target; `effective_context_window_percent` is preserved, not rewritten | User requirement and correction | Locked; six numeric values change in total |
| AIO must not touch refreshable `models_cache.json` or automatic compaction thresholds | User requirement | Locked |
| The complete base catalog is the user's current absolute `model_catalog_json` when configured, otherwise the installed Codex `debug models --bundled` result | Existing managed-catalog lifecycle | Locked implementation interpretation |
| The switch is an AIO preference and must work independently of whether CLI proxy is enabled | CLI settings semantics | Locked implementation interpretation |
| Turning the switch off restores only this feature's catalog/config overlay and preserves unrelated edits | User requirement plus safe merge-restore semantics | Locked |
| Neither toggling nor development acceptance will verify or claim that Codex actually uses 372K at runtime | User correction, 2026-08-17 | Locked; real-use feedback remains with the user |
| Current config writes can contaminate the proxy's direct backup | Code audit | Blocking dependency: `08-17-codex-config-transaction-hardening` must be accepted first |

No material product question remains open. Implementation remains blocked on the prerequisite and explicit user authorization.

## Goal

Add an authoritative `开启上下文 372K` switch to AIO's CLI settings Codex section. When enabled, AIO builds one complete owned model catalog from the current complete source and changes only the six requested context-window values; when disabled, it removes only that overlay and returns the catalog binding to its prior/default state.

## Requirements

### R1. Add one authoritative toggle

- Place a switch labeled `开启上下文 372K` in the existing Codex settings/features area.
- The backend owns the target slugs, `372000` constant, transformation, persistence, and side effects. The frontend submits only the requested boolean state.
- Disable the control while saving, serialize repeated toggles, and use existing success/error feedback patterns.
- The displayed state comes from AIO-owned preference/manifest state, not solely from a mutable `config.toml` pointer.

### R2. Build from the current complete catalog

- Use the current user-selected absolute `model_catalog_json` as the base when present; otherwise obtain the complete catalog from the installed Codex executable's bundled-model command.
- Parse and transform JSON structurally. Preserve all unknown top-level fields, all unknown model fields, model order, and every non-target model.
- Match the three exact slugs only. Missing, duplicate, or structurally ambiguous targets fail before any feature state is committed.
- Never read, copy, edit, delete, or depend on `models_cache.json`.

### R3. Change exactly six values

- For each exact target slug, set `context_window` to `372000` and `max_context_window` to `372000`.
- Do not write `effective_context_window_percent`; preserve its source value unchanged. Test fixtures use `95` to guard that preservation, but this is not a runtime-effectiveness check.
- Do not change per-model auto-compaction fields or root `model_auto_compact_token_limit`.
- Do not use the global root `model_context_window`, because that would affect models outside the requested set.

### R4. Share one managed catalog owner

- Generate one complete AIO-owned catalog and point `config.toml` to it through `model_catalog_json`.
- Extend the existing managed-catalog ownership/hash/generation lifecycle; do not create a second independent writer or competing generated catalog.
- Compose the catalog deterministically as base catalog -> optional 372K transform -> optional managed-profile rows.
- Include the 372K preference and base identity in ownership/drift metadata so profile changes, proxy re-sync, and restarts rebuild the same intended output.

### R5. Back up and restore safely

- Before first enable, preserve the prior `model_catalog_json` binding or its absence and the relevant original config baseline through the shared Codex transaction manifest.
- Apply generated catalog, config pointer, preference, and ownership metadata as one recoverable transaction.
- On disable with no other managed-catalog consumer, restore the prior pointer or remove the AIO-added key and safely remove only AIO-owned generated data.
- On disable while managed profiles still need the catalog, rebuild the same single owned catalog without the 372K transform and retain the profile projection.
- Preserve unrelated config edits made while the switch is enabled. External drift fails closed rather than being overwritten.

### R6. Do not perform or claim runtime verification

- Toggling does not call `refreshCodex`, scan the catalog again for proof, launch a model request, or display a `372000/372000/95` verification status.
- Automated tests verify only AIO's transformation, ownership, and config-file output. They must not describe those assertions as proof that a Codex process adopted the values.
- AIO does not add a runtime-effectiveness notice or restart Codex automatically.
- Actual Codex behavior is outside the acceptance claim; the user will report real-use problems.

### R7. Keep contracts and bindings synchronized

- Extend the managed model-catalog contract from a profile-only trigger to a shared catalog-policy lifecycle.
- Define the preference/command/state contract and regenerate checked-in frontend bindings through the repository's normal CI-owned workflow.
- Preserve existing model refresh, profile, proxy, and ordinary Codex settings behavior.

## Non-Goals

- Do not modify Codex source code, Codex's refreshable cache, automatic compression thresholds, API model capabilities, or gateway request bodies.
- Do not make all models use 372K and do not use the global `model_context_window` shortcut.
- Do not prove runtime effectiveness, inspect a real user's installed catalog during tests, launch validation requests, or add ongoing health/status monitoring.
- Do not automatically restart a Codex process, add an effectiveness notice, or refresh the model catalog after toggling.
- Do not add a release, change package versions, or alter release configuration.

## Acceptance Criteria

- [ ] AC1: The Codex settings page exposes one `开启上下文 372K` switch with authoritative on/off state, saving disablement, serialized mutations, and existing feedback behavior.
- [ ] AC2: Enabling from a complete fixture catalog produces a complete owned JSON catalog in which only the six target `context_window`/`max_context_window` values differ and equal `372000`.
- [ ] AC3: `effective_context_window_percent` remains byte-semantic/JSON-value equivalent to each target's source value; a fixture value of `95` remains `95` without being written by the transform.
- [ ] AC4: Every non-target model, unknown field, root field, model ordering, per-model auto-compaction value, and root `model_auto_compact_token_limit` remains semantically unchanged.
- [ ] AC5: `config.toml` points to the one AIO-owned complete catalog through `model_catalog_json`, and a sentinel `models_cache.json` remains byte-for-byte untouched.
- [ ] AC6: User absolute-catalog and installed-bundled-catalog base paths are covered; missing, duplicate, invalid, oversized, unsafe-path, or externally drifted inputs fail without partial state.
- [ ] AC7: Disable restores the prior catalog pointer or absence when no other consumer exists, and rebuilds the shared catalog without the 372K transform when managed profiles remain active.
- [ ] AC8: Enable/enable, disable/disable, enable/disable, restart, proxy on/off, and zero/one/multiple managed-profile combinations are idempotent and preserve unrelated config edits.
- [ ] AC9: A generated-catalog write, config-pointer write, preference write, or manifest failure commits nothing or performs ownership-safe compensation under the shared Codex transaction.
- [ ] AC10: The toggle path does not invoke `refreshCodex`, execute a model request, scan for post-toggle proof, modify auto-compaction settings, or expose a runtime-verification status.
- [ ] AC11: Tests and delivery evidence explicitly limit their claim to AIO-generated files and state; actual Codex runtime adoption is not an acceptance assertion.
- [ ] AC12: Applicable specifications and generated bindings match the implementation, fixed local verification passes against the registered base SHA, and required GitHub checks pass on the final PR head.

## Stop Conditions

- `08-17-codex-config-transaction-hardening` is not yet accepted and merged.
- The installed Codex catalog schema cannot be transformed while preserving unknown fields and exact non-target semantics.
- Implementation would need to edit `models_cache.json`, change auto-compaction behavior, use a global model window, or introduce a competing catalog owner.
- A public API compatibility break, destructive settings migration, credential handling change, or weakened path protection becomes necessary.
- Upstream drift changes the Codex catalog command, ownership contract, config path, or proxy lifecycle assumed here.

## Scope and Decision Changes

| Date | Old / new decision | Affected acceptance criteria | Decision owner / resume condition |
|---|---|---|---|
| 2026-08-17 | Split the new Codex requirement from the TUI task and add a configuration-transaction prerequisite | All | Code audit; prerequisite must be accepted first |
| 2026-08-17 | Initial wording requested verification of `372000/372000/95`; final wording removes all runtime-effectiveness verification and leaves real-use feedback to the user | AC3, AC10-AC11 | Latest user correction is authoritative |
| 2026-08-17 | Use one shared owned catalog instead of global `model_context_window` or another catalog writer | AC2-AC9 | Existing managed-catalog contract and exact three-model scope |

## PENDING Review

- `PENDING.md` was reviewed on 2026-08-17 and contains no unresolved entries.

## Notes

- Public OpenAI model documentation confirms the named GPT-5.6 model family, but the local `model_catalog_json` lifecycle is a repository/installed-client contract rather than a documented public API.
- File-output tests are necessary safety checks, but they intentionally make no claim about an already running or future Codex process using the generated values.
