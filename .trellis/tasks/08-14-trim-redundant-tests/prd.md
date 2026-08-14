# 精简冗余测试与流程合同

## Plan Status

- Implementation authorization: confirmed
- Confirmation: 2026-08-14；用户要求按审计结论实施、删除孤立的插件合同 selftest，并使用独立 multi-worktree execution session。
- Confirmed coverage: 本 PRD 的范围、锁定决定和 AC。
- Planning revision: `PLANNING_SHA_PENDING`（main 在规划材料提交后回填）。
- Execution route: delegated sibling worktree。
- Migrated from direct-main record: 无。

## Material Facts, Assumptions, and Open Questions

| Item | Source | Status / closure condition |
| --- | --- | --- |
| `main` 和 `origin/main` 的共同基线 | 2026-08-14 preflight | 已确认：`1b218897c09894cfb5aff796761eb8004ad6e53f`。 |
| PENDING | `PENDING.md` | 已审阅；没有 `pending` 或 `planned` 条目。 |
| 最近的 frontend CI | Actions run `31779025469` | 已确认：`src/e2e/plugins.e2e.test.tsx` 先在独立 E2E 步骤运行 2.16s，后又被 262.99s 的 coverage run 发现并执行。 |
| 最近的 Rust CI | 同一 Actions run | 已确认：Rust tests 约 18 分 26 秒；没有充分证据支持删除行为测试。 |
| 实现问题 | 用户决定、现行合同和审计证据 | 无材料性未决问题。 |

## Goal

删除重复、失效和孤立测试入口，收敛 CI 合同职责，同时保留必要的前端、Rust、安全、release 和分支保护门禁。

## Locked Decisions

1. `src/e2e/plugins.e2e.test.tsx` 必须仍在每次 frontend CI 中执行，但只能执行一次，由根 `vitest run --coverage` 发现；不再单独运行 `pnpm test:e2e`。
2. 删除孤立的 `scripts/check-plugin-api-contract.selftest.mjs`，并移除仅为它存在的 `AIO_PLUGIN_CONTRACT_TEST_ROOT` 注入分支；生产插件合同检查仍必须在 CI 中运行。
3. 删除无 workflow 调用、且按 cloud-only 合同无法在本地使用的 unit/shard/watch 聚合入口；不恢复本地 hooks、依赖安装或本地测试路线。
4. 将 `docs-contract` 与 `support-contract` 的无依赖静态检查合并为一个 `contracts` job，保留 change-scope 的 fail-closed 语义和稳定的自动 `ci-gate`。
5. `check-cloud-only-verification` 只拥有本地/云端执行边界；`check-ci-quality-gates` 只拥有 CI 拓扑、workflow 命令和 required-gate 结构。不得保留两者对同一 CI 命令列表的双重断言。
6. 删除已被专用测试覆盖的 UI 聚合 case 和无效 gateway coverage case；保留专用测试、Popover、Dialog、页面 smoke、插件 SDK/脚手架测试和全部正常 Rust 行为测试。
7. 将 `FormField.branch.test.tsx` 的自动 ID、显式 `htmlFor` 两项独特断言合入 `FormField.test.tsx` 后再删除 branch 文件。
8. 删除被 `src-tauri/examples/export-bindings.rs` 取代的 ignored `export_bindings` test；CI 的 bindings example 仍是唯一导出入口。

## Requirements

