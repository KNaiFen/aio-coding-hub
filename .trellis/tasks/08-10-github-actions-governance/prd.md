# GitHub Actions 流程治理与提速

## Plan Status

- Implementation authorization: confirmed by user
- Confirmation date and summary: 2026-08-10 实施初版；2026-08-11 用户确认扩展为 PR 按前端、Rust 与共享路径分别验证，主干推送与手动候选构建保持全量；同日，在 GitHub App 已配置后的真实 Sync Upstream 运行暴露新建 PR 的可见性竞态，用户确认按本修订修复编号解析。
- Confirmed coverage: workflow files, CI contracts/selftests, current operational docs, GitHub Actions settings, upstream-sync credentials, Dependabot, CodeQL, CI path-domain classification, and post-merge upstream-sync PR resolution
- Planning revision: v3; resolve a newly created upstream PR from `gh pr create` output rather than an eventually consistent list query
- Execution route: delegated worktree, main-owned implementation and acceptance
- Migrated from direct-main record: none

## Material Facts, Assumptions, and Open Questions

| Item | Source | Status / closure condition |
|---|---|---|
| 最近 200 次 CI 中 48 组同 SHA 的 PR + workflow_dispatch，45 组在 PR 完成前重叠 | Actions audit | confirmed；本期目标是消除无意重复 |
| Rust 测试显式使用 `CARGO_BUILD_JOBS=1` 与 `--test-threads=1`，测试存在共享环境/缓存状态 | repository source and history | confirmed；本期不改并行度 |
| 默认 `GITHUB_TOKEN` 为 read，且不允许创建/批准 PR；同步当前因权限失败 | GitHub Actions logs/settings | confirmed；改用最小权限 App token |
| `upgrade-tui.command` 为 `AIO-PENDING-029` 未跟踪项 | `PENDING.md` and worktree status | confirmed；本任务不得读取、执行、修改、移动或删除 |
| GitHub App 已仅安装到本仓库，`SYNC_UPSTREAM_APP_ID` variable 与私钥 secret 已由 owner 配置 | 用户确认与 2026-08-11 真实工作流 | confirmed；不得读取、展示或记录值、PEM 或短期 token |
| Sync Upstream run `31487461146` 已输出 `https://github.com/KNaiFen/aio-coding-hub/pull/113`，随后立即 `gh pr list` 返回空并报 `Failed to resolve sync PR number.` | GitHub Actions failed-job log | confirmed；新建 PR 的编号不能依赖创建后的立即列表查询 |
| PR #113 已创建，当前为冲突状态 | GitHub PR metadata | confirmed；修复后的回归运行应 fail-closed 并明确报告该 PR 的 `DIRTY`/人工冲突状态，不得 push、merge 或自动批准 |

Do not start implementation with a material open question. Record approved scope or decision changes in this file with their affected acceptance criteria and confirmation.

## Goal

消除 PR 与手动 CI 重复运行，分离 required gate，补齐安全自动化，并让上游同步在创建人工审核 PR 后可靠且 fail-closed 地解析该 PR。

## Requirements

- 自动 PR/push CI 继续产生唯一 required `ci-gate`；PR 标题校验迁移为独立 required `pr-title`，覆盖标题编辑而不启动完整 CI。
- PR 变更分类必须独立输出前端、Rust 与共享/未知范围；纯前端路径不运行 Rust，纯 Rust 路径不运行前端，跨层、控制面、依赖与未知路径保守运行两者。`dev`/`main` push 与 `main` 手动运行始终全量，候选制品边界不变。
- `workflow_dispatch` 保留在 `ci.yml`，但仅 `main` 可进入重任务；手动聚合状态命名为 `manual-ci-gate`，不得满足自动 required gate。
- 手动恢复 CI 不运行 Provider trend release benchmark；相关自动代码路径仍保留现有 benchmark 覆盖，新 `performance.yml` 提供 main-only 手动 benchmark。
- 保留候选桌面/TUI 制品仅来自主仓 `main` 的 push/manual、签名环境和 release 选择合同；不改发布人工审批、CODEOWNERS 或 merge queue。
- `sync-upstream.yml` 使用仓库限定的 GitHub App 短期 token，删除 `github.token`/旧 PAT fallback，继续禁止 push、merge 和自动合并。
- 对已存在的同步 PR，保留一次受限的 `gh pr list` 查询；对新建 PR，必须捕获 `gh pr create` 的 stdout，严格接受且仅接受 `https://github.com/${GITHUB_REPOSITORY}/pull/<正整数>`，直接提取编号。不得在新建分支中重新 list；URL 或编号异常必须 fail-closed。
- 启用 Dependabot alerts/security updates、添加 npm/Cargo/Actions 配置、添加非 required CodeQL（JS/TS + Rust）及工作流 SHA pin policy；为现有 jobs 设置超时。

## Acceptance Criteria

- [ ] AC-01：docs-only PR 只运行轻量检查；前端-only 与 Rust-only PR 分别只运行所属验证；共享、未知、`dev`/`main` push 与 main 手动运行保持全量；普通 PR 自动 `ci-gate` 通过时不需要同 SHA 手动 `ci`。
- [ ] AC-02：手动非 `main` 运行在 checkout/前端/Rust/候选构建前失败，手动 `main` 恢复与候选场景均能完成并产生 `manual-ci-gate`。
- [ ] AC-03：`pr-title` 在 opened/synchronize/reopened/edited 上正确校验；Ruleset required contexts 最终为 `ci-gate` + `pr-title`。
- [ ] AC-04：自动相关 Rust 路径仍运行 benchmark；手动恢复不运行 benchmark；`performance.yml` 可独立运行 benchmark。
- [ ] AC-05：上游同步在缺失凭据、无效新建 PR URL 或无效编号时 fail-closed；有效 App token 可创建/更新人工 PR，创建时直接从严格验证的 `gh pr create` URL 取得编号，已有 PR 才查询列表，且不能 push、merge 或自动批准。`DIRTY`/`UNKNOWN` 继续准确要求人工处理。
- [ ] AC-06：Dependabot、CodeQL、SHA pinning 与 job timeout 配置通过本地契约和 GitHub 设置核验，现有候选/release 合同不回归。
- [ ] AC-07：本地 selftest、工作流静态检查和最新 PR required CI 通过；交付记录绑定同一 PR head SHA 与 `ci-gate` 运行。

## Scope and Decision Changes

| Date | Old / new decision | Affected acceptance criteria | Decision owner / resume condition |
|---|---|---|---|
| 2026-08-10 | Rust 测试并行化从本期实施改为后续独立实验 | AC-04、未来性能路线 | 用户确认；需单独非 required 实验、稳定性和 p50/p95 门槛 |
| 2026-08-11 | PR 从二元 docs/full 分类改为 frontend/Rust/shared 域分类；主干推送和手动候选构建仍全量 | AC-01、AC-07 | 用户确认；未知、CI 控制面、依赖和跨层路径 fail-closed 运行两端 |
| 2026-08-11 | 新建同步 PR 的编号从“创建后立即列表查询”改为“严格解析 `gh pr create` 返回 URL”；已有 PR 的列表查询不变 | AC-05、AC-07 | 用户确认；只接受当前 GitHub 仓库的正整数 PR URL，格式异常与冲突状态均 fail-closed |

## PENDING Review

- `AIO-PENDING-029`：明确排除；不读取、执行、修改、移动、删除或暂存 `upgrade-tui.command`。

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks implemented by main in the same session can remain PRD-only.
- Delegated worktree tasks also need `execution.md` before handoff and `delivery.md` before acceptance.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
