# 项目知识库

这里是 AIO Coding Hub 长期项目知识的统一入口。根目录 README 面向用户，`AGENTS.md` 规定执行边界；本页负责回答“某类信息应该去哪里找、哪一份资料才是现行事实”。

## 事实源优先级

发生冲突时，按以下顺序判断：

1. 当前代码、类型、机器可读合同与自动化校验。
2. `AGENTS.md` 和 `.trellis/spec/` 中适用于当前目录的强制规则。
3. 本页列出的现行产品、架构、插件与运维文档。
4. `PENDING.md` 和 `.trellis/tasks/` 中尚未交付的任务决策。
5. `docs/history/`、`PENDING_COMPLETED.md` 与 Trellis 任务归档中的历史证据。
6. `.trellis/workspace/` 中的会话日志。

历史审计、旧计划、任务正文和会话日志用于解释当时发生了什么，不得覆盖当前实现和现行规范。

## 用户与产品

- [中文项目概览](../README.md)
- [English project overview](../README_EN.md)
- [产品定位与设计原则](./product/overview.md)
- [发布记录](../CHANGELOG.md)

## 架构与插件

- [插件系统 RFC](./plugin-system-rfc.md)
- [Plugin Manifest v1 规范](./plugin-manifest-v1.md)
- [插件开发手册](./plugins/README.md)
- [插件架构说明](./plugins/architecture/README.md)
- [插件 API 参考](./plugins/reference/README.md)
- [插件运行时说明](./plugins/runtime/README.md)
- [机器可读 Plugin API v1 合同](./plugins/plugin-api-v1-contract.json)

## 运维与贡献

- [Homebrew 发布指南](./release-homebrew.md)
- [GitHub Actions 治理与远端配置](./operations/github-actions-governance.md)
- [GKD 项目 adapter](./operations/gkd-adapter.md)
- [任务方案与实施结果留痕规范](./operations/task-documentation-records.md)
- [多 Worktree 任务交付入口](./operations/multi-worktree-delivery.md)：按 planning、execution、acceptance、cleanup 阶段加载专题。
- [仓库执行规则](../AGENTS.md)
- [Trellis 工作流](../.trellis/workflow.md)
- [Trellis 规范目录](../.trellis/spec/)

本仓库执行零产物本地策略。本地允许的验证范围以 `AGENTS.md` 为准；依赖安装、前端完整质量门、Rust、生成绑定和构建由 GitHub Actions 负责。

## 任务与决策

- [待处理事项](../PENDING.md)：用户明确要求累积、尚未完成的小问题。
- [已完成事项](../PENDING_COMPLETED.md)：已交付或明确放弃的 PENDING 历史。
- [Trellis 任务索引](../.trellis/tasks/README.md)：正式任务、实施计划、研究与验证证据。
- [Trellis 会话记录](../.trellis/workspace/index.md)：按开发者保存的会话追溯材料，不是现行事实源。

## 历史资料

- [历史资料索引](./history/README.md)：带日期的审计、被替代计划和工程分析。
- [Trellis 已归档任务](../.trellis/tasks/archive/)：完成任务的 PRD、设计、实施和检查上下文。

## 文档状态

- **现行**：必须随代码与公共行为同步更新；默认不需要状态横幅。
- **历史终版**：记录某个日期的完整审计或交付证据；正文保持当时语义，顶部必须说明证据日期和当前入口。
- **已被替代**：计划或说明已不再指导实现；顶部必须链接替代它的现行文档。
- **会话记录**：只用于追溯过程，不应被索引为产品或架构规范。

## 维护规则

1. 新增长期文档时放入稳定分类目录，并在本页或对应子索引中添加入口；不要恢复根目录散落文档。
2. 实现、公共 API、发布流程或验证边界变化时，同一变更内更新相关现行文档和机器合同。
3. Trellis 活动状态由 `task.py` 写入 `task.json`；main 在终态 `acceptance.md` 记录验收版本、结果、merge、知识库/PENDING 和清理事实。有功能 PR 时在其合并后收尾；无功能 PR 的失败、放弃或部分完成通过 records-only PR 收尾；阻塞任务保持活动。
4. 历史文件只修正状态说明、有效入口或明确的链接损坏，不把旧结论改写成今天的结论。
5. 使用相对链接；提交前通过 `$gkd-local-verify` 的固定 runner 运行仓库允许的合同、Node 语法和 diff 检查，并按需运行适用的 Trellis 检查。没有覆盖某类链接时如实说明，不把 `validate --all` 当 Markdown 校验。
6. `.local/` 外部参考 checkout、`.playwright-cli/`、`.impeccable/`、`.trellis/.runtime/`、`.codegraph/` 等本地产物不进入知识库。
