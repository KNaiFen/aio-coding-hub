# 插件 Hooks

Hooks 是网关和日志 pipeline 中稳定的扩展点。Plugin API v1 刻意保持 active surface 小而明确，让社区插件能清楚判断调用时机、capability 边界和 mutation 行为。

默认 v1 hook timeout: 5000 ms.
默认 vNext hook timeout: 5000 ms.
默认 v1 failure policy: `fail-open`.

`fail-open` 的实际语义是可用性优先：hook 失败、超时、输出超预算或输出 payload 无效时，宿主会记录诊断并继续原路径。对 `log.beforePersist`，这表示保留原始日志继续入库。显式声明 `fail-closed` 时，request、response 和 stream hook 会返回安全错误；`log.beforePersist` 在 hook 失败、熔断或最终 payload 不是可解析 JSON object 时会放弃该条请求日志，同时保留插件诊断。熔断状态按插件和 hook 名称分别计算。

Timeout 是宿主为每次 hook invocation 选择的执行预算。Gateway pipeline 会把本次预算显式传给 runtime executor；Extension Host gateway hooks 使用同一个预算完成 activation、hook dispatch 和 runtime cleanup，不在 executor 内再叠加固定上限或 grace window。

Reserved hooks 在宿主实现对应调用点前，会被 manifest validation 拒绝：

- `gateway.request.received`
- `gateway.request.beforeProviderResolution`
- `gateway.response.headers`

## Resource Budgets

Plugin hook contexts are host-trimmed and budget-trimmed. Extension Host manifests do not declare `permissions`; the host decides visible context from the hook, granted capability, runtime policy, and context budget. The gateway may accept request bodies larger than the plugin context budget, but plugins only receive bounded visible context. When a body, stream chunk, log message, or normalized message list exceeds the plugin budget, the host truncates the visible value and marks the public camelCase flag: `bodyTruncated`, `normalizedMessagesTruncated`, `chunkTruncated`, or `messageTruncated`. Extension Host v1 also exposes the previous snake_case names, including `context.hook_name` and `context.trace_id`, as runtime compatibility aliases that resolve to the same values; JSON-RPC payloads only contain the canonical camelCase fields.

Hook outputs are also bounded. Oversized body, stream, log, or header mutations are rejected with `PLUGIN_OUTPUT_TOO_LARGE`; the pipeline then applies the hook failure policy and circuit-breaker behavior.

## Observability And Replay

宿主会把 hook 执行结果写入 `plugin_hook_execution_reports`。这张表是 host-owned runtime evidence，不属于 Plugin API v1 的插件可调用能力。它记录 plugin id、trace id、hook name、runtime kind、status、duration、failure kind、failure policy、circuit state、context/output budget summary、mutation summary 和 replayable reason。

开发者可以通过宿主命令 `plugin_export_replay_fixture` 导出 trace replay fixture。fixture 会包含 request log metadata、attempts 和 matching runtime reports。当前 request logs 不持久化完整 request/response body，所以 fixture 可能只包含 body notes；复现 body 需要插件作者提供本地 fixture。

Replay 支持按 hook 分层：

| Hook | Runtime report | Replay fixture 用途 | 限制 |
| --- | --- | --- | --- |
| `gateway.request.afterBodyRead` | 记录 completed、failedOpen、failedClosed、budgetRejected、policyRejected、circuitOpen 等状态 | 复现读取 body 后的 prompt rewrite、redaction、block 和 header/body mutation | 需要本地补齐未持久化 request body |
| `gateway.request.beforeSend` | 记录最终 upstream request 前的 hook 结果 | 复现 provider resolution 后的最终 body/header mutation | 只表示 semantic decoded body，不保证完整 wire-level encoding |
| `gateway.response.chunk` | 记录 chunk 级执行结果和 timeout/budget 状态 | 复现单个有界 chunk 或滑动窗口场景 | 不代表完整响应；需要 streamed fixture |
| `gateway.response.after` | 记录 non-streaming response body hook 结果 | 复现完整响应检查、替换或阻断 | 只适用于 non-streaming response body |
| `gateway.error` | 记录 gateway-generated error response hook 结果 | 复现错误响应脱敏或改写 | 不处理 provider success response |
| `log.beforePersist` | 记录日志入库前脱敏结果 | 复现日志 redaction 和 mutation summary | fail-open 在失败或非法 payload 时保留原始日志；显式 fail-closed 会放弃该条请求日志 |

