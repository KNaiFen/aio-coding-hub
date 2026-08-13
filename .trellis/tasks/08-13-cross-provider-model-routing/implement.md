# 实施计划：跨供应商模型路由

> 执行 session 按顺序完成本文件；每一步都必须满足“完成信号”后再进入下一步。每个阶段可以产生一个逻辑提交，提交前运行本阶段允许的最小本地检查；不得把多个未验证的大阶段堆成一个提交。所有代码、测试和合同修改仍须服从 `prd.md`/`design.md`。

## 0. 开工门与任务初始化

### 目标

确认 worktree、分支、规划提交和依赖 PR 正确；激活任务后才允许产品代码写入。

### 操作

1. 在 `/Users/knaifen/Documents/Codex/aio-coding-hub/08-13-cross-provider-model-routing` 读取项目合同入口 `.trellis/workflow.md`、`docs/operations/multi-worktree-delivery.md`、`docs/operations/templates/execution.md`、本任务 `execution.md`、`prd.md`、`design.md`、`implement.md`、以及 `implement.jsonl/check.jsonl` 中列出的现行 spec。当前 checkout 没有仓库根 `AGENTS.md`；不要把不存在的路径当作开工条件，项目级边界以用户提供的合同和上述文件为准。
2. 运行：
   ```bash
   test "$(pwd)" = "/Users/knaifen/Documents/Codex/aio-coding-hub/08-13-cross-provider-model-routing"
   test "$(git branch --show-current)" = "feat/cross-provider-model-routing"
   BASE_SHA="<execution.md 中登记值>"; test "$(git merge-base "$BASE_SHA" HEAD)" = "$BASE_SHA"
   git status --short --branch
   ```
3. 在 main 确认 PR #136 已合并后，main 更新 `prd.md`、`execution.md`、任务索引中的 base/head 事实；执行 session 不自行改基线。
4. 由 main 执行 `task.py start <task-dir>`，确认 `task.json.status=in_progress`；执行 session 不提前 start 或写产品代码。
5. 尽早创建指向 `main` 的 Draft PR，PR 正文链接 `.trellis/tasks/08-13-cross-provider-model-routing/`，注明本任务不改 TUI `format.rs`。

### 完成信号

- 开工核验全部成功；任务状态为 `in_progress`；Draft PR 已创建或因 GitHub 暂时不可用已在 `delivery.md` 如实记录；工作树只有规划提交内容。

## 1. 规则类型、sanitizer 和强度协议

### 修改范围

- `src-tauri/src/infra/settings/types.rs`
- `src-tauri/src/gateway/configured_model_route.rs`
- `src-tauri/src/gateway/proxy/handler/middleware/model_inference.rs`
- `src-tauri/src/gateway/proxy/handler/middleware/body_reader.rs`（只在需要捕获插件前原始 body 时）
- `src/services/gateway/modelRoutingPolicy.ts`
- `src/services/settings/settings.ts` 及其测试
- 对应 Rust/TS route tests

### 要求

1. 新增 source effort 字段、独立 cross policy/rule 类型和 specta 类型；旧三字段反序列化不变。
2. 统一后端/前端标准 effort 集合、trim/lowercase/长度/控制字符校验；普通旧 model-only 规则继续可用。
3. 将 `resolve` 改为纯确定性匹配 + route 结果；固定四级优先级、大小写和重复键校验。优先级为四级（跨精确、跨 source wildcard、普通精确、普通 source wildcard）；不引入 model wildcard/default rule；旧 model-only 普通规则继续命中。
4. 在插件前捕获标准来源 effort marker；预算字段不参与匹配，marker 有界且不含 body。
5. Gemini 出站只写 `thinkingLevel`，更新旧数字预算测试。

### 完成信号

- 单元测试覆盖五个协议入口（Codex Responses、Claude Messages、Gemini generate/streamGenerate、Grok Chat、Grok Responses）的来源 pointer、非法/预算不命中、精确/通配优先级、旧规则兼容、aio/和辅助请求排除；前端 validate/normalize 测试通过。
- `git diff --check` 通过；提交 `feat: add cross-provider routing policy types`（或等价清晰消息）。

## 2. SQLite schema、方案 UUID 与 domain/query

### 修改范围

