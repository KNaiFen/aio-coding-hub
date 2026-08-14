# 施工入口：跨供应商模型路由

> 执行提示：按照本文件和它列出的任务材料施工。完成实现、PR、CI 和 `delivery.md` 后暂停，等待 main 验收。你是本 sibling worktree 的唯一写者；不要另建 worktree、不要派生子任务，也不要让 main 与你同时写产品文件。

## 快速定位

- 任务目录：`.trellis/tasks/08-13-cross-provider-model-routing/`
- Worktree：`/Users/knaifen/Documents/Codex/aio-coding-hub/08-13-cross-provider-model-routing`
- 分支：`feat/cross-provider-model-routing`
- 基线：`origin/main`
- 完整 base SHA：`875ff441c5ba9f1a7f235ad95dadb945a41bba61`（两个 sibling 均从该完整 `origin/main` SHA 派生；最终集成时由 main 处理分支漂移）
- 规划提交：`c6d59507c7a1de46abdb07427aa8bc153c69739c`（包含并行 sibling 边界、PRD、设计、实施计划、施工入口、初始交付记录和活动索引）
- 历史 checkpoint `71392b672ee665b6ee96e13bf3871b2816185873` 与其后 `2b8e52e7071fb59cc54a8082bb9bc05f10b8cf1c`、`01915697174eacd623c4e75a03cc10030cde2f9c` 是本次修订之前的计划登记；执行 session 以本 checkpoint 中的 PRD/设计/实施内容为权威，不得把早期“等待 TUI 合并”表述当作现行门禁。
- 实施授权：已确认（2026-08-13；用户确认本任务全部 PRD 决定、AC 和单 sibling 端到端交付路线）
- PR 目标：`main`
- PR：[KNaiFen/aio-coding-hub#137](https://github.com/KNaiFen/aio-coding-hub/pull/137)（Draft；Round 2 已拒绝冻结 head `2e7a8e284ff3b3e60678150eec0b07768f4db3a2`）
- 并行 sibling：TUI PR #136 已合并并完成归档；其产品边界仍只包含 TUI formatter。本任务不得修改 `src-tauri/crates/aio-tui/src/format.rs`，同步 `origin/main` 时须保留该 sibling 的归档与索引事实。
- PENDING 审阅：`PENDING.md` 已审阅，当前无 `pending`/`planned` 条目
- 当前唯一写者：`main-direct-fix`（main 于 2026-08-14 09:49:29 CST 临时接管记录性文档）。执行 session 已明确暂停；接管冻结 head 为 `2e7a8e284ff3b3e60678150eec0b07768f4db3a2`，接管前工作树干净，本地、远端分支与 PR head 一致，无未提交内容。main 仅写 Round 2 验收记录与返工指导，不修改产品代码、测试、依赖或现行合同。
- 当前阶段：Round 2 验收不通过，`F-002`～`F-005` 已落盘；记录提交推送后把唯一写权交回执行 session 待返工。

## 阅读顺序

1. 项目合同入口 `.trellis/workflow.md` 和 `docs/operations/multi-worktree-delivery.md`。
2. `docs/operations/templates/execution.md`（字段要求）和 `docs/operations/templates/delivery.md`（交付字段）。
3. 本文件。
4. `prd.md`。
5. `design.md`。
6. `implement.md`。
7. `implement.jsonl`、`check.jsonl` 引用的现行规范。
8. 相关代码：先看任务材料列出的符号，再扩展到调用者/测试；不要以聊天摘要代替当前代码。
9. `findings.md`（仅 main 验收返工轮次）。

材料冲突时，按 `prd.md` 的用户需求/AC、`design.md` 的技术设计、`implement.md` 的步骤、本文施工导航判断；不清楚或冲突就暂停报告 main。

## 冻结交接与开工核验

开始任何产品代码或测试逻辑写入前必须全部成功：

```bash
test "$(pwd)" = "/Users/knaifen/Documents/Codex/aio-coding-hub/08-13-cross-provider-model-routing"
test "$(git branch --show-current)" = "feat/cross-provider-model-routing"
BASE_SHA="875ff441c5ba9f1a7f235ad95dadb945a41bba61"
test "$(git merge-base "$BASE_SHA" HEAD)" = "$BASE_SHA"
test -f .trellis/tasks/08-13-cross-provider-model-routing/task.json
test -f .trellis/tasks/08-13-cross-provider-model-routing/prd.md
test -f .trellis/tasks/08-13-cross-provider-model-routing/design.md
test -f .trellis/tasks/08-13-cross-provider-model-routing/implement.md
test -f .trellis/tasks/08-13-cross-provider-model-routing/execution.md
```

然后确认：

1. `task.json.status=in_progress`，不是 `planning`；只有 main 在计划审阅后运行 `task.py start`。
2. 本文件已经回填真实规划提交 SHA，且该提交存在。
3. TUI sibling PR #136 的文件边界已核对；未合并不影响本任务实现。确认本任务不修改 `src-tauri/crates/aio-tui/src/format.rs`，且当前 worktree 以登记 base SHA 为 merge-base。
4. worktree 没有来源不明或其他 session 的未提交内容；当前唯一写者已登记为本执行 session。

任一项失败都只报告 main，不通过聊天猜测或自行修复基线。

## 施工摘要（权威材料在同目录）

### 期望结果

- 供应商覆盖编辑器可按当前 named sort mode 配置普通规则与跨供应商规则；Default/方案外/跨 CLI/`aio/...` 均被排除。
- 来源强度只匹配标准字符串；跨规则最多一次临时 A->B，B 走公共 gate/重试/熔断/Ready 限额，失败后按原基线恢复。
- 完整备份、分享、复制、日志、费用、observer/桌面投影和 generated bindings 与 PRD/设计一致。

### 不可漏掉的完成信号

1. v53/v5（或现行实现选择的等价版本）迁移、UUID、组合 IPC 和坏数据 fail-open 有测试。
2. 五个协议入口（Codex Responses、Claude Messages、Gemini generate/streamGenerate、Grok Chat、Grok Responses）的来源强度和 Gemini `thinkingLevel` 只写规则有测试。
3. B success/failure、Ready cap、去重、session binding guard、SSE/non-SSE 有测试。
4. import/share/duplicate/cost/attempts/marker/最终投影有测试。
5. 最新 PR head 的必需 CI 绿色，`delivery.md` 绑定该 head/base/ci-gate 并且执行 session 已暂停。

## 已锁定决定（执行者不得改变）

- 跨供应商规则只存在于供应商覆盖，绑定当前 selected named mode；全局策略和 Default 不支持跨目标。
- 目标默认本供应商；目标只能是当前 CLI、当前 named mode 的启用成员；当前 provider 非成员时只禁用跨区块，普通规则仍可编辑。
- 强度集合固定为 `none|minimal|low|medium|high|xhigh|max|ultra`；预算字段不参与来源匹配；Gemini 目标只写 `thinkingLevel`；旧非标准目标规则整条删除。
- 匹配优先级为跨精确、跨通配、普通精确、普通通配；一次请求最多一次跳转；B 不链式、不更新/清除 session binding；失败按现有 failover 判定恢复 A->C 基线。
- `aio/...`、模型列表、token count、probe、discovery、非推理请求排除；不泄露 body、URL、凭据；不伪造 provider switch count。
- 本任务不修改 `src-tauri/crates/aio-tui/src/format.rs`，不承担 TTFB 或 `切/重` 文案。

完整决定和 AC 以 `prd.md` 为准；技术实现细节以 `design.md` 为准。

## 实现自由度

- 可以在保持字段/键/状态机语义不变的前提下选择 JSON 列或等价子表、命令命名、模块拆分和测试 fixture 组织；偏移必须写入 `delivery.md`。
- 可以复用现有 provider model catalog，只对已支持的 provider 提供建议；不得伪造其他 CLI 的目录能力。
- 可以按当前代码选择 v52->v53/v4->v5 的具体迁移文件名；版本递增、幂等、旧 bundle/数据库兼容和 UUID 绑定语义不可改变。

## 范围

### 必须完成

- Rust 类型、sanitizer、来源强度提取、匹配器、方案 UUID/成员策略、迁移、组合 IPC、候选窄投影。
- provider selection 快照、failover 外层临时 work item、公共 gate 复用、模型恢复、去重、session binding guard。
- 前端普通/跨规则编辑器、严格 effort Select、方案/草稿/无效目标状态、目录建议。
- 完整 config backup/import、single-provider share、duplicate、request marker、attempts/cost/observer/desktop 投影、合同和测试。

### 允许修改

- `src-tauri/src/infra/settings/`、`gateway/`、`domain/providers/`、`domain/sort_modes.rs`、`commands/`、`infra/db/migrations/`、`infra/config_migrate/`、相关 tests。
- `src/components/gateway/`、`src/pages/providers/`、`src/services/providers/`、`src/services/gateway/`、`src/query/`、相关 tests。
- 相关现行 `.trellis/spec/aio-coding-hub/{backend,cross-layer}/` 合同、generated bindings（仅由 CI drift 证明需要时）和任务材料。

### 明确禁止触碰

- `src-tauri/crates/aio-tui/src/format.rs` 及 TUI TTFB/`切/重` 展示实现。
- `aio/...` 受管 Codex 路由固定绑定、跨 CLI 目标、链式跳转、方案外目标。
- 无关重构、依赖/锁文件（除 CI 证明的绑定/锁同步）、发布/签名/打包配置、真实凭据或外部账号。
- 通过 provider resolution 重跑 selection/session API，或手工绕过公共 prepare/gate 发送 B。

### 并行任务与冲突边界

- TUI sibling `fix/tui-request-card-ttfb` / PR #136 只改 TUI formatter；本任务从同一 `origin/main` 基线派生，可并行施工。不得 cherry-pick 未合并 TUI head；最终集成时只处理任务索引等文档冲突。
- 当前只有这一个跨供应商实现 sibling；执行 session 不再创建 sibling/子代理工作树。其他活动任务若触碰共享模块，暂停并报告 main。

## 技术导航

- `src-tauri/src/infra/settings/types.rs`：旧规则兼容字段、标准 effort 类型和 sanitizer 类型。
- `src-tauri/src/gateway/configured_model_route.rs`：纯匹配、协议写入、configured/cross marker。
- `src-tauri/src/gateway/proxy/handler/middleware/model_inference.rs` + `body_reader.rs`：客户端标准来源强度 marker，插件前捕获。
- `src-tauri/src/domain/sort_modes.rs` + `infra/db/migrations/`：mode UUID、成员策略、级联与迁移。
- `src-tauri/src/domain/providers/queries.rs` + `provider_selection.rs`：实际方案候选和每请求快照。
- `src-tauri/src/gateway/proxy/handler/failover_loop/mod.rs`：baseline/work-item 调度；`prepare/provider_iterator.rs`：公共 Ready/gate/route override 接口。
- `failover_loop/context.rs`、`gateway/streams/types.rs`、`streams/finalize.rs`、`success_non_stream.rs`：`session_binding_allowed` 传递和 guard。
- `src/components/gateway/ModelRoutingPolicyFields.tsx`、`ProviderEditorDialog.tsx`、`ProvidersView.tsx`：编辑器和方案草稿。
- `infra/config_migrate/`、`domain/providers/share.rs`：全量 bundle、single-provider share、duplicate 边界。
- request log/usage/observer projection：保留完整 attempts，最终 provider/model/price，bounded marker。

详细数据流、状态机、错误语义和取舍见 `design.md`，逐步操作、测试和提交点见 `implement.md`。

## 验收入口

- AC-01~AC-02：migration/domain/IPC tests；坏数据和 UUID identity evidence。
- AC-03~AC-04：frontend editor/service tests、五协议入口/effort matcher tests。
- AC-05~AC-07：failover/session/stream/non-stream/attempt/cost tests。
- AC-08~AC-09：config/share/duplicate/contract/generated-binding tests。
- AC-10：PR 最新 head 的 `ci-gate`、`pr-title`、按 scope 选中的 frontend/rust/shared/docs jobs 和 `delivery.md`。

## 本地允许验证

项目合同 `.trellis/workflow.md`、`docs/operations/multi-worktree-delivery.md` 与 cloud-only contract 优先。本 checkout 没有仓库根 `AGENTS.md`；本任务本地只能运行：

```bash
node scripts/check-cloud-only-verification.mjs
node scripts/check-cloud-only-verification.selftest.mjs
node --check <本任务实际修改的 .mjs 文件（如有）>
python3 ./.trellis/scripts/task.py validate .trellis/tasks/08-13-cross-provider-model-routing
git diff --check origin/main...HEAD
```

禁止依赖安装、pnpm/npm/yarn 脚本、Cargo/Rust 测试/格式化/Clippy/绑定生成、lint/typecheck/build、开发服务器、Tauri、签名、打包。完整检查交给 GitHub Actions；不得在 `delivery.md` 把未运行的本地命令写成通过。

## PR、交付和暂停

- 执行 session 尽早创建 Draft PR，持续推送逻辑提交；PR 必须指向 `main`，正文链接本任务目录。
- 可以修复本任务范围内的 CI/编译失败；疑似基础设施或 main 既有问题保留日志并报告 main。
- 完成后把 PR 标记 Ready for review，填写同目录 `delivery.md` 的实际结果、关键符号、每条 AC、偏移、验证、风险、完整 head/base/ci-gate，停止写入并通知 main。
- 不得 merge PR、开自动合并、运行 `/trellis:finish-work`、archive 任务、删除 worktree/分支或写 main 验收。

## 可直接粘贴的执行 session 启动提示

```text
Active task: /Users/knaifen/Documents/Codex/aio-coding-hub/08-13-cross-provider-model-routing/.trellis/tasks/08-13-cross-provider-model-routing

你是本任务唯一执行 session。当前目录必须是
/Users/knaifen/Documents/Codex/aio-coding-hub/08-13-cross-provider-model-routing，
分支必须是 feat/cross-provider-model-routing。先完整阅读本任务的 execution.md、prd.md、design.md、implement.md，
以及 implement.jsonl/check.jsonl 引用的规范；不要创建 sibling、worktree 或子代理。

开工前只做 preflight：确认 task.json.status=in_progress、规划提交
`c6d59507c7a1de46abdb07427aa8bc153c69739c` 存在、当前 HEAD 以登记 base SHA 为 merge-base、
且已核对 TUI sibling PR #136 只修改 TUI formatter/任务材料，本任务不会触碰其文件。PR #136 未合并不构成启动门禁；任何路径、分支、base、规划提交或写者不一致仍停止并报告 main，不猜测、不自行修基线。

通过 preflight 后，严格按 implement.md 的 0 -> 8 顺序施工：每个阶段先读范围和完成信号，完成最小允许验证后提交；尽早创建指向 main 的 Draft PR，
持续推送并修复本任务范围内的 CI。跨供应商规则只能存在供应商覆盖的 named mode；目标 B 必须使用完整成员快照，最多一次跳转，复用公共 gate/重试/Ready 预算，
不重跑 selection/session API，不更新 B 的 session binding；TUI format.rs、aio/... 受管路由和所有明确禁止范围保持不变。

本地只运行 execution.md 列出的 cloud-only checker、selftest、task.py validate、变更 .mjs 的 node --check 和 git diff --check；不要安装依赖、运行 pnpm/Cargo/Rust 测试/构建/生成/格式化。
完成代码、测试、合同、迁移、PR 和绿色 CI 后，按模板重写 delivery.md，标记 PR Ready for review，停止写入并通知 main；不得 merge、archive、删除 worktree 或运行 finish-work。
```

### 必须停止并报告 main

- 当前路径、分支、base、规划提交、任务授权或唯一写者不一致；或发现 TUI sibling 实际修改了本任务产品代码范围。
- 用户锁定决定、现行合同和当前代码冲突，或需要新增产品决定/破坏兼容性。
- 必须修改禁止范围、其他活动 worktree 的共享语义、公共 API、迁移边界或真实凭据。
- `change-scope` 意外选中不应触发的长任务，或 CI 失败无法证明有任务内修法。
- 任何 AC 不能满足，或者需要把“只切供应商”改变为“必须指定模型/强度”。

## 初始交付模板

施工开始前可把本模板保留为 `delivery.md`，但不得填入虚构的 head/CI；执行 session 必须基于实际代码重写。

- 结果：尚未开始（规划材料阶段）
- PR/head/CI：尚未创建/尚未提交/未触发
- 当前唯一写者：执行 session；暂停后由 main 临时接手并落盘记录
- 阻塞：无；TUI sibling 可并行，最终集成时由 main 处理任务索引文档冲突
- 任务状态：`in_progress`，执行 session 按本入口施工并在交付后暂停
