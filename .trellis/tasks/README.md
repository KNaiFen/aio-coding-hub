# Trellis 任务索引

`.trellis/tasks/` 保存正式任务；活动目录表示尚未归档，不等于仍在施工或已经交付。机器状态以各任务 `task.json` 为准，不在本页维护第二份状态表。

## 查找任务

```bash
python3 .trellis/scripts/task.py list
python3 .trellis/scripts/task.py list --mine
python3 .trellis/scripts/task.py status <task> --json
python3 .trellis/scripts/task.py doctor <task>
python3 .trellis/scripts/task.py list-archive [YYYY-MM]
```

无法可靠解析当前 session 时显式传任务路径；不要从一个全局指针或本页猜测 writer、worktree、分支和阶段。

## 文件与流程

- 用户决定、范围和 AC：`prd.md`。
- 复杂任务设计与步骤：`design.md`、`implement.md`。
- 独立执行入口和实际交付：`execution.md`、`delivery.md`。
- 验收不通过的当前整改：`findings.md`。
- main 终态验收与收尾：`acceptance.md`。
- 完整角色、阶段和命令：[多 Worktree 任务交付](../../docs/operations/multi-worktree-delivery.md)。

活动期的详细任务目录位于登记 worktree；不要把它复制回 main 形成第二份计划。功能或 records-only 终态 PR 合并后，main 的任务目录才是归档基础。

## 归档

- [按月份浏览归档](./archive/)
- 归档保留当时的 PRD、设计、实施和验收证据，只解释历史，不覆盖现行规范。

只有 main 在终态记录已持久化、内容归属明确且任务不再被写入时运行 `task.py archive --no-commit`。阻塞任务保持活动。archive 非事务性，`status=completed` 只表示目录已归档；`task.py validate --all` 只检查已有 JSONL，不证明交付、CI 或业务成功。
