# 模型路由与任务结束提醒可靠性

## Goal

在一个短期功能分支中交付可配置模型路由和可靠的任务结束提醒，确保两项辅助能力均不削弱 AIO 的核心请求转发能力。

## Requirements

- 子任务一实现四类 CLI 共用的全局模型路由和供应商整套覆盖，并按最终出站模型计价。
- 子任务二恢复 Codex 120 秒静默期，并在通知前用后端活跃请求快照消除漏事件误报。
- 桌面首页、请求日志和 TUI 均能观测主动模型路由。
- 所有异常分类、审计和通知校验都必须 fail-open，不进入请求转发热路径。
- 只操作 `origin`，通过 PR 合入 `main`；本任务不提版本、不发 Release。

## Acceptance Criteria

- [ ] 两个子任务均满足各自验收标准并形成独立逻辑提交。
- [ ] 本地 Node/TypeScript/前端验证通过，所有 Rust/TUI/迁移/生成绑定检查由 GitHub Actions 通过。
- [ ] PR CI 全绿后合入 `main`，且不提交 `.trellis/workspace/KNaiFen/` 或其它无关未跟踪文件。

## Child Tasks

- `08-02-model-routing-policy`: 配置、迁移、网关改写、计价、桌面与 TUI 审计。
- `08-02-task-complete-notify-reliability`: 分 CLI 静默期与权威活跃快照校验。
