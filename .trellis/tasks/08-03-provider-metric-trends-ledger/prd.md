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
- 不新增数据库表、迁移、回填或 ledger 压缩。

## Acceptance Criteria

- [ ] 平均耗时、有效 TTFB 和输出速率公式与现有 summary 完全一致。
- [ ] 错误、排除统计、NULL usage 和无效 TTFB 不污染指标分母。
- [ ] 全局结果不超过 10 个 Provider、120 个桶和 1200 行，过滤单 Provider 时不超过 120 行。
- [ ] 删除已被 ledger 覆盖的过期 `request_logs` 前后趋势结果一致。
- [ ] Usage 页面可切换三个性能指标，图例可真实隐藏系列，tooltip 显示值和样本总数。
- [ ] 百万行 fixture 的 release 查询目标不超过 1 秒；未达到时该子任务不得合并。
- [ ] 设计参考 `d27efdb8c8bbfadf12c3b76c677a9524f312baee`，但不移植其 request_logs 或无限 limit 实现。

## Notes

- ledger 已是请求详情的紧凑统计层；本项不以牺牲会话、文件夹、模型或成本历史换取额外压缩。
