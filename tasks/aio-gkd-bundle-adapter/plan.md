# AIO GKD Bundle Pin And Project Adapter Plan

## Goal

将 AIO 接入固定为已发布并已验证的 GKD `v0.1.1` bundle，并在不复制通用 GKD 逻辑的前提下建立最小项目 policy、review adapter 和 project staging 证据。

## User Decisions

- 用户已在当前对话授权继续 AIO adoption、必要的任务 PR、CI 修复和无阻塞验收后的合并。
- 固定发行输入为 `v0.1.1`，source/tag SHA `ded7a727fb391b8b7062fc531d03c9b6942c834a`，asset SHA-256 `502875847b1c6c1aa0843f9fe0f1d37810db457cd5e3be288183d1a7ff8c531e`，bundle digest `68188dcaeb98d93902b435c98784e242090ed18828e9d96a8dee735244f7d1ef`。
- 本 bootstrap 任务使用 manual route；AIO policy 尚未进入受控候选前不得伪造 automatic activation、claim 或 executor 替代。
- action mode 为 `implement_and_merge_on_acceptance`，但只有同一 fixed PR head 的 required checks 和独立验收均无阻塞 finding 才能合并。

## Behavior And Defaults

- GKD bundle 只从已验证的临时安装根调用；调用会话必须提供 Python 3.11+，但任何 AIO tracked file 不得写入机器绝对路径。
- policy 的唯一 GitHub repository identity 是 `github.com/KNaiFen/aio-coding-hub`，base branch 是 `main`，required checks 按 canonical order 固定为 `ci-gate`、`pr-title`。
- project stage 只写候选 worktree 的机器本地受管文件；已有冲突、symlink、bundle drift 或 project verify drift 都立即停止。

## Scope

- 新增 AIO 的 `.gkd/policy.json`，绑定 `github.com/KNaiFen/aio-coding-hub`、`main` 与已确认的 required checks `ci-gate`、`pr-title`。
- 新增 schema-valid 的 AIO review adapter，声明仓库身份、policy 路径和项目可用的 review capabilities。
- 增加最小、可复核的 AIO bundle pin 记录和 adapter compatibility smoke，使用已发布 asset 的验证输入，不嵌入 bundle 代码或机器绝对路径。
- 从干净候选 worktree 对已验证 bundle 运行 project stage/verify，生成的 `.codex`、`.agents` 与 runtime inventory 只作为机器本地 staging，不提交。
- 记录 legacy mapping，并保持后续 state/history migration、CI/release integration 与 canary/deletion 为独立任务。

## Non-Goals

- 不迁移或手改 `.trellis` task state、archive、offer、claim、receipt 或历史交接。
- 不删除旧 Trellis/Skill 实现，不改产品代码、Rust/Tauri、CI/release workflow、runner、Secrets、GitHub settings、tag、Release 或生产 `~/.codex`。
- 不在 AIO 实现或 vendoring 任意通用 GKD CLI、role、monitor、acceptance 或 scanner 逻辑。
- 不启动 automatic route 或通过 generic worker、角色替换、模型降级或 fallback 规避其门禁。

## Acceptance Criteria

- [ ] B-01：发布 asset 的 SHA-256、隔离 install verify、bundle version 和 content digest 均精确等于 User Decisions。
- [ ] B-02：`.gkd/policy.json` 通过 bundle schema，且 policy/repository/origin/base branch/required checks 一致。
- [ ] B-03：AIO review adapter 通过 bundle schema、canonical digest 和 repository-policy linkage 验证。
- [ ] B-04：adapter compatibility smoke 在固定 base 和 final candidate head 上可复现，并且不运行本地依赖安装、构建、Rust 或前端测试。
- [ ] B-05：project stage/verify 对 candidate 成功并生成绑定 bundle/role/config/skill/inventory digest 的机器本地事实；受管 staging 文件不进入 Git。
- [ ] B-06：完整 diff 仅覆盖本任务 documents、`.gkd` adapter/policy 与最小验证入口；旧状态和项目专有 CI/release 行为未变化。
- [ ] B-07：执行者提交 delivery document、固定 head CI 和独立 acceptance 均绑定同一 PR head；无阻塞 finding 后才允许精确合并。

## Compatibility

- 现有 AIO 产品 PR 继续使用既有 `ci-gate` 和 `pr-title`；本任务不改它们的工作流实现或 required-check 名称。
- 已归档的 `.trellis` 任务是历史只读材料。任何 v1 state migration 只在后续独立任务通过受支持的 GKD 命令完成。

## Security And Data

- 不读取、复制、编辑或记录认证材料、Secrets、生产 `~/.codex` 内容、agent transcript 或 machine-local runtime identity。
- 验证输出只记录 digest、版本、路径类别和终态；不得把 capability、receipt、token 或绝对机器路径提交到候选。

## Migration

- 本任务仅建立第一层 adapter，不迁移 legacy machine state。
- state/history migration 必须等待 B-01 至 B-05 被独立验收并合并，再以 fresh main 和 accepted bundle 进入独立任务。

## Public Interfaces

- `.gkd/policy.json` 采用 GKD CI policy schema version 1。
- review adapter 采用 GKD review adapter schema version 1，并以 bundle canonical-digest algorithm 生成 `adapterDigest`。
- compatibility smoke 是 AIO 项目入口，不是 GKD CLI 的副本；它只验证 tracked adapter facts 与已验证 bundle 的公开 schema/API。

## Execution Route

- manual route。trusted main 只创建/审批任务、登记 branch/worktree 和验收；独立执行 session 只在注册 candidate worktree 实施、验证、提交、推送、维护 PR 与 delivery，然后停止。

## External Side Effects

- 允许创建/推送本任务分支、PR 和范围内 CI 修复；在 B-07 满足后允许对该 fixed PR head 同步合并。
- 不允许生产安装、AIO GitHub 设置、Secrets、付费 runner、tag、Release、部署或计划外远端写入。

## Action Mode

- `implement_and_merge_on_acceptance`：只允许在 fixed-head local evidence、required checks、独立 review 和 acceptance 全部成功后合并。

## Implementation Notes

1. 执行者先复核发布 asset SHA-256、临时 install verify、bundle version/content digest 和 candidate base/head。
2. 根据公开 schema 写 policy 与 review adapter；adapter digest 必须由 bundle canonical function 生成，不能手算或复用样例。
3. 写最小 compatibility smoke，并让它检验 policy/repository/origin、review adapter digest 和 release pin；不得调用 package manager 或运行云端职责。
4. 对无未跟踪产物的 candidate 运行 AIO 当前 local verification contract 与 compatibility smoke；随后 project stage/verify 并把机器结果写入 delivery，不提交受管 staging。
5. 提交、推送、创建/更新 PR，固定最终 head 后等待项目 required checks；delivery document 是 final task transition 前唯一可提交的文档。
