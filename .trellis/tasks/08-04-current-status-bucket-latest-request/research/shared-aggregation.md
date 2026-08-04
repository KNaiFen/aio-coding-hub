# 当前状态格共享聚合调研

## 数据流

`request_logs` 终态写入 -> `provider_availability_observations` -> `provider_availability::timelines()` -> AIO/TUI/托盘消费者。

- 请求日志投影观测：`src-tauri/src/infra/request_logs.rs:992`。
- 成功/失败事实分类：`src-tauri/src/domain/provider_availability.rs:741-843`。
- 共享聚合及时间升序格数组：`src-tauri/src/domain/provider_availability.rs:881-1017`。
- 当前颜色阈值：无数据灰；成功率 `>= 75%` 绿，否则红：`src-tauri/src/domain/provider_availability.rs:870`。
- AIO 映射：`src/components/providers/ProviderAvailabilityStrip.tsx:73-124`。
- TUI 映射：`src-tauri/crates/aio-tui/src/ui.rs:1470`。
- TUI 请求 12 格：`src-tauri/src/app/observer/snapshot.rs:363`；AIO 请求 36 格：`src/query/providers.ts:638`；托盘请求 18 格：`src-tauri/src/app/tray_provider_mini.rs:225`。

## 结论

最小正确所有者是后端 `timelines()`：历史格保留比例，最后格由该格最新观测覆盖状态。UI 无需增加协议字段。当前 SQL 仅按 `observed_at_ms ASC`，同毫秒顺序尚不确定：`src-tauri/src/domain/provider_availability.rs:965`。

观测表没有独立自增 ID，但属于普通 rowid 表，且 retention 已依赖 `rowid`。本任务将查询顺序锁定为 `observed_at_ms ASC, rowid ASC`；同毫秒时以首次插入较晚的观测为最新。该规则稳定、无需迁移，并保持同一 trace/provider UPSERT 的既有最终成功优先语义。
