# AUD-055 实施清单

- [x] 增加 archived session 不扫描/不改写的 failure-first 回归。
- [x] 定义 v2 session-only manifest 与 managed 目录分类。
- [x] 将变更集、snapshot 和 backup 收窄到配置与活动 sessions。
- [x] 实现 v2 单代保留、v1 managed 迁移删除和 symlink/unmanaged 保护。
- [x] 覆盖正常写入、写失败回滚、旧备份迁移、替换和只留一代。
- [x] 将根/子项操作改为句柄相对、no-follow 的分类/隔离/复验/删除路径，并为普通文件增加有界流式 SHA-256、Windows ChangeTime 与末次删除复验。
- [x] 共享单次 prune 的深度、条目和哈希预算，限制详细 warning，并覆盖根替换、子项替换、等长改写、预算耗尽和平台删除边界。
- [x] 本地只运行源码合同、解析和差异检查。
- [ ] 由统一 PR 精确 head 的全量 Actions 验证 rustfmt、bindings、Clippy、跨平台 Rust tests 与 audit。
- [ ] 在统一 PR 内记录精确 head、CI 与替代旧 PR #87 的证据，并与 AUD-002、AUD-035、AUD-033 一起合并。
