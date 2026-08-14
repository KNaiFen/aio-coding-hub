# 施工入口：TUI 请求时间与 CLI 监听切换修复

> 按本文件和同目录权威任务材料施工。完成实现、PR、CI 和 `delivery.md` 后暂停等待 main 验收。你是此 sibling worktree 的唯一写者；不得再创建 worktree、派生实施任务或与其他 session 同时写入。

## 快速定位

- 任务目录：`.trellis/tasks/08-14-tui-duration-cli-listen-fix/`
- Worktree：`/Users/knaifen/Documents/Codex/aio-coding-hub/tui-duration-cli-listen-fix`
- 分支：`fix/tui-duration-cli-listen`
- PR base：`main`
- 完整 base SHA：`1b218897c09894cfb5aff796761eb8004ad6e53f`
- 规划提交：`5419ccf64ba73387f999133389ab3d347e63270c`
- 实施授权：已确认，2026-08-14；覆盖 PRD locked decisions、范围与 AC。
- PR：尚未创建；preflight 后尽早创建 Draft PR。
- PENDING：无未解决条目。
- 当前唯一写者：独立 execution session。
- 当前阶段：已完成规划交接；execution session 仅在 preflight 全部通过后施工。

## 阅读顺序

1. 根 `AGENTS.md`、`.trellis/workflow.md`、`docs/operations/multi-worktree-delivery.md`。
2. 本文件。
3. `prd.md`、`design.md`、`implement.md`。
4. `implement.jsonl` 和 `check.jsonl` 中的现行 specs。
5. 与当前阶段直接相关的 source/tests；`delivery.md` 首次仅作模板。

材料冲突时依次以 PRD 用户决定/AC、设计、实施计划、本入口为准；不根据聊天记录猜测。

## 开工 Preflight

```bash
test "$(pwd)" = "/Users/knaifen/Documents/Codex/aio-coding-hub/tui-duration-cli-listen-fix"
test "$(git branch --show-current)" = "fix/tui-duration-cli-listen"
BASE_SHA="1b218897c09894cfb5aff796761eb8004ad6e53f"
test "$(git merge-base "$BASE_SHA" HEAD)" = "$BASE_SHA"
test -f .trellis/tasks/08-14-tui-duration-cli-listen-fix/task.json
test -f .trellis/tasks/08-14-tui-duration-cli-listen-fix/prd.md
test -f .trellis/tasks/08-14-tui-duration-cli-listen-fix/design.md
test -f .trellis/tasks/08-14-tui-duration-cli-listen-fix/implement.md
test -f .trellis/tasks/08-14-tui-duration-cli-listen-fix/execution.md
git cat-file -e "5419ccf64ba73387f999133389ab3d347e63270c^{commit}"
```

再确认 `task.json.status=in_progress`、授权 confirmed、规划 SHA 已回填、工作树无来源不明修改、唯一写者为 execution session。任一失败即停止并报告 main；不得自行 rebase、修改任务边界或接管其他 worktree。

## 必须达到

- Active TUI request card 显示实时 duration；Terminal 显示 TTFB。
- 运行中 gateway 双向切换 listen mode 不自锁，mutation 有界完成。
- LAN 保存成功后同一次交互立即显示 token，不要求切 tab。
- 保存 pending 有反馈；null/error 回滚；结束后可切回 localhost。
- token reveal 单一 owner 跨 tab 存活，后端一次性/明文安全语义不变。
- 相关 frontend/Rust regression tests、现行 specs 和 full-scope CI 通过。

## 允许修改

- `src-tauri/crates/aio-tui/src/format.rs` 及其邻近 tests。
- `src-tauri/src/app/settings_service.rs`、必要的 gateway lifecycle 测试辅助；只有确有必要时才改 `gateway_lifecycle_lock.rs`。
- `src/components/cli-manager/NetworkSettingsCard.tsx` 及其 tests。
- `src/pages/CliManagerPage.tsx`、`src/pages/cli-manager/useCliManagerPageDataModel.ts`、`src/components/cli-manager/tabs/GeneralTab.tsx` 及直接相关 tests。
- 可新增小型 page-level token controller/dialog 组件或 hook，但仅当它消除真实的生命周期/重复逻辑。
- `.trellis/spec/aio-coding-hub/cross-layer/index.md`、`local-observer-tui-contract.md`、新 `gateway-listen-token-contract.md`。
- 本任务目录和 `.trellis/tasks/README.md` 的实时登记字段。

## 明确禁止

- gateway token public IPC、generated bindings、command registry、token 算法/digest/长度、route auth 或 loopback 例外。
- settings schema/migration、listen mode 枚举/解析、CLI proxy 内容和 managed profile/catalog 行为。
- observer protocol/snapshot schema、详情页、TTFB 统计、output rate、路由计数和 TUI scope。
- 依赖/lockfile、release/candidate/signing/performance/CodeQL/workflow 语义。
- `chore/trim-redundant-tests` worktree 的任何文件；若其先合并导致 index 冲突，暂停报告 main，不自行猜测合并。
- 历史 docs、archive、PENDING、真实 token 或凭据。

## 实施与验证

严格按 `implement.md` 的 0 至 7 顺序。每阶段先读即将修改的确切代码并确认完成信号；需要公共 API、安全语义、数据迁移或范围外文件时停止。

本地只运行计划列出的无依赖 Node 合同、task validate 和 diff check。禁止 package manager、Vitest、Cargo/Rustfmt/Clippy、构建、生成、dev server、Tauri、签名或打包。

PR 只等待自动检查；本任务预期 full scope。CI 监控 3-5 分钟一次、最长 60 分钟，始终绑定同一完整 head。

## 交付与暂停

- 尽早 Draft PR，完成后 Ready for review。
- 每个逻辑切片独立提交；提交前检查 dirty paths，不 amend、不推 `main`。
- `delivery.md` 记录实际符号、每条 AC、完整 head/base、scope/jobs、CI URL、局部验证、手工验证状态、偏移和风险。
- 绿色后停止写入并通知 main；不得 merge、archive、清理 worktree/branch 或 `/trellis:finish-work`。

## 必须停止并报告 main

- 路径、分支、base、规划 SHA、授权、task status、唯一写者或 dirty ownership 不一致。
- 需要改变 token backend semantics/public API、route auth、settings schema、observer protocol 或 CLI proxy 行为。
- 无法在不破坏 lifecycle 串行性的前提下移除自锁。
- 一次性 token 无法在 tab 卸载后可靠展示，且唯一修法要求改成后端重复 reveal。
- CI 失败无法归因到任务范围，或活动测试清理 PR 造成任务索引以外的冲突。

## 初始交付状态

- 结果：尚未开始（已交接，等待 execution session 完成 preflight）。
- PR/head/CI：尚未创建 / 尚未提交 / 未触发。
- execution session 当前为唯一写者；交付后暂停。
