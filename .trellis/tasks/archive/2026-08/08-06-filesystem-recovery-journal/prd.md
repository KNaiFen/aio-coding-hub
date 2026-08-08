# AUD-002 SQLite 文件系统双写恢复

## Goal

以已提交的 SQLite 状态为权威，为 Prompt、MCP、Skills 及工作区切换的文件系统副作用建立可重启恢复协议，消除补偿失败被吞掉后留下的永久双写漂移。

## Requirements

- 在任何外部文件系统副作用前，使用独立事务持久提交 recovery journal；journal 不能与可能回滚的业务事务共命运。
- 业务 SQLite 提交结果决定最终状态：重启对账必须把外部投影收敛到当前已提交 DB，而不是猜测崩溃发生在哪一步。
- Prompt、MCP、Skills 以及跨三类投影的 workspace switch 全部纳入；现有 best-effort 补偿不得继续吞错。
- Skills 的 SSOT 内容不能仅靠 metadata 重建。install/update/uninstall/return-to-local 必须保留带 hash 的受管 staging/backup artifact，直到 journal resolved；不得把任意外部路径或非受管目录当作恢复材料。
- 启动对账在 DB 可用后、request-log 恢复、observer、gateway 和后台任务之前阻断执行，并复用 AUD-008 的 maintenance coordinator。
- 对账失败保持 journal 与维护态，可重试或退出；不得带着部分投影继续普通启动或接受写 IPC。
- journal 与用户错误只保存定长脱敏摘要，不保存 Prompt 正文、MCP env/header、Bearer/token/secret、原始文件字节或敏感绝对路径。
- 失败返回必须同时保留 primary error 与 compensation/replay failure 的脱敏分类，不能用后者覆盖前者。

## Acceptance Criteria

- [ ] 每个受管外部写入在副作用发生前已有 durable `prepared` journal，完成收敛后才标记 resolved 或删除。
- [ ] 外部写失败、业务 commit 失败、补偿失败、commit 后崩溃和 journal 清理前崩溃均能在重启后收敛到已提交 SQLite 状态。
- [ ] Skills 恢复只使用 journal 拥有且 hash 匹配的 staging/backup artifact；缺失或不匹配时保持 maintenance 阻断，不损坏非受管文件。
- [ ] workspace switch 在 Prompt、MCP、Skills 或 active-workspace 任一阶段失败后可重启恢复为单一一致工作区投影。
- [ ] replay 成功前 observer、gateway、retention、usage backfill 和前端后台任务均未启动，写命令不可与 replay 交错。
- [ ] 错误与 journal 内容不泄露 env/header、正文、token、secret、password 或原始路径；补偿失败对用户与诊断可见。
- [ ] 云端 Rust 覆盖 schema 幂等、故障注入、跨实例 replay、并发 gate、artifact ownership/hash 和脱敏边界。

## Notes

- AUD-008 必须先提供共享 maintenance coordinator；关联 `AIO-PENDING-021`。
- “SQLite 权威”描述业务意图；Skills 的受管内容 artifact 是完成该意图所必需的临时恢复材料，不是第二个长期权威源。
