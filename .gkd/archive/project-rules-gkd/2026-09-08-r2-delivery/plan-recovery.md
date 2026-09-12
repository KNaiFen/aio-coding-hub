# 项目规则统一采用 GKD：合并交付

- 日期：2026-09-08；revision：r3-recovery；状态：用户明确要求继续合并并处理问题，正在续办。
- 路线：direct-main。既有 gkd_closeout 已完成归档、推送和 PR 创建，并在 CI 实现失败后停止；main 修复原范围文档，独立 CI 监控角色等待新 head，main 完成剩余获准 Git 交付，不重开收尾角色。
- 用户先要求清理无用的 gkd-project-rules/worktrees，现明确要求该分支未合并则合并，有冲突告知。此授权包含必要的任务分支推送、PR 创建、正常门禁后 squash 合并；交付完成后清理已无独有成果的本任务工作树和分支。

## 目标与当前证据

把 chore/gkd-project-rules 中已经实施、检查和审查通过的规则精简合入 origin/main。原任务归档位于 `.gkd/archive/project-rules-gkd/2026-09-07-r1/`，历史正文保持原观察时点，不改写当时的授权。

- source：a04f49597793d4f421e466208bcb9310da82c78e；原实施基线：a35f2aa4ea469f6e4066582b2e969f1ec44fca2e。
- 已 fetch 的 origin/main：42b64aa646e19ce6b1f3bf6a08219043ea960dd4；本地 main：193767510ef647193ce5f16390bc1f663c3dffb0。
- 来源工作树干净，未找到此 head 分支的远端分支或 PR；git cherry 仍列出 source 独有提交。
- git merge-tree --write-tree origin/main chore/gkd-project-rules 返回成功，无冲突；合并树为 3d7b3a1e29dbf0d6aaad3a63ac6e60c4fa2a52d8。
- PR 差异为 19 个已有文件加 5 个原任务归档文件，与已审查提交一致；git diff --check origin/main...chore/gkd-project-rules 成功。
- 本地 main 与 origin/main 有历史分叉和 124 个文件的实际差异，并有其他任务的未跟踪归档；不得 reset/rebase/stash 或删除未知内容。

## 范围、验证与终点

1. 复用 r1 的实施和独立验收结果：六项获批轻量检查通过，未重跑。实现 source 不变，新增交付记录只保存本轮必要事实。
2. 允许在来源工作树新增 .gkd/archive/project-rules-gkd/2026-09-08-r2-delivery/ 下本计划、审查和摘要的脱敏 Markdown，使用简短中文 Conventional Commit 提交；不创建空 execution/progress。
3. 推送 chore/gkd-project-rules，创建面向 main 的 PR。只等自动 ci-gate、pr-title 与现有分类选择的检查，不额外手动运行常规 CI。CI 等待按 gkd_closeout 的共用监控材料执行，总预算 6 小时。
4. 门禁成功且 GitHub 无冲突时，使用已验证 PR head 绑定 squash 合并，不管理员绕过，不直接推送 main，不发布版本或标签。任何冲突停止受影响合并并报告文件及原因，不自行选择冲突内容。
5. 合并后 fetch origin，尝试普通 git merge --no-edit origin/main 同步本地 main，保留其历史与本地内容，不把原任务分支再合入 main。若 Git 因冲突或未跟踪文件覆盖拒绝同步，保留现场并报告；不能把本地同步问题误报为远端未合并。
6. 交付和归档真实进入远端 main、source 工作树无新增内容后，清理用户所指 gkd-project-rules 工作树和相应本地/远端任务分支。以实际 PR head/merge 及内容事实确认 squash 成果，不仅凭 -d 失败使用强制删除。不触及其他分支、worktree 或归档。
7. 本轮主工作树的 .gkd/plan.md、.gkd/review.md 只有在已保存到仍可访问的归档、必要交付结束后精确移除。

