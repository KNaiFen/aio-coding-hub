# Technical Design

## Boundaries

- 图表与余额只改变前端格式/布局，不改数据源。
- 状态条的颜色判定由 Rust 领域层唯一拥有，前端只把状态枚举映射到语义色。
- 主动 probe 复用现有网络协议、超时、凭据和 circuit refresh，新 coordinator 只拥有单飞、调度和观测持久化。
- `usage_ledger` 仍为统计事实源；日 rollup 仅为可重建投影。

## Data And Interfaces

- schema 52 为 Provider 增加 `availability_probe_enabled` 和 `availability_probe_interval_minutes`，为日 rollup 增加 `success_output_tokens_per_second_sum`。
- Provider upsert/summary/Specta 公开字段同步新增两个 probe 配置；可用性状态枚举新增 `degraded`。
- Usage Summary/Leaderboard/Trend 保持公开字段 `avg_output_tokens_per_second`，内部改为 rate sum/count，不改 IPC 外形。
- 主动 probe 结果写入现有 `provider_availability_observations`，不增加来源列，不向 UI 暴露真实请求/手动/定时区别。

## Timing And Aggregation

- 最终成功 attempt 从该 attempt 开始计时，在首个可信协议完成事件冻结，无显式事件时退化到干净 EOF。
- 下游断开仅作废输出流完整性；上游错误、超时、终端错误和完成前背压作废最终 attempt 计时。
- 合格单条 rate 为 `output_tokens * 1000.0 / final_attempt_duration_ms`；终端聚合、ProviderAgg 合并和 rollup 只相加 rate sum/count。
- v51→v52 清理旧 rollup 投影、标记历史日期 dirty 并重置 cursor，从永久 ledger 重建。

## Scheduler

- 当地日 00:00 为锚点，按 `1..=1440` 分钟生成每日边界，目标时间为边界 +5s + provider ID 稳定 0..3s 错峰。
- 启动、唤醒、开启或改间隔从下一边界开始；不跑当前周期或历史周期。
- 只调度 `provider.enabled && availability_probe_enabled`，全局定时并发为 4；手动入口不被全局排队延迟，但与同 Provider 的进行中 probe 共享结果。
- 定时 trace ID 绑定 Provider 与边界，手动 trace ID 唯一；配置 generation 变化、禁用或删除后的旧结果丢弃。

## Failure And Security

- 配置分钟在前端、领域层和数据库 CHECK 三层限制，所有 SQL 参数化。
- probe 只读现有后端凭据快照，不向 React 或日志暴露密钥；后台错误只记录脱敏摘要。
- `Ok(ok=false)` 是供应商失败观测；内部 `Err` 不伪造供应商失败，也不阻断后台循环。
- 本批不改供应商路由、自动禁用、账户余额协议或请求日志原始 JSON 字段。
