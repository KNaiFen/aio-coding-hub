# 交付报告：GitHub Actions 流程治理与提速

> 本交付记录冻结代码实现候选 `09dfe0794522436c14e6bee278199ec6a5f9acfa`。本文件本身提交后会产生新的文档 head，main 验收时必须以 PR 实时 head 和实时检查为准。

## 交付状态

- 结果：前序 PR 已合并；真实 Sync Upstream 运行发现新建 PR 编号解析缺陷，用户已确认后续修复，任务保持活动。
- PR：前序 [#108](https://github.com/KNaiFen/aio-coding-hub/pull/108) 已合并；修复 PR 待创建
- 分支：前序 `chore/github-actions-governance` 已合并；当前 `fix/upstream-sync-pr-resolution`
- PR base：`main` @ `9b05b28d5841584dc6f2a867947afd5d23f76246`
- 交付候选 head：`09dfe0794522436c14e6bee278199ec6a5f9acfa`
- 规划提交：`30a021269f3b6ae2c46f195faa273a1af81f26f9`
- `ci-gate`：通过，[run 31451178867](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31451178867)（job 93658916582）
- 其他检查：`pr-title`、`change-scope`、`docs-contract`、`support-contract`、`frontend`、`rust`、CodeQL JS/TS 和 Rust 均通过；候选制品 job 按 PR 条件跳过。
- 交付时间：2026-08-11T02:25:57Z
- 执行 session：正在提交本记录；提交后暂停写入，等待 main 验收。

## 阻塞快照

无。

## 合并后实际运行

- PR #108 已于 2026-08-11 squash merge，merge commit 为 `82820b2ea10ec6028d1fcb8d130a993bfae39b6d`。
- owner 已完成 GitHub App 安装以及 `SYNC_UPSTREAM_APP_ID` variable、`SYNC_UPSTREAM_APP_PRIVATE_KEY` secret 配置；本记录不保存其值或私钥。
- 手动 Sync Upstream [run 31487461146](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31487461146) 已成功完成凭据预检、短期 App token、checkout 与 fetch，且已创建 [PR #113](https://github.com/KNaiFen/aio-coding-hub/pull/113)。
- 该 run 的 `gh pr create` 输出 URL 后，立即 `gh pr list` 尚未返回新 PR，导致 `Failed to resolve sync PR number.`。PR #113 当前为 `DIRTY`/冲突状态；工作流没有 push、merge、自动批准或修改目标分支。
- 用户已确认修复：新建路径从创建返回 URL 严格解析编号，已有 PR 才使用 list；修复后对 #113 的回归运行应以清晰的人工冲突失败结束。

## 实现摘要

### 用户可见结果

- 自动 PR/push CI 不再因手动入口 guard 的正常 skipped 状态而跳过应执行的合同、前端和 Rust job；`ci-gate` 保持 fail-closed。
- CodeQL Rust 分支改用受支持的 no-build 模式，避免初始化阶段失败及无意义的 Rust 构建准备步骤。
- PR 现在按前端、Rust 与共享/未知变更域选择验证：纯前端改动不再等待 Rust job，纯 Rust 改动不再等待前端 job；跨层、依赖、CI 控制面与未知路径仍保守运行两端。

### 内部实现

- `.github/workflows/ci.yml` 为 docs、support、frontend、rust、candidate plan/build/assemble 显式加入 `always()` 与直接依赖成功条件，保留非 main 手动触发的早失败和候选制品边界。
- `.github/workflows/codeql.yml` 的 JS/TS 与 Rust matrix 均使用 `build-mode: none`；移除 Rust 系统依赖、工具链和 `autobuild` step。
- 两个 CI 合同检查器和 selftest 锁定上述依赖图与 CodeQL no-build step 集合，现行运维文档和任务设计同步记录原因。
- `.github/ci-scope.json`、`scripts/ci-change-scope.mjs` 与 `ci.yml` 增加 `frontend_ci`、`rust_ci`、`shared_ci` 输出；`ci-gate` 对每个选中域要求 success、对未选中域要求 expected skipped，避免一端被错误跳过或被误当成失败。

## 验收标准对应

| 标准 | 结果 | 证据 |
|---|---|---|
| AC-01 自动 required gate、重复运行与分域边界 | 通过（共享路径实机；单域路径合同） | 当前 PR 修改 CI 控制面，按 fail-closed 规则实机运行 frontend 与 Rust 并由 `ci-gate` 成功汇总；`ci-change-scope`、quality-gate 和 cloud-only selftest 覆盖 frontend-only、Rust-only、shared、混合、rename/copy 与 docs 混合路径。未为验证而制造额外 PR/dispatch。 |
| AC-02 main-only 手动 CI | 合同通过，未做真实 dispatch | `manual-dispatch-guard`、`change-scope` 和 aggregate contract/selftest 通过；仓库规则禁止为常规验证触发手动 CI。 |
| AC-03 独立 PR 标题门禁 | 部分完成 | 当前 `pr-title` 成功；Ruleset required context 更新是合并后仓库设置。 |
| AC-04 benchmark 分离 | 合同通过，未做真实 performance dispatch | `ci-change-scope` 的 manual 输出、`performance.yml` 合同与 selftest 通过；本 PR 非 benchmark 路径，Rust benchmark 正确跳过。 |
| AC-05 上游 App token 边界 | 合同通过，待 owner 配置 | sync policy checker/selftest 通过；GitHub App 安装、ID 和私钥由仓库 owner 合并后配置及实际验证。 |
| AC-06 Dependabot、CodeQL、pin 与 timeout | 部分完成 | CodeQL 两个 matrix job 实际成功；pin/timeout、quality 和 sync 合同通过；远端 SHA pinning 与 Dependabot alerts/security updates 待 owner 启用。 |
| AC-07 本地合同和 PR CI | 通过（实现候选） | 本地允许矩阵全部通过；同一 `09dfe079...` head 的 `ci-gate` 及相关 CI 已通过。交付记录提交后的最新 head 仍需 main 复验。 |

## 主要代码位置

| 文件或符号 | 变更 | 设计原因 |
|---|---|---|
| `.github/workflows/ci.yml` jobs `docs-contract` 至 `assemble-release-candidate` | 显式 dependency result 条件 | 避免 skipped manual guard 通过 GitHub 隐式 `success()` 传播并静默跳过必跑 job。 |
| `.github/workflows/codeql.yml` job `analyze` | Rust no-build matrix 与三步 action 流程 | CodeQL Rust 不支持 `autobuild`；删除不会执行的构建准备以减少耗时。 |
| `.github/ci-scope.json`、`scripts/ci-change-scope.mjs` | PR 前端/Rust/shared 分类与 fail-closed 默认值 | 将无关域从 PR 关键路径中移出，同时保证跨层与未知改动不会降级验证。 |
| `.github/workflows/ci.yml` jobs `change-scope`、`frontend`、`rust`、`ci-gate` | 分域输出、job 条件与聚合真值表 | 只运行被选中域，并严格断言选中 success、未选 skipped。 |
| `scripts/check-ci-quality-gates.mjs` | CI condition 与 CodeQL step-set 合同 | 防止后续 YAML 修改重引入隐式依赖传播或死 Autobuild step。 |
| `scripts/check-cloud-only-verification.mjs` | cloud-only CI graph 合同 | 让 docs/full CI 都能验证自动与手动门禁边界。 |
| `docs/operations/github-actions-governance.md` | 运行边界与合并后设置说明 | 持久化 skipped 传播和 Rust no-build 的运维知识。 |

## 与计划的偏移

- CodeQL 初始设计曾为 Rust 使用 `autobuild` 并准备系统依赖/工具链。真实 run 的 `Initialize CodeQL` 明确报错“Rust does not support the autobuild build mode”，因此改为官方建议的 `none`，并删除永久无用步骤。该偏移不改变用户行为、权限、触发范围或验收标准，已同步到 `design.md` 和运维文档。
- 2026-08-11 经用户确认，PR 分类由 docs/full 二元模型扩展为 frontend/Rust/shared 域模型。该扩展不改变 `dev`/`main` push、main 手动恢复、候选制品和未知路径的全量验证边界；已同步 PRD、设计、合同、自测和运维文档。
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
| `node --check`（6 个变更 Node 文件）与 `git diff --check` | 通过 | 语法、空白和补丁完整性。 |

未运行 `pnpm`、Cargo、依赖安装、构建或格式化，遵从仓库 cloud-only 规则；机器未安装 `actionlint`，YAML 最终语义由 GitHub Actions 验证。

### GitHub CI 与编译

| Workflow / Job | 结果 | 链接或说明 |
|---|---|---|
| `ci-gate` | 通过 | [run 31451178867](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31451178867)；aggregate job 3 秒。 |
| `change-scope` | 通过 | [job 93655692119](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31451178867/job/93655692119)，11 秒。 |
| `docs-contract` / `support-contract` | 通过 | [jobs](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31451178867)，分别 10 秒 / 9 秒。 |
| `frontend` | 通过 | [job 93655751873](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31451178867/job/93655751873)，5 分 40 秒。 |
| `rust` | 通过 | [job 93655751853](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31451178867/job/93655751853)，21 分 18 秒；其中 Rust tests 17 分 21 秒。 |
| `codeql (javascript-typescript)` | 通过 | [job 93655691877](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31451178858/job/93655691877)，1 分 39 秒。 |
| `codeql (rust)` | 通过 | [job 93655691831](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31451178858/job/93655691831)，3 分 14 秒。 |
| `pr-title` | 通过 | [job 93655691734](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31451178856/job/93655691734)，2 秒。 |
| candidate desktop/TUI/assemble | 按范围跳过 | PR 不是主仓 `main` push 或 main 手动候选构建。 |

### 人工验证

- 无。未因常规 PR 验证而触发 main-only `workflow_dispatch`、`performance` 或 `Sync Upstream`。

## 测试、文档与合同

- 测试：新增 CI 条件、candidate assembly 依赖、CodeQL no-build 与前端/Rust/shared 变更域（含跨域 rename/copy）的负向 selftest。
- 现行文档：更新 `docs/operations/github-actions-governance.md`。
- 类型或机器合同：更新 cloud-only 与 CI quality gate checker 及其 selftest。
- 迁移或发布说明：不适用。

## 兼容性、风险与回滚

- 兼容性：无产品 API 或数据格式变更。
- 数据与配置：仓库内工作流和文档变更；不写入远端设置或密钥。
- 安全与隐私：CodeQL 仍只使用 `contents: read` 和 `security-events: write`；未新增凭据。
- 回滚方式：回退实现提交 `09dfe0794522436c14e6bee278199ec6a5f9acfa` 即可撤销分域扩展；如需完全恢复改造前工作流，则回退该任务分支的 CI 治理提交序列。
- 剩余风险：Rust CodeQL no-build 是 GitHub 支持的唯一路径，但会比构建型分析少依赖构建数据；初期保持 non-required，并继续以实际 PR 观察稳定性。

## 未完成项与阻塞

- 合并后由仓库 owner 依照 `docs/operations/github-actions-governance.md` 依次配置 GitHub App 凭据、SHA pinning、Dependabot alerts/security updates 和 Ruleset `ci-gate` + `pr-title` contexts；这些远端写入未获本次授权。
- 真实 main-only recovery、performance 和 upstream-sync 运行属于显式运维验证，不应由常规 PR 自动触发。

## 建议 main 重点审查

- `.github/workflows/ci.yml` 的 explicit `always()` 条件：确认条件图仍符合手动失败、PR full/docs-only 与候选制品边界。
- `.github/ci-scope.json`、`scripts/ci-change-scope.mjs` 与 `ci-gate` 的分域组合：确认 frontend-only/Rust-only 分别将另一端视为 expected skipped，而 shared/未知路径保持全量。
- `.github/workflows/codeql.yml` 的 Rust `build-mode: none`：确认保持非 required，直到得到持续运行数据。
- 合并后远端设置：确认 Ruleset 不把 `manual-ci-gate` 设为 required，且凭据不写入仓库。

## main 验收记录

> 仅 main 填写。

### Round 1 - 2026-08-11

- 审查候选：`25ad19d0379b07c08cff06447779a539b5fd3460`（base `9b05b28d5841584dc6f2a867947afd5d23f76246`）。
- CI 证据：自动 required `ci-gate` 成功，[run 31452473387](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31452473387)（job `93662790640`）；`frontend`、`rust`、`pr-title`、CodeQL JS/TS/Rust、合同检查均为成功，候选构建 jobs 依 PR 条件跳过。PR 当前可合并且无 review/comment 阻塞。
- 审查范围：PRD/设计/交付与 `base..head` 的 33 个文件、路径分类和 `ci-gate` fail-closed 条件、main-only 手动 guard、候选制品边界、GitHub App token 权限/输入验证、CodeQL no-build、SHA pin/timeout 合同、最新远端检查与本地允许 Node 合同复验。
- 结论：通过，准予合并该功能 PR；未创建 `findings.md`。代码实现符合锁定范围，没有发现权限提升、密钥泄露、命令注入、自动门禁 fail-open 或候选制品边界回归。
- 接受的限制：AC-01/02/04 的真实单域与手动运行矩阵尚未执行；AC-03 的 `pr-title` Ruleset context、AC-05 的 App 凭据、AC-06 的 SHA pinning/Dependabot alerts/security updates 均按 `implement.md` 的合并后 owner 步骤保留。它们不是本 PR 代码合并的阻断项，任务归档前必须完成或由用户明确调整范围。
- 记录提交会产生新的仅交付文档 head；main 将在该 head 的 CI 重新绿色后合并，并在收尾记录实际 merge commit 和未完成 owner 项。

## main 收尾

> 仅 main 填写。

前序功能 PR #108 已合并为 `82820b2ea10ec6028d1fcb8d130a993bfae39b6d`。因 AC-05 的真实运行发现并已获授权修复的缺陷，任务尚未完成、不得归档或清理；待修复 PR 合并、真实回归运行和最终验收后再记录收尾。

## 返工记录

无。
