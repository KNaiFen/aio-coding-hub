# ChatGPT 账户头隔离执行记录

- [x] 在每次 Provider attempt 开始时清除客户端与上一 attempt 遗留的账户头。
- [x] 仅从当前 ChatGPT Provider 凭据派生并注入非空账户 ID。
- [x] 覆盖客户端覆写、无账户 ID、failover 与重试隔离语义。
- [x] 保留 fork 的现有认证清理、Provider token 注入与插件权限模型。
- [ ] 由 GitHub Actions 完成 Rust 格式、编译与 native 测试门禁。

## Review Findings

- 范围外：具有 `request.header.write` 权限的 `beforeSend` 插件会在 Provider 身份注入后运行，且 `chatgpt-account-id` 当前不是 host-owned 保留头，因此高信任插件仍可覆写它。本次按既有插件信任模型保留该行为；若产品要求账户身份在插件边界也不可变，后续需将该头列为保留头，或在真正发送前重新锁定 Provider 身份。
- 范围外：`chatgpt-account-id` 已在 upstream fingerprint 中按敏感值处理，但通用调试日志脱敏和插件 `request.header.readSensitive` 策略尚未统一。后续应集中统一敏感头定义，并覆盖日志脱敏、普通插件不可见和敏感读取权限测试。
- 以上两项在固定上游提交中同样存在，按集成任务边界仅登记，不混入本次上游移植提交。
- 项目规则禁止本地 native 验证；Rust 格式、编译与测试交由 GitHub Actions。
