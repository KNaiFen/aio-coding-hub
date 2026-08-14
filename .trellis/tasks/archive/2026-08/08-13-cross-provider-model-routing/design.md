# 技术设计：跨供应商模型路由

> 本文件是本任务的技术事实源。它把 PRD 的用户决定落实到数据结构、调用方向、状态机、错误语义和兼容边界；执行 session 不得用更方便但语义不同的实现替换这些约束。文件/符号名以当前代码为准，若实现时发生结构性冲突必须暂停并由 main 更新本文件。

## 1. 设计目标与不变量

### 1.1 目标

在供应商编辑器的“模型路由覆盖”中增加一个方案成员级的跨供应商规则层。请求已经完成供应商选择后，若当前基线候选 A 的规则命中，则最多插入一次临时目标 B；B 仍走公共资格 gate、凭据、桥接、重试、熔断和 Ready-provider 上限。普通供应商内规则、Default 顺序、`aio/...` 受管 Codex 路由和 TUI formatter 行为保持原语义。

### 1.2 必须始终成立的性质

1. **范围隔离**：全局 `ModelRoutingPolicy` 和供应商普通覆盖永远不包含目标供应商；跨规则只能挂在 named sort mode 的 `(mode, cli, source provider member)` 上。
2. **请求快照**：请求只使用 `ProviderSelection` 阶段捕获的 `effective_sort_mode_id`/UUID、成员资格和策略。请求开始后不重新读取 active 指针、方案名称或数据库成员，方案切换/删除不会改变在途请求。
3. **单跳**：每个请求只有一个 `cross_jump_used`。B 不再解析自己的跨/普通规则，也不会触发第二次 A->B->C 跳转。
4. **基线可恢复**：原始候选向量保持不可变；跨规则失败后恢复客户端原始模型，从未修改的 A -> C… 基线继续。临时项的失败不能把 B 永久插入排序、绑定会话或污染 `session_bound_provider_id`。
5. **公共 gate 唯一入口**：B 只能作为普通 `prepare_provider`/`run_retry_loop` work item 进入；不手工发送、不绕过 `provider_limits`、bridge、auth、circuit 或 provider-enabled gate。
6. **审计分层**：`cross_provider_model_route` 记录“为什么计划 B、B 的结果”；现有 `configured_model_route` 记录最终实际发送供应商的 wire 改写。详情保留完整 attempts，列表/卡片只投影最终结果，不把计划项当作成功项。
7. **fail-open**：无法解析、未知版本、坏 JSON、无效目标或缺少方案成员时，不阻断启动/转发；跳过跨规则并按普通基线请求。只有明确的用户输入校验在保存时返回错误。

## 2. 数据模型与持久化

### 2.1 规则类型

在 `src-tauri/src/infra/settings/types.rs` 保留旧字段并只做向后兼容的加法：

```rust
pub struct ModelRoutingRule {
    pub source_model: String,
    pub source_reasoning_effort: Option<String>, // 新增；None = 任意显式/缺失强度
    pub target_model: Option<String>,
    pub reasoning_effort: Option<String>,        // 旧字段，语义仍是目标强度
}

pub struct CrossProviderModelRoutingRule {
    pub source_model: String,
    pub source_reasoning_effort: Option<String>,
    pub target_provider_uuid: String,
    pub target_model: Option<String>,
    pub target_reasoning_effort: Option<String>,
}

pub struct CrossProviderModelRoutingPolicy {
    pub enabled: bool,
    pub rules: Vec<CrossProviderModelRoutingRule>,
}
```

- `ModelRoutingPolicy` 继续承载全局策略和供应商整份普通覆盖；旧 JSON 缺少新字段时 serde 默认 `None`。
- 目标强度使用新的 `target_reasoning_effort` 只在跨规则类型中出现；普通规则继续使用已有 `reasoning_effort`，避免把旧 bundle 的字段重命名造成破坏性迁移。
- 所有强度字段使用同一标准集合 `none|minimal|low|medium|high|xhigh|max|ultra`。空值表示没有该条件/不改写；非空值 trim、ASCII 小写后严格校验。保存边界拒绝非法新值；迁移/运行时解码删除或忽略整条无效规则，不把数字预算转换成 effort。兼容读取仅覆盖旧 model-only 或标准 effort 普通规则；旧目标为数字预算/非法值时整条删除并保留有界 invalid projection，不能为了 AC-01 保留非法目标。
- `source_model` 保持 trim 后的大小写敏感精确匹配；模型 ID 和 UUID 均受现有字节/控制字符/长度限制。
- 全局 IPC DTO 使用普通 `ModelRoutingPolicy`，序列化/校验明确拒绝任何 `target_provider_uuid` 或跨规则字段。供应商编辑使用新的组合 DTO，不把跨规则字段塞进全局 DTO。

