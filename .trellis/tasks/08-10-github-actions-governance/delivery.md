# 交付报告：GitHub Actions 流程治理与提速

> 本交付记录冻结代码实现候选 `5d507fa497bd79c755c07a0e7711f3975197d4f5`。本文件本身提交后会产生新的文档 head，main 验收时必须以 PR 实时 head 和实时检查为准。

## 交付状态

- 结果：等待验收
- PR：[#108](https://github.com/KNaiFen/aio-coding-hub/pull/108)
- 分支：`chore/github-actions-governance`
- PR base：`main` @ `9b05b28d5841584dc6f2a867947afd5d23f76246`
- 交付候选 head：`5d507fa497bd79c755c07a0e7711f3975197d4f5`
- 规划提交：`30a021269f3b6ae2c46f195faa273a1af81f26f9`
- `ci-gate`：通过，[run 31403797650](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31403797650)（job 93512912035）
- 其他检查：`pr-title`、`docs-contract`、`support-contract`、`frontend`、`rust`、CodeQL JS/TS 和 Rust 均通过；候选制品 job 按 PR 条件跳过。
- 交付时间：2026-08-10T15:56:34Z
- 执行 session：已暂停写入，等待 main 验收。

## 阻塞快照

无。

## 实现摘要

### 用户可见结果

- 自动 PR/push CI 不再因手动入口 guard 的正常 skipped 状态而跳过应执行的合同、前端和 Rust job；`ci-gate` 保持 fail-closed。
- CodeQL Rust 分支改用受支持的 no-build 模式，避免初始化阶段失败及无意义的 Rust 构建准备步骤。

### 内部实现

- `.github/workflows/ci.yml` 为 docs、support、frontend、rust、candidate plan/build/assemble 显式加入 `always()` 与直接依赖成功条件，保留非 main 手动触发的早失败和候选制品边界。
- `.github/workflows/codeql.yml` 的 JS/TS 与 Rust matrix 均使用 `build-mode: none`；移除 Rust 系统依赖、工具链和 `autobuild` step。
- 两个 CI 合同检查器和 selftest 锁定上述依赖图与 CodeQL no-build step 集合，现行运维文档和任务设计同步记录原因。

## 验收标准对应

| 标准 | 结果 | 证据 |
|---|---|---|
| AC-01 自动 required gate 与重复运行边界 | 通过 | 当前 PR 自动触发 `ci-gate` 并成功；`ci-change-scope` selftest 与 cloud-only 合同覆盖 docs-only 路径；未额外 dispatch CI。 |
| AC-02 main-only 手动 CI | 合同通过，未做真实 dispatch | `manual-dispatch-guard`、`change-scope` 和 aggregate contract/selftest 通过；仓库规则禁止为常规验证触发手动 CI。 |
| AC-03 独立 PR 标题门禁 | 部分完成 | 当前 `pr-title` 成功；Ruleset required context 更新是合并后仓库设置。 |
| AC-04 benchmark 分离 | 合同通过，未做真实 performance dispatch | `ci-change-scope` 的 manual 输出、`performance.yml` 合同与 selftest 通过；本 PR 非 benchmark 路径，Rust benchmark 正确跳过。 |
| AC-05 上游 App token 边界 | 合同通过，待 owner 配置 | sync policy checker/selftest 通过；GitHub App 安装、ID 和私钥由仓库 owner 合并后配置及实际验证。 |
| AC-06 Dependabot、CodeQL、pin 与 timeout | 部分完成 | CodeQL 两个 matrix job 实际成功；pin/timeout、quality 和 sync 合同通过；远端 SHA pinning 与 Dependabot alerts/security updates 待 owner 启用。 |
| AC-07 本地合同和 PR CI | 通过（实现候选） | 本地允许矩阵全部通过；同一 head 的 `ci-gate` 及相关 CI 已通过。交付记录提交后的最新 head 仍需 main 复验。 |

## 主要代码位置

| 文件或符号 | 变更 | 设计原因 |
|---|---|---|
| `.github/workflows/ci.yml` jobs `docs-contract` 至 `assemble-release-candidate` | 显式 dependency result 条件 | 避免 skipped manual guard 通过 GitHub 隐式 `success()` 传播并静默跳过必跑 job。 |
| `.github/workflows/codeql.yml` job `analyze` | Rust no-build matrix 与三步 action 流程 | CodeQL Rust 不支持 `autobuild`；删除不会执行的构建准备以减少耗时。 |
| `scripts/check-ci-quality-gates.mjs` | CI condition 与 CodeQL step-set 合同 | 防止后续 YAML 修改重引入隐式依赖传播或死 Autobuild step。 |
| `scripts/check-cloud-only-verification.mjs` | cloud-only CI graph 合同 | 让 docs/full CI 都能验证自动与手动门禁边界。 |
| `docs/operations/github-actions-governance.md` | 运行边界与合并后设置说明 | 持久化 skipped 传播和 Rust no-build 的运维知识。 |

## 与计划的偏移

- CodeQL 初始设计曾为 Rust 使用 `autobuild` 并准备系统依赖/工具链。真实 run 的 `Initialize CodeQL` 明确报错“Rust does not support the autobuild build mode”，因此改为官方建议的 `none`，并删除永久无用步骤。该偏移不改变用户行为、权限、触发范围或验收标准，已同步到 `design.md` 和运维文档。
- 未执行任何远端设置写入、GitHub App 创建、密钥配置或手动 workflow dispatch；这些步骤原本就被设计为合并后由仓库 owner 执行，且本 session 没有相应授权。

## 验证结果

### 本地检查

| 命令 | 结果 | 说明 |
|---|---|---|
| `node scripts/ci-change-scope.selftest.mjs` | 通过 | 路径分类与手动分支输出。 |
| `node scripts/check-cloud-only-verification.selftest.mjs && node scripts/check-cloud-only-verification.mjs` | 通过 | 手动 guard、CI graph、云端验证边界。 |
| `node scripts/check-ci-quality-gates.selftest.mjs && node scripts/check-ci-quality-gates.mjs` | 通过 | 质量门、CodeQL no-build 与 job 条件合同。 |
| `node scripts/check-sync-upstream-policy.selftest.mjs && node scripts/check-sync-upstream-policy.mjs` | 通过 | GitHub App token 的人工审查边界。 |
| `node scripts/check-github-actions-pin-policy.selftest.mjs && node scripts/check-github-actions-pin-policy.mjs` | 通过 | Action SHA pin 与 job timeout。 |
| `node --check`（4 个变更 Node 文件）与 `git diff --check` | 通过 | 语法、空白和补丁完整性。 |

未运行 `pnpm`、Cargo、依赖安装、构建或格式化，遵从仓库 cloud-only 规则；机器未安装 `actionlint`，YAML 最终语义由 GitHub Actions 验证。

### GitHub CI 与编译

| Workflow / Job | 结果 | 链接或说明 |
|---|---|---|
| `ci-gate` | 通过 | [run 31403797650](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31403797650)；aggregate job 4 秒。 |
| `frontend` | 通过 | [job 93505222031](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31403797650/job/93505222031)，5 分 49 秒。 |
| `rust` | 通过 | [job 93505222032](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31403797650/job/93505222032)，25 分 44 秒。 |
| `codeql (javascript-typescript)` | 通过 | [job 93505053496](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31403798071/job/93505053496)，1 分 57 秒。 |
| `codeql (rust)` | 通过 | [job 93505053413](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31403798071/job/93505053413)，3 分 10 秒。 |
| `pr-title` | 通过 | [job 93505054137](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31403798132/job/93505054137)，4 秒。 |
| candidate desktop/TUI/assemble | 按范围跳过 | PR 不是主仓 `main` push 或 main 手动候选构建。 |

### 人工验证

- 无。未因常规 PR 验证而触发 main-only `workflow_dispatch`、`performance` 或 `Sync Upstream`。

## 测试、文档与合同

- 测试：新增 CI 条件、candidate assembly 依赖和 CodeQL no-build 的负向 selftest。
- 现行文档：更新 `docs/operations/github-actions-governance.md`。
- 类型或机器合同：更新 cloud-only 与 CI quality gate checker 及其 selftest。
- 迁移或发布说明：不适用。

## 兼容性、风险与回滚

- 兼容性：无产品 API 或数据格式变更。
- 数据与配置：仓库内工作流和文档变更；不写入远端设置或密钥。
- 安全与隐私：CodeQL 仍只使用 `contents: read` 和 `security-events: write`；未新增凭据。
- 回滚方式：回退提交 `5d507fa497bd79c755c07a0e7711f3975197d4f5` 即可恢复此前工作流配置。
- 剩余风险：Rust CodeQL no-build 是 GitHub 支持的唯一路径，但会比构建型分析少依赖构建数据；初期保持 non-required，并继续以实际 PR 观察稳定性。

## 未完成项与阻塞

- 合并后由仓库 owner 依照 `docs/operations/github-actions-governance.md` 依次配置 GitHub App 凭据、SHA pinning、Dependabot alerts/security updates 和 Ruleset `ci-gate` + `pr-title` contexts；这些远端写入未获本次授权。
- 真实 main-only recovery、performance 和 upstream-sync 运行属于显式运维验证，不应由常规 PR 自动触发。

## 建议 main 重点审查

- `.github/workflows/ci.yml` 的 explicit `always()` 条件：确认条件图仍符合手动失败、PR full/docs-only 与候选制品边界。
- `.github/workflows/codeql.yml` 的 Rust `build-mode: none`：确认保持非 required，直到得到持续运行数据。
- 合并后远端设置：确认 Ruleset 不把 `manual-ci-gate` 设为 required，且凭据不写入仓库。

## main 验收记录

> 仅 main 填写。

尚未验收。

## main 收尾

> 仅 main 填写。

尚未合并，未归档或清理。

## 返工记录

无。
