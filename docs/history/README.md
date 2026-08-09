# 历史资料索引

本目录保存已经完成、被替代或只对特定日期有效的项目资料。它们用于追溯决策和证据，不是当前实现说明；现行入口见 [项目知识库](../README.md)。

历史正文中的源码行号、分支名、命令和路径按当时状态保留，可能已经失效。需要判断当前行为时，以当前代码、机器合同、`AGENTS.md`、`.trellis/spec/` 和现行文档为准。

## 审计

- [代码库健康度与工程质量审计](./audits/codebase-health-audit-2026-08-09.md)：2026-08-09 最终收口的完整健康审计。
- [上游合并价值与冲突审计](./audits/upstream-integration-audit-2026-08-03.md)：2026-08-03 的上游候选比较。
- [插件体系架构审计](./audits/plugin-system-audit-2026-07-02.md)：2026-07-02 的插件架构快照。

## 被替代计划

- [社区插件系统开发计划](./plans/plugin-system-development-plan.md)：早期多运行时方案；现行插件方向是 Extension Host-only。

## 工程分析

- [Request Logs 实时卡片跨 CLI 分栏分析](./engineering-notes/realtime-trace-card-cli-tab-leak-analysis.md)：2026-07-04 的问题定位记录。
- [Upstream main reconciliation](./engineering-notes/upstream-main-reconciliation-2026-07-05.md)：2026-07-05 的上游同步与发布记录。

## 维护边界

- 保留当时结论，不用当前事实覆盖历史上下文。
- 顶部状态说明必须包含资料类型、证据日期和现行入口。
- 可以修复导航链接或补充“后来由什么替代”，但不要悄悄改写旧证据。
- 新的现行规范放回对应主题目录；新的完成任务进入 [Trellis 归档](../../.trellis/tasks/archive/)。
