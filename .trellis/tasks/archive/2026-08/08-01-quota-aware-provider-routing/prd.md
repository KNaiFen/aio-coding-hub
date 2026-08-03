# 限额感知的供应商路由与 TUI 首选

## Goal

让路由展示和 TUI 首选供应商使用“实际可发送”的供应商语义：已被限额或熔断门控拒绝的候选保留审计记录，但不制造无效切换；TUI 不再把已达限额的供应商显示为首选。

## Requirements

- 保留现有网关权威门控：配置费用上限与 OAuth 额度快照在上游发送前拒绝候选，零上游调用且不消耗 Ready-provider 尝试额度。
- 路由明细继续保留 circuit/cooldown/rate-limit 的 skipped hop，不能删除审计证据或改变全候选不可用时的 503 行为。
- 首页与观察者快照中的供应商切换次数只统计非 skipped hop 之间的供应商迁移；skipped hop 只计入跳过数量。
- 重试次数只累计非 skipped hop 的额外 attempts；原始 `attempt_count` 仍表示持久化 attempt 行数。
- TUI 首选供应商按当前活动路由顺序，依次排除配置费用上限已达、有效 OAuth 额度已耗尽、熔断 OPEN 与冷却中的候选。
- TUI 额度判定必须来自现有限额读模型与 OAuth 门控快照，不发起上游请求，不修改额度、熔断、Session 或路由状态。
- 额度读模型失败时只将首选供应商分区标记为 unavailable；观察功能必须 fail-open，不能影响网关核心转发。
- 不修改观察协议 DTO、数据库结构、前端生成绑定或 TUI 命令行接口。

## Acceptance Criteria

- [x] 两个限额/熔断 skipped hop 后一个实际请求显示 `跳2·请1`，不再显示无效的 `切2`。
- [x] A 实际失败、B 被限额跳过、C 实际成功只计算一次有效供应商切换。
- [x] skipped attempt 不增加有效重试次数，原始审计 attempt 数保持不变。
- [x] TUI 首选供应商跳过第一个已达配置费用限额的候选并选择下一个可用候选。
- [x] TUI 首选供应商跳过仍处于有效耗尽窗口的 OAuth 候选。
- [x] 限额查询失败时首选分区不可用，其他快照分区和网关转发不受影响。
- [x] 前端目标测试、TypeScript、ESLint 与 Vite build 通过；Rust 测试、格式化与 Clippy 仅由 CI 执行。

## Notes

- 当前 `provider_checks::run_gates` 已经在 circuit gate 后执行 provider-limit gate；两者都生成 skipped attempt 并在发送前返回。
- 本任务修正的是可观测的“有效路由”与 TUI 首选资格，不移除 skipped 审计记录。
