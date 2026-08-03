# 选择性上游集成执行计划

- [ ] 固定 `origin/main`、FingerCaster `main` 和所有候选完整 SHA，确认 `upstream` fetch-only。
- [ ] 完成六个子任务的 PRD/设计、上下文和独立实现验证。
- [ ] 依次集成 UPA-002、UPA-003、UPA-005、通过门禁的 UPA-004、重做 UPA-008、UPA-009。
- [ ] 每次吸收后检查冲突、无关文件、来源说明和已有 fork 行为。
- [ ] 运行允许的完整前端验证；触发 GitHub Actions native/check/dev-build 验证。
- [ ] UPA-004 使用隔离账号完成登录、回调、exchange、手动刷新和 401 刷新；失败则从发布范围移除。
- [ ] 所有已验证项合入精确 main SHA 后发布 `0.60.46`。
- [ ] 将 `AIO-PENDING-015` 重新基线化为后续独立 `0.60.47` 交付。
