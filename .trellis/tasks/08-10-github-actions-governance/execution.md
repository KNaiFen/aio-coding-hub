# 施工入口：GitHub Actions 流程治理与提速

## 快速定位

- 任务目录：`.trellis/tasks/08-10-github-actions-governance/`
- Worktree：`/Users/knaifen/Documents/Codex/aio-coding-hub/08-10-github-actions-governance`
- records-only 分支：`docs/close-08-10-github-actions-governance`
- 基线：`origin/main` @ `82820b2ea10ec6028d1fcb8d130a993bfae39b6d`
- 规划提交：`30a021269f3b6ae2c46f195faa273a1af81f26f9`
- 实施授权：已确认；2026-08-12 main 审计授权仅完成 records-only closeout。
- PR 目标：`main`
- 功能 PR：[#108](https://github.com/KNaiFen/aio-coding-hub/pull/108) 已合并，实际 merge commit 为 `82820b2ea10ec6028d1fcb8d130a993bfae39b6d`。
- records-only Draft PR：[#115](https://github.com/KNaiFen/aio-coding-hub/pull/115)；本记录更新前的交付快照 head 为 `ebac0bb60745d15839dde2f0425aaac363550c8c`，`ci-gate` 与 `pr-title` 已通过。
- PENDING 审阅：`AIO-PENDING-029` 明确排除，禁止触碰 `upgrade-tui.command`。
- 当前唯一写者：08-10 records-only closeout execution session。
- 当前阶段：功能开发已结束；仅创建并交付 records-only closeout Draft PR，随后暂停等待 main 验收、合并、归档和清理。

## 本轮范围与边界

- 只修正本任务正式记录和 `.trellis/tasks/README.md` 的本任务索引行；不得改动 GitHub Actions 产品代码、同步脚本或测试合同。
- 最终业务结果必须如实记录为“部分完成并已拆分后续修复”：#108 的 CI 治理功能已合并；实际 Sync Upstream 运行发现的编号解析问题属于独立 follow-up。
- follow-up 仅作交叉引用：[08-11-upstream-sync-pr-resolution](../08-11-upstream-sync-pr-resolution/) / [PR #114](https://github.com/KNaiFen/aio-coding-hub/pull/114)。不得在本任务记录 #114 的实现、head SHA、CI run 或验收结论。
- `task.json.pr_url` 继续指向 #108，且保持 `status: in_progress`、`completedAt: null`；只有 main 才能在 records-only PR 合并后归档。
- 不运行 `task.py archive`，不合并 PR、不启用 auto-merge、不删除 worktree/分支，也不推送 `main`。

## 交付定义

- records-only Draft PR 指向 `main`，其最新 head 的自动 `ci-gate` 与 `pr-title` 均为绿色。
- `delivery.md` 记录 #108 的实际 merge commit、部分完成结论、follow-up 的纯链接，以及 records-only PR 的交付快照。
- 推送并更新交付记录后立即暂停；main 负责验收、合并、归档和清理。

## Git 与 PR

执行中只写本 worktree 的允许记录文件；显式暂存这些文件，不包含 `SESSION_REMEDIATION_PLAN.md` 或任何其他未授权路径。完成后创建 Draft records-only PR，等待最新 head 的 required checks 绿色并更新交付记录后暂停等待 main 验收。
