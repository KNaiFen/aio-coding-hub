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

### AIO-PENDING-016 - 云端验证与本地零产物合同

- **状态**：`planned`
- **日期**：2026-08-06
- **观察问题**：仓库文档、package/workspace 脚本和 Trellis 模板仍保留会安装依赖或产生 Node/Rust/Tauri 本地产物的入口，与云端构建策略冲突。
- **锁定决策**：本地禁止依赖安装、dev、类型检查、Lint、测试、构建、Cargo 与 Tauri；只允许零依赖 Node 源码合同/解析检查和 `git diff --check`。跨平台桌面打包不升级为每个 PR 的必需任务。
- **拟议方向**：更新活跃规则与 README，限制受控脚本为 GitHub Actions 使用，演进零依赖合同检查并加强 CI 静态质量门；历史任务和归档不改写。
- **验收标准**：本地入口检查能拒绝仓库受控的依赖/构建命令；`ci.yml` 全量 workflow_dispatch 的 rustfmt、bindings、Clippy、Rust tests、前端质量门和 audit 均通过；合并后只清理重新核验过的仓库级产物。
- **Trellis**：[`08-06-cloud-only-zero-artifact-contract`](./.trellis/tasks/08-06-cloud-only-zero-artifact-contract/)

### AIO-PENDING-017 - Provider Sync session-only 快照

- **状态**：`planned`
- **日期**：2026-08-06
- **观察问题**：Provider Sync 当前扫描并备份 `archived_sessions`、SQLite 与全局状态，且旧格式 managed backup 最多保留五代，造成与恢复目标无关的空间增长。
- **锁定决策**：新格式只处理活动 `sessions`，不再扫描、改写或备份 `archived_sessions`；只保留最新一代新格式 managed backup；只删除 manifest 精确证明所有权的旧格式 managed backup。
- **拟议方向**：引入 v2 session-only manifest 和严格 managed/unmanaged 分类，在首次成功创建 v2 后迁移清理 v1，并保留同步失败的完整回滚。
- **验收标准**：归档会话字节不变；成功后最多一代 v2；v1 managed 可迁移、非受管/损坏/symlink 保留；云端 Rust 覆盖迁移、回滚和所有权边界。
- **Trellis**：[`08-06-provider-sync-session-snapshot`](./.trellis/tasks/08-06-provider-sync-session-snapshot/)

### AIO-PENDING-019 - 非回环 Gateway Bearer Token

- **状态**：`planned`
- **日期**：2026-08-06
- **待执行交付**：代码已在独立候选分支 `codex/aud019-gateway-lan-bearer-token` 重放为 `51224cf2`，包含精确云端格式补丁和 CI 诊断修复；本地 cloud-only checker/self-test 与 `git diff --check` 通过，未运行 Cargo、pnpm、Tauri、类型检查、Lint、测试或构建。PR、精确 head 全量 CI、生成绑定核验、主线门和合并统一后置；在完成云端验证与合并前保持 `planned`。
- **观察问题**：现有 LAN/custom 非回环 Gateway 缺少统一路由鉴权，provider/forwarded header 和 provider 专用路由扩大了可伪造信任面。
- **锁定决策**：保留 LAN；真实 TCP peer 非回环时所有路由含 health 必须使用应用生成 Bearer Token；loopback 兼容。Token 只展示一次、仅持久化摘要；删除 provider 专用路由、forced-provider 数据流和 Claude Terminal 入口。
- **拟议方向**：在最外层 Axum middleware 基于 `ConnectInfo<SocketAddr>` 鉴权并剥离敏感/转发身份头，支持旧 LAN 迁移、未确认重启轮换、主动轮换和 WSL 明文即时同步。
- **验收标准**：非回环无/错 token 在任何副作用前返回 401，正确 token 可用且认证/伪造头不外传；旧 token 立即失效；一次性明文不进入持久化、cache、日志或错误；WSL 同步成功或明确失败。
- **Trellis**：[`08-06-gateway-lan-bearer-token`](./.trellis/tasks/08-06-gateway-lan-bearer-token/)

### AIO-PENDING-020 - 跨重启数据重置

