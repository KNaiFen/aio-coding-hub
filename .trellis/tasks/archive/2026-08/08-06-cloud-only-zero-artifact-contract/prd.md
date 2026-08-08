# AUD-054 云端验证与本地零产物合同

## Goal

建立“本地零产物、云端完整验证”的仓库强制合同，消除文档、脚本、Trellis 模板与 CI 之间的相互矛盾。

## Requirements

- 禁止本地 Rust、Tauri、Node 依赖安装、开发服务、类型检查、Lint、测试和构建。
- 本地只保留无依赖 Node 源码合同检查、源文件解析和 `git diff --check`。
- 更新 AGENTS、README 中英文、活跃 Trellis 规范和 agent 模板；不改写历史任务或归档。
- 根与 workspace package scripts 必须明确为 GitHub Actions 专用，不能继续作为本地入口。
- 新增零依赖本地检查入口及 self-test，扫描受控脚本、文档、模板和工作流，失败时关闭。
- CI 必须锁定 Rust fmt、lock/bindings 漂移、Clippy、Rust tests、依赖审计和前端 lint/typecheck/test/build。
- 保留 `ci.yml` 的全量 `workflow_dispatch` 与按需 `dev-build`，不把四平台桌面打包变成每个 PR 的必需项。

## Acceptance Criteria

- [ ] 受控文档和脚本不再建议或允许本地安装、dev、质量门或构建。
- [ ] 零依赖合同正例通过，任一受控本地入口、死文本绕行或 CI 门缺失的反例均失败。
- [ ] `support-contract` 直接运行新合同，`frontend` 和 `rust` job 仍覆盖完整云端质量门。
- [ ] `workflow_dispatch` 全量 CI 与 `dev-build` 手动分发语义保持不变。
- [ ] 本地仅运行新合同/self-test、Node 语法、YAML/JSON 解析和差异检查。
- [ ] PR 合并后精确清理仓库级产物，未触碰全局 Cargo、pnpm store 或其他项目文件。

## Notes

- 这是后续七项任务的验证前置，关联 `AIO-PENDING-016`。