- `src-tauri/src/infra/db/migrations/mod.rs`
- `src-tauri/src/infra/db/migrations/baseline_v25.rs`
- 新增 `v52_to_v53.rs`（及需要的 ensure patch）
- `src-tauri/src/domain/sort_modes.rs`
- `src-tauri/src/domain/providers/types.rs`
- `src-tauri/src/domain/providers/queries.rs`
- `src-tauri/src/commands/sort_modes.rs`、`src-tauri/src/commands/providers/crud.rs` 或新 routing command 模块
- Rust migration/domain tests

### 要求

1. 为 sort mode 生成/保留 UUID；使用 `sort_mode_identities` 独立表保存 `mode_id -> mode_uuid`，不对已有 `sort_modes` 直接 `ALTER ... ADD NOT NULL UNIQUE`，不以名称或 numeric ID 做长期引用。
2. 为 mode member 持久化 cross policy；Default 永远没有跨规则列/写入口。
3. SQL mapper 同时带出 provider UUID、mode UUID、ordinary override 和 cross policy；坏 JSON fail-open。
4. 新增组合 read/write 与窄 provider candidate IPC，输入进行 provider/mode UUID identity check；使用现有 settings/db owner-scoped transaction，不能 snapshot-read 后 whole-snapshot 覆盖。验收必须证明并发 writer、身份错配、CAS loser 和失败回滚不覆盖其他 owner 字段。
5. 删除/禁用/重命名/成员顺序和 session generation 行为与现有 sort mode 合同一致；保存跨策略不改变 session binding。

### 完成信号

- migration tests 覆盖 fresh、v52->v53、重复运行、UUID backfill/immutability、坏 JSON、删除 cascade、事务失败回滚；domain tests 覆盖 Default 禁止、改名保留、member enable 和身份错配拒绝。
- Rust source contracts 可静态看到 `mode_uuid`/provider UUID 绑定；提交 `feat: persist mode-scoped cross-provider policies`。

## 3. Provider selection 快照与组合 IPC/前端数据层

### 修改范围

- `src-tauri/src/gateway/proxy/handler/provider_selection.rs`
- `src-tauri/src/gateway/proxy/handler/middleware/provider_resolution.rs`
- `src-tauri/src/gateway/proxy/request_context.rs`
- `src/services/providers/sortModes.ts`
- `src/services/providers/providers.ts`、新 `routingProviders.ts`（若需要）
- `src/query/` 对应 query keys/invalidation
- `src/pages/providers/ProvidersView.tsx`、数据模型 hook

### 要求

1. 每请求捕获 effective mode ID/UUID 和完整命名方案成员快照；不在 failover 中重新读取 active mode，也不把目标 B 限制为当前 baseline 顺序成员。
2. provider candidate DTO 只返回窄、无凭据字段；前端按 UUID key 缓存，upsert/delete/import 后精确失效。
3. 方案切换前处理 dirty cross draft（save/discard/cancel）；普通 draft 不被意外覆盖。
4. 当前 provider 非 member 时保留普通编辑，禁用跨编辑；Default 禁用跨编辑。

### 完成信号

- IPC/service/query tests 覆盖 UUID scope、缓存隔离、切换草稿、Default/非成员禁用和窄字段不泄露；提交 `feat: expose scoped routing editor data`。

## 4. 供应商编辑器与严格控件

### 修改范围

- `src/components/gateway/ModelRoutingPolicyFields.tsx`
- `src/pages/providers/ProviderEditorDialog.tsx`
- `src/pages/providers/ProvidersView.tsx` 及 provider editor hook/test
- 如需目录建议，复用 `src/services/providers/providerModels.ts`，不扩大其凭据输出

### 要求

1. 普通/跨规则分区明确；普通规则仍是供应商整份覆盖，跨规则绑定当前 named mode。
2. source/target effort 使用严格 Select；默认本供应商；目标模型可留空/自由输入；目标 provider 只列同 CLI、同 mode、启用成员。
3. 无效 target 显示失效提示而不静默改写；开关关闭保留规则但不生效。
4. 供应商编辑器保存组合 DTO，处理 loading/error/dirty/切换确认；不把 route mode 规则写入全局设置。

### 完成信号

- 前端组件测试覆盖新增/删除、默认目标、Select 值、Default/非成员禁用、失效行、保存/放弃草稿和目录建议；`node --check` 仅对变更 `.mjs`（如有）执行；提交 `feat: add scoped routing editor`。

## 5. Failover 一次性临时 B

### 修改范围

