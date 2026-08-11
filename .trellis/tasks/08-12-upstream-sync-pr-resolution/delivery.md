# 交付报告：修复 Sync Upstream PR 编号解析与冲突收敛

> 本文件保留阶段 A 的任务归属迁移快照。2026-08-12 阶段 B 已将任务分支同步至实时 `origin/main`，其代码候选和自动检查证据见下文；main 验收仍待进行。

## 交付状态

- 结果：阶段 B 的主线同步、唯一 README 冲突收敛、生命周期更新和代码候选 CI 已完成；本交付记录提交后仍需对最新 records-only head 重核验自动检查，随后交由 main 审查。
- PR：[修复 #114](https://github.com/KNaiFen/aio-coding-hub/pull/114)（OPEN；由执行 session 在交付记录更新后转为 Ready for review）
- 分支：`fix/upstream-sync-pr-resolution`
- 历史 PR base：`main` @ `82820b2ea10ec6028d1fcb8d130a993bfae39b6d`；阶段 B 实时 PR base：`main` @ `366b92fd8429f5e747d77a632cbd0299522065de`
- 归属迁移前的远端 head 快照：`ed4a7527f75ea09ff55517afa3789babd0f922a6`
- 源规划提交：`2016c25ef7cb6ae524f3f2b4e86996ef923981a3`
- 阶段 B 代码候选：`7589a3a6fba5124a0325f4a7f97d0d1ebf713e07`（merge commit；第一父为交接 head `10743a0be67f7a39d7f7cd19c89635d27ba70ee7`，第二父为 `origin/main@366b92fd8429f5e747d77a632cbd0299522065de`）。
- `ci-gate`：通过，[run 31543180209](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31543180209)（head 为上述候选）。
- 其他检查：`pr-title` 通过，[run 31543180200](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31543180200)；CodeQL JS/TS 与 Rust 通过，[run 31543180213](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31543180213)（均绑定上述候选）。
- #113 回归：[Sync Upstream run 31508611251](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31508611251) 按预期失败，输出 PR #113 的 `DIRTY` 状态并要求人工冲突处理。
- 交付时间：2026-08-12（阶段 B 代码候选证据）；本记录的 records-only 提交由执行 session 推送后，继续等待其自动检查。
- 执行 session：阶段 B 的唯一写者；完成本记录、Ready for review 和最终自动检查复核后暂停，不执行 main 验收、合并或归档。

## 阶段 B 施工记录

- 开工前与 `git fetch origin` 后均确认：当前目录、分支、交接 head 和远端 PR head 相符；工作树只存在允许保留的未跟踪 `SESSION_REMEDIATION_PLAN.md`。该文件未读取、编辑、暂存或提交。
- 已运行 `python3 ./.trellis/scripts/task.py start .trellis/tasks/08-12-upstream-sync-pr-resolution`，任务状态从 `planning` 更新为 `in_progress`。环境没有 session identity，命令按 Trellis 文档以降级模式完成状态更新，未写入 runtime pointer。
- 已使用 `git merge --no-edit origin/main` 同步。唯一冲突为 `.trellis/tasks/README.md`；解决结果保留 `origin/main` 的 08-10、08-11 归档记录，并保留本任务 `08-12-upstream-sync-pr-resolution` 的活动行。未出现 README 之外的冲突。
- 未改动 `.github/workflows/sync-upstream.yml`、策略检查或 selftest；stdout 严格解析、`DIRTY`/`UNKNOWN`/空 merge state fail-closed，以及无 direct push、自动 merge、自动 approval 的边界保持不变。未处理 PR #113。
- 自动检查候选 `7589a3a6fba5124a0325f4a7f97d0d1ebf713e07` 的 `frontend`、`rust`、`change-scope`、`docs-contract`、`support-contract` 通过；`manual-dispatch-guard`、`candidate-plan` 与 release-candidate 条件任务按预期跳过。

## 阻塞快照

- 阶段 B 代码候选无代码或失败 CI 阻塞。交付记录的 records-only 提交会重新触发自动检查；该 head 绿色且未漂移后才可交由 main 验收。未手动 dispatch、推空提交或修改工作流。

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
| AC-04 不进入阶段 B | 阶段 A 交付项 | 未运行 `task.py start`；只推送本任务分支 |
| AC-05 主线同步与 README 冲突 | 通过 | merge commit `7589a3a6...` 的第二父为 `origin/main@366b92fd...`；唯一冲突按索引规则收敛 |
| AC-06 任务索引归属 | 通过 | `.trellis/tasks/README.md` 同时保留 08-10/08-11 归档和 08-12 活动行 |
| AC-07 同步候选与云端证据 | 代码候选通过；交付记录 head 待重核验 | run 31543180209、31543180200、31543180213 与候选 `7589a3a6...` 相符 |
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

- 阶段 B 代码候选 `7589a3a6...` 的自动检查已经通过：`ci-gate` / `pr-title` / CodeQL 与相关 CI job 的 run、head 和结论如本文件顶部及阶段 B 记录所述。交付记录提交后只观察其自动触发的检查，不手动触发工作流。

### 人工验证

- 无。未处理或合并 PR #113，未读取或执行任何凭据。

## 兼容性、风险与回滚

- 兼容性：无产品 API 或工作流行为改动。
- 数据与配置：无。
- 安全与隐私：不读取或记录 secret、App 私钥或 token；保持 no-push/no-merge/no-auto-approve 边界。
- 回滚方式：回退任务归属分离提交可恢复原有记录布局，但不应把 #114 重新归属到旧任务。
- 剩余风险：本交付记录的 records-only head 必须保持未漂移并完成自动检查，之后才可由 main 验收。

## 未完成项与阻塞

- main 验收、合并、收尾和归档仍未进行。执行 session 仅待本记录推送后的最新 head 自动检查为绿，然后暂停。

## 建议 main 重点审查

- 阶段 B merge commit `7589a3a6...`：确认第二父为 `origin/main@366b92fd...`，且 README 仅合并索引记录。
- 新任务目录与 `.trellis/tasks/README.md`：确认 PR #114、分支、base、worktree 和唯一写者仅指向本包，并保留 08-10/08-11 归档。
- `.trellis/tasks/08-10-github-actions-governance/`：确认相对 `origin/main` 没有 #114 差异。
- `.github/workflows/sync-upstream.yml` 与 policy contract：阶段 B 未改动；严格 stdout 解析和 fail-closed 边界保持不变。

## 阶段 B 授权（main）

- 授权日期：2026-08-12。
- 授权范围：同步 `fix/upstream-sync-pr-resolution` 到开始施工时最新 `origin/main`，解决预期的 `.trellis/tasks/README.md` 冲突，更新 Trellis 生命周期和交付/CI 证据。
- 阶段 A 已知快照仅作历史背景：`origin/main@9aa8e4ab8e6417be4816b0811178c3f401e34171`、PR #114 旧 head `6316204274eeb6db9332b4eef0e5f182c5c31ca7`。阶段 B 实际同步源为 `origin/main@366b92fd8429f5e747d77a632cbd0299522065de`，代码候选为 `7589a3a6fba5124a0325f4a7f97d0d1ebf713e07`。
- 锁定边界：保留 #113 的 fail-closed 人工处理路径；不得修改 stdout 严格解析、放宽 `DIRTY`/`UNKNOWN`/空状态处理、读取或处理 `upgrade-tui.command`、读取或提交 `SESSION_REMEDIATION_PLAN.md`，也不得合并 PR 或推送 `main`。
- 阶段 B 交接条件已满足：代码候选自动检查为绿且 PR head 无漂移。交付记录推送后的 records-only head 完成同等自动检查后，执行 session 转 Ready for review 并暂停；main 再进行验收。

## main 验收记录

> 仅 main 填写。

## main 收尾

> 仅 main 填写。任务保持活动，未合并、未归档、未删除 worktree 或分支。

## 返工记录

无。
