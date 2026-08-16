# 多 Worktree 任务交付

本页是独立执行 session 的唯一流程入口。它只负责解释事实源、文件角色和阶段路由；进入某个阶段时，只读对应的一份专题，不要预加载整套流程。

简单、低风险且由 main 连续完成的任务使用[任务方案与实施结果留痕规范](./task-documentation-records.md)。独立执行、并行、长流程或高风险任务使用本流程和同级 worktree。

## 事实源

| 事实 | 权威来源 | 写入方式 |
|---|---|---|
| 活动阶段、唯一写者、worktree、分支、base、规划提交、阻塞 | `task.json` | `task.py status/doctor/delegate/deliver/block/resume` |
| 用户决定、范围、AC | `prd.md` | main 规划 |
| 技术设计与实施顺序 | `design.md`、`implement.md` | main 规划；复杂任务才需要 |
| 执行入口和任务特有边界 | `execution.md` | main 交接 |
| 实际实现、AC 证据、偏移、风险 | `delivery.md` | 执行 session |
| 当前整改问题 | `findings.md` | main；仅验收不通过时创建 |
| 实时 PR head、base、检查和合并状态 | GitHub PR / Checks | GitHub |
| 最终验收、merge、归档和清理证据 | `acceptance.md` | main；合并或终态后汇总 |

不要在 Markdown 中维护 `task.json` 已有的活动状态，也不要在候选分支中手写“当前 head SHA”。提交该文本本身会产生新 head。main 在活动验收时以 GitHub 上绑定具体提交的 review/comment 留证，终态再写入 `acceptance.md`。

## 文件最小集

- 轻量委派任务：`task.json`、`prd.md`、`execution.md`；交付前补 `delivery.md`。
- 复杂任务：再加 `design.md`、`implement.md`。
- 验收不通过：main 创建 `findings.md`。
- 合并、失败或放弃后的持久记录：main 创建 `acceptance.md`。

模板：

- [施工入口](./templates/execution.md)
- [交付报告](./templates/delivery.md)
- [验收整改](./templates/findings.md)
- [验收与收尾](./templates/acceptance.md)

## 生命周期

```text
planning -> ready -> implementing -> delivered -> completed
                          |              |
                          +-> blocked <-+
                          ^              |
                          +--- rework ---+
```

- `planning`：main 明确需求与方案。
- `ready`：worktree 和任务已登记，尚未启动执行。
- `implementing`：执行 session 是唯一写者，包含首次施工和返工。
- `delivered`：执行 session 已提交并暂停，`$gkd-accept` 或 main 按冻结的 GitHub head 验收。
- `blocked`：写权交给阻塞负责人，恢复条件已持久化。
- `completed`：main 已决定终态并执行归档；它不等于功能成功。

状态以 `python3 .trellis/scripts/task.py status <task>` 为准。`task.py validate` 只校验已有 JSONL 上下文文件，不证明交付、CI、Markdown 或归档资格。

## 按阶段读取

- main 规划、建 worktree、登记与交接：[规划与交接](./multi-worktree/planning-and-handoff.md)
- 执行 session 开工、施工、阻塞与交付：[执行与交付](./multi-worktree/execution-and-delivery.md)
- `$gkd-accept`/main 验收、同步合并、findings 和返工：[验收与返工](./multi-worktree/acceptance-and-rework.md)
- main 终态记录、归档与清理：[合并、归档与清理](./multi-worktree/merge-archive-cleanup.md)

## 不记录

不粘贴完整聊天、diff 或无解释日志；不逐行复述代码；不保留被新结论取代却仍伪装成当前指令的旧意见；不记录密钥、真实凭据或未脱敏用户数据；不为填模板编造 SHA、命令、CI 或人工验证。
