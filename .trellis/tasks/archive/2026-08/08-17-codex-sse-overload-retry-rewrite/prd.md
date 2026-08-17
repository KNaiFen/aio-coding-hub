# 为 Codex Responses 改写过载错误码

## Plan Status

- Implementation authorization: confirmed by the user
- Confirmation date and summary: 2026-08-17；用户确认上游是 OpenAI-compatible 第三方中转站，并授权为 AIO 增加可开关的错误码改写功能
- Confirmed coverage: AIO 设置持久化、CLI 管理中的 Codex 设置开关、原生 Codex Responses SSE 定向改写、回归测试
- Planning revision: scope frozen by the authorization commit；完整规划 SHA 由 `task.py delegate` 记录
- Execution route: delegated worktree
- Migrated from direct-main record: none

## Material Facts, Assumptions, and Open Questions

| Item | Source | Status / closure condition |
|---|---|---|
| Codex CLI 将 Responses SSE 的 `server_is_overloaded` / `slow_down` 与 `server_error` 归入不同重试类别 | 用户指定 session `01a00db2-abc9-7050-9137-599d9f0c988d` 的上一轮分析及其中固定 Codex 源码链接 | confirmed |
| 目标上游不是 OpenAI 官方服务，而是 OpenAI-compatible 第三方中转站 | 2026-08-17 用户决定 | confirmed |
| 开关位于“CLI 管理 -> Codex 设置” | 2026-08-17 用户决定 | confirmed |
| 开关控制 AIO 网关行为，不写入 Codex `config.toml` 的 `[features]` | 当前设置与网关所有权合同 | confirmed；使用 AIO `AppSettings` |
| 新设置的兼容默认值 | 现有行为保持原则 | confirmed；默认关闭 |
| 是否改 HTTP 状态码或非流式响应 | 目标机制与上一轮结论 | confirmed；均不修改 |

No material open question remains.

## Goal

允许用户在 AIO 的 Codex 设置中显式开启一个兼容功能：当原生 Codex `/responses` SSE 返回 `response.failed`，且 `/response/error/code` 精确为 `server_is_overloaded` 或 `slow_down` 时，在发给 Codex 客户端前改写为 `server_error`，使客户端按其既有策略自动重试。

## Requirements

- 新增 AIO 设置 `enable_codex_responses_overload_error_rewrite`，默认 `false`；缺失旧配置按关闭处理。
- 开关显示在“CLI 管理 -> Codex 设置”，由 AIO 普通设置写入链路持久化，不向 Codex `config.toml` 写未知字段。
- 仅在开关开启且同时满足以下条件时启用改写：
  - `cli_key == "codex"`；
  - 路径为原生 `/responses` 事件流范围；
  - 没有 active protocol bridge；
  - provider 未被 bridge；
  - SSE 事件为 `response.failed`；
  - JSON 指针 `/response/error/code` 精确为 `server_is_overloaded` 或 `slow_down`。
- 目标错误码仅改为 `server_error`；HTTP 状态、事件类型、响应 ID、消息和其他 JSON 字段保持语义不变。
- 非目标 CLI、chat-completions、bridge、`response.error`、顶层 `error.code`、其他错误码、无效 UTF-8/JSON 与非完整尾帧必须 fail-open 原样透传。
- 改写器必须按完整 SSE 帧工作，支持 LF 与 CRLF、跨 chunk 帧和同一 chunk 多帧；缓冲有明确上限，EOF 或上游错误前不得静默丢失尚未闭合的数据。
- 开启改写的目标响应在构造下游响应前移除失效的 `Content-Length`；不额外改变 `Content-Encoding`，既有 gunzip 分支除外。
- AIO 的 usage、流错误分类和请求日志继续观察上游原始错误码；只让下游 Codex 客户端看到改写后的码，不记录原始 SSE 帧或新增敏感正文。
- 不改变已有 pre-commit 流内部错误重试、provider failover、response fixer、插件处理或客户端断开 drain 语义。

## Acceptance Criteria

- [ ] AC-01：旧配置或新开关关闭时，所有 SSE 字节保持现有行为，不启用改写，也不因该功能移除响应头。
- [ ] AC-02：开关开启后，原生 Codex `/v1/responses`、`/responses`、`/v1/codex/responses` 的目标 `response.failed` 帧把两个目标码分别改为 `server_error`。
- [ ] AC-03：目标帧被任意 chunk 边界分割、使用 LF/CRLF 或与相邻帧共存时仍正确改写，且事件边界、非目标帧及其他 JSON 字段完整。
- [ ] AC-04：非目标 CLI/路径/bridge/事件/字段/错误码，以及无效 UTF-8、无效 JSON、超限或未闭合数据原样且无丢失地透传。
- [ ] AC-05：开启目标改写的响应不带上游 `Content-Length`；usage/attempt/request-log 中的 `stream_internal_error.error_code` 仍保留原始码。
- [ ] AC-06：CLI 管理的 Codex 设置中能读取、切换并持久化开关；保存失败时 UI 不伪装成功，重新读取后显示持久值。
- [ ] AC-07：设置字段完整跨越 Rust SSOT、Specta bindings、前端 adapter/fixtures、查询写入和 UI；普通设置 owner 的省略字段语义、并发保护与回滚合同不退化。
- [ ] AC-08：GitHub `ci-gate`、`pr-title` 与受影响的前端/Rust 测试通过；本地固定验证合同通过且 checkout 保持 zero-artifact。

## Scope and Decision Changes

| Date | Old / new decision | Affected acceptance criteria | Decision owner / resume condition |
|---|---|---|---|
| 2026-08-17 | 从“评估可行性”进入授权实施；采用精确 SSE 错误码改写 | AC-01..AC-08 | user |
| 2026-08-17 | 开关归 AIO `AppSettings`，UI 位于 Codex 设置；不写 Codex `[features]` | AC-06, AC-07 | main，依据现行所有权合同 |
| 2026-08-17 | 兼容默认值为关闭 | AC-01, AC-06 | main，保持升级行为 |

## PENDING Review

- `PENDING.md` 当前无未解决条目，无需并入本任务。

## Notes

- 官方 OpenAI 文档搜索未给出这一客户端内部重试分类的配置合同；本任务不把第三方中转站描述为 OpenAI 官方服务。
- 不把 `server_is_overloaded` / `slow_down` 加入 AIO 默认 pre-commit retry keyword；本任务的目标是让已提交给 Codex 客户端的目标事件触发客户端既有重试。
