# AIO 遗留 GKD 迁移映射

## 判断准则

- **bundle 替代**：通用任务状态、角色路由、固定 head 监控和验收机制，后续仅由固定、已验证的 GKD bundle 提供。
- **AIO policy/adapter**：依赖 AIO 仓库、产品栈、GitHub 设置或发布方式的规则，必须保留在 AIO，不得固化进通用 bundle。
- **证据后删除**：只有替代路径的 adapter smoke、fixture、真实 canary 与固定 head 验收完整后，才可以删除旧实现或提示词。
- **历史只读**：用于解释已完成任务的材料；不重写 Git 历史，也不迁移为可执行合同。

## 映射

| Current path / concern | Current responsibility | Target ownership | Follow-up milestone |
|---|---|---|---|
| `AGENTS.md` and `.trellis/workflow.md` | Role names, main/executor/accept boundaries and lifecycle guidance | AIO policy/adapter retains concise hard boundaries; generic role procedures become bundle references | B, then E removes superseded prose |
| Accepted bundle `bin/gkd-role` | Runtime entrypoint for bundle/project verification and automatic route gates | GKD generic runtime; it must run on the supported host interpreter before any AIO adapter can rely on it | Pre-B GKD runtime repair |
| `.trellis/scripts/task.py` | Legacy task CLI command routing and persisted coordination entry points | GKD bundle task core replaces generic task lifecycle; AIO adapter supplies repository policy inputs | B and C |
| `.trellis/scripts/common/task_coordination.py` | Legacy writer, worktree, base and handoff state | Bundle task core plus machine-local attachments; no hand-edited task JSON migration | C |
| `.trellis/scripts/common/task_acceptance.py` | Legacy fixed-head verification and synchronous squash merge | Bundle acceptance core; AIO adapter provides repository identity and required checks | B and C |
| `.trellis/scripts/tests/test_task_coordination.py` and `test_task_acceptance.py` | Legacy coordination and acceptance contract tests | Replace only after equivalent bundle/adapter fixture coverage is accepted | C and E |
| `.trellis/agents/implement.md` and `.trellis/agents/check.md` | Trellis-channel prompts for legacy implementation/check flows | Historical compatibility until the new handoff route has canary evidence; then remove only the superseded prompts | E |
| `scripts/check-local-verification.mjs` | AIO's Air-safe, dependency-free local verification entry point; it differs from the currently installed generic verifier entry | AIO policy/adapter retained; B must add pinned bundle dispatch without broad local builds | B and D |
| `scripts/check-cloud-only-verification.mjs` and `.selftest.mjs` | Enforces AIO's cloud-only boundary and currently checks exact `gkd-*` wording | AIO policy/adapter retained and updated together with its selftest; it must not encode generic bundle internals | B and D |
| `scripts/require-github-actions.mjs` and CI quality-gate scripts | AIO check selection, required-check expectations and runner policy | AIO policy/adapter retained | B and D |
| `.github/workflows/ci.yml` | AIO's dependency, frontend, Rust and quality workflows | AIO policy/adapter retained; connect fixed-head monitoring without changing product check ownership | D |
| `.github/workflows/release.yml` and release scripts | AIO version, tag, release and artifact contract | AIO policy/adapter retained; bundle provides generic task finalization only | D |
| `.trellis/spec/`, product documentation and cross-layer contracts | AIO product behavior and repository conventions | AIO policy/adapter retained | B through D as affected |
| `.trellis/tasks/archive/` and `docs/history/change-records/` | Historical decisions, delivery and acceptance evidence | Historical read-only; no rewrite and no attempt to make deleted worktrees live | C and E |

## Sequencing and Dependencies

1. **A, this task** records the boundary above without changing executable behavior.
2. **B** pins a released, independently verified bundle and introduces the smallest AIO repo/policy/origin adapters. It must prove three-way agreement before any legacy state changes.
3. **C** migrates active and archived task state through supported bundle commands and fixture evidence. It cannot hand-edit legacy machine JSON or depend on deleted worktrees.
4. **D** connects AIO local verification, fixed-head CI monitor, CI quality and release rules through its adapter. It must keep normal product PRs from running the full GKD suite.
5. **E** runs a low-risk manual canary and an automatic-route canary under the user authorization recorded in `prd.md`, but only after its complete mechanical route gates pass. Legacy code and prompts are deleted only after both paths have complete evidence.

## Explicit Non-Goals

- No GKD canonical source change or AIO-local fork of generic GKD logic.
- No production `~/.codex` change, Secrets, paid runners, GitHub setting change, tag, Release, deployment or product refactor.
- No alteration, staging or interpretation as machine state of the user-owned untracked `08-17-gkd-workflow-remediation` directory.
