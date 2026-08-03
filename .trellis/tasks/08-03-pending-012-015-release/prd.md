# 处理待办 012/014/015 并发布 0.60.45

## Goal

在一个补丁版本中完成三个已锁定的界面待办，经 `origin` PR 合并到 `main`，发布并验证 `aio-coding-hub-v0.60.45`，最后归档待办与任务证据。

## Requirements

- 子任务 `08-03-tui-request-card-model-route-lines` 独立负责 AIO-PENDING-012。
- 子任务 `08-03-provider-account-usage-heartbeat-ui` 独立负责 AIO-PENDING-014。
- 子任务 `08-03-tray-provider-mini-density` 独立负责 AIO-PENDING-015。
- 三项保持可独立审查，不改变网关路由、账户余额后端调度、Observer 协议或 Tray 数据口径。
- 本地只运行 Node、TypeScript、前端测试、Vite build 和静态检查；Rust/native 验证、格式化、锁文件与绑定漂移由 GitHub Actions 负责。
- 仅操作 `origin`，通过功能 PR 和独立版本 PR合入 `main`，版本从 `0.60.44` 提升到 `0.60.45`。
- Release 必须复用精确 `main` SHA 的成功 CI 候选制品，不在标签工作流重新构建。

## Acceptance Criteria

- [ ] 三个子任务分别满足自身验收标准，且跨任务审查确认边界没有扩大。
- [ ] 功能 PR、发布 PR 与精确 main SHA 的 CI 全部成功。
- [ ] `aio-coding-hub-v0.60.45` 正式发布，标签指向版本 PR 的 main 合并提交。
- [ ] Release 的 12 个资产齐全，`SHA256SUMS.txt` 中 11 个载荷校验通过，`latest.json` 有效。
- [ ] 三个 PENDING 条目在发布后携带 PR、提交、CI 和 Release 证据迁入完成归档。

## Notes

- 用户明确决定不增加 Tray 截图、DPR 或实机视觉门禁；代码、前端测试、云端 Rust 测试和发布构建仍必须通过。
