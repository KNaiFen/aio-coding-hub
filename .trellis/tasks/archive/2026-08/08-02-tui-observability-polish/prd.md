# TUI 观测与显示改造

## Goal

实现 P002/P003/P004/P006/P007/P008/P009/P011 的 TUI 部分

## Requirements

- 完成 P002/P003/P004/P006/P007/P008/P009/P011 的全部 TUI 行为。
- 供应商卡动态显示限额、余额、OAuth 和最后一行可用性，不固定行数。
- 请求卡保持 5 行，供应商位于目录前，路由行追加输出速率。
- 全 TUI 使用同一语义调色板并确定性降级。

## Acceptance Criteria

- [ ] 顶栏不再出现页码和供应商数量，左右切换及帮助不变。
- [ ] `限`/`余`/OAuth/可用性按配置正确增减，窄终端无空行或孤立分隔符。
- [ ] 详情页按 `t` 异步测试且结果按供应商隔离。
- [ ] TrueColor、256、16、未知和无颜色模式均有稳定测试。

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
