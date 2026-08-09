# Provider 删除收敛设计

- 增加按 CLI 聚合 sort-mode query keys 的前缀 helper。
- mutation 开始时先 cancel，再对主列表、默认路由和所有已缓存 sort-mode rows 做同步过滤。
- mutation 完成后 invalidate 同一组 query families，确保服务端真值最终收敛。
- attempt 标签优先使用请求时快照，ID 永远来自记录本身；缺失名称时使用现有 fallback。
