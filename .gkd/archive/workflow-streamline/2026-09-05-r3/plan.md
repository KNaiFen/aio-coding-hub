# 批准方案 r3

本文件摘录主工作树 PLAN 的目标、范围、成功标准和最终交付授权；r1 为已被取代的只读调查稿。

- r2 授权：用户要求“开始执行 PLAN”。main 以 direct-main 在 workflow-streamline worktree 实现四项已说明方案。
- r3 授权：用户要求“推送，合并，清理分支”。main 推送本任务分支、创建 PR，自动门禁通过后 squash 合并，归档、提交清理并删除已合并的本轮分支/worktree。
- 范围：AGENTS 与双语 README、Actions 治理文档、CI scope JSON/分类器/selftest、cloud-only checker/selftest、sync workflow/policy/selftest、CI scope/cloud-only active spec 和本任务 Markdown。
- AC：现行 GKD 交接可通过检查、旧入口仍拒绝；纯过程文档 PR/push 不启动重型 CI，AGENTS 仍验证合同；代码混合、未知路径与手动 CI 保留 full；DIRTY/UNKNOWN 成功警告且区分语义，空状态与真实命令错误仍失败；required check 和签名候选晋升不变。
- 验证：仅 Git 只读/diff、引用扫描、Node 语法与既有零依赖 checker/selftest；产品检查通过自动 PR CI。固定 PR 由 gkd-ci-monitor 以 `--interval 30 --timeout 3600` 只读跟踪。
- 非目标：产品功能、依赖、版本、候选晋升实现、GitHub 设置、真实发版、其他 PR/分支及历史归档。
- 本地 main 原有独有历史和旧发布记录保留；通过普通 Git 合并远端 main 同步 squash 结果，不重新合入原任务分支，不推送 main。
- 合并前 main 审查归档及 cleanup commit；合并后确认工作树干净再清理本轮现场。
