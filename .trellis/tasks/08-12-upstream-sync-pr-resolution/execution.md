# 施工入口：修复 Sync Upstream PR 编号解析与冲突收敛

> 阶段 A 已完成。main 于 2026-08-12 授权阶段 B；本文件是唯一施工入口，执行 session 必须按下列顺序同步 PR #114、重绑交付并暂停等待 main 验收。

## 快速定位

- 任务目录：`.trellis/tasks/08-12-upstream-sync-pr-resolution/`
- Worktree：`/Users/knaifen/Documents/Codex/aio-coding-hub/08-11-upstream-sync-pr-resolution`
- 分支：`fix/upstream-sync-pr-resolution`
- 历史 PR base：`main` @ `82820b2ea10ec6028d1fcb8d130a993bfae39b6d`
- 当前同步目标快照：`origin/main` @ `9aa8e4ab8e6417be4816b0811178c3f401e34171`；开始前必须重新 `git fetch origin`，不得机械复用此值
- 源规划提交：`2016c25ef7cb6ae524f3f2b4e86996ef923981a3`
- 实施授权：2026-08-12 阶段 B 已获 main 授权，范围为主线同步、README 冲突收敛、交付和 CI 重绑。
- PR 目标：`main`
- PR：[PR #114](https://github.com/KNaiFen/aio-coding-hub/pull/114)（Draft）
- 前序 PR：[#108](https://github.com/KNaiFen/aio-coding-hub/pull/108) 已合并，merge commit `82820b2ea10ec6028d1fcb8d130a993bfae39b6d`。
- PENDING 审阅：`AIO-PENDING-029` 明确排除，禁止触碰 `upgrade-tui.command`。
- 当前唯一写者：main（本授权提交）；提交推送并完成交接后，恰好一个执行 session 成为唯一写者。
- 当前阶段：阶段 A 已完成；阶段 B 已授权、待执行 session 接手。

## 阅读顺序

1. 当前 worktree 生效的 `AGENTS.md`。
2. 本文件、`prd.md` 与 `delivery.md`。
3. `.github/workflows/sync-upstream.yml`、`scripts/check-sync-upstream-policy.mjs` 与对应 selftest。
4. 不读取、暂存、编辑、删除或移动 `SESSION_REMEDIATION_PLAN.md`；它不是本任务的权威计划材料。

## 阶段 A 完成定义

- 本任务包拥有 PR #114 的 PRD、施工入口、交付快照和任务元数据；不再把 #108 作为当前修复 PR。
- `.trellis/tasks/08-10-github-actions-governance/` 的 #114 时期改动全部恢复为 `origin/main`，且指定 `git diff --name-only` 命令无输出。
- `.trellis/tasks/README.md` 将 #114、分支、base、worktree、唯一写者和 #113 的 fail-closed 边界定位到本任务包。
- 仅提交并推送任务归属分离切片；不改工作流代码、不处理 #113 冲突、不运行 `task.py start`。

## 已锁定边界

- 新建同步 PR 的路径只接受 `gh pr create` stdout 中当前 `KNaiFen/aio-coding-hub` 的正整数 PR URL；已有 PR 路径才允许 list 查询。
- URL、编号、`DIRTY`、`UNKNOWN` 或空 merge state 一律 fail-closed；#113 留给人工处理。
- 不得 direct push、local merge、`gh pr merge`、自动批准或扩大 token/Actions 权限。
- 不得修改、读取、执行、移动、删除或暂存 `upgrade-tui.command`，也不得暂存、编辑、删除或提交 `SESSION_REMEDIATION_PLAN.md`。
- 不得运行 pnpm、Cargo、依赖安装、构建、格式化、Rust tests 或本地服务；不得合并 PR、开启 auto-merge、归档、删除 worktree/分支或推送 `main`。

## 阶段 B 施工指令

1. 确认当前目录和分支分别为本 worktree 与 `fix/upstream-sync-pr-resolution`。`git status --short` 只允许出现既有未跟踪的 `SESSION_REMEDIATION_PLAN.md`；任何其他改动、rebase/merge 进行中状态或无法归属的文件都必须停止并报告 main。
2. 执行 `git fetch origin`，重新查询 PR #114 的 head、base、Draft、merge state 和检查。历史快照为 head `6316204274eeb6db9332b4eef0e5f182c5c31ca7`、PR base `82820b2e...`；它们仅供比对，实时结果优先。
3. 确认没有上述阻塞后，运行 `python3 ./.trellis/scripts/task.py start .trellis/tasks/08-12-upstream-sync-pr-resolution`，使 Trellis 生命周期进入 `in_progress`。随后本执行 session 成为唯一写者。
4. 使用 `git merge --no-edit origin/main` 将已发布分支同步到最新主线。预期只在 `.trellis/tasks/README.md` 冲突：以 `origin/main` 为底，保留 08-10 和 08-11 的归档条目，并保留 `08-12-upstream-sync-pr-resolution` 的活动行；不得恢复任何旧任务活动目录或 #114 归属材料。出现其他冲突时停止并报告 main。
5. 不修改 `.github/workflows/sync-upstream.yml` 或 policy/selftest 的既有行为，除非同步冲突迫使改动且 main 先确认。始终保持新建 PR stdout 严格解析、`DIRTY`/`UNKNOWN`/空状态 fail-closed，以及无 direct push、merge、approval 的边界；不得处理 #113。
6. 在提交前运行允许的验证：`node --check scripts/check-sync-upstream-policy.mjs`、`node --check scripts/check-sync-upstream-policy.selftest.mjs`、`node scripts/check-sync-upstream-policy.mjs`、`node scripts/check-sync-upstream-policy.selftest.mjs`、`python3 ./.trellis/scripts/task.py validate 08-12-upstream-sync-pr-resolution`、`git diff --check`，以及 `git diff --name-only origin/main...HEAD -- .trellis/tasks/08-10-github-actions-governance`（必须无输出）。不得运行 pnpm、Cargo、构建、格式化、Rust tests、服务或手动 workflow dispatch。
7. 仅提交阶段 B 的同步、Trellis 生命周期和交付记录，推送 `fix/upstream-sync-pr-resolution`。等待自动 PR CI，不得开启 auto-merge、执行 `gh pr merge`、推送 `main`、归档或删除 worktree/分支。
8. CI 完成后更新 `delivery.md`：说明实际同步结果、冲突处理、最新可验证候选、对应 `ci-gate`/`pr-title`/CodeQL、#113 fail-closed 证据和未执行项。将 PR 标为 Ready for review 后停止写入，向 main 报告完整 head SHA、检查链接、变更文件和最终 `git status`。
