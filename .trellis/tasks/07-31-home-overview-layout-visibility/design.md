# 首页布局与可见项优化设计

## 布局

- `HomeOverviewPanel` 统一使用单一网格：`xl:grid-cols-12`，左列 `xl:col-span-5`，右列 `xl:col-span-7`。
- 左列使用纵向 flex；用量卡固定约 192px，信息面板 `flex-1 min-h-0`。右列的日志面板直接占据整个网格高度。
- 窄窗口保持左栏优先的 DOM 顺序，主内容区纵向滚动；用量、信息、日志分别具备稳定的最小高度。

## 本机偏好

使用 `aio-home-overview-visibility` 保存版本化对象：

```ts
type HomeOverviewVisibilityV1 = {
  version: 1;
  hiddenTabs: HomeOverviewTabKey[];
  hiddenCliKeys: CliKey[];
};
```

- 存隐藏集合保证未来新增项目默认可见。
- 解析未知版本、错误 JSON、错误类型、存储读写错误时，使用全可见默认值。
- 未知和重复键被过滤。某一类若全部隐藏，只重置该类隐藏集合，另一类偏好保持不变。
- 服务提供稳定快照、订阅和写入 API；`HomePage` 用 `useSyncExternalStore` 消费。

## 状态与兼容

- 面板显示顺序为现有排序与可见集合的交集；当前 tab 失效时通过 effect 回退到第一个可见 tab。
- 仅在熔断面板可见时保留现有的熔断自动切换行为。
- CLI 显示顺序继续采用全局 CLI 优先顺序，仅在首页配置信息渲染前过滤。
- 删除旧个性化布局分支、布局 localStorage 读取、页头曲线切换和首页热力图分支。
- `show_home_usage` 继续控制首页曲线；`show_home_heatmap` 仅保持持久化兼容，不再被首页使用。
