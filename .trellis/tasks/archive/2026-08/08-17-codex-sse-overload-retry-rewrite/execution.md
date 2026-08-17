# 施工入口：Codex Responses 过载错误码改写

> 先调用 `$gkd-execute`。活动状态以 `python3 .trellis/scripts/task.py status 08-17-codex-sse-overload-retry-rewrite` 为准；本文件只保存任务特有边界。

## 权威材料

1. `AGENTS.md`
2. `.trellis/tasks/08-17-codex-sse-overload-retry-rewrite/prd.md`
3. `.trellis/tasks/08-17-codex-sse-overload-retry-rewrite/design.md`
4. `.trellis/tasks/08-17-codex-sse-overload-retry-rewrite/implement.md`
5. `docs/operations/multi-worktree/execution-and-delivery.md`
6. `.trellis/spec/aio-coding-hub/cross-layer/settings-ownership-rollback-contract.md`
7. `.trellis/spec/aio-coding-hub/cross-layer/gateway-failover-route-contract.md`
8. `src-tauri/src/gateway/proxy/handler/failover_loop/response/success_event_stream.rs`
9. `src-tauri/src/gateway/streams/usage_tee.rs`
10. 首次实施，无 `findings.md`

- 实施授权：已确认；2026-08-17，覆盖 PRD 的设置、UI、SSE 改写和测试范围
- PENDING：无未解决条目
- 依赖：无；现有 `08-03-upstream-claude-oauth` 仍为 legacy planning，范围不重叠

材料冲突或实施授权不明确时停止并报告 main，不用本摘要覆盖 PRD、设计或现行合同。

## 锁定边界

- 设置归 AIO `AppSettings`，默认关闭；不得写成 Codex `config.toml` 的 `[features]` 字段。
- 只匹配 native Codex Responses `response.failed` 的 `/response/error/code` 两个精确枚举。
- 只改下游客户端视图；usage、stream evidence 和请求日志保留原始上游码。
- 不改变 HTTP 状态、AIO pre-commit retry、provider failover、bridge 或 response fixer 语义。
- 开关放在“CLI 管理 -> Codex 设置”，文案不得暗示上游是 OpenAI 官方服务。

## 实现自由度

- 可决定改写器的模块名、内部状态机和缓冲上限，但必须满足 PRD 的 fail-open、无丢失和范围测试。
- 可在保持单一判定源的前提下调整共享 SSE helper 的 crate 内可见性。
- 可复用现有 Codex OAuth 兼容设置卡片的布局/持久化模式，或建立同层新卡片；不得把页面拆成无关重构。
- 在 `delivery.md` 解释目标帧重序列化策略、超限旁路策略和 generated bindings 的来源。

## 范围

### 必须完成

- I-01 至 I-06 全部步骤。
- PRD AC-01 至 AC-08 的自动化或 CI 证据。
- `delivery.md`、任务提交、任务 PR 与最终 head CI。

### 允许修改

- `src-tauri/src/infra/settings/`：设置字段、默认值和兼容测试。
- `src-tauri/src/app/settings_service.rs`、`src-tauri/src/commands/settings.rs`：普通 owner patch/view/binding 边界。
- `src-tauri/src/gateway/`：请求快照、native SSE 判定、流改写器与测试。
- `src/generated/bindings.ts`：仅应用 GitHub 生成的本字段有界 drift。
- `src/services/settings/`、`src/query/`、`src/test/`：前端设置 adapter、写入合同和 fixtures。
- `src/pages/cli-manager/`、`src/pages/CliManagerPage.tsx`、`src/components/cli-manager/tabs/CodexTab.tsx` 及相邻测试：Codex 设置开关。
- `.trellis/tasks/08-17-codex-sse-overload-retry-rewrite/delivery.md` 与 task state：交付证据。

### 范围外

- Codex CLI 源码、模型/provider 定义、Codex `config.toml` structured fields。
- HTTP 非流式错误改写、默认 retry keyword、其他错误码或 CLI 的通用转换器。
- 与本功能无关的 UI 重设计、settings owner 重构或 gateway relay 重构。
- 真实凭据、中转站 URL、发布/版本号、main 合并和归档。

### 并行冲突

- 与 `08-03-upstream-claude-oauth` 无已知共享文件或语义冲突。
- 若实时 main/其他任务修改 settings service、CodexTab、success SSE 或 usage relay，停止并报告 main 重新判断 merge 顺序。

## AC 与证据入口

| AC | 执行结果入口 | 需要的证据 |
|---|---|---|
| AC-01 / AC-07 | `src-tauri/src/infra/settings/`、`src-tauri/src/app/settings_service.rs`、frontend settings tests | 默认、缺失字段、显式 patch、owner token/rollback 与 adapter 测试 |
| AC-02 / AC-03 / AC-04 | gateway streams 新改写器及其单测 | 两个码、chunk/LF/CRLF/multi-frame、完整负例与 fail-open |
| AC-05 | `src-tauri/src/gateway/proxy/handler/failover_loop/response/success_event_stream.rs`、route tests | `Content-Length` 与原始 request-log evidence 断言 |
| AC-06 | `CodexTab.tsx`、page data model 与组件测试 | 控件位置、读取、保存、禁用/失败状态 |
| AC-08 | `delivery.md` | 本地固定 runner、PR、final head、`ci-gate`、`pr-title` |

## 验证

- 本地允许：只调用 `$gkd-local-verify` 规定的 `node scripts/check-local-verification.mjs --base <登记完整 SHA>`。
- GitHub：自动 `ci-gate`、`pr-title`，其中必须覆盖 generated bindings、frontend typecheck/lint/tests、Rust fmt/Clippy/tests。
- 人工/环境：无需真实中转站或凭据；确定性 SSE upstream fixture 即可证明协议改写。

不得运行 `AGENTS.md` 禁止的依赖安装、构建或测试。未运行项必须在 `delivery.md` 说明原因。

## 任务特有停止条件

- 必须把功能扩大到 HTTP 错误、非原生 bridge 或第三个错误码才能满足测试。
- 需要改变公共 API、数据库 schema、发布配置或 Codex `config.toml` 合同。
- 无法在 usage tee 后改写而不丢失原始 request-log evidence。
- generated bindings 出现本字段之外的材料性 drift。
- 与实时 main 或活动任务发生 settings/gateway/CLI Manager 共享写冲突。

通用阻塞、交付和恢复命令见执行专题。停止时先持久化证据和恢复条件，再暂停。

## 当前返工

- 未解决 finding：无
- 本轮只处理：首次实施
- 保持不变：现有未命中行为、所有非目标 gateway 路径与设置所有权合同
