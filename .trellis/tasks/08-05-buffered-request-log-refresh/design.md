# 技术设计

底层 feed 继续响应 complete 和前台恢复事件，展示层与数据源分离。顶部刷新时利用 React Query placeholder 的旧页数据，避免中间空数组。首次离顶时，LogsPage 捕获持久日志、active requests、traces、可用性、总数和冻结时间；面板只消费冻结投影，源查询继续更新。

滚动容器以 4px 为顶部阈值并只在布尔值变化时上报。待展示数以筛选一致的最新 `totalCount - frozen totalCount` 计算。提示操作先同步滚到顶部再解除冻结和刷新，避免虚拟列表 prepend 锚点复杂度。

筛选、时间范围、CLI、错误范围、页大小、翻页和手动刷新是显式用户意图，统一清除冻结并通过 reset key 使面板回顶。首页调用不传新 props，行为保持不变。