- `src-tauri/src/gateway/proxy/handler/failover_loop/mod.rs`
- `src-tauri/src/gateway/proxy/handler/failover_loop/context.rs`
- `src-tauri/src/gateway/proxy/handler/failover_loop/prepare/provider_iterator.rs`
- `src-tauri/src/gateway/proxy/handler/failover_loop/attempt/` 与 response/finalize 相关上下文（仅为传递 route override/guard）
- `src-tauri/src/gateway/streams/types.rs`
- `src-tauri/src/gateway/streams/finalize.rs`
- `src-tauri/src/gateway/proxy/handler/failover_loop/response/success_non_stream.rs`
- failover/session/gateway route tests

### 要求（必须按此状态机实现）

1. 将当前 provider `for` 循环抽为 baseline + `ProviderWorkItem` 调度；baseline Vec 不可变，CrossTemporary 携带完整目标 provider 快照，不使用 `target_index`。
2. A 命中跨规则时只创建一个 CrossTemporary B；B 标记 processed，`cross_jump_used=true`；B 不触发自身规则。
3. B 完全复用公共 prepare/retry/gate/limit/circuit/auth；Ready 名额、attempt row、route hop 和失败判定与普通 provider 一致，并逐项遵循 failover contract 对 limit/OAuth 预过滤、common-gate skip、credential/bridge 失败的既有计数规则。
4. B 失败且 failover 可继续时恢复原始 requested model，继续 A/C 基线；B 不重复；A/C 普通规则照常。
5. 传递 `session_binding_allowed=false` 到非流/SSE 成功绑定点；B 成功不绑定，B 失败后 A/C 仍可按旧逻辑绑定。
6. 临时 route marker、configured marker、最终 provider/model/price、stream/non-stream 终态一致；禁止手工 send 或重跑 selection/session API。

### 完成信号

- Rust route tests 至少覆盖：B success、B HTTP/transport/gate/credential failure、B 已在 baseline、B 不在 baseline 但为方案成员、B=A/不存在/禁用/circuit-open、max providers 1/2、B success SSE/non-SSE、B 不绑定、B 失败后 A 绑定、普通规则恢复、managed alias/辅助请求排除、跨跳单次；另逐项断言每类 gate skip 的 attempt/route/Ready 计数。
- `git diff --check` 通过；提交 `feat: execute one-shot cross-provider failover`。

## 6. 日志、费用、导入/分享/复制

### 修改范围

- `src-tauri/src/gateway/response_fixer.rs`、request end/attempt projection、usage ledger/cost helpers
- observer/snapshot projection 与 desktop request detail adapters
- `src-tauri/src/infra/config_migrate/{mod,export,import}.rs` 及 tests
- `src-tauri/src/domain/providers/share.rs`、provider duplicate paths/tests
- 现行合同必须按 PRD AC-09 实际同步，不能用“如实现改变公共行为”跳过：
  `configured-model-routing-contract.md`、`gateway-failover-route-contract.md`、
  `config-migration-skill-bundle-contract.md`（强制 v5 schema）、`provider-share-contract.md`、
  `local-observer-tui-contract.md`、`settings-ownership-rollback-contract.md`。
  只更新 shipped template（`src/templates/markdown/spec/...`）的合同，如果当前仓库存在对应模板；
  缺失的 configured routing/provider share/observer TUI 模板不在本任务补造，必须在 delivery 中明确未新增模板。

### 要求

1. 完整 bundle v5 保存 mode/provider UUID 和 cross policy；旧 bundle 生成 UUID、无 cross；未知目标保留 invalid projection；导入预检在 destructive lock 前完成；同步更新 `config-migration-skill-bundle-contract.md` 的 v5 schema 与兼容阈值。
2. share 只输出普通 provider override；duplicate 不复制 mode member/cross；override off 保留但不执行。
3. attempts 保留完整链；marker bounded/fail-open；final provider/model 驱动 cost/ledger；不伪造 switch count；同步更新 configured-routing、failover、provider-share、observer/TUI、settings ownership 合同的跨规则章节或明确兼容条款。

### 完成信号

- config migration tests 覆盖 v1-v4/v5、UUID rebind、missing target invalid、malformed/legacy effort deletion、share strip、duplicate isolation；request log/usage/observer tests 覆盖 marker、attempts、cost and final projection。
- 现行合同与生成类型同步；完成合同中的 wildcard/effort/Gemini、临时 B 计数、bundle v5、share strip、observer fail-open、settings owner/CAS 章节；提交 `feat: persist and project cross-route audit`。

