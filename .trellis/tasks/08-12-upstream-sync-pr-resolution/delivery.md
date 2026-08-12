# 交付报告：修复 Sync Upstream PR 编号解析与冲突收敛

> 本文件记录阶段 B 同步、返工整改和当前冻结交付候选。旧阶段 B merge commit 仅作历史快照；main 的验收记录保留在下方，当前候选以 PR 实时 head 和对应检查为准。

## 交付状态

- 结果：阶段 B 的主线同步、唯一 README 冲突收敛、F-001/F-002 记录整改和生命周期更新已完成；交付候选的自动检查已通过，待 main 验收。
- PR：[修复 #114](https://github.com/KNaiFen/aio-coding-hub/pull/114)（OPEN，Ready for review）
- 分支：`fix/upstream-sync-pr-resolution`
- 历史 PR base：`main` @ `82820b2ea10ec6028d1fcb8d130a993bfae39b6d`；阶段 B 实时 PR base：`main` @ `366b92fd8429f5e747d77a632cbd0299522065de`
- 归属迁移前的远端 head 快照：`ed4a7527f75ea09ff55517afa3789babd0f922a6`
- 源规划提交：`2016c25ef7cb6ae524f3f2b4e86996ef923981a3`
- 阶段 B 历史 merge 快照：`7589a3a6fba5124a0325f4a7f97d0d1ebf713e07`（第一父为交接 head `10743a0be67f7a39d7f7cd19c89635d27ba70ee7`，第二父为 `origin/main@366b92fd8429f5e747d77a632cbd0299522065de`）；该提交不是最终交付候选。
- 交付候选 head：`8005a625faf99d74f053947b9f3d77ac31a02838`；PR base：`main` @ `366b92fd8429f5e747d77a632cbd0299522065de`。
- `ci-gate`：通过，[run 31578042883](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31578042883)（`ci-gate` job [94060039820](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31578042883/job/94060039820)，绑定上述交付候选）。
- 其他检查：`pr-title` 通过，[run 31578042866](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31578042866)；CodeQL JS/TS 与 Rust 通过，[run 31578042790](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31578042790)；frontend、rust、change-scope、docs-contract、support-contract 均通过，candidate-plan、manual-dispatch-guard 和 release-candidate jobs 按范围跳过（均绑定上述交付候选）。
- #113 回归：[Sync Upstream run 31508611251](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31508611251) 按预期失败，输出 PR #113 的 `DIRTY` 状态并要求人工冲突处理。
- 交付时间：2026-08-12（交付候选 `8005a625…` 的检查已终态成功）。
- 执行 session：阶段 B 和 F-001/F-002 整改已完成；PR 已 Ready for review；执行 session 已暂停，main 负责验收、合并和收尾。

## 阶段 B 施工记录

- 开工前与 `git fetch origin` 后均确认：当前目录、分支、交接 head 和远端 PR head 相符；工作树只存在允许保留的未跟踪 `SESSION_REMEDIATION_PLAN.md`。该文件未读取、编辑、暂存或提交。
- 已运行 `python3 ./.trellis/scripts/task.py start .trellis/tasks/08-12-upstream-sync-pr-resolution`，任务状态从 `planning` 更新为 `in_progress`。环境没有 session identity，命令按 Trellis 文档以降级模式完成状态更新，未写入 runtime pointer。
- 已使用 `git merge --no-edit origin/main` 同步。唯一冲突为 `.trellis/tasks/README.md`；解决结果保留 `origin/main` 的 08-10、08-11 归档记录，并保留本任务 `08-12-upstream-sync-pr-resolution` 的活动行。未出现 README 之外的冲突。
- 未改动 `.github/workflows/sync-upstream.yml`、策略检查或 selftest；stdout 严格解析、`DIRTY`/`UNKNOWN`/空 merge state fail-closed，以及无 direct push、自动 merge、自动 approval 的边界保持不变。未处理 PR #113。
- 交付候选 `8005a625faf99d74f053947b9f3d77ac31a02838` 的 `frontend`、`rust`、`change-scope`、`docs-contract`、`support-contract` 通过；`manual-dispatch-guard`、`candidate-plan` 与 release-candidate 条件任务按预期跳过。`ci-gate`、`pr-title`、CodeQL 均绑定该完整 head。

## 阻塞快照

- 无。交付候选的自动检查均已终态成功，PR head 未漂移；未手动 dispatch、推空提交或修改工作流。

## 实现摘要

### 用户可见结果

- 不改变 Sync Upstream 的运行行为。任务与交付事实从 #108 的旧治理任务中分离，使 PR #114 的审查入口不再混淆。

### 内部实现

- `.github/workflows/sync-upstream.yml` 的既有修复在新建 PR 路径严格解析 `gh pr create` stdout；已有 PR 才运行受限 list 查询。
- `scripts/check-sync-upstream-policy.mjs` 与 selftest 继续锁定 URL、正整数编号、无 push/merge/approval 以及 `DIRTY`/`UNKNOWN` fail-closed 合同。
- 本阶段创建独立 Trellis 包，并将旧任务包的 #114 时期材料恢复至 `origin/main`。

## 验收标准对应

| 标准 | 结果 | 证据 |
|---|---|---|
| AC-01 任务身份一致 | 阶段 A 交付项 | 新 `task.json`、`execution.md`、本文件与 `.trellis/tasks/README.md` |
| AC-02 旧任务无 #114 差异 | 阶段 A 交付项 | `git diff --name-only origin/main...HEAD -- .trellis/tasks/08-10-github-actions-governance` 无输出 |
| AC-03 #114 证据归属 | 阶段 A 快照 | 上列 `ed4a7527...` 的 `ci-gate`、`pr-title`、CodeQL 和 run `31508611251` |
| AC-04 不进入阶段 B | 阶段 A 交付项 | 阶段 A 未运行 `task.py start`；阶段 B 已按授权只推送本任务分支 |
| AC-05 主线同步与 README 冲突 | 通过 | merge commit `7589a3a6...` 的第二父为 `origin/main@366b92fd...`；唯一冲突按索引规则收敛 |
| AC-06 任务索引归属 | 通过 | `.trellis/tasks/README.md` 同时保留 08-10/08-11 归档和 08-12 活动行 |
| AC-07 同步候选与云端证据 | 通过，待 main 验收 | 交付候选 `8005a625faf99d74f053947b9f3d77ac31a02838`；`ci-gate` run 31578042883、`pr-title` run 31578042866、CodeQL run 31578042790 及相关检查均绑定该 head |
| AC-08 工作流边界和本地合同 | 通过 | 下列 Node 合同/selftest、Trellis 验证与差异检查均通过；未改 workflow/policy/selftest |

`cdc427b9c6b386ca6106a371880710155704a81e` / run `31506469918` 仅是历史候选背景，不能作为本任务的最终交付候选或最终 CI 证据。

## 验证结果

### 本地检查

| 命令 | 结果 | 说明 |
|---|---|---|
| `node --check scripts/check-sync-upstream-policy.mjs` 与 selftest | 通过 | 语法解析通过 |
| `node scripts/check-sync-upstream-policy.mjs` | 通过 | 输出 `Sync upstream manual-review policy check passed.` |
| `node scripts/check-sync-upstream-policy.selftest.mjs` | 通过 | 输出 `Sync upstream manual-review policy self-test passed.` |
| `python3 ./.trellis/scripts/task.py validate 08-12-upstream-sync-pr-resolution` | 通过 | 任务 manifest 验证通过；无 context manifest 时按工具约定跳过 |
| `git diff --check` 与 `git diff --cached --check` | 通过 | 无空白错误 |
| `git diff --name-only origin/main...HEAD -- .trellis/tasks/08-10-github-actions-governance` | 通过（无输出） | 阶段 B 合并后确认旧任务目录仍无 PR #114 差异 |

### GitHub CI 与编译

- 交付候选 `8005a625faf99d74f053947b9f3d77ac31a02838` 的自动检查已经通过：`ci-gate` / `pr-title` / CodeQL 与相关 CI job 的 run、head 和结论如本文件顶部及阶段 B 记录所述。未手动触发工作流。

### 人工验证

- 无。未处理或合并 PR #113，未读取或执行任何凭据。

## 兼容性、风险与回滚

- 兼容性：无产品 API 或工作流行为改动。
- 数据与配置：无。
- 安全与隐私：不读取或记录 secret、App 私钥或 token；保持 no-push/no-merge/no-auto-approve 边界。
- 回滚方式：回退任务归属分离提交可恢复原有记录布局，但不应把 #114 重新归属到旧任务。
- 剩余风险：无自动检查阻塞；main 仍需按交付候选和 PR 实时状态审查需求、范围与本轮整改。

## 未完成项与阻塞

- main 验收、合并和收尾仍未进行；任务保持 `in_progress`，执行 session 已暂停。

## 建议 main 重点审查

- 阶段 B merge commit `7589a3a6...`：确认第二父为 `origin/main@366b92fd...`，且 README 仅合并索引记录。
- 新任务目录与 `.trellis/tasks/README.md`：确认 PR #114、分支、base、worktree 和唯一写者仅指向本包，并保留 08-10/08-11 归档。
- `.trellis/tasks/08-10-github-actions-governance/`：确认相对 `origin/main` 没有 #114 差异。
- `.github/workflows/sync-upstream.yml` 与 policy contract：阶段 B 未改动；严格 stdout 解析和 fail-closed 边界保持不变。

## 阶段 B 授权（main）

- 授权日期：2026-08-12。
- 授权范围：同步 `fix/upstream-sync-pr-resolution` 到开始施工时最新 `origin/main`，解决预期的 `.trellis/tasks/README.md` 冲突，更新 Trellis 生命周期和交付/CI 证据。
- 阶段 A 已知快照仅作历史背景：`origin/main@9aa8e4ab8e6417be4816b0811178c3f401e34171`、PR #114 旧 head `6316204274eeb6db9332b4eef0e5f182c5c31ca7`。阶段 B 实际同步源为 `origin/main@366b92fd8429f5e747d77a632cbd0299522065de`；merge commit `7589a3a6fba5124a0325f4a7f97d0d1ebf713e07` 仅为历史同步快照，不是最终交付候选。
- 锁定边界：保留 #113 的 fail-closed 人工处理路径；不得修改 stdout 严格解析、放宽 `DIRTY`/`UNKNOWN`/空状态处理、读取或处理 `upgrade-tui.command`、读取或提交 `SESSION_REMEDIATION_PLAN.md`，也不得合并 PR 或推送 `main`。
- 阶段 B 交付条件已满足：交付候选 `8005a625faf99d74f053947b9f3d77ac31a02838` 未漂移，自动 `ci-gate`、`pr-title`、CodeQL 和相关检查均为成功；PR 已 Ready for review，执行 session 已暂停，由 main 验收、合并和收尾。

## main 验收记录

> 仅 main 填写。

### Round 1

- 结论：需要整改。
- 审查范围：阶段 B 同步提交、README 冲突收敛、任务归属与生命周期记录、最新 PR diff、workflow/policy 合同和冻结候选的云端检查。
- 审查候选 head：`880196484bb291754b783d8cd7de3b5ca588f24e`（PR base `366b92fd8429f5e747d77a632cbd0299522065de`）。
- `ci-gate`：通过，[run 31544998029](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31544998029)；同一 head 的 `pr-title` [run 31544998027](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31544998027)、CodeQL [run 31544998030](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31544998030)、frontend、rust、合同检查均成功。
- 通过项：`7589a3a6...` 的第二父正确为 `origin/main@366b92fd...`；唯一 README 冲突保留 08-10/08-11 归档与 08-12 活动行；本轮同步未新增 workflow/policy 语义，既有 stdout 严格解析与 `DIRTY`/`UNKNOWN`/空 merge state fail-closed 合同保持有效。
- 未通过项：`delivery.md` 仍将最终证据绑定到旧代码候选 `7589a3a6...`，并明确 records-only head 待重核验；活动索引和 `execution.md` 仍写 Draft、阶段 B 开工前及 main 交接中状态。详细可执行整改见 `findings.md` 的 F-001、F-002。
- 接受的偏移或风险：无。代码和 CI 绿色不替代最终交付记录与生命周期事实的同步。
- 日期：2026-08-12。

### Round 2

- 结论：通过。F-001、F-002 均已解决；允许进入验收记录提交的自动 CI 与合并前复核。
- 审查范围：Round 1 findings 的执行回应、阶段 B 同步与 README 冲突收敛、任务归属和生命周期、PR 相对 base 的完整 diff、workflow/policy/selftest 合同、最新冻结 head 的实时 PR 状态与自动检查。
- 审查候选 head：`76d13b5ee2b2d227e6468cf7d349ff13e2901f14`（PR base `366b92fd8429f5e747d77a632cbd0299522065de`，OPEN、Ready for review、CLEAN/MERGEABLE）。
- 交付证据分层：`8005a625faf99d74f053947b9f3d77ac31a02838` 是执行 session 完成 F-001/F-002 整改的交付候选，对应 `ci-gate` run `31578042883`、`pr-title` run `31578042866` 和 CodeQL run `31578042790`；其后 `76d13b5...` 只更新 `delivery.md`、`execution.md`、`findings.md` 以绑定该交付证据，没有产品、workflow、policy 或 selftest 变化。
- 最新检查：`76d13b5...` 的 `ci-gate` 通过，[run 31580090972](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31580090972)；同一 head 的 `pr-title` [run 31580090988](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31580090988)、CodeQL [run 31580090954](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31580090954)、frontend、rust、change-scope、docs-contract、support-contract 均成功。仓库严格必需检查 `ci-gate`、`pr-title` 均通过。
- 复验结论：F-001 已解决，交付候选、base 与 CI 证据一致，阶段 B merge commit `7589a3a6...` 仅保留为历史同步快照；F-002 已解决，受验 head 的 README 与施工入口均显示阶段 B 完成、执行 session 暂停、main 接管验收，`task.json.status` 正确保留为 `in_progress`。
- 范围与合同：阶段 B 同步提交的第二父仍为 `origin/main@366b92fd...`，唯一冲突仍限于 `.trellis/tasks/README.md`；08-10/08-11 归档与 08-12 活动行均保留。允许的 Node policy/selftest、Trellis validate、旧 08-10 目录差异检查和 `git diff --check` 通过；未运行本地依赖安装、pnpm、Cargo、构建、格式化或产品测试。
- 接受的偏移或风险：接受 `8005a625...` 交付候选与其后纯记录 head `76d13b5...` 的两层证据；这是提交内容不能自引用其自身 SHA 的记录边界，不改变被验收实现。写入本轮记录会产生一个新的纯记录 head；main 只在该 head 的自动检查全部成功、相对 `76d13b5...` 仍仅为本轮验收/生命周期记录且 PR head 未漂移时合并，实际合并 head 与 merge commit 在 records-only 收尾 PR 中落盘。
- 日期：2026-08-12。

## main 收尾

> 仅 main 填写。任务保持活动，未合并、未归档、未删除 worktree 或分支。

## 返工记录

### Round 2 - F-001/F-002 执行回应

- 返工交付候选 head：`8005a625faf99d74f053947b9f3d77ac31a02838`
- PR base：`main` @ `366b92fd8429f5e747d77a632cbd0299522065de`
- `ci-gate`：通过，[run 31578042883](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31578042883)
- 其他检查：`pr-title` [run 31578042866](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31578042866)、CodeQL [run 31578042790](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31578042790)、frontend、rust、change-scope、docs-contract、support-contract 均成功；条件性 candidate/release jobs 按范围跳过。
- 修改内容：仅更新本交付记录中的阶段 B 当前 head/base/CI 绑定、验证结果和生命周期说明；保留阶段 B merge commit `7589a3a6…` 作为历史同步快照，不再作为最终交付候选；未修改 main 验收记录或 main 收尾。
- 验证证据：允许的 Node policy/selftest、Trellis validate、`git diff --check` 与旧任务目录差异检查通过；PR 为 Ready for review，task.json 保持 `in_progress`。
- 尚未解决：无；main 仍需完成验收、合并和收尾。