- 删除独立 E2E workflow step、其 root script 和只为该命令存在的机器合同断言；根 coverage 仍覆盖 `src/e2e`。
- 删除下列过期入口及其死引用：`test:unit`、四个 `test:unit:shard:*`、`test:unit:shards`、`test:unit:coverage:shards`、`test:unit:watch`、`scripts/run-coverage-shards.mjs`、未被实际 workflow 调用的 aggregate runner/stage，以及过期 plugin completion checker。
- CI 仅保留实际需要的根或 workspace scripts；`create-aio-plugin` 测试在 workflow 中使用保留的 root script，避免同一命令存在不一致的两种入口。
- 合并 contract jobs 后，所有原有检查仍在原有适用范围执行：docs-only、frontend-only、Rust-only、mixed/shared 和 process-documentation 均有明确 job 结果；frontend/Rust 只依赖新的 `contracts` 结果。
- 更新两个无依赖合同检查器及其 selftest，使它们验证新 job 图和职责边界；移除对删除入口、删除 job 或 aggregate runner 的要求。
- 删除无独特行为的 `ui.test.tsx` case、gateway 无效 case 和 Rust ignored wrapper；合并 FormField 的独特断言。
- 更新活跃 cross-layer spec 和活跃插件运行时文档，准确反映新 CI job 和不再存在的 `check:plugin-hardening` 聚合入口。不得改写历史审计、历史计划或归档记录。

## Acceptance Criteria

- **AC-01 Exactly-once E2E**：frontend workflow 不再含独立 `pnpm test:e2e`；`test:unit:coverage` 仍运行，Vitest 配置明确包含 `src/e2e` 测试；最新 PR frontend log 证明该 E2E 文件恰好运行一次。
- **AC-02 Dead entry removal**：删除的 package scripts、coverage shard runner、aggregate runner、plugin completion checker 和 plugin API selftest 不再被源码、workflow、现行文档或机器合同引用；保留的 `create-aio-plugin` root script 被 frontend workflow 实际调用。
- **AC-03 Contract consolidation**：`contracts` 是 docs/source 静态合同的唯一执行 owner；`ci-gate`、frontend 和 Rust 对它的依赖、条件与 selected/skipped 结果均符合 change-scope 合同。不存在 docs/support/frontend 对同一静态检查的重复执行。
- **AC-04 Contract ownership**：cloud-only checker 不再维护 CI quality matrix 的重复 command assertions；CI quality checker 不再依赖或验证未调用的 `run-checks` stage 配置。两者的 selftest 仍覆盖各自的 fail-closed 负例。
- **AC-05 Test cleanup**：指定 UI/gateway/Rust 测试删除或合并后，保留的专用测试仍表达可观察行为；`FormField.test.tsx` 覆盖自动 ID 和显式 `htmlFor`；没有删除 Popover、Dialog 或 gateway 的资源边界测试。
- **AC-06 Required gates**：自动 `ci-gate` 与独立 `pr-title` 保持；CodeQL、candidate/release、performance、dev-build、Rust Clippy/test/audit、SDK/脚手架 test/typecheck 和 root coverage/build 都未被降级或删除。
- **AC-07 Verification**：修改的无依赖 Node contract/selftest 和 `node --check` 通过；`git diff --check` 通过；最新 PR head 的 `ci-gate`、`pr-title` 和按 full scope 选中的 frontend/Rust jobs 绿色。候选/release jobs 对 PR 按设计跳过并在 `delivery.md` 说明。

## Non-Goals

- 不为缩短 Rust CI 删除、放宽或并行化任何根 Rust/integration 行为测试；若未来有 timing 证据，可单列任务拆分无全局状态 workspace crate。
- 不改变 change-scope 分类规则、`.github/ci-scope.json`、候选构建、release promotion、CodeQL、performance、dev-build 或 PR title 的产品/安全语义。
- 不增加依赖、不改 lockfile、不改变插件 API、运行时、发布制品或用户可见功能。
- 不改写历史文档、归档任务或 `PENDING_COMPLETED.md`。

## Scope and Decision Changes

| Date | Old / new decision | Affected acceptance criteria | Decision owner / resume condition |
| --- | --- | --- | --- |
| 2026-08-14 | 从“仅审计”变为“按审计结论实施”；孤立 plugin API selftest 的处置锁定为删除。 | AC-01 至 AC-07 | 用户确认。 |

## PENDING Review

- 无未解决条目；本任务不创建或迁移 PENDING 项。
