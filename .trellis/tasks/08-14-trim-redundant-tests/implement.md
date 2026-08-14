# 实施计划：精简冗余测试与流程合同

## 0. 开工与冻结

1. 完成 `execution.md` 中全部 preflight；只在登记 sibling worktree 的任务分支操作。
2. 读取 `prd.md`、`design.md`、本计划、curated context 和现行 workflow/test source；遇到与锁定决定冲突的现状先暂停。
3. 推送任务分支并创建指向 `main` 的 Draft PR，正文链接本任务目录；记录完整 head/base。不要触发额外手动 CI。

完成信号：Draft PR 存在，当前唯一写者是 execution session，PR 仍指向 `main`。

## 1. 删除未执行的测试入口

1. 从 `package.json` 删除无调用的 unit、shard、coverage-shard、watch scripts；保留 `test:unit:coverage`。
2. 删除 `scripts/run-coverage-shards.mjs`。
3. 删除只有过期 aggregate stages 消费的 `scripts/run-checks.mjs` 和 `check:plugin-hardening` script；将其 workflow 结构断言直接留在 `check-ci-quality-gates`，不得新建替代 aggregate runner。
4. 删除 `scripts/check-plugin-system-completion.mjs` 与 root package script；检查其约束已由保留 plugin API/docs/CI contracts 覆盖。
5. 保留 `create-aio-plugin:test` 并将 frontend workflow 的 scaffolder test step 改为调用它，消除 workflow 直接 filter 命令与 root script 的双入口。

完成信号：逐个搜索删除文件名和 script key，不存在实时代码/工作流/活跃文档引用；历史记录命中不改写。

## 2. 让 E2E 只运行一次

1. 明确根 Vitest `include` 覆盖 `src/**/*.{test,spec}.{ts,tsx}`，且不要在 `exclude` 中加入 `src/e2e`。
2. 删除 `test:e2e` root script 和 frontend 的 `Plugin GUI E2E smoke` step。
3. 在两个保留的机器合同及其 selftest 中删除“必须存在 `pnpm test:e2e`”的断言，替换为对 coverage run 和 E2E discovery 的新合同；不要使此变更削弱 frontend coverage threshold。

完成信号：CI frontend 仅有一个 root Vitest coverage step；最终 log 中该 E2E 文件出现一次。

## 3. 收敛 CI contract jobs

1. 按设计创建单一 `contracts` job，条件为任一 checked docs 或 source domain 被选中。
2. 迁移 docs/support 的每一项 Node command，并以 step-level condition 保持原触发范围；避免 docs-only 运行 source-only selftests。
3. frontend/Rust 改依赖 `contracts`，保留 `always()` 和 success guards；删除重复 plugin docs/API commands。
4. 更新 `ci-gate` expected results、needs 和环境变量，使 process-only、docs-only、frontend-only、Rust-only、mixed/full 各自的 selected/skipped 状态明确且 fail-closed。
5. 不修改 `change-scope` 算法、scope JSON、candidate/release/CodeQL/performance/dev-build/pr-title 的触发和权限。

完成信号：YAML 仍符合 pin-policy canonical structure；CI checker/selftest 覆盖新 job 图并拒绝关键 job/condition/command 缺失。

## 4. 拆分合同职责并删除孤立 selftest

1. 在 `check-cloud-only-verification.mjs` 中只保留 cloud-boundary 断言；移除由 quality checker 持有的重复 CI command/topology assertions，并同步其 selftest。
2. 在 `check-ci-quality-gates.mjs` 中直接验证实际 workflow；移除 `run-checks` import 与针对未调用 stage 的 assertions，并同步 selftest fixtures/negative cases。
3. 删除 `check-plugin-api-contract.selftest.mjs`；在 production checker 固定 repository root，删除 `AIO_PLUGIN_CONTRACT_TEST_ROOT` 分支。
4. 保留 production plugin API contract invocation；不得让删除 selftest 变成跳过 plugin API contract。

完成信号：每个保留 checker 的 source 和 selftest 都可作为直接 Node 合同运行；不存在被删 selftest、环境变量或 aggregate runner 的 active references。

## 5. 删除或合并具体测试

1. 从 `src/ui/__tests__/ui.test.tsx` 删除 Tooltip、RadioGroup、Select、Textarea、FormField 五个重复 case，保留 Popover/Dialog。
2. 将 FormField branch 文件中自动生成 ID 和显式 `htmlFor` case 合入 `FormField.test.tsx`；删除 branch 文件。
3. 删除 `gatewayEvents.coverage.test.ts` 中“clears circuit dedup map”无效 case，保留 `gatewayEvents.test.ts` 对 500-entry eviction 的断言。
4. 删除 `src-tauri/src/lib.rs` 中 ignored `export_bindings` test，保留 CI bindings example 及其 workflow command。

完成信号：无弱断言残留；每项保留测试都直接表达可观察行为或资源边界。

## 6. 同步现行合同和文档

1. 更新 cloud-only 与 CI change-scope specs 对 job 名称、依赖及 frontend E2E 执行方式的描述。
2. 更新 active 插件运行时文档，删除对不存在 `check:plugin-hardening` 的指示，改为准确的 CI frontend plugin gates。
3. 不改历史 audit/plan/archive；任务内 `delivery.md` 只记录真实实施和验证。

完成信号：spec-links 和相关 static docs contract 通过，现行文档没有过期 job/script 名称。

## 7. 本地允许验证与提交

按仓库 cloud-only contract 执行且只执行以下无依赖、非写入检查：

```bash
node --check <每个实际修改且未删除的 .mjs 文件>
node scripts/check-cloud-only-verification.selftest.mjs
node scripts/check-cloud-only-verification.mjs
node scripts/check-ci-quality-gates.selftest.mjs
node scripts/check-ci-quality-gates.mjs
node scripts/check-plugin-api-contract.mjs
node scripts/check-plugin-system-docs.mjs
node scripts/check-spec-links.mjs
node scripts/check-tui-release-contract.mjs
node scripts/check-github-actions-pin-policy.selftest.mjs
node scripts/check-github-actions-pin-policy.mjs
python3 ./.trellis/scripts/task.py validate .trellis/tasks/08-14-trim-redundant-tests
git diff --check origin/main...HEAD
```

禁止执行 pnpm/npm/yarn scripts、Vitest、Cargo、Rustfmt、Clippy、构建、生成、开发服务器、Tauri、签名或打包。每个逻辑阶段提交一次，提交前检查 staged diff 和未识别文件；不得 amend 或把无关修改纳入提交。

## 8. CI、交付与暂停

1. 推送最新提交，等待自动 `ci-gate` 与 `pr-title`；本任务会触发 complete CI，frontend/Rust 应运行，candidate/release jobs 对 PR 应跳过。
2. 以 3-5 分钟间隔检查相同完整 PR head，最长 60 分钟；head 漂移、失败、终态或接近截止才提前核验。修复仅限本任务范围。
3. 绿色后把 PR 标记 Ready for review，按模板填充 `delivery.md`，写入完整 head/base、`ci-gate` 链接、每条 AC、实际 scope/jobs、偏移和风险。
4. 停止写入 worktree，通知 main 进行冻结 head 验收；不得 merge、archive、删除 worktree/branch 或运行 `/trellis:finish-work`。