### 2.2 方案 UUID 与成员策略

`sort_modes.id` 继续作为 SQLite 内部外键；稳定的方案身份放在独立表
`sort_mode_identities(mode_id INTEGER PRIMARY KEY REFERENCES sort_modes(id) ON DELETE CASCADE, mode_uuid TEXT NOT NULL UNIQUE)`。
这样不需要对已有
`sort_modes` 直接添加动态生成的 `NOT NULL UNIQUE` 列，也不会让旧的按 `id` 写入路径在升级瞬间失效。
新建命名方案必须在同一个 owner-scoped transaction 中先/后写入 `sort_modes` 和 identity 行；改名只更新
`sort_modes.name`，不更新 UUID；删除方案由 FK 级联清理 identity 和成员策略。所有长期配置、bundle、日志和前端缓存使用
`mode_uuid`，本机查询仍可使用 `mode_id`，两者不得混为同一身份。

在 `sort_mode_providers` 增加 nullable `cross_provider_model_routing_policy_json TEXT`。该 JSON 是
`CrossProviderModelRoutingPolicy`，源成员身份由 `(mode_id, cli_key, provider_id)` 再 join
`providers.provider_uuid` 得出；目标只保存 `target_provider_uuid`，不保存易变的整数 ID。Default 使用
`default_route_providers`，没有 identity 行，也没有跨规则列/写入口。

迁移要求（必须按顺序实现并测试）：

1. `v52 -> v53` 在一个 SQLite transaction 中确认旧表存在，然后创建
   `sort_mode_identities`（含 `PRIMARY KEY`、`NOT NULL`、唯一索引和 `ON DELETE CASCADE`），按
   `sort_modes.id ASC` 为每个既有方案生成 UUID；若重跑发现 identity 已存在，必须校验 canonical UUID、全局唯一和对应 mode，不能重新生成或静默覆盖。
2. 在同一事务中以 nullable 方式增加 `cross_provider_model_routing_policy_json`（SQLite 对已有表不能直接添加动态值的
   `NOT NULL` 列）；已有普通策略只做 sanitizer 清洗，坏 JSON/未知字段/非法目标强度变成空或失效投影，不阻断启动。
3. 迁移提交前重建/校验所有需要的索引、FK 和 `PRAGMA foreign_keys=ON` 语义；任何校验失败回滚整个事务且不推进 `user_version`。不要用 `ensure` 代替一次性数据迁移。`sort_mode_identities` 的 FK 必须在 fresh baseline 和升级库都启用；删除 mode 时 identity/cross policy 一并级联。
4. 更新 `baseline_v25`，使新安装从 fresh schema 直接包含 identity 表、成员策略 nullable 列和相应索引；更新 migration dispatch、`LATEST/MAX`、schema 常量和测试夹具。
5. 迁移必须幂等：重复执行不新增 UUID、不覆盖既有 JSON、不重复插入 identity；插入/改名/删除/复制/导入路径均显式维护 identity。若未来要把列收紧为 `NOT NULL`，只能通过完整表重建并另立版本，不得在本版本直接 `ALTER ... ADD ... NOT NULL`。

### 2.3 组合读写 DTO 与 IPC

新增窄、成组的 provider routing DTO（命名可按现有命令风格采用 `provider_model_routing_policy_*`）：

```rust
pub struct ProviderModelRoutingPolicyView {
    pub provider_id: i64,
    pub provider_uuid: String,
    pub cli_key: String,
    pub provider_override_enabled: bool,
    pub ordinary_policy: ModelRoutingPolicy,
    pub selected_mode: Option<SortModeRoutingContext>,
    pub cross_policy: Option<CrossProviderModelRoutingPolicy>,
    pub source_member_enabled: bool,
    pub source_member_present: bool,
}

pub struct SortModeRoutingContext {
    pub mode_id: i64,       // 本机 DB 操作使用
    pub mode_uuid: String,  // 持久引用/备份/UI key
    pub name: String,
}
```

