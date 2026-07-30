# Codex 请求压缩兼容

## Goal

将 Codex 压缩 JSON 请求在网关边界有界解码并以明文转发，兼容不支持 HTTP 请求压缩的中转站。

## Requirements

- 仅规范化 Codex 的 POST JSON 端点：`responses`、`responses/compact`、`chat/completions`；路径匹配支持 `/v1/`、嵌套前缀和尾斜杠。
- 支持 `gzip`、`x-gzip`、`deflate`、`br`、`zstd`、`zst`、重复 `Content-Encoding` 头和按 HTTP 语义反向解码的堆叠编码；`identity` 不产生解码层。
- `deflate` 先按 zlib 流解码，失败后再尝试 raw deflate。
- 有效编码层最多 8 层，每一层解码结果都受现有请求体大小上限保护。
- 成功解码后，插件、模型识别、Session、隐私过滤、请求日志、供应商选择和重试都只使用明文 JSON。
- 发往上游的规范化 Codex 请求必须删除 `Content-Encoding`、`Content-Length` 和 `Transfer-Encoding`，不得恢复原始压缩字节或重新压缩。
- 未知编码、损坏压缩流和超过编码层数上限时本地返回结构化 400；解压结果超过请求体上限时返回现有结构化 413。
- 编码失败不得触发上游请求、供应商熔断或重试，错误响应和日志不得包含请求正文或底层解码器细节。
- 非目标请求继续使用现有压缩透传或重编码语义。
- 新增公开错误码 `GW_INVALID_REQUEST_CONTENT_ENCODING`，同步后端状态映射、前端常量、短标签和诊断说明。
- 新增后端规范，区分远程上下文压缩与 HTTP 请求体压缩，并记录规范化合同。

## Acceptance Criteria

- [x] 所有受支持编码、别名、重复头和堆叠编码均能在 Codex 目标端点解码，发往上游的正文可直接按 JSON 解析且无实体编码头。
- [x] `responses`、`responses/compact` 和 `chat/completions` 的路径变体均受保护，非 POST 或非目标路径不改变现有行为。
- [x] 插件或隐私过滤修改正文后仍以明文发送；模型、思考强度、Session 和请求日志识别继续工作。
- [x] 未知编码、损坏数据和超过 8 层返回 `GW_INVALID_REQUEST_CONTENT_ENCODING` 与 HTTP 400，且上游尝试数为零。
- [x] 任一解码层超过请求体上限返回现有正文过大错误与 HTTP 413，且上游尝试数为零。
- [x] 错误与请求日志不记录正文、密钥或解码器内部错误。
- [x] 非 Codex 压缩请求的原始透传或重编码回归测试通过。
- [x] 前后端错误码合同检查、TypeScript 检查、Lint、格式化和 Rust 测试通过；本机与 CI 均已验证。

## Notes

- 不修改 `remote_compaction`、OpenAI 供应商命名、认证方式、响应解压或 `Accept-Encoding: identity`。
- 不新增供应商开关，不做失败后的明文重试，不修改 IPC、数据库或配置结构。
- 原实现范围不包含版本升级、发布、推送或 PR；后续按用户单独授权完成 `0.60.35` 提交、推送与发布。
- 保留现有未跟踪的 `.trellis/workspace/KNaiFen/`。
