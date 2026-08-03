# 技术设计

## 权威边界

- 网关 `provider_limits::gate_provider` 保持上游发送前的权威限额门控，不新增第二套转发判定。
- 请求路由仍包含所有 gate-only skipped attempts，用于解释限额、熔断与冷却。
- 展示层定义“有效供应商序列”为 `route.filter(!skipped)`；切换次数只比较该序列中的相邻供应商。

## 路由计数

- `skippedCount`：全部 skipped hop 数量。
- `requestCount`：全部非 skipped hop 的 attempts 总和。
- `retryCount`：全部非 skipped hop 的 `max(attempts - 1, 0)` 总和。
- `transitionCount`：非 skipped hop 序列中相邻供应商身份变化的次数。
- `attemptCount`：持久化 attempt 行数，保持原值，不参与有效切换/重试派生。

供应商身份优先使用正整数 `provider_id`；畸形或未来数据退化为有界名称比较。所有计算保持有界、无异常出口。

## TUI 首选供应商

观察者的只读数据库投影继续加载活动路由中的最小供应商身份，并额外构造 `limited_provider_ids`：

1. 通过现有 `provider_limit_usage::list_v1` 获取当前 CLI 的配置费用上限用量，使用与 UI 一致的窗口读模型判断已达上限。
2. 仅对 OAuth 候选读取 `provider_oauth_limits::gate_snapshot`，复用网关的耗尽窗口与重置时间语义。
3. 在只读 circuit peek 前后均不产生状态写入；选择顺序为活动路由原顺序，候选必须不在 limited 集合、非 OPEN 且不处于 cooldown。

任一首选资格查询失败时，`preferred_provider.available=false`；请求历史、今日用量、活动请求等其他分区继续按已有逻辑返回。

## 兼容与安全

- 不扩展 `ObserverSnapshotV1`；TUI 格式化将 `provider_switch_count` 与
  `retry_count` 作为相互独立的有效路由指标展示。
- 不加载或序列化供应商密钥；观察者身份查询只增加非敏感 `auth_mode`。
- 不删除 skipped hops，不改变 503、重试、供应商健康或 Session 亲和。
- 观察者仍使用独立 `query_only` SQLite 连接和既有 1.5 秒超时。

## 验证

- TypeScript 单元测试覆盖 skipped-only、实际 A→C、同供应商重试和畸形身份。
- Rust 单元测试覆盖费用限额边界、OAuth 候选、首选过滤和观察者有效路由计数。
- 本机仅运行前端目标测试、typecheck、lint 和 build；Rust 由 CI 验证。
