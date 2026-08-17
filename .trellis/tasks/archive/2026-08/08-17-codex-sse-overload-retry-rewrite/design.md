# 设计：Codex Responses 过载错误码改写

## 1. 决策摘要

这个功能属于 AIO 网关兼容行为，而不是 Codex CLI 自己的配置能力。因此：

- 持久状态进入 AIO `AppSettings`，字段为 `enable_codex_responses_overload_error_rewrite: bool`，默认关闭。
- 控件放在 `CliManagerCodexTab` 内，与现有 OAuth 兼容代理模式同属 AIO 管理项。
- 请求开始时由 `RuntimeSettingsMiddleware` 读取并冻结本次请求的布尔快照，沿 request/failover context 传到成功 SSE 响应链路。
- 仅复用现有 `is_native_codex_responses_event_stream_path` 判定原生 Codex 范围，避免多个近似条件漂移。
- 改写放在 `UsageSseTeeStream` 已摄取原始 chunk 之后、relay `tx` 写给客户端之前。

## 2. 数据与所有权

设置跨层链路：

```text
AppSettings + Default(false)
  -> SettingsUpdate / ordinary-owner token / SettingsView
  -> generated bindings
  -> settings adapter camelCase mapping
  -> CliManager page persistCommonSettings
  -> Codex settings Switch
```

普通 settings owner 只修改显式 patch 字段。新增字段必须进入 previous/committed token、apply/rollback 和测试 helper，确保运行时同步失败或并发写入时遵守 `.trellis/spec/aio-coding-hub/cross-layer/settings-ownership-rollback-contract.md`。这个字段没有外部副作用，不应触发 Codex config 刷新、CLI proxy 重写或 gateway 重绑。

生成 bindings 以 Rust/Specta 为源。不得把手工编辑的 `src/generated/bindings.ts` 当作源；按仓库流程使用 GitHub 生成结果并只应用该字段的有界 drift。

## 3. 请求快照

在 `HandlerRuntimeSettings` 中解析新设置：读取失败或字段缺失时为 `false`。把值依次加入：

```text
HandlerRuntimeSettings
  -> RequestContextParts / RequestContext
  -> FailoverInput / CommonCtx / CommonCtxOwned
  -> handle_success_event_stream
```

不在 relay 中重新读取设置文件。这样一个请求从开始到结束使用一致快照，并沿用已有 response-fixer 等运行时设置的结构。

## 4. SSE 改写位置

`handle_success_event_stream` 先完成 prefix 检查和 pre-commit retry 判定，再构造最终流：

```text
upstream
  -> gunzip / observers / bridge / response fixer / plugins
  -> UsageSseTeeStream（观察原始上游语义）
  -> bounded frame-aware rewrite（仅客户端视图）
  -> relay channel
  -> Codex client
```

启用条件是“设置开启 AND native Codex Responses predicate 为真”。目标启用时在构造响应前移除 `Content-Length`。

改写器维护跨 chunk 缓冲并逐个完整 SSE 帧处理。目标帧通过结构化 JSON 解析确认 `/response/error/code`，只修改这个值。实现可以把目标帧的 `data:` JSON 规范化为单行，但必须保留显式 `event:`、SSE `id:`/`retry:`/注释等非 data 行、原终止符风格和全部非目标帧。解析失败直接返回原帧。

缓冲达到上限后进入 fail-open 旁路，至少保证当前超限帧及其余字节不丢失、不重复、不被部分改写。EOF 与 transport error 处理先向仍连接的下游 flush 原始未闭合尾巴，再结束或转发错误。

## 5. 可观察性

`UsageSseTeeStream` 在改写前继续驱动 usage tracker、terminal error evidence 与请求日志，因此 `stream_internal_error.error_code` 保存上游原始码。改写器不记录 raw SSE、message 或凭据。若增加诊断，只允许记录固定 feature 名、原始固定枚举和目标固定枚举。

## 6. 非目标

- 不改 HTTP 非 2xx 错误映射。
- 不改非 SSE 或 chat-completions 响应。
- 不改 AIO pre-commit retry keyword 默认值。
- 不为 `response.error`、顶层 `error.code` 或其他容量错误扩展匹配。
- 不修改 Codex CLI 源码或写入 Codex `config.toml` 未知字段。
- 不把第三方中转站标记为 OpenAI 官方 provider。

## 7. 风险控制

- **帧被 chunk 切开**：完整帧前不解析，覆盖多种切分测试。
- **内存增长**：有界缓冲，超限 fail-open。
- **Content-Length 失效**：只在精确启用的目标响应上预先移除。
- **观测被污染**：改写严格位于 usage tee 之后。
- **范围外误改写**：复用 native predicate 并覆盖 CLI/path/bridge 负例。
- **设置并发回滚**：完整纳入 ordinary-owner token，不引入 whole-snapshot writer。