读命令一次性返回普通策略和指定 named mode 的跨策略；写命令一次性校验并在一个事务中保存两者，要求调用者同时传 `provider_id + provider_uuid` 与 `mode_id + mode_uuid`，任何身份不匹配返回 typed `*_IDENTITY_CHANGED`，不按名称或数字 ID 猜测。跨策略写入必须满足：mode 非 Default、source provider 是该 CLI 的启用成员、目标 UUID 是同 CLI/同 mode 的启用成员（未知引用仅在导入/历史修复路径保留为 invalid projection，不可由普通编辑器新建）。

候选查询新增窄投影 `routing_provider_candidates_list`，只返回：

```text
provider_id, provider_uuid, cli_key, name, enabled,
source_provider_id, bridge_type, model_catalog_supported
```

不返回 URL、API key/OAuth、extension、策略、邮箱或错误 JSON。后端负责 CLI/bridge/启用资格；前端不得复制资格判断。

### 2.4 运行时捕获结构

扩展 `ProviderForGateway`（`domain/providers/types.rs`）和其 SQL mapper：

- `provider_uuid`；
- `mode_uuid: Option<String>`（Default 为 `None`）；
- `cross_provider_model_routing_policy: Option<CrossProviderModelRoutingPolicy>`（仅 named member 读出）；
- 必要时保留 `cross_policy_invalid_reasons` 的有界投影，供日志/编辑器显示而不进入执行器。

`ProviderSelection`/`RequestContext` 继续以 `effective_sort_mode_id: Option<i64>` 表示实际选择方案；同时必须在 named mode 请求中捕获并传递 `effective_sort_mode_uuid: Option<String>` 作为稳定审计/规则键（Default 为 `None`）。这两个值都来自 selection 快照，不得在 `provider_iterator` 里重新 query `sort_mode_active`。

## 3. 规则解析与强度提取

### 3.1 客户端来源强度 marker

在 `BodyReaderMiddleware` 第一次成功 JSON 解码、`requestAfterBodyRead` 插件之前，新增统一的 bounded helper，写入 `special_settings` 的请求级 marker `request_reasoning_effort`。marker 只含 `cli`, `pointer`, `effort`（标准化值或 null）、`raw_effort`（限长）和 `source=request`；绝不含 body、凭据或 URL。

协议分支固定如下，不能跨分支 fallback；验收必须把 Grok Chat 和 Grok Responses 作为两个独立入口（总计五个入口）：

| CLI/入口 | 唯一标准来源 pointer | 忽略 |
|---|---|---|
| Codex Responses/compact | `/reasoning/effort`；若字段不存在再看 `/reasoning_effort`、`/reasoningEffort` | 非 string、数字、非法值 |
| Claude Messages | `/output_config/effort` | `/thinking/budget_tokens` |
| Gemini generate/streamGenerate | `/generationConfig/thinkingConfig/thinkingLevel` | `thinkingBudget` |
| Grok Chat Completions | `/reasoning_effort` | Responses pointer |
| Grok Responses | `/reasoning/effort` | Chat pointer |

标准化只接受八项集合；数字预算即使合理也产生“来源强度缺失”，因此声明 source effort 的规则不命中。Codex 多 alias 的既有“首个存在且为 string 的字段占位，非法即缺失且不继续”行为要保留并补测试；其他协议没有臆造别名。

### 3.2 解析优先级

把现有 `configured_model_route::resolve` 拆成纯匹配器和 provider-scoped 结果构造器，签名增加 `source_effort`、`ordinary_policy`、`cross_policy`、`effective_mode_id`/UUID 和 `allow_cross`。匹配按以下固定顺序独立于列表顺序：

1. 跨供应商：`source_model` 精确 + `source_reasoning_effort` 精确；
2. 跨供应商：`source_model` 精确 + 来源强度为空；
3. 普通：`source_model` 精确 + `source_reasoning_effort` 精确；
4. 普通：`source_model` 精确 + 来源强度为空。

同一策略类别内 `(source_model, source_reasoning_effort)` 必须唯一；重复保存返回 `SEC_INVALID_INPUT`。来源模型大小写敏感，强度已 canonical lowercase。旧 model-only 普通规则仍命中任何来源强度。

匹配结果分为：

- `OrdinaryRoute { execution_provider=A, target_model, target_effort }`：现有单次改写；
- `CrossRoutePlan { source_provider=A, target_provider_uuid=B, target_model, target_effort, mode_uuid, rule_fingerprint }`：只作为调度计划，尚未改变当前发送 provider。

