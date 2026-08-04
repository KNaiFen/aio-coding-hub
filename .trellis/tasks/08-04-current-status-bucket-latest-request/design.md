# 当前状态格按最新请求着色：技术设计

## 边界

业务规则只属于 `provider_availability::timelines()`。AIO 36 格、TUI 12 格和托盘 18 格继续消费同一 `bucket.state`，不增加 IPC、Observer 或前端字段。

## 数据与顺序

- 可用性事实继续来自 `provider_availability_observations`，成功/失败分类、trace/provider 去重和计数不变。
- 查询额外读取 SQLite `rowid`，按 `(observed_at_ms, rowid)` 升序。
- `rowid` 是同毫秒观测的稳定次级顺序；UPSERT 更新既有 trace/provider 时不改变 rowid。

## 聚合算法

1. 按现有自然时间边界生成 buckets。当前对齐时间段为 `[end_at_ms - alignment_ms, end_at_ms)`；AIO 的三个 10 分钟格继续对应一个 30 分钟 TUI 时间段。
2. 扫描观测时照常累加每格和整条 timeline 的成功/失败计数。
3. 仅当观测落在当前对齐时间段时，覆盖保存该 provider 的 `latest_current_success`。
4. 所有格先继续调用现有 75% 成功率函数；只有最后一个显示格再根据 `latest_current_success` 映射为绿/红/灰。AIO 最后格的计数仍属于自身 10 分钟区间，颜色则代表当前 30 分钟对齐时间段的最新状态。
5. 不缓存覆盖结果。时间跨格后重新聚合，原当前格自然恢复成功率语义。

## 兼容与回滚

- 协议、计数、时间边界、失败分类和保留策略均不变。
- 托盘会因共享聚合自然获得相同行为，属于预期效果。
- 回滚只需恢复查询列/排序和最后格覆盖，不涉及数据迁移。

## 验证

- 当前格：多数成功但最后失败为红；多数失败但最后成功为绿；无观测为灰。
- 同毫秒：后插入 rowid 决定最终色，重复查询稳定。
- 跨格：旧当前格恢复 75% 规则，新当前格按最新请求或无数据。
- 12/18/36 格均命中相同规则，计数和边界保持不变。
