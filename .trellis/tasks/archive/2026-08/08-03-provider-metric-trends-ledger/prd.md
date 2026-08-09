# 基于 ledger 实现有界 Provider 趋势

## Goal

参考 d27efdb8，基于 usage ledger 实现 Top 10、120 桶的性能趋势并治理缓存率趋势预算。

## Requirements

- 基于 `usage_events/usage_ledger` 新增 Provider 平均耗时、TTFB 和加权输出速率趋势，禁止查询 `request_logs`。
- 未选择 Provider 时固定成功请求数 Top 10；选择 Provider 时只返回该 Provider。
- 今天按小时，短范围按天，长范围按周/月/年自动降采样；最多 120 时间桶和 1200 行。
- 现有缓存率趋势使用相同 Provider 排名与预算，消除 `None/0` 代表无限的行为。
- 复用现有 CLI、Provider、日期和排除 CX2CC bridge 过滤；趋势使用本地自然日，不读取首页自定义统计日开始时间。
- 完全沿用「系统设置 → 请求记录保留」；过期详情删除后长期趋势结果保持一致。
- 新增可重建的 Provider 本地自然日日汇总投影，后台有界回填历史 ledger；已完成日优先读取日汇总，未完成或未覆盖日继续读取 raw ledger。
- `usage_ledger` 继续作为长期分析事实源，不在本任务中删除；它仍服务模型、Session、文件夹、成本修正和 Provider 限额等非趋势功能。
- 日汇总失败、脏日或回填未完成时必须自动退回 raw ledger，不得阻断网关启动、请求日志保留或其他统计查询。

## Acceptance Criteria

- [ ] 平均耗时、有效 TTFB 和输出速率公式与现有 summary 完全一致。
- [ ] 错误、排除统计、NULL usage 和无效 TTFB 不污染指标分母。
- [ ] 全局结果不超过 10 个 Provider、120 个桶和 1200 行，过滤单 Provider 时不超过 120 行。
- [ ] 删除已被 ledger 覆盖的过期 `request_logs` 前后趋势结果一致。
- [ ] 日汇总与 raw ledger 混合查询等于纯 ledger 结果，且不会重复计算已覆盖自然日。
- [ ] 日汇总回填可恢复、幂等；事务失败不得推进覆盖状态或删除事实数据。
- [ ] 当前自然日和部分日自定义范围保持 raw ledger 精度；只有完整、已覆盖自然日使用日汇总。
- [ ] Usage 页面可切换三个性能指标，图例可真实隐藏系列，tooltip 显示值和样本总数。
- [ ] 百万行 fixture 的 release 查询目标不超过 1 秒；未达到时该子任务不得合并。
- [ ] 设计参考 `d27efdb8c8bbfadf12c3b76c677a9524f312baee`，但不移植其 request_logs 或无限 limit 实现。

## Notes

- ledger 已是请求详情的紧凑统计层；本项不以牺牲会话、文件夹、模型或成本历史换取额外压缩。
- 物理删除 ledger 需要先迁移所有非趋势消费者，作为独立产品改造处理。
