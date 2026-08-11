# 交付报告：修复 Sync Upstream PR 编号解析与冲突收敛

> 本文件先保存阶段 A 的任务归属迁移快照。它不是 PR #114 的最终验收交付；任务归属分离提交推送后，main 必须以 GitHub 的实时 head 和实时检查决定是否进入阶段 B。

## 交付状态

- 结果：阶段 A 任务归属分离完成后暂停，等待 main 指令；本阶段不改变 Sync Upstream 代码。
- PR：[修复 #114](https://github.com/KNaiFen/aio-coding-hub/pull/114)（Draft，OPEN）
- 分支：`fix/upstream-sync-pr-resolution`
- PR base：`main` @ `82820b2ea10ec6028d1fcb8d130a993bfae39b6d`
- 归属迁移前的远端 head 快照：`ed4a7527f75ea09ff55517afa3789babd0f922a6`
- 源规划提交：`2016c25ef7cb6ae524f3f2b4e86996ef923981a3`
- `ci-gate`：通过，[run 31509027197](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31509027197)
- 其他检查：`pr-title` 通过，[run 31509027080](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31509027080)；CodeQL JS/TS 与 Rust 通过，[run 31509027104](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31509027104)。
- #113 回归：[Sync Upstream run 31508611251](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31508611251) 按预期失败，输出 PR #113 的 `DIRTY` 状态并要求人工冲突处理。
- 交付时间：2026-08-12（阶段 A 快照）
- 执行 session：完成阶段 A 提交和推送后暂停。

## 阻塞快照

- 阶段 B 被明确暂停，等待 main 指令；这不是代码或 CI 阻塞。

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

- 上述云端证据属于归属迁移前的 #114 head 快照。阶段 A 文档提交将产生新 head；不手动触发 CI，等待 PR 自动检查。

### 人工验证

- 无。未处理或合并 PR #113，未读取或执行任何凭据。

## 兼容性、风险与回滚

- 兼容性：无产品 API 或工作流行为改动。
- 数据与配置：无。
- 安全与隐私：不读取或记录 secret、App 私钥或 token；保持 no-push/no-merge/no-auto-approve 边界。
- 回滚方式：回退任务归属分离提交可恢复原有记录布局，但不应把 #114 重新归属到旧任务。
- 剩余风险：阶段 A 提交后的实时 PR head 与 CI 必须在阶段 B 前由 main 重新核验。

## 未完成项与阻塞

- 阶段 B 的交付重绑、最新 head CI 复核与后续维护未开始，必须等待 main 指令。

## 建议 main 重点审查

- 新任务目录与 `.trellis/tasks/README.md`：确认 PR #114、分支、base、worktree 和唯一写者仅指向本包。
- `.trellis/tasks/08-10-github-actions-governance/`：确认相对 `origin/main` 没有 #114 差异。
- `.github/workflows/sync-upstream.yml` 与 policy contract：阶段 A 未改动；后续只允许维持严格 stdout 解析和 fail-closed 边界。

## main 验收记录

> 仅 main 填写。

## main 收尾

> 仅 main 填写。任务保持活动，未合并、未归档、未删除 worktree 或分支。

## 返工记录

无。
