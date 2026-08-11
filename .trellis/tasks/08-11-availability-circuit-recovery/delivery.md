# 交付报告：可用性探测熔断恢复

> 本文件记录 Round 2 的 F-002 测试返工；功能候选已通过自动 PR CI，等待 main 再次验收。

## 交付状态

- 结果：Round 2 返工候选已通过自动 PR CI，等待 main 再次验收
- PR：[#109](https://github.com/KNaiFen/aio-coding-hub/pull/109)
- 分支：`fix/availability-circuit-recovery`
- PR base：`main` @ `82820b2ea10ec6028d1fcb8d130a993bfae39b6d`（初始规划 base 为 `9b05b28d5841584dc6f2a867947afd5d23f76246`）
- 交付候选 head（功能实现）：`9baeeb72522dff47178f0ff6675bc1904dd39f84`（冻结起点为 `36ef9df65b7d6eeb22eb3a19ecbf892e39194a02`）
- 规划提交：`7de765738df6a0be4a31309a0f0c1a28852f1657`
- `ci-gate`：通过，[job 93873788290](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31517935633/job/93873788290)
- 其他必需检查：`rust`、`pr-title`、`change-scope`、`support-contract` 与 CodeQL 通过；`frontend` 按 Rust-only scope 预期跳过
- 交付时间：2026-08-12
- 执行 session：Round 2 返工完成，等待 main 再次验收

## 阻塞快照

无。

## 实现摘要

### 用户可见结果

- Open 未到期时，手动和定时可用性探测仍记录观测，但不会提前恢复熔断。
- Open 到期后的探测只能作为 HalfOpen 恢复证据；连续三次成功后沿用既有恢复事件关闭熔断，失败则立即重新 Open。
- 定时成功且写入后仍为 HalfOpen 时，约 30 秒后通过现有调度器安排一次补测；使熔断关闭的第三次成功不会排补测。

### 内部实现

- 探测 flight 在 `finish_probe` 中仅写入一次熔断证据；私有 `RecoveryDirective` 将实际 completion 时刻计算出的 due 传给合并等待者，只有 scheduled consumer 会使用它入队。
- 补测 target 使用既有 scheduler、generation、四槽 limiter、in-flight 合并和过期宽限，并增加私有 Pending/Claimed 生命周期与 recovery epoch。有效失败重新 Open 或成功关闭电路都会撤销旧 target；禁用、删除、revision/generation 变化、完整扫描缺失与休眠过期同样会取消它。
- 只有正在运行的 Gateway 才接受熔断证据或执行 HalfOpen 检查，Gateway 未运行时不写离线持久化熔断状态。

## 验收标准对应

| 标准 | 结果 | 证据 |
|---|---|---|
| AC-01 Open 未到期不提前恢复 | 通过 | `GatewayRuntime::record_availability_probe_outcome` 保留 Closed/Open 语义；最新候选未改变该路径。 |
| AC-02 到期后 HalfOpen 三次成功关闭 | 通过 | `record_availability_probe_outcome` 仍仅在 `should_allow` 后为 HalfOpen 时记录结果。 |
| AC-03 HalfOpen 失败立即重开 | 通过 | `update_recovery_work_after_circuit_evidence` 使有效失败同步撤销 Pending/Claimed recovery；新增确定性回归覆盖。 |
| AC-04 HalfOpen 成功后 30 秒补测 | 通过 | `RecoveryDirective::from_completion` 固定 `due_at_ms = completed_at_ms + 30_000`；新增 completion/waiter 延迟回归覆盖。 |
| AC-05 无重复、过期或无用补测 | 通过 | Claimed target 的 generation/epoch/due 在 `begin_probe_with_recovery` 创建或加入 in-flight 前原子复核；失效 target 返回 Stale。 |
| AC-06 Gateway 停止无副作用且合同不变 | 通过 | Gateway 守卫和公开合同未改动。 |
| AC-07 后端验证、编译和 ci-gate | 通过 | [自动 PR ci run 31511661925](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31511661925) 绑定候选 `36ef9df6…`。 |

## 主要代码位置

| 文件或符号 | 变更 | 设计原因 |
|---|---|---|
| `src-tauri/src/gateway/runtime.rs:GatewayRuntime::record_availability_probe_outcome` | 将可用性 probe 限定为 HalfOpen 恢复证据，并保留 Open 未到期和 Closed 的既有状态。 | 熔断状态机及既有恢复事件均由 Gateway 所有。 |
| `src-tauri/src/app/provider_availability_probe_runtime.rs:ProviderAvailabilityProbeRuntimeState::finish_probe` | 单一实际 HTTP flight 写一次观测和熔断证据，并在 completion 时刻生成私有 recovery directive。 | 防止合并等待者重复计数或用 waiter 恢复时刻延后 due。 |
| `src-tauri/src/app/provider_availability_probe_runtime.rs:begin_probe_with_recovery`、`recovery_claim_matches` | 在同一 mutation gate/inner 临界区复核 Claimed target 的 generation、epoch 和 due。 | 失效 target 不能在 scheduler 预检后创建或加入 in-flight。 |
| `src-tauri/src/app/provider_availability_probe_runtime.rs:update_recovery_work_after_circuit_evidence`、`take_due_recovery_targets`、`settle_recovery_target` | 用 Pending/Claimed 生命周期管理补测，并在失败重开或成功关闭时撤销旧链。 | 保持既有 scheduler 的并发、失效和休眠语义。 |

## 与计划的偏移

- 无产品语义偏移。
- 首轮云端格式化发现一个多余的 `else` 语法分支；已删除。返工随后经过两轮仅含云端 artifact 的 Rust 格式化补丁；最终候选的格式化与绑定生成步骤通过且无漂移。
- 为跟进最新 `main`，PR 已 rebase 到 `82820b2e`；仅在任务索引发生文本冲突，保留双方活动任务条目。随后以 `d68050e6` 修正 rebase 后规划提交的完整 SHA；未引入产品代码改动。
- 手动触发的 run `31493759164` 按最新主线规则仅允许 `main` 分支而被拒绝，未作为交付证据；自动 PR run `31495975865` 是此前候选的历史验证，最终候选以自动 PR run `31511661925` 为准。
- Round 1 验收发现 F-001/F-002 后，`8a418837` 以私有 recovery epoch、Pending/Claimed target 和 completion-time directive 恢复既有锁定语义；没有产品、公开接口或 schema 偏移。

## 验证结果

### 本地检查

| 命令 | 结果 | 说明 |
|---|---|---|
| `node scripts/check-cloud-only-verification.selftest.mjs` | 通过 | 仓库云端验证合同自检。 |
| `node scripts/check-cloud-only-verification.mjs` | 通过 | 仓库云端验证合同。 |
| `git diff --check`、`git diff --staged --check` | 通过 | 源码及整改记录提交前无空白错误。 |
| `python3 ./.trellis/scripts/task.py start .trellis/tasks/08-11-availability-circuit-recovery` | 通过（降级模式） | 当前环境无 session identity，未持久化 active-task pointer；施工上下文不受影响。 |

未在本地运行 Cargo、pnpm、构建、测试、格式化或依赖安装，遵循仓库云端验证规则。

### GitHub CI 与编译

| Workflow / Job | 结果 | 链接或说明 |
|---|---|---|
| `ci-gate` | 通过 | [job 93853377435](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31511661925/job/93853377435) |
| `rust` | 通过 | 云端格式化、绑定导出、Clippy、Rust tests、benchmark 与依赖审计均由 [自动 PR workflow](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31511661925) 完成。 |
| `frontend` | 预期跳过 | 本次 scope 为 `rust`；`ci-gate` 已接受该跳过。 |
| `pr-title`、CodeQL | 通过 | 均绑定候选 `36ef9df6…`。 |

### 人工验证

- 未执行真实 Gateway 手动/定时探测场景；本地运行该类测试与构建受仓库规则禁止，相关自动验证由上述云端 Rust tests 完成。

## 测试、文档与合同

- 测试：在两个现有 Rust 模块内新增/扩展状态机与 recovery scheduler 单测；完整 Rust 测试在云端 CI 通过。
- 现行文档：不适用；产品语义已经由任务 PRD、设计和实施材料锁定，未引入需要长期维护的公开行为文档变更。
- 类型或机器合同：不适用；未修改 IPC、设置、数据库 schema、生成绑定或前端合同。
- 迁移或发布说明：不适用。

## 兼容性、风险与回滚

- 兼容性：无公开接口或前端合同变化。
- 数据与配置：无迁移、无 schema 或设置变化；Gateway 未运行时不会写入离线持久化熔断状态。
- 安全与隐私：无边界变化。
- 回滚方式：回退本任务的功能提交即可。
- 剩余风险：真实运行 Gateway 的人工端到端场景未在本地执行，遵循仓库云端验证规则；main 仍应重点审查 HalfOpen 与 scheduler 的并发边界。

## 未完成项与阻塞

- 无。

## 建议 main 重点审查

- `src-tauri/src/gateway/runtime.rs:GatewayRuntime::record_availability_probe_outcome`：确认 Open 未到期、Open 到期转 HalfOpen、Closed 三种分支均符合锁定语义。
- `src-tauri/src/app/provider_availability_probe_runtime.rs:run_scheduled_probe`：确认补测的写入后状态检查、generation 失效与 in-flight 合并不会重复发出 HTTP probe。

## main 验收记录

### Round 1 - 需要整改

- 审查日期：2026-08-11
- 审查候选 head：`d68050e6c8f0efdd0a5917755d1f93bd56feff85`
- 实时 CI：当前 head 的 `ci-gate`、`pr-title`、`rust`、`support-contract`、`change-scope` 和 CodeQL 均通过；[ci-gate job 93800301051](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31495975865/job/93800301051)。
- 审查范围：任务材料、当前 PR diff、Gateway HalfOpen 证据路径、recovery scheduler、测试与实时 PR 状态。
- 结论：不接受，不能合并当前候选。
- 原因：HalfOpen 失败未清除已有 recovery target（F-001），且 recovery target 的 30 秒 due time 未以实际 probe 完成时刻计算（F-002）。详见同任务目录的 `findings.md`。
- 接受的偏移或风险：无。

### Round 2 - 需要整改

- 审查日期：2026-08-12
- 审查候选 head：`36ef9df65b7d6eeb22eb3a19ecbf892e39194a02`
- 实时 CI：当前 head 的 `ci-gate`、`pr-title`、`rust`、`support-contract`、`change-scope` 和 CodeQL 均通过；[ci-gate job 93853377435](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31511661925/job/93853377435)。
- 审查范围：首轮 F-001、F-002 的整改 diff、对应测试和实时 PR 状态。
- F-001：已解决。失败或成功关闭会失效 Pending/Claimed recovery work，创建或加入 flight 前会原子复核 generation、epoch、due 和 claim。
- F-002：未完全解决。运行时代码已使用实际 completion time，但新增测试只验证 directive/queue helper，未经过 scheduled consumer；恢复旧的 waiter-time 逻辑时测试不会失败。
- 结论：不接受，当前候选不能合并；仅需补齐 F-002 的有效回归测试后再次交付。
- 接受的偏移或风险：无。

## main 收尾

> 仅 main 填写。

## 返工记录

- 日期：2026-08-11
- 返工候选：`36ef9df65b7d6eeb22eb3a19ecbf892e39194a02`
- 范围：处理 `findings.md` 的 F-001 与 F-002；附带撤销第三次成功关闭后遗留的 claimed recovery，防止旧恢复链跨越 Closed 后的新周期。
- 本地证据：`git diff --check`、cloud-only self-test 和合同检查通过；未运行 Cargo、pnpm、构建、格式化或 Rust tests。
- 云端证据：候选 `498c8e45` 的 Rust job 只报告单文件格式漂移，已应用对应 artifact 并推送 `36ef9df6`；该最新候选的 `rust`、CodeQL、`pr-title`、`change-scope`、`support-contract` 和 [ci-gate](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31511661925/job/93853377435) 均通过。

### Round 2 - F-002 消费路径回归测试

- 日期：2026-08-12
- 冻结起点：`36ef9df65b7d6eeb22eb3a19ecbf892e39194a02`
- 范围：只补齐 F-002 的确定性回归保护；保持 completion-time 实现、F-001 recovery epoch/claim 生命周期、scheduler、generation、limiter 与 in-flight 语义不变。
- 实现：`run_scheduled_probe` 在 probe waiter 返回后调用私有 `consume_scheduled_completion`；该消费路径只使用 `CompletedProbe.recovery` 入队或 settle，不以 waiter resume/current time 改写 due。
- 测试：`scheduled_completion_uses_probe_completion_time_not_waiter_resume_time` 通过上述生产消费路径，固定 completion=`10_000ms`、waiter resume=`55_000ms`，断言 recovery due 恒等于 completion 加 `RECOVERY_PROBE_DELAY_MS` 且不等于 waiter resume 加该延迟。
- 本地证据：cloud-only self-test、合同检查和 `git diff --check` 通过；按仓库规则未运行 Cargo、pnpm、构建、测试或格式化。
- 功能候选：`9baeeb72522dff47178f0ff6675bc1904dd39f84`；`1f60e4130236e75539acdadbf01ad18965ac35e3` 新增生产 scheduled completion 消费路径回归测试，`9baeeb72522dff47178f0ff6675bc1904dd39f84` 仅应用云端 Rust 格式化 artifact。
- 云端证据：自动 [`ci` run 31517935633](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31517935633) 成功，含 [`rust` job 93867720063](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31517935633/job/93867720063)、[`ci-gate` job 93873788290](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31517935633/job/93873788290)、`change-scope` 与 `support-contract`；[`pr-title` job 93867621012](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31517935618/job/93867621012) 和 [CodeQL run 31517935626](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31517935626) 亦成功。上述检查均绑定该完整功能候选 SHA。
- 交付说明：本次仅补充 CI 证据的 Markdown 提交不改变功能实现候选；推送后仍会等待 PR 最新 head 的自动必需检查终态，再暂停等待 main 复验。
