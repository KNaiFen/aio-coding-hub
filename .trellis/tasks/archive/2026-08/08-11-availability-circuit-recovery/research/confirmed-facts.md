# 已确认代码事实

本文件是规划期的代码证据摘要；现行代码和任务设计优先于此记录。

- `shared/circuit_breaker::CircuitBreaker::should_allow` 在 Open 的 `open_until` 到期后才迁移为 HalfOpen；`record_success` 只在 HalfOpen 累计，`record_failure` 在 HalfOpen 立即重开。
- `app/provider_availability_probe_runtime::probe_manual` 与 `run_scheduled_probe` 都进入同一 `probe`，最后由 `finish_probe` 写入可用性 observation，并以 generation 保证旧结果不记录。
- `finish_probe` 在发送各 waiter 前完成结果处理，因此 scheduled caller 在获得结果后可以读取“写入后的”熔断 snapshot 来决定是否排 30 秒 target。
- 现有 schedule 以 `RuntimeEntry`、Provider generation、mutation gate、in-flight flight 和全局 4-slot semaphore 管理；配置 mutation/禁用/删除会失效 generation。
- `GatewayRuntime` 持有与路由器共用的 `Arc<CircuitBreaker>`；`app/gateway_state` 提供只读的 running runtime access，Gateway 未运行时仅有数据库持久化快照。
- `gateway/proxy/provider_router` 已有成功/失败记录并发送 `gateway:circuit` transition 的辅助逻辑；应复用它而不是复制事件 payload。
- `ProviderAvailabilityResult` 已携带 `ok`、provider ID、名称与 Base URL；CLI key 可从现有 Provider query 获得。现有 IPC 返回结构无需改变。
