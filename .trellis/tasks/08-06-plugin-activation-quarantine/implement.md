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
- 未在本地运行：Cargo、pnpm、Tauri、格式化、类型检查、Lint、测试、构建或生成器。
- 已推送：`codex/final-hardening-stack`；Actions [31155660000](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31155660000) 对精确 head `0ea611f9a0ac1b49a31ead32eb807b55005dd435` 执行全量 CI，`support-contract` 通过，`frontend` 因候选未改动的 `js-yaml` 新高危公告失败，`rust` 因生成器编译错误失败且没有修复 artifact。
- 用户授权后，Actions [31156864930](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31156864930) 已对精确最终 head `77f41f820e295189780781025a20630ab2f68482` 完成全量 CI：`change-scope` 与 `support-contract` 通过；`frontend` 仍受候选未改动的 `js-yaml` 高危公告 `GHSA-5p4m-2wfm-xmqj` 阻断；`rust` 在 bindings 导出阶段被 `origin/main@d32106c3706edc7535ea074d4c352c6b7e701dbf` 同样存在的 Provider IPC 注册失配阻断（`provider_copy_api_key_to_clipboard`、`base_url_ping_ms`），未进入 Clippy 或测试。该运行未重现此前候选相关的维护态、Gateway 参数和请求日志类型错误，但不能作为整批测试通过的证据。
- 按用户决定先跳过 CI 修复、PR 与合并；`AIO-PENDING-023` 继续为 `planned`，等待后续处理云端阻断并完成验证与合并。
