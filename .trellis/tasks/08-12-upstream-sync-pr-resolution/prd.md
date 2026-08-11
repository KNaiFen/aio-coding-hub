# 修复 Sync Upstream PR 编号解析与冲突收敛

## Plan Status

- Implementation authorization: 2026-08-12 已确认仅执行阶段 A“任务归属分离”；阶段 B 的交付重绑和后续维护必须等待 main 明确指令。
- Confirmation date and summary: 2026-08-11 用户已确认修复语义；2026-08-12 main 审计确认 PR #114 是独立 follow-up，不能继续附着在 `08-10-github-actions-governance`。
- Confirmed coverage: 新任务包承接 PR #114 的 PRD、施工入口、交付证据和索引身份；恢复旧任务的 #108 历史；本阶段不改工作流实现。
- Planning revision: 源规划提交 `2016c25ef7cb6ae524f3f2b4e86996ef923981a3`；本任务包由 2026-08-12 的 Trellis create 创建，不伪造回溯日期目录。
- Execution route: delegated worktree `/Users/knaifen/Documents/Codex/aio-coding-hub/08-11-upstream-sync-pr-resolution`。
- Migrated from direct-main record: 无；从 `08-10-github-actions-governance` 的错误归属迁入。

## Material Facts, Assumptions, and Open Questions

| Item | Source | Status / closure condition |
|---|---|---|
| PR #108 已合并，merge commit 为 `82820b2ea10ec6028d1fcb8d130a993bfae39b6d`，不是本任务的功能 PR。 | main 审计与 GitHub 历史 | confirmed |
| PR #114 是 Draft `fix/upstream-sync-pr-resolution`，base 为上述 #108 merge commit；阶段 A 开始时远端 head 为 `ed4a7527f75ea09ff55517afa3789babd0f922a6`。 | GitHub PR 实时查询 | confirmed；每次交付以 GitHub 的实时 head 为准 |
| 新建路径必须从 `gh pr create` stdout 严格解析当前仓库的正整数 PR URL；已有 PR 才允许受限 `gh pr list` 查询。 | 用户确认、workflow 与 policy contract | confirmed |
| PR #113 是既有冲突 PR；`DIRTY`、`UNKNOWN` 或空 merge state 必须 fail-closed，并要求人工处理。 | run `31508611251` 与现行 workflow | confirmed；不得在本任务中处理或合并 #113 |
| 旧 `08-10-github-actions-governance` 的 PRD、设计、施工、交付与 `task.json` 被 #114 链条改写，造成任务归属不一致。 | `origin/main...HEAD` 差异与 main 审计 | confirmed；阶段 A 必须将该目录恢复至 `origin/main` |
| 阶段 B 何时开始由 main 决定。 | 2026-08-12 main 指令 | open，但仅阻止阶段 B，不阻止阶段 A 收尾 |

## Goal

让 PR #114、分支、base、worktree、唯一写者和交付证据只属于本独立任务包，同时保留新建同步 PR 编号解析与冲突 fail-closed 的既有修复语义。

## Requirements

- 使用 Trellis 在当前日期创建独立任务包；不得手工复制或伪造回溯日期目录。
- 新 `prd.md`、`execution.md` 和 `delivery.md` 只描述 PR #114；旧任务继续保留 #108 的历史，不再承载 #114。
- 新任务元数据和 `.trellis/tasks/README.md` 必须一致地记录 PR #114、`fix/upstream-sync-pr-resolution`、base `82820b2ea10ec6028d1fcb8d130a993bfae39b6d`、本 worktree 与当前唯一写者。
- `git diff --name-only origin/main...HEAD -- .trellis/tasks/08-10-github-actions-governance` 必须为空。
- 不修改、读取、执行、移动、删除或暂存 `upgrade-tui.command`；不修改、暂存或提交 `SESSION_REMEDIATION_PLAN.md`。
- 不运行 pnpm、Cargo、依赖安装、构建、格式化、Rust tests 或本地服务；不合并 PR、开启 auto-merge、归档任务、删除 worktree/分支或推送 `main`。

## Acceptance Criteria

- [ ] AC-01：PR #114 的任务材料存在于本任务包，且 `task.json`、`execution.md`、`delivery.md` 与任务索引的 PR、分支、base、worktree 和唯一写者一致。
- [ ] AC-02：PR #114 对 `.trellis/tasks/08-10-github-actions-governance/` 不再有任何差异；#108 历史未被改写。
- [ ] AC-03：交付快照只引用 PR #114 的代码、当前 head、`ci-gate`、`pr-title`、CodeQL 和 #113 的 fail-closed 回归证据；旧 `cdc427b9` / run `31506469918` 仅可作为历史背景，不能当作最终候选。
- [ ] AC-04：阶段 A 以一个任务归属分离切片提交并只推送 `fix/upstream-sync-pr-resolution`；不进入阶段 B。

## Scope and Decision Changes

| Date | Old / new decision | Affected acceptance criteria | Decision owner / resume condition |
|---|---|---|---|
| 2026-08-12 | 从旧 CI 治理任务包中拆出 PR #114，创建独立 follow-up 包；阶段 A 只处理归属与记录。 | AC-01 至 AC-04 | main 审计决定；阶段 B 需 main 明确指令 |

## PENDING Review

- `AIO-PENDING-029`：明确排除。该项仅调查 `upgrade-tui.command`；本任务不得读取、执行、修改、移动、删除或暂存该文件。

## Notes

- 阶段 A 是文档与任务元数据迁移，不需要复制旧任务完整的 CI 治理设计，也不新增 `design.md` 或 `implement.md`。
- 阶段 B 开始前，main 必须重新审核本任务材料并决定是否运行 `task.py start`。
