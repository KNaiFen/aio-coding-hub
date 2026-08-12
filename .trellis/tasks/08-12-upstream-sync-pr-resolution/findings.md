# 验收整改记录：修复 Sync Upstream PR 编号解析与冲突收敛

## Round 1

- 候选 head：`880196484bb291754b783d8cd7de3b5ca588f24e`
- 日期：2026-08-12
- 结论：代码、workflow/policy 和自动 CI 通过；交付记录与活动索引未绑定冻结 head，暂不通过 main 验收。
- 当前唯一写者：执行 session 已暂停；整改期间由执行 session 按本文件继续，完成后重新推送并暂停。main 保留验收、合并和收尾权限。

### F-001（必须整改）：交付证据必须绑定最新 PR head

- 严重程度：重要
- 证据：`delivery.md:13-16,26,30,52-55,72-74,86,90` 仍把 `7589a3a6fba5124a0325f4a7f97d0d1ebf713e07` 作为阶段 B 候选，并引用旧的 `ci-gate`/`pr-title`/CodeQL runs；同一记录还写着 records-only head 尚待重核验。
- 当前事实：PR #114 最新 head 为 `880196484bb291754b783d8cd7de3b5ca588f24e`，base 为 `366b92fd8429f5e747d77a632cbd0299522065de`。该 head 的 `ci-gate` run `31544998029`、`pr-title` run `31544998027`、CodeQL run `31544998030` 及相关检查均已成功。
- 影响：`delivery.md` 没有满足“完整 PR head、base 和对应 ci-gate 必须属于本次交付候选”的交付合同，main 无法仅凭任务材料确认最终候选。
- 期望结果：将当前交付状态、候选 SHA、base、ci-gate/pr-title/CodeQL/相关检查和验证表全部更新为 `880196484bb291754b783d8cd7de3b5ca588f24e`；旧 `7589a3a6...` 只能保留为阶段 B 历史快照，并明确不是最终交付证据。删除“records-only head 待重核验/等待自动检查”的过期阻塞表述。
- 复验方式：执行 session 推送新提交后，main 重新查询 PR #114 的完整 head 与所有 required checks，确认新 head 未漂移且 `gh pr checks 114 --required` 全部通过；delivery 顶部和验收标准 AC-07/验证结果必须引用同一 head 和 ci-gate run。

**执行回应**

- 修改：已将 `7589a3a6fba5124a0325f4a7f97d0d1ebf713e07` 降为阶段 B 历史 merge 快照，并明确其不是最终交付证据；已删除旧 records-only head 待重核验/等待自动检查的过期表述。本轮提交后的最终 head、当前 PR base、`ci-gate`、`pr-title`、CodeQL、相关检查和验证结果将在自动检查终态后统一绑定。
- 代码位置：不涉及产品代码；变更仅在本任务 `delivery.md` 的交付状态、验收标准、验证结果、授权状态和返工记录。
- 验证：整改前冻结 head `e9ad1971dff00d3b563f05877ef841c70988d8d5` 的 `ci-gate` run `31574818709`、`pr-title` run `31574818763`、CodeQL run `31574818848` 及 frontend、rust、change-scope、docs-contract、support-contract 均成功；`gh pr checks 114 --required` 通过。本轮最终 head 的对应检查待推送后核验。
- 计划偏移：无；旧 merge 快照仅保留为历史同步证据，未改变产品、workflow 或 policy/selftest 行为。

### F-002（必须整改）：同步任务索引与施工入口的当前状态

- 严重程度：重要
- 证据：`.trellis/tasks/README.md:13` 仍写 PR 为 Draft、阶段为“阶段 B 已授权，先解决 README 冲突”、唯一写者为“main（交接中）；交接后一个执行 session”；`execution.md:15,18-19` 同样保留 Draft、main 交接中和阶段 B 开工前状态。
- 当前事实：PR #114 已 `isDraft=false`、`OPEN`、`CLEAN/MERGEABLE`；阶段 B 已完成，执行 session 已暂停，当前应进入 main 验收。
- 影响：活动索引和施工入口不能作为当前生命周期事实源，可能导致错误恢复旧启动流程或误判唯一写者。
- 期望结果：将 README 活动行和 `execution.md` 快速定位/阶段说明更新为“阶段 B 完成，待 main 验收”；PR 标为 Ready for review；执行 session 标明已暂停，main 负责验收、合并和收尾。`task.json.status` 保持 `in_progress`，直到 main 完成归档，不要提前伪造 completed。
- 复验方式：main 读取更新后的任务文件，核对其 PR/head/base/worktree 与 GitHub 实时状态一致；确认执行 session 再次暂停、工作树仅保留既有未跟踪 `SESSION_REMEDIATION_PLAN.md`，随后才进入合并决策。

**执行回应**

- 修改：将 `.trellis/tasks/README.md` 和 `execution.md` 更新为“阶段 B 完成，待 main 验收”；PR #114 标为 Ready for review；明确本轮最终交付绑定完成后执行 session 暂停，由 main 负责验收、合并和收尾。`task.json` 保持 `status: in_progress`，未提前归档。
- 代码位置：`.trellis/tasks/README.md:13`、`.trellis/tasks/08-12-upstream-sync-pr-resolution/execution.md:3,15,18-19,45,53`；不涉及产品代码。
- 验证：`gh pr view 114` 显示 `isDraft=false`、`OPEN`、base `366b92fd8429f5e747d77a632cbd0299522065de`；Trellis validate 通过；`task.json` 仍为 `in_progress`；最终工作树状态将在交付绑定完成后复核。
- 计划偏移：无；未修改 main 的原始 findings、`delivery.md` 的 main 验收记录或 main 收尾区块。

### 必须保持不变

- 不修改产品或既有 Sync Upstream 行为；不得改变 stdout 严格解析、受限 PR 查询、`DIRTY`/`UNKNOWN`/空 merge state fail-closed 合同。
- 不读取、编辑、暂存、删除或提交 `SESSION_REMEDIATION_PLAN.md`；不触碰 `upgrade-tui.command`。
- 不处理 PR #113，不运行手动 workflow dispatch，不推空提交，不合并 PR、不启用 auto-merge、不推送 `main`，不归档或删除 worktree。

### Main 复验门

整改 session 完成后必须：

1. 推送新 head 并将 `delivery.md` 更新到该完整 SHA。
2. 等待该新 head 的自动 `ci-gate`、`pr-title`、CodeQL 和相关检查全部终态成功。
3. 将 PR 保持 Ready for review，暂停写入，并报告完整 head、CI 链接、变更文件和最终 `git status`。

### Round 2 执行 session 交付状态

- 整改前冻结 head：`e9ad1971dff00d3b563f05877ef841c70988d8d5`；PR base：`366b92fd8429f5e747d77a632cbd0299522065de`。
- 该 head 的自动检查已终态成功；本轮提交后的最终 head 仍需绑定对应 `ci-gate`、`pr-title`、CodeQL 和相关检查。PR 保持 Ready for review，完成绑定后执行 session 暂停并等待 main 复验。
- 变更文件限于本任务的 `delivery.md`、`execution.md`、`.trellis/tasks/README.md` 和本文件；未读取、编辑、暂存、删除或提交 `SESSION_REMEDIATION_PLAN.md`，未触碰 `upgrade-tui.command`。
