# 根因证据

- 当前 `taskCompleteNotifyEvents.ts` 只有 `QUIET_PERIOD_MS_DEFAULT = 30_000`，所有 CLI 共用。
- 当前设置页仍声明 Claude/Gemini 30 秒、Codex 120 秒，代码与用户界面不一致。
- `git show e3e0fccb` 证明该提交把 Codex 测试从 120 秒改成 30 秒；逐请求并发提交没有触碰通知文件。
- Rust `gated_emit` 在 WebView heartbeat 被判定失活时跳过 `gateway:request_signal`，存在前端漏 start/complete 的合法路径。
- `active_request_logs_snapshot` 已暴露后端注册表快照，可在通知到期时复用，无需改动转发链或新增 IPC。
