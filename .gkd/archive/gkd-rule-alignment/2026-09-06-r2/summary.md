# GKD 规则对齐任务归档

- 任务：gkd-rule-alignment；归档日期：2026-09-06（Asia/Shanghai）。
- 路线：delegated/automatic；PLAN、execution、review revision：r2。
- 逻辑目录：main-worktree 维护计划与审查，execution-worktree 执行和验证。
- 任务分支：`chore/gkd-rule-alignment`；目标分支：`origin/main`。
- 基线：`bc891afbb80b70efc81b628a1c48b1bd0a7051da`。
- 实现提交：`94caf7a1476e6c3e00b3764ba8eb12824ef9388b`。
- 交付入口：[PR #191](https://github.com/KNaiFen/aio-coding-hub/pull/191)。归档和活动记录清理随同一 PR 交付，实际 squash merge SHA 以该 PR 记录为准。

## 目标与结果

AIO 项目规则以用户级 GKD 为生命周期优先依据，保留项目环境和 PR 边界；资料、检索和验证按受影响行为触发。修正 17 个既有规则/治理文件，删除经复核的 20 个空模板，4 份历史文件仅加状态说明且正文不变。业务代码、依赖、workflow 和 CI 分类器未改动。

文档合同检查器仅取消四项自然语言原句断言，等义正例及 GKD 入口负例完成回归。实际 package guard、本地命令限制、Tauri hook 和 CI 门禁保持。没有新 parser、工作流实现、状态机制、空模板或不必要的抽象。

## 验证与审查

本地批准的 Node 语法、cloud-only 合同及自测、spec-links、diff、15 个本地链接、2 个锚点和历史正文比较通过。实现 head 的 [CI run 33972234038](https://github.com/KNaiFen/aio-coding-hub/actions/runs/33972234038) 成功，ci-gate、pr-title 及所选前端/Rust/合同检查通过。

执行 session 已停止；命名 gkd_accept 独立验收无返工项；main review r2 通过。实现和审查证据在本目录五份快照内。归档提交本身仍按项目规则核对新 head 自动检查，不将旧 head 的结果替代新 head 证据。

## 取舍与限制

- GKD 自身问题保存在其仓库 `docs/reports/2026-09-05-aio-rules-gkd-issues.md`，不纳入 AIO 提交，也未修改/安装 GKD。
- PR 监控脚本等待 open PR 合并，造成首轮 timeout；实际 CI 全绿已被 main 和独立验收核实。后续使用 Skill 现有 workflow run 目标监控，问题作为 GKD-06 报告。
- 收尾时安装版 GKD 已更新，按现行 Skill 允许的任务自有成果和同 PR 清理执行；不由项目添加例外。
- 原 main checkout 的独有历史保留，不 reset，也不把 squash 前任务分支重复合入 main。必要远端同步使用 fetch；最终本地保留状态单独报告。
- 仅本任务活动记录、已合并且干净的任务 worktree/分支在授权清理范围；发布、其他现场及 GKD 的在途修改不在范围。

## 归档与后续事实

本归档保存归档时已验证的实现、取舍和审查。归档快照完整且不含本机绝对路径；本任务活动记录在快照核对后清理。合并及现场删除在当前 head 门禁通过后执行，其实际结果由 PR 记录和最终交付报告核对，不在归档提交前预填成功。
