# 模型自定义定价与倍率收尾事实

## 归档观察时点

- 时间：2026-09-08 12:29 +08:00；作者：本次 GKD 收尾角色，依据 main r5 交接与既有审查。
- 本目录完整保存 plan、plan-changes、execution、progress、review 五类 Markdown；保留各来源作者、日期与历史失败结论，脱敏本机路径和运行时角色标识。无凭据、用户数据或全量日志。
- 产品行为：按 CLI/完整模型配置单价、单模型整体或分项倍率；同一表单仅检测互斥，显式 0/1 均视为设置，不改变历史费用。业务 execution r4.1，交付 PLAN r5。
- 基线：`b147ced0f8cab342ee57925a5bd6b08d37e061db`；已审查实现 head：`173ce95b66ff674d67f5e802e4c466125215aa85`。
- PR：[#193](https://github.com/KNaiFen/aio-coding-hub/pull/193)，任务分支 `feat/model-price-rules`，目标 `main`，拟发布 `aio-coding-hub-v0.60.60`。

## 已有验证与边界

- main 已依据两轮独立审查确认原四项 findings 全部解决；所有施工和独立审查角色已停止。r5 版本准备、云端格式/绑定修正及两测试的七行连接生命周期修正已由 main 审查，无后续业务变化。
- 实现 head 的 [CI run 34184637399](https://github.com/KNaiFen/aio-coding-hub/actions/runs/34184637399)，attempt 1，conclusion success；2026-09-08 12:25 +08:00 独立监控确认 required + expected 的 `ci-gate`、`pr-title` 均 SUCCESS，无缺失。
- 前端 313 测试文件及构建通过；Rust 主库 2969 passed / 0 failed / 4 ignored，workspace/集成测试、Clippy、生成绑定一致性、audit 和必要 benchmark 通过。
- 历史失败 run 34182820598（绑定漂移）及 34183291396（两个测试连接占用）保留在 progress/review；上述新 head 的成功证据表明修正已通过，不改写旧结果。
- 本地仅执行获准零依赖检查；没有安装依赖或运行测试、构建、生成器、格式化、签名及打包。未进行桌面窗口手工视觉验证。
- 收尾归档后 `git diff --check` 和 `node scripts/check-cloud-only-verification.mjs` 均退出 0；限定归档目录的本机路径及运行时句柄检查无匹配。归档提交只含本任务五类事实、摘要及已跟踪 progress 迁移。

## 本次授权和待办

- 用户已批准当前方案、automatic，以及推送、PR、合并和发版。收尾仅提交归档及已跟踪 progress 迁出，不更改业务实现。
- 本归档将形成新的 PR head，其必要自动检查尚待该次提交完成后确认；不得以旧 head 通过代替。
- 合并、实际合并提交的 main CI、唯一 release candidate、tag 和 Release 均尚未完成；后续事实追加于主工作树同目录 summary，注明实际观察时点，不为回填自身 SHA/CI 递归提交。
- 新的用户清理授权已由 main 交接：在成功 squash 合并、发布完成及归档独立可访问后，删除本任务本地/远端分支、执行 worktree、已归档五类独占活动记录及明确归属的临时 PR/补丁文件。该授权替代原保留任务分支/worktree 的约定。
- 本地 main `193767510ef647193ce5f16390bc1f663c3dffb0` 的既有分叉保留，允许 fetch 更新远端引用；其他任务归档、worktree 和旧根目录记录不属于本次清理。
