# 完成待办 012/014/015 并补丁发布

## Goal

完成三个已锁定界面待办。`AIO-PENDING-012` 与 `014` 已随 `aio-coding-hub-v0.60.45` 完成；`015` 的 24px 行密度已交付，但用户实际截图重新打开了横向布局验收，需修正后发布 `aio-coding-hub-v0.60.46`，再归档整批任务。

## Requirements

- 子任务 `08-03-tui-request-card-model-route-lines` 独立负责 AIO-PENDING-012，并以 PR #24、CI 和 `0.60.45` 证据归档。
- 子任务 `08-03-provider-account-usage-heartbeat-ui` 独立负责 AIO-PENDING-014，并以 PR #24、CI 和 `0.60.45` 证据归档。
- 子任务 `08-03-tray-provider-mini-density` 继续负责 AIO-PENDING-015；保留 24px 密度基础，修正面板宽度、名称省略和成功/失败固定列。
- 三项保持可独立审查，不改变网关路由、账户余额后端调度、Observer 协议或 Tray 数据口径。
- 本地只运行 Node、TypeScript、前端测试、Vite build、静态检查和前端 fixture 截图；Rust/native 验证、格式化、锁文件与绑定漂移由 GitHub Actions 负责。
- 仅操作 `origin`。015 续作通过独立功能 PR 合入 `main`，随后以独立版本 PR 将补丁版本从 `0.60.45` 提升到 `0.60.46`。
- Release 必须复用精确 `main` SHA 的成功 CI 候选制品，不在标签工作流重新构建。

## Acceptance Criteria

- [x] AIO-PENDING-012 满足自身验收并随 `aio-coding-hub-v0.60.45` 发布。
- [x] AIO-PENDING-014 满足自身验收并随 `aio-coding-hub-v0.60.45` 发布。
- [ ] AIO-PENDING-015 满足重新打开后的固定宽度、名称省略、固定计数列与视觉验收。
- [ ] 015 功能 PR、版本 PR 与精确 main SHA 的 CI 全部成功。
- [ ] `aio-coding-hub-v0.60.46` 正式发布，标签指向版本 PR 的 main 合并提交。
- [ ] Release 的 12 个资产齐全，`SHA256SUMS.txt` 中 11 个载荷校验通过，`latest.json` 有效。
- [ ] AIO-PENDING-015 在发布后携带 PR、提交、CI、截图和 Release 证据迁入完成归档，父任务随后归档。

## Notes

- `aio-coding-hub-v0.60.45` 及其资产仍是 012/014 的有效完成证据，也是 015 首轮 24px 密度基础的交付证据。
- 用户截图推翻了“015 已全部验收”的状态，不回滚已发布版本，也不覆盖历史证据；只通过新补丁完成剩余横向布局验收。
