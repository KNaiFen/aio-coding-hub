# 供应商可用性事实源

## Goal

实现 P008/P009 的数据库、聚合、设置与观察接口

## Requirements

- 只从真实上游尝试产生供应商级短期可用性事实，手动测试永不入账。
- 同一请求/供应商最多一条结果；成功率 75% 为绿边界，无样本为灰。
- 支持桌面 36 桶、TUI/Tray 12 桶、3/6/12 小时设置，默认 6 小时。
- Observer 增加认证、回环、有界的手动测试 POST，并复用现有探测领域函数。

## Acceptance Criteria

- [ ] SQLite v45 加法迁移、24 小时保留及每小时异步清理通过测试。
- [ ] A 失败切 B 成功分别记 A 失败、B 成功，重试不放大权重。
- [ ] 客户端取消、本地错误、插件拒绝、跳过和手动测试均不入账。

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
