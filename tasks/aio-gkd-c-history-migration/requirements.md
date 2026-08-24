# AIO GKD Historical Task Adapter

## Goal

Complete adoption milestone C by adding a read-only AIO history adapter that locates tracked active Trellis task manifests, validates archived manifests without requiring their former worktrees, and keeps all new execution on the installed canonical GKD lifecycle.

## User Decisions

- Continue automatically after the accepted B4 closeout using the published and verified GKD `v0.1.5` bundle.
- Preserve the existing Trellis task archive as historical evidence; do not rewrite archived task manifests or Git history.
- The original AIO checkout's untracked `.trellis/tasks/08-17-gkd-workflow-remediation/` directory is outside scope and must never be read as tracked project state, modified, deleted, or committed.
- The fixed AIO bundle pin, generic GKD policy, review/resource adapters, project policy, workflows, GitHub settings, Secrets, runners, tags, Releases, and product behavior remain unchanged.

## Scope

- Add one canonical AIO-owned `.gkd/history-adapter.json` that declares the tracked Trellis active/archive roots, single-active locator rule, active tracked-worktree prohibition, completed-archive rule, and archived-worktree ignore rule.
- Add a zero-dependency history checker that reads only `git ls-files` results, requires exactly one tracked active Trellis manifest, rejects active legacy coordination or a committed worktree path, and validates every tracked archive manifest without resolving its historical worktree path.
- Add deterministic selftests covering the current success shape, repeat-run stability, zero and multiple active manifests, active absolute-path/legacy-coordination rejection, malformed archive rejection, and archived missing-worktree acceptance.
- Bind the new declaration and checker into the existing AIO adapter validator and versioned local verifier, with exact-field and drift tests.
- Update the AIO GKD operations document and root fact inventory with the project-specific history boundary and the current tracked inventory result.
- Deliver one fixed-head AIO PR through the installed GKD `v0.1.5` automatic route; independent acceptance and merge occur only after local verification and policy-required CI succeed.

## Non-Goals

- Do not pass a Trellis manifest to `gkd-task migrate-v1`; that command remains reserved for its canonical GKD legacy schema.
- Do not create a second task lifecycle, locator runtime, offer/claim implementation, acceptance implementation, CI monitor, review engine, or release engine in AIO.
- Do not modify `.trellis/scripts/common/task_coordination.py`, `.trellis/scripts/common/task_acceptance.py`, any tracked `.trellis/tasks/**` file, the protected untracked planning directory, or historical commits.
- Do not implement milestone D CI/release changes or milestone E legacy-entrypoint removal in this task.
- Do not install dependencies, build products, change production `~/.codex`, alter GitHub settings or Secrets, use paid runners, create tags/Releases, or deploy.

## Acceptance Criteria

- [ ] `.gkd/history-adapter.json` is canonical JSON, contains exact public project facts only, and is strictly validated with unknown fields rejected.
- [ ] The checker scans only tracked manifests, reports one active and 107 archived tasks on the fixed base, and does not treat the original checkout's untracked plan as project state.
- [ ] An active manifest with a committed worktree path or Trellis coordination v1 fails closed; zero or multiple active manifests also fail closed.
- [ ] Archived completed fixtures with stale Unix, Windows, relative, or missing worktree paths pass without filesystem resolution; malformed or non-completed archives fail.
- [ ] Repeated checks on the same fixture return the same result and make no writes; all tracked Trellis task files remain byte-identical to the base.
- [ ] The canonical C task itself completes clean-main bootstrap, portable locator, automatic offer/claim, exact executor delivery, independent fixed-head acceptance, and records-only closeout using the verified `v0.1.5` runtime.
- [ ] `scripts/gkd-verify --base-sha 3f856c88749f4875889164fa72caeebc22143d98`, `git diff --check`, and fixed-head `ci-gate`/`pr-title` pass before merge.
