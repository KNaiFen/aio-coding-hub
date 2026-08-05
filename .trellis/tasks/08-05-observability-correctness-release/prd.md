# 修复可观测性正确性并发布 0.60.49

## Goal

统一修复 Token 速度、请求日志刷新和供应商状态条，并完成 PR 与补丁发版。

## Requirements

- 以一个 ready PR 交付三个独立修复：最终成功上游尝试吞吐、请求日志离顶缓冲、桌面供应商状态格自然推进。
- 从最新 `origin/main` 开发，仅操作 `origin`，不合并或推送 `upstream`。
- 版本统一提升到 `0.60.49`，合并后复用精确 main SHA 的 CI release candidate 发布。
- 本地不运行 Cargo、rustfmt、Clippy、Rust 测试、Specta 或 Tauri；原生验证和生成文件由 GitHub Actions 负责。
- 交付报告解释 `aio-coding-hub-v0.60.48` 之后 PR #53 至 #67，以及本次修复提交的作用。

## Acceptance Criteria

- [ ] 三个子任务各自满足验收条件，并通过一次全量跨层审查。
- [ ] 前端 lint、typecheck、单测和 Vite build 本地通过，原生 CI 全绿。
- [ ] ready PR 合并到 `origin/main`，合并 SHA 的 main CI 生成唯一 release candidate。
- [ ] `aio-coding-hub-v0.60.49` 指向该合并 SHA，Release 资产、签名、校验和与 updater manifest 完整。
- [ ] 本地最终回到已快进的 `main`，任务记录包含 PR、提交和 Release 证据。

## Notes

- 当前 `PENDING.md` 没有未解决条目。
- 父任务负责集成、PR、发版和最终提交说明；产品实现由三个子任务负责。
