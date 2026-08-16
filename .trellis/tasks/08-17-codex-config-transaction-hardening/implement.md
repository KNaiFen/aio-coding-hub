# Codex 配置事务与代理恢复实施计划

## Preconditions

1. Obtain explicit implementation authorization for this prerequisite task.
2. Commit the complete planning materials and record the full planning commit SHA.
3. Fetch `origin`, confirm a clean synchronized main checkout, create a dedicated task worktree from the full `origin/main` SHA, and register it through `task.py delegate`.
4. Add `execution.md`, run `task.py start`, commit the coordination transition, verify `task.py doctor`, and generate the handoff.
5. Keep `08-17-codex-372k-context-window` in planning until this task is accepted and merged.

Do not perform implementation before all preconditions pass.

## Work Package 1: Freeze Contracts And Reproduce The Defect

- Update the config and managed-catalog contracts with canonical/live/manifest terminology and ownership rules.
- Add temp-home regression fixtures for direct config -> proxy enable -> structured/raw/MCP save -> disable/exit.
- Add a failing assertion that the direct backup never captures AIO projection-only values.
- Add deterministic failpoints and barrier helpers before refactoring production flows.

Exit condition: the backup contamination and multi-writer races are represented by precise failing tests without touching a real user home.

## Work Package 2: Add The Lifecycle Coordinator

- Introduce the shared coordinator, outer lock, mutation gate, canonical-state reader, projection builder, and bounded error taxonomy.
- Define transaction pre-images, owned writes, generation/hash metadata, and journal phases.
- Reuse atomic writes and shared path protections; add missing ancestor symlink/reparse checks.
- Add pure tests for canonical mutation, deterministic projection, drift, and ownership-aware rollback.

Exit condition: one private API can plan and apply canonical/live transactions with no product-specific caller conversion yet.

## Work Package 3: Convert Every Config Writer

- Route structured config and raw TOML saves through canonical mutation plus live reprojection.
- Route Codex MCP sync through the same coordinator.
- Route proxy enable, re-sync, disable, and managed-catalog changes through the same coordinator and lock order.
- Remove or encapsulate direct write paths that can bypass the transaction boundary.
- Preserve command payloads and user-facing semantics.

Exit condition: no in-scope writer can independently overwrite `config.toml`, direct backup, managed catalog, or lifecycle manifest.

## Work Package 4: Startup And Exit Recovery

- Implement journal-phase startup repair before proxy startup/reprojection.
- Close the mutation gate at exit, wait for or reject in-flight work, and restore the direct projection under the coordinator.
- Preserve external drift and surface recovery-required state instead of guessing.
- Cover ordinary exit, interrupted exit, proxy startup failure, and every simulated crash phase.

Exit condition: restart and shutdown cannot leave an owned half-applied proxy projection.

## Work Package 5: Concurrency And Failure Regression

- Run barrier tests for config vs MCP, config vs proxy, config vs catalog, and config vs exit.
- Inject failures at each owned write and manifest transition.
- Verify comment/key preservation, idempotent re-sync, custom homes, platform path rules, and no sensitive logs.
- Review all rollback branches for compare-before-restore ownership.

Exit condition: AC1-AC9 have deterministic automated evidence.

## Work Package 6: Delivery

- Review the diff against requirements, non-goals, stop conditions, and both updated contracts.
- Run `$gkd-local-verify` with the registered full base SHA only; do not run ad hoc dependency, Cargo, frontend, or build commands locally.
- Write `delivery.md` with AC-by-AC evidence and any justified deviation.
- Commit, deliver, push the task branch, and open or update the PR.
- Wait for required checks on the exact final head through `$gkd-ci-monitor`, then pause for `$gkd-accept`/main fixed-head acceptance.

Exit condition: delivery evidence and required CI bind to the same clean full PR head SHA.

## Dependency And Merge Notes

- This task must be accepted before `08-17-codex-372k-context-window` starts because both own Codex config/catalog lifecycle.
- It has no primary file overlap with `08-17-tui-observability-consistency`; those tasks may be scheduled independently after separate authorization.
- Use one execution writer for all Codex transaction modules because splitting them would violate the single-writer and lock-order design.
