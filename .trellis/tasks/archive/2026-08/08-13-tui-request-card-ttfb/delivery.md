# 交付报告：TUI 请求卡片改用首字时间与紧凑路由计数

> 本文件描述 PR #136 的实际交付候选。功能实现候选与最近已验证的记录 head 分层记录；本次交付状态记录提交会形成新的 records-only PR head，main 验收时仍须读取实时 PR head 与实时检查。

## 交付状态

- 结果：等待验收
- PR：[#136](https://github.com/KNaiFen/aio-coding-hub/pull/136)
- 分支：`fix/tui-request-card-ttfb`
- PR base：`main` @ `875ff441c5ba9f1a7f235ad95dadb945a41bba61`
- 功能实现候选 head：`5b8414b9f6dfd156c702b4d229cde69d013136b6`
- 最近已验证记录 head：`139ed0c5e8a9fcf9f4cda8f3d65835e658dc80d3`（仅任务记录提交，产品 diff 与功能实现候选相同）
- 规划提交：`bd91552393f36419ce215d9de283b7519c0efb07`
- `ci-gate`：最近已验证记录通过，[job 94383913838](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31678728115/job/94383913838)
- 其他必需检查：最近已验证记录的 `pr-title`、`rust`、`support-contract`、`change-scope` 与两项 CodeQL 均通过；`frontend` 按 Rust-only scope 跳过
- 交付时间：2026-08-13T15:29:41+08:00
- 执行 session：本记录推送后停止写入；等待实时最新 head 的自动检查绿色后，PR 保持 Ready 并通知 main 验收

## Preflight

- 目录：`pwd -P` 为 `/Users/knaifen/Documents/Codex/aio-coding-hub/08-13-tui-request-card-ttfb`；shell 的逻辑路径 `/Users/knaifen/codex/...` 来自 `/Users/knaifen/codex -> ./Documents/Codex`，两者 inode 与 Git worktree 登记一致。
- 分支：`fix/tui-request-card-ttfb`。
- base：`git merge-base 875ff441c5ba9f1a7f235ad95dadb945a41bba61 HEAD` 精确返回该完整 SHA。
- 规划提交：`bd91552393f36419ce215d9de283b7519c0efb07` 存在，且 `execution.md` 已登记非占位完整 SHA。
- 任务状态：`task.json.status` 为 `in_progress`；PR base 为 `main`。
- 授权与写权：`execution.md` 已登记实施授权，当前唯一写者为本独立执行 session；材料性未决问题为无。
- 初始工作树：开工检查时无未提交或来源不明修改；HEAD 为交接提交 `d6f04e115949d644dbc0a5a861086ed65440ff8e`。

## 阻塞快照

无。

## 实现摘要

### 用户可见结果

- 请求卡片 route 行显示首字时间；缺少首字时间时显示 `—`，输出速率仍按既有规则追加。
- 请求卡片将供应商切换与重试分别缩写为 `切N`、`重N`，两者存在时显示 `切N/重N`；直连与未上游文案不变。
- 状态行和详情页仍使用完整的 `切换N/重试N`，详情页继续分别显示总耗时和首字时间。

### 内部实现

- `request_card_lines` 从 `ttfb_ms` 读取卡片时间，并继续复用 `format_duration`。
- 私有 `request_card_route_result` 只服务请求卡片；共享 `route_result` 未修改。

## 验收标准对应

| 标准 | 结果 | 证据 |
|---|---|---|
| AC-01 卡片使用 TTFB | 通过 | 邻近测试固定 `duration_ms=2000`、`ttfb_ms=500`，断言 route 行为 `500ms`；功能候选 Rust job 通过。 |
| AC-02 紧凑且独立的路由计数 | 通过 | 新测试覆盖 `切1`、`重3`、`切1/重3`、`直连`、`未上游`；功能候选 Rust job 通过。 |
| AC-03 缺失 TTFB 与输出速率 | 通过 | 邻近测试断言 `ttfb_ms=None` 时为 `直连  —  100.0 t/s`；功能候选 Rust job 通过。 |
| AC-04 共享/详情文案不回归 | 通过 | 新测试断言共享及详情路由为 `切换1/重试3`，详情仍含 `耗时  2.0s` 与 `首字  500ms`；功能候选 Rust job 通过。 |
| AC-05 变更范围 | 通过 | 相对 base 的产品代码仅改 `src-tauri/crates/aio-tui/src/format.rs`；其余为本任务 Trellis 记录与活动索引。 |
| AC-06 本地合同与云端检查 | 通过（最近已验证记录） | 四项允许的本地检查通过；记录 `139ed0c5…` 的 `ci-gate`、`pr-title`、Rust 相关检查及 CodeQL 均绿色。本记录推送后的实时 head 仍须自动复验。 |

## 主要代码位置

| 文件或符号 | 变更 | 设计原因 |
|---|---|---|
| `src-tauri/crates/aio-tui/src/format.rs:request_card_lines` | 卡片时间改读 `ttfb_ms`，路由摘要改用卡片专用 formatter。 | 把展示变更限制在请求卡片。 |
| `src-tauri/crates/aio-tui/src/format.rs:request_card_route_result` | 独立格式化两个计数的紧凑文案。 | 保持共享 `route_result`、状态行与详情页完整文案不变。 |
| `src-tauri/crates/aio-tui/src/format.rs:tests` | 覆盖 TTFB、缺失值、输出速率、计数组合和详情回归。 | 将 AC-01 至 AC-04 固定为邻近 Rust 单元测试。 |

## 与计划的偏移

- 无。

## 验证结果

### 本地检查

| 命令 | 结果 | 说明 |
|---|---|---|
| `node scripts/check-cloud-only-verification.mjs` | 通过 | `[cloud-only-verification] repository contract passed`。 |
| `node scripts/check-cloud-only-verification.selftest.mjs` | 通过 | 所有合同自测断言通过。 |
| `python3 ./.trellis/scripts/task.py validate 08-13-tui-request-card-ttfb` | 通过 | `implement.jsonl`、`check.jsonl` 各 2 个有效条目。 |
| `git diff --check` | 通过 | 实现与记录均无空白错误。 |

按仓库合同未在本地运行 Cargo、rustfmt、Clippy、Rust tests、构建、Tauri、package-manager 脚本或依赖安装。

### GitHub CI 与编译

| Workflow / Job | 结果 | 链接或说明 |
|---|---|---|
| `ci-gate` | 通过 | [最近已验证记录 job 94383913838](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31678728115/job/94383913838) |
| `rust` | 通过 | [job 94379144418](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31678728115/job/94379144418)，覆盖云端格式、绑定导出、Clippy、Rust tests 与 audit。 |
| `support-contract`、`change-scope` | 通过 | [自动 PR workflow 31678728115](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31678728115)；scope 为 Rust-only。 |
| `pr-title` | 通过 | [job 94379057915](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31678728014/job/94379057915) |
| CodeQL | 通过 | [run 31678728066](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31678728066)，JavaScript/TypeScript 与 Rust 均成功。 |
| `frontend`、候选发布 jobs | 按范围跳过 | `ci-gate` 已接受 Rust-only scope 的预期跳过。 |

### 人工验证

- 无；本任务行为由邻近格式化单元测试与云端 Rust job 验证。

## 测试、文档与合同

- 测试：更新 `format.rs` 邻近测试并新增卡片/共享视图回归。
- 现行文档：不适用；现有 TUI 合同已规定两个计数独立及请求卡片使用语义 route 行，本任务没有改变合同。
- 类型或机器合同：不适用；未修改 Observer 协议、snapshot 或生成绑定。
- 迁移或发布说明：不适用。

## 兼容性、风险与回滚

- 兼容性：无公共接口、状态行或详情字段变化。
- 数据与配置：无影响，无迁移或默认值变化。
- 安全与隐私：无边界变化。
- 回滚方式：回退本任务实现提交即可。
- 剩余风险：无已知实现风险；main 仍需按 PR diff 和 AC 独立验收。

## 未完成项与阻塞

- 无。推送本交付状态记录后，只等待实时最新 PR head 的自动检查终态；随后执行 session 停止写入，等待 main 验收。

## 建议 main 重点审查

- `src-tauri/crates/aio-tui/src/format.rs:request_card_lines`：确认时间来源只改为 `ttfb_ms`，输出速率条件和其他卡片行保持不变。
- `src-tauri/crates/aio-tui/src/format.rs:request_card_route_result`：确认两个计数独立，且共享 `route_result` 没有被缩写。

## main 验收记录

### Round 1 - 通过，待验收记录提交的最新 head 复验

- 审查日期：2026-08-13。
- 审查候选 head：`2b12c68cd99f7bc7c21fb8fa2b5354c9992a229b`；PR base 为 `875ff441c5ba9f1a7f235ad95dadb945a41bba61`，PR #136 为 Ready for review、`CLEAN`。
- 写权与工作树：执行 session 已暂停；main 接手时 worktree 干净、无未提交内容。本轮仅写入验收/生命周期记录，不改产品代码或测试逻辑。
- 实时 CI：同一 head 的严格必需 [`ci-gate` job 94399249451](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31683320910/job/94399249451)、[`pr-title`](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31683320919/job/94393553329) 均成功；`rust`、`change-scope`、`support-contract` 和 CodeQL 亦成功，`frontend`、文档合同、候选/发布 jobs 按 Rust-only scope 跳过。
- 审查范围：任务 PRD/施工入口/交付材料、相对 base 的完整 PR diff、`request_card_lines` 与路由 formatter、邻近单元测试、Observer/TUI 合同、实时 PR 元数据与检查。
- 验收结论：通过。AC-01 至 AC-04 分别由 TTFB 来源、紧凑且独立的 `切`/`重` formatter、缺失 TTFB 加输出速率回归以及共享/详情完整文案回归测试覆盖；AC-05 确认产品代码仅改 `src-tauri/crates/aio-tui/src/format.rs`；AC-06 的允许本地检查和当前 head 云端检查均通过。
- 接受的偏移或风险：无产品行为偏移。未在本地运行 Cargo、Rust tests、格式化或构建，遵循 cloud-only 规则，云端 Rust job 已覆盖格式、生成绑定、Clippy、测试与 audit。
- 后续门：本验收记录会产生新的纯记录 head；main 仅在该 head 的自动检查终态成功、PR head 未漂移且相对本候选仍仅为验收/生命周期记录时合并。

## main 收尾

> 仅 main 填写。功能 PR 与 records-only 收尾 PR 均已合并，Trellis 任务已归档且相关 worktree/分支已清理；本节保留最终验收、归档与清理事实。

- 最终结果：完成。
- 功能 PR 与验收候选：[PR #136](https://github.com/KNaiFen/aio-coding-hub/pull/136)；main 最终接受并合并的完整 head 为 `94b071f2b20be9ecf3693e463775de9a99273ca4`。
- main 合并提交：`6effa37d31de5f7e8f8c30b6a06f1cb93cae4243`，2026-08-13 squash merge 到 `main`。
- 合并前 CI：最终验收 head 的 required [`ci-gate` job 94459062001](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31701888756/job/94459062001)、[`pr-title` job 94452789964](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31701888782/job/94452789964)、[`rust` job 94452930311](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31701888756/job/94452930311) 与 [CodeQL run 31701888795](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31701888795) 均成功；`change-scope`、`support-contract` 同样成功，frontend 与候选/发布 jobs 按 Rust-only scope 跳过。
- 收尾记录 PR：[PR #138](https://github.com/KNaiFen/aio-coding-hub/pull/138) 已合并；初始归档候选为 `a9b9ddbdf9930fe06be57137cd7bf9eaf1665184`。最终纯记录 head `ca7d6f5d8128e4563993e61de2ead5de1eabd5bc` 的 [`ci-gate` job 94466088880](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31705829035/job/94466088880)、[`pr-title` job 94466033531](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31705828981/job/94466033531)、[CodeQL run 31705828974](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31705828974) 与 `change-scope` 均成功；`docs-contract`、`support-contract`、frontend、rust 与候选/发布 jobs 按纯过程记录范围跳过。2026-08-13 以 squash merge commit `70f103467c5770c7a7a29f564b7a5620409fff5a` 进入 `main`。
- 知识库与合同：本任务没有产生新的长期知识、公共接口、协议或运维合同；现行 TUI 合同已覆盖请求卡片与详情的语义边界，无需新增知识库条目。
- 配置、迁移与 PENDING：无配置或数据迁移；`PENDING.md` 当前无未解决条目，本任务不新增或迁移 PENDING 项。
- 归档：main 已于 2026-08-13 运行 `python3 ./.trellis/scripts/task.py archive --no-commit 08-13-tui-request-card-ttfb`，任务已迁入 `.trellis/tasks/archive/2026-08/08-13-tui-request-card-ttfb/`，`task.json` 已更新为 `completed`；README 活动行同步转换为归档条目，随后运行全量 Trellis 校验。
- worktree 与分支清理：2026-08-13 以 `git ls-remote --heads origin` 核验功能分支与 records-only closeout 分支的远端 ref 均已删除；确认两个 worktree 均无已跟踪或未跟踪修改后，main 已删除原执行 worktree `/Users/knaifen/Documents/Codex/aio-coding-hub/08-13-tui-request-card-ttfb`、closeout worktree `/Users/knaifen/Documents/Codex/aio-coding-hub/08-13-tui-request-card-ttfb-closeout` 及本地 `fix/tui-request-card-ttfb`、`docs/close-tui-request-card-ttfb` 分支。
- 遗留风险：无。功能、归档与相关 worktree/分支清理均已完成。

## 返工记录

无。
