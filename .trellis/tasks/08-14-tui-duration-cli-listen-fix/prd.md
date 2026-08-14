# 修复 TUI 请求时间与 CLI 监听切换

## Plan Status

- Implementation authorization: confirmed
- Confirmation: 2026-08-14；用户要求把 TUI 请求时间显示和 CLI 监听地址/令牌问题放在同一个任务中继续实施。
- Confirmed coverage: 本 PRD 的范围、锁定决定、非目标与 AC。
- Planning revision: `PLANNING_SHA_PENDING`（main 在规划提交后回填）。
- Execution route: delegated sibling worktree。
- Migrated from direct-main record: 无。

## Material Facts, Assumptions, and Open Questions

| Item | Source | Status / closure condition |
| --- | --- | --- |
| `main` / `origin/main` 基线 | 2026-08-14 preflight | 已确认：`1b218897c09894cfb5aff796761eb8004ad6e53f`。 |
| PENDING | `PENDING.md` | 已审阅；没有 `pending` 或 `planned` 条目。 |
| TUI 根因 | `request_card_lines`、observer projection、提交 `6effa37d` | 已确认：Active 有实时 `duration_ms` 且无 `ttfb_ms`，卡片却无条件读取 `ttfb_ms`。 |
| CLI 根因 | `settings_set_impl_with_gateway`、`sync_cli_proxy_for_settings` | 已确认：运行中网关重绑持有 `gateway_lifecycle_lock`，随后 helper 再次获取同一不可重入锁。 |
| 令牌提示延迟 | `NetworkSettingsCard` 与 tab 生命周期 | 已确认：保存路径等待死锁 IPC；重新挂载 effect 可独立 reveal 已生成令牌。 |
| 前端附带缺陷 | `commitListenMode`、一次性 reveal | 已确认：`null` 返回不回滚；挂载和保存两条 reveal 路径可竞争并在卸载时丢失 UI 结果。 |
| 其他活动任务 | `chore/trim-redundant-tests` worktree | 产品文件无重叠；两分支只会共同修改 `.trellis/tasks/README.md`，合并顺序由 main 处理。 |
| 材料性未决问题 | 用户决定与当前代码 | 无。 |

## Goal

恢复 TUI 请求卡片对进行中请求的实时持续时间显示；让 CLI 监听模式在本地与非回环地址之间可靠切换，消除后端自锁，并保证访问令牌在同一次用户操作中及时、唯一、可恢复地呈现。

## Locked Decisions

1. TUI 请求卡片对 `Active` 请求显示 `duration_ms`；对 `Terminal` 请求显示 `ttfb_ms`。终态缺少 TTFB 时仍显示 `—`。
2. TUI 详情页继续分别显示总耗时与首字时间；输出速率、路由计数、observer 协议和 snapshot 投影语义不改变。
3. 后端必须保持 gateway 重绑与 CLI proxy 同步的生命周期串行性，不通过简单移除锁解决；采用现有风格的 locked/unlocked 分层或等价的显式锁所有权方案，任何路径不得重复等待同一把锁。
4. 从 `localhost` 切到 `lan` 或其他非回环监听时，设置 mutation 必须有界完成；成功后在同一次交互中立即显示访问令牌，不要求切换选项卡。
5. 监听模式保存期间必须有明确的进行中反馈。成功、失败或返回 `null` 后控件都必须恢复可操作；失败或未提交时选择值回滚到真实 settings。
6. 访问令牌 reveal 只有一个前端 owner，状态位于不会随 General tab 卸载而丢失的层级；挂载恢复与保存成功不得并发消费一次性令牌。
7. 保留现有安全语义：非回环 peer 必须使用 Bearer Token；明文不落盘、不写日志；后端 pending reveal、acknowledge、rotate 和 digest 持久化接口不改变。
8. 用户关闭未确认的令牌提示后，继续沿用“需要轮换才能再次获得明文”的现有行为；本任务不把后端 reveal 改成可重复读取。

## Requirements

