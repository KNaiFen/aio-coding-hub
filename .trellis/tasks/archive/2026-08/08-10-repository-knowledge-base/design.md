# 项目知识库整理设计

## 1. 事实源分层

项目知识按用途而不是文件扩展名分层：

1. `AGENTS.md`：执行边界与代理工作规则，只放强约束和入口。
2. `docs/README.md`：长期知识导航和文档生命周期，是“去哪里找答案”的唯一入口。
3. 现行规范：产品基线、架构/RFC、插件参考、运维指南与 `.trellis/spec/`，必须跟随实现更新。
4. 任务事实：`PENDING.md` 与 `.trellis/tasks/` 表达尚未完成的工作；完成后迁入对应归档。
5. 历史证据：带日期的审计、被替代计划、工程分析和 `PENDING_COMPLETED.md`，保留当时上下文但不指导当前实现。
6. 会话记录：`.trellis/workspace/` 仅供追溯，不作为当前事实源。

发生矛盾时，当前代码/机器合同与现行规范优先于历史审计、任务正文和会话日志。

## 2. 目录设计

保留已有稳定公共路径，避免无收益的大规模重命名：

```text
docs/
  README.md
  product/
    overview.md
  plugin-system-rfc.md
  plugin-manifest-v1.md
  release-homebrew.md
  plugins/
  history/
    README.md
    audits/
      codebase-health-audit-2026-08-09.md
      upstream-integration-audit-2026-08-03.md
      plugin-system-audit-2026-07-02.md
    plans/
      plugin-system-development-plan.md
    engineering-notes/
      realtime-trace-card-cli-tab-leak-analysis.md
      upstream-main-reconciliation-2026-07-05.md
```

`CHANGELOG.md`、`PENDING.md`、`PENDING_COMPLETED.md` 保留在根目录，因为它们是通用仓库入口且已被自动化/规则引用。

## 3. 历史化策略

- 历史文件保留原始正文和当时命令，顶部增加醒目的历史状态、证据日期、替代入口与“不要作为当前操作说明”的边界。
- 不把历史源码行号批量改成当前路径；这会破坏当时证据。知识索引明确历史记录的链接/路径可能随代码演进失效。
- 删除只包含旧索引生成状态的 `omx_wiki/index.md`、`omx_wiki/log.md`，由 `docs/history/README.md` 替代。
- 未跟踪的旧健康审计副本只在确认被终版覆盖后移除；本地工具产物通过 `.gitignore` 排除。

## 4. 现行文档纠错

- `hostCompatibility.platforms`：Rust `validate_host_compatibility` 已对当前 OS 强制校验，应统一写为安装、更新、重验和启用的权威阻断条件；市场列表是否预筛选属于 UI 体验，不改变宿主校验。
- diagnostics：声明 `diagnostics.read` 后，插件可读取自身最近运行报告；宿主强制插件身份和 `1..100` 上限，不能读取其他插件。
- 版本叙述：Plugin API v1 与应用版本解耦。现行说明使用“当前宿主/当前实现”，带日期的 `0.62` 设计只留在历史审计。
- 本地执行：项目贡献者遵循 `AGENTS.md` 的零产物规则；源码内协议说明把测试命令标为 GitHub Actions 所有。
- 文档合同脚本校验行为语义和关键边界，不锁定不存在的未来应用版本文字。

## 5. Trellis 归档

归档前先解除 `08-03-upstream-claude-oauth` 与已完成父任务的关系，并把其 base branch 收敛到 `main`。其余确认已经进入 `main` 的任务使用 `task.py archive --no-commit` 迁入 `archive/2026-08/`；原有 PRD、设计、实现和研究证据原样保留。

需要归档的范围：

- 早期遗留：`00-join-fingercaster`、`07-20-codex-provider-model-discovery`、`07-25-session-reuse-switch`、`07-31-cloud-only-build-artifact-promotion`、`08-01-home-route-tooltip-density`。
- 上游选择性集成：父任务及除 Claude OAuth 外的 5 个已交付子任务。
- 请求可观测与供应商状态：父任务及 5 个已由 PR #52 和后续 `0.60.49` 收口的子任务。

另一个 worktree 中 49 个未跟踪任务目录不直接复制：它们的代码已进入 `main`，且多数元数据无效或重复。当前主线已有的任务/归档与健康审计保留可追溯入口，避免把旧工作区噪声并入事实源。

## 6. 风险与回滚

- 移动使用 Git 可追踪 rename；任何漏链可由静态链接检查发现。
- 文档合同脚本与文档在同一变更中更新，防止旧短语继续锁死错误事实。
- Trellis 归档使用 `--no-commit`，便于在一个可审阅 diff 中回滚；不清理 worktree 注册或其他分支。
- 不删除 `.local/` 参考 checkout、`upgrade-tui.command` 或未知用户文件。
