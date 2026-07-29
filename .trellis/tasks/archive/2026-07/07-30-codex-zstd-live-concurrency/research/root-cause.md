# Root Cause Evidence

## Codex

- Codex Desktop User-Agent：`Codex Desktop/0.146.0-alpha.3.1`。
- 对应标签提交：`ff75c5b939c477c49eb1bd5248da6dab71b109d1`。
- `ModelProviderInfo::is_openai()` 以供应商名称等于 `OpenAI` 判定；同一判定允许远程压缩。
- 稳定功能 `enable_request_compression` 默认开启；OpenAI + Codex backend 的流式 Responses 请求使用 Zstd。
- 请求结构中的 `reasoning.effort` 来自当前 Turn 配置，并未因远程压缩删除。

## AIO

- 日志 trace `1785341367-3563` 显示 `/v1/responses`、`content-encoding: zstd` 和二进制请求体，模型在请求阶段为 `-`。
- `GatewayRequestBody` 只有 Identity/Gzip/Unsupported；Zstd 不解压，`introspection_json` 为 None。
- 模型和思考强度提取器本身已支持 `model`、`reasoning.effort` 等 Codex 字段。
- `gateway_active_session_count` 返回五分钟 TTL 绑定总数；现有 `ActiveRequestRegistry` 才是请求开始到完成/失败/取消期间的实时状态。
- 最新使用量规范化数据包含显式 `cache_creation_input_tokens: 0` 和非零 cache read；零值早于 Zstd 切换存在。
- `effective_input_tokens` 已根据 OpenAI、Gemini、Claude 不同输入桶语义计算未缓存输入。
