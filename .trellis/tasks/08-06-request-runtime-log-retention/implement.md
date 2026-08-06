# AUD-056 实施清单

- [ ] 增加 schema 迁移、默认值和 0 拒绝的 failure-first 回归。
- [ ] 修改请求日志 retention，保留 ledger backfill/coverage 门。
- [ ] 实现运行日志精确文件识别、7 天与 256 MiB 双边界。
- [ ] 移除清空路径自动 VACUUM，增加 SQLite reclaimable bytes。
- [ ] 更新设置 UI、validation、service/query、fixtures 与文案。
- [ ] 接收云端 bindings 漂移时只应用精确生成补丁。
- [ ] 本地运行源码合同、解析和 diff；完整测试交 Actions。
- [ ] 合并后在 AUD-016 候选中记录证据。