`aio/...`、非 POST/非 inference、model list/search/token-count/probe/discovery 在纯匹配器入口直接返回 None。

### 3.3 出站强度写入

`apply_reasoning_effort` 的 Gemini 分支统一写 `generationConfig.thinkingConfig.thinkingLevel` 并删除 `thinkingBudget`；任何数字文本不再被解释为预算。Claude、Grok Chat、Grok/Codex Responses 继续写各自标准字段。跨路由和普通路由共用该 helper，确保流式/非流式一致。

## 4. Failover 调度状态机

### 4.1 基线与 work item

在 `failover_loop::run`（`src-tauri/src/gateway/proxy/handler/failover_loop/mod.rs`）保留 selection 产生的 `baseline_providers: Vec<ProviderForGateway>` 快照，不原地重排。引入请求私有：

```rust
enum ProviderWorkItem {
    Baseline { provider_index: usize },
    CrossTemporary {
        source_provider_id: i64,
        source_provider_uuid: String,
        // A is the source candidate; target is a complete snapshot captured
        // from the named mode's eligible members before baseline ordering and
        // Ready/limit truncation. It intentionally has no baseline index.
        target: ProviderForGateway,
        route: ConfiguredModelRoute,
        audit_id: String,
    },
}

`ProviderSelection` 必须同时捕获两组不可变数据：
`baseline_providers`（完成既有排序、session binding 和必要的基线过滤后的顺序）以及
`mode_member_snapshots`（同一 named mode、同一 CLI 的全部启用成员快照，尚未按当前排序位置或 Ready 上限截断）。
目标 B 从后者按 `target_provider_uuid` 查找，因此 B 即使不在当前 baseline 顺序、被当前排序位置排在后面，仍可作为临时 work item；不得重新执行 provider-selection/session API，也不得重新 query active mode。
临时 B 仍必须在公共 `prepare_provider` 中接受同样的 enabled/CLI/bridge/credential/circuit/limit gate，且 Ready 上限照常计数。
```

以及 `cross_jump_used: bool`、`processed_provider_ids: HashSet<i64>`。`processed_provider_ids` 不替代现有 `failed_provider_ids`：后者仍只表示失败决策，前者防止 B 在后续基线中重复发送。

调度循环逐项处理：

1. 读取当前 Baseline A 的已捕获跨策略，先校验 `cross_jump_used=false`、mode/CLI/source member/target member/UUID/enabled/bridge 资格，且 B != A；无效则只处理 A。
2. 有效时把 B 的 `CrossTemporary` 放在 A 前，立即将 `cross_jump_used=true`、记录 B 为 processed；再执行 B。
3. B 使用同一 `prepare_provider` 和 `run_retry_loop`，但传入 `RouteExecutionOverride`（目标模型/强度来自 A 规则）和 `session_binding_allowed=false`；不调用 provider selection/session API，不解析 B 自己的规则。
4. B 的 gate/凭据/桥接失败沿现有 `PreparationOutcome`/`FailoverDecision` 处理。若可继续，丢弃临时项，恢复 `active_requested_model` 为客户端原值，再进入 A baseline；若终止，沿原终止响应返回。
5. A/C baseline 每次按各自普通 provider policy 重新解析；`cross_jump_used` 已为 true，所以不会再插入第二个临时项。若 B 同时出现在 baseline，按稳定 `provider_uuid` 的 processed set 使其不重复发送。

`providers_tried` 只在公共 gate 后 Ready 时递增；B 的 Ready 消耗同一 `max_providers_to_try`。B 的跳过/失败记录必须逐项复用现有 failover 合同：已知 limit/OAuth/spend 预过滤不产生 attempt 行、route hop 或 Ready 消耗；circuit/cooldown/runtime-disabled 等 common-gate skip 保留现有 skipped attempt 形态但不计 Ready；credential/auth/bridge 等结果使用现有 `PreparationOutcome` 的原有 attempt/route 规则，不新增跨专用计数。`cross_provider_model_route` marker 可记录所有 skip 原因，但不能凭 marker 伪造 attempt、route 或 switch count。临时 B 也必须遵守 provider-level retry budget，禁止手工 `send`。

### 4.2 模型、marker 与恢复

