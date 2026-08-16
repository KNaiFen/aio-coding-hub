# Codex 372K 上下文开关实施计划

## Preconditions

1. `08-17-codex-config-transaction-hardening` is accepted, merged, archived, and reflected in current specifications.
2. Obtain explicit user authorization for this feature task.
3. Re-read current Codex settings, managed-catalog, proxy, and transaction contracts for drift.
4. Commit the complete planning materials and record the full planning commit SHA.
5. Fetch `origin`, confirm a clean synchronized main checkout, create a dedicated task worktree from the full `origin/main` SHA, and register it through `task.py delegate`.
6. Add `execution.md`, run `task.py start`, commit the coordination transition, verify `task.py doctor`, and generate the handoff.

Do not perform implementation before all preconditions pass.

## Work Package 1: Freeze Contracts And Fixtures

- Extend the managed-catalog contract from profile-only activation to shared catalog policies.
- Define the dedicated boolean preference/command, ownership metadata, enable/disable semantics, and no-runtime-verification boundary.
- Add complete-catalog fixtures containing all three exact targets, non-target models, unknown fields, ordering, percent `95`, and auto-compaction fields.
- Add failing deep-diff expectations for exactly six transformed values and zero cache-file writes.

Exit condition: AC2-AC6 and the runtime-claim boundary are represented by precise fixtures before production behavior changes.

## Work Package 2: Extend The Catalog Composer

- Add the pure exact-slug 372K transform.
- Preserve all unknown and non-target values, including percent and auto-compaction fields.
- Generalize catalog activation from “profiles exist” to “one or more managed policies are active”.
- Extend owner generation/hash metadata to include base identity, 372K preference, and profile policy.
- Cover missing, duplicate, invalid, oversized, unsafe, and drifted inputs.

Exit condition: a deterministic complete catalog can be planned for every zero/one/multiple-profile and switch-state combination without writing files.

## Work Package 3: Add Transactional Preference And Lifecycle

- Persist the backend-owned boolean preference and expose a dedicated query/mutation contract.
- Apply preference, generated catalog, config pointer, and owner manifest through the shared prerequisite transaction.
- Implement exact prior-pointer/absence restoration and shared-catalog rebuild on disable.
- Cover idempotence, restart, proxy on/off, write failpoints, external drift, and ownership-safe compensation.

Exit condition: AC5-AC9 have temp-home integration evidence and no path touches a real Codex home or `models_cache.json`.

## Work Package 4: Add The Codex Settings Switch

- Add `开启上下文 372K` to the existing Codex settings/features area.
- Bind it to authoritative backend state and the dedicated boolean mutation.
- Reuse saving disablement, serialization, toast, and bounded error presentation.
- Do not add a runtime-effectiveness notice or automatic restart.
- Assert that toggling does not call `refreshCodex` or a model request.

Exit condition: AC1 and AC10 are covered by focused frontend/service/query tests.

## Work Package 5: Regression And Claim Review

- Run the full planned matrix in CI: both base sources, switch cycles, profile counts, proxy states, restart, drift, platform paths, and failure injection.
- Deep-compare non-target JSON and assert the cache sentinel and all compaction settings are unchanged.
- Review UI copy, tests, specs, and delivery evidence for any wording that falsely claims Codex runtime adoption.
- Confirm ordinary model refresh, profile management, proxy enable/disable, and other Codex settings retain existing behavior.

Exit condition: AC2-AC11 have artifact/state evidence with no runtime-effectiveness claim.

## Work Package 6: Delivery

- Review the final diff against every requirement, non-goal, stop condition, and updated specification.
- Run `$gkd-local-verify` with the registered full base SHA only; do not run ad hoc dependency, Cargo, frontend, test, build, or binding-generation commands locally.
- Write `delivery.md` with AC-by-AC evidence and explicitly state that actual Codex use of 372K was not verified.
- Commit, deliver, push the task branch, and open or update the PR.
- Wait for required checks on the exact final head through `$gkd-ci-monitor`, then pause for `$gkd-accept`/main fixed-head acceptance.

Exit condition: delivery evidence and required CI bind to the same clean full PR head SHA, with runtime behavior left to user real-use feedback.

## Dependency And Merge Notes

- This task must start only after the config-transaction prerequisite is accepted; do not develop the two tasks concurrently in separate writers.
- It has no primary file overlap with `08-17-tui-observability-consistency`; the TUI task remains an independently authorizable delivery.
- Use one execution writer across settings, catalog, config, and frontend surfaces because the feature's rollback and generated bindings cross those boundaries.
