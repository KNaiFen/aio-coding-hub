# 交付报告：TUI 请求时间与 CLI 监听切换修复

> 本文件描述 PR #147 的实际交付候选。功能实现 head 与后续记录 head
> 分层记录；每次记录提交形成新 PR head 后，仍须等待该 head 的自动检查。

## 交付状态

- 结果：实现与现行合同已完成；最新功能候选的任务内检查通过，但 Rust job
  被范围外 Grok SSE route test 的单次 `502 != 200` 阻塞。该失败无法归因到
  本任务且没有可靠的任务内修法，execution session 已按停止条件暂停写入。
- PR：[#147](https://github.com/KNaiFen/aio-coding-hub/pull/147)（Draft）
- 分支：`fix/tui-duration-cli-listen`
- PR base：`main` @ `1b218897c09894cfb5aff796761eb8004ad6e53f`
- 功能实现候选 head：`e4e457beea239ee89cb5e2dacafbe38eeab74408`
- 冻结失败候选 head：`125fba0ec5a47c1ecd12c9f32ac80426d627d5bd`
- 规划提交：`5419ccf64ba73387f999133389ab3d347e63270c`
- `ci-gate`：[失败](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31798457041/job/94765660089)；
  聚合失败来自 Rust job。
- 其他必需检查：`pr-title`、frontend、change-scope、docs/support contracts 与
  Rust/JavaScript CodeQL 通过；
  [Rust job](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31798457041/job/94760734452)
  因范围外 Grok SSE route test 失败。
- 手工桌面验证：未执行。
- 执行 session：独立 execution session 仍是登记的唯一写者，但已停止写入并
  等待 main 判断失败归属与恢复条件。

## 实际实现

- TUI 请求卡片按状态选择时间字段：Active 使用 `duration_ms`，Terminal
  使用 `ttfb_ms`；详情页、输出速率、路由计数和 observer 协议未改变。
- settings runtime transaction 显式区分 lifecycle locked/unlocked CLI proxy
  sync；运行中 gateway 重绑、proxy sync 和允许的 rollback 共用同一 guard。
- CLI Manager 将一次性 token owner 提升到 `CliManagerPage`，保存成功和
  初始恢复复用单一 in-flight reveal；tab 卸载不丢失异步结果。
- 网络设置保存提供“正在应用”状态，成功采用返回的 canonical settings；
  `null`/error 回滚到最新真实 settings，且不在 render 阶段 dispatch。
- 更新 observer/TUI 现行合同，新增 gateway listen/token 跨层合同并链接索引。

## Acceptance Criteria

| 标准 | 当前结果 | 证据 |
|---|---|---|
| AC-01 TUI state metric | 实现完成；任务测试未列为失败 | `dfb02db8`；邻近测试覆盖 Active/Terminal/混合字段/缺失 TTFB/输出速率；Rust job 的唯一失败位于未修改的 `gateway/routes.rs`。 |
| AC-02 No lifecycle self-deadlock | 实现完成；任务测试未列为失败 | `078f2b70`；timeout 行为测试覆盖 localhost 与 LAN 双向切换的持锁分支；Rust job 的唯一失败位于未修改的 `gateway/routes.rs`。 |
| AC-03 Immediate token presentation | 通过 frontend CI | `e4e457be`；LAN 成功回调与 page-level dialog 测试。 |
| AC-04 State rollback | 通过 frontend CI | `NetworkSettingsCard` tests 覆盖 pending、canonical success、`null`、error 与 LAN -> localhost。 |
| AC-05 Single reveal owner | 通过 frontend CI | `CliManagerPage` test 覆盖单次 in-flight reveal、tab 卸载、copy、close、rotate、ack。 |
| AC-06 Security and compatibility | 实现与审查完成；相关检查通过 | 未改 public IPC、bindings、schema、鉴权、token 算法或持久化；明文只进入短生命周期 controller state。 |
| AC-07 Contracts and regression tests | 合同与 frontend 通过；Rust job 被范围外测试阻塞 | 新增 gateway contract，更新 observer contract/index；任务测试位于相邻模块，唯一失败为既有 Grok SSE route test。 |
| AC-08 Verification | 阻塞 | 五项允许的本地检查通过；`125fba0e` 的 frontend、合同、title、CodeQL 通过，但 Rust/`ci-gate` 未绿。 |

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

- 冻结 head：`125fba0ec5a47c1ecd12c9f32ac80426d627d5bd`
- 自动 run：[ci #31798457041](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31798457041)
- 通过：change-scope、docs-contract、support-contract、frontend（lint、unit
  tests、build）、Rust 格式/绑定漂移、Clippy、`pr-title`、Rust/JavaScript
  CodeQL。
- 失败：Rust tests 共 `2899 passed, 1 failed, 5 ignored`；唯一失败为
  `gateway::routes::tests::mock_runtime_router_grok_responses_sse_is_transparent_and_logged`
  在 `src-tauri/src/gateway/routes.rs:2146` 得到 `502`、预期 `200`。因此
  `ci-gate` 同步失败。
- PR diff 不含 `src-tauri/src/gateway/routes.rs`；该测试在 base 已存在，请求链
  不经过本任务修改的 settings lifecycle 路径。CI 断言在读取错误响应体前终止，
  没有足够证据安全修改任务内代码或范围外 route test。

按仓库合同未在本 worktree 运行 package-manager、Vitest、Cargo、rustfmt、
Clippy、构建、生成、dev server、Tauri、签名或打包，也未手动 dispatch 额外 CI。

## 偏移、风险与回滚

- 计划偏移：无。
- 兼容性：无 public IPC、settings schema、observer protocol 或生成绑定变化。
- 安全：AIO token sidecar 仍只持久化 digest/metadata；一次 reveal、acknowledge、
  rotate、非回环 Bearer 鉴权和 loopback 例外不变。
- 人工验证：真实桌面 LAN 切换与 tab 交互未在 execution session 本地运行，
  交由 main/用户在 CI 绿色后按需复验。
- 回滚：可分别回退 TUI、backend、frontend 和 spec/delivery 原子提交；无迁移。

## 阻塞快照

- 失败证据绑定 head：`125fba0ec5a47c1ecd12c9f32ac80426d627d5bd`；base
  仍为 `1b218897c09894cfb5aff796761eb8004ad6e53f`，PR head 未漂移。
- 最后安全功能提交：`125fba0ec5a47c1ecd12c9f32ac80426d627d5bd`；本阻塞
  记录写入前，工作树干净且与 `origin/fix/tui-duration-cli-listen` 同步。
- 受影响 AC：仅 AC-08 无法满足“必需 CI 全绿”；AC-01/02 的任务测试没有在
  Rust job 中失败，但不能用整体失败的 job 宣称完整云端验证通过。
- 决定归属：main。execution session 不修改范围外 `gateway/routes.rs`，不削弱
  测试，也不把缺少响应体证据的 502 猜测为 settings lifecycle 回归。
- 恢复条件：main 提供可归因到本任务的失败证据和范围内修法，或确认该失败按
  仓库流程作为既有/基础设施不稳定处理并明确恢复执行。恢复前不再写入、推送、
  标记 Ready、merge、auto-merge、archive 或清理 worktree。

## main 验收记录

### Round 1 - 2026-08-14

- 冻结审查 head：`8e6ca2fbb35e92e3a68544b2b07da6d087d5325f`；PR #147 为 Draft，工作树干净且本地、远端分支与 PR head 一致。
- 审查范围：完整任务；两路只读审查分别覆盖 TUI/Rust lifecycle 和前端 token/draft 时序，main 按精确 `file:line` 点验关键证据。
- 结论：不通过，详见 `findings.md` Round 1。TUI Active/Terminal 时间字段和 gateway lifecycle locked/unlocked 分层未发现阻断项；前端存在 F-001/F-002 两个可达时序缺陷，且分支必须完成 F-003 的最新 main 集成和固定 head CI。
- 旧 AC 结论更正：AC-03、AC-05 当前未满足；AC-04 对 `null`/error 的覆盖成立，但并发 external settings 同步仍未满足；AC-08 仍未满足。
- CI：本轮审查时 [run 31800876521](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31800876521) 仍运行；上一完整候选的 [Rust job](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31798457041/job/94760734452) 和 [ci-gate](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31798457041/job/94765660089) 失败，不能作为交付证据。
- 返工责任：独立 execution session。main 只写本轮验收和恢复边界，不修改产品代码、测试逻辑、依赖或现行合同。

## main 收尾

尚未进入收尾。
