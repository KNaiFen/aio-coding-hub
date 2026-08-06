# AUD-033 实施清单

- [ ] 增加无效/精确/legacy activation event 的 failure-first 合同测试。
- [ ] 实现内部 ActivationPolicy，并在 command、gateway snapshot 与 startup dispatcher 执行精确 gate。
- [ ] 定义严重故障分类，扩展持久记录与 600 秒/3 次原子 quarantine 事务。
- [ ] 将 command、startup 和 gateway 故障统一接入，并保持第三次请求 fail-open/fail-closed。
- [ ] 隔离后刷新 gateway snapshot、释放 host，保护 in-flight snapshot 隔离。
- [ ] 实现 quarantined-only revalidate 与废弃历史事件迁移为 disabled。
- [ ] 同步 SDK、合同、文档、前端 lifecycle UI；bindings 留给 CI。
- [ ] 云端覆盖并发阈值、跨重启、恢复、legacy 与 snapshot 刷新。
- [ ] 本地只运行零依赖源码合同、解析与 `git diff --check`。
- [ ] 合并后创建纯文档收口 PR，记录八项 PR/CI/提交证据。
