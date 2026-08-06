# AUD-016 技术设计

## 入站边界

`axum::serve` 使用带 `ConnectInfo<SocketAddr>` 的 make-service。最外层 middleware 以真实 peer IP 判定是否回环；非回环先验证 Bearer 摘要，再进入任何 route、body read、DB/provider 查询或健康处理。成功后删除认证与转发身份头。

## Token 生命周期

settings 持久化摘要、是否已确认展示和代次，不保存明文。创建/轮换由 settings owned transaction 生成一次性明文，通过 transient mutation result 返回并同步 runtime Router 与 WSL 配置。旧非回环配置迁移为未确认代次；启动看到未确认代次时先轮换再暴露。

## 删除旧信任面

移除 provider-specific route、`forced_provider_id` 在 middleware/selection/diagnostics 中的所有分支，以及 Claude Terminal 的命令、service、query 和 UI。客户端 header 只作为普通不可信输入，不能控制 provider 或观测跳过。

## 失败与回滚

settings 持久化、runtime 摘要、gateway rebind 和 WSL 同步沿现有 owned transaction 收敛。无法完整提交时返回可见错误并恢复上一代摘要；一次性明文不写诊断。
