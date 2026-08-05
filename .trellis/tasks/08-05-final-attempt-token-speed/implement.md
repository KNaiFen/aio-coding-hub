# 实施清单

1. 为流式 relay 和非流式 body read 增加最终上游尝试完成计时及污染规则。
2. 将字段贯通 completion/event/log insert/query/detail/ledger/view，并新增 schema 49 migration 与 ensure 修复。
3. 改写汇总、排行、趋势和日 rollup 的资格条件与加权公式。
4. 更新前端与 TUI 单请求格式化和所有消费点，保留旧字段但不再展示旧速度。
5. 增加重试排除、EOF 与下游延迟、单事件、非流式、污染未知、迁移和聚合回归测试。
6. 本地只运行 TypeScript/前端验证，原生测试交给 CI。
