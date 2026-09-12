# 项目规则统一采用 GKD：远端交付归档

- 任务标识：project-rules-gkd；revision：r2-delivery。
- 归档观察：2026-09-08，Asia/Shanghai。
- 整理作者：GKD 收尾角色；方案、授权与最终审查决定仍归 main。
- 来源提交：a04f49597793d4f421e466208bcb9310da82c78e；目标基线：42b64aa646e19ce6b1f3bf6a08219043ea960dd4。

## 已有事实与本轮范围

r1 已完成 19 个已有文件的规则精简，并保存独立验收、main 审查和六项轻量检查的事实。实施不在本轮改变，也不重跑本地实现检查。PR 将仅带原 24 个任务路径：19 个实施文件和 5 个 r1 归档文件；本 revision 仅补充这三份交付记录。

本轮已获准推送任务分支、创建面向 main 的 PR、等待自动 ci-gate、pr-title 及分类选中的检查，在成功后以实际 PR head 绑定 squash 合并。后续会 fetch origin/main，并保留本地 main 的独有历史与其他未跟踪归档；本地同步仅尝试普通合并。远端成果和本归档实际进入 main 且来源没有新增待保留内容后，才清理本任务工作树及本地、远端任务分支。

## 观察边界

本摘要写入时 PR、自动 CI、合并、本地同步和清理均未执行。其真实终态由本轮收尾返回记录，不预填成功，也不为回填提交或 CI SHA 再创建递归提交。归档没有记录本机绝对路径、账号、凭据、会话句柄或完整日志。

## 最终续办与合并

- 观察：2026-09-08；作者：main。用户明确授权继续合并并处理问题，续办计划与审查保存在本目录 plan-recovery.md、review-recovery.md；它们记录当时决定，以下是最终事实。
- 首次交付的文档合同问题经单句修复，完整 PR CI 已通过；HTTP 502 后规则成果实际进入 b3d1b77c，但 GitHub PR 状态未完成。具体事实见 correction.md、recovery.md。
- 任务分支普通合并已落地 main，无内容变化；新增 recovery.md 后的最终 head 为 caf8accbe889aef2b5278dca1adc6c4d973d1c3e，相对 main 仅一份交付记录。自动 ci-gate 与 pr-title 均成功。
- gh pr merge --squash --match-head-commit 绑定该 head，exit 0。PR #194 于 2026-09-08T08:19:00Z 正式 MERGED，merge SHA 为 692da663ce0eaac2569e2c74eff78ee0c2317118；最终 head 与该提交整树比较相同。
- 六份阻挡本地同步的模型定价归档已逐一保全：五份与远端 blob 相同，summary 原有 36 行本地追加事实已原样还原。普通合并远端 main 两次均成功，既有本地 main 历史完整保留，没有 reset/rebase/stash 或代码冲突。
- 本地 main 已包含上述最终 merge SHA。相对 origin/main 的既有文件差异仅根级 progress.md、review.md 的旧发布完成记录；模型定价 summary 追加事实与其他供应商任务归档保持本地未提交状态，不夹带进本任务 PR。
- 删除前任务工作树无修改、未跟踪或 ignored 文件，任务成果与归档均存在于实际合并提交。普通 git worktree remove 成功，按真实 squash 内容及 PR 合并证据删除本地 chore/gkd-project-rules 分支；远端同名分支已由 GitHub 自动删除。
- 主工作树本任务 plan/review 已迁入本目录；原 worktrees 空目录在首轮已移除。其他分支、工作树和记录未清理。最终观察记录保留本地，不为回填自身提交或 CI 递归新增 PR。
