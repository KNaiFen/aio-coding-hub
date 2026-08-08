# AUD-035 history_limit=0 Observer I/O

## Goal

让 Observer 在 `history_limit=0` 时跳过无用的 500 行日志投影和完整会话树扫描，同时保持现有 last、dominant、active 与 all-scope 摘要语义。

## Requirements

- `recent_requests` 在 `history_limit=0` 时仍为 available 的空数组，不得变为 unavailable。
- `last_request` 仍是当前 scope 最新的 terminal model-inference；`dominant_provider` 仍按最新最多 10 条 terminal inference 计数且平手优先较新记录。
- active request 与 all-scope 可见性、Claude messages path 过滤及现有排序语义保持不变。
- 为 last/dominant/recent 使用与投影需求相匹配的受限 SQL 查询；零历史路径不得读取或构造 500 条 `RequestLogSummary`。
- session-folder lookup 只服务实际渲染的 active/last/recent 投影，不得从未渲染的 500 行推导键集合。
- folder lookup 使用 Observer 生命周期内的有界内存缓存，键包含 CLI source 与 session id，并为命中和未命中设置有限 TTL，避免永久缓存后来出现的 session。
- 缓存容量、单次 lookup 数量、session id 长度与允许 source 都必须有显式上限。

## Acceptance Criteria

- [ ] `history_limit=0` 不调用 500-row recent 查询、不构造 recent terminal 投影，并返回 `recent_requests=[]`。
- [ ] zero-history 与既有路径的 last、dominant、active count、all-scope 排序和可见性结果一致。
- [ ] folder lookup 输入只包含会实际出现在响应中的 active/last/recent session，零历史不引入隐藏历史行。
- [ ] Codex/Claude folder 扫描结果按 `(source, session_id)` 隔离，缓存有容量与 TTL 上限，未命中后文件出现可在过期后被发现。
- [ ] 其它 history limit 仍返回正确数量，快照 cache key 与 folder cache 不互相污染。
- [ ] 云端 Rust 覆盖查询调用边界、投影一致性、folder lookup 范围及缓存淘汰/过期。

## Notes

- 不改变 Observer 协议字段或摘要定义；关联 `AIO-PENDING-022`。