## Hook 矩阵

| Hook | 阶段 | Visible context labels | Mutation labels | Mutation fields | Context fields |
| --- | --- | --- | --- | --- | --- |
| `gateway.request.afterBodyRead` | 读取 request body 后、发送 upstream provider 前。 | `request.meta.read`, `request.header.read`, `request.header.readSensitive`, `request.body.read` | `request.header.write`, `request.body.write` | `headers`, `requestBody` | `traceId`, `request.cliKey`, `request.method`, `request.path`, `request.query`, `request.headers`, `request.body`, `request.bodyTruncated`, `request.requestedModel`, `request.normalizedMessages`, `request.normalizedMessagesTruncated` |
| `gateway.request.beforeSend` | provider resolution 后、发送 upstream provider 前。 | `request.meta.read`, `request.header.read`, `request.header.readSensitive`, `request.body.read` | `request.header.write`, `request.body.write` | `headers`, `requestBody` | `traceId`, `request.cliKey`, `request.method`, `request.path`, `request.query`, `request.headers`, `request.body`, `request.bodyTruncated`, `request.requestedModel`, `request.normalizedMessages`, `request.normalizedMessagesTruncated` |
| `gateway.response.chunk` | 每个有边界的 streaming response chunk。 | `stream.inspect` | `stream.modify` | `streamChunk` | `traceId`, `stream.sequence`, `stream.chunk`, `stream.chunkTruncated` |
| `gateway.response.after` | 完整 non-streaming upstream response body 可用后。 | `response.header.read`, `response.body.read` | `response.header.write`, `response.body.write` | `headers`, `responseBody` | `traceId`, `response.status`, `response.headers`, `response.body`, `response.bodyTruncated` |
| `gateway.error` | gateway error response materialization 后、发送前。 | `response.header.read`, `response.body.read` | `response.header.write`, `response.body.write` | `headers`, `responseBody` | `traceId`, `response.status`, `response.headers`, `response.body`, `response.bodyTruncated` |
| `log.beforePersist` | gateway request log persistence 前。 | `log.redact` | `log.redact` | `logMessage` | `traceId`, `log.message`, `log.messageTruncated` |

## gateway.request.afterBodyRead

- 阶段：读取 request body 后、发送 upstream provider 前。
- 默认超时：5000 ms。
- 默认失败策略：`fail-open`。
- Visible context labels：`request.meta.read`、`request.header.read`、`request.header.readSensitive`、`request.body.read`。
- Mutation labels：`request.header.write`、`request.body.write`。
- Mutation fields：`headers`、`requestBody`。
- Provider-neutral field：`request.normalizedMessages`。
- Truncation fields：`request.bodyTruncated`、`request.normalizedMessagesTruncated`。

这个 hook 适合 prompt optimization、privacy filtering 和 request-body checks。只有宿主为当前 hook 调用提供 body context 时，插件才会看到 `request.body` 和 `request.normalizedMessages`；插件不能通过 Extension Host manifest 申请 `request.body.read`。

Claude-style fixture：

```json
{
  "messages": [
    {
      "role": "user",
      "content": [{ "type": "text", "text": "hello claude" }]
    }
  ]
}
```

Codex/OpenAI Responses-style fixture：

```json
{
  "input": [
    {
      "type": "message",
      "role": "user",
      "content": [{ "type": "input_text", "text": "hello codex" }]
    }
  ]
}
```

