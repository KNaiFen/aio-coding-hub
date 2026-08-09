# 实施清单

1. [x] 激活子任务并准备独立分支/worktree。
2. [x] 扩展筛选 DTO 与 SQLite 错误/时间谓词，并将用户中断从“全部错误”中排除。
3. [x] 新增 managed snapshot state、成员/按 ID 查询和 v2 IPC，并限制快照成员上限。
4. [x] 同步生成绑定、service、query key、hook 与 reducer 状态。
5. [x] 实现错误分段、时间弹层和任意页输入/总页数；最新页刷新会重建快照，历史页保持稳定。
6. [x] 补齐 Rust 合同测试和前端交互/边界测试，包括合法旧版 attempts 标量、跨午夜时间范围和快照上限。
7. [x] 运行 Vitest、TypeScript、ESLint、Prettier 和 Vite build。
8. [ ] 通过 GitHub Actions 完成 Rust、绑定漂移和 `dev-build` 原生验证。
9. [x] 更新跨层规范。
10. [x] 已同步 `origin/main@02b9980d`（PR #51 合并提交）并完成实现审计；无直接路径冲突，PR #51 的 attempts JSON 兼容路径已补充合法标量防护。

## 回滚点

- 旧 cursor 命令保持不变；可独立回滚新 snapshot page 命令和 LogsPage 控件。