- 进入 B 前保存 `original_requested_model`；每次 loop 依据 `active_requested_model` 同步 Codex prepared body/path，B 失败后先恢复再准备 A。
- B 的 route execution object 的 `provider_id/name` 是 B，`policy_source="provider_cross"`，source model 来自客户端，target 来自 A 的跨规则；这样最终 configured marker、价格和 ledger 使用 B 的实际 provider/model。
- `cross_provider_model_route` marker 在计划、成功、失败、跳过四种状态中更新，字段至少有 source/target provider UUID+名称、source/target model、source/target effort、mode UUID、status、single_hop=true；值限长、无 body/URL/凭据。
- B 失败后不得把 B 的 configured marker 留作最终成功路由。恢复 A 前清理/重写 configured marker；跨 marker 保留 `failed` 或 `skipped` 终态。A/C 成功后其普通 marker 成为最终计费依据。
- B 成功后按 B 的最终模型/CLI 计费和请求终态；B 失败而 A 成功时按 A 的最终模型/CLI 计费。未知价格保持 unknown，不回退到源模型价格。

### 4.3 会话绑定与流式一致性

在 `ProviderCtxOwned`、`StreamFinalizeCtx` 和非流式成功上下文增加 `session_binding_allowed`。普通 baseline 为 true；CrossTemporary 固定 false。所有 `bind_success`（`success_non_stream.rs`、`streams/finalize.rs`）必须同时检查该标志、`enable_session_reuse`、非 managed route、session id 和 route generation。临时 B 不调用 bind/clear/selection API；B 失败后 A/C 仍按原绑定行为。

非流式与 SSE 必须共享同一个 flag 和 marker 生命周期。成功、fake-200、客户端 abort、终端错误的 circuit/usage/attempt 逻辑不变；只改变绑定 guard 和 route audit。新增测试必须证明 B 成功不会绑定、B 失败后 A 成功会按旧规则绑定。

## 5. UI、目录和草稿

### 5.1 编辑器组合

`ModelRoutingPolicyFields.tsx` 保留普通规则编辑，但改为严格 `<select>`/现有 Select 组件选择来源和目标强度，并新增 `source_reasoning_effort`。供应商页使用一个组合编辑器：

- 顶部显示当前 provider 与“普通规则 / 方案跨供应商规则”两个明确区块；普通区块继续覆盖开关语义。
- 方案选择只允许 named mode；Default 下跨区块显示禁用提示，不写草稿。
- 当前 provider 不在选中 mode 的启用成员中时，普通区块可编辑，跨区块禁用并提示“先加入方案”。
- 目标 provider 下拉只使用 `routing_provider_candidates_list` 的同 CLI、同 mode、enabled 成员；默认 sentinel 是“本供应商”。选其他 provider 即跨规则并绑定当前 mode UUID。
- 目标模型可从 `model_catalog_supported` provider 的现有目录给建议，但输入框允许任意合法模型 ID；目标模型/目标强度都可留空（只切供应商）。
- 无效/已删除 target UUID 显示失效行和原因；允许保存其他规则，但不能把它自动改成本 provider。
- source/target effort 都是空值或八项标准集合；不接受自由文本或数字预算。

### 5.2 方案切换与查询缓存

扩展 `ProvidersView`/`useProvidersViewDataModel` 的 mode state，使读取 key 同时包含 `provider_id + provider_uuid + mode_id + mode_uuid`。切换 mode 前检查跨规则 dirty draft：先保存组合 DTO 或明确放弃；取消切换不改变当前草稿。provider upsert/enable/disable/duplicate/delete、sort-mode member/order/active 变化和 config/share import 后失效候选列表与对应 policy query；目录 key 继续使用 provider UUID，避免 numeric ID 重用污染缓存。

## 6. 导入、分享、复制和生命周期

### 6.1 完整配置 bundle

完整 bundle 从 schema v4 升级为 v5；这会强制修改现行
`.trellis/spec/aio-coding-hub/cross-layer/config-migration-skill-bundle-contract.md` 的 schema/能力阈值章节，不能只在代码里默默扩展：

- SortMode export 增加 canonical `mode_uuid`，成员增加 `provider_uuid`、`enabled`、顺序、reuse priority 和可选 cross policy；目标 UUID 原样保存。
- 导入先做有界 JSON/UUID/重复/引用预检，再在已有 import lock 内清空并重建 provider、sort mode、members。provider UUID 保留，mode UUID 保留；内部 numeric IDs 重新映射。
- 旧 v1-v4 没有 mode UUID/cross policy：为新建 mode 生成 UUID，cross policy 为空；普通策略按旧字段迁移。
- target UUID 语法合法但不在 bundle 的规则作为 invalid projection 保留并在 UI 标示；不静默改成本 provider。无法解析的未知 JSON/旧非标准目标强度整条失效，启动/导入主体仍 fail-open。
- 导出不得包含凭据以外的新敏感字段；所有引用均为 bounded UUID/字符串。导入失败必须在破坏性操作前返回并保留旧状态。

