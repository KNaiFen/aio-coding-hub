# 项目知识库

这里是 AIO Coding Hub 长期项目知识的统一入口。根目录 README 面向用户，`AGENTS.md` 规定执行边界；本页负责回答“某类信息应该去哪里找、哪一份资料才是现行事实”。

## 实现事实与执行约束

核对实现事实时，优先查看当前代码、类型、机器可读合同和自动化校验，再按需阅读本页列出的现行产品、架构、插件与运维文档。文档与代码不一致时记录差异，不能把当前实现自动视为获准行为。

执行约束先遵循系统、开发者和用户明确指令；在其允许的范围内，生命周期、路线、角色、授权、验收和收尾以 `$gkd-main` 为准，[AGENTS.md](../AGENTS.md) 补充 AIO 环境与 Git 边界。`.trellis/spec/` 提供受影响行为的项目合同，不另设生命周期；项目规则与 GKD 冲突时修正项目。

按任务涉及的模块、协议和行为读取相关索引与合同。只有共享配置、协议、常量、重命名或影响面不明时才扩大检索，不要求每个任务通读全部资料。

`PENDING.md` 保存尚未交付的事项；历史审计、旧计划、归档、任务正文和会话日志只解释当时发生了什么，不指导新任务，也不提供新的执行授权。

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
- [仓库执行规则](../AGENTS.md)
- [项目规范目录](../.trellis/spec/)

本仓库执行零产物本地策略。提交前按获批方案执行 `AGENTS.md` 允许的零依赖检查；合并前等待自动 CI 按改动分类选中的 job。依赖安装、前端质量门、Rust、生成绑定和构建由 GitHub Actions 负责，具体边界见 [云端验证合同](../.trellis/spec/aio-coding-hub/cross-layer/cloud-only-verification-contract.md)。

## 任务与决策

- [待处理事项](../PENDING.md)：用户明确要求累积、尚未完成的小问题。
- [已完成事项](../PENDING_COMPLETED.md)：已交付或明确放弃的 PENDING 历史。
- main 按 GKD 维护 `.gkd/plan.md` 与 `.gkd/review.md`；仅 delegated 生成执行 worktree 的 `.gkd/execution.md` 和 `.gkd/progress.md`，归档按 GKD 收尾规则处理。

## 历史资料

- [历史资料索引](./history/README.md)：带日期的审计、被替代计划和工程分析。

## 文档状态

- **现行**：必须随代码与公共行为同步更新；默认不需要状态横幅。
- **历史终版**：记录某个日期的完整审计或交付证据；正文保持当时语义，顶部必须说明证据日期和当前入口。
- **已被替代**：计划或说明已不再指导实现；顶部必须链接替代它的现行文档。
- **会话记录**：只用于追溯过程，不应被索引为产品或架构规范。

## 维护规则

1. 新增长期文档时放入稳定分类目录，并在本页或对应子索引中添加入口；不要恢复根目录散落文档。
2. 实现、公共 API、发布流程或验证边界变化时，同一变更内更新相关现行文档和机器合同。
3. 任务目录只保存正式任务、计划、交付与验收证据。
4. 历史文件只修正状态说明、有效入口或明确的链接损坏，不把旧结论改写成今天的结论。
5. 使用相对链接；提交前按 `AGENTS.md` 规定的验证范围执行检查。
6. `.local/` 外部参考 checkout、`.playwright-cli/`、`.impeccable/`、`.trellis/.runtime/`、`.codegraph/` 等本地产物不进入知识库。
