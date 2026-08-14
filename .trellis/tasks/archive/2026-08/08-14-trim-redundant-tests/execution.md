# 施工入口：精简冗余测试与流程合同

> 按本文件和同目录权威任务材料施工。完成实现、PR、CI 和 `delivery.md` 后暂停等待 main 验收。你是此 sibling worktree 的唯一写者；不得再创建 worktree、派生任务或与 main 同时写产品/流程文件。

## 快速定位

- 任务目录：`.trellis/tasks/08-14-trim-redundant-tests/`
- Worktree：`/Users/knaifen/Documents/Codex/aio-coding-hub/workflow-test-cleanup`
- 分支：`chore/trim-redundant-tests`
- 基线：`origin/main`
- 完整 base SHA：`1b218897c09894cfb5aff796761eb8004ad6e53f`
- 规划提交：`cea9dad385e508c716956d644e3ef6021c8d04fe`（包含本文件与开工前材料）
- 实施授权：已确认，2026-08-14；覆盖本任务 PRD 的 locked decisions、范围与 AC。
- PR 目标：`main`
- PR：尚未创建；执行 session preflight 后尽早创建 Draft PR。
- 关联任务：`08-03-upstream-claude-oauth` 仅在 planning、没有 worktree，认证范围与本任务无文件或语义冲突。
- PENDING 审阅：无未解决条目。
- 当前唯一写者：独立 execution session。
- 当前阶段：已完成规划交接；execution session 只有在 preflight 全部通过后才可施工。

## 阅读顺序

1. 仓库根 `AGENTS.md`、`.trellis/workflow.md`、`docs/operations/multi-worktree-delivery.md`。
2. 本文件。
3. `prd.md`、`design.md`、`implement.md`。
4. `implement.jsonl` 和 `check.jsonl` 所列现行 specs；再读与当前阶段相关的 workflow、script、test source。
5. `delivery.md`（首次仅作模板）；返工时再读 `findings.md`。

材料冲突时依次以 PRD 用户决定/AC、设计、实施计划、本入口为准。不根据聊天摘要猜测。

## 冻结交接与开工核验

开始任何写入前逐项执行并确认：

```bash
test "$(pwd)" = "/Users/knaifen/Documents/Codex/aio-coding-hub/workflow-test-cleanup"
test "$(git branch --show-current)" = "chore/trim-redundant-tests"
BASE_SHA="1b218897c09894cfb5aff796761eb8004ad6e53f"
test "$(git merge-base "$BASE_SHA" HEAD)" = "$BASE_SHA"
test -f .trellis/tasks/08-14-trim-redundant-tests/task.json
test -f .trellis/tasks/08-14-trim-redundant-tests/prd.md
test -f .trellis/tasks/08-14-trim-redundant-tests/design.md
test -f .trellis/tasks/08-14-trim-redundant-tests/implement.md
test -f .trellis/tasks/08-14-trim-redundant-tests/execution.md
git cat-file -e "cea9dad385e508c716956d644e3ef6021c8d04fe^{commit}"
```

然后确认 `task.json.status=in_progress`、`prd.md` 授权为 confirmed、execution.md 已回填非占位规划 SHA、当前 worktree 没有来源不明修改、唯一写者已转为 execution session。任一条件失败：停止写入并报告 main；不要自行 rebase、补写登记或修改任务边界。

## 锁定结果

- E2E 文件保留且只由 root coverage run 执行一次；不能删除、排除或降为非覆盖测试。
- 删除用户指定的孤立 plugin API selftest；production plugin API checker 保留且 CI 继续运行。
- 删除没有 workflow 调用的 test/shard/watch/aggregate entry，不恢复本地测试路线。
- `contracts` 是唯一静态合同 job；change-scope、`ci-gate`、`pr-title` 和所有保留的 heavy quality gates 语义不变。
- Rust 长耗时测试、CodeQL、candidate/release、performance、dev-build、SDK/脚手架测试、Popover/Dialog、gateway map-bound test 都不在删除范围。

