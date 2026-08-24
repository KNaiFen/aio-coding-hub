# AIO Coding Hub Agent Rules

本文件只定义所有 worktree 和 session 共同遵守的权限与硬边界。具体步骤按角色加载 skill，不在这里展开生命周期教程。

## 角色入口

- **main session**：使用 `$gkd-main`。负责需求与方案、任务路由、worktree 生命周期、验收协调、知识库/PENDING 和清理。
- **独立执行 session**：使用 `$gkd-execute`。只在登记 worktree 施工，维护任务分支、PR、CI 和 `delivery.md`，交付后暂停。
- **验收 session/agent**：使用 `$gkd-accept`。只读审查固定 PR head；无阻塞 finding 且机器门通过后可同步合并该 head，但不写文件或任务状态，不归档或清理。
- **本地验证**：修改仓库文件后使用 `$gkd-local-verify`；长时等待固定 PR head 的 CI 使用 `$gkd-ci-monitor`。
- `.trellis/agents/*.md` 只属于 `trellis channel`，不得作为独立执行 session 或 main 验收角色使用。

角色说明不构成授权。新窗口默认是顶层 session，不会因 `.codex/agents` 自动变成执行者；以 handoff、当前唯一写者和对应 skill 为准。

## 共享事实与写权

- 一个 worktree 同时只有一个唯一写者；其他 session 在明确交回写权前不得写入。
- 活动协调状态以 `task.json` 为准，由 `task.py` 写入；用户决定、范围、AC、设计和结果保存在任务 Markdown；commit、PR head、CI 和 merge 以实时 Git/GitHub 为准。
- 执行、交付、CI 和验收证据必须绑定同一完整 PR head SHA；任何新提交都会使旧结论失效。
- 任务记录不能覆盖当前代码、机器合同或现行规范。发现冲突时以当前代码和合同为准并报告 main。
- 不清理含未知修改、来源不明文件或仍被 session 使用的 worktree；归属不清时停止。

## Git、远端与本地边界

- Keep the local checkout zero-artifact.
- 仓库和 PR 默认使用 `origin`；`upstream` 只读抓取。GitHub 操作显式使用 `-R KNaiFen/aio-coding-hub`。
- 不推送远端 `main`。执行 session 只推任务分支；PR 仅由 main 或获授权的 `$gkd-accept` 通过固定 head 验收命令合并。
- 常规 checkout 不安装依赖，不运行 package-manager、开发服务器、格式化器、lint、类型检查、测试、构建、Cargo、rustfmt、Clippy、Specta、Tauri、签名或打包。
- 本地验证只使用 `$gkd-local-verify` 调用 `scripts/gkd-verify --base-sha <登记的完整 SHA>`；该版本化入口固定委托 `node scripts/check-local-verification.mjs --base <登记的完整 SHA>`，执行依赖无关合同/selftest、变更 Node 文件语法和 diff 检查，不接受命令透传。
- Git 内的 GKD 项目事实仅为 `.gkd/policy.json`、`.gkd/bundle-pin.json`、`.gkd/review-adapter.json` 与 AIO 专有的 `.gkd/adapter-policy.json`、`.gkd/resource-facts.json`；adapter policy 只声明版本化本地验证、公开 workflow 资源/artifact 和 release promotion 边界，resource facts 只保存公开可复核的 policy binding、runner 来源和未验证资源边界。二者都不是通用 GKD schema、实时 workflow/API 或资源扫描、账单事实；角色技能、claim receipt 和 runtime inventory 属于 project-local staging，不能提交或替代项目 policy。

- GitHub Actions 承担依赖安装、前端/Rust 检查、audit、签名和桌面打包。普通 PR 等自动 `ci-gate` 与 `pr-title`，不额外手动启动常规 `ci`。
- upstream merge/drift repair 只做最小集成；若上游与 fork 产品行为冲突，停止并把证据和选项交给 main/用户。

## 独立执行硬边界

- 唯一入口是 `.trellis/tasks/<task>/execution.md`。开始写入前运行交接给出的 `task.py status` 与 `task.py doctor`；失败即停止，不手改 branch、worktree、base、planning commit 或 writer 来绕过。
- 只按锁定范围施工。公共 API、迁移、真实凭据、兼容性、安全边界、发布配置、范围外重要文件或材料性未决问题出现时，先 `task.py block` 并报告 main。
- 可以提交/推送任务分支、创建或更新 PR、修复任务相关 CI；不得合并、自动合并、归档、清理、写 main 验收或运行 main 收尾。
- 交付要求和阻塞恢复只读 [执行与交付](docs/operations/multi-worktree/execution-and-delivery.md)。完成后停止写入，等待 `$gkd-accept`/main 验收或明确返工。

## Main 专属硬边界

- 用户授权实施后、修改前先写仓库记录：简单连续任务用月度 change record；复杂、隔离或委派任务使用 Trellis 任务包和 worktree。
- 新 worktree 从已抓取的完整 `origin/main` SHA 派生。使用 `task.py delegate/start/handoff` 登记和生成交接，不手写机器状态副本。
- 验收必须针对暂停执行者后的实时 PR diff、固定 head 和 CI。普通探索 subagent 只提供线索；`$gkd-accept` 是唯一可在验收通过后调用同步合并命令的验收角色。验收命令只从干净、已同步 `origin/main` 的可信 main checkout 运行，候选 worktree 只作为显式只读输入。
- `main-direct-fix` 仅限写权已登记转移、方案确定、任务范围内的记录性文档修正，且实时 scope 证明不会触发长检查；否则交回执行 session。
- main 可直接验收并运行 `task.py accept` 合并，也可交给 `$gkd-accept` 验收后从可信 main checkout 调用该命令。只有 main 可写终态记录、archive、更新长期知识/PENDING、删除已合并且干净无占用的 worktree/branch。完整步骤由 `$gkd-main` 按阶段加载。
- `PENDING.md` 的 `pending/planned` 条目必须在正式计划前全部审阅；仅在完成合并和验证后迁入 `PENDING_COMPLETED.md`。放弃需要用户明确决定。
- fork release 默认只递增 patch；更大版本变化需用户决定。release 先解析或创建 tag，再把不可变 commit SHA 传给构建。

<!-- TRELLIS:START -->
# Trellis Instructions

These instructions are for AI assistants working in this project.

This project is managed by Trellis. The working knowledge you need lives under `.trellis/`:

- `.trellis/workflow.md` — development phases, when to create tasks, skill routing
- `.trellis/spec/` — package- and layer-scoped coding guidelines (read before writing code in a given layer)
- `.trellis/workspace/` — per-developer journals and session traces
- `.trellis/tasks/` — active and archived tasks (PRDs, research, jsonl context)

Use the `gkd-*` user skills and repository `task.py` commands for role routing and durable state. `$gkd-local-verify` owns the fixed local runner and `$gkd-ci-monitor` owns frozen-head waiting. Independent execution sessions stop after `deliver`、最终 head CI 和暂停；`$gkd-accept` 可在固定 head 验收通过后同步合并，main 继续负责记录、归档和清理。

Codex role workflows are installed as user skills under `~/.codex/skills/gkd-*`; the repository keeps role names and machine contracts, not a second copy of the skill bodies. Optional project custom subagents may live in `.codex/agents/`.

Managed by Trellis. Edits outside this block are preserved; edits inside may be overwritten by a future `trellis update`.

<!-- TRELLIS:END -->
