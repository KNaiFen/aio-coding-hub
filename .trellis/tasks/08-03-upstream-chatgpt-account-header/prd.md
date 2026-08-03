# 隔离 ChatGPT 来访账户头

## Goal

选择性移植 7cc1d8ac，确保每个 Provider attempt 只携带当前 Provider 派生的 chatgpt-account-id。

## Requirements

- 在每次 Codex ChatGPT Provider attempt 前移除客户端提供的 `chatgpt-account-id`。
- 仅在当前 Provider 能解析出非空账户 ID 时重新注入，禁止跨 Provider 或客户端身份残留。
- 保留现有 Authorization 清理、Provider token 注入和插件边界；不扩展为入站认证改造。

## Acceptance Criteria

- [ ] 客户端 account ID 被当前 Provider ID 覆写。
- [ ] Provider 无 account ID 时请求中不存在该 header。
- [ ] failover 每个 attempt 使用各自 Provider 身份，不能复用前一个 attempt 的 ID。
- [ ] 变更可追溯到 `7cc1d8accc3725d63ff34519fde9d82f285d3510`，且不宣称解决 `AUD-016`。

## Notes

- 仅移植安全行为和对应测试，不接收上游文件整体版本。
