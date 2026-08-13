# 跨供应商模型路由

## Plan Status

- Implementation authorization: 已确认。
- Confirmation date and summary: 2026-08-13；用户要求采用一个 sibling worktree，由一个执行 session 端到端实现、提交、创建 PR、等待 CI 并暂停；用户确认本文件的全部行为、兼容边界和验收标准。
- Confirmed coverage: 供应商覆盖中的普通/跨供应商模型路由、命名调用方案绑定、配置迁移/导入/分享/复制、运行时一次性临时跳转、请求审计和日志投影；不包含全局跨供应商规则、`aio/...` 受管别名和 TUI 任务本身。
- Planning revision: 3；本次修订明确两个 sibling 可并行施工，规划提交后由 main 回填完整 SHA，执行 session 不得以占位值开工。
- Execution route: delegated sibling worktree `/Users/knaifen/Documents/Codex/aio-coding-hub/08-13-cross-provider-model-routing`。
- Base: `origin/main` @ `875ff441c5ba9f1a7f235ad95dadb945a41bba61`；TUI PR #136 与本任务从同一基线并行施工，不是本任务的实现启动门禁。最终集成时仅处理任务索引等文档冲突。
- Migrated from direct-main record: 无。

## Material Facts, Assumptions, and Open Questions

| Item | Source | Status / closure condition |
|---|---|---|
| 现有普通模型路由是全局策略或供应商整份覆盖，当前只改已选供应商的模型/强度，不选择供应商。 | 现行代码与 configured-model-routing contract | confirmed；普通规则兼容语义必须保持 |
| `effective_sort_mode_id` 是请求实际采用的命名方案；`None` 表示 Default，不能在请求中途重新读取 active 指针。 | provider selection 代码与方案域调查 | confirmed |
| TUI sibling PR #136（`fix/tui-request-card-ttfb`，head `2b12c68cd99f7bc7c21fb8fa2b5354c9992a229b`）只修改 TUI formatter 与其任务材料，当前尚未合并。 | GitHub/local refs，2026-08-13 | confirmed；与本任务可并行施工，不从其 head 派生、不 cherry-pick；本任务不得修改 TUI `format.rs`。最终合并/索引同步时处理可能的 README 文档冲突 |
| PENDING 当前无 `pending`/`planned` 条目。 | `PENDING.md` | confirmed |
| 旧规则中的非标准目标强度（含 Gemini 数值预算文本）应如何处理。 | 用户决定 | closed：迁移时删除整条无效规则 |
| 供应商覆盖关闭、方案成员缺失、方案切换草稿、复制与分享边界。 | 用户决定 | closed，详见“锁定决策” |
| SQLite 既有方案表不能直接添加动态值 `NOT NULL UNIQUE` UUID。 | 迁移审查与 SQLite 约束 | closed：使用 `sort_mode_identities(mode_id PRIMARY KEY, mode_uuid UNIQUE)` 独立表；v52->v53 事务内创建、回填、校验，详见 `design.md` |
| 现行 configured-routing/failover/bundle/share/observer/settings 合同与新增跨规则语义的差异。 | 合同审查 | closed：本任务将按下方合同变更清单同步现行 spec，并逐项纳入 AC-09/验收清单；若代码证明需要超出清单的公共行为变化，必须暂停回报 main |

不得从聊天记录或实现者直觉补充未写入本文件的产品行为。材料性冲突须停止并报告 main。

## Goal

在现有供应商覆盖模型路由编辑器中增加“目标供应商”能力：目标默认为本供应商；选择其他供应商时，规则仅绑定当前编辑器所选的命名调用方案，并在请求中最多一次把当前候选 A 临时切到方案内启用的目标 B。普通供应商内路由继续按原逻辑工作，并新增来源强度匹配。

## Locked Product Decisions

### 配置范围和规则形态

