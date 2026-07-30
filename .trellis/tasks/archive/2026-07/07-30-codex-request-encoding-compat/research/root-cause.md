# Root Cause Evidence

## Codex CLI

- 本地参考仓库：`.local/codex-cli-reference/`，提交 `f9b18d04ba78266b1e802ae2f85ff5ebea1e973a`。
- 稳定功能 `enable_request_compression` 默认开启。
- 使用 Codex backend 且 `ModelProviderInfo::is_openai()` 为真时，Responses 请求可使用 Zstd。
- `remote_compaction` 控制服务端语义上下文压缩，与 HTTP `Content-Encoding` 无关。

## CCS

- 临时克隆提交：`c0ff89b9b208c092d6ef40b155403dcf290e5767`。
- 相关修复提交：`7ae4ce38ad6305a7fe4ed5d964218c0136f4133d`。
- `src-tauri/src/proxy/handlers.rs` 在 Chat Completions、Responses 和 Responses Compact 三个入口调用 `decode_codex_request_body`。
- 解码成功后删除 `Content-Encoding`、`Content-Length` 和 `Transfer-Encoding`，forwarder 从 JSON 值生成明文请求体。
- `content_encoding.rs` 支持 Gzip、X-Gzip、Deflate、Brotli、Zstd、Zst、重复头和反向堆叠解码。
- CCS 的实现没有 AIO 所需的逐层请求体上限，本任务不能直接照搬无界读取。

## AIO

- 当前基线相关提交：`13a3c6ffe183a0e19ced2e63b425b78815504c3f`。
- `GatewayRequestBody::from_wire` 为审计解压单层 Gzip/Zstd。
- `finalize_for_upstream` 在正文未修改时恢复原始编码与字节，在正文被插件修改时按原编码重压缩。
- 这能恢复模型与思考强度识别，但不能兼容只接受明文 JSON 的中转站。
- `BodyReaderMiddleware` 位于插件、模型识别、Session 与供应商尝试之前，是建立统一明文不变量的最小边界。
- `max_request_body_bytes()` 和现有 413 early error 可复用来限制每一解码层。
- `Cargo.lock` 已包含 `brotli 8.0.2`，应将其声明为直接依赖。

## External Artifacts

- CCS Repomix 快照：`/tmp/cc-switch-analysis.xml`，963 个文件，约 238 万 tokens。
- 临时 CCS 克隆和 Repomix 快照只作为本机研究材料，不进入 AIO 仓库。
