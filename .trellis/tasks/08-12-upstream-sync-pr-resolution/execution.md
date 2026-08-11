# 施工入口：修复 Sync Upstream PR 编号解析与冲突收敛

> 阶段 A 已限定为任务归属分离。不得运行 `task.py start` 或进入阶段 B，直到 main 发出新的明确指令。

## 快速定位

- 任务目录：`.trellis/tasks/08-12-upstream-sync-pr-resolution/`
- Worktree：`/Users/knaifen/Documents/Codex/aio-coding-hub/08-11-upstream-sync-pr-resolution`
- 分支：`fix/upstream-sync-pr-resolution`
- 基线：`origin/main`
- 完整 base SHA：`82820b2ea10ec6028d1fcb8d130a993bfae39b6d`
- 源规划提交：`2016c25ef7cb6ae524f3f2b4e86996ef923981a3`
- 实施授权：2026-08-12 已确认阶段 A；阶段 B 暂停，等待 main 指令。
- PR 目标：`main`
- PR：[PR #114](https://github.com/KNaiFen/aio-coding-hub/pull/114)（Draft）
- 前序 PR：[#108](https://github.com/KNaiFen/aio-coding-hub/pull/108) 已合并，merge commit `82820b2ea10ec6028d1fcb8d130a993bfae39b6d`。
- PENDING 审阅：`AIO-PENDING-029` 明确排除，禁止触碰 `upgrade-tui.command`。
- 当前唯一写者：本执行 session（仅阶段 A；提交推送后暂停）。
- 当前阶段：planning；阶段 A“任务归属分离”。

## 阅读顺序

1. 当前 worktree 生效的 `AGENTS.md`。
2. `SESSION_REMEDIATION_PLAN.md`（只读、不得暂存或编辑）。
3. 本文件、`prd.md` 与 `delivery.md`。
4. `.github/workflows/sync-upstream.yml`、`scripts/check-sync-upstream-policy.mjs` 与对应 selftest。
5. 阶段 B 只有在 main 指令后才继续读取新的交付与 CI 状态。

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

## 阶段 B 入口

main 明确授权后，先重新核验当前目录、分支、base、PR #114 最新 head 及检查状态；再决定是否运行 `task.py start`。届时仍必须保持上述 stdout 解析、fail-closed 和 no-merge 边界，并在最新 PR head 的 CI 完成后重写 `delivery.md`。
