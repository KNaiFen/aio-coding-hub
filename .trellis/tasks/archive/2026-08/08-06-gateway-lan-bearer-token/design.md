# AUD-016 技术设计

## 入站边界

`axum::serve` 使用带 `ConnectInfo<SocketAddr>` 的 make-service。最外层 middleware 以真实 peer IP 判定是否回环；非回环先验证 Bearer 摘要，再进入任何 route、body read、DB/provider 查询或健康处理。成功后删除认证与转发身份头。

## Token 生命周期

私有 `gateway-bearer-token.json` sidecar 持久化摘要、是否已确认展示和代次，不进入 `AppSettings`、SettingsView、配置导入导出或诊断。创建/轮换只在受控内存保留一次性明文，通过 reveal mutation 返回并同步 runtime verifier 与 WSL 配置。旧非回环配置在首次启动时生成未确认代次；启动看到不属于当前进程的未确认代次时先轮换再暴露。

## 删除旧信任面

移除 provider-specific route、`forced_provider_id` 在 middleware/selection/diagnostics 中的所有分支，以及 Claude Terminal 的命令、service、query 和 UI。客户端 header 只作为普通不可信输入，不能控制 provider 或观测跳过。

## 失败与回滚

sidecar 原子写入后才替换 runtime verifier；router 在启动时读取受控 verifier，因此轮换立即使旧 token 失效。WSL 同步失败作为一次性 reveal 的可见错误返回，不写入日志、manifest、argv 或错误细节；WSL manifest v2 只保存非秘密 managed keys，拒绝持久化 Gateway credential。
