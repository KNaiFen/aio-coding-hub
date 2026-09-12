# 工作流优化交付归档

- Task：workflow-streamline；日期：2026-09-05；PLAN：r3；execution：r2；review：r3。
- Route：direct-main，main 为唯一 writer；逻辑 worktree：workflow-streamline；分支：`ci/streamline-gkd-workflow`。
- 基线：`ca17a2d0312ed5ed83cda1931d5396834b4a284c`；实现提交：`6c9cec5ed670c660d6397449a5414e7bd649e58a`。
- 状态：实现完成；首轮 PR CI 在 Linux 的测试输出重定向处失败，已追加 r4 修复，自动门禁通过才允许合并。PR：https://github.com/KNaiFen/aio-coding-hub/pull/190 。最终云端结果以 PR 的检查与合并记录为准。

## 目标与结果

1. 修正旧 GKD 规则误禁现行 `.gkd/` 交接与只读监控/验收引用的问题，继续拒绝旧生命周期命令和运行时状态机制。
2. GKD Markdown 与旧根级任务 Markdown 归入过程文档；AGENTS 保留文档合同检查。纯文档 push 可跳过前端/Rust 重型任务；代码、混合、未知路径和手动 CI 保持完整检查。
3. 统一从远端 main 建任务分支、PR 自动门禁、squash 后同步真实远端结果的提交路径；本地独有提交先比较并保留。
4. upstream PR 已创建但 DIRTY/UNKNOWN 时警告并显示链接；空状态和 API/命令错误仍失败。
5. 发版继续晋升同一 main SHA 的成功签名候选；本轮没有版本、tag 或 release 动作。

## 验证与风险

- 零依赖检查、变更 Node 语法、diff 检查和 main 消融审查通过，详见 review.md。
- 本地没有安装依赖、运行产品测试或构建；前端/Rust 检查交由自动 PR CI。
- upstream 分支冲突仍需要人工处理；本轮没有解决已有 upstream PR 的冲突，也没有手动触发真实同步。
- 本地 main 的旧发布记录及独有历史保留；已有历史归档不改动。
- 现行外部 GKD 监控脚本的 `--pr` 等待 PR 合并/关闭，不能作为合并前 CI 等待目标；本轮停止该监控并改为 `--run` 跟踪明确 Actions run，外部 Skill 未修改。

## 授权与清理

- 用户先要求执行 PLAN，随后明确要求“推送，合并，清理分支”。
- 本次 cleanup commit 归档任务活动记录；main 在确认 PR 合并和工作树干净后删除本轮 worktree 与本地/远端分支。
- 不推送 main、不修改保护设置、不删除其他任务现场。归档不是活动状态源。
