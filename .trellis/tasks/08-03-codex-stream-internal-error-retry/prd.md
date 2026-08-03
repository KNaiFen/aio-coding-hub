# Codex 流内错误拦截、重试与日志补全

## Goal

让 AIO 在 Codex Responses 的 HTTP 200 SSE 中识别可重试的终止错误，在下游响应提交前拦截并复用现有重试/熔断策略，避免 `Selected model is at capacity` 直接终止客户端；同时让每次流内错误的脱敏内容在请求日志中可追溯。

## Current Evidence

- 现有非流式假 200 检查可识别部分顶层 `error` JSON，但不覆盖已经开始转发的 SSE。
- 现有 SSE 前缀检查识别 `response.failed`，却在收到 `response.created` 等非错误帧后立即提交；错误帧分块稍晚到达时会原样进入客户端。
- 现有流结束跟踪器只识别 `error` / `response.error`，不能稳定识别并记录 `response.failed` / `response.incomplete` 的错误字段。
- sub2api PR #2481 对 `Selected model is at capacity` 做显式瞬时错误分类，并在客户端输出开始前触发 failover；AIO 当前行为与其不等价。
- CLI 的“上游响应错误规则”只负责最终 HTTP 4xx/5xx 的客户端响应改写，不参与 HTTP 200 SSE 检测、重试或熔断。

## Requirements

- 能识别 Codex Responses SSE 的 `error`、`response.error`、`response.failed`、`response.incomplete`，匹配 SSE `event` 和 JSON `type`，只读取明确的错误信封字段。
- 全局提供 `0..=5000ms` 的流内错误保护窗，默认 `500ms`；元数据帧不启动计时，首次具有真实文本、拒绝、推理摘要、工具调用参数或具体输出项的帧才启动。
- 每请求保护窗最多缓冲 1 MiB；到达成功完成、EOF、计时结束或上限后立即提交。上限超出不误判上游失败，而是放行并记录诊断标记。
- 在保护窗内命中重试关键词时丢弃本次缓冲，复用有效供应商策略的 `max_retries`、`backoff_ms`、共享瞬时重试计数、供应商切换和 `counts_toward_circuit_breaker`。
- 默认启用流内错误策略。重试关键词默认包含 `selected model is at capacity`；禁止关键词预置 `invalid_request`、`content_policy`、`policy`、`safety`、`high-risk cyber`、`not allowed`、`violat`。列表可编辑、大小写不敏感、字面子串匹配，正向词优先。
- 策略关闭、命中禁止词或未匹配时不重试，原样下传并记录；下游已经提交后的任何错误也只下传和记错，禁止混合两次生成内容。
- 在现有 HTTP 重试规则中增加可见、可编辑的 `400 + selected model is at capacity` 默认规则，不新增隐藏分类器。
- 对全局策略和每个供应商完整覆盖策略做一次去重兼容迁移；已有等价、覆盖全部 400 或显式禁用的容量规则不得重复或被覆盖。
- 每个流内错误尝试都保存结构化、限长、脱敏的事件、type、code、message、分类、命中词与处置，即使最终重试成功仍可在供应商链路查看。
- 不保存原始 SSE、普通输出或完整上游响应。错误消息必须遮蔽常见 Bearer/API Key/access token，最多 2048 个 Unicode 字符。
- 请求日志最终失败详情与供应商链路均可展示证据，并提供复制已脱敏消息的按钮；查看日志不得自动修改重试配置。
- 仅改变原生 Codex Responses SSE；其他 SSE 协议和现有非流式假 200 行为保持兼容。

## Acceptance Criteria

- [ ] `response.created` / `response.in_progress` 与稍后独立 chunk 的容量 `response.failed` 在下游提交前被拦截，客户端收到成功重试结果而不是原始错误帧。
- [ ] 同 chunk、跨 chunk、大小写变化及 SSE `event` / JSON `type` 组合均能稳定分类；正反词同时命中时正向词获胜。
- [ ] 禁止词、未知错误、策略关闭、保护窗到期后的错误均不发生不可靠重试，并保留原始下游协议行为和准确日志。
- [ ] 保护窗 0/500/5000ms、成功完成/EOF 提前提交和 1 MiB 放行均有边界测试；默认只增加约 500ms 首次真实输出延迟。
- [ ] 流内重试与 HTTP/传输重试共享同一供应商级 `max_retries` 计数，退避、熔断计数、供应商切换和严格辅助请求预算保持既有合同。
- [ ] 新旧全局配置、供应商完整覆盖、单供应商分享和完整备份均保留新字段；400 默认规则迁移幂等且不破坏用户规则。
- [ ] 重试成功和最终失败的请求日志都能显示脱敏错误证据；合成凭据不会进入 `attempts_json`、`error_details_json`、前端或导出。
- [ ] 前端策略编辑、校验、继承/覆盖和复制交互通过定向测试，TypeScript、ESLint、Prettier 与 Vite build 通过。
- [ ] GitHub Actions 完成 Rust 格式、Clippy、测试、生成绑定和桌面集成验证；PR 只提交到 `origin/main`，不合并、不发布。

## Out Of Scope

- 不改“上游响应错误规则”的职责或匹配语义。
- 不为 Claude、Gemini、Grok 或任意通用 SSE 增加保护窗。
- 不实现下游已提交后的生成回滚、拼接或透明重试。
- 不包含 `AIO-PENDING-015` Tray 悬浮窗布局任务。
