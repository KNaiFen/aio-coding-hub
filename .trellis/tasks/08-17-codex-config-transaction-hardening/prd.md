# 硬化 Codex 配置事务与代理恢复

## Plan Status

- Implementation authorization: pending explicit user confirmation
- Confirmation date and summary: 2026-08-17; the user requested an implementation-ready plan, not implementation
- Confirmed coverage: the prerequisite defect, transaction boundary, recovery semantics, acceptance criteria, and dependency on the 372K task recorded below
- Planning revision: pending planning commit
- Execution route: delegated Trellis worktree after explicit authorization
- Migrated from direct-main record: none; this is a new complex prerequisite task

## Material Facts, Assumptions, and Open Questions

| Item | Source | Status / closure condition |
|---|---|---|
| `codex_config_set` and raw TOML save derive their next bytes from the live config and copy those bytes into the direct-config backup while the proxy is enabled | `infra/codex_config/mod.rs` and `infra/cli_proxy/codex.rs` | Confirmed defect; this task must merge before the 372K switch starts |
| Disabling or exiting the proxy restores provider, auth, catalog, and AIO-provider keys from that backup | `infra/cli_proxy/codex.rs` | Confirmed; a contaminated backup can leave a stopped localhost proxy configured |
| Structured config save, raw save, MCP sync, proxy lifecycle, managed catalog, and exit cleanup can all write Codex state without one shared transaction boundary | Current Rust modules and service contracts | Confirmed; one Codex lifecycle coordinator and lock are required |
| Semantic user changes made while the proxy is active must survive proxy disable/exit, while proxy-only projection must not | Existing merge-restore intent and user-facing behavior | Locked requirement |
| The dependent 372K task needs one catalog/config owner rather than another independent writer | Managed catalog contract and 372K requirement | Confirmed; Task `08-17-codex-372k-context-window` remains blocked until this task is accepted |

No material product decision remains open. Implementation still requires explicit user authorization.

## Goal

Make direct Codex configuration the canonical semantic baseline and make the proxy-facing live configuration a deterministic projection, so configuration, MCP, proxy, catalog, and exit operations cannot contaminate backups, lose concurrent edits, or leave a partially applied state.

## Requirements

### R1. Separate canonical and live configuration

- Treat the direct, non-proxy Codex configuration as the source of truth for semantic user edits.
- Treat the live proxy configuration as a derived projection that may add AIO provider, provider selection, auth, and managed-catalog bindings.
- Never persist proxy-only projection back into the direct baseline unless those values were genuinely present in the user's direct configuration.
- Preserve unrelated TOML keys, comments, ordering, and supported external edits.

### R2. Use one Codex lifecycle transaction boundary

- Serialize structured config writes, raw TOML writes, Codex MCP sync, proxy enable/re-sync/disable, managed model-catalog changes, and exit restoration through one process-wide coordinator.
- Define and document one lock order. No participating module may acquire a second independent lock in the reverse order.
- Every transaction captures the expected pre-image of each owned file, checks for drift before writing, and checks ownership before rollback.
- External drift wins: return a bounded recovery-required error instead of overwriting a later external edit.

### R3. Project and restore deterministically

- Apply a semantic mutation to the canonical baseline first, then regenerate the live proxy projection from that baseline when the proxy is enabled.
- On proxy disable or application exit, restore direct provider, auth, catalog, and provider-table semantics while retaining user edits made during the proxy session.
- Raw TOML save and MCP sync follow the same canonical-then-project flow as structured config save.
- Proxy re-sync must be idempotent and must not accumulate or duplicate projection keys.

### R4. Recover partial multi-file operations

- Record enough transaction state to distinguish not-started, baseline-written, projection-written, and manifest-committed phases.
- Use atomic same-directory file replacement and bounded reads for every owned file.
- Startup repairs or rolls back an interrupted Codex transaction before starting or re-projecting the proxy.
- Exit prevents new Codex writes after shutdown begins and waits for or safely rejects an in-flight transaction within the existing lifecycle budget.

### R5. Preserve security and privacy boundaries

- Keep existing path, symlink/reparse-point, backup-relative-path, and file-size protections.
- Do not log raw config, catalog contents, credentials, tokens, or full environment values.
- Tests use temporary Codex homes and deterministic failpoints; they never inspect or mutate a real user configuration.

### R6. Update contracts and regression coverage

- Update the Codex config contract to define canonical baseline, live projection, shared coordinator, drift, rollback, startup repair, and exit behavior.
- Update the managed model-route contract where its catalog lifecycle participates in the shared transaction.
- Add deterministic concurrency and failure-injection coverage for every participating writer.

## Non-Goals

- Do not add the 372K UI toggle or change any model catalog values in this task.
- Do not change gateway routing, provider selection policy, MCP payload semantics, model auto-compaction thresholds, or Codex CLI behavior.
- Do not add a release, change package versions, or alter release configuration.
- Do not replace TOML parsing with textual search/replace or broaden logging of user configuration.

## Acceptance Criteria

- [ ] AC1: Direct config -> proxy enable -> structured config save -> proxy disable restores direct provider/auth/catalog/provider-table semantics and retains the semantic edit.
- [ ] AC2: The same lifecycle succeeds for raw TOML save and Codex MCP sync while the proxy is enabled.
- [ ] AC3: Proxy re-sync is idempotent, and the direct baseline never contains projection-only AIO provider or stopped-localhost routing introduced by AIO.
- [ ] AC4: Deterministic barrier tests cover config vs MCP, config vs proxy, config vs catalog, and config vs exit without lost updates or deadlock.
- [ ] AC5: Failure injection after each baseline, live-config, backup, generated-catalog, and manifest write either commits the whole transaction or restores only bytes still owned by that transaction.
- [ ] AC6: If an external editor changes an owned file before write or rollback, AIO preserves the external bytes and returns a recovery-required error.
- [ ] AC7: Restart tests recover every recorded partial-transaction phase before proxy projection begins.
- [ ] AC8: Shutdown rejects new Codex mutations after exit begins and leaves no proxy-only live configuration after successful cleanup.
- [ ] AC9: Temp-home tests preserve unrelated TOML comments and keys and cover default, followed, and custom Codex homes plus supported platform path protections.
- [ ] AC10: Applicable cross-layer specifications match the implementation, fixed local verification passes against the registered base SHA, and required GitHub checks pass on the final PR head.

## Stop Conditions

- The solution requires a public API compatibility break, settings migration with user-data loss risk, credential handling change, or weakened path protection.
- A participating writer cannot be routed through the shared coordinator without changing its product semantics.
- Upstream drift changes the proxy backup, managed catalog, MCP sync, startup, or exit contracts assumed by this plan.
- The dependent 372K task starts writing the same files before this prerequisite is accepted.

## Scope and Decision Changes

| Date | Old / new decision | Affected acceptance criteria | Decision owner / resume condition |
|---|---|---|---|
| 2026-08-17 | Add a prerequisite task after confirming that live proxy bytes can contaminate the direct-config backup | AC1-AC10 | Repository evidence; user authorization is still required before implementation |
| 2026-08-17 | Keep transaction hardening separate from the TUI task and make the 372K task depend on it | All | Locked planning decision; revise only if implementation evidence disproves the overlap |

## PENDING Review

- `PENDING.md` was reviewed on 2026-08-17 and contains no unresolved entries.

## Notes

- A previous canonical/live implementation existed before commit `93a08f15`; it is historical design evidence, not a wholesale revert target.
- This task fixes a pre-existing configuration-safety defect even without the 372K feature.
