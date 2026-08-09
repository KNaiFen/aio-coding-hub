# Provider 性能趋势执行计划

- [x] 提取共同 bucket/Top Provider/limit 规划，修复缓存率趋势无限 limit。
- [x] 新增 ledger 性能聚合、DTO、Tauri command 和 native 测试。
- [x] 更新 service/query 层与生成绑定合同。
- [x] 新增性能趋势 UI、指标切换、图例显隐、tooltip、空态和错误态。
- [x] 覆盖加权公式、过滤、时区、Provider 删除快照、日志清理前后一致性和预算。
- [x] 运行前端完整测试、类型检查、Lint、Prettier 和 Vite build。
- [x] 云端验证原始 ledger 实现：格式、Clippy、2588 个 Rust 测试和前端门禁通过；百万行查询 `5.057927737s` 未达 `1s`。
- [x] 新增版本化日汇总 schema、脏日触发器、可恢复逐日回填和迁移测试。
- [x] 重构性能/缓存趋势为“可信日汇总 + 未覆盖 raw ledger”的单次物化源，保留纯 ledger 兼容路径。
- [x] 覆盖混合源等价性、部分日、当前日、脏日、重试、时区和 Provider 清空语义。
- [ ] 更新百万行 release 基准，证明两项趋势查询均低于 `1s`。
- [ ] Rust、Specta、SQL 百万行基准和桌面 build 交由 GitHub Actions。
- [ ] 合并前重新同步最新 `origin/main`、解决其他 Session 造成的冲突并在最终基线上重跑 CI。
- [ ] CI 通过后完成 PR 合并；运行期间约每五分钟检查一次状态。
