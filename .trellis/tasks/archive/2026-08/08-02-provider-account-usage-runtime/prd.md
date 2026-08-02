# 共享账户余额运行时

## Goal

实现 P004/P005 的共享缓存与按需刷新

## Requirements

- 后端成为余额结果与定时刷新唯一所有者，桌面和 TUI 共用无敏感字段缓存。
- 有活跃消费者时按保存配置刷新，供应商关闭仍可查询；无消费者时停止定时远端请求。
- 成功缓存 60 分钟硬过期，加载、失败或过期均不得显示旧金额。

## Acceptance Criteria

- [ ] 桌面或 TUI 单独活跃都能刷新，全部退出后租约到期停止。
- [ ] 每供应商最多一个在途查询，手动与定时结果不会乱序覆盖。
- [ ] TUI 可读取套餐优先、现金回退的一位小数余额摘要。

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
