# 实施计划

1. 在余额内联组件中分离首次加载、后台心跳和手动刷新状态。
2. 用同步 ref 防止手动刷新重入，并保持现有查询 helper 为唯一 force 入口。
3. 更新卡片集成测试，新增多卡、三次心跳、后台成功/失败和首次加载回归。
4. 更新账户余额查询合同，运行定向测试、typecheck 与 lint。

## 实施记录（2026-08-03）

- `ProviderAccountUsageInline` 仅使用 `isLoading` 处理首次无结果的加载文案；五秒心跳不再影响可见摘要、指标、图标或按钮状态。
- 手动刷新使用本地 `manualRefreshing` 与同步 ref 锁，复用既有 `refreshProviderAccountUsage` force 查询，并在 `finally` 清理状态。
- 新增 fake-timer 组件回归测试，覆盖双卡三个心跳周期、后台成功/失败、初始加载、手动成功/失败及快速重复点击；未修改 `src/query/providers.ts`。