- 在 `request_card_lines` 中按 `ObserverRequestState` 选择卡片时间，并增加 Active、Terminal、防御性混合字段和输出速率组合测试。
- 将 CLI proxy 同步核心拆为不重复获取 lifecycle lock 的内部路径；无外层 guard 的调用者仍通过加锁 wrapper。
- 为“网关运行中，监听模式切换会触发重绑和 CLI proxy sync”增加有界完成的 Rust 回归测试。若 Wry-only 入口不可直接测试，应抽取可测试的锁所有权分支，不得以源码字符串断言替代行为测试。
- 把令牌 dialog/controller 提升到 `CliManagerPage` 或其持久 data-model 层；General tab 卸载后异步 reveal 结果仍可显示。
- `NetworkSettingsCard` 保持网络设置职责，但不得同时拥有 mount reveal 和保存 reveal 两条无协调路径。
- 将外部 settings 到本地 draft 的 render-phase dispatch 改为 effect 或等价的提交后同步，避免 render 期间 set-state。
- 监听选择保存返回 `null`、抛错、成功三条路径都要有测试；成功后可从 `lan` 再切回 `localhost`。
- 增加 tab 切换期间 deferred settings/reveal 的测试，证明令牌不会因卸载丢失，也不会重复调用 reveal。
- 更新 `local-observer-tui-contract.md`，明确 Active/Terminal 卡片时间字段。
- 新增最小 `gateway-listen-token-contract.md` 并链接到 cross-layer index，记录生命周期锁、监听重绑、一次性令牌和 UI owner 合同。

## Acceptance Criteria

- **AC-01 TUI state metric**：Active 卡片显示非负 `duration_ms`，即使携带防御性 `ttfb_ms` 也不改用首字；Terminal 卡片显示 `ttfb_ms`，无 TTFB 为 `—`。详情页与输出速率测试保持原语义。
- **AC-02 No lifecycle self-deadlock**：运行中的 gateway 从 `localhost` 切到 `lan`，以及从非回环切回 `localhost` 时，设置 mutation 在测试 timeout 内返回；重绑与 CLI proxy sync 仍串行，其他调用者仍受 lifecycle lock 保护。
- **AC-03 Immediate token presentation**：非回环监听保存成功后，无需切 tab 即出现可复制的令牌提示；保存期间有进行中状态，提示关闭或确认后监听控件恢复可用并可切回本地模式。
- **AC-04 State rollback**：保存返回 `null` 或抛错时，选择值回滚到真实 `settings.gateway_listen_mode`；成功响应和后续 query cache 更新不会触发 render-phase state update。
- **AC-05 Single reveal owner**：初始恢复、保存成功和 tab 卸载/重挂载最多由一个协调流程消费 pending token；获得明文的异步结果不会随 General tab 卸载丢失。rotate、copy、acknowledge 仍工作。
- **AC-06 Security and compatibility**：非回环入站鉴权、token digest、一次 reveal、确认和轮换语义不变；未新增 public IPC、bindings、依赖、迁移或明文持久化，CLI proxy rollback/ownership 合同未削弱。
- **AC-07 Contracts and regression tests**：新增/更新的现行合同与代码一致；frontend tests 覆盖保存 pending/null/error/success、tab 切换和 token dialog；Rust tests 覆盖 TUI 状态选择及锁路径有界完成。
- **AC-08 Verification**：允许的本地无依赖合同与 `git diff --check` 通过；最新 PR head 的自动 `ci-gate`、`pr-title` 及 full-scope frontend/Rust jobs 绿色。真实桌面交互若不能在 execution session 环境验证，须在 `delivery.md` 明确交给 main/用户。

## Non-Goals

- 不改变 gateway listen mode 枚举、默认端口、custom/WSL 地址解析或远程 peer 鉴权范围。
- 不改变 bearer token 的长度、生成算法、digest 格式、后端一次性 reveal API、acknowledge 或 rotate 公共合同。
- 不改变 CLI proxy 配置内容、managed profile/catalog 语义、settings CAS rollback 或 gateway 启停错误恢复策略。
- 不改变 observer protocol、snapshot 字段、请求详情页、TTFB 统计、输出速率公式、路由计数或 TUI scope 选择。
- 不增加依赖、不改 lockfile、generated bindings、release/candidate/signing/performance/CodeQL 行为。
- 不修改测试清理 worktree、历史任务、审计记录或 PENDING 文件。

## Scope and Decision Changes

| Date | Old / new decision | Affected acceptance criteria | Decision owner / resume condition |
| --- | --- | --- | --- |
| 2026-08-14 | 两个问题合并为一个独立 worktree 任务；TUI 按状态选择时间，CLI 同时修复后端锁与前端令牌生命周期。 | AC-01 至 AC-08 | 用户确认。 |

## PENDING Review

- 当前无未解决条目；本任务不创建或迁移 PENDING 项。
