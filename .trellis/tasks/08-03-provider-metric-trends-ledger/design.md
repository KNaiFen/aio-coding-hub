# Provider 性能趋势设计

## 数据合同

- 新增内部 `usage_provider_metric_trend_v1` IPC 和行 DTO：时间桶、粒度、Provider 身份、请求总数、成功样本数、平均耗时、平均 TTFB、加权输出速率。
- 现有缓存率 IPC 保留旧字段并增加粒度元数据，复用共同的 bucket planner、Top Provider 选择和硬预算，避免消费者按 period 错判自适应桶。
- 性能 SQL 使用参数化范围/CLI/Provider 过滤和 `usage_events` 视图，不引入 schema 变更。

## 聚合公式

- 平均耗时：成功请求耗时和 / 成功请求数。
- 平均 TTFB：仅 `ttfb_ms < duration_ms` 的成功请求 TTFB 和 / 有效计数。
- 输出速率：有效成功请求的输出 Token 总和 / `(duration_ms - ttfb_ms)` 总秒数，禁止平均单请求速率。

## 预算与 UI

- 后端根据范围选择 hour/day/week/month/year 中满足 120 桶的最细粒度；响应再次执行 1200 行硬校验。
- Top 10 按过滤范围内成功请求数排序，指标切换不改变 Provider 集合。
- 显式 Provider 过滤绕过 Top 10 成功样本门槛，因此仅有失败请求的已选 Provider 仍返回请求数趋势，性能平均值为空。
- Usage 页面新增性能趋势页签和三项 segmented control；沿用现有 loading/error/stale/custom-range overlay。
- 不建立物化汇总；若百万行门禁失败则停止合并并重新设计，而非临时加迁移。
