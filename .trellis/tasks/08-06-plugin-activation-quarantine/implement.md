# AUD-033 实施清单

- [x] 增加无效/精确/legacy activation event 的 failure-first 合同测试。
- [x] 实现内部 ActivationPolicy，并在 command、gateway snapshot 与 startup dispatcher 执行精确 gate。
- [x] 定义严重故障分类，扩展持久记录与 600 秒/3 次原子 quarantine 事务。
- [x] 将 command、startup 和 gateway 故障统一接入，并保持第三次请求 fail-open/fail-closed。
- [x] 隔离后刷新 gateway snapshot、释放 host，保护 in-flight snapshot 隔离。
- [x] 实现 quarantined-only revalidate 与废弃历史事件迁移为 disabled。
- [x] 同步 SDK、合同、文档、前端 lifecycle UI；bindings 留给 CI。
- [ ] 在 GitHub Actions 覆盖并发阈值、跨重启、恢复、legacy 与 snapshot 刷新。
- [x] 本地仅运行零依赖源码合同与 `git diff --check`。
- [ ] 合并后创建纯文档收口 PR，记录八项 PR/CI/提交证据。

## 候选证据（已提交、未合并）

- 2026-08-07：`codex/final-hardening-stack` 已以 `3195b3cb` 提交 AUD-033 源码、云端测试覆盖和跨层合同更新；`src/generated/bindings.ts` 保持由 CI 生成，未手工修改。
- 已通过：`node scripts/check-cloud-only-verification.selftest.mjs`、`node scripts/check-cloud-only-verification.mjs`、`git diff --check`。
- 未运行：Cargo、pnpm、Tauri、格式化、类型检查、Lint、测试、构建或生成器；尚未推送、创建 PR、触发 CI 或合并。`AIO-PENDING-023` 继续为 `planned`，等待候选提交后的云端验证与合并证据。
