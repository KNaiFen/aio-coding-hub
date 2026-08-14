# 交付报告：TUI 请求时间与 CLI 监听切换修复

> 本文件描述 PR #147 的实际交付候选。功能实现 head 与后续记录 head
> 分层记录；每次记录提交形成新 PR head 后，仍须等待该 head 的自动检查。

## 交付状态

- 结果：完成并通过 main 验收；Round 1 的 F-001/F-002/F-003 均已关闭。
- PR：[#147](https://github.com/KNaiFen/aio-coding-hub/pull/147)（已合并）
- 分支：`fix/tui-duration-cli-listen`
- PR base：`main` @ `1b218897c09894cfb5aff796761eb8004ad6e53f`
- 初始功能实现 head：`e4e457beea239ee89cb5e2dacafbe38eeab74408`
- Round 1 main 集成 merge：`08ac062af5454cf09a811ba71d597430c513c33b`
- Round 1 返工代码 head：`c7800118876f79412236783c4abe260013d606a3`
- Round 1 绿色交付候选 head：`a91f663385f310069aa836d1c23b396d7b822fce`
- 最终接受 head：`a9dd8288285c0149c3cd58315a7ac5c602488755`
- 功能 merge commit：`da7ec35b31b44019432f6cdb61dee19bf84fc397`
- 历史失败候选 head：`125fba0ec5a47c1ecd12c9f32ac80426d627d5bd`
- 规划提交：`5419ccf64ba73387f999133389ab3d347e63270c`
- 最终 `ci-gate`：[通过](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31804550712/job/94786036687)。
- 其他必需检查：最终接受 head 的 `pr-title`、contracts、frontend、Rust 与两项 CodeQL 全绿。
- 手工桌面验证：未执行。
- 执行 session：已暂停并关闭；原 worktree、本地任务分支和远端任务分支均已清理。

## 实际实现

- TUI 请求卡片按状态选择时间字段：Active 使用 `duration_ms`，Terminal
  使用 `ttfb_ms`；详情页、输出速率、路由计数和 observer 协议未改变。
- settings runtime transaction 显式区分 lifecycle locked/unlocked CLI proxy
  sync；运行中 gateway 重绑、proxy sync 和允许的 rollback 共用同一 guard。
- CLI Manager 将一次性 token owner 提升到 `CliManagerPage`，保存成功和
  初始恢复共用串行 reveal owner；Round 1 增加 post-save queue，旧 flight
  返回 `null`/error 时所有保存意图共享一次后续 reveal，已取得 token 时不重复消费。
- 网络设置保存提供“正在应用”状态，成功采用返回的 canonical settings；
  `null`/error 回滚到最新真实 settings，且不在 render 阶段 dispatch。Round 1
  进一步保证 applying 期间到达的外部 canonical source 在结束后胜出。
- 通过 merge commit `08ac062a` 集成 `origin/main@0ae7f03a`，保留 contracts
  workflow、测试清理结果和归档索引事实。
- 更新 observer/TUI 现行合同，新增 gateway listen/token 跨层合同并链接索引。

## Acceptance Criteria

| 标准 | 当前结果 | 证据 |
|---|---|---|
| AC-01 TUI state metric | 通过 | `dfb02db8`；邻近行为测试随 [Rust job](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31802412409/job/94773833109) 通过。 |
| AC-02 No lifecycle self-deadlock | 通过 | `078f2b70`；双向 timeout 行为测试随 Rust job 通过。 |
| AC-03 Immediate token presentation | 通过 | `86a48497`；deferred LAN 保存队列测试随 [frontend job](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31802412409/job/94773833165) 通过。 |
| AC-04 State rollback | 通过 | `c7800118`；external mode/address 的 success、`null`、error 参数化测试随 frontend job 通过。 |
| AC-05 Single reveal owner | 通过 | `86a48497`；去重、旧 flight 成功不重复消费及既有 tab/copy/close/rotate/ack 流程通过。 |
| AC-06 Security and compatibility | 通过 | 未改 public IPC、bindings、schema、鉴权、token 算法或持久化；contracts、CodeQL 与 full-scope jobs 全绿。 |
| AC-07 Contracts and regression tests | 通过 | gateway contract 已补充 queued reveal 和 deferred canonical winner；contracts、frontend、Rust 全绿。 |
| AC-08 Verification | 通过 | 五项允许的本地检查通过；最终接受 head `a9dd8288285c0149c3cd58315a7ac5c602488755` 的所有必需检查全绿。 |

## 验证

### 本地检查

| 命令 | 结果 | 说明 |
|---|---|---|
| `node scripts/check-cloud-only-verification.selftest.mjs` | 通过 | `[cloud-only-verification:selftest] all assertions passed`。 |
| `node scripts/check-cloud-only-verification.mjs` | 通过 | `[cloud-only-verification] repository contract passed`。 |
| `node scripts/check-spec-links.mjs` | 通过 | 新增及既有现行 spec 链接有效。 |
| `python3 ./.trellis/scripts/task.py validate .trellis/tasks/08-14-tui-duration-cli-listen-fix` | 通过 | `implement.jsonl`、`check.jsonl` 各 7 个有效条目。 |
| `git diff --check` | 通过 | 当前实现与记录无空白错误。 |

### GitHub CI 与编译

- 最终接受 head：`a9dd8288285c0149c3cd58315a7ac5c602488755`
- 自动 run：[ci #31804550712](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31804550712)
- 通过：change-scope、contracts、frontend、Rust、`ci-gate`、`pr-title`、
  JavaScript/TypeScript CodeQL、Rust CodeQL 和 CodeQL 汇总；候选构建 jobs 对 PR 按设计跳过。
- Round 1 绿色交付候选：`a91f663385f310069aa836d1c23b396d7b822fce`
- 自动 run：[ci #31802412409](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31802412409)
- 通过：change-scope、contracts、frontend（lint、unit tests、build）、Rust
  （格式/绑定漂移、Clippy、tests）、`ci-gate`、`pr-title` 与两项 CodeQL。
- Rust tests 未复现历史 Grok SSE 失败；Rust job 20m48s 后成功。
- 历史失败 head：`125fba0ec5a47c1ecd12c9f32ac80426d627d5bd`；
  自动 run：[ci #31798457041](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31798457041)。
- 历史 run 中通过：change-scope、docs-contract、support-contract、frontend（lint、unit
  tests、build）、Rust 格式/绑定漂移、Clippy、`pr-title`、Rust/JavaScript
  CodeQL。
- 历史 run 中失败：Rust tests 共 `2899 passed, 1 failed, 5 ignored`；唯一失败为
  `gateway::routes::tests::mock_runtime_router_grok_responses_sse_is_transparent_and_logged`
  在 `src-tauri/src/gateway/routes.rs:2146` 得到 `502`、预期 `200`。因此
  `ci-gate` 同步失败。
- PR diff 不含 `src-tauri/src/gateway/routes.rs`；该测试在 base 已存在，请求链
  不经过本任务修改的 settings lifecycle 路径。CI 断言在读取错误响应体前终止，
  没有足够证据安全修改任务内代码或范围外 route test。

按仓库合同未在本 worktree 运行 package-manager、Vitest、Cargo、rustfmt、
Clippy、构建、生成、dev server、Tauri、签名或打包，也未手动 dispatch 额外 CI。

## 偏移、风险与回滚

- 计划偏移：初始交付经 main Round 1 审查新增 F-001/F-002 时序整改，并按
  F-003 普通 merge 最新 main；未改变用户锁定行为、公共 API 或安全语义。
- 兼容性：无 public IPC、settings schema、observer protocol 或生成绑定变化。
- 安全：AIO token sidecar 仍只持久化 digest/metadata；一次 reveal、acknowledge、
  rotate、非回环 Bearer 鉴权和 loopback 例外不变。
- 人工验证：真实桌面 LAN 切换与 tab 交互未在 execution session 本地运行，
  交由 main/用户在 CI 绿色后按需复验。
- 回滚：可分别回退 TUI、backend、frontend 和 spec/delivery 原子提交；无迁移。

## 阻塞快照

当前无实现阻塞。历史 `125fba0e` 的范围外 Grok SSE 失败保留为证据；若最新
完整 head 再次出现同一失败，按 `execution.md` 停止并交 main，不修改
`gateway/routes.rs` 或削弱测试。

## Round 1 返工

- main 交接 head：`52232d72993f83be4ba2bd04b7e11171616a06cf`；恢复前
  本地、远端与 PR head 一致，preflight 全部通过。
- F-001：`86a484973738adcc27e24738ed1019f8dde6cfb6`。
- F-002：`c7800118876f79412236783c4abe260013d606a3`。
- F-003：`08ac062af5454cf09a811ba71d597430c513c33b`，父提交包含
  `origin/main@0ae7f03abaa37c7021fdf8718373e27fe61f62fd`。
- 绿色交付候选：`a91f663385f310069aa836d1c23b396d7b822fce`；
  [ci-gate](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31802412409/job/94778879744)
  及所选完整 jobs 全绿。
- 本地禁止 Vitest、package-manager、Cargo、rustfmt、Clippy、构建和生成；
  新增 frontend 行为测试与既有完整套件由自动 CI 执行。

## main 验收记录

### Round 1 - 2026-08-14

- 冻结审查 head：`8e6ca2fbb35e92e3a68544b2b07da6d087d5325f`；PR #147 为 Draft，工作树干净且本地、远端分支与 PR head 一致。
- 审查范围：完整任务；两路只读审查分别覆盖 TUI/Rust lifecycle 和前端 token/draft 时序，main 按精确 `file:line` 点验关键证据。
- 结论：不通过，详见 `findings.md` Round 1。TUI Active/Terminal 时间字段和 gateway lifecycle locked/unlocked 分层未发现阻断项；前端存在 F-001/F-002 两个可达时序缺陷，且分支必须完成 F-003 的最新 main 集成和固定 head CI。
- 旧 AC 结论更正：AC-03、AC-05 当前未满足；AC-04 对 `null`/error 的覆盖成立，但并发 external settings 同步仍未满足；AC-08 仍未满足。
- CI：本轮审查时 [run 31800876521](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31800876521) 仍运行；上一完整候选的 [Rust job](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31798457041/job/94760734452) 和 [ci-gate](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31798457041/job/94765660089) 失败，不能作为交付证据。
- 返工责任：独立 execution session。main 只写本轮验收和恢复边界，不修改产品代码、测试逻辑、依赖或现行合同。

### Round 2 - 2026-08-14

- 冻结审查 head：`a9dd8288285c0149c3cd58315a7ac5c602488755`；base `0ae7f03abaa37c7021fdf8718373e27fe61f62fd`，PR 为 Ready、`OPEN/CLEAN/MERGEABLE`，工作树干净且本地、远端和 PR head 一致。
- CI：[ci-gate job 94786036687](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31804550712/job/94786036687)、`pr-title`、contracts、frontend、Rust、JavaScript/TypeScript CodeQL、Rust CodeQL 与 CodeQL 汇总均绑定该 head 且成功。
- 审查：两路只读复核分别覆盖 TUI/Rust 与前端 token/draft 时序；main 点验 `request_card_lines`、observer 刷新链路、lifecycle locked/unlocked 分层、post-save reveal queue、applying-time canonical settings winner、邻近回归测试及现行合同。
- 结论：通过。F-001、F-002、F-003 全部关闭；AC-01 至 AC-08 满足，无新阻断 finding。
- 接受风险：真实桌面 LAN 切换未在本地执行；现有 Rust timeout 测试使用 mock app，未覆盖真实 gateway harness 的完整 rebind 与启动失败 rollback。相关 formatter、frontend、Rust 行为测试和完整 CI 已通过，该缺口不阻断本次交付。

## main 收尾

- 功能 PR #147 于 2026-08-14T14:03:49Z 合并；merge commit 为 `da7ec35b31b44019432f6cdb61dee19bf84fc397`。
- 本地 `main` 已 fetch 并 fast-forward 到该 merge commit，确认最终接受 head 已进入 `main`。
- 原 worktree `/Users/knaifen/Documents/Codex/aio-coding-hub/tui-duration-cli-listen-fix` 已删除；远端任务分支由 GitHub 合并策略自动删除，本地任务分支已删除。
- 长期知识：`gateway-listen-token-contract.md`、`local-observer-tui-contract.md` 与 cross-layer index 已随功能 PR 同步；`docs/README.md` 无需新增重复入口。
- PENDING：当前无未解决条目，本任务不新增或迁移 PENDING。
- Trellis 归档与全局校验：`task.py archive --no-commit 08-14-tui-duration-cli-listen-fix`、`task.py validate --all` 和 `git diff --check` 均在 records-only 收尾提交前执行并通过。
