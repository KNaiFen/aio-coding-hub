# 施工入口：Codex 配置生命周期与 372K 开关

> 先调用 `$gkd-execute`。活动状态以 `python3 .trellis/scripts/task.py status .trellis/tasks/08-17-codex-372k-context-window` 为准；本文件只保存任务特有边界。

## 权威材料

1. `AGENTS.md`
2. `.trellis/tasks/08-17-codex-372k-context-window/prd.md`
3. `.trellis/tasks/08-17-codex-372k-context-window/design.md`
4. `.trellis/tasks/08-17-codex-372k-context-window/implement.md`
5. `docs/operations/multi-worktree/execution-and-delivery.md`
6. `.trellis/spec/aio-coding-hub/cross-layer/codex-config-contract.md`
7. `.trellis/spec/aio-coding-hub/cross-layer/codex-managed-model-route-contract.md`
8. `src-tauri/src/infra/codex_config/mod.rs`
9. `src-tauri/src/infra/cli_proxy/{mod,codex}.rs`
10. `src-tauri/src/infra/mcp_sync/sync.rs`
11. `src-tauri/src/infra/codex_model_catalog/managed.rs`
12. `src/components/cli-manager/tabs/CodexTab.tsx`
13. `src/pages/cli-manager/useCliManagerPageDataModel.ts`
14. `src/query/cliManager.ts` and `src/services/cli/cliManager.ts`
15. 首次实施，无 `findings.md`

- 实施授权：已确认；2026-08-17，覆盖 PRD R1-R7、AC1-AC16，并确认事务硬化与 372K 合并为一个任务
- PENDING：无未解决条目
- 依赖：无；本任务内部先建立安全事务边界，再通过同一边界落地 372K policy

材料冲突或实施授权不明确时停止并报告 main，不用本摘要覆盖 PRD、设计或现行合同。

## 锁定边界

- canonical/direct 是用户语义基线，live 是 AIO overlay 投影；禁止从 projected live 刷新 direct backup。
- structured/raw config、MCP、proxy、catalog/profile、372K、startup 和 exit 使用一个 coordinator、固定锁序和 ownership-aware rollback。
- 外部 drift 获胜；tests 使用 temp home，禁止真实用户配置和敏感内容日志。
- UI 只提交 boolean；后端固定拥有三个 slug、`372000`、目录来源/变换/路径和副作用。
- 只改六个 context 值；percent、其他模型/字段/顺序、auto-compaction、global window 和 `models_cache.json` 不变。
- 只有一个完整 managed catalog owner；关闭精确恢复 pointer/absence 或保留 profile-only catalog。
- 不 refresh/restart/request/rescan，不显示或宣称运行时 `372000/372000/95` 验证。

## 实现自由度

- 可扩展现有 backup/manifest 或新增私有 coordinator/journal 类型；保持公共命令和用户配置语义兼容。
- 可选择内部写入顺序、journal schema、generation/hash、error 名称、preference 落点和 query/command 结构，只要满足 PRD 合同。
- 可重构/封装现有写 helper 和 catalog plan；删除或替换调用点必须在 `delivery.md` 逐项说明。
- 并发测试必须使用 barrier/failpoint，不得用 sleep 猜测顺序。

## 范围

### 必须完成

- `implement.md` Work Package 1-6：合同/fixture、coordinator、所有写者迁移、catalog policy/preference、startup/exit、UI 和完整回归。
- Work Package 7：固定本地验证、交付文档、PR 和固定 final-head CI。
- AC1-AC15 的 deterministic file/state evidence；实际 Codex 采用 372K 不属于验收。

### 允许修改

- `src-tauri/src/infra/codex_config/**`、`cli_proxy/**`、`mcp_sync/sync.rs`、`codex_model_catalog/**`。
- 相关 Codex app service/startup/cleanup/resident lifecycle。
- app settings persistence/types/migration、CLI manager commands/services/queries、generated bindings。
- `CodexTab.tsx`、page data model、focused frontend tests。
- 两份 Codex cross-layer specs 和本任务 `delivery.md`、必要 findings/证据、`task.json`。

### 范围外

- TUI、gateway routing/request bodies、provider policy、MCP payload semantics、model capabilities/pricing、refreshable cache、auto-compaction、runtime validation、versions/releases。
- 无关设置、代理、数据库或前端重构。

### 并行冲突

- 与 `08-17-tui-observability-consistency` 无主文件或语义冲突，可并行实施。
- 所有 Codex config/catalog/frontend/spec 文件由本 worktree 唯一 writer 修改，不再拆分 transaction 子任务。

## AC 与证据入口

| AC | 执行结果入口 | 需要的证据 |
|---|---|---|
| AC1-AC2 | config/proxy/MCP coordinator temp-home tests | canonical save/projection/disable/exit、direct backup 无 proxy-only 键、re-sync 幂等 |
| AC3-AC5 | barrier/failpoint/journal/startup/exit tests | 全写者无 lost update/deadlock、外部 drift 保留、owned rollback 和每阶段恢复 |
| AC6 | CodexTab/page/query/service tests | authoritative toggle、pending disable、串行 mutation、现有反馈 |
| AC7-AC10 | catalog pure/lifecycle tests | 恰好六值、其他 JSON/percent/auto-compact 不变、两种 base、cache sentinel、invalid fail closed |
| AC11-AC12 | catalog/config/profile/proxy/restart tests | pointer restore/profile-only rebuild、幂等组合和无关编辑保留 |
| AC13 | frontend/backend negative assertions | 无 refresh/restart/request/rescan/runtime status |
| AC14 | temp-home/platform/log tests | 注释/未知键、路径/size/symlink/reparse、无真实 home/敏感日志 |
| AC15-AC16 | specs/bindings/`delivery.md`/fixed verify/PR checks | 合同同步、claim 边界、完整 base/head、`local_ready` 和同一 final head CI |

完整可观察结果只写在 `prd.md`，这里不复制 Given/When/Then。

## 验证

- 本地允许：仅 `$gkd-local-verify` 要求的 `node scripts/check-local-verification.mjs --base <完整 base SHA>`。
- GitHub：自动 `ci-gate`、`pr-title`，以及 frontend unit/typecheck/lint、generated bindings、Rust fmt/Clippy/check/tests、spec/contract 和平台检查。
- 人工/环境：不验证 Codex 实际采用 372K；用户后续实测反馈不阻塞 artifact/state 交付。

不得运行 `AGENTS.md` 禁止的依赖安装、构建或测试。未运行项必须在 `delivery.md` 说明原因。

## 任务特有停止条件

- 需要破坏公共 API、进行可能丢失用户数据的迁移、改变凭据处理或弱化路径安全。
- 任一写者无法进入共享 coordinator，或 drift/rollback 无法保证不覆盖后来写入。
- catalog 无法在一个 owner 中组合 372K/profile 或保留未知/非目标语义。
- 实现需要触碰 cache/global window/auto-compaction/真实 home/runtime request。
- 上游 drift 使 backup/catalog/proxy/MCP/startup/exit 合同失效，且无法在任务内安全修复。

通用阻塞、交付和恢复命令见执行专题。停止时先持久化证据和恢复条件，再暂停。

## 当前返工

- 未解决 finding：无
- 本轮只处理：首次实施；包含合并前独立计划中的 transaction hardening
- 保持不变：六值变换、用户配置安全、无 cache/auto-compact 修改、无运行时验证声明
