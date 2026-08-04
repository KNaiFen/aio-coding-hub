# 请求日志筛选与分页调研

## 数据链路

`LogsPage` -> `useRequestLogsPageFeed` -> React Query -> gateway service -> 生成绑定 -> Tauri command -> SQLite keyset 查询。

关键入口：

- `src/pages/LogsPage.tsx:226`
- `src/hooks/useRequestLogsPageFeed.ts:45`
- `src/query/requestLogs.ts:93`
- `src/services/gateway/requestLogs.ts:197`
- `src-tauri/src/commands/request_logs.rs:72`
- `src-tauri/src/infra/request_logs/queries.rs:673`

## 筛选事实

- 现有状态筛选只有 `eq/neq/gte/lte`；`!200` 会匹配 201/204、中断等，不能表示所有失败：`src/pages/LogsPage.tsx:156`、`src-tauri/src/infra/request_logs/queries.rs:695`。
- 流内错误可靠保存在 `attempts_json[*].stream_internal_error`，页筛选 DTO 尚无对应字段：`src-tauri/src/infra/request_logs/types.rs:114`、`src-tauri/src/gateway/streams/request_end.rs:200`。
- 可先用 SQLite JSON 查询实现，无需迁移；若数据规模证明查询成本不可接受，再授权增加冗余布尔列/索引。

## 分页与时间

- 当前游标为 Base64URL JSON `{v, createdAtMs, id}`，按 `(created_at_ms,id)` 倒序 keyset：`src-tauri/src/infra/request_logs/queries.rs:94-187,721`。
- 响应只有 `items,nextCursor`，没有总数；UI 仅保存走过的 `cursorStack`：`src/pages/LogsPage.tsx:45`。
- 任意页跳转需要新增 count/分页契约，并处理实时插入导致的页漂移；只跳已访问页无需改变协议。
- 现有 `(created_at_ms DESC,id DESC)` partial index可支持 `createdAtMsFrom`/`createdAtMsTo` 范围：`src-tauri/src/infra/db/migrations/ensure.rs:824`。
