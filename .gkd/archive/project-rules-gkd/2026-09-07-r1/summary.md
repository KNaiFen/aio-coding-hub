# 项目规则统一采用 GKD：本地交付归档

- 任务标识：project-rules-gkd；PLAN / execution：r1 / r1。
- 归档观察：2026-09-07 02:54:17 +08:00，Asia/Shanghai。
- 整理作者：GKD 收尾角色；方案、授权与最终审查决定仍归 main。
- 本轮终点：保留任务分支和执行 worktree，完成已审查实施差异与本归档的一次本地提交，提供可审阅差异。
- 任务分支：`chore/gkd-project-rules`；实施基线与提交前 HEAD：`a35f2aa4ea469f6e4066582b2e969f1ec44fca2e`。
- 原主分支 HEAD：`193767510ef647193ce5f16390bc1f663c3dffb0`；既有历史分叉与原文件保留，未获授权整理。

## 已完成实施与审查事实

执行角色已停止。19 个既有文件的实施差异经独立验收及 main 审查通过：14 修改、5 删除，未发现需要返工的 findings。项目入口与现行文档清除自行规定的生命周期、固定路线、提交、交接和模板要求；云端合同检查器解除对 GKD 文案和交接文件名的绑定。项目资源限制、package guard、Tauri、Actions 实际合同及模块业务语义保留。

六项获批轻量检查全部 exit 0：`git diff --check`、cloud-only contract、cloud-only selftest、CI quality gates、spec links、Markdown links。执行环境、实际命令及输出摘要见 [progress](progress.md)；独立验收与 main 决定见 [review](review.md)。收尾未重跑实现测试，也不改变实施内容或审查结论。

未运行依赖安装、package scripts、开发服务器、完整 lint/typecheck/test/build、Cargo、Tauri、签名、打包或其他重型本地检查；未运行远端 CI。当前本地终点无必要 CI。后续若获准 PR，应等待现有自动 `ci-gate`、`pr-title` 及按脚本变化选中的两端 CI，本地结果不能替代远端结果。

## 记录与范围

- [plan](plan.md)：main 的实际获批方案和交付许可。
- [execution](execution.md)：main 的实际 execution r1。
- [progress](progress.md)：执行角色的实际实施与验证记录。
- [review](review.md)：main 的最终通过结论、独立验收事实及收尾许可。
- 本任务没有 plan-changes，未制造空记录。快照均可在本目录独立阅读，不依赖活动记录继续存在。
- 归档中的绝对本机路径和账号已用角色占位符替换；未保存运行句柄、真实凭据、完整对话或全量日志。

收尾收到许可：仅提交已审查的 19 个既有文件和本目录五份归档；提交成功并确认归档可访问后，精确删除主工作树 `.gkd/plan.md`、`.gkd/review.md` 与执行 worktree `.gkd/execution.md`、`.gkd/progress.md` 四个本任务独占未跟踪活动文件。原根级记录、既有归档、未知文件及他人修改不属于本次清理对象。

## 观察边界与后续事实来源

本摘要写入时归档提交与活动记录清理尚未执行。包含本目录的实际 Git 提交是本次本地交付对象；完整 SHA、提交命令结果、四个活动文件的清理状态及相关工作树最终状态由本轮收尾返回记录，不预填成功，也不为回填归档自身 SHA 递归提交。保留的任务分支和 worktree 使这些 Git 对象与归档在清理后继续可访问。

未获准推送、创建 PR、合并、发布、删除 worktree 或任何分支；这些动作不属于本轮本地交付终点。没有本任务已知实施阻塞，后续远端交付及原 main 历史整理均需 main 根据实际授权另作决定。
