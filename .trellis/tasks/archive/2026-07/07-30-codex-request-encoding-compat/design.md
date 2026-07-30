# Technical Design

## Root Cause

Codex 的 `remote_compaction` 是上下文语义压缩；`enable_request_compression` 则是独立的 HTTP 请求体压缩功能。供应商被识别为 `OpenAI` 且使用官方 Codex 登录态时，普通 Responses 请求默认可用 Zstd 编码。

AIO 当前只为内部审计解压 Gzip/Zstd。未修改正文会恢复原始压缩字节，插件改写后会重新压缩。因此，上游中转站不支持请求体压缩时仍会把压缩字节当 JSON 解析并报错。CCS 在三个 Codex JSON 入口先解压，再删除失效实体头并以明文转发。

## Route Boundary

- 在完整请求体读入后、`GatewayRequestBody` 构建前执行 Codex 规范化。
- 仅处理 POST 请求，并按路径段匹配以下后缀：
  - `responses`
  - `responses/compact`
  - `chat/completions`
- 匹配前忽略查询串和尾斜杠；允许 `/v1/` 和任意合法嵌套前缀。
- 非目标请求继续进入现有 `GatewayRequestBody::from_wire`，保持原始编码透传和插件改写后的重编码行为。

## Content-Encoding Decoder

- 读取所有 `Content-Encoding` 头，按出现顺序拼接并以逗号拆分，去除空白并转为小写。
- 忽略 `identity`；其余有效编码层不得超过 8。
- 在开始解码前验证所有编码均属于 `gzip`、`x-gzip`、`deflate`、`br`、`zstd`、`zst`，避免部分解码后才发现未知层。
- 按编码声明的反序逐层解码。
- Gzip 和 Zstd 复用有界读取实现；Deflate 先尝试 zlib，再尝试 raw；Brotli 使用直接依赖 `brotli 8.0.2`。
- 每层输出都用现有请求体上限约束，中间层和最终层都不能越界。
- 解码错误只分类为 `InvalidEncoding` 或 `BodyTooLarge`，不向客户端或持久日志暴露具体解码器消息。

## Normalized Request State

成功解码后，用明文字节替换请求上下文正文，并删除：

- `Content-Encoding`
- `Content-Length`
- `Transfer-Encoding`

随后照常构建 `GatewayRequestBody`。因为规范化状态为 Identity，插件、模型识别、Session 补全、隐私过滤、供应商尝试与重试共享同一明文状态，最终发送阶段不会恢复或重新生成压缩编码。

## Failure Contract

- 未知编码、损坏流或编码层数超过 8：HTTP 400，`GW_INVALID_REQUEST_CONTENT_ENCODING`。
- 任一解码层输出超过上限：复用现有 HTTP 413 正文过大错误。
- 两类错误都在供应商选择与尝试之前终止，不触发重试、上游请求、失败计数或熔断。
- 公开消息只描述请求编码无效或正文过大；日志不包含正文、凭证或内部解码错误。

## Error-Code Contract

新增 `GW_INVALID_REQUEST_CONTENT_ENCODING`：

- Rust 错误码枚举与字符串双向映射。
- Early error 分类为不可重试客户端错误，HTTP 400。
- 状态覆盖表映射为 400。
- 前端常量、短标签与诊断说明。
- 错误码一致性脚本必须通过。

## Compatibility

- 不改变响应处理和 `Accept-Encoding: identity`。
- 不改变 `remote_compaction`、认证、供应商命名、路由选择、插件协议或数据库。
- 不对任意 JSON 请求全局解压，避免改变非 Codex 客户端既有传输语义。
