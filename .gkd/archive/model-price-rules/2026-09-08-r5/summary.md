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

## 最终 PR 与合并事实

- 观察：2026-09-08 13:07 +08:00，作者：本次收尾角色。归档提交为 `27ce7c27db5676b5445c088d7155ccbffd0e7ab8`，提交说明 `docs: 归档模型自定义定价交付记录`，已推送并成为最终 PR head。
- 该 head 的 [PR CI run 34187295190](https://github.com/KNaiFen/aio-coding-hub/actions/runs/34187295190)，attempt 1，conclusion success；contracts、frontend、rust、observer-macos 和 ci-gate 均 SUCCESS。PR 发布候选 job 按工作流条件 SKIPPED，不代表运行过打包。
- 只读 watcher 于 2026-09-08 13:05:50 +08:00 返回 success、退出 0；检查范围 required + expected 的 ci-gate/pr-title 全部 SUCCESS，无缺失和失败。
- `gh pr merge --squash --match-head-commit` 绑定上述完整 head，退出 0。PR #193 于 2026-09-08 13:06:15 +08:00 MERGED，实际 merge commit 为 `42b64aa646e19ce6b1f3bf6a08219043ea960dd4`。
- `git diff --quiet <最终 PR head> <merge commit>` 退出 0，证明 squash 后完整树与任务成果一致。`git fetch origin` 成功，origin/main 更新至该合并提交；远端任务分支已由仓库自动删除。
- main 的 [CI run 34189360548](https://github.com/KNaiFen/aio-coding-hub/actions/runs/34189360548) 已因 push 自动启动，head 为上述实际合并提交；当前仍运行。发布候选、tag 和 Release 尚待完成。

## 主线候选与发布触发

- 观察：2026-09-08 14:07 +08:00，作者：本次收尾角色。上述 main CI run 34189360548 attempt 1 已 success；watcher 于 14:05:17 +08:00 取得真实终态并退出 0。后续 run jobs API 确认 contracts、frontend、rust、observer-macos、candidate-plan、两桌面目标、四 TUI 目标、assemble-release-candidate 和 ci-gate 全部 success；仅不适用的 manual-dispatch-guard skipped。
- 同一 source SHA 的 eligible main CI 仅上述成功 push run；其唯一未过期最终候选为 `release-candidate-42b64aa646e19ce6b1f3bf6a08219043ea960dd4-34189360548-1`，artifact ID `10042904584`，74197468 bytes。不存在第二个 eligible candidate；平台中间产物不作为最终候选。
- 核对 tag 与 Release 原先均不存在，本地同名 tag 也不存在；source 是 origin/main 祖先，五份包/应用 manifest 均为 0.60.60。
- 已创建并推送 annotated tag `aio-coding-hub-v0.60.60`，tag object 为 `a30c0e7bfec5ff9988de9f8ef8342a902451293c`，远端 peeled commit 为 `42b64aa646e19ce6b1f3bf6a08219043ea960dd4`。
- 普通 `git push origin refs/tags/aio-coding-hub-v0.60.60` 退出 0；服务器报告创建限制由当前凭据已有权限绕过。未调用 admin 参数、修改规则或覆盖已有引用。
- tag push 已自动触发 [release run 34193256944](https://github.com/KNaiFen/aio-coding-hub/actions/runs/34193256944)，head 与 source 一致；当前仍运行。只使用现有 release.yml 的候选复用流程，没有重建、签名或手动启动普通 CI。

## 正式发布事实

- 观察：2026-09-08 14:09 +08:00，作者：本次收尾角色。release run `34193256944` attempt 1 已 success，head 为 `42b64aa646e19ce6b1f3bf6a08219043ea960dd4`；watcher 于 14:07:31 +08:00 返回成功、退出 0。run jobs API 确认 publish 全部步骤 success，包括 source 验证、唯一候选选择、精确下载、文件及校验和验证、不可覆盖检查和 without rebuilding 发布。
- [正式 Release aio-coding-hub-v0.60.60](https://github.com/KNaiFen/aio-coding-hub/releases/tag/aio-coding-hub-v0.60.60) 于 2026-09-08 14:06:58 +08:00 published，isDraft=false，isPrerelease=false。12 项资产全部 state=uploaded、大小大于零且带 SHA-256 digest，名称集合与既有 release.yml 完全一致。
- 桌面资产：`aio-coding-hub-win64.msi`、`aio-coding-hub-win64.msi.sig`、`aio-coding-hub-win64-portable.zip`、`aio-coding-hub-macos-arm.tar.gz`、`aio-coding-hub-macos-arm.tar.gz.sig`、`aio-coding-hub-macos-arm.zip`。
- TUI 资产：`aio-tui-win64.zip`、`aio-tui-macos-intel.tar.gz`、`aio-tui-macos-arm.tar.gz`、`aio-tui-linux-x64.tar.gz`；更新及校验资产：`latest.json`、`SHA256SUMS.txt`。
- 依据当前成功 main CI 唯一候选复用原签名产物；未运行本地构建、签名、打包或安装，也未手动重复常规 CI。没有桌面窗口人工视觉验证证据。
- 本目录五类事实已存在于实际远端合并提交；主工作树副本附有最终 PR/main/release 事实，独立于即将清理的执行 worktree 可访问。远端任务分支已自动删除；本地任务分支、worktree 和剩余活动记录即将按用户新授权清理，尚未把准备动作写为完成。

## 活动记录与现场整理

- 本次收尾已依据用户追加的清理授权，移除主工作树的本任务独占 `.gkd/plan.md`、`.gkd/plan-changes.md`、`.gkd/review.md` 及执行 worktree 的 `.gkd/execution.md`。已跟踪 `.gkd/progress.md` 已随归档提交迁入本目录；五类完整事实均已保存，没有空模板或仅链接已删除来源的记录。
- 删除前执行 worktree 的 `git status --short --untracked-files=all` 无输出，且此前检查无忽略文件；无未保存变化。已确认 PR MERGED、完整 head 与 squash commit 树一致，以及归档和成果进入实际 origin/main。
- 从主工作树执行普通 `git worktree remove` 删除本任务执行 worktree，退出 0。随后按上述明确授权和 squash 内容证据执行 `git branch -D feat/model-price-rules`，退出 0；未靠普通 -d 失败推断可删除。远端同名分支在合并后已由仓库自动删除。
- 本任务原 PR 描述临时文件、本收尾 PR 描述临时文件及首轮已应用云端 patch 已精确删除；补丁临时目录为空后以 rmdir 删除成功。未下载发布二进制到 checkout。
- 保留本地 main 原有分叉、其他任务 worktree、provider-reliability 旧归档和根目录旧任务文件；其归属不在本次清理授权范围。没有直接推送远端 main，没有 reset 或将 squash 前任务分支合入本地 main。
- 业务、PR、必要 CI、正式发布及本任务记录/分支/worktree 清理均已完成。最终事实留在主工作树本目录，源五类归档已随 PR 合入远端；未为回填最终事实递归创建提交。
