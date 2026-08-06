# AUD-002 实施清单

- [ ] 先补 failure-first 测试，覆盖 Prompt、MCP、Skills 与 workspace switch 的副作用/commit/补偿失败。
- [ ] 增加 journal schema、索引、幂等迁移、状态机和脱敏摘要。
- [ ] 增加 journal 专属 Skills staging/backup ownership、hash 校验和安全回收。
- [ ] 将 Prompt/MCP/Skills 的每个外部写入口改为 prepare-first 协议，并消除吞错补偿。
- [ ] 将 workspace switch 纳入父 operation 和可重放阶段。
- [ ] 在 DB 初始化后接入阻断 replay，复用 AUD-008 maintenance retry/exit gate。
- [ ] gate 普通写 IPC 与前端后台启动，防止 replay 期间交错修改。
- [ ] 云端覆盖跨实例 replay、并发、artifact 越界/symlink/hash、错误脱敏和 schema。
- [ ] 本地只运行零依赖源码合同、解析与 `git diff --check`。
- [ ] 合并后在 AUD-035 候选记录 PR、提交和 CI 证据。