1. 全局模型路由只允许本供应商内改写模型/目标强度；全局设置不得保存或编辑跨供应商目标字段。
2. 供应商覆盖中并行存在两类规则：供应商级普通规则（本供应商内改模型/强度，不绑定调用方案）与方案级跨供应商规则（仅在当前命名调用方案的该供应商成员上保存）。
3. 编辑继续复用当前供应商模型路由编辑器。每条规则的目标供应商默认“本供应商”；选本供应商即普通规则，选其他供应商即跨规则，并隐式绑定当前选中的命名方案。
4. `Default` 没有稳定方案 ID，不支持跨供应商目标；选中 Default 时只能保存普通规则。
5. 当前编辑供应商不属于所选命名方案时，普通规则仍可编辑；跨规则区域禁用并提示先加入该方案。切换方案前必须保存或放弃未保存跨规则草稿。
6. 目标供应商候选仅来自当前 CLI、当前命名方案内 `provider.enabled=true` 且方案成员 `enabled=true` 的供应商；目标 B 可不在当前排序位置，但必须在该方案成员中。无效/已删除目标保留为失效项并允许保存其他改动。
7. 目标模型对已有本地目录的供应商提供建议，但允许自由输入目录之外的模型 ID。目标模型和目标强度均可留空，允许“只切供应商”。
8. `aio/...` 受管 Codex 模型别名保持现有固定绑定，不参与本功能。

### 强度和匹配

1. 普通和跨规则均有可选 `source_reasoning_effort` 与可选目标 `reasoning_effort`。
2. 来源强度只读取客户端明确携带的标准字符串并 trim/ASCII 小写：Codex Responses 读取 `reasoning.effort`（兼容 `reasoning_effort`、`reasoningEffort`）；Claude Messages 读取 `output_config.effort`；Gemini generate/streamGenerate 读取 `generationConfig.thinkingConfig.thinkingLevel`；Grok Chat 读取 `reasoning_effort`，Grok Responses 读取 `reasoning.effort`。
3. 合法集合固定为 `none|minimal|low|medium|high|xhigh|max|ultra`。Claude `thinking.budget_tokens` 和 Gemini `thinkingBudget` 不参与来源匹配；数值或非法字符串不构成有效来源强度。
4. 来源强度留空表示该来源模型全部强度；来源模型继续精确、区分大小写匹配。编辑器对来源/目标强度使用严格下拉，只能选择合法集合或空值。
5. 同一类别内同一“来源模型 + 来源强度”组合唯一，列表顺序不能改变匹配结果。
6. 匹配优先级固定为：跨精确强度 -> 跨通配强度 -> 普通精确强度 -> 普通通配强度。命中跨规则后直接使用 A 规则填写的目标模型/强度，B 不再匹配任何自身规则；每请求最多一次跨供应商跳转。

### 运行时和会话

1. 对已完成方案选择的基线候选 A，命中有效跨规则时，在 A 前插入一次性临时 B；不要重跑 provider-selection/session API，也不要原地重排基线队列。
2. B 必须复用现有准备、gate、凭据、桥接、重试、熔断和 Ready-provider 上限；B 占用一个供应商尝试名额且不重复尝试。
3. B 成功则请求终态使用 B 的实际供应商/模型；B 不创建、更新或清除会话绑定。
4. B 失败后沿用现有 failover 的终止/继续判定。若判定可继续，恢复原始客户端模型并继续原始 A -> C… 基线；A/C 的普通供应商内规则照常生效。若现有判定终止，则按现有终止行为返回，不额外强行回退。
5. 临时 B 不改变会话绑定；B 失败后的 A/C 恢复既有正常会话行为（包括原有成功绑定逻辑）。
6. 目标无效、禁用、删除、移出实际方案、协议不兼容或不满足现有资格 gate 时，跨规则整条跳过，按原始基线处理；配置保留失效提示。

### 持久化、迁移、复制和分享

