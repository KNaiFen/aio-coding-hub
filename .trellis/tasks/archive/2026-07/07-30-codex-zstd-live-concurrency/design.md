# Technical Design

## Root Causes

1. Codex Desktop 把自定义供应商命名为 `OpenAI` 后同时启用远程压缩和稳定的请求压缩功能。流式 Responses 请求使用 `Content-Encoding: zstd`，请求 JSON 仍包含 `model` 与 `reasoning.effort`。
2. AIO 的请求体抽象只支持 Identity 和 Gzip。Zstd 被当作不支持的原始字节透传，因此 JSON 解析失败；响应完成时能恢复模型，但请求级思考强度无法恢复。
3. 首页并发使用 `SessionManager::active_count`，实际统计五分钟 TTL 内的会话复用绑定，不代表正在处理的请求。
4. `cache_write_tokens: 0` 是上游明确值。现有 `effective_input_tokens` 已按协议计算未缓存输入，不能把它改写成缓存写入。

## Request Body Encoding

- 在 Rust 依赖中增加 `zstd 0.13`，沿用 Codex CLI 当前使用的主版本。
- 将请求体编码扩展为 `Zstd`；只接受单一、不区分大小写的 `zstd`，复合编码继续归为 Unsupported。
- 新增带上限的 Zstd 解压和压缩助手，使用现有 `max_request_body_bytes()` 作为解压输出与重编码结果上限。
- 成功解压后，模型推断、Session 补全和插件钩子读取语义 JSON。请求未变更时恢复原编码头并复用原始压缩字节；发生变更时重新压缩为 Zstd。
- 解压失败或超限时记录警告并保留原始透传。若无法重压缩，则发送 Identity JSON 并移除编码头，保持 Gzip 的既有降级策略。

## Live Concurrency

- 不新增 IPC。继续使用 `active_request_logs_snapshot` 与 `gateway:request_signal`，开始和终止信号经现有 200ms 合并窗口刷新。
- 在前端 active request 服务层集中实现推理请求判定和去重：
  - Claude：POST `/v1/messages`
  - Codex：POST Responses 与 Responses Compact 路径
  - Grok：POST Chat Completions 路径
  - Gemini：POST `generateContent` / `streamGenerateContent` 路径
- Session 键为规范化后的 `cli_key:session_id`；缺失 Session ID 时使用 `cli_key:trace_id`，避免漏计真实请求。
- active snapshot 查询错误不再伪装为空数组；Feed 暴露可用性。首页加载/错误显示 `--`，成功空数组显示 `0`。
- 首页请求面板显式开启该指标并从传入的实时快照计算；独立日志页默认不渲染。
- TTL 活跃 Session IPC、列表以及路由切换逻辑全部保留。

## Usage Presentation

- 不增加列、绑定或统计公式。
- 复用后端 `effective_input_tokens`，在持久记录、实时卡片和详情页标为“未缓存输入”。
- 将现有“缓存创建”展示标为“缓存写入”，并用 `resolveCacheCreationDisplay` 保留 5m/1h 桶优先级。
- 缓存写入字段全缺失时显示 `—`，任一字段明确为 0 时显示 `0`。

## Compatibility

- 新行为只影响新请求的检查结果和首页展示；历史记录保持原样。
- 不改变上游请求认证、路由、重试、熔断、会话复用及缓存成本计算。
- Release 使用既有 Fork 标签流水线，版本为 `0.60.34`。
