# 当前行为调研

## 网关

- `failover_loop/prepare/provider_checks.rs` 先执行 circuit gate，再执行 provider-limit gate。
- 两种拒绝都在上游发送前返回 `None`，写入 `outcome=skipped` attempt，且不增加 Ready-provider 计数。
- `provider_limits.rs` 同时覆盖配置费用上限和 OAuth quota snapshot；数据库错误按现有策略允许转发。
- 因此网关核心已经不会向已知限额候选发送请求，问题来自 skipped 审计行被投影成供应商切换。

## 首页与 TUI 路由

- `requestLogPresentation.ts` 当前用 `route.length - 1` 计算切换，两个限额 skipped hop 加一个实际请求会显示 `切2`。
- 观察者 `project_terminal` 同样比较完整 route 的相邻名称，并用 `attempt_count - 1` 计算重试，都会被 gate-only skipped 行放大。
- 正确的有效序列应排除 skipped hop，同时继续把 skipped hop 留在详情与审计中。

## TUI 首选

- `app/observer/snapshot.rs` 当前按活动路由加载供应商 id/name，只过滤 circuit OPEN/cooldown。
- 配置费用上限已有 `provider_limit_usage::list_v1` 只读读模型；OAuth 耗尽已有 `provider_oauth_limits::gate_snapshot` 纯读取门控。
- 观察者使用独立 `query_only` SQLite 连接，适合复用这两个读入口；失败应仅让首选分区 unavailable。

## 规范冲突

- 旧观察者规范明确“不引入账户配额猜测”，与用户的新要求冲突；本任务将其更新为复用权威本地限额读模型，不做模型或远端额度猜测。
- 旧路由规范把所有 route hop 计为 transition；需要改为区分审计候选序列与实际发送序列。
