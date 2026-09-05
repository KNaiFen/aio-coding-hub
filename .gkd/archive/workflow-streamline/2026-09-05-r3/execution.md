# 执行交接 r2

本记录为已完成的 r2 实现快照；后续交付授权以同目录 plan.md 的 r3 为准。

- PLAN：主工作树 `.gkd/plan.md` r2，用户已批准四项方案并要求开始执行。
- Route：direct-main；writer：main；worktree：`../workflow-streamline`；branch：`ci/streamline-gkd-workflow`；base：`ca17a2d0312ed5ed83cda1931d5396834b4a284c`。
- 本任务校准现行 GKD 交接和检查器、纯文档 PR/push 分类、提交/发版操作说明、upstream 同步警告终态。
- 适用规则：AGENTS、本会话批准范围。当前根规则的旧 plan 交接与禁用现行 GKD 角色条款是本次明确批准修正的对象。
- 修改范围：AGENTS、双语 README、既有 GitHub Actions 治理文档、CI scope JSON/分类器/selftest、cloud-only checker/selftest、sync workflow/policy/selftest、CI scope/cloud-only 两份 active spec、本任务 `.gkd/` Markdown。
- AC：现行 handoff 能通过合同；旧命令仍失败；GKD Markdown 和根级旧任务文档的 PR/push 不触发重型 CI，AGENTS 仍跑 docs contracts；代码混合/未知路径/手动 CI 保持完整门槛；DIRTY/UNKNOWN 成功警告且区分语义，空状态/外部错误仍失败；候选晋升/签名/check 名不变。
- 验证：Git diff/check、引用扫描、变更 Node 语法；ci-change-scope selftest；cloud-only、sync-upstream-policy、ci-quality-gates 各 checker/selftest；github-actions-pin-policy、spec-links、release-promotion selftest。仅 Node 内置模块和不写文件的 Bash 替身，禁止依赖安装、产品测试/构建、Cargo/Tauri 和真实 GitHub 写调用。
- 更新 progress，主代理负责最终审查与中文提交；推送/PR 按用户答复，合并/tag/release 不在本轮范围。
- 不改旧根级 plan/progress/review、历史归档、本地 main 独有提交或产品文件；不复制生命周期实现。
