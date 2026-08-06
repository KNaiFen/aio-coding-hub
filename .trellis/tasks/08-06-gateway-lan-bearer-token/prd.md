# AUD-016 非回环网关访问边界

## Goal

保留 LAN 访问能力，同时为所有非回环连接建立统一、不可伪造的 Gateway Bearer Token 边界。

## Requirements

- loopback 连接保持现有本机兼容；真实 TCP peer 非回环时，所有路由包括 `/` 和 `/health` 都必须鉴权。
- Token 使用高熵随机值，只展示一次，仅持久化摘要；摘要和确认状态不得出现在 SettingsView 或诊断日志。
- 旧 LAN/custom 非回环配置自动生成 token；未确认一次性展示即退出时，下次启动必须轮换。
- 严格解析 `Authorization: Bearer`，认证后剥离认证和转发身份头，不能转发给上游。
- 删除 Provider 专用路由、forced-provider 分支与 Claude Terminal 启动入口。
- 客户端传入的 provider/forwarded header 不再构成信任或观测边界。
- Token 轮换使旧客户端立即失效，并刷新实际 Router/runtime 摘要。
- WSL Auto 在 token 生成/轮换时接收一次性明文并同步 CLI 配置；同步失败必须可见且不得静默留下错误状态。

## Acceptance Criteria

- [ ] loopback 无 token 的全部既有路径保持可用。
- [ ] 非回环 peer 对全部 route 无/错 token 返回 401，且没有上游调用、凭据注入或请求日志副作用。
- [ ] 正确 token 可访问，认证头和伪造 forwarded/provider headers 不外传、不绕过观测。
- [ ] 一次性明文不进入 settings cache、持久化、日志或错误；磁盘只保存摘要。
- [ ] 旧 LAN 迁移、未确认重启轮换、主动轮换与旧 token 失效均有云端测试。
- [ ] Provider 专用 URL、Claude Terminal IPC/UI 和所有 forced-provider 分支完全移除。
- [ ] WSL Auto 获得新 token 或明确失败，不产生不可恢复的隐式断联。

## Notes

- 认证判断只信任 Axum `ConnectInfo<SocketAddr>`；关联 `AIO-PENDING-019`。
