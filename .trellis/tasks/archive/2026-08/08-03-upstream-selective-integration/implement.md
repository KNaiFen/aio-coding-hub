# 选择性上游集成执行计划

- [x] 固定 `origin/main`、FingerCaster `main` 和所有候选完整 SHA，确认 `upstream` fetch-only。
- [ ] 完成六个子任务的 PRD/设计、上下文和独立实现验证。
- [x] 集成并独立复审 UPA-002、UPA-003、UPA-005 和 UPA-009。
- [ ] 在原生 CI 门禁通过后集成重做的 UPA-008。
- [x] UPA-004 因真实隔离账号门禁尚未完成而保持候选分支，不进入本次发布。
- [ ] 每次吸收后检查冲突、无关文件、来源说明和已有 fork 行为。
- [ ] 运行允许的完整前端验证；触发 GitHub Actions native/check/dev-build 验证。
- [ ] 所有已验证项合入精确 main SHA 后发布 `0.60.47`。
- [x] 保留远端已完成的 `AIO-PENDING-015`、任务归档和 `0.60.46` 发布，不重复实施。
