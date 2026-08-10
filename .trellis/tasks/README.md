# Trellis 任务索引

`.trellis/tasks/` 保存正式任务的需求、设计、实施计划、研究和验证上下文。活动目录只表示尚未完成或尚未归档；已交付任务必须迁入按月份组织的 `archive/`。

## 当前活动任务

本表是 main 的协调索引，不复制计划或交付正文。活动期的详细事实源是登记任务目录；委派任务的该目录位于登记 worktree 内。main 依据本表定位材料、确认当前唯一写者并安排合并顺序。规划提交、PR、阶段或写者变化时由 main 更新本表。

委派任务的初始行可随任务分支的规划提交或短期协调 PR 持久化；它不阻塞已完成规划的执行 session。功能合并后，main 将最终索引与归档收尾一并同步。

| 任务目录 | 阶段 | 分支 | base SHA | 规划提交 | Worktree | PR | 当前唯一写者 | 依赖/说明 |
|---|---|---|---|---|---|---|---|---|
| [`08-03-upstream-claude-oauth`](./08-03-upstream-claude-oauth/) | planning（暂停，未委派） | 无（旧分支已删除） | 不适用 | 未登记 | 未创建 | 未创建 | main session | 用户已决定暂不处理；不影响当前工作。恢复前需重新确认方案，并完成真实隔离账号验证。 |

## 归档

- [按月份浏览已归档任务](./archive/)
- 归档任务保留原始 PRD、设计、计划、研究、实施与检查记录，作为交付证据，不作为当前行为规范。

## 生命周期

1. 采用 Trellis 的正式工作在活动目录中创建任务，并以 `task.json` 表达既有生命周期、父子关系和元数据。明确跳过 Trellis 的简单 main 任务改用 [月度 Markdown 变更记录](../../docs/operations/task-documentation-records.md)，不在这里创建空任务包。
2. main 在移交独立 worktree 前记录完整 base SHA、规划提交、任务分支和唯一写者；执行 session 首次开工核对这些事实。任务创建默认把 PR 目标设为 `main`；只有有意使用其他目标时才显式传入 `--base-branch`。
3. 独立 worktree 执行任务按 [多 Worktree 任务交付规范](../../docs/operations/multi-worktree-delivery.md) 维护 `execution.md` 和 `delivery.md`；验收不通过时由 main 维护 `findings.md`。`delivery.md` 还保留 main 的验收和收尾结论。这些 Markdown 记录不增加额外 JSON 门禁。
4. 复杂任务在实施过程中由 main 同步维护 `implement.md`；轻量任务可以没有该文件。存在 `implement.jsonl`、`check.jsonl` 时，只允许它们引用实际存在的仓库文件。
5. 有功能 PR 时，main 在其合并后记录最终结果和 merge commit；没有功能 PR 的失败、放弃或部分完成则通过只包含记录的收尾 PR 保存真实终态。阻塞任务保持活动，不归档；不得虚构 PR、head 或 merge commit。
6. 允许归档的终态在收尾记录已持久化后，使用 `python3 ./.trellis/scripts/task.py archive --no-commit <task>`；不要手工移动或复制目录。`task.json.status=completed` 仅表示归档，不代替最终结果。
7. 归档命令成功后运行 `python3 ./.trellis/scripts/task.py validate --all`，确认活动和归档 manifest 全部有效；归档修改仍须经归档收尾 PR 合并。
8. 若任务产生长期有效的产品、架构或运维知识，应同步更新 [项目知识库](../../docs/README.md) 中的现行文档；不要要求读者从任务记录推断当前行为。部分完成项保留或拆分到 PENDING，阻塞项不得伪装成已完成。

其他 worktree 中的活动任务副本在活动期是详细事实源，但不应被复制回 main 形成第二份计划。功能或只含记录的终态 PR 合并后，main 中同一任务目录成为可归档的事实基础；归档后的副本是历史证据，不覆盖现行规范。
