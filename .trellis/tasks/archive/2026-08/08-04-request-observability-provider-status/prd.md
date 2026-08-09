# 请求可观测性与供应商状态改进

## Goal

分五个可独立验收、独立核对主线、独立提交 PR 的交付，改善 TUI 请求日志、AIO/TUI 供应商即时状态、macOS 托盘悬浮窗、请求日志检索以及 Token 生成速度口径。

## Child Deliverables

1. `08-04-tui-log-reasoning-effort`：TUI 每条 Codex 请求都显示思考强度，路由前后分别显示对应强度。
2. `08-04-current-status-bucket-latest-request`：AIO 与 TUI 的当前状态格按该格最新请求结果着色。
3. `08-04-aio-float-status-visuals`：重排悬浮窗状态条、计数和状态字，并落实半透明液态背景。
4. `08-04-request-log-filters-pagination-time`：增加错误快捷筛选、流内错误筛选、页码选择和分钟级时间范围。
5. `08-04-token-speed-semantics`：用代码证据确定 Token 速度异常根因并统一修正所有请求速度消费者。

## Shared Requirements

- 每个子任务在独立分支/worktree 完成开发、允许的本地验证和提交。
- 提交 PR 前必须获取并记录最新 `origin/main` 提交；只检查 `origin`，不检查 `upstream`。
- 若最新主线出现相关实现，必须比较功能目标、实现方式、接口行为和最终效果。
- 可兼容时基于最新 `origin/main` 整合、解决冲突并重新验证后再提交 PR。
- 根本冲突且必须二选一时，不覆盖主线、不丢弃候选成果；保留 worktree、分支和提交，将候选 PR 标为“待决策”，继续其他不受影响子任务。
- 最终统一汇报全部待决策项；若没有其他可推进子任务则立即汇报。汇报须包含任务、分支/提交、核对的主线 SHA、冲突功能/文件、不能共存原因、方案影响和建议。
- 当前 `PENDING.md` 没有未解决条目，因此本批无需并入额外延期事项。
- 本地不运行 Cargo、rustfmt、Clippy、Rust 测试、Specta 生成或 Tauri 原生构建；Rust/绑定/原生验证交给 GitHub Actions。本地仅运行 Node.js、TypeScript、前端测试和 Vite 构建。

## Acceptance Criteria

- [ ] 五个子任务均有经过用户确认的 PRD；复杂子任务另有 `design.md` 和 `implement.md`。
- [ ] 每个可推进子任务均在独立分支/worktree 完成，保留独立验证和主线核对证据。
- [ ] 每个候选 PR 提交前记录对应的最新 `origin/main` SHA 和相关实现审计结论。
- [ ] 所有可推进子任务完成后，统一汇报已提交 PR、验证结果和待决策候选。

## Open Questions

- 子任务中的产品歧义按一次一个问题与用户确认；答案立即回写对应 PRD。