## 7. 绑定、合同和 CI 漂移收敛

### 操作

1. 检查 `git diff --name-only origin/main...HEAD`，确认不含 `src-tauri/crates/aio-tui/src/format.rs`、凭据文件、构建产物或无关重构。
2. 不在本地运行 Cargo、Rust tests、格式化、Specta 生成、pnpm、lint、typecheck 或 build。Rust/前端/生成绑定由 Actions 负责。
3. 推送后阅读 `change-scope`、`ci-gate` 和生成 drift artifact。若 drift 仅是本任务对应的 generated bindings，采用 CI 给出的精确文件补丁，提交独立 `chore: sync generated bindings`；若 scope 选中不应触发的长 job，立即暂停报告 main。
4. 运行允许的：
   ```bash
   node scripts/check-cloud-only-verification.mjs
   node scripts/check-cloud-only-verification.selftest.mjs
   python3 ./.trellis/scripts/task.py validate .trellis/tasks/08-13-cross-provider-model-routing
   git diff --check origin/main...HEAD
   ```

### 完成信号

- 最新 head 的必需 `ci-gate`、`pr-title`、相关 frontend/rust/shared/docs jobs 绿色；绑定/合同无 drift；`delivery.md` 已基于实际代码填写。

## 8. 交付、暂停和 main 验收

1. 将 PR 标记 Ready for review，停止写入 sibling；不得 merge、archive、delete worktree 或运行 `/trellis:finish-work`。
2. 在 `delivery.md` 记录完整 base SHA、规划 SHA、PR/head SHA、ci-gate URL、每条 AC、关键符号、实际验证、偏移、风险和人工验证缺口。
3. 向 main 发送暂停交接；main 读取实时 PR diff/CI 和 delivery，必要时创建 `findings.md`。新提交会使旧候选验收失效。
4. main 验收通过后合并 PR、快进 main、更新知识库/PENDING、archive --no-commit、validate --all，再清理干净且无人使用的 sibling；执行 session 不做这些动作。

## 提交点总表

| 点 | 建议提交内容 | 失败/回滚点 |
|---|---|---|
| P0 | 规划材料（main，单独规划 checkpoint） | 材料错误时只回退规划提交，不接触产品代码 |
| P1 | 类型、sanitizer、强度提取和 route matcher | 可独立回退，旧三字段仍可读 |
| P2 | schema/UUID/domain/query/IPC | migration transaction 回滚；不得删除旧列 |
| P3 | selection snapshot + UI data layer | 回退不改变既有 Default/普通规则 |
| P4 | editor controls | 回退只影响 UI/组合 IPC，不改变 runtime |
| P5 | failover work item + binding guard | 回退恢复原 for-loop/普通 route；保留新增列向后兼容 |
| P6 | migration/share/log projection/contracts | 依据 marker fail-open 和 UUID 迁移回滚 |
| P7 | generated drift/CI fixes | 只接受同一 head 的 CI artifact，禁止跨 SHA 复制 |

## 测试矩阵（云端执行）

| 维度 | 必测情形 | 证据位置 |
|---|---|---|
| 规则 | model-only、source effort exact/wildcard、大小写、重复键、五协议入口 pointer | configured route/model inference tests |
| 方案 | Default、active named、会话绑定旧 mode、改名、删除、切换、member disable | sort mode/domain/selection tests |
| 临时 B | success、gate/credential/transport/http fail、B duplicate、Ready cap、single hop | gateway routes/failover tests |
| 会话 | B non-stream/SSE success 不 bind；B fail 后 A/C 正常 bind；generation race | session manager/route tests |
| 协议 | Codex Responses、Claude Messages、Gemini generate/streamGenerate、Grok Chat、Grok Responses；stream/non-stream；Gemini budget only 不命中 | middleware/configured route tests |
| 数据 | v1-v4/v5 bundle、UUID rebind、missing target invalid、malformed fail-open | config_migrate tests |
| 分享/复制 | cross stripped、new UUID、disabled/no route membership、ordinary preserved | provider share/duplicate tests |
| 投影 | attempts 完整、final provider/model/cost、marker bounded、switch count unchanged | request logs/usage/observer tests |
| 安全 | candidate DTO 无 secret、marker 无 body/URL、aio/与辅助请求排除 | service/domain/route tests |
| 交付 | bindings drift、cloud-only checker、scope classification、PR title/gate | Actions checks + delivery.md |
