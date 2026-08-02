# 任务结束提醒静默期与活跃快照兜底

## Goal

让任务结束提醒严格遵守分 CLI 静默期，并在前端漏掉请求事件时避免错误通知。

## Requirements

- Codex 最后一个请求完成后静默 120 秒；Claude、Gemini、Grok 静默 30 秒。
- 同一 CLI 任一 trace 活跃时不得启动或发送结束通知。
- 通知发送前读取后端活跃请求快照；同 CLI 仍活跃或读取失败时不通知。
- 快照等待期间出现新请求时，旧回调不得发送通知。
- 修复只影响提醒模块，不修改网关注册、转发、重试和熔断。

## Acceptance Criteria

- [ ] Codex 119.999 秒不通知，120 秒通知；其他三类 CLI 在 30 秒通知。
- [ ] 并行请求、漏 start、漏事件后的后端活跃快照和异步竞态均不会误报。
- [ ] 快照失败只记录诊断，不抛出到界面或影响其它事件监听。
- [ ] 设置页文案与实际静默期一致。

## Root Cause

- 历史提交 `e3e0fccb` 删除了 Codex 120 秒分支并把测试改成统一 30 秒；逐请求并发提交 `3a24448b` 未修改通知模块。
- `gateway:request_signal` 由 WebView heartbeat gate 控制，WebView 被判定失活时 start/complete 都可能丢失，因此前端 trace 集合不能单独作为最终通知依据。
