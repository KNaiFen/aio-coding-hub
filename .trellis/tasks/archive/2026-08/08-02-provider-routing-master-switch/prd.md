# 供应商全局路由总开关

## Goal

实现 P001

## Requirements

- 全局供应商开关必须覆盖默认/自定义路由、Session 复用和所有资格投影。
- 关闭不取消已经取得发送许可的当前尝试，但下一次重试、切换和新请求不得使用该供应商。
- 路由编辑器保留成员开关值，全局关闭时仅禁用控件。

## Acceptance Criteria

- [ ] 自定义路由 SQL 不再绕过全局开关。
- [ ] 关闭竞态满足“当前尝试完成、后续尝试排除”。
- [ ] 首页、Observer、TUI 和路由编辑器语义一致。

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
