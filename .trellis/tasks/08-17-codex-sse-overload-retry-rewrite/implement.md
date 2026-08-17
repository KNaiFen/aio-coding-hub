# 实施计划：Codex Responses 过载错误码改写

## I-01 设置合同

1. 在 Rust `AppSettings` 增加 `enable_codex_responses_overload_error_rewrite` 与默认常量 `false`。
2. 把字段加入 `SettingsUpdate`、ordinary-owner previous/committed token、apply/rollback、`SettingsView` 和测试 helper。
3. 更新设置序列化/缺失字段兼容、显式 patch、并发保留和 rollback 相关测试。
4. 通过 CI 生成 Specta bindings，只接受本字段引起的有界生成变化。
5. 更新前端 settings adapter camelCase 映射、生成 update 投影和通用 settings fixtures/contract tests。

## I-02 请求时设置快照

1. 在 `HandlerRuntimeSettings` 中以 fail-closed-to-feature（读取失败即 `false`）解析开关。
2. 沿 `RequestContextParts`、`RequestContext`、middleware/failover input、`CommonCtx` 和 `CommonCtxOwned` 传递快照。
3. 增加 runtime settings 单测，证明默认关闭且显式开启可达成功 SSE 链路。

## I-03 帧感知改写器

1. 在 gateway streams 下实现独立、有界、frame-aware 的 Codex Responses SSE 改写器，复用或适度提升共享 SSE helper 的 crate 内可见性。
2. 完整帧结构化解析后只改 `/response/error/code` 的两个精确值。
3. 保留非 data SSE 行、事件边界和非目标字节；处理 LF/CRLF、多帧、跨 chunk、EOF、transport error 与缓冲超限。
4. 在 `spawn_usage_sse_relay_body` 中把改写放在 `UsageSseTeeStream` 后、下游 channel 前；关闭时走无额外缓冲的现有路径。

## I-04 成功 SSE 集成

1. 在 `handle_success_event_stream` 计算一次 native predicate 与 feature enabled 值。
2. 仅对精确启用的响应移除 `Content-Length`，将开关传入 relay；不额外改 `Content-Encoding`。
3. 保持 response fixer 两条分支行为一致，避免重复实现改写逻辑。
4. 覆盖 native path predicate 的正负范围和 response-fixer on/off。

## I-05 Codex 设置 UI

1. 在 `CliManagerCodexTabProps` 增加独立 persist callback，内部复用 `persistCommonSettings`，不刷新/写入 Codex `config.toml`。
2. 在 Codex 设置页加入“过载错误自动重试”开关和精确说明：只改写目标 Responses SSE 错误码，让 Codex 客户端自行重试。
3. 使用通用 settings saving/read-only 状态禁用控件；成功 toast 与实际持久值一致，失败不伪装成功。
4. 更新组件、page data model、service/query fixtures 测试。

## I-06 端到端回归证据

1. gateway route 测试用带 `Content-Length` 的 SSE upstream 覆盖两个目标码，断言客户端收到 `server_error`、响应头无长度、请求日志保留原始码。
2. 负例覆盖 Claude、Codex chat-completions、active/provider bridge、开关关闭、非目标事件/字段/码。
3. 运行 `$gkd-local-verify` 固定 runner；提交、推送任务分支并创建 PR。
4. 等待最终 head 的 `ci-gate` 与 `pr-title`，在 `delivery.md` 绑定完整 head SHA、CI 和所有未运行项。
