# AIO Coding Hub Agent Rules

本文件只定义项目边界；GKD 生命周期由已安装的 canonical bundle 提供，不在仓库复制实现。

## 角色入口

- main 使用 `$gkd-main`：读取需求与项目 policy，调用 `gkd-task`/`gkd-role` 完成规划、路由和收尾。
- 执行者使用 `$gkd-execute`：只在 bundle 返回的任务 worktree 中施工，完成 claim 到 delivery 后停止。
- 固定 head 验收由 `gkd_acceptor` 完成；执行者不得验收、合并或归档。
- 本地验证使用 `$gkd-local-verify`，唯一入口是 `scripts/gkd-verify --base-sha <full-lowercase-sha>`。
- 长时 GitHub 检查使用 `$gkd-ci-monitor`；CI 终态只绑定明确的 PR/head。

## GKD 事实与信任边界

- `gkd-task` 生成并维护 `requirements`、`plan`、`authorization`、`offer`、`claim`、`delivery`、`rework` 和最终状态；禁止手写协调 JSON。
- `gkd-task bootstrap` 必须先绑定 `.gkd/policy.json`、canonical repository、origin、base branch 和 required checks；policy、bundle、route 或 head 漂移时 fail-closed。
- `gkd-role route` 返回六门决策；可信 main 通过 `TrustedMainRuntimeBridge.prepare` 启动唯一 `gkd_executor`。恢复只使用 bridge 的 `recover`。
- executor 启动后才进入 `implementing`；健康等待按 bundle 的 `wait-transition` 合同执行，终态、阻塞、漂移和超时立即停止。
- 交付后由 `gkd_acceptor` 对固定 head、实时 CI 和 diff 独立验收；拒绝时由可信 main 使用 `gkd-task rework` 创建全新 offer，禁止复用旧 claim。
- 只有可信 main 在验收通过后执行窄 merge、归档和清理；任何新提交都会使旧交付与验收证据失效。

## 项目与 Git 边界

- Keep the local checkout zero-artifact.
- 默认远端为 `origin`，GitHub 操作显式使用 `-R KNaiFen/aio-coding-hub`；不推送远端 `main`。
- 一个 worktree 同时只有一个 bundle 登记的 writer；不清理来源不明的修改、worktree 或分支。
- Git 内唯一的 GKD 项目事实为 `.gkd/policy.json`、`.gkd/bundle-pin.json`、`.gkd/review-adapter.json`、`.gkd/adapter-policy.json`、`.gkd/resource-facts.json`、`.gkd/history-adapter.json` 和对应 adapter 校验脚本。角色、claim receipt、runtime inventory 只存在本机 staging。
- GitHub Actions 承担依赖安装、前端/Rust 检查、audit、签名和桌面打包；普通 PR 依赖自动 `ci-gate` 与 `pr-title`。
- 普通 PR 等自动 `ci-gate` 与 `pr-title`，不额外手动启动常规 `ci`。
- upstream 合并只做最小集成；若上游与 fork 行为冲突，停止并报告证据。

## 不可绕过的规则

- 需求、范围、非目标和可判定 AC 必须先写入 bundle 任务 Markdown；材料性变更必须重新获批。
- 不使用通用 worker、临时 agent、手写状态、候选 worktree 脚本或管理员绕过替代 canonical bundle。
- 不安装依赖，不运行 package-manager、开发服务器、lint、类型检查、测试、构建、Cargo、Tauri、签名或打包；只运行 `$gkd-local-verify` 允许的零依赖检查。
- 不记录真实凭据、完整对话、全量日志或未脱敏用户数据。

<!-- TRELLIS:START -->
Trellis 目录仅保存既有项目资料和历史记录；它不是 GKD 的任务状态、路由或验收入口。
<!-- TRELLIS:END -->
