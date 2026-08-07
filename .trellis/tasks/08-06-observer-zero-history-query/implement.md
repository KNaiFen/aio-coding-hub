# AUD-035 实施清单

- [x] 增加 zero-history failure-first 测试并记录当前 500-row 查询与 folder 扫描调用。
- [x] 为 last、dominant、recent 增加受限且复用既有可见性谓词的查询。
- [x] 调整 snapshot 投影，使 `history_limit=0` 跳过 recent 查询但返回 ready empty。
- [x] 只从实际渲染的 active/last/recent 收集 folder lookup keys。
- [x] 在 Observer state 增加 source-aware、有容量与正/负 TTL 的 folder cache。
- [x] 覆盖 All/Claude scope、摘要平手、active count、cache key、淘汰和 miss 后出现。
- [x] 本地只运行零依赖源码合同、解析与 `git diff --check`。
- [ ] 由全量 Actions 验证 rustfmt、Clippy、Rust tests 与 audit。
- [ ] 合并后在 AUD-033 候选记录 PR、提交和 CI 证据。
