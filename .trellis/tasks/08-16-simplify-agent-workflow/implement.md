# 实施清单

- [x] 建立任务记录，确认 main、origin/main、活动 worktree 和 PENDING 基线；提交规划材料并从完整 SHA 派生任务 worktree。
- [x] 新增集中式 task coordination writer/validator，扩展 `task.py` 的 `status/doctor/delegate/handoff/deliver/block/resume`。
- [x] 修复 session runtime stale pointer；补充 task state、routing、start/archive 兼容测试。
- [x] 用 skill-creator 初始化并实现三个 `gkd-` 用户级 skills，安装到 `~/.codex/skills/`；仓库只固定角色名，不保留重复 skill 本体。
- [x] 精简根 `AGENTS.md`，只保留角色权限、硬边界、三 skill 路由和 Trellis managed block。
- [x] 将多 worktree 规范拆为主入口与 planning/execution/acceptance/cleanup 四专题，精简 execution/delivery/findings 模板并新增 main-only acceptance 模板。
- [x] 精简 `.trellis/workflow.md`，保留真实 CLI、breadcrumb 和阶段导航，删除不存在能力、死 completed block 和重复教程。
- [x] 同步 `docs/README.md`、`.trellis/tasks/README.md`、任务记录与相关导航；扩展现有 Markdown 链接合同。
- [x] 增加只从可信 main checkout 运行的固定 head 验收合并 CLI，更新 `$gkd-accept` 与角色权限，补充 Git-tree/ruleset/fail-closed/幂等单测。
- [ ] 完成最终验证：本地允许的 Node contracts、Node 语法、文档链接和全分支 `git diff --check`；Python unittest 和完整 CI 矩阵由 PR 最终 head 的 GitHub Actions 运行。

## 回滚点

- CLI 与文档拆分分别保持独立提交，便于单独回退。
- 不迁移归档历史；新增 `coordination` 可被旧 reader 忽略。
- skill 路由失效时仍可通过 `execution.md` 和显式 `task.py` 命令工作。
