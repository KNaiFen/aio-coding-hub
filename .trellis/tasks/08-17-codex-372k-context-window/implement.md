# Codex 配置生命周期与 372K 开关实施计划

## Preconditions

1. Record the user-approved consolidation, complete scope, and implementation authorization.
2. Commit the complete planning and execution materials and record the full planning SHA.
3. Fetch `origin`, confirm a clean main checkout with current `origin/main`, create one dedicated Codex task worktree from the full base SHA, and register it through `task.py delegate`.
4. Run `task.py start`, commit the coordination transition, verify `task.py doctor`, and generate the handoff.

Do not implement before these preconditions pass.

## Work Package 1: Freeze Contracts And Reproduce Safety Defects

- Update both Codex cross-layer contracts with canonical/live lifecycle, shared catalog policies, exact transform, restore, and no-runtime-verification semantics.
- Add temp-home fixtures reproducing live-projection contamination of the direct backup for structured save, raw save, and MCP sync.
- Add complete catalog fixtures with three targets, non-target models, unknown fields, percent `95`, and auto-compaction fields.
- Add deterministic failpoint/barrier infrastructure before production refactoring.

Exit condition: the existing defect and all intended file/state semantics have precise failing tests.

## Work Package 2: Implement The Lifecycle Coordinator

- Add canonical reader/mutator, deterministic live projection, outer lock, exit gate, journal generation/phases, bounded error taxonomy, and ownership-aware compensation.
- Preserve current atomic writes and strengthen missing path/symlink/reparse checks.
- Route structured/raw config save and Codex MCP sync through canonical-then-project transactions.
- Add pure, drift, rollback, and idempotence tests.

Exit condition: no ordinary config writer refreshes canonical state from projected live bytes.

## Work Package 3: Add The Catalog Policy And Preference

- Add the pure exact-slug transform and exactly-six-difference tests.
- Generalize one managed catalog owner/activation predicate to 372K and profile policies.
- Persist the backend-owned boolean and expose dedicated authoritative query/mutation contracts.
- Include base identity, active policies, output hash, original pointer, and transaction generation in ownership state.
- Cover user/bundled bases, invalid input, cache sentinel, pointer restore, profiles, proxy states, restart, and repeated transitions.

Exit condition: catalog/preference/config/manifest can be planned and applied as one transaction for every supported combination.

## Work Package 4: Convert Remaining Lifecycle Writers And Recovery

- Route proxy enable/re-sync/disable, managed-profile catalog rebuilds, startup repair, and exit restore through the coordinator and fixed lock order.
- Implement every journal recovery phase and reject new mutation after exit starts.
- Add barrier/failpoint coverage for config vs MCP/proxy/catalog/372K/exit and external edits before write/rollback.
- Remove or encapsulate direct file writes that bypass ownership checks.

Exit condition: AC1-AC5 and AC10-AC12 have deterministic temp-home evidence.

## Work Package 5: Add The Codex Settings Switch

- Add `开启上下文 372K` to the existing Codex settings/features area.
- Bind authoritative state and boolean mutation through service/query/page layers and generated bindings.
- Reuse pending disablement, serialization, toast, and bounded error behavior.
- Do not add refresh, restart, validation request, rescan, effectiveness notice, or runtime status.
- Cover AC6 and AC13 with focused frontend/service/query tests.

Exit condition: UI and backend state agree without claiming runtime adoption.

## Work Package 6: Full Regression And Safety Review

- Execute the planned CI matrix across base sources, switch cycles, profile counts, proxy states, config writers, restart, drift, failure phases, and platform paths.
- Deep-compare non-target catalog values and assert percent/cache/auto-compaction preservation.
- Review lock order, compare-before-rollback, startup/exit, path safety, and logging for every caller.
- Review UI/spec/tests/delivery wording for any false runtime-effectiveness claim.

Exit condition: AC1-AC15 have file/state evidence and no scope or security regression.

## Work Package 7: Delivery

- Review the final diff against every requirement, non-goal, stop condition, and updated contract.
- Run `$gkd-local-verify` with the registered full base SHA only; do not run ad hoc dependency, Cargo, frontend, test, build, or binding-generation commands locally.
- Write `delivery.md` with AC-by-AC evidence and explicitly state that actual Codex use of 372K was not verified.
- Commit implementation/evidence, run `task.py deliver`, push the task branch, and open or update the PR.
- Wait for required checks on the exact final head through `$gkd-ci-monitor`, then pause for `$gkd-accept`/main fixed-head acceptance.

Exit condition: one clean final head contains the safety foundation and feature, and all required CI/evidence bind to that exact SHA.

## Dependency And Merge Notes

- This single task owns all Codex config/catalog/372K changes; there is no separate transaction worktree to merge or synchronize.
- `08-17-tui-observability-consistency` has no primary file or semantic overlap and may run in parallel.
- Keep one unique writer for all backend/frontend/spec changes in this task because transaction ownership and generated bindings cross module boundaries.
