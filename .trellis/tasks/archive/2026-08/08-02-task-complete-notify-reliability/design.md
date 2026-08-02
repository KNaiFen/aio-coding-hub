# 技术设计

- 用 `quietPeriodMs` 恢复 Codex 120 秒、其它 CLI 30 秒。
- Session 增加单调递增 generation；任何 start/complete/disable/cleanup 都会让旧异步回调失效。
- 定时器到期后调用现有 `activeRequestLogsSnapshot()`，只检查相同 `cli_key` 的活跃请求。
- 快照为空后重新读取 session，确认 generation、pending 状态和 in-flight 集合仍一致才发送通知。
- 快照失败 fail-closed：本轮不发送，清理 pending timer 并保留未通知状态，等待下一轮请求事件重新调度。
