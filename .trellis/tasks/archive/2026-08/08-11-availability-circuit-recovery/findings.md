# 验收整改意见：可用性探测熔断恢复

> main 负责写问题、证据和复验要求；执行 session 不删除或改写原始意见，只在“执行回应”中说明整改和证据。

## 当前结论

- 结论：已解决并接受，功能 PR 已合并
- PR：[#109](https://github.com/KNaiFen/aio-coding-hub/pull/109)
- 审查轮次：Round 3
- 审查版本：`8c1c9d27e046aeab8290308e40d4e6570218539c`
- CI 状态：严格必需的 [`ci-gate` job 93919034995](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31531473148/job/93919034995) 与 [`pr-title` job 93912283035](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31531473151/job/93912283035) 均通过；同一 head 的 Rust、support-contract、change-scope 和 CodeQL 亦成功。
- 审查范围：F-001、F-002 最终复验、最新 PR diff、任务索引冲突处理与实时 PR 状态。

## 总结

F-001 的 recovery epoch/claim 生命周期已阻止旧 target 跨越失败或 Closed。F-002 已补充经过 `consume_scheduled_completion` 生产消费路径的确定性测试，completion 与 waiter resume 使用不同时间；恢复 waiter-time 重算会使断言失败。最新 head 的严格门禁与相关云端检查均通过，PR #109 已以 merge commit `15d08f4399d6b1a5361b48d8110e9b49ca3650bb` 合并。

## 当前问题状态

- [x] F-001 HalfOpen 失败必须撤销已有 recovery target（Round 2 已解决）
- [x] F-002 Recovery target 必须以实际 probe 完成时刻计算 due time（Round 3 已解决）

## Round 1

- 本轮整改候选 head：`d68050e6c8f0efdd0a5917755d1f93bd56feff85`
- `ci-gate`：通过，[job 93800301051](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31495975865/job/93800301051)
- 本轮范围：首次完整验收

### F-001：HalfOpen 失败必须撤销已有 recovery target

- 严重程度：阻塞
- 对应要求：R-03、R-04、R-05、AC-03、AC-05；设计“定时恢复补测”要求失败后不能保留旧恢复链。
- 证据：`src-tauri/src/app/provider_availability_probe_runtime.rs:394` 的 `finish_probe` 在记录有效失败证据后不清除 `RuntimeEntry::recovery`；该 target 仅会在到期或配置失效路径中由 `take_due_recovery_targets` 处理（`:767`）。`src-tauri/src/gateway/runtime.rs:485` 的 `GatewayRuntime::record_availability_probe_outcome` 会使 HalfOpen 失败重新 Open（`:501`）。
- 当前行为：定时成功在 HalfOpen 中排入 recovery target 后，同 generation 的手动或定时失败会重新 Open 熔断器，但旧 target 继续保留。若冷却结束后其他请求先把熔断器带回 HalfOpen，`run_scheduled_probe` 只检查当前是否 HalfOpen（`provider_availability_probe_runtime.rs:536`），于是旧 target 会发出 HTTP 请求并成为新周期的恢复证据。
- 影响：失败前的恢复链可穿过一次重新 Open，并额外产生请求或累计新周期的 HalfOpen 成功，违反“失败立即重新 Open 并清零恢复进度”和“不产生过期、无用补测”的验收标准。
- 必须达到的结果：任何当前 generation 的有效 `ok: false` HalfOpen 证据使熔断器重新 Open 时，必须使该 Provider 已排队的 recovery work 失效；之后只能由一次新的、成功完成且写入后仍为 HalfOpen 的定时 probe 创建新 target。
- 修改边界：可修改 `provider_availability_probe_runtime.rs`、现有 Gateway/runtime 测试和本任务交付材料；必须保持 generation、in-flight 合并、四槽 limiter、Open 冷却时长和公开合同不变。
- 测试和文档：新增确定性回归测试，预置同 generation recovery target，模拟 HalfOpen 有效失败、重新进入 HalfOpen 并推进旧 due time，断言 target 不会被取出且没有对应 HTTP probe；在 `delivery.md` 的返工记录说明覆盖方式。
- 复验方式：main 检查 target 的失效条件和该回归测试，并核对新 head 的云端 Rust tests 与 `ci-gate`。

**执行回应**

- 修改：有效 HalfOpen 证据完成后统一更新 recovery work。`ok: false` 重新 Open，或 `ok: true` 已使电路 Closed，都会清除 Pending/Claimed target 并推进私有 epoch；只有成功且写入后仍为 HalfOpen 才保留下一次 recovery directive。recovery 开始路径在创建或加入 in-flight 前原子校验 generation、epoch、due 和 Claimed 状态，失效 target 返回 Stale，不会发出 HTTP。
- 代码位置：`src-tauri/src/app/provider_availability_probe_runtime.rs` 的 `finish_probe`、`update_recovery_work_after_circuit_evidence`、`begin_probe_with_recovery`、`take_due_recovery_targets` 和 `recovery_claim_matches`。
- 验证：新增 `valid_half_open_failure_invalidates_pending_and_claimed_recovery_work`、`invalidated_claimed_recovery_cannot_start_a_flight` 与 `successful_circuit_closure_invalidates_claimed_recovery_work`。本地 `git diff --check`、cloud-only self-test 和合同检查通过；返工最终候选 `36ef9df65b7d6eeb22eb3a19ecbf892e39194a02` 的云端 `rust` 与 [ci-gate](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31511661925/job/93853377435) 均通过。
- 计划偏移：无产品语义偏移；以私有 recovery epoch/claim 生命周期实现既有“失败或关闭后不保留旧恢复链”的锁定行为。

**main 复验**

- 状态：已解决
- 复验候选 head：`36ef9df65b7d6eeb22eb3a19ecbf892e39194a02`
- `ci-gate`：通过，[job 93853377435](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31511661925/job/93853377435)
- 结论与证据：`finish_probe` 在有效失败或成功关闭后经 `update_recovery_work_after_circuit_evidence` 清除 Pending/Claimed target 并推进 epoch；`begin_probe_with_recovery` 在任何 Lead/Wait 分支前校验 generation、epoch、due 与 Claimed 状态。新增测试覆盖 pending、claimed、Closed 和失效 target 不创建 flight，满足 F-001。
- 日期：2026-08-12

### F-002：Recovery target 必须以实际 probe 完成时刻计算 due time

- 严重程度：阻塞
- 对应要求：R-04、AC-04；`design.md` 明确要求 `due_at_ms = successful_completion + 30_000`。
- 证据：`src-tauri/src/app/provider_availability_probe_runtime.rs:383` 在 `finish_probe` 取得实际 `completed_at_ms`，但 `CompletedProbe`（`:89`）没有携带它；`run_scheduled_probe` 在 waiter 返回后才于 `:554` 重新读取当前时间，并在 `:567` 用该值创建 recovery target。
- 当前行为：共享 flight 的 scheduled waiter 被 Tokio 延后恢复，或在完成后才得到调度时，30 秒从 `run_scheduled_probe` 的恢复时间开始计算，而不是从 HTTP probe 的实际完成时间开始计算。
- 影响：补测可被调度延迟任意推后，不满足成功完成后约 30 秒触发的锁定行为；延迟越长，越可能错过预期的恢复检测窗口。
- 必须达到的结果：对于成功、已写入熔断证据且写入后仍为 HalfOpen 的定时 probe，recovery target 的 due time 必须由同一实际 HTTP flight 的完成时间加 30 秒得出；实现必须继续走既有 scheduler，不得引入裸 `sleep`。
- 修改边界：可修改 `provider_availability_probe_runtime.rs` 的 completion 数据和调度测试，以及本任务交付材料；不得改变 probe 公开结果、IPC、数据库 schema、调度失效和 limiter 语义。
- 测试和文档：新增确定性测试，模拟 completion 与 scheduled waiter 消费之间存在延迟，断言排队 target 的 `due_at_ms` 恒等于实际 completion 时间加 `RECOVERY_PROBE_DELAY_MS`；在 `delivery.md` 记录该行为。
- 复验方式：main 检查 completion 时间的传递与 target 计算，审阅回归测试，并核对新 head 的云端 Rust tests 与 `ci-gate`。

**执行回应**

- 修改：`finish_probe` 在实际 HTTP completion 时刻构造 `RecoveryDirective { due_at_ms = completed_at_ms + 30_000 }`，通过 `CompletedProbe` 交给 scheduled caller；`run_scheduled_probe` 只消费该 directive，不再用 waiter 恢复时刻重新计算延迟。
- 代码位置：`src-tauri/src/app/provider_availability_probe_runtime.rs` 的 `RecoveryDirective::from_completion`、`finish_probe`、`CompletedProbe`、`schedule_recovery_probe` 和 `queue_recovery_target`。
- 验证：新增 `recovery_due_uses_probe_completion_time_not_waiter_resume_time`，固定 completion/waiter 时刻并断言 due 恒等于 completion 加 `RECOVERY_PROBE_DELAY_MS`；本地允许检查通过，返工最终候选 `36ef9df65b7d6eeb22eb3a19ecbf892e39194a02` 的云端 Rust tests 与 [ci-gate](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31511661925/job/93853377435) 均通过。
- Round 2 测试整改：将 `run_scheduled_probe` 的 completion 后处理提取为其直接调用的私有 `consume_scheduled_completion` 消费路径，并用 `scheduled_completion_uses_probe_completion_time_not_waiter_resume_time` 经过该生产路径完成入队；运行时仍只透传 `CompletedProbe.recovery`，未改变 F-001 生命周期或调度语义。
- Round 2 确定性证据：测试固定 HTTP completion 为 `10_000ms`、scheduled waiter resume 为 `55_000ms`，并断言入队 due 等于 `10_000 + RECOVERY_PROBE_DELAY_MS`、不等于 `55_000 + RECOVERY_PROBE_DELAY_MS`。若消费路径恢复为 waiter/current-time 重算，该测试会失败。本地 cloud-only self-test、合同检查和 `git diff --check` 通过；新候选云端 Rust tests 与 `ci-gate` 待提交后验证。
- 计划偏移：无；仍复用既有 scheduler、generation、四槽 limiter 与 in-flight 合并，不引入裸 `sleep`。
- Round 2 云端证据：功能候选 `9baeeb72522dff47178f0ff6675bc1904dd39f84` 已通过自动 PR CI；[`rust` job 93867720063](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31517935633/job/93867720063)、[`ci-gate` job 93873788290](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31517935633/job/93873788290)、[`pr-title` job 93867621012](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31517935618/job/93867621012) 与 [CodeQL run 31517935626](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31517935626) 均为成功，且均绑定该完整 head。
- 交付状态：执行 session 已完成 F-002 返工并暂停，等待 main 进行 Round 2 再次验收；未改写上方 main 复验结论。

**main 复验**

- 状态：未解决
- 复验候选 head：`36ef9df65b7d6eeb22eb3a19ecbf892e39194a02`
- `ci-gate`：通过，[job 93853377435](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31511661925/job/93853377435)
- 结论与证据：实现已在 `finish_probe` 以实际 completion time 构造 `RecoveryDirective`，`run_scheduled_probe` 只消费该 directive，运行时语义正确。但 `recovery_due_uses_probe_completion_time_not_waiter_resume_time`（`provider_availability_probe_runtime.rs:1422`）仅直接调用 `RecoveryDirective::from_completion` 和 `queue_recovery_target`；它没有经过 scheduled consumer，也没有让 waiter 实际延迟。若 `run_scheduled_probe` 恢复为用当前时间重算 due，该测试仍会通过，不满足本 finding 要求的确定性回归测试。
- 日期：2026-08-12

## Round 2

- 本轮整改候选 head：`36ef9df65b7d6eeb22eb3a19ecbf892e39194a02`
- `ci-gate`：通过，[job 93853377435](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31511661925/job/93853377435)
- 本轮范围：F-001、F-002 复验
- 结论：F-001 已解决；F-002 的实现已修正，但回归测试尚未覆盖原始缺陷路径，仍需整改。
- F-002 最小复验目标：新增或重构一个会经过实际 scheduled completion 消费逻辑的确定性测试，使 completion 与 waiter resume 使用不同时间，并保证任何重新以 waiter/current time 覆盖 directive due 的改动都会导致测试失败。不得仅重复测试 `RecoveryDirective::from_completion` 本身。

## CI、编译或环境问题

无。当前候选的相关自动检查均通过；本轮拒绝来自功能与调度语义审查。

## 计划偏移需要处理

- F-001 与 F-002 的产品语义均已恢复；F-002 仍需补齐首轮明确要求的回归保护，不需要新的产品决策。

## 本轮返工边界

### 必须处理

- F-002

### 不要顺带处理

- 不要重构一般熔断策略、修改默认阈值或冷却时长，或改变定时可用性探测设置语义。
- 不要处理其他 PENDING 项，尤其不得读取、执行、修改、移动或删除 `upgrade-tui.command`。

### 可以自主决定

- 失效 recovery target 与传递 completion time 的具体内部表示，只要满足两个 findings、保留既有 scheduler/generation/limiter/in-flight 合同并补齐回归测试。

## 再次交付要求

- [x] F-002 已增加会覆盖 actual scheduled consumer 的确定性回归测试，且恢复 waiter-time 重算逻辑时该测试会失败。
- [x] 本 `findings.md` 已随返工提交保留在 PR 中，`delivery.md` 已更新实现、偏移、验证和返工记录。
- [x] 新提交已推送，PR 最新 head 的必需 CI 和相关编译为绿色。
- [x] 新候选完整 head SHA 和对应 `ci-gate` 已写入 `delivery.md` 与本文件。
- [x] 执行 session 已暂停并通知 main 复验。

## Round 3

- 本轮验收候选 head：`8c1c9d27e046aeab8290308e40d4e6570218539c`
- `ci-gate`：通过，[job 93919034995](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31531473148/job/93919034995)
- 本轮范围：F-002 scheduled completion 消费路径回归测试、F-001 生命周期复验、任务分支同步和最新 PR 状态。
- 结论：F-001、F-002 均已解决。`scheduled_completion_uses_probe_completion_time_not_waiter_resume_time` 经生产 consumer 固定 completion=`10_000ms`、waiter resume=`55_000ms`，证明 due 固定为 completion 加 30 秒；过期 claimed recovery 的生命周期保护保持有效。
- 合并事实：PR #109 于 2026-08-12 合并，merge commit 为 `15d08f4399d6b1a5361b48d8110e9b49ca3650bb`。

## 建议项

无。
