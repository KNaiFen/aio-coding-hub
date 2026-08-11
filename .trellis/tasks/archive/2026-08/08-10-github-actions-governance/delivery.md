# 交付报告：GitHub Actions 流程治理与提速

> 功能实现已随 [#108](https://github.com/KNaiFen/aio-coding-hub/pull/108) 合并到 `main`，实际 merge commit 为 `82820b2ea10ec6028d1fcb8d130a993bfae39b6d`。本记录用于纠正任务终态；本次 Draft PR 仅包含记录，不是新的功能交付。任何记录提交都会产生新的文档 head，main 验收时仍须以 PR 实时 head 和实时检查为准。

## 交付状态

- 最终业务结果：部分完成并已拆分后续修复。
- 功能 PR：[#108](https://github.com/KNaiFen/aio-coding-hub/pull/108) 已合并到 `main`；实际 merge commit 为 `82820b2ea10ec6028d1fcb8d130a993bfae39b6d`。
- 功能分支：`chore/github-actions-governance`；远端跟踪分支已在合并后删除。
- 功能 PR base：`main` @ `9b05b28d5841584dc6f2a867947afd5d23f76246`。
- 历史功能交付候选 head：`09dfe0794522436c14e6bee278199ec6a5f9acfa`；历史 `ci-gate` 证据为[run 31451178867](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31451178867)。
- records-only closeout 分支：`docs/close-08-10-github-actions-governance`，基线为 `main` @ `82820b2ea10ec6028d1fcb8d130a993bfae39b6d`。
- records-only Draft PR：[#115](https://github.com/KNaiFen/aio-coding-hub/pull/115)。
- records-only 交付快照：本记录更新前的完整 head 为 `ebac0bb60745d15839dde2f0425aaac363550c8c`；[`ci-gate` run 31526356676](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31526356676) 与 [`pr-title` run 31526356680](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31526356680) 均通过。
- 本次交付记录提交会产生新的 PR head；执行 session 将等待该最新 head 的 `ci-gate` 与 `pr-title` 重新绿色后暂停，main 验收必须读取实时 PR 状态。不得把 #114 的任何交付证据写入此处。

## 阻塞快照

无。任务保持 `in_progress`，直至 main 验收并合并 records-only closeout PR 后决定归档；这不是功能开发阻塞。

## 最终业务结果

- #108 中的 CI 治理功能已经合并，merge commit 为 `82820b2ea10ec6028d1fcb8d130a993bfae39b6d`。
- 实际 Sync Upstream 运行发现的编号解析问题未被表述为 #108 已解决。本任务的最终业务结果因此为“部分完成并已拆分后续修复”。
- 该后续修复仅交叉引用到任务 ID `08-12-upstream-sync-pr-resolution` 和 [PR #114](https://github.com/KNaiFen/aio-coding-hub/pull/114)。本记录不复制其实现、head SHA、CI run 或验收结论。

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

下表保留 #108 功能交付时的证据快照；任务的当前终态以“最终业务结果”一节为准，records-only closeout PR 另行接受 `ci-gate` 与 `pr-title` 验证。

| 标准 | 结果 | 证据 |
|---|---|---|
| AC-01 自动 required gate、重复运行与分域边界 | 通过（共享路径实机；单域路径合同） | 当前 PR 修改 CI 控制面，按 fail-closed 规则实机运行 frontend 与 Rust 并由 `ci-gate` 成功汇总；`ci-change-scope`、quality-gate 和 cloud-only selftest 覆盖 frontend-only、Rust-only、shared、混合、rename/copy 与 docs 混合路径。未为验证而制造额外 PR/dispatch。 |
| AC-02 main-only 手动 CI | 合同通过，未做真实 dispatch | `manual-dispatch-guard`、`change-scope` 和 aggregate contract/selftest 通过；仓库规则禁止为常规验证触发手动 CI。 |
| AC-03 独立 PR 标题门禁 | 通过 | #108 的 `pr-title` 成功；合并后已核验 Ruleset required contexts 为 `ci-gate` 与 `pr-title`。 |
| AC-04 benchmark 分离 | 合同通过，未做真实 performance dispatch | `ci-change-scope` 的 manual 输出、`performance.yml` 合同与 selftest 通过；本 PR 非 benchmark 路径，Rust benchmark 正确跳过。 |
| AC-05 上游 App token 边界 | 部分完成并已拆分后续修复 | 凭据名称已在远端存在且未读取任何值；实际运行发现的编号解析问题交由 [PR #114](https://github.com/KNaiFen/aio-coding-hub/pull/114) 的独立 follow-up 处理，本记录不陈述其实施或验证事实。 |
| AC-06 Dependabot、CodeQL、pin 与 timeout | 通过 | CodeQL 两个 matrix job 实际成功；合并后已核验 Actions SHA pinning、Dependabot alerts/security updates 均已启用。 |
| AC-07 本地合同和 PR CI | #108 通过；records-only closeout 待验 | 同一 `09dfe079...` head 的 `ci-gate` 及相关 CI 已通过且 #108 已合并；本次 records-only PR 的最新 head 必须单独通过 `ci-gate` 与 `pr-title`。 |

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

- 后续修复：实际 Sync Upstream 运行发现的编号解析问题已拆分到任务 ID `08-12-upstream-sync-pr-resolution` / [PR #114](https://github.com/KNaiFen/aio-coding-hub/pull/114)。该链接不代表本任务拥有或验证其实施状态。
- 已核验的远端治理状态：Actions SHA pinning、Ruleset 的 `ci-gate` + `pr-title` required contexts、Dependabot alerts/security updates 均已启用；`SYNC_UPSTREAM_APP_ID` variable 和 `SYNC_UPSTREAM_APP_PRIVATE_KEY` secret 的名称存在。未读取或记录任何 secret 值。
- main 仍需验收、合并本 records-only closeout PR，并在合并后按任务规则决定归档和清理。本 execution session 不运行 `task.py archive`，不删除 worktree 或分支。

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

### Round 2 - 2026-08-12

- 审查候选：records-only PR [#115](https://github.com/KNaiFen/aio-coding-hub/pull/115) @ `6b52d775a5484c5d1272231fd649be684a833d2c`（base `82820b2ea10ec6028d1fcb8d130a993bfae39b6d`）。
- CI 证据：该 head 的 [`ci-gate` run 31527846703](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31527846703)、[`pr-title` run 31527846689](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31527846689) 与 CodeQL JS/TS/Rust 均成功；PR 为 Draft、OPEN、CLEAN。
- 审查范围：#108 实际 merge 事实、records-only 终态、任务元数据和活动索引；返工文件 `findings.md` 的 F-001；以及 PR diff 未扩展到 `.github/`、产品代码、同步脚本、测试合同或 `upgrade-tui.command`。
- F-001 复验：所有 08-10 正式记录已改用纯文本任务 ID `08-12-upstream-sync-pr-resolution`，删除了指向不存在目录的相对链接，仅保留 PR #114 的持久链接；返工记录已随同一候选提交。
- 结论：通过，准予合并 records-only closeout PR。该结论只认可 #108 的“部分完成并已拆分后续修复”终态，不认可或预先接受 PR #114 的实现、CI 或验收结果。
- 本 main 验收记录会产生新的文档 head；main 仅在该最新 head 的 required CI 重新绿色后执行合并和后续归档。

## main 收尾

> 仅 main 填写。

- 功能 PR [#108](https://github.com/KNaiFen/aio-coding-hub/pull/108) 已合并，实际 merge commit 为 `82820b2ea10ec6028d1fcb8d130a993bfae39b6d`。
- 本任务最终业务结果为“部分完成并已拆分后续修复”；后续项仅链接 [PR #114](https://github.com/KNaiFen/aio-coding-hub/pull/114)，不得将其实现或 CI 事实回写为 #108 证据。
- records-only closeout PR [#115](https://github.com/KNaiFen/aio-coding-hub/pull/115) 已于 2026-08-12 squash merge，merge commit 为 `585acf9c9367e3f1387494512609f5f86c41684a`。
- main 已于 2026-08-12 使用 `task.py archive --no-commit` 归档本任务，并运行 `task.py validate --all`；129 个 manifest 均通过。
- 归档变更将通过独立 archive PR 合并。其合并后，main 才会删除本 worktree 与已合并分支；不会清理仍活动的 PR #114 worktree。

## 返工记录

### Round 1 - F-001 执行回应

- 已将本任务正式记录中的错误任务 ID 改为纯文本 `08-12-upstream-sync-pr-resolution`，并删除所有指向未进入当前分支或 `main` 的任务目录相对链接。
- 后续修复仅保留 [PR #114](https://github.com/KNaiFen/aio-coding-hub/pull/114) 的持久链接；#108 merge commit、部分完成结论及“不继承 #114 实现、CI、验收事实”的边界保持不变。
- 本返工提交只修复 F-001；未改动 `main 验收记录` 或 `main 收尾`。推送后等待 PR #115 最新 head 的 `ci-gate` 与 `pr-title`。
