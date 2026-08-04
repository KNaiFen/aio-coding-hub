# Token 速度根因调研

## 现有公式

- AIO：`output_tokens / ((duration_ms - ttfb_ms) / 1000)`：`src/utils/formatters.ts:96-121`。
- TUI 使用同一公式：`src-tauri/crates/aio-tui/src/format.rs:790-812`。
- 聚合性能趋势也使用 `SUM(output_tokens) / SUM(duration_ms - ttfb_ms)`：`.trellis/spec/aio-coding-hub/cross-layer/request-log-usage-ledger-pagination-contract.md:93-109`。

## 已证实的时间原点不一致

- `duration_ms` 从整条下游请求的 `ctx.started` 计算：`src-tauri/src/gateway/streams/request_end.rs:190`。
- 最终成功流的 `ttfb_ms` 从最终上游尝试的 `ctx.attempt_started` 计算：`src-tauri/src/gateway/streams/usage_tee.rs:379-380`。
- `StreamFinalizeCtx` 同时保存这两个不同起点：`src-tauri/src/gateway/streams/types.rs:87-88`。
- 因此当前分母实际为：

```text
整请求耗时 - 最终尝试内 TTFB
= 早期失败尝试/退避/切换耗时 + 最终尝试生成耗时
```

重试越多，显示速率越低，用户观察成立。

## 引入历史

提交 `bfa14913e915d658b3ca4fa944bf4e376e16f561`（2026-06-28，`fix: correct guard-aware TTFB tracking and display`）把流首字节计时从 `ctx.started.elapsed()` 改为 `ctx.attempt_started.elapsed()`，以得到最终供应商 TTFB；Token/s 公式没有同步改变时间原点。这是可复现的回归来源。

## 方案判断

不能把 `ttfb_ms` 改回端到端值，否则会污染供应商 TTFB；也不能用“下游实际收到首个增量”作为供应商生成起点，因为流内错误保护会先缓存有意义输出，导致缓冲期被错误排除、速度虚高。

最终采用独立生成阶段字段：

1. `ttfb_ms` 保持最终尝试相对值，`visible_ttfb_ms` 保持下游可见性职责。
2. 在原始上游协议上记录最终被采用流的首个与最后一个有效增量，持久化其间隔 `upstream_stream_duration_ms`。
3. 前缀缓冲中的有效增量计入；失败并重试的整次尝试不计入。
4. 计时放在协议桥、响应修复、插件和下游发送之前。独立有界中继负责持续拉取；队列满时无法再排除消费侧背压，因此版本降为 0。
5. 所有 Token/s 消费者只接受版本 1；旧日志、流内错误和不可可靠采集记录显示未知，不回退到混合时间原点的旧公式。
