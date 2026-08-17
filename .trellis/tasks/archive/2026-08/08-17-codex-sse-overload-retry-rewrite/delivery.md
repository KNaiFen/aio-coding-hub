# 交付报告：Codex Responses 过载错误码改写

> 只写实际实现和证据，不复制计划，不缓存 PR head/base/CI URL。实时候选以 GitHub PR 和 `task.py status .trellis/tasks/08-17-codex-sse-overload-retry-rewrite` 为准。

## 结果

- 结果：等待验收
- PR：https://github.com/KNaiFen/aio-coding-hub/pull/164
- 执行者：`/root/codex_sse_overload_executor`
- 一句话结果：用户可在 CLI 管理的 Codex 设置中开启兼容改写，使原生 Responses SSE 的两类第三方中转过载码以 `server_error` 发送给 Codex 客户端，同时保留上游原始日志证据。

## 实际实现

### 用户可见行为

- Codex 设置新增“Responses 过载错误自动重试”开关，默认关闭，文案明确适用于第三方兼容中转站。
- 开启后，仅原生 Codex `/v1/responses`、`/responses`、`/v1/codex/responses` SSE 中完整的 `response.failed` 事件会参与判断；精确错误码 `server_is_overloaded` 和 `slow_down` 会在下游视图中变为 `server_error`。
- 保存复用 AIO 通用设置写入，不修改 Codex `config.toml`；开关由回读设置控制，保存失败或保存中不会显示虚假切换状态。

### 内部机制

- 新设置进入 `AppSettings` schema 61、ordinary settings owner token、请求时 `HandlerRuntimeSettings` 快照、failover common context、Specta settings DTO 和前端 adapter/fixtures。
- `CodexResponsesOverloadErrorRewriter` 按完整 SSE 帧工作，支持 LF、CRLF、跨 chunk 与多帧；仅修改 `/response/error/code`。目标帧的 JSON `data` 会重序列化为单行，其他 SSE 行及帧结束符保留。
- 单帧待处理缓冲上限为 1 MiB；超限时先原样发出已缓冲和当前剩余字节，再对该流永久进入 fail-open 原样旁路。EOF 或 transport error 前会尽力原样 flush 未闭合尾部。
- 改写位于 `UsageSseTeeStream` 摄取之后、下游 channel 之前，因此 usage、attempt 和 request log 继续解析原始上游码。功能开启时移除 `Content-Length`，不改变 HTTP 状态、pre-commit 重试、provider failover、response fixer 或插件语义。

## AC 证据

| AC | 结果 | 证据 |
|---|---|---|
| AC-01 | 通过 | `disabled_codex_responses_overload_rewrite_preserves_body_and_content_length`、迁移缺字段默认关闭测试，及前端/MSW 默认值测试。 |
| AC-02 | 通过 | 改写器双目标码单测；原生路径谓词既有三条正例与 CLI/path/bridge 负例；route 集成测试验证 `/v1/responses`。 |
| AC-03 | 通过 | `handles_chunk_boundaries_crlf_and_multiple_frames` 覆盖跨 chunk、CRLF、keepalive、相邻非目标帧；双目标码测试覆盖 LF 和其他 JSON 字段。 |
| AC-04 | 通过 | `leaves_all_non_targets_byte_exact`、超限旁路、未闭合 EOF 测试；原生路径谓词负例覆盖非 Codex、非 Responses、active bridge 和 bridged provider。 |
| AC-05 | 通过 | `codex_responses_overload_rewrite_changes_client_view_but_keeps_log_evidence` 断言下游 `server_error`、无 `Content-Length`，日志保留原始 `server_is_overloaded` 与 message。 |
| AC-06 | 通过 | `CodexTab.test.tsx` 覆盖可读、切换、受控回读和保存中禁用；`CliManagerPage.test.tsx` 覆盖通用设置 patch、回读 toast 且不触发 Codex config 刷新。 |
| AC-07 | 通过 | settings migration/runtime/ordinary-owner rollback 测试、adapter 测试、Specta 生成绑定云端一致性检查、fixtures/MSW contract 测试。 |
| AC-08 | 通过 | 固定本地验证返回 `local_ready`；GitHub `frontend`、`rust`、contracts、CodeQL、`ci-gate` 与 `pr-title` 在实现提交阶段全部通过，交付状态最终 head 仍按流程重新验证。 |

## 关键位置

