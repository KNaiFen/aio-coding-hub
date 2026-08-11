# 技术设计：熔断恢复探测接入

## 目标与边界

本任务把已完成的手动、定时可用性探测结果接入**运行中的** Gateway 熔断器恢复路径。它不创建新的熔断器、不改变 Open 时长、不改变 IPC、设置、数据库 schema 或前端合同。

恢复证据仅用于 HalfOpen：Open 尚未到期时，成功探测只保留原有可用性观测；Open 到期后，探测结果可促成 `Open -> HalfOpen`，再由现有状态机按成功或失败处理。

## 状态与调用流

```text
manual / scheduled probe
  -> shared per-provider HTTP flight
  -> finish_probe validates generation
  -> record availability observation
  -> record one circuit evidence result in active GatewayRuntime
  -> notify all coalesced callers
  -> scheduled caller may queue one recovery probe at completion + 30s
```

### 熔断证据写入

在 `ProviderAvailabilityProbeRuntimeState::finish_probe` 中，只有 `should_record` 为真且结果为 `Ok(ProviderAvailabilityResult)` 时才写入熔断证据。该位置保证配置变更、禁用或删除导致的旧 generation 不能影响新 Provider 状态。

新增 crate-private Gateway 访问入口，不把 `CircuitBreaker` 暴露给 probe runtime：

1. 通过现有 app Gateway state 取得正在运行的 `GatewayRuntime`；不存在时返回“未接入”，不报错、不持久化离线状态。
2. 用结果完成时间调用现有 `should_allow`。若 Open 已过期，复用既有 `OPEN_EXPIRED` 迁移和事件；未到期 Open 保持拒绝状态。
3. 只有迁移后的状态为 HalfOpen 时，成功调用既有成功记录和事件逻辑，失败调用既有失败记录和事件逻辑。
4. Closed、未到期 Open、失效 generation、缺失 Provider 元信息和内部 `Err` 都不改变熔断器。

该入口需要沿用既有 Provider 名称、Base URL、CLI key、trace ID 和 `gateway:circuit` 事件字段。它不得直接调用一般成功记录路径来影响 Closed 的失败滑窗。

### 定时恢复补测

在 probe runtime 内为每个 Provider 增加一个可失效的 recovery target：

- target 记录当前 generation、`due_at_ms = successful_completion + 30_000` 和独立的 source/trace identity；不能复用自然边界的 trace 或 deadline。
- `run_scheduled_probe` 收到成功结果后，由于 `finish_probe` 已先完成状态写入，读取 active circuit snapshot。仅 snapshot 仍为 HalfOpen 时设置 target；Closed 时绝不设置。
- target 由现有 scheduler tick 取出，走同一个 4-slot limiter、deadline、`probe` 与同 Provider in-flight 合并。target 执行前再次确认 circuit 仍为 HalfOpen；否则静默丢弃，避免在其他请求已恢复后发无用请求。
- 一个 Provider 同一 generation 最多保留一个 recovery target。成功的 recovery probe 若仍 HalfOpen 会安排下一次；使其 Closed 的成功不安排下一次。
- `invalidate_generation`、Provider disable/delete、schedule revision 改变和完整扫描确认 Provider 消失时清除 target。应用长时间休眠后沿用现有过期任务跳过策略，不补跑陈旧恢复请求。

## 错误、并发与兼容性

- 网络和上游不可用通常表现为 `ProviderAvailabilityResult { ok: false }`，它们在 HalfOpen 是有效反证；只有内部 `Err` 不参与熔断状态。
- 同一实际 HTTP flight 只在 `finish_probe` 记一次结果；多个等待者不能把一条网络响应累计成多次 HalfOpen 成功。
- 定时 caller 即使与手动 flight 合并，仍可在取得成功结果后按“写入后仍 HalfOpen”的规则排 recovery target；但不会再写第二次熔断证据。
- availability observation 写入失败仅按现有行为记录告警；有效 probe 结果的熔断证据不应依赖该时间线写入成功。
- 无迁移、无公开 API/类型变动。既有前端已经识别 `OPEN_EXPIRED`、`HALF_OPEN_SUCCESS` 与 `HALF_OPEN_FAILURE`，仅会收到带 availability trace ID 的现有事件。

## 风险与回滚

- 主要风险是把 Closed 状态的探测成功误当作一般业务成功，从而清空失败滑窗；专用 HalfOpen 门控和单元测试必须防止此回归。
- 主要调度风险是用独立 `sleep` 绕过 generation 与 limiter；禁止该实现方式。
- 回滚为回退本任务 PR；没有数据迁移或配置回滚步骤。
