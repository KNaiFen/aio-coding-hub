# 实施清单

1. 恢复分 CLI 静默期选择器并同步设置页文案。
2. 在提醒发送前复用 active request snapshot 服务，增加 generation 竞态保护。
3. 更新通知单元测试：分 CLI 时间、并发、漏事件、快照失败、不同 CLI 和查询期间新请求。
4. 运行目标 Vitest、typecheck、lint 和 build。
5. 提交 `fix(notification): restore reliable task completion quiet periods`。