| 文件或符号 | 实际变化 | 设计原因 |
|---|---|---|
| `src-tauri/src/gateway/streams/codex_responses_overload_rewrite.rs:CodexResponsesOverloadErrorRewriter` | 有界、帧感知、fail-open 的精确 JSON 指针改写 | 不依赖 chunk 边界，异常输入无损透传。 |
| `src-tauri/src/gateway/streams/usage_tee.rs:spawn_usage_sse_relay_body` | 原始 tee 后按开关改写下游 chunk，并在结束前 flush 尾部 | 保持原始观测证据与既有 relay 背压/断开语义。 |
| `src-tauri/src/gateway/proxy/handler/failover_loop/response/success_event_stream.rs:handle_success_event_stream` | 限定 native Codex Responses 范围并移除目标响应长度 | 避免 bridge、其他 CLI/路径和固定长度响应误处理。 |
| `src-tauri/src/app/settings_service.rs:SettingsUpdate` | 普通 owner 写入、省略保持旧值、CAS rollback | 保持 settings 并发所有权合同。 |
| `src/components/cli-manager/tabs/CodexTab.tsx:CodexGatewayCompatibilitySection` | 展示受控开关与第三方兼容文案 | 功能属于 AIO 网关设置，不属于 Codex 配置文件。 |

## 计划偏移

- 原计划建议复用一次 native predicate；实际只为新功能计算自身 predicate，既有 gzip guard 与终态观测继续使用基线中的独立判定。一次完整 CI 暴露既有 gzip 回归测试失败后采用此收缩，避免本任务重构无关敏感路径；功能范围和判定语义未变化，下一轮完整 Rust 测试通过。
- 云端格式 artifact 仅包含 5 个 Rust 文件的 rustfmt 换行，没有逻辑或绑定变化，已原样应用。Specta 以 Rust 为源生成的 settings 字段与已提交绑定完全一致，没有额外 drift。

## 验证

| 类型 | 命令或检查 | 结果 | 说明 |
|---|---|---|---|
| 本地 | `node scripts/check-local-verification.mjs --base cbdc1d32cf24df7da553c1d027ed5d7d266b89b5` | 通过 | `local_ready`；runner selftest、cloud-only contract、全 diff、未跟踪空白和变更 Node 语法检查通过，changed Node files 为空。 |
| GitHub | `frontend` | 通过 | 前端格式、类型和受影响测试通过。 |
| GitHub | `rust` | 通过 | 云端生成绑定/rustfmt、Clippy、2916 项库测试及 Rust 集成测试通过。 |
| GitHub | contracts / CodeQL | 通过 | contracts、JavaScript/TypeScript CodeQL、Rust CodeQL 通过。 |
| GitHub | `ci-gate` / `pr-title` | 通过 | 实现提交阶段终态通过；交付状态最终 head 由 `$gkd-ci-monitor` 再确认。 |
| 人工 | 桌面 UI 与真实第三方中转流量 | 未运行 | 常规 checkout 合同禁止本地依赖、开发服务器和运行时 UI；由自动化组件/route 测试覆盖，验收可按需人工抽查。 |

## 合同与影响

- 测试：新增 SSE 改写器单测、gateway route 下游/日志集成测试、settings 迁移/rollback/runtime 测试及 Codex UI/page/adapter/fixture 测试；更新 schema 61 集成断言。
- 现行文档与机器合同：任务 PRD/design/implement 无需改动；生成 bindings 已通过 GitHub Rust/Specta 一致性门。
- API、兼容性与迁移：`SettingsView` 和 `SettingsUpdate` 增加布尔字段；schema 从 60 升至 61，旧配置缺字段时默认关闭。关闭时保持字节和响应头现有行为。
- 数据、配置、安全与隐私：只持久化布尔设置；改写器不新增 raw SSE、message、凭据或 payload 日志，原日志路径保持原始错误证据。
- 发布与回滚：无数据库迁移和外部依赖；回退本任务提交即可，已写入 schema 61 的配置在旧版本由既有兼容策略处理。

## 风险与审查重点

- 剩余风险：目标帧 JSON 会规范化为单行，语义及其他字段保持但字节不再完全相同；1 MiB 超限后该流永久旁路，因此后续目标帧不再改写，以无损和内存上限优先。
- main 重点审查：`src-tauri/src/gateway/streams/usage_tee.rs:spawn_usage_sse_relay_body`：确认 raw tee、尾部 flush、背压和客户端断开顺序；`src-tauri/src/gateway/streams/codex_responses_overload_rewrite.rs:CodexResponsesOverloadErrorRewriter`：确认边界和永久旁路策略。
- 未完成项：未做真实桌面 UI/第三方中转人工验证；不阻塞自动化 AC，由 main 决定是否在验收时补充。

## 阻塞快照

无。
