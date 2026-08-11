# GitHub Actions 流程治理与提速

## Plan Status

- Implementation authorization: confirmed by user
- Confirmation date and summary: 2026-08-10 实施初版；2026-08-11 用户确认扩展为 PR 按前端、Rust 与共享路径分别验证，主干推送与手动候选构建保持全量。
- Confirmed coverage: workflow files, CI contracts/selftests, current operational docs, GitHub Actions settings, upstream-sync credentials, Dependabot, CodeQL, and CI path-domain classification
- Planning revision: v2; add domain-specific PR validation after the initial implementation
- Execution route: delegated worktree, main-owned implementation and acceptance
- Migrated from direct-main record: none

## Material Facts, Assumptions, and Open Questions

| Item | Source | Status / closure condition |
|---|---|---|
| 最近 200 次 CI 中 48 组同 SHA 的 PR + workflow_dispatch，45 组在 PR 完成前重叠 | Actions audit | confirmed；本期目标是消除无意重复 |
| Rust 测试显式使用 `CARGO_BUILD_JOBS=1` 与 `--test-threads=1`，测试存在共享环境/缓存状态 | repository source and history | confirmed；本期不改并行度 |
| 默认 `GITHUB_TOKEN` 为 read，且不允许创建/批准 PR；同步当前因权限失败 | GitHub Actions logs/settings | confirmed；改用最小权限 App token |
| `upgrade-tui.command` 为 `AIO-PENDING-029` 未跟踪项 | `PENDING.md` and worktree status | confirmed；本任务不得读取、执行、修改、移动或删除 |

Do not start implementation with a material open question. Record approved scope or decision changes in this file with their affected acceptance criteria and confirmation.

## Goal

消除 PR 与手动 CI 重复运行，分离 required gate，补齐安全自动化并修复上游同步令牌边界。

## Requirements

- 自动 PR/push CI 继续产生唯一 required `ci-gate`；PR 标题校验迁移为独立 required `pr-title`，覆盖标题编辑而不启动完整 CI。
- PR 变更分类必须独立输出前端、Rust 与共享/未知范围；纯前端路径不运行 Rust，纯 Rust 路径不运行前端，跨层、控制面、依赖与未知路径保守运行两者。`dev`/`main` push 与 `main` 手动运行始终全量，候选制品边界不变。
- `workflow_dispatch` 保留在 `ci.yml`，但仅 `main` 可进入重任务；手动聚合状态命名为 `manual-ci-gate`，不得满足自动 required gate。
- 手动恢复 CI 不运行 Provider trend release benchmark；相关自动代码路径仍保留现有 benchmark 覆盖，新 `performance.yml` 提供 main-only 手动 benchmark。
- 保留候选桌面/TUI 制品仅来自主仓 `main` 的 push/manual、签名环境和 release 选择合同；不改发布人工审批、CODEOWNERS 或 merge queue。
- `sync-upstream.yml` 使用仓库限定的 GitHub App 短期 token，删除 `github.token`/旧 PAT fallback，继续禁止 push、merge 和自动合并。
- 启用 Dependabot alerts/security updates、添加 npm/Cargo/Actions 配置、添加非 required CodeQL（JS/TS + Rust）及工作流 SHA pin policy；为现有 jobs 设置超时。

## Acceptance Criteria

- [ ] AC-01：docs-only PR 只运行轻量检查；前端-only 与 Rust-only PR 分别只运行所属验证；共享、未知、`dev`/`main` push 与 main 手动运行保持全量；普通 PR 自动 `ci-gate` 通过时不需要同 SHA 手动 `ci`。
- [ ] AC-02：手动非 `main` 运行在 checkout/前端/Rust/候选构建前失败，手动 `main` 恢复与候选场景均能完成并产生 `manual-ci-gate`。
- [ ] AC-03：`pr-title` 在 opened/synchronize/reopened/edited 上正确校验；Ruleset required contexts 最终为 `ci-gate` + `pr-title`。
- [ ] AC-04：自动相关 Rust 路径仍运行 benchmark；手动恢复不运行 benchmark；`performance.yml` 可独立运行 benchmark。
- [ ] AC-05：上游同步在缺失凭据时 fail-closed；有效 App token 可创建/更新人工 PR，不能 push、merge 或自动批准。
- [ ] AC-06：Dependabot、CodeQL、SHA pinning 与 job timeout 配置通过本地契约和 GitHub 设置核验，现有候选/release 合同不回归。
- [ ] AC-07：本地 selftest、工作流静态检查和最新 PR required CI 通过；交付记录绑定同一 PR head SHA 与 `ci-gate` 运行。

## Scope and Decision Changes

| Date | Old / new decision | Affected acceptance criteria | Decision owner / resume condition |
|---|---|---|---|
| 2026-08-10 | Rust 测试并行化从本期实施改为后续独立实验 | AC-04、未来性能路线 | 用户确认；需单独非 required 实验、稳定性和 p50/p95 门槛 |
| 2026-08-11 | PR 从二元 docs/full 分类改为 frontend/Rust/shared 域分类；主干推送和手动候选构建仍全量 | AC-01、AC-07 | 用户确认；未知、CI 控制面、依赖和跨层路径 fail-closed 运行两端 |

## PENDING Review

- `AIO-PENDING-029`：明确排除；不读取、执行、修改、移动、删除或暂存 `upgrade-tui.command`。

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks implemented by main in the same session can remain PRD-only.
- Delegated worktree tasks also need `execution.md` before handoff and `delivery.md` before acceptance.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
