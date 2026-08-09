# 选择性集成 FingerCaster 上游修复与趋势

## Goal

以固定上游 SHA 选择性移植 UPA-002/003/004/005/009，并基于 usage ledger 重做有界 UPA-008；不整支合并上游。

## Requirements

- 仅从 `FingerCaster/aio-coding-hub` 的完整提交 SHA 选择性移植 UPA-002、UPA-003、UPA-004、UPA-005、UPA-009，并参考 UPA-008 重做 ledger 版本；禁止整支 merge、裸 tag 与 `--tags`。
- 每个风险域使用独立子任务、提交和验证边界；任一条件项失败不得污染或阻塞其他已验证项。
- 保留 fork 的 Provider 扩展、错误脱敏、usage ledger、发布矩阵和现有设置语义。
- UPA-004 只有隔离 Claude Pro/Max 账号完成登录和刷新验证后才允许合并。
- UPA-008 必须使用 `usage_events/usage_ledger`，同时为现有缓存率趋势和新性能趋势建立 Top 10、120 时间桶和 1200 行的共同硬预算。
- 不纳入 UPA-006、UPA-007、UPA-010、上游版本、锁文件、Release、CI 和任务文档。
- `AIO-PENDING-015` 已由并行远端工作在 `origin/main` 完成并随 `0.60.46` 发布；本批必须保留该结果，不再重复实施。本批次目标补丁版本顺延为 `0.60.47`。

## Acceptance Criteria

- [ ] 所有纳入项均可从提交说明追溯到精确上游 SHA，且没有整支上游合并或无关文件漂移。
- [ ] 安全、Provider、OAuth、趋势和 About 子任务可独立验证、合并与回滚。
- [ ] 本地前端验证通过，所有 Rust、Specta、native 和桌面构建验证由 GitHub Actions 通过。
- [ ] 条件 UPA-004 未验证时保持未合并状态，其余子任务仍可完成集成发布。
- [x] `AIO-PENDING-015` 的远端实现、任务归档和 `0.60.46` 发布在重放时完整保留。

## Notes

- 审计报告基线已经过时；实施始终以当前 `origin/main` 和重新解析的上游 SHA 为准。
- 实施期间 `origin/main` 前进到 `dc311482c2af00177544c1b526dd173a2b7f20c9`；集成分支已无冲突重放并保留 Tray 与版本历史。
- UPA-006 的文本冲突比报告描述更小，但联合 settings 更新和 `aio`/`OpenAI` 身份往返仍存在语义缺口，因此继续排除。
