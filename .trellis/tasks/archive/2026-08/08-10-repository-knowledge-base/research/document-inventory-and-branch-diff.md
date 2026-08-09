# 文档资产与跨 worktree 核对

## 主线资产

- 受跟踪文档型文件 544 个，其中 `.trellis/` 占 489 个；长期知识入口目前只有根 README、`AGENTS.md` 和插件局部入口，缺少 `docs/README.md`。
- `.local/codex-cli-reference/` 是固定外部参考 checkout，不属于项目知识正文。
- `.playwright-cli/`、`.impeccable/live/`、`.trellis/.runtime/` 和 `.codegraph/` 是本地工具/runtime 产物。
- `PRODUCT.md`、`UPSTREAM_INTEGRATION_AUDIT.md` 是有价值但未跟踪的项目资料；`CODEBASE_HEALTH_AUDIT.local-untracked-20260810.md` 是受跟踪终版之前的旧截面。

## `aio-feature-merge`

- 登记分支 `codex/close-pending-024-028` 的 HEAD `276c401a` 已是 `main` 祖先，所有已提交文档变更均进入 `main`。
- worktree 根还有 49 个未跟踪 Trellis 任务目录（230 文件）。其中 48 个已由 PR #40、#51、#53-85 等进入主线，1 个父任务只保存谱系；不能把这些旧活动副本原样复制进 `main`。
- 一份 `task.json` 还是无效 JSON，进一步证明应依据主线交付与现有归档治理，而不是导入该工作区状态。

## `aio-float-fix`

- 登记分支 `codex/archive-release-tag-fetch` 的 HEAD `fef05dec` 已是 `main` 祖先，`main` 领先 131 提交，分支无独有提交。
- 根目录 08-04 任务的差异是未勾选的旧实施状态或尾随空行；不应回灌。
- Release tag-fetch、Tray 和早期 observer 资料都已在 `main` 有等价或更晚归档；现行合同以 `.trellis/spec/` 和 08-05 归档为准。

## 决策

两个 worktree 不执行 merge/cherry-pick。只把它们用作“主线是否遗漏历史证据”的反证；知识库整理全部基于 `main` 当前树和可验证 Git 历史。
