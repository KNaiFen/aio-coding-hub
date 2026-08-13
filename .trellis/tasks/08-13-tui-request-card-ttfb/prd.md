# TUI 请求卡片改用首字时间与紧凑路由计数

## Plan Status

- Implementation authorization: 已确认。2026-08-13 用户明确要求实施，并指定采用多 worktree、多 session 协作模式。
- Confirmation date and summary: 2026-08-13。请求卡片的时间摘要由总耗时改为首字时间；请求卡片中的 `切换1/重试3` 改为 `切1/重3`。
- Confirmed coverage: 仅 standalone TUI 请求卡片的 route 行及其单元测试；详情页、状态行、Observer 协议和快照投影保持现状。
- Planning revision: `bd91552393f36419ce215d9de283b7519c0efb07`（冻结需求、AC、执行边界与上下文清单的源规划提交）。
- Execution route: delegated worktree `/Users/knaifen/Documents/Codex/aio-coding-hub/08-13-tui-request-card-ttfb`。
- Migrated from direct-main record: 无。本任务从一开始即采用 Trellis delegated worktree，不存在 main 检出的实现改动。

## Material Facts, Assumptions, and Open Questions

| Item | Source | Status / closure condition |
|---|---|---|
| `ObserverRequest` 已提供 `duration_ms` 和 `ttfb_ms`；本需求无需协议或快照字段变更。 | `aio-observer-protocol` 与 observer snapshot 现行代码 | confirmed |
| `request_card_lines` 当前使用 `duration_ms`；详情页分别显示总耗时与首字时间。 | `src-tauri/crates/aio-tui/src/format.rs` | confirmed |
| `route_result` 同时供状态行、详情和卡片使用，直接缩写会扩大用户要求的“请求卡片”范围。 | `src-tauri/crates/aio-tui/src/format.rs` 调用点 | confirmed；卡片应使用局部紧凑格式，其他调用保留完整文案 |
| 运行中请求可能没有 TTFB。 | Observer 投影与现有 Optional 字段语义 | confirmed；沿用缺失指标显示 `—` 的现行降级规则 |
| 本地常规 checkout 禁止 Cargo、Rust test、格式化、构建和依赖安装。 | 根 `AGENTS.md` 与 cloud-only verification contract | confirmed；Rust 编译/测试交由 PR CI |
| 材料性未决问题 | 用户决定与当前代码调查 | 无 |

## Goal

让请求卡片的路由行更贴近首屏诊断需求：时间位置显示首字时间，并用更紧凑的中文缩写呈现供应商切换和重试次数，同时不改变详情页或其他 TUI 输出的现有语义。

## Requirements

1. 请求卡片 route 行中的时间必须读取 `ttfb_ms`，不得再读取 `duration_ms`。
2. `ttfb_ms` 缺失时继续显示 `—`；负值、毫秒和秒级格式继续复用现有 `format_duration` 行为。
3. 仅请求卡片使用紧凑路由摘要：
   - 有上游且无切换/重试：`直连`
   - 仅有供应商切换：`切N`
   - 仅有重试：`重N`
   - 两者均非零：`切N/重N`
   - 尚未上游：`未上游`
4. `provider_switch_count` 与 `retry_count` 必须继续独立显示和计算，不得从另一个计数推导。
5. 输出速率 `t/s` 的显示条件和计算来源保持不变；只替换它前面的卡片时间指标。
6. 状态行和详情页继续使用现有 `route_result` 完整文案（例如 `切换1/重试3`）。详情页继续分别显示 `耗时 duration_ms` 与 `首字 ttfb_ms`。
7. 不修改 Observer 协议、快照投影、日志持久化、转发、重试、供应商选择或费用/Token 计算。
8. 在 `src-tauri/crates/aio-tui/src/format.rs` 的邻近单元测试中覆盖时间来源、紧凑计数的组合语义和详情/共享文案不回归。

## Acceptance Criteria

- [ ] AC-01：当同一请求的 `duration_ms=2000`、`ttfb_ms=500` 时，请求卡片 route 行显示 `500ms`，不显示 `2.0s`。
- [ ] AC-02：请求卡片对切换/重试计数分别显示 `切1`、`重3`，两者均存在时精确显示 `切1/重3`；直连和未上游文案保持现状。
- [ ] AC-03：请求卡片缺少 `ttfb_ms` 时使用 `—`，且可选的输出速率仍按现行规则追加。
- [ ] AC-04：现有共享 `route_result` 与详情页仍显示完整的 `切换N/重试N`，详情的总耗时和首字时间字段不变。
- [ ] AC-05：PR 相对 `origin/main` 的产品代码变更限于 `src-tauri/crates/aio-tui/src/format.rs`，另允许本任务 Trellis 记录和活动索引；不包含协议、快照或其他产品模块改动。
- [ ] AC-06：仓库允许的本地合同、Trellis validate 和 `git diff --check` 通过；PR 最新完整 head 的必需 `ci-gate`、`pr-title` 及该范围选中的 Rust/相关检查为绿色。

## Scope and Decision Changes

| Date | Old / new decision | Affected acceptance criteria | Decision owner / resume condition |
|---|---|---|---|
| 2026-08-13 | 初始锁定：只缩写请求卡片，不改变详情页或状态行的完整文案。 | AC-02、AC-04、AC-05 | 用户；若实现需要扩大输出范围，暂停并交 main 重新确认 |

## PENDING Review

- `PENDING.md` 已于 2026-08-13 完整审阅；当前无 `pending` 或 `planned` 条目，无冲突、依赖或本任务候选项。

## Planning Scope

- 本任务属于单模块、低风险显示变更，无公共接口、数据流、迁移或架构取舍，因此不创建 `design.md` 或 `implement.md`。
- 实施顺序、允许文件、验证和停止条件集中写在 `execution.md`。
