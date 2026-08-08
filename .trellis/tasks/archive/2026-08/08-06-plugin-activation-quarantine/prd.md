# AUD-033 插件激活与持久隔离

## Goal

让插件激活事件真正约束执行，并把重复严重运行时故障持久隔离，使跨重启行为可预测且不改变既有网关 fail-open/fail-closed 语义。

## Requirements

- `activationEvents` 只接受精确的 `onStartup`、`onCommand:<command>` 和 `onGatewayHook:<hook>`；显式拒绝 `onProviderEditor:*`、`onProtocolBridge:*`、空 payload、空白变体和未知事件。
- 缺失或空数组保持 legacy 按需激活；不得重写已签名 manifest，也不得隐式执行 startup activation。
- 显式事件不匹配时不得创建 extension host 或执行插件代码；`onStartup` 每个进程对匹配且 enabled 的插件最多执行一次。
- 严重故障仅包括 host crash、JavaScript/runtime error 和执行 timeout；capability、权限、输出预算、header policy 等宿主策略拒绝不计入阈值。
- 同一插件在任意 startup/command/gateway hook 的 10 分钟窗口内累计 3 次严重故障后，必须在单一事务中持久转为 `quarantined` 并写 audit。
- 第三次故障所在请求保持原 fail-open/fail-closed 结果；隔离提交后刷新 gateway snapshot 并释放该插件 host，使后续请求不再执行。
- revalidate 必须复核 manifest、host compatibility、配置、安装路径/main、来源完整性/签名与市场撤销状态；成功只转为 `disabled`，绝不自动启用或激活。
- 已安装且使用废弃事件的插件迁移为有明确原因的 `disabled`，不能继续显示 enabled 但静默跳过。

## Acceptance Criteria

- [ ] Rust、SDK、合同文档和前端只表达三个允许事件族，并返回稳定的无效事件错误。
- [ ] legacy manifest 保持现有 command/hook 按需激活；显式不匹配事件不会创建 host 或运行插件。
- [ ] enabled 且显式声明 `onStartup` 的插件每进程最多激活一次，失败纳入同一严重故障统计。
- [ ] 600 秒内第三次严重故障原子持久隔离，2 次、窗口外或 policy rejection 不隔离，重启后仍不可运行或启用。
- [ ] 第三次 gateway 故障保持原 fail-open/fail-closed，后续请求使用已刷新快照且不再包含隔离插件。
- [ ] revalidate 失败维持 quarantined；成功仅到 disabled，需显式 enable 才能再次执行。
- [ ] 历史废弃事件安装具有可见迁移原因，legacy manifest 字节与签名不被改写。
- [ ] 云端覆盖事件精确匹配、legacy、startup、并发阈值、跨重启 quarantine、恢复校验和 in-flight snapshot 隔离。

## Notes

- 现有内存 circuit breaker 可保留作为单 hook 快速保护，但不能替代持久 quarantine；关联 `AIO-PENDING-023`。
