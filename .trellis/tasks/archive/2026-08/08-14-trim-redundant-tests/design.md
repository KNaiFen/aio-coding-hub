# 技术设计：精简冗余测试与流程合同

## 目标边界

这是 CI 和测试维护任务，不改变应用运行时行为。所有删除必须满足至少一个条件：

1. 同一可观察行为已经由更强的保留测试覆盖；
2. 入口在当前 repository workflow 和现行合同中没有可执行调用者；
3. 入口是同一实现的手动包装，而 CI 已有唯一生产入口；
4. 两个合同检查器维护同一事实，能够明确归属到一个 owner。

不能因名字相似、同一 `cargo` 命令或相同目标文件就删除不同平台、不同触发器或不同门禁层级的验证。

## CI 设计

### 单一 `contracts` job

以 `contracts` 取代 `docs-contract` 与 `support-contract`：

```text
change-scope
    |
    +--> contracts (docs_checks OR frontend_ci OR rust_ci)
    |        |
    |        +--> frontend (frontend_ci)
    |        +--> rust (rust_ci)
    |
    +--> candidate-plan (现有 main-only 语义不变)

ci-gate consumes contracts/frontend/rust/candidate jobs
```

`contracts` 是无依赖 Node 静态合同的唯一 owner。它使用 step-level `if` 保持原有执行范围，而不是通过放宽 classification 把检查迁入更便宜的 tier。

| 检查组 | 执行条件 | 原有语义 |
| --- | --- | --- |
| cloud-only production checker | 任一 checked docs 或 source scope | 原 docs/support 覆盖并集 |
| cloud-only selftest、scope selftest、quality/pin/selftest、support/release/signing contracts | `frontend_ci || rust_ci` | 原 support-contract |
| plugin docs/API contract | `docs_checks || frontend_ci` | 原 docs-contract 与 frontend 覆盖并集 |
| spec links | `docs_checks` | 原 docs-contract |
| TUI release contract | 任一 checked docs 或 source scope | 原 docs/support 覆盖并集 |

`frontend` 与 `rust` 的 `needs` 改为 `[change-scope, contracts]`，条件仍要使用 `always()` 并显式验证两个 dependency result，防止 GitHub 对 skipped ancestor 的隐式传播。`ci-gate` 保持自动 `ci-gate` / 手动 `manual-ci-gate` 命名，仅将 `docs-contract`、`support-contract` 的 expected-result 分支换为 `contracts`。process-documentation-only 仍要求 `contracts=skipped`。

### E2E 恰好一次

根 Vitest 的 coverage run 已收集 `src/e2e`。移除单独的 `test:e2e` script 和 workflow step，保留并显式化根 Vitest 的 `include`，使 `src/e2e/plugins.e2e.test.tsx` 由 `test:unit:coverage` 唯一发现。不能把 E2E 文件移出 coverage，也不能将 coverage 门替换为非覆盖 unit run。

### 合同检查职责

| Owner | 保留职责 | 移除职责 |
| --- | --- | --- |
| `check-cloud-only-verification` | package/README/Trellis local-boundary、Actions-only guard、manual/candidate boundary、`contracts` 对 cloud checker 的直接调用 | frontend/Rust 精确 command matrix、`ci-gate` 拓扑、`pr-title` 和 performance workflow 结构断言 |
| `check-ci-quality-gates` | `contracts`/frontend/Rust/`ci-gate` topology、精确 workflow commands、CodeQL、Dependabot、`pr-title`、performance | `run-checks.mjs` 的未执行 stages 和 cloud-only 文档边界 |

必须保留对应 selftest 的 YAML/注释/condition bypass 负例。删除重复断言不等于降低 fail-closed 覆盖。

## 删除与合并设计

### 删除

- `scripts/check-plugin-api-contract.selftest.mjs` 与 `check-plugin-api-contract.mjs` 中仅供该 selftest 注入的环境变量根目录覆盖。
- `scripts/check-plugin-system-completion.mjs`、其 root script，以及无调用的 `scripts/run-checks.mjs` / `scripts/run-coverage-shards.mjs` / unit shard/watch scripts。
- `src/ui/__tests__/ui.test.tsx` 中 Tooltip、RadioGroup、Select、Textarea、FormField 聚合 cases；保留 Popover、Dialog。
- `gatewayEvents.coverage.test.ts` 中不验证 map 清理/容量的 case。
- `src-tauri/src/lib.rs` 中 ignored `export_bindings` wrapper。

### 合并

`FormField.branch.test.tsx` 的自动 ID 与 explicit `htmlFor` 两项断言搬入 `FormField.test.tsx`。hint/group 语义已经由主文件强断言覆盖，不重复迁移。

## 不变量与回滚

- 仍保留 `src/e2e/plugins.e2e.test.tsx`、根 coverage thresholds、插件 SDK/脚手架测试、gateway map-bound test、Rust bindings example。
- 候选、release、CodeQL、performance、dev-build、`pr-title` 和独立 `ci-gate` 的触发与权限边界不变。
- 无数据、API、安全、凭据、生成绑定或兼容性迁移。
- 回滚仅需回退本 PR；不会产生需清理的运行时数据或制品。

## 现行文档

更新 active specs 和活跃插件运行时文档中旧 job/aggregate command 描述；历史 records 只作为审计证据，禁止重写。
