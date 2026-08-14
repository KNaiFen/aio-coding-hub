# 技术设计：TUI 请求时间与 CLI 监听切换修复

## 边界

本任务修复两个可独立验证但共享同一交付的回归：

1. TUI request card 的状态到时间字段投影错误。
2. CLI Manager 网络设置的 gateway lifecycle 自锁与一次性令牌 UI 所有权错误。

不改变 public IPC、数据格式或鉴权模型。新增 cross-layer 规范只记录现有安全语义和本次锁定的 UI/锁所有权规则。

## TUI 状态投影

`ObserverRequest` 已经提供可靠的 `state`：

```text
Active   -> duration_ms = now - created_at, ttfb_ms = None
Terminal -> duration_ms = persisted total, ttfb_ms = persisted first byte
```

`request_card_lines` 仅在 route summary 选择显示值：

```text
match state {
  Active   => duration_ms,
  Terminal => ttfb_ms,
}
```

该选择不得复用 `duration_ms - ttfb_ms`，不得改变 `output_tokens_per_second`，也不修改详情页的独立“耗时/首字”行。Active 即使收到防御性的非空 TTFB 也以 state 为准。

## Gateway 生命周期锁

当前死锁路径：

```text
settings_set_impl_with_gateway
  acquires gateway_lifecycle_lock
  -> stop/start/rebind
  -> sync_cli_proxy_for_settings
       acquires gateway_lifecycle_lock again
```

目标结构沿用已有 `*_unlocked` 命名与所有权模式：

```text
sync_cli_proxy_for_settings
  acquire lifecycle lock
  -> sync_cli_proxy_for_settings_unlocked

settings_set_impl_with_gateway
  if it already owns lifecycle guard:
    -> sync_cli_proxy_for_settings_unlocked
  otherwise:
    -> sync_cli_proxy_for_settings
```

内部 helper 只省略 lifecycle lock acquisition，不省略 status reread、base origin normalization、blocking CLI sync、失败计数或日志。外层 guard 继续覆盖 gateway convergence 与关联 proxy sync，避免在重绑和配置写入之间插入另一个 lifecycle transition。

测试应对锁所有权分支使用 `tokio::time::timeout`。如果具体 Wry entry 无法在 unit runtime 调用，抽取泛型协调函数或注入窄接口，使测试执行真实的“已持锁 -> unlocked sync”分支；禁止仅搜索函数名或断言源码文本。

## 前端令牌所有权

令牌 dialog/controller 放在 `CliManagerPage` 或其 data-model 层，生命周期覆盖全部 tab。建议职责：

- 保存成功和初始 pending-token 恢复调用同一个 serialized/deduplicated reveal 操作。
- reveal promise 正在运行时，后续调用复用或跳过同一 promise，不并发触发后端 `pending.take()`。
- token/dialog state 不放在只渲染 General tab 时存在的 `NetworkSettingsCard` 中。
- dialog 在页面根部渲染，因此保存完成时无论当前 tab 是否变化都能展示。
- rotate、copy、acknowledge 与错误 toast 由同一 controller 管理。

`NetworkSettingsCard` 只负责监听模式 draft、校验、保存调用和网络状态展示。成功的非回环保存触发 controller reveal；保存失败或 `null` 则回滚 draft，不 reveal。

## UI 状态机

```text
idle
  -> select new mode
  -> applying (select disabled + compact progress feedback)
     -> success: apply canonical settings -> reveal if non-loopback -> idle
     -> null/error: rollback to canonical settings -> idle

token dialog open
  -> copy (no state loss)
  -> acknowledge: backend confirm -> close
  -> close without acknowledge: preserve existing rotate-required rule
```

外部 `settings` prop 到 draft 的同步使用 effect，不能在 render body dispatch。Effect 不得覆盖当前仍在提交中的用户选择；以 canonical response 或明确失败回滚作为提交边界。

## 安全与兼容性

- 后端仍只持久化 token digest/metadata，不持久化或记录明文。
- 不把一次性 reveal 改成重复 reveal；可靠性通过前端 owner 生命周期解决。
- 不改变非 loopback peer 的 Bearer 校验、401 行为或内部 loopback 例外。
- settings ownership token、CAS rollback、gateway runtime recovery 与 CLI profile/catalog 合同保持不变。
- 不生成新的 Specta binding，也不改变命令 registry。

## 现行规范

- 更新 `local-observer-tui-contract.md`：明确 Active/Terminal 卡片时间选择。
- 新建 `gateway-listen-token-contract.md`：记录 listen/rebind 生命周期锁、token 安全语义、一次性 reveal owner 与失败回滚。
- 在 cross-layer `index.md` 添加入口与对应 pre-development/quality check。

## 回滚

本任务没有数据迁移。回滚 PR 会恢复旧 UI/锁结构；已生成的 token 仍遵循既有 digest 与轮换语义，不需要清理数据。若新 frontend owner 出现问题，可以独立回退前端提交而保留后端死锁修复和 TUI 修复。
