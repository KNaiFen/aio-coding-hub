# 技术设计

## 边界

- `requestLogPresentation` 继续作为链路计数与摘要的单一来源。
- `RouteTooltipContent` 只负责结构化展示，不反推或修改链路数据。
- 共享 Tooltip 新增显式的面板 surface 与可悬停选项，默认值保留现有反色、不可悬停行为。
- `HomeRequestLogsPanel` 只为代理链详情选择面板 surface，普通提示不受影响。

## 摘要语义

从 `route` 中对每一跳做有界、非负整数归一化：

- `providerCount = route.length`
- `transitionCount = max(providerCount - 1, 0)`
- `skippedCount = skipped hop 数量`
- `requestCount = 非 skipped hop 的 attempts 总和`
- `retryCount = 非 skipped hop 的 max(attempts - 1, 0) 总和`

短摘要最多优先显示三段，避免固定窄栏溢出：

- 无切换、无重试：`直连`
- 无切换、有重试：`重N·请N`
- 有切换：始终显示 `切N` 与 `请N`；中间优先显示 `跳N`，没有跳过时显示 `重N`
- 只有跳过且未发出请求时显示 `跳N`

完整自然语言摘要保留全部计数，不受短摘要省略策略影响。

## 详情面板

- 使用 `bg-popover text-popover-foreground border-border shadow-panel`。
- 宽度约束为 `min(35rem, calc(100vw - 1.5rem))`，高度不超过视口可用空间，纵向滚动。
- 开启 hoverable content，允许鼠标从触发器移动到面板内滚动；碰撞边距为 12px。
- 每一跳第一行显示供应商和状态，第二层显示状态码、错误分类、是否实际发出请求；未知原始字段单独换行。
- 仅抑制明确等价的已知内部字段，未知值原样保留但不执行、不插入 HTML。
- 颜色使用语义色及 `dark:` 配对，不使用 `text-white` 等单主题硬编码。

## 容错与兼容

- 所有数值归一化都拒绝 NaN、Infinity、负数和异常大值造成的计算异常。
- 缺失、畸形链路继续使用现有 fallback；详情中的未知值通过普通 React 文本渲染。
- 不改变 IPC DTO、后端 attempt 语义、缓存数据结构或日志持久化。

## 验证

- 单元测试覆盖直连、重试、切换、跳过、三位数字和畸形数据。
- 组件测试覆盖主题 surface、可悬停配置、单行短摘要、完整辅助标签、已知原因去重与未知原因保留。
- Playwright 在 1024px 和 1440px、明暗主题下检查边界、长内容与默认 Tooltip 回归。
