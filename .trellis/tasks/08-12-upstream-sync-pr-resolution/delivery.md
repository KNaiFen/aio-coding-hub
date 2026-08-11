# 交付报告：修复 Sync Upstream PR 编号解析与冲突收敛

> 本文件保留阶段 A 的任务归属迁移快照。2026-08-12 main 已授权阶段 B；阶段 B 的正式交付必须以同步后的实时 PR head 和自动检查为准。

## 交付状态

- 结果：阶段 A 已完成；阶段 B 已授权但尚未由执行 session 开始。本次 main 授权提交不改变 Sync Upstream 代码。
- PR：[修复 #114](https://github.com/KNaiFen/aio-coding-hub/pull/114)（Draft，OPEN）
- 分支：`fix/upstream-sync-pr-resolution`
- PR base：`main` @ `82820b2ea10ec6028d1fcb8d130a993bfae39b6d`
- 归属迁移前的远端 head 快照：`ed4a7527f75ea09ff55517afa3789babd0f922a6`
- 源规划提交：`2016c25ef7cb6ae524f3f2b4e86996ef923981a3`
- 阶段 B 授权提交：`e4e797e42274e8c87f70121d70f9c51a160be9f5` 及随后记录更新 `1326a6f391a15d6c351bbc367bc232c15087d88e` 都未产生 Actions run、check suite 或 check run；PR #114 当前为 `DIRTY`/`CONFLICTING`，本地三方合并确认唯一冲突为 `.trellis/tasks/README.md`，所以这些 head 都不是可验证 CI 候选。
- `ci-gate`：通过，[run 31509027197](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31509027197)
- 其他检查：`pr-title` 通过，[run 31509027080](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31509027080)；CodeQL JS/TS 与 Rust 通过，[run 31509027104](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31509027104)。
- #113 回归：[Sync Upstream run 31508611251](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31508611251) 按预期失败，输出 PR #113 的 `DIRTY` 状态并要求人工冲突处理。
- 交付时间：2026-08-12（阶段 A 快照）
- 执行 session：阶段 A session 已暂停；尚未创建阶段 B 执行 session。main 将发送冲突恢复交接包；执行 session 必须先解决唯一 README 冲突并推送干净 head，之后才等待自动检查。

## 阻塞快照

- 无代码或失败 CI 阻塞；当前 PR 冲突阻止现有 head 产生常规自动检查。用户已授权按已锁定的阶段 B 范围先解决唯一 README 冲突；不得手动 dispatch、推空提交或修改工作流。解决冲突并推送新 head 后才等待实时自动检查。

## 实现摘要

### 用户可见结果

- 不改变 Sync Upstream 的运行行为。任务与交付事实从 #108 的旧治理任务中分离，使 PR #114 的审查入口不再混淆。

### 内部实现

- `.github/workflows/sync-upstream.yml` 的既有修复在新建 PR 路径严格解析 `gh pr create` stdout；已有 PR 才运行受限 list 查询。
- `scripts/check-sync-upstream-policy.mjs` 与 selftest 继续锁定 URL、正整数编号、无 push/merge/approval 以及 `DIRTY`/`UNKNOWN` fail-closed 合同。
- 本阶段创建独立 Trellis 包，并将旧任务包的 #114 时期材料恢复至 `origin/main`。

## 验收标准对应

| 标准 | 结果 | 证据 |
|---|---|---|
| AC-01 任务身份一致 | 阶段 A 交付项 | 新 `task.json`、`execution.md`、本文件与 `.trellis/tasks/README.md` |
| AC-02 旧任务无 #114 差异 | 阶段 A 交付项 | `git diff --name-only origin/main...HEAD -- .trellis/tasks/08-10-github-actions-governance` 无输出 |
| AC-03 #114 证据归属 | 阶段 A 快照 | 上列 `ed4a7527...` 的 `ci-gate`、`pr-title`、CodeQL 和 run `31508611251` |
| AC-04 不进入阶段 B | 阶段 A 交付项 | 未运行 `task.py start`；只推送本任务分支 |

`cdc427b9c6b386ca6106a371880710155704a81e` / run `31506469918` 仅是历史候选背景，不能作为本任务的最终交付候选或最终 CI 证据。

## 验证结果

### 本地检查

| 命令 | 结果 | 说明 |
|---|---|---|
| `python3 ./.trellis/scripts/task.py validate 08-12-upstream-sync-pr-resolution` | 未运行 | 仓库 cloud-only 规则只允许依赖无关的 Node 合同/语法与 `git diff --check`；阶段 A 不扩大本地验证权限 |
| `git diff --check` | 通过 | 检查任务归属分离补丁 |
| `git diff --name-only origin/main -- .trellis/tasks/08-10-github-actions-governance` | 通过（无输出） | 提交前确认旧任务工作树已恢复为 `origin/main` |

### GitHub CI 与编译

- 上述云端证据属于归属迁移前的 #114 head 快照。`e4e797e4...` 和 `1326a6f3...` 因 PR 冲突没有自动检查，不能作为当前候选；执行 session 解决唯一 README 冲突并推送干净 head 后，main 再观察自动 CI，不手动触发工作流。

### 人工验证

- 无。未处理或合并 PR #113，未读取或执行任何凭据。

## 兼容性、风险与回滚

- 兼容性：无产品 API 或工作流行为改动。
- 数据与配置：无。
- 安全与隐私：不读取或记录 secret、App 私钥或 token；保持 no-push/no-merge/no-auto-approve 边界。
- 回滚方式：回退任务归属分离提交可恢复原有记录布局，但不应把 #114 重新归属到旧任务。
- 剩余风险：阶段 A 提交后的实时 PR head 与 CI 必须在阶段 B 前由 main 重新核验。

## 未完成项与阻塞

- 阶段 B 的同步、交付重绑和最新 head CI 复核尚未由执行 session开始；当前先按 `execution.md` 的冲突恢复启动条件解决 README 冲突，再等待自动检查。

## 建议 main 重点审查

- 新任务目录与 `.trellis/tasks/README.md`：确认 PR #114、分支、base、worktree 和唯一写者仅指向本包。
- `.trellis/tasks/08-10-github-actions-governance/`：确认相对 `origin/main` 没有 #114 差异。
- `.github/workflows/sync-upstream.yml` 与 policy contract：阶段 A 未改动；后续只允许维持严格 stdout 解析和 fail-closed 边界。

## 阶段 B 授权（main）

- 授权日期：2026-08-12。
- 授权范围：同步 `fix/upstream-sync-pr-resolution` 到开始施工时最新 `origin/main`，解决预期的 `.trellis/tasks/README.md` 冲突，更新 Trellis 生命周期和交付/CI 证据。
- 已知快照：当前 `origin/main` 为 `9aa8e4ab8e6417be4816b0811178c3f401e34171`；PR #114 旧 head 为 `6316204274eeb6db9332b4eef0e5f182c5c31ca7`，旧必需 CI 已通过。执行 session 必须在写入前重新查询，不能把快照当最终证据。
- 锁定边界：保留 #113 的 fail-closed 人工处理路径；不得修改 stdout 严格解析、放宽 `DIRTY`/`UNKNOWN`/空状态处理、读取或处理 `upgrade-tui.command`、读取或提交 `SESSION_REMEDIATION_PLAN.md`，也不得合并 PR 或推送 `main`。
- 接手条件：main 发送冲突恢复交接包后，一个执行 session 可写入本 worktree，仅同步 `origin/main` 并解决预期 README 冲突；其推送的新 head 自动检查绿色、head 未漂移后，才进入 main 验收交接。其余 session 保持暂停。

## main 验收记录

> 仅 main 填写。

## main 收尾

> 仅 main 填写。任务保持活动，未合并、未归档、未删除 worktree 或分支。

## 返工记录

无。