- **状态**：`planned`
- **日期**：2026-08-06
- **待执行交付**：代码候选已在独立分支 `codex/aud020-cross-restart-data-reset` 重放为 `c4c069e2`；本地仅通过零依赖 cloud-only checker/self-test 与 `git diff --check`，未运行 Cargo、pnpm、Tauri、生成器、类型检查、Lint、测试或构建。PR、精确 head 全量 CI、跨平台故障注入与主线门统一后置；在云端验证和合并前保持 `planned`。
- **观察问题**：数据重置在当前进程逐文件删除，而退出清理可能再次初始化数据库；文件占用或部分失败会让应用带着不完整清理继续运行。
- **锁定决策**：reset IPC 只持久写 marker 后走专用退出；下次启动在 DB、observer、gateway 和后台任务前删除；失败保留 marker 并进入 retry/exit 维护态。
- **拟议方向**：建立应用级 maintenance coordinator、幂等 marker 生命周期和专用退出路径，并 gate 原生/前端启动任务。
- **验收标准**：marker durable；失败不清 marker、不启动普通服务；重试完整目标集合；成功清 marker 后首次正常启动；云端覆盖跨平台文件占用与 UI 文案。
- **Trellis**：[`08-06-cross-restart-data-reset`](./.trellis/tasks/08-06-cross-restart-data-reset/)

### AIO-PENDING-021 - SQLite/文件系统双写恢复

- **状态**：`planned`
- **日期**：2026-08-06
- **观察问题**：Prompt、MCP、Skills 和 workspace switch 在 SQLite 与外部文件之间双写，多处补偿吞错；进程中断或 commit/恢复失败可留下跨重启漂移。
- **锁定决策**：已提交 SQLite 状态为权威；任何外部副作用前 durable journal；启动前阻断自动对账并复用 AUD-008 维护态；补偿失败必须可见且错误摘要脱敏。
- **拟议方向**：统一 prepare-first 操作协议。Skills 的不可由 metadata 重建内容使用 journal 专属、带 ownership/hash 的临时 staging/backup，resolved 后回收。
- **验收标准**：外部写、commit、补偿、重启各故障点最终收敛到 DB；artifact 缺失/越界/hash 不符保持维护态；普通写 IPC 不与 replay 交错；journal/错误不泄露正文、env/header 或密钥。
- **Trellis**：[`08-06-filesystem-recovery-journal`](./.trellis/tasks/08-06-filesystem-recovery-journal/)

### AIO-PENDING-022 - history_limit=0 Observer I/O

- **状态**：`planned`
- **日期**：2026-08-06
- **待执行交付**：代码已在独立候选分支 `codex/aud022-observer-zero-history-query` 重放为 `dc9a4418`；本地 cloud-only checker/self-test 与 `git diff --check` 通过，未运行 Cargo、pnpm、Tauri、生成器、类型检查、Lint、测试或构建。PR、精确 head 全量 CI、bindings 核验、主线门和合并统一后置；在云端验证和合并前保持 `planned`。
- **观察问题**：`history_limit=0` 只在投影末端生效，之前仍读取并构造 500 条日志，并基于隐藏历史触发完整 Claude/Codex session-folder 扫描。
- **锁定决策**：保持 last/dominant/active/all-scope 与 recent ready-empty 语义；改用受限查询；folder lookup 只服务实际渲染投影并使用有界内存缓存。
- **拟议方向**：拆分 last/dominant/recent SQL，zero-history 跳过 recent；以 `(source, session_id)` 为键增加容量和正/负 TTL 均受限的 Observer folder cache。
- **验收标准**：zero-history 不发起 500-row 查询；摘要与可见性一致；folder keys 不来自隐藏行；缓存 source 隔离、可淘汰、未命中后文件出现可被发现。
- **Trellis**：[`08-06-observer-zero-history-query`](./.trellis/tasks/08-06-observer-zero-history-query/)

### AIO-PENDING-023 - 插件激活与持久隔离

- **状态**：`planned`
- **日期**：2026-08-06
- **观察问题**：`activationEvents` 当前基本不参与 command/gateway 调度，重复 runtime failure 只触发进程内 circuit breaker，重启后清零且没有校验恢复路径。
- **锁定决策**：仅支持 `onStartup`、`onCommand:*`、`onGatewayHook:*`，空数组保持 legacy；显式拒绝两种废弃事件；10 分钟内 3 次严重故障持久 quarantine；revalidate 成功只到 disabled。
- **拟议方向**：引入精确 ActivationPolicy gate，统一 startup/command/gateway 严重故障分类和原子阈值事务，隔离后刷新 gateway snapshot/host，增加 quarantined-only revalidate。
- **验收标准**：不匹配事件不启动 host；legacy 兼容；第三次 crash/runtime/timeout 跨重启隔离且保留当前 fail-open/fail-closed；policy rejection 不计数；恢复不自动启用，历史废弃事件迁移原因可见。
- **Trellis**：[`08-06-plugin-activation-quarantine`](./.trellis/tasks/08-06-plugin-activation-quarantine/)
