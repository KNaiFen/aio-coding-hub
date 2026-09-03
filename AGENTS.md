# AIO Coding Hub Agent Rules

本文件只定义项目边界；普通任务统一使用用户级 `$gkd-main` skill 的
manual-first worktree 流程，仓库不复制任何外部生命周期实现。

## 工作流

- main 创建独立 worktree，并在其中写明具体目标、worktree 和行为约束的 `plan.md`。
- 施工 session 只读取声明的 `plan.md` 和适用的 `AGENTS.md`，只修改计划范围。
- 在判断、里程碑、阻塞或验证结果影响交接时更新 `progress.md`。
- main 审查 diff、`plan.md` 和 `progress.md`，在 `review.md` 记录通过、返工要求和剩余风险。
- 执行 session 不验收、合并、发布或启动其他任务；通过后由 main 使用普通 Git 操作保留或合并。

## 禁止项

- 不使用外部任务状态 JSON、生命周期命令、独立验收、CI 监控或其他旧工作流 skill。
- 不在仓库复制、安装或假设存在外部工作流命令。

## 项目与 Git 边界

- Keep the local checkout zero-artifact.
- 默认远端为 `origin`，GitHub 操作显式使用 `-R KNaiFen/aio-coding-hub`；不推送远端 `main`。
- 一个 worktree 同时只有一个 writer；不清理来源不明的修改、worktree 或分支。
- GitHub Actions 承担依赖安装、前端/Rust 检查、audit、签名和桌面打包；普通 PR 依赖自动 `ci-gate` 与 `pr-title`。
- 普通 PR 等自动 `ci-gate` 与 `pr-title`，不额外手动启动常规 `ci`。
- upstream 合并只做最小集成；若上游与 fork 行为冲突，停止并报告证据。

## 不可绕过的规则

- 需求、范围、非目标和可判定 AC 必须先写入 `plan.md`；材料性变更必须重新获批。
- 不使用通用 worker、临时 agent、手写状态或管理员绕过替代 `$gkd-main` 流程。
- 不安装依赖，不运行 package-manager、开发服务器、lint、类型检查、测试、构建、Cargo、Tauri、签名或打包；只运行计划中批准的零依赖检查。
- 不记录真实凭据、完整对话、全量日志或未脱敏用户数据。
