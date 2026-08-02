# 完成 PENDING 001-011 并发布 0.60.43

## Goal

按六个功能批次完成全部待办、合并、发布并交付制品

## Requirements

- 完成 `PENDING.md` 中 AIO-PENDING-001 至 AIO-PENDING-011，不遗漏条目。
- 六个功能批次在同一分支与同一功能 PR 内按依赖顺序交付；功能合并后单独完成 0.60.43 发布记账。
- 观测、余额和界面功能全部失败开放，不改变无关网关行为。
- 本机只运行 Node/TypeScript/前端验证，所有 Rust/native 校验与打包交给 GitHub Actions。

## Acceptance Criteria

- [ ] 六个子任务全部合并并通过完整 CI 与 macOS 云端构建。
- [ ] P001-P011 均有提交、PR、合并与发布证据并标记为 `done`。
- [ ] 发布 `aio-coding-hub-v0.60.43`，校验所有桌面、TUI、更新器和校验和制品。

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
