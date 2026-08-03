# Provider 性能趋势设计

## 数据合同

- 新增内部 `usage_provider_metric_trend_v1` IPC 和行 DTO：时间桶、粒度、Provider 身份、请求总数、成功样本数、平均耗时、平均 TTFB、加权输出速率。
- 现有缓存率 IPC 保留旧字段并增加粒度元数据，复用共同的 bucket planner、Top Provider 选择和硬预算，避免消费者按 period 错判自适应桶。
- 性能 SQL 使用参数化范围/CLI/Provider 过滤，并组合 `usage_events` 与 Provider 日汇总投影。

## 聚合公式

- 平均耗时：成功请求耗时和 / 成功请求数。
- 平均 TTFB：仅 `ttfb_ms < duration_ms` 的成功请求 TTFB 和 / 有效计数。
- 输出速率：有效成功请求的输出 Token 总和 / `(duration_ms - ttfb_ms)` 总秒数，禁止平均单请求速率。

## 预算与 UI

- 后端根据范围选择 hour/day/week/month/year 中满足 120 桶的最细粒度；响应再次执行 1200 行硬校验。
- Top 10 按过滤范围内成功请求数排序，指标切换不改变 Provider 集合。
- 显式 Provider 过滤绕过 Top 10 成功样本门槛，因此仅有失败请求的已选 Provider 仍返回请求数趋势，性能平均值为空。
- Usage 页面新增性能趋势页签和三项 segmented control；沿用现有 loading/error/stale/custom-range overlay。
- 百万行 raw ledger 基准已测得 `5.057927737s`，因此建立可重建的 Provider 日汇总；该投影是性能缓存，不替代 ledger 的事实源地位。

## 日汇总投影

- 版本化迁移创建 Provider 日汇总、自然日覆盖状态和后台回填游标；迁移本身不扫描或复制无界历史。
- 每行按本地自然日、CLI 和 Provider 聚合趋势所需的请求计数、成功计数、耗时、有效 TTFB、有效生成耗时、输出 Token 和缓存 Token 累计值，并保留 Provider 名称快照。
- ledger 的插入、删除和趋势相关字段更新通过数据库触发器把对应旧/新自然日标记为脏；已是脏状态时不重复写状态，无法转换为本地日期的异常时间戳永久走 raw ledger。
- 后台任务在 `IMMEDIATE` 事务内逐日重建：先确保 coverage 父行存在，再删除该日旧投影、从 ledger 聚合，并用独立 raw `COUNT(*)` 校验投影请求总数后标记完成。提交失败时所有步骤一并回滚。
- 请求日志保留先于日汇总执行；每轮最多重建 32 个自然日，批次之间释放维护互斥锁并延迟续跑，因此多年历史回填不会长期占用保留任务。
- 当前本地自然日不物化。部分日范围也读取 raw ledger；只有完整包含在查询范围内的 `complete` 日进入汇总源，因此自定义范围不近似扩张。
- 小时粒度直接读取 raw ledger；日/周/月/年先把可信日汇总与未覆盖 raw 日合并，再进行 Top Provider 和目标桶聚合。coverage 选择、raw 补集与最终聚合共享同一个只读事务快照，避免迟到更新并发标脏时漏算整日。
- 日汇总 schema 不保存模型、Session 或成本修正维度，因此本任务保留 `usage_ledger`。直接删除 ledger 会破坏现有非趋势消费者。

## 兼容与恢复

- 未初始化或测试夹具没有日汇总表时，趋势查询保持纯 `usage_events` 路径。
- ledger backfill 未完成时不运行日汇总回填；完成后由现有后台链路触发。
- ensure 发现任一日汇总表或脏日触发器缺失时，会在同一事务恢复 schema、清空派生投影并重置游标；查询立即回退 raw，避免信任触发器缺失期间产生的陈旧 `complete` 日。
- 查询与后台任务都按每个本地日重新计算准确的 `day_start_ts/day_end_ts`；任一边界不匹配即回退 raw ledger并重建，不能用单一 UTC offset 代替时区规则。
- Provider 清空统计时在现有删除事务内物理删除对应日汇总，ledger 删除同时把受影响日标脏；不清空统计时保留名称快照。
- 触发器与 ledger 写入保持原子一致；若派生表发生无法由 ensure 修复的列级严重损坏，ledger 写入可能失败。该取舍避免“事实已更新但完成日未标脏”的静默错误，恢复方式是先修复 schema 再重试写入。
