# macOS 托盘供应商面板

## Goal

实现 P010

## Requirements

- 仅 macOS 在 Tray 悬停时显示无边框、不抢焦点的供应商 mini 面板。
- 每次打开冻结 CLI，按活跃请求、最近完成请求、首个启用网关顺序选择。
- 只列当前路由与全局均开启的供应商，暂时不可用者保留并标记原因。
- 最多同时显示 10 行，超出滚动；左键与右键行为保持不变。

## Acceptance Criteria

- [ ] Enter/Move/Leave 与窗口共同悬停状态无闪烁、无残留。
- [ ] 多屏和边缘定位保持可见，关闭后滚动和冻结状态复位。
- [ ] 12 格状态与 TUI 一致，打开面板不产生上游探测。
- [ ] Linux CI 与 macOS ARM dev-build 均通过。

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