不运行依赖安装、package manager、开发服务器、lint、类型检查、测试、构建、Cargo、Tauri、签名或打包。允许的本地验证限普通只读 Git/gh、目录状态和必要归档差异检查；远端自动 CI 承担实现验证。

AC：准确报告是否曾合并；本次 PR 的自动门禁与实际合并事实可核实；发生冲突则报告并保留材料；已有规则变更不扩大；说明本地 main 同步、归档、清理的真实状态。

## 同范围返工

PR #194，head e0c5ff84d8b5e82ab261d7651851794699a66c55，前端、Rust、PR 标题通过，但 contracts 中 check-plugin-system-docs.mjs 要求插件指南保留 pnpm 本地禁令而失败，ci-gate 随之失败。收尾角色已终止，尚未合并和清理。

main 仅在原范围 docs/plugins/developer-guide.md 的工具说明中补回“仓库本地 checkout 不得运行任何 `pnpm` 命令”，符合原计划保留 package guard 和环境边界的目标，不修改检查器。允许直接 Node 执行现有零依赖、只读 check-plugin-system-docs.mjs 和 check-cloud-only-verification.mjs 验证此次文档修复，以及 git diff --check。若分支落后，先无冲突地普通合并 origin/main 到任务分支；发生冲突仍停止。推送修复后按最终 head 等自动 CI，再完成原获准合并和清理。新增修复、验证和后续交付事实仅更新本轮必要记录。

## 实际交付状态

最终 head 4ffc45211cf7535b3429ff6bae76ff02f329c31a 的 ci-gate、contracts、frontend、pr-title、rust 全部成功。2026-09-08T07:59:49Z 远端 main 新增 b3d1b77ce40288975c64784d0bcbc57d27b82f71，提交标题包含 PR #194，与已验证 head 的完整文件树相同。但 gh pr merge 首次返回 502，后续 GitHub REST/GraphQL 查询仍为 PR OPEN、merged=false，普通重试因 BEHIND 被拒；未使用 admin 或手动关闭 PR。

本地普通合并 origin/main 被 .gkd/archive/model-price-rules/2026-09-08-r5/ 下 execution、plan-changes、plan、progress、review、summary 六份未跟踪 Markdown 阻挡，Git 在覆盖前中止。保留这些其他任务文件、本地 main 历史、任务工作树/分支和本轮活动记录；不宣称 PR 完成、本地已同步或清理完成。后续应先处理 GitHub PR 状态异常，并根据用户决定保留/安置六份文件后再同步。

## r3 续办决定与授权

用户明确要求“继续合并，有问题就处理”，授权范围内修复远端交付状态和本地同步阻挡，无需重问。本轮已确认远端 b3d1b77c 与已验 CI 的任务 head 整树一致；PR #194 仍 OPEN。

1. 普通合并 origin/main 到现有任务分支，无实现变化；新增本任务续办归档，记载已进入远端的规则、CI 与 502 事实。更新 PR 描述为最终差异：规则已在 b3d1b77c 进入 main，本次剩余差异只含交付记录。推送后等待新 head 的自动 ci-gate/pr-title 及实际选定检查，通过后正常 squash 合并，修复 GitHub PR 元数据；不重开 PR、不绕过规则、不重复实现。
2. 六份阻挡本地同步的模型定价归档逐文件比对：五份 Git blob 相同，summary.md 是远端正文加本地最终发布记录。允许临时搬到仓库外本任务临时目录，普通合并 origin/main 后校验五份相同文件已落地，并将含追加事实的 summary 原样还原；其本地修改保持未提交，不夹入规则任务。
3. 若需处理冲突，以保留已交付实现和真实本地记录为准，最小处理；不 reset、rebase、stash、强推或丢弃独有历史。其他 provider-reliability 归档继续保留。
4. 最终正常 PR MERGED、归档可访问、工作树无新增内容后，完成原获准工作树/本地及远端任务分支清理；本轮活动记录归档后精确移除。继续沿用已验证实现，不运行新业务检查；只使用零依赖 Git/gh/文件比对和必要文档合同，自动 CI 按最终 head 验证。
