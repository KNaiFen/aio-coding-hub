# 当前待处理列表

本文件只保存用户明确要求暂缓、以后集中处理的未完成事项。已完成或明确放弃的历史保存在 [`PENDING_COMPLETED.md`](./PENDING_COMPLETED.md)，实施前无需加载归档文件。

## 工作规则

- 活跃状态只使用 `pending` 或 `planned`；完成后使用 `done`，明确放弃时使用 `dropped`，并迁入完成归档。
- 进入正式计划模式，或用户明确要求开始修改时，必须先读取本文件，并把所有未解决条目加入当次候选处理清单。
- 若条目之间存在冲突、依赖或明显不适合在同一批交付，必须明确说明并请用户决定，不能静默遗漏。
- 记录条目不代表授权实现；进入实施后必须关联 Trellis 任务。
- 只有完成合并和验证并记录 PR、提交或版本证据后，才可标记为 `done` 并迁入 `PENDING_COMPLETED.md`。
- `dropped` 必须记录用户的明确决定和原因后再迁入归档。
- 新条目继续使用稳定递增 ID；下一个 ID 为 `AIO-PENDING-024`。

## 未解决条目

### AIO-PENDING-017 - Provider Sync session-only 快照

- **状态**：`planned`
- **日期**：2026-08-06
- **待执行交付**：session-only v2、v1 managed 迁移、单代保留、回滚及句柄相对清理加固已汇入 `codex/final-hardening-unified`。本项不再单独更新或合并 PR #87；由统一 PR 的精确远端 head 执行全量 Actions、跨平台竞态/预算回归与主线门后，与 AUD-002、AUD-035、AUD-033 一起合并。
- **观察问题**：Provider Sync 当前扫描并备份 `archived_sessions`、SQLite 与全局状态，且旧格式 managed backup 最多保留五代，造成与恢复目标无关的空间增长。
- **锁定决策**：新格式只处理活动 `sessions`，不再扫描、改写或备份 `archived_sessions`；只保留最新一代新格式 managed backup；只删除 manifest 精确证明所有权的旧格式 managed backup。
- **拟议方向**：引入 v2 session-only manifest 和严格 managed/unmanaged 分类，在首次成功创建 v2 后迁移清理 v1，并保留同步失败的完整回滚。
- **验收标准**：归档会话字节不变；成功后最多一代 v2；v1 managed 可迁移、非受管/损坏/symlink 保留；云端 Rust 覆盖迁移、回滚和所有权边界。
- **Trellis**：[`08-06-provider-sync-session-snapshot`](./.trellis/tasks/08-06-provider-sync-session-snapshot/)

### AIO-PENDING-021 - SQLite/文件系统双写恢复

- **状态**：`planned`
- **日期**：2026-08-06
- **待执行交付**：prepare-first journal、SQLite 权威 replay、Skills 受管 artifact 和补偿错误聚合已汇入 `codex/final-hardening-unified`，直接基于 `origin/main@99de56bb`。本项不再更新或单独合并 PR #92；由统一 PR 的精确远端 head 运行 PR CI、workflow_dispatch 全量 CI、故障注入、bindings、并发与主线门后，与 AUD-055、AUD-035、AUD-033 一起合并；合并前保持 `planned`。
- **观察问题**：Prompt、MCP、Skills 和 workspace switch 在 SQLite 与外部文件之间双写，多处补偿吞错；进程中断或 commit/恢复失败可留下跨重启漂移。
- **锁定决策**：已提交 SQLite 状态为权威；任何外部副作用前 durable journal；启动前阻断自动对账并复用 AUD-008 维护态；补偿失败必须可见且错误摘要脱敏。
- **拟议方向**：统一 prepare-first 操作协议。Skills 的不可由 metadata 重建内容使用 journal 专属、带 ownership/hash 的临时 staging/backup，resolved 后回收。
- **验收标准**：外部写、commit、补偿、重启各故障点最终收敛到 DB；artifact 缺失/越界/hash 不符保持维护态；普通写 IPC 不与 replay 交错；journal/错误不泄露正文、env/header 或密钥。
- **Trellis**：[`08-06-filesystem-recovery-journal`](./.trellis/tasks/08-06-filesystem-recovery-journal/)

### AIO-PENDING-022 - history_limit=0 Observer I/O

- **状态**：`planned`
- **日期**：2026-08-06
- **待执行交付**：受限 last/dominant/recent 查询、zero-history ready-empty 和 source-aware 有界 folder cache 已汇入 `codex/final-hardening-unified`。旧独立候选不再作为验证或合并表面；由统一 PR 的精确远端 head 运行查询/缓存回归、bindings、全量 CI 与主线门后，与 AUD-055、AUD-002、AUD-033 一起合并；合并前保持 `planned`。
- **观察问题**：`history_limit=0` 只在投影末端生效，之前仍读取并构造 500 条日志，并基于隐藏历史触发完整 Claude/Codex session-folder 扫描。
- **锁定决策**：保持 last/dominant/active/all-scope 与 recent ready-empty 语义；改用受限查询；folder lookup 只服务实际渲染投影并使用有界内存缓存。
- **拟议方向**：拆分 last/dominant/recent SQL，zero-history 跳过 recent；以 `(source, session_id)` 为键增加容量和正/负 TTL 均受限的 Observer folder cache。
- **验收标准**：zero-history 不发起 500-row 查询；摘要与可见性一致；folder keys 不来自隐藏行；缓存 source 隔离、可淘汰、未命中后文件出现可被发现。
- **Trellis**：[`08-06-observer-zero-history-query`](./.trellis/tasks/08-06-observer-zero-history-query/)

### AIO-PENDING-023 - 插件激活与持久隔离

- **状态**：`planned`
- **日期**：2026-08-06
- **待执行交付**：精确 activation policy、startup/command/gateway gate、600 秒三次严重故障 quarantine、revalidate 与废弃事件迁移已汇入 `codex/final-hardening-unified`，并继承主线 Gateway Bearer Token 合同。本项不再单独验证或合并 PR #94；由统一 PR 的精确远端 head 执行 Actions 漂移、bindings 和完整回归后，与 AUD-055、AUD-002、AUD-035 一起合并；合并前保持 `planned`。
- **观察问题**：`activationEvents` 当前基本不参与 command/gateway 调度，重复 runtime failure 只触发进程内 circuit breaker，重启后清零且没有校验恢复路径。
- **锁定决策**：仅支持 `onStartup`、`onCommand:*`、`onGatewayHook:*`，空数组保持 legacy；显式拒绝两种废弃事件；10 分钟内 3 次严重故障持久 quarantine；revalidate 成功只到 disabled。
- **拟议方向**：引入精确 ActivationPolicy gate，统一 startup/command/gateway 严重故障分类和原子阈值事务，隔离后刷新 gateway snapshot/host，增加 quarantined-only revalidate。
- **验收标准**：不匹配事件不启动 host；legacy 兼容；第三次 crash/runtime/timeout 跨重启隔离且保留当前 fail-open/fail-closed；policy rejection 不计数；恢复不自动启用，历史废弃事件迁移原因可见。
- **Trellis**：[`08-06-plugin-activation-quarantine`](./.trellis/tasks/08-06-plugin-activation-quarantine/)
