# AIO Coding Hub Agent Rules

在系统、开发者和用户明确指令允许的范围内，生命周期、路线、角色、授权、验收和收尾以用户级 `$gkd-main` skill 为准。本文件只补充 AIO 的项目约束；项目规则与 GKD 冲突时修正项目规则。

## 工作流

- main 在施工前维护 `.gkd/plan.md`，写清需求、范围、非目标和可判定 AC，获批后按 GKD 选择 `direct-main` 或 `delegated`；材料性变化交回 main 确认。
- 仅 delegated 在独立 worktree 生成 `.gkd/execution.md`，执行 session 按该交接和适用规则施工，并在 `.gkd/progress.md` 记录重要判断、阻塞和验证事实；默认手动交接，自动执行须由用户明确选择。
- main 在 `.gkd/review.md` 记录审查；独立验收、CI 监控与收尾按 GKD 路由。执行 session 完成后停止并交回 main。
- 资料按改动涉及的行为和边界读取，入口见 [项目知识库](docs/README.md)。历史记录不指导新任务。
- 仓库不复制外部生命周期实现，不维护旧状态 JSON、固定 head 验收器或常驻 watcher。

## 项目与 Git 边界

- 本地工作树保持零产物。
- 不安装依赖，不运行 package-manager、开发服务器、lint、类型检查、测试、构建、Cargo、Tauri、签名或打包；本地仅执行计划批准、无依赖且不写产物的直接 Node 检查及只读文件/Git 检查。
- 默认远端为 `origin`，GitHub 操作显式使用 `-R KNaiFen/aio-coding-hub`；不推送远端 `main`。
- 实施使用从已更新 `origin/main` 建立的任务分支；`direct-main` 指主代理直接执行，仍经任务分支 PR 集成。
- 每个完成的任务使用简短中文 Conventional Commit，经任务分支 PR 合入；squash 后同步远端合并结果，不再把原任务分支合入本地 `main`。
- 本地 `main` 有独有提交时先比较文件差异并保留现场，不用 ahead 数量推断未发布功能，不自动重置历史。
- 一个 worktree 同时只有一个 writer；不清理来源不明的修改、worktree 或分支。
- GitHub Actions 承担依赖安装、前端/Rust 检查、audit、签名和桌面打包；按现行分类器选择 job，提交前本地检查不替代合并前 CI。
- 普通 PR 等待自动 `ci-gate` 和 `pr-title`，常规验证不重复手动运行 `ci`。
- upstream 合并只做最小集成；若上游与 fork 行为冲突，停止并报告证据。
- 不记录真实凭据、完整对话、全量日志或未脱敏用户数据。
