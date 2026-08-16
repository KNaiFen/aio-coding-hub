# 交付报告：统一 TUI 观测语义与窄屏布局

> 只写实际实现和证据，不复制计划，不缓存 PR head/base/CI URL。实时候选以 GitHub PR 和 `task.py status .trellis/tasks/08-17-tui-observability-consistency` 为准。

## 结果

- 结果：等待验收
- PR：https://github.com/KNaiFen/aio-coding-hub/pull/157
- 执行者：tui-observability-executor
- 一句话结果：独立 TUI 现在在窄屏保留压缩模式，统一跨供应商模型与路由结果语义，并把供应商可用性时间和结果拆成可滚动的两行布局。

## 实际实现

### 用户可见行为

- 请求卡片在本地/远程压缩、普通模型、同模型路由和跨模型路由下优先保留完整压缩模式；极窄宽度继续按 grapheme 安全截断。
- 有效 `provider_cross` 请求使用源模型到有效模型的两行展示，详情显示 `跨供应商`；未来或畸形路由仍回退普通模型行。
- 路由摘要同时显示切换、跳过、重试和实际请求数。仅跳过候选时不再显示或着色为成功直连，详情明确说明未发送并标记每个 hop 结果。
- 缓存双缺失显示 `C —`；详情新增 Session 复用和既有规则计算的输出速率。
- 每个供应商可用性 bucket 固定为时间行和状态/计数行，24/32 列渲染保持 `成N 败N` 在同一结果行。

### 内部机制

- `format.rs` 使用一个 bounded route presentation 派生 skipped、sent attempts、retry 和 switch 证据，卡片、详情、statusline 和 tone 共同消费它；route 缺失时保留旧计数 fallback。
- 模型 formatter 使用 display-width-aware suffix helper，在完整行放不下时先为压缩标签预留宽度，再截断可变模型/effort。
- availability formatter 仍使用 host-local time、原顺序和十二桶上限，仅把每桶映射为两条逻辑行；既有 detail scroll 负责垂直展开。

## AC 证据

| AC | 结果 | 证据 |
|---|---|---|
| AC1 | 已实现，待 GitHub 验证 | `format.rs:compaction_suffix_remains_visible_across_all_model_card_paths` 覆盖 local/remote 和三种模型路径的 31/32 列断言。 |
| AC2 | 已实现，待 GitHub 验证 | 同一矩阵覆盖 0/1/24/31/32/80；所有行断言不超过 display width。 |
| AC3 | 已实现，待 GitHub 验证 | `format.rs:provider_cross_route_uses_shared_model_and_policy_presentation` 覆盖 terminal/active 箭头模型和 `跨供应商`。 |
| AC4 | 已实现，待 GitHub 验证 | `format.rs:future_route_policy_falls_open_without_a_target_or_policy_label` 与既有 malformed fixture 覆盖 fail open。 |
| AC5 | 已实现，待 GitHub 验证 | `format.rs:structured_route_summaries_keep_skip_retry_switch_and_sent_counts` 覆盖 skipped-only 和 switch/skip/retry/sent 组合。 |
| AC6 | 已实现，待 GitHub 验证 | formatter 断言未发送与 hop outcome；`ui.rs:request_card_line_kinds_control_color_and_selection` 断言 skipped-only status/route 均为 warning。 |
| AC7 | 已实现，待 GitHub 验证 | `format.rs:request_card_cache_keeps_unknown_distinct_from_zero_and_detail_keeps_evidence` 覆盖 cache、Session 复用和有效速率；既有 rate 测试覆盖无效规则。 |
| AC8 | 已实现，待 GitHub 验证 | `ui.rs:provider_availability_detail_uses_local_time_without_a_timezone_suffix` 逐对断言时间/结果边界及四种状态。 |
| AC9 | 已实现，待 GitHub 验证 | `ui.rs:provider_availability_detail_keeps_result_lines_intact_at_narrow_widths_and_scrolls` 覆盖 24/32 列、多位计数和 scroll。 |
| AC10 | 已实现，待 GitHub 验证 | 保留并扩展既有 local-time、arrow、fallback、optional field 和 provider card 回归测试。 |
| AC11 | 已实现 | 两份 cross-layer contract 已同步；陈旧的 TUI 不变表述已删除。 |
| AC12 | 本地通过，GitHub 待最终 head | 固定 runner 返回 `local_ready`；云端 checks 在实现提交后触发。 |

## 关键位置

| 文件或符号 | 实际变化 | 设计原因 |
|---|---|---|
| `src-tauri/crates/aio-tui/src/format.rs:truncate_display_with_suffix` | 预留压缩语义并安全截断模型 lead。 | bounded 状态比可变模型尾部优先。 |
| `src-tauri/crates/aio-tui/src/format.rs:route_presentation` | 统一 route 计数、sent 事实和 tone 输入。 | 避免卡片、详情和 statusline 产生第二套路由真相。 |
| `src-tauri/crates/aio-tui/src/format.rs:detail_lines` | 增加 route outcome、Session 复用和输出速率。 | 只消费 Observer 已投影证据。 |
| `src-tauri/crates/aio-tui/src/ui.rs:provider_availability_detail_lines` | 每桶输出时间与结果两条逻辑行。 | 窄屏不混排独立语义。 |
| `.trellis/spec/aio-coding-hub/cross-layer/local-observer-tui-contract.md` | 锁定本任务完整 TUI 行为。 | 规格与 shipped formatter 保持一致。 |

## 计划偏移

- 无用户可见偏移。Observer hop 的 `ok` 是必填布尔值而非可选值，因此 active 且无状态/错误的 `ok=false` hop 显示 `进行中`，terminal 的 `ok=false` hop 显示 `失败`；不需要协议变化。

## 验证

| 类型 | 命令或检查 | 结果 | 说明 |
|---|---|---|---|
| 本地 | `node scripts/check-local-verification.mjs --base <登记的完整 base SHA>` | 通过 | `local_ready`；检查 runner/cloud-only 合同、自测、committed/index/worktree diff、空白和变更 Node 语法，本任务无变更 Node 文件。 |
| GitHub | `ci-gate`、`pr-title` 及适用 Rust jobs | 等待 | 依赖、格式、Clippy、check、tests 和构建只由最终 PR head 的 Actions 验证。 |
| 人工 | 无 | 未运行 | 窄屏与滚动行为由 formatter 和 `TestBackend` 回归覆盖，按任务合同无需运行时人工验收。 |

## 合同与影响

- 测试：扩展 `aio-tui/src/format.rs` formatter tests 和 `aio-tui/src/ui.rs` `TestBackend` tests。
- 现行文档与机器合同：更新 local Observer/TUI 与 configured-model-routing 两份 cross-layer contract。
- API、兼容性与迁移：无协议、公共 API 或迁移变化；route 缺失与未知可选值继续 fail open。
- 数据、配置、安全与隐私：无数据库、配置、认证、凭据或隐私边界变化。
- 发布与回滚：无版本和发布配置变化；回退本任务提交即可。

## 风险与审查重点

- 剩余风险：本地合同禁止 Rust 编译、格式和测试；这些结果必须以最终 PR head 的 GitHub Actions 为准。
- main 重点审查：`format.rs:route_presentation` 的 skipped/sent fallback 与 `ui.rs:provider_availability_detail_lines` 的逻辑行/scroll 保持。
- 未完成项：等待最终 PR head 的适用 GitHub checks。

## 阻塞快照

无。
