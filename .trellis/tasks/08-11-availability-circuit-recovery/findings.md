# 验收整改意见：可用性探测熔断恢复

> main 负责写问题、证据和复验要求；执行 session 不删除或改写原始意见，只在“执行回应”中说明整改和证据。

## 当前结论

- 结论：需要整改
- PR：[#109](https://github.com/KNaiFen/aio-coding-hub/pull/109)
- 审查轮次：Round 1
- 审查版本：`d68050e6c8f0efdd0a5917755d1f93bd56feff85`
- CI 状态：`ci-gate`、`pr-title`、`rust`、`support-contract`、`change-scope` 和 CodeQL 均已通过；[ci-gate job 93800301051](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31495975865/job/93800301051)。
- 审查范围：完整任务；PRD/设计/交付材料、当前 PR diff、Gateway 熔断状态机、probe scheduler 与实时 PR 状态。

## 总结

自动检查属于当前候选且为绿色，但恢复补测的生命周期仍有两处与锁定语义不符：HalfOpen 失败不会撤销已经排队的旧补测，且 30 秒延迟从 waiter 恢复执行时计算而不是从 HTTP probe 实际完成时计算。两者会导致过期恢复链跨越重新 Open，或让补测明显晚于指定时刻，当前版本不能合并。

## 未解决问题

- [ ] F-001 HalfOpen 失败必须撤销已有 recovery target
- [ ] F-002 Recovery target 必须以实际 probe 完成时刻计算 due time

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
- 验证：新增 `valid_half_open_failure_invalidates_pending_and_claimed_recovery_work`、`invalidated_claimed_recovery_cannot_start_a_flight` 与 `successful_circuit_closure_invalidates_claimed_recovery_work`。本地 `git diff --check`、cloud-only self-test 和合同检查通过；Rust tests / `ci-gate` 待新候选推送后由 GitHub Actions 验证。
- 计划偏移：无产品语义偏移；以私有 recovery epoch/claim 生命周期实现既有“失败或关闭后不保留旧恢复链”的锁定行为。

**main 复验**

- 状态：待复验
- 复验候选 head：
- `ci-gate`：
- 结论与证据：
- 日期：

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
- 验证：新增 `recovery_due_uses_probe_completion_time_not_waiter_resume_time`，固定 completion/waiter 时刻并断言 due 恒等于 completion 加 `RECOVERY_PROBE_DELAY_MS`；本地允许检查通过，云端 Rust tests / `ci-gate` 待新候选验证。
- 计划偏移：无；仍复用既有 scheduler、generation、四槽 limiter 与 in-flight 合并，不引入裸 `sleep`。

**main 复验**

- 状态：待复验
- 复验候选 head：
- `ci-gate`：
- 结论与证据：
- 日期：

## CI、编译或环境问题

无。当前候选的相关自动检查均通过；本轮拒绝来自功能与调度语义审查。

## 计划偏移需要处理

- 两项问题均需恢复 `prd.md` 和 `design.md` 的既有锁定行为，不需要新的产品决策。

## 本轮返工边界

### 必须处理

- F-001
- F-002

### 不要顺带处理

- 不要重构一般熔断策略、修改默认阈值或冷却时长，或改变定时可用性探测设置语义。
- 不要处理其他 PENDING 项，尤其不得读取、执行、修改、移动或删除 `upgrade-tui.command`。

### 可以自主决定

- 失效 recovery target 与传递 completion time 的具体内部表示，只要满足两个 findings、保留既有 scheduler/generation/limiter/in-flight 合同并补齐回归测试。

## 再次交付要求

- [ ] F-001 和 F-002 均有执行回应、代码证据和相应回归测试。
- [ ] 本 `findings.md` 已随返工提交保留在 PR 中，`delivery.md` 已更新实现、偏移、验证和返工记录。
- [ ] 新提交已推送，PR 最新 head 的必需 CI 和相关编译为绿色。
- [ ] 新候选完整 head SHA 和对应 `ci-gate` 已写入 `delivery.md` 与本文件。
- [ ] 执行 session 已暂停并通知 main 复验。

## 建议项

无。
