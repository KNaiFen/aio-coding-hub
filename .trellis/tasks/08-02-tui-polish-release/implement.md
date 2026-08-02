# 实施计划

1. 完成 `08-02-pending-completed-archive`：迁移历史、更新规则、检查链接与状态。
2. 完成 `08-02-tui-summary-local-time`：共享摘要、本地时区转换、回归测试和 TUI 合同更新。
3. 完成 `08-02-macos-tray-mini-polish`：18 桶投影、总计数字段、TS 校验、紧凑 UI、透明 Popover 圆角窗口和测试。
4. 运行允许的本地前端测试、typecheck、lint、Vite build、`git diff --check`、敏感信息和未跟踪文件审计。
5. 做五轴代码审查，确认没有改变路由、熔断、余额刷新或手动探测语义。
6. 按逻辑提交计划取得一次确认后提交并推送 `origin`，创建 PR，等待云端完整 CI；应用必要的 CI 漂移补丁并复跑。
7. 合并功能 PR，提升版本到 `0.60.44`，通过 PR 合并版本提交。
8. 等待精确 `main` CI 成功，发布正式 Release，核对资产数量、清单和 SHA-256。
9. 归档三个子任务和父任务，记录 PR、提交、CI、Release 与资产证据。
