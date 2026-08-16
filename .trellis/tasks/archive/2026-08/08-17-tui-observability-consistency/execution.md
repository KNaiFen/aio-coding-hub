# 施工入口：统一 TUI 观测语义与窄屏布局

> 先调用 `$gkd-execute`。活动状态以 `python3 .trellis/scripts/task.py status .trellis/tasks/08-17-tui-observability-consistency` 为准；本文件只保存任务特有边界。

## 权威材料

1. `AGENTS.md`
2. `.trellis/tasks/08-17-tui-observability-consistency/prd.md`
3. `.trellis/tasks/08-17-tui-observability-consistency/design.md`
4. `.trellis/tasks/08-17-tui-observability-consistency/implement.md`
5. `docs/operations/multi-worktree/execution-and-delivery.md`
6. `.trellis/spec/aio-coding-hub/cross-layer/local-observer-tui-contract.md`
7. `.trellis/spec/aio-coding-hub/cross-layer/configured-model-routing-contract.md`
8. `src-tauri/crates/aio-tui/src/format.rs`
9. `src-tauri/crates/aio-tui/src/ui.rs`
10. 首次实施，无 `findings.md`

- 实施授权：已确认；2026-08-17，覆盖 PRD 的 R1-R6、AC1-AC12、规格和回归测试
- PENDING：无未解决条目
- 依赖：无；合并后的 Codex 372K/config-lifecycle 任务是独立子系统，不阻塞本任务

材料冲突或实施授权不明确时停止并报告 main，不用本摘要覆盖 PRD、设计或现行合同。

## 锁定边界

- 只消费现有 Observer 投影；不得重解析原始日志标记或扩展协议来绕过 TUI 格式问题。
- `provider_cross` 使用现有配置路由双行模型展示；未知、未来、畸形值继续 fail open。
- 31 列可用宽度下，本地/远程压缩模式优先于可变模型/effort 尾部，所有极窄宽度必须 grapheme-safe 且不 panic。
- skipped-only 不能显示或着色为成功直连；卡片、详情和状态使用同一派生路由语义。
- 可用性 bucket 必须是时间行和结果行两条逻辑行；本地时间、12 桶上限、汇总和滚动不变。
- 不改变网关路由、重试、价格、持久化、Observer 认证、桌面日志 UI 或发布配置。

## 实现自由度

- 可根据当前 `format.rs` 结构选择新增小型格式化 helper 或重构既有 helper，但不得建立第二套路由真相。
- 可选择内部派生结构和测试 fixture 组织方式；若改变现有帮助函数签名或卡片行结构，须在 `delivery.md` 解释兼容性影响。
- 可调整详情内的换行辅助，只要 PRD 的逻辑行和窄屏 AC 可判定成立。

## 范围

### 必须完成

- `implement.md` Work Package 1-5：规格、fixture、模型/压缩、路由结果、指标详情和供应商可用性布局。
- Work Package 6：范围复核、固定本地验证、交付文档、PR 和固定 head CI。
- 为 AC1-AC11 添加或修正精确的 formatter 与 `TestBackend` 回归覆盖。

### 允许修改

- `src-tauri/crates/aio-tui/src/format.rs`：模型、压缩、路由、缓存和详情格式。
- `src-tauri/crates/aio-tui/src/ui.rs`：供应商详情两行布局、语义色调和渲染测试。
- `.trellis/spec/aio-coding-hub/cross-layer/local-observer-tui-contract.md`：本任务完整 TUI 合同。
- `.trellis/spec/aio-coding-hub/cross-layer/configured-model-routing-contract.md`：移除 TUI 不变的陈旧表述并锁定跨供应商展示。
- 本任务目录内的 `delivery.md`、必要 findings/证据和 `task.json` 状态文件。

### 范围外

- Observer 协议、snapshot 投影、网关分类/路由/重试、数据库、桌面 UI、版本和发布流程。
- Codex 配置事务与 372K 开关任务的任何文件。
- 无关 TUI 重构、样式重做或新交互。

### 并行冲突

- 与 `08-17-codex-372k-context-window` 无主文件或语义冲突，可并行实施。
- 本任务所有 `aio-tui` 主文件由本 worktree 的唯一 writer 修改，不再拆分第二执行者。

## AC 与证据入口

| AC | 执行结果入口 | 需要的证据 |
|---|---|---|
| AC1-AC4 | `aio-tui/src/format.rs` 模型/压缩 helper 与 tests | local/remote、三种路由形态、0/1/24/31/32/80 宽度及 `provider_cross`/future 值断言 |
| AC5-AC6 | `aio-tui/src/format.rs` 路由派生、详情和 tone tests | skipped-only、skipped->sent、retry、switch 组合与 hop outcome 断言 |
| AC7 | `aio-tui/src/format.rs` 指标与详情 tests | 双缺失缓存、单/双 bucket、Session 复用和有效/无效速率 |
| AC8-AC10 | `aio-tui/src/ui.rs:provider_availability_detail_lines` 与 `TestBackend` tests | 两条逻辑行、24/32 列、四状态、多位计数、滚动及列表不回归 |
| AC11 | 两份 cross-layer spec | 规格与实现一致，删除 TUI 不变陈述 |
| AC12 | `delivery.md`、固定本地验证和 PR checks | 完整 base/head SHA、`local_ready`、同一 final head 的必需 CI |

完整可观察结果只写在 `prd.md`，这里不复制 Given/When/Then。

## 验证

- 本地允许：仅 `$gkd-local-verify` 要求的 `node scripts/check-local-verification.mjs --base <完整 base SHA>`。
- GitHub：自动 `ci-gate`、`pr-title`，以及该 PR 触发的 Rust fmt/Clippy/check/tests、规格和仓库合同检查。
- 人工/环境：无必需运行时验收；窄屏行为由 formatter 与 `TestBackend` 证据证明，用户后续体验反馈不阻塞交付。

不得运行 `AGENTS.md` 禁止的依赖安装、构建或测试。未运行项必须在 `delivery.md` 说明原因。

## 任务特有停止条件

- 任一必需值不在当前 bounded Observer 投影中，必须新增协议或持久化字段。
- 修复需要改变网关路由、重试、价格、认证、安全、迁移或发布行为。
- 当前规格或上游代码使五行卡片、旧 Observer fail-open、详情滚动或 provider 列表兼容性无法同时满足。
- 需要修改范围外的重要模块、公共接口或与另一活动 worktree 产生真实冲突。
- CI 失败无法证明属于本任务且没有可靠任务内修法。

通用阻塞、交付和恢复命令见执行专题。停止时先持久化证据和恢复条件，再暂停。

## 当前返工

- 未解决 finding：无
- 本轮只处理：首次实施
- 保持不变：PRD 的非目标、Observer 数据边界和现有网关行为
