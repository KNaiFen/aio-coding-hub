# AUD-055 实施清单

- [x] 增加 archived session 不扫描/不改写的 failure-first 回归。
- [x] 定义 v2 session-only manifest 与 managed 目录分类。
- [x] 将变更集、snapshot 和 backup 收窄到配置与活动 sessions。
- [x] 实现 v2 单代保留、v1 managed 迁移删除和 symlink/unmanaged 保护。
- [x] 覆盖正常写入、写失败回滚、旧备份迁移、替换和只留一代。
- [x] 本地只运行源码合同、解析和差异检查。
- [x] 提交并推送代码，建立草稿 PR #87（head `59ea8235`）。
- [ ] GitHub Actions 平台恢复后，对精确 head 重跑 rustfmt、bindings、Clippy、Rust tests 与 audit；现有三次运行均为平台故障，未形成产品失败结论。
- [ ] 将 PR 转 Ready，执行最终主线门并合并。
- [ ] 合并后在下一候选分支记录 PR/CI/提交证据。
