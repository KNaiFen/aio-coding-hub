# 实施清单

1. 建立任务记录，确认 main、origin/main、活动 worktree 和 PENDING 基线；提交规划材料并从完整 SHA 派生任务 worktree。
2. 新增集中式 task coordination writer/validator，扩展 `task.py` 的 `status/doctor/delegate/handoff/block/resume`。
3. 修复 session runtime stale pointer；补充 task state、routing、start/archive 兼容测试。
4. 用 skill-creator 初始化并实现三个 repo skills，精确放行 `.agents/skills/**`。
5. 精简根 `AGENTS.md`，只保留角色权限、硬边界、三 skill 路由和 Trellis managed block。
6. 将多 worktree 规范拆为主入口与 planning/execution/acceptance/cleanup 四专题，精简 execution/delivery/findings 模板。
7. 精简 `.trellis/workflow.md`，保留真实 CLI、breadcrumb 和阶段导航，删除不存在能力、死 completed block 和重复教程。
8. 同步 `docs/README.md`、`.trellis/tasks/README.md`、任务记录与相关导航；检查体量和 Markdown 链接。
9. 运行 Python 单元测试、skill quick validation、允许的 Node source contracts、变更 Node 文件语法检查和 `git diff --check`；派独立只读验收，修复发现后交付。

## 回滚点

- CLI 与文档拆分分别保持独立提交，便于单独回退。
- 不迁移归档历史；新增 `coordination` 可被旧 reader 忽略。
- skill 路由失效时仍可通过 `execution.md` 和显式 `task.py` 命令工作。
