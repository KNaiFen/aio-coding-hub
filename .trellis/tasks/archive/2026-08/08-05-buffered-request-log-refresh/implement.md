# 实施清单

- [x] 给 feed 增加只用于 live/foreground refresh 的 placeholder 保留模式。
- [x] 在 LogsPage 建立冻结展示状态、待展示计数和明确重置路径。
- [x] 给共享日志面板增加顶部状态、待更新提示、同步回顶和冻结时钟接口，保持默认兼容。
- [x] 覆盖顶部无闪烁、离顶冻结、计数、点击恢复、筛选/分页重置和前后台恢复测试。
- [ ] 集成三项修复后运行全量前端单测和 build。

## 本地验证

- `CI=true pnpm lint`
- `CI=true pnpm typecheck`
- `CI=true pnpm vitest run src/hooks/__tests__/useRequestLogsPageFeed.test.tsx src/pages/__tests__/LogsPage.test.tsx src/components/home/__tests__/HomeRequestLogsPanel.test.tsx`：3 个测试文件、50 个测试通过。
- 虚拟列表回顶覆盖使用 35 条请求日志，超过 30 条虚拟化阈值。
