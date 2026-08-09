# 首页统计与供应商健康统一改造

## Goal

在一个集成分支中完成 AIO-PENDING-024 至 028，修复首页图表与供应商布局，统一供应商健康观测，并将单请求和全局 TPS 对齐 sub2api。

## Requirements

- 五个子任务的生产代码、迁移、测试代码、活跃规范和任务文档全部完成后，才能进入统一验证。
- 使用 React/TypeScript/Recharts/React Query 和现有 Tauri IPC；后端使用 Rust/Tokio/SQLite/Specta；不新增依赖。
- 实施过程不纳入当前工作树中的 230 个既有未跟踪文件。
- 本地只运行 cloud-only 零依赖合同检查、必要的 `node --check` 和 `git diff --check`。
- 五项只使用一个功能提交、一次精确 SHA 的 Actions 验证流程和一个 PR。

## Acceptance Criteria

- [ ] 超过 1000M 的 Y 轴标签使用 B 且不裁切，余额文字与卡片上方内容左对齐。
- [ ] 状态条完整实现红/黄/绿/no-data 和当前格最后观测规则。
- [ ] 新的可信流式完成请求能显示 TPS，所有统计表面使用逐请求 TPS 算术平均。
- [ ] 每供应商可选配置定时 probe，页面关闭后运行，不补跑且有界并发，同义手动入口同样记录观测。
- [ ] schema 52 能从 51 和更旧数据库安全升级，日 rollup 不会把旧加权速率冒充为新口径。
- [ ] 本地零产物检查通过，GitHub Actions `ci-gate` 对精确提交绿色后创建单一 PR。

## Child Mapping

- AIO-PENDING-024 → `08-09-home-usage-axis-format`
- AIO-PENDING-025 → `08-09-provider-balance-left-align`
- AIO-PENDING-026 → `08-09-provider-availability-thresholds`
- AIO-PENDING-027 → `08-09-request-log-sub2api-tps`
- AIO-PENDING-028 → `08-09-provider-scheduled-availability-probe`
