# 收敛 OAuth 状态缓存

## Goal

选择性移植 84564a5b，修复 OAuth 刷新和断开后的 stale cache 竞态。

## Requirements

- OAuth 用户动作前取消旧状态查询，并使用 `staleTime: 0` 获取服务器真值。
- 登录成功但随后状态读取失败时，将成功结果作为可用 fallback 写入缓存。
- 断开连接时同时清理编辑器本地状态和 React Query 状态缓存。
- 不改变 token 本身、后台刷新协议或 settings 并发写入语义。

## Acceptance Criteria

- [ ] 旧状态请求晚于刷新完成返回时，最终 `expires_at` 仍为新值。
- [ ] 登录成功但状态读取失败时缓存仍显示已连接和新到期时间。
- [ ] 断开后在 staleTime 内重新打开编辑器不显示旧连接状态。
- [ ] 变更可追溯到 `84564a5b27db017cab02c77e5f8ad82f799befef`。

## Notes

- 本项不等同于 `AUD-012`，不得扩大修复声明。
