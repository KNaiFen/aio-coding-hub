# AUD-035 技术设计

## 受限数据库投影

把数据库读取拆成明确的 last、dominant 与 recent 需求。last 只取当前 scope 最新 terminal inference；dominant 只取按现有排序的最多 10 条 terminal inference；recent 只取请求的有界数量。`history_limit=0` 完全省略 recent 查询，但仍组装 available 的空 section。All 与 scope-specific 查询复用现有可见性谓词，不能退化为先读 500 行再在内存过滤。

## Folder 投影

先确定会渲染的 active、last 和 recent terminal，再为这些行收集合法 `(source, session_id)`；dominant 计算本身不触发 folder lookup。lookup 继续复用 Claude/Codex 现有解析语义，但由 Observer state 持有小型 TTL/LRU 缓存，限制容量与每次 miss 批量大小。

## 缓存一致性

缓存键必须包含 source，正命中和负命中分别使用有限 TTL。容量淘汰不得改变响应正确性，只影响后续扫描成本。现有 snapshot cache 仍按 scope/history/include-providers 隔离；folder cache 是更低层、跨快照复用的纯投影缓存。

## 兼容边界

保留 recent available/empty、last terminal inference、dominant 最近十条与平手优先较新、active count 和 Claude message visibility。优化通过查询计数/spy 与投影结果双重测试证明，而不靠时延阈值。
