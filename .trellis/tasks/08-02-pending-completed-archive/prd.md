# 归档已完成 PENDING 历史

## Goal

让每次实现前读取的 `PENDING.md` 只包含未完成工作，避免已经交付的长条目持续占用上下文；同时把稳定 ID、决策、验收标准和发布证据完整保存在独立归档中。

## Requirements

- 新建 `PENDING_COMPLETED.md`，接收当前 `AIO-PENDING-001` 至 `AIO-PENDING-011` 的完整条目、完成证据和历史说明。
- `PENDING.md` 保留活跃列表用途、状态规则、下一个稳定 ID 和归档链接；当前没有未解决条目时明确写“暂无”。
- 不删除、压缩或改写已完成条目的锁定决策和验收证据。
- 更新 `AGENTS.md`：正式计划和实施前只需读取活跃 `PENDING.md`；条目完成并记录合并/发布证据后迁入完成归档。
- `PENDING.md` 继续是未解决事项的唯一活跃入口，`PENDING_COMPLETED.md` 不作为每次规划的必读上下文。

## Acceptance Criteria

- [ ] `PENDING.md` 不再包含 11 个已完成条目的正文，文件保持简短。
- [ ] `PENDING_COMPLETED.md` 完整保留 `AIO-PENDING-001` 至 `AIO-PENDING-011` 及其证据。
- [ ] 两个文件互相链接，且新条目明确从 `AIO-PENDING-012` 开始。
- [ ] `AGENTS.md` 的规则同时满足“减少活跃上下文”和“保留历史”两项目标。
- [ ] 未跟踪的用户文件没有被纳入迁移或提交。

## Notes

- 本任务只调整仓库工作文档，不改变产品运行时行为。
