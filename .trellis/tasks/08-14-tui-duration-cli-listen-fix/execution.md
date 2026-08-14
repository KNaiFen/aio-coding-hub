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
- PR：[#147](https://github.com/KNaiFen/aio-coding-hub/pull/147)（Draft；Round 1 返工代码 head `c7800118876f79412236783c4abe260013d606a3`，记录提交后等待固定 head CI）。
- PENDING：无未解决条目。
- 当前唯一写者：独立 execution session。已从 main 的交接 head `52232d72993f83be4ba2bd04b7e11171616a06cf` 完成 preflight 并恢复写入；等待最新固定 head CI 后再次暂停。
- 当前阶段：Round 1 的 F-001/F-002 已修复，F-003 已通过 merge commit `08ac062af5454cf09a811ba71d597430c513c33b` 集成 `origin/main@0ae7f03abaa37c7021fdf8718373e27fe61f62fd`；正在更新交付记录并等待自动 CI。

## 阅读顺序

1. 根 `AGENTS.md`、`.trellis/workflow.md`、`docs/operations/multi-worktree-delivery.md`。
2. 本文件。
3. `prd.md`、`design.md`、`implement.md`。
4. `implement.jsonl` 和 `check.jsonl` 中的现行 specs。
5. 与当前阶段直接相关的 source/tests；`delivery.md` 首次仅作模板。
6. `findings.md`；返工时保留 main 原始意见，只填写“执行回应”。

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

## Round 1 返工恢复步骤

1. `git fetch origin` 后重新执行开工 preflight，确认绝对路径、分支、`task.json.status=in_progress`、规划提交存在、原登记 base 仍是祖先、工作树干净，并且本地 `HEAD`、远端任务分支和 PR head 都是 main 写入 findings 后的同一完整交接 head。任一不一致即停止报告 main。
2. 使用普通 merge 同步 `origin/main@0ae7f03abaa37c7021fdf8718373e27fe61f62fd`：`git merge --no-edit origin/main`。禁止 rebase、force-push 或 cherry-pick PR #146/#148。
3. 解决 `.trellis/tasks/README.md` 集成冲突时，保留 `main` 中 `08-14-trim-redundant-tests` 的归档条目；保留 `08-14-tui-duration-cli-listen-fix` 的活动任务行并更新阶段、PR/head 和唯一写者。不得把已归档测试清理任务恢复为活动状态。
4. 保留 `main` 已合入的 `contracts` workflow 拓扑和测试清理结果，不恢复旧 `docs-contract`/`support-contract` 拆分，不重新加入已删除的冗余测试或孤立文件。其他非冲突 main 变化全部保留。
5. 按 `findings.md` 修复 F-001：保存成功的 reveal 意图不得被保存前的旧 flight 吞掉；增加 deferred 初始 reveal 与 LAN 保存重叠的回归测试，同时保持单一 owner、同阶段去重和一次性 token 安全语义。
6. 按 `findings.md` 修复 F-002：应用期间到达的外部 canonical settings 不得被标记为已同步后丢弃；增加 applying 期间 rerender 的回归测试，同时覆盖 listen mode/custom address 的成功、`null`、error 路径。
7. 更新 `findings.md` 的执行回应和 `delivery.md` 的实际实现、merge commit、完整 head/base、Round 1、CI 与人工验证；不要改写 main 的 finding 原文或预先填写 main 复验。
8. 只运行本文件允许的 Node 合同/selftest、spec links、Trellis validate、变更 `.mjs` 的 `node --check`（如有）和 `git diff --check`。不得运行 package-manager、Vitest、Cargo、rustfmt、Clippy、构建、生成、dev server 或 Tauri。
9. 提交、推送任务分支，等待自动 full-scope CI。若同一个未修改的 Grok SSE test 再次失败，记录完整日志/响应证据并暂停交 main，不修改 `gateway/routes.rs`、不削弱测试。
10. 只有最新完整 head 的必需检查与所选 frontend、Rust、contracts、CodeQL、`pr-title`、`ci-gate` 全绿后，才把 PR 转为 Ready for review；随后停止写入并报告完整 head、CI URL 和工作树状态。

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
