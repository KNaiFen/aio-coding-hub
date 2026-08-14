# 交付报告：TUI 请求时间与 CLI 监听切换修复

> 本文件描述 PR #147 的实际交付候选。功能实现 head 与后续记录 head
> 分层记录；每次记录提交形成新 PR head 后，仍须等待该 head 的自动检查。

## 交付状态

- 结果：实现与现行合同已完成，等待最新 head 的自动 CI。
- PR：[#147](https://github.com/KNaiFen/aio-coding-hub/pull/147)（Draft）
- 分支：`fix/tui-duration-cli-listen`
- PR base：`main` @ `1b218897c09894cfb5aff796761eb8004ad6e53f`
- 功能实现候选 head：`e4e457beea239ee89cb5e2dacafbe38eeab74408`
- 最近验证记录 head：尚无；本轮 spec/delivery 提交推送后等待其自动检查。
- 规划提交：`5419ccf64ba73387f999133389ab3d347e63270c`
- `ci-gate`：等待最新 head 自动检查。
- 其他必需检查：等待 `pr-title` 与 full-scope frontend/Rust jobs。
- 手工桌面验证：未执行。
- 执行 session：当前唯一写者为独立 execution session；完成允许的本地验证、
  推送记录并等待最新 head 检查后停止写入。

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
| AC-01 TUI state metric | 实现完成，待 Rust CI | `dfb02db8`；邻近测试覆盖 Active/Terminal/混合字段/缺失 TTFB/输出速率。 |
| AC-02 No lifecycle self-deadlock | 实现完成，待 Rust CI | `078f2b70`；timeout 行为测试覆盖 localhost 与 LAN 双向切换的持锁分支。 |
| AC-03 Immediate token presentation | 实现完成，待 frontend CI | `e4e457be`；LAN 成功回调与 page-level dialog 测试。 |
| AC-04 State rollback | 实现完成，待 frontend CI | `NetworkSettingsCard` tests 覆盖 pending、canonical success、`null`、error 与 LAN -> localhost。 |
| AC-05 Single reveal owner | 实现完成，待 frontend CI | `CliManagerPage` test 覆盖单次 in-flight reveal、tab 卸载、copy、close、rotate、ack。 |
| AC-06 Security and compatibility | 实现与审查完成，待 CI | 未改 public IPC、bindings、schema、鉴权、token 算法或持久化；明文只进入短生命周期 controller state。 |
| AC-07 Contracts and regression tests | 实现完成，待 spec/frontend/Rust CI | 新增 gateway contract，更新 observer contract/index；测试位于相邻模块。 |
| AC-08 Verification | 本地通过，云端进行中 | 五项允许的本地检查通过；最新 head 自动检查尚待终态。 |

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

最新 records head 推送后等待自动 `ci-gate`、`pr-title` 及 full-scope
frontend/Rust jobs。按仓库合同不在本 worktree 运行 package-manager、Vitest、
Cargo、rustfmt、Clippy、构建、生成、dev server、Tauri、签名或打包；这些由
GitHub Actions 验证。

## 偏移、风险与回滚

- 计划偏移：无。
- 兼容性：无 public IPC、settings schema、observer protocol 或生成绑定变化。
- 安全：AIO token sidecar 仍只持久化 digest/metadata；一次 reveal、acknowledge、
  rotate、非回环 Bearer 鉴权和 loopback 例外不变。
- 人工验证：真实桌面 LAN 切换与 tab 交互未在 execution session 本地运行，
  交由 main/用户在 CI 绿色后按需复验。
- 回滚：可分别回退 TUI、backend、frontend 和 spec/delivery 原子提交；无迁移。

## 阻塞快照

无。

## main 验收记录

尚未进入验收。

## main 收尾

尚未进入收尾。
