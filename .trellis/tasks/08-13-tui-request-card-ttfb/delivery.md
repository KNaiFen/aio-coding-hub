# 交付报告：TUI 请求卡片改用首字时间与紧凑路由计数

> 本文件描述 PR #136 的实际交付候选。功能候选及其云端证据固定如下；包含本文件的记录提交会形成新的仅记录 PR head，执行 session 仍会等待该实时最新 head 的自动检查全部绿色后再暂停。

## 交付状态

- 结果：等待验收
- PR：[#136](https://github.com/KNaiFen/aio-coding-hub/pull/136)
- 分支：`fix/tui-request-card-ttfb`
- PR base：`main` @ `875ff441c5ba9f1a7f235ad95dadb945a41bba61`
- 功能交付候选 head：`5b8414b9f6dfd156c702b4d229cde69d013136b6`
- 实时 PR head：本记录提交推送后以 [PR #136](https://github.com/KNaiFen/aio-coding-hub/pull/136) 为准；该提交只增加任务记录，不改变上述功能候选
- 规划提交：`bd91552393f36419ce215d9de283b7519c0efb07`
- `ci-gate`：功能候选通过，[job 94376200331](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31676371396/job/94376200331)
- 其他必需检查：功能候选的 `pr-title`、`rust`、`support-contract`、`change-scope` 与两项 CodeQL 均通过；`frontend` 按 Rust-only scope 跳过
- 交付时间：2026-08-13T15:29:41+08:00
- 执行 session：记录提交推送后停止写入；等待实时最新 head 的自动检查绿色后标记 Ready 并通知 main

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
| AC-06 本地合同与云端检查 | 通过（功能候选） | 四项允许的本地检查通过；功能候选的 `ci-gate`、`pr-title`、Rust 相关检查及 CodeQL 均绿色。记录提交的最新 head 仍须重新等待自动检查。 |

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
| `ci-gate` | 通过 | [功能候选 job 94376200331](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31676371396/job/94376200331) |
| `rust` | 通过 | [job 94371826810](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31676371396/job/94371826810)，覆盖云端格式、绑定导出、Clippy、Rust tests 与 audit。 |
| `support-contract`、`change-scope` | 通过 | [自动 PR workflow 31676371396](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31676371396)；scope 为 Rust-only。 |
| `pr-title` | 通过 | [run 31676371483](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31676371483) |
| CodeQL | 通过 | [run 31676371422](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31676371422)，JavaScript/TypeScript 与 Rust 均成功。 |
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

- 无。记录提交推送后只等待实时最新 PR head 的自动检查终态，不再修改实现或任务记录。

## 建议 main 重点审查

- `src-tauri/crates/aio-tui/src/format.rs:request_card_lines`：确认时间来源只改为 `ttfb_ms`，输出速率条件和其他卡片行保持不变。
- `src-tauri/crates/aio-tui/src/format.rs:request_card_route_result`：确认两个计数独立，且共享 `route_result` 没有被缩写。

## main 验收记录

> 仅 main 填写。

## main 收尾

> 仅 main 填写。

## 返工记录

无。
