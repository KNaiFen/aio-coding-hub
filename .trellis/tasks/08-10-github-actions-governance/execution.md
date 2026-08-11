# 施工入口：GitHub Actions 流程治理与提速

## 快速定位

- 任务目录：`.trellis/tasks/08-10-github-actions-governance/`
- Worktree：`/Users/knaifen/Documents/Codex/aio-coding-hub/08-10-github-actions-governance`
- 分支：`chore/github-actions-governance`
- 基线：`origin/main`
- 完整 base SHA：`9b05b28d5841584dc6f2a867947afd5d23f76246`
- 规划提交：`30a021269f3b6ae2c46f195faa273a1af81f26f9`
- 实施授权：已确认；2026-08-10 按审查结果实施，2026-08-11 确认扩展 PR 前端/Rust/共享路径分域验证；Rust 并行化仍延后独立实验。
- PR 目标：`main`
- PR：[#108](https://github.com/KNaiFen/aio-coding-hub/pull/108)
- 直接 main 迁移来源：无
- PENDING 审阅：`AIO-PENDING-029` 明确排除，禁止触碰 `upgrade-tui.command`
- 当前唯一写者：main session
- 当前阶段：已按 2026-08-11 确认范围重新进入施工；既有交付候选不再代表最新验收对象。

## 阅读顺序

1. 根目录及 worktree 生效的 `AGENTS.md`。
2. 本文件、`prd.md`、`design.md`、`implement.md`。
3. `.github/workflows/ci.yml`、`sync-upstream.yml`、现有 contracts/selftests。
4. `delivery.md`（完成实现、PR 与 CI 后补齐）。

## 锁定边界

- 自动 PR 的 required `ci-gate` 不能被手动运行替代。
- `pr-title` 必须独立并覆盖 `edited`。
- 候选制品、签名环境、release 选择和 main push 验证不能削弱。
- 仅 PR 可以按路径分域跳过 frontend 或 Rust；`dev`/`main` push、main 手动恢复和候选构建必须全量，未知/共享/控制面路径必须 fail-closed。
- 不启用 CODEOWNERS、人工 PR 审批或发布人工审批环境。
- 不改 Rust 测试并行度，不处理 `upgrade-tui.command`。

## 完成定义

所有允许修改范围内的代码、合同、自测和文档完成；frontend-only、Rust-only、shared、docs 与主干全量语义都有自测；PR 最新 head 的 required CI 与 `ci-gate` 通过；`delivery.md` 记录完整 head/base SHA、CI URL、验收结果和未完成外部设置。

## Git 与 PR

执行中只写本 worktree；提交按逻辑切片，禁止推送或合并 `main`、禁止自动合并。完成后创建 Draft PR，CI 绿色并更新 `delivery.md` 后暂停等待 main 验收。