1. 为命名调用方案增加不可变 UUID；跨规则按方案 UUID + CLI + 供应商成员身份绑定，改名不影响。旧数据库通过独立 `sort_mode_identities` 表映射 numeric mode id，不直接改造已有 `sort_modes` 的非空约束。
2. 在 `sort_mode_providers` 增加 nullable 方案成员策略 JSON（或等价子表）；物理本机键仍为 `(mode_id, cli_key, provider_id)`，通过 identity/provider UUID 映射为长期外部键 `(mode_uuid, cli_key, provider_uuid)`；普通规则继续位于 `providers.model_routing_policy_json`。
3. 组合读写 DTO 必须同时返回/保存当前供应商的普通策略和当前命名方案成员的跨策略；全局策略 DTO 严格禁止 `target_provider_uuid`。
4. 完整配置备份升级 schema，保存方案 UUID、成员跨规则和目标 provider UUID；旧 bundle 导入生成新方案 UUID 且无跨规则。目标引用不存在时保留为失效项，不静默改成本供应商。
5. 单供应商分享剔除方案级跨规则，只分享普通供应商内覆盖；分享导入仍遵守现有 disabled-only/不加入调用顺序合同。
6. 供应商复制只复制供应商级普通规则，不复制方案级跨规则；副本不自动加入任何方案。
7. 供应商覆盖开关关闭时保留方案级跨规则，但运行时不生效；重新开启后恢复。
8. 旧的非标准目标强度规则（特别是 Gemini 数值预算）在迁移/写入清洗时整条删除，不转换为 `thinkingLevel`。

### 日志、费用和 UI 投影

1. 新增请求级 `cross_provider_model_route` 审计 marker，至少包含来源候选 A、目标 B、来源/目标模型、来源/目标强度、方案 UUID、命中/跳过/失败状态；不得包含请求体、凭据或 URL。
2. 继续保留最终实际供应商范围内的 configured-model-route marker，用于最终模型展示、费用和定价；B 成功时费用按 B 的 effective model/provider 计算。
3. 卡片按最终成功结果显示供应商和模型；跨规则成功可显示 `A / source -> B / target` 的紧凑路径。B 失败后卡片不把失败 B 当作最终结果；详情/审计保留完整 attempts 链。
4. 不伪造 `provider_switch_count`：计数仍只来自实际发送 hop 的相邻供应商变化；临时 B 在 A 尚未发送前成功时通常为 0。原有 TUI PR #136 的 TTFB 与 `切/重` 文案由其自身任务负责，本任务不得覆盖或回退。

## Acceptance Criteria

- [ ] **AC-01 数据模型与向后兼容**：旧三字段普通 model-only 规则以及使用八项标准目标强度的旧规则仍可读取，目标默认为本供应商；包含数字预算文本或其他非法目标强度的旧规则在迁移/写入清洗时整条删除并记录 bounded invalid projection，不得为满足兼容而保留非法目标；新增字段可通过 Rust/TS IPC；全局策略拒绝跨供应商字段；损坏/未知数据 fail-open，不阻断启动或请求。
- [ ] **AC-02 方案持久化**：命名方案拥有稳定 UUID；方案成员跨规则可增删改查；改名保持绑定，删除级联清理；Default 不产生跨规则；供应商覆盖关闭保留但不生效。
- [ ] **AC-03 编辑器与候选**：当前编辑器显示本供应商默认目标、两类规则、来源/目标强度下拉、同 CLI 方案启用成员候选、目录建议/手输、失效项；非成员/Default/草稿切换行为符合锁定决策；普通规则仍可编辑。
- [ ] **AC-04 强度提取与匹配**：五个协议入口逐项覆盖 Codex Responses、Claude Messages、Gemini generate/streamGenerate、Grok Chat Completions、Grok Responses；预算字段不命中；精确/通配和跨优先级确定性匹配；大小写敏感；无强度的旧规则继续命中。
- [ ] **AC-05 运行时跳转**：命中 A->B 时 B 临时优先一次、占 Ready/尝试上限、使用 A 目标模型/强度且不链式；B 成功终态正确；B 失败按现有判定恢复原始 A->C；B 不重复，A/C 普通规则继续。
- [ ] **AC-06 资格与安全边界**：B 仅可为实际方案启用成员且满足现有 enabled/CLI/bridge/gate/circuit/limit/auth 资格；无效目标跳过；`aio/...`、非推理、模型列表、计数 token、探测/发现请求不受影响；不泄露凭据/请求体。
- [ ] **AC-07 会话与审计**：B 不创建/更新/清除绑定；A/C 恢复原会话行为；attempts、provider chain、跨规则 marker、最终 provider/model/费用保持一致且可审计；流式和非流式路径一致。
- [ ] **AC-08 导入/分享/复制**：完整备份可保留方案/目标 UUID 并在导入重建；失效引用可见；单供应商分享剔除跨规则；复制不复制跨规则；旧 schema 可导入。
- [ ] **AC-09 测试与合同**：新增/更新 Rust、前端、迁移、导入分享、运行时和投影测试；必须同步更新下列现行合同并在 PR 中逐项给出证据：
  - `configured-model-routing-contract.md`：新增仅 supplier-override/mode-scoped cross policy；允许 source-effort wildcard 但不允许 model wildcard；固定八项 effort；Gemini 仅 `thinkingLevel`，旧预算目标删除；B 不二次匹配；最终 provider marker/计费规则保持。
  - `gateway-failover-route-contract.md`：记录一次性临时 B 的资格、Ready 上限、processed 去重、原基线恢复，以及各 gate skip 遵循既有 attempt/route 计数。
  - `config-migration-skill-bundle-contract.md`：schema v5 字段、mode/provider UUID 保留与旧 v1-v4 导入兼容，迁移预检先于 destructive lock。
  - `provider-share-contract.md`：单供应商分享剔除 mode-scoped cross policy，导入 disabled/no-route/no-network，保持 v2 严格未知字段边界。
  - `local-observer-tui-contract.md`：cross marker additive、bounded、provider-scoped/fail-open；详情保留 attempts，卡片只投影最终 provider；本任务不改 TUI formatter。
  - `settings-ownership-rollback-contract.md`：组合 DTO 使用 owner-scoped transaction/CAS，不 whole-snapshot 覆盖并发设置，失败按 owned-field 回滚。
  合同缺失的 shipped template 不在本任务补造；以 `.trellis/spec/...` 现行文件为唯一合同事实源。
