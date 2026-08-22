# AIO GKD Bundle Pin And Project Adapter

## Goal

将 AIO 接入固定为已发布并已验证的 GKD `v0.1.1` bundle，并在不复制通用 GKD 逻辑的前提下建立最小项目 policy、review adapter 和 project staging 证据。

## User Decisions

- 用户已在当前对话授权继续 AIO adoption、必要的任务 PR、CI 修复和无阻塞验收后的合并。
- 固定发行输入为 `v0.1.1`，source/tag SHA `ded7a727fb391b8b7062fc531d03c9b6942c834a`，asset SHA-256 `502875847b1c6c1aa0843f9fe0f1d37810db457cd5e3be288183d1a7ff8c531e`，bundle digest `68188dcaeb98d93902b435c98784e242090ed18828e9d96a8dee735244f7d1ef`。
- 本 bootstrap 任务使用 manual route；AIO policy 尚未进入受控候选前不得伪造 automatic activation、claim 或 executor 替代。
- action mode 为 `implement_and_merge_on_acceptance`，但只有同一 fixed PR head 的 required checks 和独立验收均无阻塞 finding 才能合并。

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
