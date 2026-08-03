# 技术设计

## 数据流与所有权

```text
Codex SSE bytes
  -> 单一 SSE 帧解析/错误证据提取器（Rust）
  -> 保护窗分类状态机（Rust failover loop）
  -> RetryPolicyMatch::StreamInternalError（既有预算/退避/熔断）
  -> FailoverAttempt.stream_internal_error（持久化审计）
  -> attempts_json / error_details_json
  -> 前端共享解析器
  -> 供应商链路与最终错误详情
```

Rust 是协议、分类、脱敏和持久化格式的唯一所有者。React 只消费结构化字段，不重新解析 SSE、`reason` 字符串或原始 JSON。

## 设置合同

为 `UpstreamRetryPolicy` 增加：

```rust
pub struct UpstreamStreamInternalErrorPolicy {
    pub enabled: bool,
    pub retry_keywords: Vec<String>,
    pub non_retry_keywords: Vec<String>,
}
```

缺失字段按启用的默认策略反序列化，因此旧全局设置和供应商覆盖立即获得默认能力。列表各限制 16 项、每项 512 字符；保存时 trim、拒绝空值/控制字符，并按大小写不敏感值去重。供应商覆盖继续替换整套 `UpstreamRetryPolicy`，不与全局列表合并。

`AppSettings` 增加全局 `stream_internal_error_guard_ms: u32`，默认 500、范围 0..=5000；普通设置写入路径拥有该字段，与 `upstream_retry_policy` 在同一锁内字段更新。供应商编辑器只展示策略字段，不展示全局保护窗。

默认 HTTP 规则为启用的状态 400 规则，`body_contains` 仅含容量短语。兼容迁移扫描全局和非空供应商覆盖：任意 400 status-only 规则或包含容量短语的 400 规则（含显式禁用）均视为已表达用户意图；否则追加默认规则。迁移幂等、不删规则；旧策略已满 16 条时允许迁移兼容槽位，并把规则上限同步为 17。

## SSE 分类与保护窗

扩展现有 Codex SSE 帧辅助函数，使前缀检查和流结束跟踪器共用：

- 终止事件集合：`error`、`response.error`、`response.failed`、`response.incomplete`，同时接受事件名和 `data.type`。
- 错误字段顺序：`error.message/type/code`、`response.error.message/type/code`、顶层 `message/code`；只读取这些已知位置。
- 匹配文本由事件名、data type 和提取出的 error type/code/message 组成，不扫描任意响应字段或正常输出。
- 重试词先匹配；未命中再匹配禁止词；两者都未命中为 unknown。

Codex 原生 Responses 路径使用提交前状态机：

1. 原生 Codex Responses 成功 SSE 先构造成单一“已解码字节流”；首字节探测、保护窗和后续 relay 顺序消费它。上游若返回 gzip，只在这里解压一次，分类器与客户端看到相同字节；其他 SSE 仍保持原有的下游解压顺序。
2. 继续缓冲 `response.created`、`response.in_progress` 等元数据。
3. 首次非空文本、拒绝、推理摘要、函数参数或具体 output/tool item 启动保护窗。
4. 在成功 completion、EOF、计时到期或 1 MiB 上限时提交已缓冲 bytes，并把剩余 upstream stream 接到现有 relay。
5. 在提交前遇到正向匹配错误，丢弃 bytes 并返回 failover loop；遇到禁止/unknown/disabled 则提交原始帧并按现有流结束逻辑记错。
6. 1 MiB 上限只停止保护并放行，同时追加 `stream_internal_error_guard` special setting；它不是供应商失败。

保护窗只在 response handler 返回下游 `Response` 前运行，因此能够复用 failover loop；一旦返回 response，后续 tracker 仅观察、记错并按既定决定透传。

## 重试与熔断

增加 `RetryPolicyMatch::StreamInternalError`，与 HTTP/Transport 共用 `configured_transient_retries_used` 和已有 attempt reservation。正向匹配调用 `transient_failure_decision`：

- `RetrySameProvider` 才消费一次配置重试、等待 `backoff_ms`，并按 `counts_toward_circuit_breaker` 决定是否计入熔断。
- 配置额度耗尽后沿用 `SwitchProvider`；跨供应商不增加隐式退避。
- 所有供应商耗尽时返回标准 `502 / GW_FAKE_200`，原始容量帧不下传。
- count-tokens、严格模型发现和其他内部修复继续遵守现有预算合同，流内规则不得额外扩容。

## 日志与安全

`FailoverAttempt` 增加可选 `StreamInternalErrorEvidence`：

```text
event_type, error_type, error_code, message,
classification, matched_keyword, disposition, truncated
```

所有字段进入持久化前统一规范化。message 合并换行、最多 2048 字符；其他文本使用现有 512 字符短字段上限。脱敏器覆盖 `Bearer <value>`、常见 `sk-`/API key 形态、JSON/query 风格的 `access_token` / `api_key` / `token`，替换值而保留错误语义。测试只使用 `SYNTHETIC_SECRET`。

提交前失败直接把证据写入新 attempt。提交后错误由 tracker 保存证据，通过 `StreamRequestCompletion` 更新最后一个 optimistic success attempt。最终成功仍通过 `attempts_json` 保留早期失败证据；最终失败的 `error_details_json` 投影最后一个错误 attempt 的同一结构。原始 SSE 和正常内容从不持久化。

前端 `attemptsJson` 是结构化解析入口；供应商链路始终可展示证据。最终错误卡只在请求最终失败时展示同一证据。复制按钮使用 Lucide `Copy`、tooltip 和已有 toast，只复制已脱敏持久化 message。

## 兼容与回滚

- 缺失新字段有确定默认值，旧日志缺少 evidence 时按现有 UI 降级。
- 非 Codex Responses、非 event-stream、非 2xx 及现有非流式 fake 200 路径不变。
- “上游响应错误规则”继续只在最终 HTTP 错误完成 retry/failover 后执行。
- 功能可通过关闭 `stream_internal_errors.enabled` 和把保护窗设为 0 降级；代码回滚时未知设置字段由旧版本既有容错处理。
- 保护窗默认增加约 500ms 首次真实输出延迟；1 MiB 硬上限限制并发内存放大。