### 6.2 单供应商分享与复制

- Provider share 仍输出现有 v2 envelope；导出前将组合策略投影为普通 `model_routing_policy_override`，剔除所有 mode/cross 字段。若实现选择新增可选字段，必须保持旧 v2 reader 不被未知字段击穿，优先使用现有剔除语义而非无必要升版。
- 分享导入生成新 provider UUID、默认 disabled、不加入 Default/named mode，且不携带跨规则；普通策略的 v1/v2 兼容继续由 `share.rs` 所有。
- Provider duplicate 只复制 providers 行的普通策略；不复制任何 `sort_mode_providers` 成员/跨策略。副本不自动加入调用方案。
- 关闭 provider override 时保留 cross JSON，运行时传入的 effective policy 为 None；重新启用恢复。

## 7. 日志与投影

请求级 marker 通过既有 `special_settings` 通道进入 request log、observer snapshot 和桌面详情；所有消费者必须 bounded/fail-open：

- 详情解析完整 `attempts_json`，包含 B 的中间失败、gate skip 和之后的 A/C；不删除或重写历史 attempt。
- Home/Observer/TUI 卡片使用最终成功 provider/model；跨成功时可显示紧凑 `A/source -> B/target`，跨失败不显示 B 为终态。
- `provider_switch_count` 继续由相邻非 skipped 实际 provider hop 推导；不能用“是否存在 cross marker”伪造切换。TTFB 与 `切/重` 文案归 PR #136，不改 `src-tauri/crates/aio-tui/src/format.rs`。
- final provider/model/CLI 和 configured route marker 决定 usage ledger、价格、provider spend-limit；cross marker 只是解释性审计，不直接计费。
- marker 解析遇到未知版本、超长、provider mismatch 或 malformed 时只隐藏路由文案，不能让列表/请求失败。

## 8. 兼容、回滚和安全

- 旧客户端只发送三字段普通规则时，后端默认 source effort/target provider 为空，行为与当前一致。
- 旧数据库升级失败不得删除旧数据；migration transaction 回滚。新字段坏 JSON 读作 disabled/empty，保存时由 sanitizer 清理。
- 方案切换、provider 删除和 config import 必须先完成 durable write，再推进 route generation/清理 session；请求捕获的旧快照可完成但不能写回新绑定。
- 候选 DTO 不携带 endpoint、key、OAuth token/email、extension 或策略详情；日志 marker 不携带 body、URL、header 或凭据。目标 provider 的实际认证仍由 B 的既有 credential resolver 负责。
- 回滚功能 PR 时回退代码和 v53/v5 migration 的单个提交即可；已经升级的数据库保留新增列/UUID，旧代码应按默认/忽略未知字段运行，不依赖删除列回滚。执行 session 必须在 delivery 记录此兼容事实。

## 9. 取舍记录

### 方案成员列而非独立规则表

选择 `sort_mode_providers.cross_provider_model_routing_policy_json` 是因为成员行已经是 mode+CLI+provider 的生命周期边界，删除方案/删除 provider 会自然级联，查询时可与候选一起捕获；方案稳定身份则由独立 `sort_mode_identities` 表提供。若当前 migration 约束禁止 JSON 列，允许等价子表，但键和生命周期必须完全相同。

### `mode_uuid` 与 numeric `mode_id` 并存

numeric ID 继续服务本机 FK/查询，UUID 解决改名、导入重建和跨实例引用不稳定的问题。任何长期配置、日志、bundle 或前端缓存不得依赖 numeric ID alone。

### Failover 外层插入而非 provider resolution

跨跳转必须发生在已捕获候选之后、公共 gate 之前；在 provider resolution 重跑会污染 session binding/方案排序，在 provider preparation 之后插入会先占用 A 的 Ready 名额。外层 work item 可以精确恢复原队列和模型。

### 组合 DTO 而非把 mode ID 塞入全局策略

global/provider ordinary policy 的继承/替代语义不能被 mode 切换改变；组合 DTO 让 UI 一次读写两个 ownership 域，并明确 Default 没有跨规则。
