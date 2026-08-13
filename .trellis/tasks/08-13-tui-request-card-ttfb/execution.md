# 施工入口：TUI 请求卡片改用首字时间与紧凑路由计数

> 本文件是独立执行 session 的唯一施工入口。完整需求和验收标准以同目录 `prd.md` 为准。

## 快速定位

- 权威任务目录：`.trellis/tasks/08-13-tui-request-card-ttfb/`
- 绝对 worktree：`/Users/knaifen/Documents/Codex/aio-coding-hub/08-13-tui-request-card-ttfb`
- 分支：`fix/tui-request-card-ttfb`
- PR target：`main`
- 完整 base SHA：`875ff441c5ba9f1a7f235ad95dadb945a41bba61`
- 规划提交：`bd91552393f36419ce215d9de283b7519c0efb07`
- 实施授权：已确认（2026-08-13，范围见 `prd.md`）
- PR：未创建；由执行 session 尽早创建 Draft PR
- 当前唯一写者：独立执行 session（本 worktree 的唯一写者）
- 当前阶段：in_progress；允许按锁定范围开工
- PENDING：已审阅，当前无未解决条目

## 开工前阅读顺序

1. 仓库根 `AGENTS.md`。
2. 本文件与同目录 `prd.md`。
3. `.trellis/spec/aio-coding-hub/cross-layer/index.md` 中 “When changing the local observer or standalone TUI” 与 Quality Check。
4. `.trellis/spec/aio-coding-hub/cross-layer/local-observer-tui-contract.md`，尤其路由计数语义和请求卡片行为。
5. `src-tauri/crates/aio-tui/src/format.rs` 中 `request_card_lines`、`route_result`、`detail_lines` 及邻近测试；这是即将修改的确切代码，必须由执行 session 自己完整读取相关范围。
6. `docs/operations/multi-worktree-delivery.md` 的执行 session、交付与暂停规则。

## 强制 Preflight

开工前逐项确认并把事实写入 `delivery.md`：

```bash
test "$(pwd)" = "/Users/knaifen/Documents/Codex/aio-coding-hub/08-13-tui-request-card-ttfb"
test "$(git branch --show-current)" = "fix/tui-request-card-ttfb"
BASE_SHA="875ff441c5ba9f1a7f235ad95dadb945a41bba61"
test "$(git merge-base "$BASE_SHA" HEAD)" = "$BASE_SHA"
git status --short --branch
```

还必须确认：本文件已回填非占位的完整规划提交、该提交存在、`task.json.status` 为 `in_progress`、当前唯一写者已登记为独立执行 session、实施授权为已确认、工作树无来源不明修改。任一项不满足即暂停并报告 main。

## 锁定边界

- 必须完成：`prd.md` 的 AC-01 至 AC-06。
- 允许修改：
  - `src-tauri/crates/aio-tui/src/format.rs`
  - `.trellis/tasks/08-13-tui-request-card-ttfb/delivery.md`
  - 如 PR/head/阶段事实变化，允许同步本任务的 `task.json`、`execution.md` 与 `.trellis/tasks/README.md`，但不得改写 main 锁定的需求、决定或 AC。
- 实现自由度：可选择私有卡片专用 formatter 或等价的局部实现，但必须避免改变共享 `route_result` 的现有完整文案。
- 明确禁止：Observer 协议与 snapshot、状态行语义、详情字段语义、网关转发/重试逻辑、其他任务目录、依赖/锁文件、生成文件、版本与发布配置。
- 不需要更新现行 TUI 合同：现有合同已规定两个计数独立且请求卡片使用语义 route 行；本任务仅调整该行的展示选择和缩写。若实现发现合同需变更，停止并交 main 判断。

## 建议施工顺序

1. 在 `request_card_lines` 中将卡片时间来源从 `duration_ms` 切换为 `ttfb_ms`，保持 `None -> —` 与 `format_duration`。
2. 为请求卡片提供局部紧凑 route 文案，覆盖 `直连`、`未上游`、仅切换、仅重试、两者都有；共享 `route_result` 不改语义。
3. 更新现有卡片测试，使 `duration_ms=2000`、`ttfb_ms=500` 明确断言 `500ms`；新增或扩展测试覆盖 `切1/重3`、缺失 TTFB 和详情/共享文案不回归。
4. 审查完整 diff，确认输出速率、Token、费用、模型和 provider/folder 行无无关变化。

## 验证边界

本地只允许运行：

```bash
node scripts/check-cloud-only-verification.mjs
node scripts/check-cloud-only-verification.selftest.mjs
python3 ./.trellis/scripts/task.py validate 08-13-tui-request-card-ttfb
git diff --check
```

- 不运行 Cargo、rustfmt、Clippy、Rust tests、构建、Tauri、package-manager 脚本、依赖安装、开发服务器或手动 workflow dispatch。
- Rust 格式、Clippy、单元测试和相关编译由 PR 自动 CI 执行。
- 创建指向 `main` 的 Draft PR，标题遵守仓库约定；常规验证只等待自动触发的 `ci-gate` 与 `pr-title`，不得额外手动触发 `ci`。

## 交付与暂停

1. 按锁定范围实现、提交并推送 `fix/tui-request-card-ttfb`；不得推送 `main`。
2. 填写同目录 `delivery.md`，记录实际实现、关键符号、AC 证据、偏移、本地验证、云端检查、兼容性/安全/回滚、完整 PR head/base SHA 和对应 `ci-gate` 链接。
3. PR 最新完整 head 的必需检查及该范围相关编译绿色后，将 PR 标记 Ready for review。
4. 更新活动索引为“等待 main 验收”，停止写入并通知 main。不得开启自动合并、合并 PR、运行 `/trellis:finish-work`、归档任务、删除 worktree 或分支。

## 停止条件

- preflight 的路径、分支、base、规划提交、任务状态、授权、唯一写者或工作树归属不一致。
- 实现需要越过允许文件、改变详情/状态行/协议/公共接口，或无法同时满足计数独立语义与紧凑显示。
- CI/环境失败无法证明属于本任务且没有可靠的范围内修法。

阻塞时如实更新 `delivery.md`：记录证据、最后安全提交、工作树状态、受影响 AC、决定归属和恢复条件，然后暂停；没有 PR/CI 时明确写“尚未创建/未触发及原因”。