这两种结构的 normalized context 都会包含类似条目：

```json
{
  "request": {
    "normalizedMessages": [
      { "role": "user", "text": "hello codex", "source": "openai.responses.input_text" }
    ]
  }
}
```

## gateway.request.beforeSend

- 阶段：provider resolution 后、发送 upstream provider 前。
- 默认超时：5000 ms。
- 默认失败策略：`fail-open`。
- Visible context labels：`request.meta.read`、`request.header.read`、`request.header.readSensitive`、`request.body.read`。
- Mutation labels：`request.header.write`、`request.body.write`。
- Mutation fields：`headers`、`requestBody`。
- Provider-neutral field：`request.normalizedMessages`。
- Truncation fields：`request.bodyTruncated`、`request.normalizedMessagesTruncated`。

它在当前 attempt 的 provider selection、auth/header preparation、request body sanitizers 和 protocol rectifiers 后执行，紧贴 gateway 向 upstream provider 发送 bytes 前。插件必须保证最终 upstream request-body 或 request-header mutation 时，使用这个 hook。

这个 hook 看到的是 semantic decoded request body content。如果插件修改 body，gateway 会更新最终 upstream body，并按需要移除或重新计算 wire-level length/encoding 语义。未改变的请求会尽量保留原始 passthrough body。

## gateway.response.chunk

- 阶段：每个有边界的 streaming response chunk。
- 默认超时：5000 ms。
- 默认失败策略：`fail-open`。
- Visible context labels：`stream.inspect`。
- Mutation labels：`stream.modify`。
- Mutation fields：`streamChunk`。
- Context fields：`traceId`、`stream.sequence`、`stream.chunk`、`stream.chunkTruncated`。

这个 hook 接收有边界的 streaming chunks，而不是完整响应。需要完整 response bodies 的插件，应在 non-streaming requests 中使用 `gateway.response.after`。

## gateway.response.after

- 阶段：完整 non-streaming upstream response body 可用后。
- 默认超时：5000 ms。
- 默认失败策略：`fail-open`。
- Visible context labels：`response.header.read`、`response.body.read`。
- Mutation labels：`response.header.write`、`response.body.write`。
- Mutation fields：`headers`、`responseBody`。
- Context fields：`traceId`、`response.status`、`response.headers`、`response.body`、`response.bodyTruncated`。

这个 hook 适合 non-streaming response redaction、warnings 或 response blocking。

## gateway.error

- 阶段：gateway error response materialization 后、发送前。
- 默认超时：5000 ms。
- 默认失败策略：`fail-open`。
- Visible context labels：`response.header.read`、`response.body.read`。
- Mutation labels：`response.header.write`、`response.body.write`。
- Mutation fields：`headers`、`responseBody`。
- Context fields：`traceId`、`response.status`、`response.headers`、`response.body`、`response.bodyTruncated`。

这个 hook 用于脱敏或改写 gateway-generated error responses，不应处理 provider success responses。

## log.beforePersist

- 阶段：gateway request log persistence 前。
- 默认超时：5000 ms。
- 默认失败策略：`fail-open`。
- Visible context labels：`log.redact`。
- Mutation labels：`log.redact`。
- Mutation fields：`logMessage`。
- Context fields：`traceId`、`log.message`、`log.messageTruncated`。

这个 hook 用于 request logs 入队或写入前的不可逆脱敏。

默认失败边界是 fail-open：如果 hook 执行失败，或返回的 `logMessage` 不是宿主能解析回 request log payload 的 JSON object，宿主会保留原始日志继续入库。隐私插件需要强制日志脱敏时，应在 `log.beforePersist` 明确声明 `failurePolicy: "fail-closed"`；此时 hook 失败、熔断或最终 payload 无效会让宿主放弃该条请求日志，而不是回退到原始敏感日志。诊断事件和执行报告仍会保留。
