# AUD-055 实施清单

- [ ] 增加 archived session 不扫描/不改写的 failure-first 回归。
- [ ] 定义 v2 session-only manifest 与 managed 目录分类。
- [ ] 将变更集、snapshot 和 backup 收窄到配置与活动 sessions。
- [ ] 实现 v2 单代保留、v1 managed 迁移删除和 symlink/unmanaged 保护。
- [ ] 覆盖正常写入、写失败回滚、旧备份迁移、替换和只留一代。
- [ ] 本地只运行源码合同、解析和差异检查。
- [ ] 由全量 Actions 验证 rustfmt、bindings、Clippy、Rust tests 与 audit。
- [ ] 合并后在下一候选分支记录 PR/CI/提交证据。
