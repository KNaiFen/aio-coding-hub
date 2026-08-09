# Trellis 任务索引

`.trellis/tasks/` 保存正式任务的需求、设计、实施计划、研究和验证上下文。活动目录只表示尚未完成或尚未归档；已交付任务必须迁入按月份组织的 `archive/`。

## 当前活动任务

- [`08-03-upstream-claude-oauth`](./08-03-upstream-claude-oauth/)：独立候选，仍需使用真实隔离账号验证完整 OAuth 登录、exchange、refresh 和 401 refresh 流程。

## 归档

- [按月份浏览已归档任务](./archive/)
- 归档任务保留原始 PRD、设计、计划、研究、实施与检查记录，作为交付证据，不作为当前行为规范。

## 生命周期

1. 正式工作在活动目录中创建任务，并以 `task.json` 表达状态、父子关系和交付元数据。
2. 独立 worktree 执行任务按 [多 Worktree 任务交付规范](../../docs/operations/multi-worktree-delivery.md) 维护 `execution.md` 和 `delivery.md`；验收不通过时由 main 维护 `findings.md`。这些 Markdown 记录不增加额外 JSON 门禁。
3. 实施过程中同步维护 `implement.md`，并让 `implement.jsonl`、`check.jsonl` 只引用存在的仓库文件。
4. 任务完成且交付证据明确后，运行 `python3 ./.trellis/scripts/task.py archive --no-commit <task>`；不要手工移动或复制目录。
5. 归档后运行 `python3 ./.trellis/scripts/task.py validate --all`，确认活动和归档 manifest 全部有效。
6. 若任务产生长期有效的产品、架构或运维知识，应同步更新 [项目知识库](../../docs/README.md) 中的现行文档；不要要求读者从任务记录推断当前行为。

其他 worktree 中的活动任务副本不是 `main` 的事实源。只有经过交付证据核对并进入 `main` 的任务状态才可在这里归档或更新。
