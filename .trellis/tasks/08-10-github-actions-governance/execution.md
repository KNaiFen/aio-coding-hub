# 施工入口：GitHub Actions 流程治理与提速

## 快速定位

- 任务目录：`.trellis/tasks/08-10-github-actions-governance/`
- Worktree：`/Users/knaifen/Documents/Codex/aio-coding-hub/08-11-upstream-sync-pr-resolution`
- 分支：`fix/upstream-sync-pr-resolution`
- 基线：`origin/main`
- 完整 base SHA：`82820b2ea10ec6028d1fcb8d130a993bfae39b6d`
- 规划提交：待本规划检查点提交后回填；未回填前不得开始施工。
- 实施授权：已确认；2026-08-10 按审查结果实施，2026-08-11 确认扩展 PR 前端/Rust/共享路径分域验证；同日确认修复 Sync Upstream 新建 PR 后的编号解析。Rust 并行化仍延后独立实验。
- PR 目标：`main`
- 已合并前序 PR：[#108](https://github.com/KNaiFen/aio-coding-hub/pull/108)，merge commit `82820b2ea10ec6028d1fcb8d130a993bfae39b6d`
- 当前修复 PR：待创建；真实失败证据为 [run 31487461146](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31487461146) 与 [PR #113](https://github.com/KNaiFen/aio-coding-hub/pull/113)
- 直接 main 迁移来源：无
- PENDING 审阅：`AIO-PENDING-029` 明确排除，禁止触碰 `upgrade-tui.command`
- 当前唯一写者：main session（仅完成规划；移交后由执行 session 独占写入）
- 当前阶段：已确认的 post-merge 修复规划；等待规划提交、执行 session 施工、PR 与 CI。

## 阅读顺序

1. 根目录及 worktree 生效的 `AGENTS.md`。
2. 本文件、`prd.md`、`design.md`、`implement.md`。
3. `.github/workflows/sync-upstream.yml`、`scripts/check-sync-upstream-policy.mjs`、对应 selftest 与 `docs/operations/github-actions-governance.md`。
4. `delivery.md`（完成实现、PR 与 CI 后补齐）。

## 锁定边界

- 自动 PR 的 required `ci-gate` 不能被手动运行替代。
- `pr-title` 必须独立并覆盖 `edited`。
- 候选制品、签名环境、release 选择和 main push 验证不能削弱。
- 仅 PR 可以按路径分域跳过 frontend 或 Rust；`dev`/`main` push、main 手动恢复和候选构建必须全量，未知/共享/控制面路径必须 fail-closed。
- 不启用 CODEOWNERS、人工 PR 审批或发布人工审批环境。
- 不改 Rust 测试并行度，不处理 `upgrade-tui.command`。
- 仅在已有同步 PR 的路径调用 `gh pr list`；新建路径必须从 `gh pr create` 的 stdout 严格解析当前 GitHub 仓库的正整数 PR URL。格式异常不得回退到 list、重试、猜测或跳过 merge-state 校验。
- 保持无 direct push、无本地 merge、无 `gh pr merge`/API merge、无自动批准；`DIRTY`、`UNKNOWN` 或空 merge state 必须 fail-closed 并明确要求人工处理。

## 完成定义

所有允许修改范围内的代码、合同、自测和文档完成；新建 PR 的 URL/数字解析与新建后不 list 都有负向 selftest；`DIRTY`/`UNKNOWN` fail-closed 保持；PR 最新 head 的 required CI 与 `ci-gate` 通过；`delivery.md` 记录完整 head/base SHA、CI URL、验收结果和真实回归运行。回归运行面对现有冲突 PR #113 时必须准确报告人工冲突，而非编号解析失败。

## Git 与 PR

执行中只写本 worktree；提交按逻辑切片，禁止推送或合并 `main`、禁止自动合并。完成后创建 Draft PR，CI 绿色并更新 `delivery.md` 后暂停等待 main 验收。