完整范围、AC 和不变量以 `prd.md` 为准；数据流和 owner 分界以 `design.md` 为准；实施顺序以 `implement.md` 为准。

## 允许修改

- `.github/workflows/ci.yml`。
- `package.json`、`vitest.config.ts`。
- `scripts/check-cloud-only-verification*.mjs`、`scripts/check-ci-quality-gates*.mjs`、`scripts/check-plugin-api-contract.mjs`、相关直接依赖的无依赖 contract/selftest。
- 删除 `scripts/check-plugin-api-contract.selftest.mjs`、`scripts/check-plugin-system-completion.mjs`、`scripts/run-checks.mjs`、`scripts/run-coverage-shards.mjs` 及其死引用。
- `src/ui/__tests__/ui.test.tsx`、`src/ui/__tests__/FormField.test.tsx`、删除 `src/ui/__tests__/FormField.branch.test.tsx`、`src/services/gateway/__tests__/gatewayEvents.coverage.test.ts`、`src-tauri/src/lib.rs`。
- `.trellis/spec/aio-coding-hub/cross-layer/{cloud-only-verification-contract,ci-change-scope-contract}.md`、`docs/plugins/runtime/README.md` 和任务材料。

## 明确禁止

- `.github/ci-scope.json` 或 `scripts/ci-change-scope.mjs` 的分类算法；除非执行发现 job consolidation 无法保持既有 selected/skipped 合同，先暂停。
- 任何 dependency/lockfile、产品 runtime、插件 API、generated bindings、发布/签名/候选构建、CodeQL、performance、dev-build、PR title 语义。
- 删除、放宽或并行化正常 Rust 行为测试；不触及 `gatewayEvents.test.ts` 的 map-bound test。
- 历史 docs、归档任务、PENDING 文件、其他 worktree、真实凭据。

## 实施与验证

严格按 `implement.md` 的 0 至 8 顺序。每阶段先确认范围和完成信号，再做最小改动；需要改动未列出的控制面、公共 API、兼容性或一项锁定结果时停止。

本地仅可运行计划列出的无依赖 Node contract/selftest、修改 `.mjs` 的 `node --check`、`task.py validate` 和 `git diff --check`。禁止 pnpm/npm/yarn、Vitest、Cargo、Rustfmt、Clippy、build、generator、dev server、Tauri、签名和打包。

PR CI 监控：只等待自动触发的同一完整 head；正常每 3-5 分钟检查一次，最长 60 分钟。此 PR 预期 full scope：frontend/Rust 运行，candidate/release PR jobs 跳过。CI 失败优先限于任务范围修复；无法可靠归因或需要扩大范围时写入 `delivery.md` 的阻塞快照并暂停。

## 交付与暂停

- 尽早推送分支并创建 Draft PR；完成后标记 Ready for review。
- 每次提交前核对 dirty paths，仅提交本任务变更；不得 amend、不得 push `main`。
- 绿色 CI 后基于实际结果填写 `delivery.md`：完整 head/base、`ci-gate` URL、AC、实际 scope/jobs、局部验证、偏移、风险和回滚。
- 填写后停止写入并通知 main。不得 merge、开启 auto-merge、archive、删除 worktree/branch 或运行 `/trellis:finish-work`。

## 必须停止并报告 main

- 路径、分支、base、规划 SHA、授权、task status 或唯一写者不一致。
- 需要修改 `.github/ci-scope.json`、classifier、依赖/lockfile、产品 runtime、public API、发布/签名或任务外文件。
- 现行合同不能支持 `contracts` job 的 selected/skipped 语义，或 selftest 不能在不扩大范围的前提下覆盖。
- CI 失败无法证明是任务内问题，或同一失败签名重复出现。
- 任一 AC 无法满足，或删除将使 E2E、SDK、scaffolder、Rust、security/release gates 缺失。

## 初始交付状态

- 结果：尚未开始（已交接，等待 execution session 完成 preflight）。
- PR/head/CI：尚未创建 / 尚未提交 / 未触发。
- execution session 当前是唯一写者；完成交付后暂停。