- [ ] **AC-10 交付门**：执行 session 在 sibling 中完成代码、测试和文档，创建指向 `main` 的 PR，等待最新 head 的必需 `ci-gate`、`pr-title` 和范围相关编译/检查绿色，填写 `delivery.md` 后暂停；不得合并或归档。

## Explicit Non-Goals

- 不在全局模型路由中提供跨供应商目标。
- 不改 `aio/...` 受管 Codex 别名的固定供应商语义。
- 不允许跨 CLI 目标或方案外目标；不实现链式 A->B->C。
- 不改普通规则的供应商整份覆盖/继承语义，只在供应商覆盖中增加方案级跨规则并行存储。
- 不把 Gemini 数值预算转换为标准 effort；旧数值目标规则删除。
- 不在本任务修改 TUI `format.rs`、TUI 协议或 TTFB/`切/重` 展示任务。

## Scope and Decision Changes

| Date | Decision | Affected AC | Owner / resume condition |
|---|---|---|---|
| 2026-08-13 | 从“全局与覆盖都可跨供应商”收敛为“仅供应商覆盖；跨规则绑定当前命名方案成员”。 | AC-01/02/03/05/06/08 | 用户已确认；执行 session 以本版为唯一范围 |
| 2026-08-13 | 目标强度标准化；Gemini 只写 `thinkingLevel`；旧非标准目标规则迁移时整条删除。 | AC-01/04/09 | 用户已确认 |
| 2026-08-13 | 目标候选限定为当前 CLI、当前命名方案中启用成员；Default 禁止跨规则。 | AC-02/03/06 | 用户已确认 |
| 2026-08-13 | 临时 B 不改绑定；B 失败后按现有 failover 判定恢复原基线。 | AC-05/07 | 用户已确认 |

## PENDING Review

- `PENDING.md` 已完整审阅；当前无 `pending` 或 `planned` 条目，无需纳入本任务。

## Required Evidence

- Rust 单测/集成测试覆盖 AC-01/02/04/05/06/07/08/09。
- 前端服务/组件测试覆盖 AC-03/04/08。
- `node scripts/check-cloud-only-verification.mjs`、对应 selftest、`task.py validate`、`git diff --check` 本地通过。
- PR 最新 head 的 `ci-gate`、`pr-title` 及范围选中的 frontend/rust/shared/docs jobs 绿色；未运行的本地构建/测试不得在交付文档中冒充通过。
