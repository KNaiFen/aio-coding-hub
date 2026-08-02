# 桌面供应商可用性

## Goal

实现 P009 的桌面卡片与设置

## Requirements

- 供应商卡内部底部显示 36 格短期可用性，不创建嵌套卡片。
- 全局设置提供 3/6/12 小时，默认 6 小时。
- 每格 Tooltip 显示自然起止时间及成功/失败计数，浅色和深色主题均可读。

## Acceptance Criteria

- [ ] 状态条与后端三态、TUI 汇总一致并每 15 秒批量更新。
- [ ] 空数据、错误与未来字段失败开放为灰色或隐藏。
- [ ] 桌面宽窄布局、主题和 Tooltip 通过组件及视觉检查。

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
