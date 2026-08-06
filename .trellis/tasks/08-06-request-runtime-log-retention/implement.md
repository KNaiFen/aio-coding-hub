# AUD-056 实施清单

- [x] 增加 schema 迁移、默认值和 0 拒绝的 failure-first 回归。
- [x] 修改请求日志 retention，保留 ledger backfill/coverage 门。
- [x] 实现运行日志精确文件识别、7 天与 256 MiB 双边界及惰性跨日活动文件保护。
- [x] 移除清空路径自动 VACUUM，增加 SQLite reclaimable bytes。
- [x] 更新设置 UI、validation、service/query、fixtures 与文案。
- [x] 源码侧同步 `DbDiskUsage.reclaimable_bytes` 精确 binding 字段，未运行本地生成器；待 Actions 核验无额外漂移。
- [x] 本地运行 cloud-only checker/self-test 与 diff；完整测试交 Actions。
- [x] 代码提交 `1a733301e33d2d9f605406339c197e050104d2c5`。
- [ ] 平台可用后建立/重建 PR，对精确 head 运行 frontend、Rust format/bindings、Clippy、Rust tests 与 audit。
- [ ] 执行最终主线门并合并；在此之前保持 `in_progress/planned`。
- [ ] 合并后在 AUD-016 候选中记录证据。
