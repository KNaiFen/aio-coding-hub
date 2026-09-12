> 状态：历史发布进度，正文中的“进行中”仅反映当时记录，不指导新任务。现行流程使用 `$gkd-main`；delegated 进度位于执行 worktree 的 `.gkd/progress.md`，项目入口见 [AGENTS.md](../../../../AGENTS.md)。

# 进度

## 已完成

- PR #188 已以 squash merge 进入 `main`，实际 merge SHA 为 `e7c01882`。
- 八个版本源已同步为 `0.60.58`，并新增对应 changelog 条目。
- 本地版本一致性、release source/promotion 自测与 `git diff --check` 已通过。
- PR #189 已以 squash merge 进入 `main`，实际 merge SHA 为 `ca17a2d0312ed5ed83cda1931d5396834b4a284c`。
- 该 merge SHA 的 `ci`、CodeQL 与 `ci-gate` 均成功；候选 `release-candidate-ca17a2d0312ed5ed83cda1931d5396834b4a284c-33923149369-1` 未过期且由 release workflow 复用。
- annotated tag `aio-coding-hub-v0.60.58` 已指向该 merge SHA；release run `33926317174` 成功，正式 Release 已发布 12 个资产。

## 收尾状态

- 版本发布目标已完成；完整构建、测试、签名、候选制品与正式发布均由 GitHub Actions 验证。
- 没有未解决的发布阻塞；后续工作不属于本次 0.60.58 任务范围。
