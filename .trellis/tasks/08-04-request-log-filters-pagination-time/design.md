# 请求日志筛选、时间与稳定分页：技术设计

## 筛选契约

- 新增错误范围枚举：全部、所有报错、流内错误。它与现有 CLI、状态、错误码、方法和路径筛选按 AND 组合。
- “所有报错”先排除 `is_interrupted`，再匹配非 2xx 状态、非空顶层 `error_code` 或任一 attempt 的非空 `stream_internal_error`。
- “流内错误”只以 `attempts_json[*].stream_internal_error` 为事实源；SQLite 查询对空值、无效 JSON 和旧结构安全降级。
- 时间范围使用 `created_at_ms >= from`、`created_at_ms < to`；后端验证非负且 `from < to`。前端 `datetime-local` 按系统本地时区转换。

## 稳定任意页

- 保留旧 cursor 命令兼容现有调用，新增 snapshot page 命令和响应 DTO。
- 首次查询捕获当前 active trace 排除集合，并用完整后端筛选一次性物化按 `(created_at_ms DESC,id DESC)` 排序的日志 ID。
- Tauri managed state 以随机不透明 token 保存 ID 成员、筛选指纹、页大小、过期时间和最近访问时间；后续页只切片固定 ID 并按成员顺序回读，不重跑筛选。
- 同一 token 固定成员、顺序、总数和总页数；终态行的字段若被合法补全可更新。成员被 retention/清空删除时 token 明确失效，前端回到第一页创建新快照。
- 快照滑动 TTL 10 分钟，最多 32 个会话并限制总成员数；锁内只做内存操作，数据库 I/O 始终在锁外。
- 筛选、页大小、手动刷新或时间范围改变时，前端清空 token 并回到第一页。历史页暂停持久日志自动刷新，活动请求 feed 保持现状。

## 前端交互

- 错误范围使用紧凑分段控件，不要求手填状态表达式。
- 页码区显示 `当前 / 总页数`，数字输入支持任意有效页，上一页/下一页继续保留；提交时夹取并校验范围。
- 时间弹层提供开始/结束分钟输入和“最近 1 小时 / 今天 / 昨天 / 清除”快捷项；未应用前不改变查询。
- 快照过期自动清空 token、回到第一页并重新查询，界面不保留错误页码。

## 兼容与验证

- 生成绑定新增命令/DTO；旧 cursor DTO 和命令不删除。
- Rust 覆盖筛选 SQL、无效 JSON、同时间戳顺序、快照 TTL/失配/删除失效；前端覆盖快捷筛选、时间边界、任意页、总页数、筛选/刷新重置。
- 本地只运行 TypeScript、Vitest、ESLint 和 Vite build；Rust/生成绑定校验交给 GitHub Actions `dev-build`。
