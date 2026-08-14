# 交付报告：跨供应商模型路由

> 本文件记录执行 session 已完成的产品实现、Round 1 基线返工、Round 2 F-002～F-005、Round 3 F-006 返工结果，以及 main 的 Round 4 验收。任何新 push 都会使旧 head 的 CI 证据不再代表最新 PR head。

## 交付状态

- 结果：Round 4 main 验收通过；`F-006` 已在固定候选 `74523fe49213f148e4979ce01bc6b1f7d8707368` 关闭，`F-001`～`F-005` 的关闭结论保持有效，没有新增 required finding。
- PR：[KNaiFen/aio-coding-hub#137](https://github.com/KNaiFen/aio-coding-hub/pull/137)（Ready for review；`MERGEABLE` / `CLEAN`）
- 分支：`feat/cross-provider-model-routing`
- 初始派生 base：`main` @ `875ff441c5ba9f1a7f235ad95dadb945a41bba61`；同步时实时 PR base：`main` @ `bd2535796fdf847008b7b55789572367d3e615e9`
- Round 1 同步验证 head：`c47f1a30fc538cd89b2dfe48416dacc05df6888e`
- 普通 merge：第一父 `1ed9c37832a4a135bbb321dd7f7e244b95245f9b`，第二父 `bd2535796fdf847008b7b55789572367d3e615e9`；包含 main 基线修复 merge `a0f6823be399b2a2a31fb27b9fdfb0b6cb60e885`。
- 规划提交：`c6d59507c7a1de46abdb07427aa8bc153c69739c`
- Round 2 产品候选 head：`30b43ebdabb865bf36d54336bd8f4f4ea1746a92`；base：`bd2535796fdf847008b7b55789572367d3e615e9`。
- `ci-gate`：通过，[run 31764918629](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31764918629) / [job 94662402408](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31764918629/job/94662402408)，绑定 Round 2 产品候选 head。
- 其他检查：change-scope、frontend、Rust、docs-contract、support-contract、pr-title、JS/Rust CodeQL 全部通过；手动 dispatch guard 和候选构建按范围跳过。
- Round 2 产品候选验证终态：2026-08-14。
- Round 3 产品候选 head：`abd79c7484a51be2bee80787092db847a1466f47`；base：`bd2535796fdf847008b7b55789572367d3e615e9`。
- Round 3 `ci-gate`：通过，[run 31769486773](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31769486773) / [job 94675950139](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31769486773/job/94675950139)，绑定 Round 3 产品候选 head。
- Round 3 其他检查：change-scope、frontend、Rust、docs-contract、support-contract、pr-title、JS/Rust CodeQL 全部通过；手动 dispatch guard 和候选构建按范围跳过。
- Round 3 产品候选验证终态：2026-08-14。
- Round 4 验收候选 head：`74523fe49213f148e4979ce01bc6b1f7d8707368`；base：`bd2535796fdf847008b7b55789572367d3e615e9`。
- Round 4 `ci-gate`：通过，[run 31770929381](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31770929381) / [job 94679845780](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31770929381/job/94679845780)；[CodeQL run 31770929314](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31770929314) 与 [`pr-title` run 31770929342](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31770929342) 同一 head 通过。
- 执行 session：已停止写入；main 于 2026-08-14 13:34:16 CST 接管验收记录。

## Round 1 返工快照

- preflight 确认登记路径、分支、`task.json.status=in_progress`、规划提交与唯一写者均符合；工作树干净，本地/远端 head 同为 `1ed9c37832a4a135bbb321dd7f7e244b95245f9b`。
- `git fetch origin` 后执行 `git merge --no-edit origin/main`，自动生成 merge commit `c47f1a30fc538cd89b2dfe48416dacc05df6888e`，没有内容冲突、rebase、force-push 或 cherry-pick。
- merge 完整保留 main 的 `nanoid 3.3.18` workspace/lockfile 修复、TUI 归档和历史记录；相对第一父未修改跨供应商产品实现、测试逻辑或 `src-tauri/crates/aio-tui/src/format.rs`。
- F-001 结果：原 frontend 依赖审计阻塞已解除；同一同步验证 head 的 audit、lint、typecheck、frontend tests/build、Rust、合同检查、CodeQL、标题和总门禁全部通过。

## Round 2 返工快照

- preflight 确认登记路径、分支、`task.json.status=in_progress`、规划提交与唯一写者均符合；工作树干净，本地、远端分支和 PR head 同为 `f95f9cca8a5738efb986c0f8389f6537c9a60657`。
- F-002：gateway selection 仅在 source provider ordinary override 存在时暴露成员 cross policy；关闭 override 保留原始成员 JSON/revision 并回到 A baseline，重新开启即恢复 A -> B。`gateway/routes.rs` 新增 named-mode 实际请求的开 -> 关 -> 开回归。
- F-003：保存前取消同 provider 的所有 routing-policy 请求，成功后把 provider 级 ordinary policy/revision 广播到所有已缓存 mode，保留各 mode 自己的 cross policy。表单只在 ordinary revision 真正变化且 draft clean 时同步，dirty draft 保持并继续用旧 revision 做 CAS。
- F-004：`ModelRoutingRule` 在反序列化时保留未知字段；global、provider ordinary 和 share v2 写入口统一以稳定 `SEC_INVALID_INPUT` 拒绝 cross-only/未知字段。历史 settings/provider JSON 仍经 sanitizer 整条丢弃坏规则并 fail-open。
- F-005：v52 -> v53 在同一 transaction 内持久化清洗 provider ordinary JSON；非法 source/target effort、cross-only/未知规则整条删除，malformed/non-text JSON 写为 disabled/empty。迁移 projection 只记录计数和最多 16 个 provider ID，更新失败会回滚 identity、member column、policy 与 `user_version`。
- 本轮提交：`9621934d`（Rust runtime/写边界/迁移）、`547ae870`（frontend cache/form revision）和 `30b43ebd`（仅应用 CI artifact 证明为本任务造成的 rustfmt 排版）；最终 Rust format/generate 无 drift。

## Round 3 返工快照

- preflight 确认登记路径、分支、`task.json.status=in_progress`、规划提交与唯一写者均符合；工作树干净，本地、远端分支和 PR head 同为 `177ebae034abd0d0bf687c0cef22a596af9a54c7`。
- F-006：cross draft 使用独立的 adopted revision，并通过统一 helper 同步 policy、baseline 和 revision。dirty 同 scope refetch 同时保留旧 draft、旧 baseline 和旧 revision；clean refetch 原子采纳服务端新三元组。
- 保存成功和放弃草稿会采纳最新三元组，scope 切换采纳新 scope 三元组，编辑器关闭会清空三者和 revision ref。保存只使用 adopted revision，因此并发 writer 会触发 `PROVIDER_ROUTING_CONCURRENT_UPDATE`，不会被旧 draft 覆盖。
- `ProviderRoutingEditor.test.tsx` 新增 dirty refetch + CAS conflict、clean refetch 和 scope 切换回归；云端 frontend unit tests 与 build 均通过。本轮产品提交：`abd79c74`。

## 实现摘要

### 用户可见结果

- 供应商覆盖编辑器将普通规则和当前命名调用方案绑定的跨供应商规则分区编辑；Default、非成员、dirty draft、失效目标、严格 effort 下拉与模型目录建议均有明确状态。
- 推理请求可按固定优先级匹配来源模型/标准 effort，并最多一次从当前候选 A 临时跳转到同 CLI、当前命名方案中的启用成员 B；B 失败后恢复原始 A -> C 基线。
- 请求日志/Home/Observer 使用最终成功供应商、模型和费用，并保留有界 cross marker 与 attempts；没有修改 TUI formatter、TTFB 或“切/重”文案。
- 关闭 provider override 时已保存的跨供应商配置仍可在编辑器中恢复，但运行时不会创建 `CrossTemporary`、cross marker 或 B attempt；再次开启后同一命名方案规则恢复。
- 跨供应商草稿遇到同 scope 后台刷新时不会借用新 revision 覆盖并发更新；冲突会保留当前草稿并显示保存失败，用户可放弃草稿后采纳服务端新值。

### 内部实现

- SQLite schema v53 新增稳定 mode UUID identity 与成员 cross policy；组合 domain/query/IPC 以 mode/provider UUID 和 owner revision 读写，坏 JSON fail-open。
- 五个协议入口只提取八项标准来源 effort；matcher 固定为“跨精确 > 跨通配 > 普通精确 > 普通通配”，Gemini 出站只写 `thinkingLevel`。
- failover 使用完整 `ProviderForGateway` 快照构造 `CrossTemporary` work item，复用既有 gate、凭据、bridge、retry、circuit 和 Ready 上限；B 禁止 session binding 且不链式匹配。
- bundle v5 保留方案/provider UUID 与成员策略；v1-v4 兼容导入，provider share 剥离 cross policy，duplicate 仅复制普通策略且不复制方案成员。
- ordinary policy 是 provider 级共享状态：跨 mode cache 只同步 ordinary 字段与 owner revision，cross policy/revision 继续按 mode 独立。普通写入边界不再静默吞掉跨供应商字段，v53 升级会把历史非法 ordinary JSON 持久化为安全表示。
- cross editor 把 draft、adopted baseline 和 adopted revision 作为同一 scope 的独立状态；dirty refetch 不更新三者，clean refetch、保存成功、放弃和 scope 切换统一采纳三者，关闭时统一清空。

## 验收标准对应

| 标准 | 结果 | 证据 |
|---|---|---|
| AC-01 数据模型与向后兼容 | 实现完成，Rust 验证通过 | `settings/types.rs` 保留未识别字段供写边界拒绝；`settings_service.rs`、`domain/sort_modes.rs`、`providers/share.rs` 覆盖 global/provider/share 拒绝与历史 fail-open；Rust job 通过 |
| AC-02 方案持久化 | 实现完成，Rust 验证通过 | `v52_to_v53.rs`、baseline v25、迁移测试覆盖 UUID 回填/不可变/级联、持久化 effort 清洗、有界 projection、幂等与更新失败回滚 |
| AC-03 编辑器与候选 | 实现及云端前端验证通过 | `ModelRoutingPolicyFields.tsx`、provider form/query；`ProviderRoutingEditor.test.tsx` 与 `sortModes.test.tsx` 覆盖 Default/非成员/失效目标/dirty draft/目录建议、跨 mode ordinary revision、延迟旧请求，以及 cross dirty/clean refetch、CAS conflict 和 scope 切换 |
| AC-04 强度提取与匹配 | Rust 与前端验证通过 | `model_inference.rs` 覆盖五入口与预算排除，`configured_model_route.rs` 覆盖精确/通配/大小写及 Gemini 写入 |
| AC-05 运行时跳转 | Rust 验证通过 | `failover_loop/mod.rs` 与 `gateway/routes.rs` 覆盖 B 非流/SSE 成功、B 失败 A 基线恢复、bridge prepare 失败恢复、processed 去重及 override 开 -> 关 -> 开的 A/B/A 路由恢复 |
| AC-06 资格与安全边界 | Rust 验证通过 | effective mode snapshot、inference-only gate、managed alias/非推理排除、bounded marker/candidate DTO 测试 |
| AC-07 会话与审计 | Rust 验证通过 | attempt 级 `session_binding_allowed=false`；request-end/stream finalize、cost/Observer/Home marker 投影测试通过 |
| AC-08 导入/分享/复制 | Rust 与前端服务验证通过 | config v5/v1-v4 导入测试、share/duplicate 测试；Rust suite 通过单连接池 duplicate 路径，frontend unit tests 通过 |
| AC-09 测试与合同 | 通过 | Round 3 产品候选的 frontend unit tests/build、Rust tests/Clippy/audit/generated drift、现行合同和 docs/support contract 全部通过 |
| AC-10 交付门 | 通过 | 产品实现、三轮返工、推送与 delivery 已完成；固定候选 `74523fe49213f148e4979ce01bc6b1f7d8707368` 的 required CI、frontend、Rust、docs/support、CodeQL 与 `pr-title` 全绿，PR Ready 且可合并 |

## 主要代码位置

| 文件或符号 | 变更 | 设计原因 |
|---|---|---|
| `infra/db/migrations/v52_to_v53.rs`、`domain/sort_modes.rs` | mode UUID、成员策略、组合 DTO/CAS | 把跨规则绑定稳定命名方案成员，而不污染普通 provider policy |
| `gateway/proxy/handler/middleware/model_inference.rs`、`gateway/configured_model_route.rs` | 标准 effort 提取、纯 matcher、Gemini 写入和 marker | 集中协议边界并保持匹配顺序确定 |
| `gateway/proxy/handler/failover_loop/mod.rs` | `CrossTemporary` 一次性 B work item | 复用原 failover 基线与公共资格/重试状态机，不创建第二套路由器 |
| `gateway/routes.rs` | effective mode/member snapshot 和端到端回归 | 请求开始时冻结完整目标快照，避免运行中重读 active 指针 |
| `domain/providers/queries.rs` | override-gated effective cross policy | 不清空成员配置，但关闭 ordinary override 时不让 runtime 获取 cross policy |
| `query/sortModes.ts`、`useProviderEditorForm.ts` | cache request 取消、ordinary 广播、ordinary/cross revision-aware draft 同步 | 避免 mode B 旧响应回滚 provider 级普通策略，并把 cross dirty draft 固定到 adopted baseline/revision |
| `settings/types.rs`、`settings/migration.rs`、`app/settings_service.rs` | ordinary rule 未知字段写拒绝与读取 sanitizer | 保持 global/provider/share 范围隔离，又不让历史坏数据阻断启动 |
| `ModelRoutingPolicyFields.tsx`、`useProviderEditorForm.ts` | 普通/跨规则 UI 与草稿状态 | 在既有供应商编辑器内实现锁定交互，不扩展全局规则 |
| `infra/config_migrate/*`、`domain/providers/share.rs`、`app/provider_service.rs` | bundle v5、share/duplicate 边界 | 保持旧 bundle 兼容且不把方案成员隐式带入单供应商操作 |
| `gateway/request_end.rs`、`app/observer/snapshot.rs`、`requestLogPresentation.ts` | attempts、marker、最终 provider/model/cost 投影 | 审计完整，同时避免把失败 B 当最终结果 |

## 与计划的偏移

- 产品行为、兼容性、范围和 AC 无偏移；严格按阶段 0 -> 8 推进。
- CI 首轮发现四条本任务 Clippy 警告，使用参数 DTO、返回类型别名和 boxed 完整 provider 快照消除；没有用 `allow` 掩盖。
- CI 次轮发现本任务抽取的 duplicate helper 在单连接池内嵌套取连接；缩短 source 读取连接作用域后，最新 Rust suite 通过。该修复不改变复制语义。
- 两轮云端 rustfmt artifact 均先核对文件范围后原样应用；最终 head 的 generated-file drift 检查通过。
- Round 1 仅用普通 merge 同步 `origin/main` 并更新交付记录；来自 main 的 workspace/lockfile、任务归档和历史记录原样保留，没有修改跨供应商产品语义。
- Round 2 首个产品 CI head `547ae870...` 只在 cloud rustfmt 检查失败；artifact 明确仅包含本轮 Rust 格式变更，原样应用为 `30b43ebd...`。最终候选的 format/generate 和 generated-file drift 均通过，没有修改 binding、依赖或 allowlist。
- Round 3 只修改 provider editor form 与对应前端测试；没有改变 F-002～F-005、锁定产品决定、依赖、TUI formatter 或审计 allowlist。

## 验证结果

### 本地检查

| 命令 | 结果 | 说明 |
|---|---|---|
| `node scripts/check-cloud-only-verification.mjs` | 通过 | 仓库云端验证合同通过 |
| `node scripts/check-cloud-only-verification.selftest.mjs` | 通过 | 全部断言通过 |
| `python3 ./.trellis/scripts/task.py validate .trellis/tasks/08-13-cross-provider-model-routing` | 通过 | 12 implement / 11 check entries |
| `git diff --check origin/main...HEAD` | 通过 | Round 3 产品候选无 whitespace error |
| `git diff --name-only origin/main...HEAD` | 通过 | 已审查 PR 文件范围；当前 PR diff 不含 `src-tauri/crates/aio-tui/src/format.rs`、依赖版本或审计 allowlist |
| `node --check <修改的 .mjs>` | 不适用 | 本任务未修改 `.mjs` 文件 |

未在本地运行 pnpm、Cargo、rustfmt、Clippy、测试、构建或绑定生成；这些命令按仓库合同仅由 GitHub Actions 执行。

### GitHub CI 与编译

| Workflow / Job | 结果 | 链接或说明 |
|---|---|---|
| `change-scope` | 通过 | [job 94672866899](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31769486773/job/94672866899)，选择 frontend、Rust 与合同检查 |
| `rust` | 通过 | [job 94672930081](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31769486773/job/94672930081)：格式/绑定无 drift、Clippy、Rust tests、cargo audit 通过 |
| `frontend` | 通过 | [job 94672930047](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31769486773/job/94672930047)：dependency audit、lint、typecheck、unit tests、build 与 plugin SDK contracts 通过 |
| `docs-contract` / `support-contract` | 通过 | jobs [94672889290](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31769486773/job/94672889290) / [94672889221](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31769486773/job/94672889221) |
| `pr-title` / CodeQL | 通过 | [pr-title job 94672271723](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31769486784/job/94672271723)；[CodeQL run 31769486775](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31769486775) 的 JS/TS 与 Rust jobs 通过 |
| `ci-gate` | 通过 | [job 94675950139](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31769486773/job/94675950139)，绑定 `abd79c7484a51be2bee80787092db847a1466f47` |

### 人工验证

- 未运行桌面应用或开发服务器；项目合同禁止在常规 checkout 启动。UI 和真实供应商请求仍需 main 在可用候选环境中人工验收。

## 测试、文档与合同

- 测试：新增/更新 Rust 迁移、domain/CAS、五协议 matcher、SSE/非流 failover、override-gated A/B/A、session/marker/cost、bundle/share/duplicate 测试，以及前端 editor/query/service/projection/延迟请求测试。
- 现行文档：已同步 configured routing、gateway failover、config bundle、provider share、local observer/TUI、settings ownership 六份 `.trellis/spec` 合同。
- 类型或机器合同：SQLite schema v53、Rust DTO、IPC registry、`src/generated/bindings.ts` 与 config bundle schema v5 已同步。
- 模板：只同步仓库中已存在的 config migration、gateway failover、settings ownership shipped templates；未补造缺失模板。

## 兼容性、风险与回滚

- 兼容性：旧三字段/标准 effort 普通规则继续命中；旧 bundle v1-v4 可导入；历史非法或 unknown/cross-only ordinary 规则丢弃并 fail-open；未知 cross target UUID 保留为失效项。
- 数据与配置：v52 -> v53 是事务迁移，新增 identity 表和 nullable member policy，并持久化安全清洗旧 ordinary JSON；升级后的数据库 schema 不应直接交给只支持 v52 的旧二进制，回滚需先恢复数据库备份或使用兼容迁移版本。
- 安全与隐私：candidate DTO 与 marker 有界且不包含 body、URL、header 或凭据；B 复用既有 auth/gate/circuit 边界。
- 回滚方式：代码可回退本 PR；配置 bundle 可用 v5 备份恢复。数据库降级不能只回退二进制，需配套恢复升级前备份。
- 剩余风险：真实 UI、真实跨供应商 bridge/credential 组合未人工验证；桌面候选环境中的人工验收仍交由 main。

## 未完成项与剩余风险

- 产品、Round 1 基线、Round 2 和 Round 3 findings 阻塞已解除。delivery 记录提交会生成新的 PR head；执行 session 将等待该 head 的自动检查终态，并在最终报告中提供完整 SHA/链接，不再修改本文件造成自引用循环。
- 未进行桌面 UI/真实上游人工验收，交由 main 在固定候选上完成。

## 建议 main 重点审查

- `failover_loop/mod.rs`：CrossTemporary 是否始终只跳一次、占 Ready budget、processed 去重，并在所有 B 失败分支恢复 A 原始模型。
- `domain/sort_modes.rs`：组合策略 transaction/CAS、失效目标保留、Default/非成员 fail-open 与 owner revision 语义。
- `config_migrate`：v5 UUID 映射、旧 schema 导入、share/duplicate 不携带 mode-scoped policy。
- `request_end`/stream finalize：B 终端错误 marker、最终 provider/model/cost 和 session binding 在 SSE/非流路径是否一致。
- Round 2：关闭 override 的 source 是否只走 baseline 且成员 JSON/revision 保留；ordinary cache/revision 广播是否不触及 mode-local cross policy；v53 清洗与严格写边界是否只影响本任务声明的 ordinary policy 语义。
- Round 3：dirty 同 scope refetch 是否完整保留旧 draft/baseline/revision，clean refetch、保存、放弃、scope 切换和关闭是否同步采纳或清空三者，以及 CAS conflict 是否保持服务端新值不被覆盖。

## main 验收记录

### Round 4

- 结论：通过。候选 head `74523fe49213f148e4979ce01bc6b1f7d8707368` 已关闭 `F-006`，`F-001`～`F-005` 的关闭结论保持有效；未发现新的 required finding。
- 审查范围：Round 3 的 3 个变更文件、F-006 实现与新增前端测试、PRD/设计/实施/交付材料、实时 PR #137 head/base/Ready/合并状态，以及同一 head 的全部自动检查。
- 候选 base：`bd2535796fdf847008b7b55789572367d3e615e9`；接管前工作树干净，本地、远端分支和 PR head 一致，PR 为 `MERGEABLE` / `CLEAN`。
- CI 证据：[run 31770929381](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31770929381)、[CodeQL run 31770929314](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31770929314)、[`pr-title` run 31770929342](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31770929342)、[`ci-gate` job 94679845780](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31770929381/job/94679845780)；frontend、Rust tests/Clippy/audit/generated drift、docs/support、CodeQL、`pr-title` 与 `ci-gate` 均成功且绑定本候选。
- F-006 复验：cross draft 使用独立 adopted revision；dirty refetch 保留 draft/baseline/revision，clean refetch、保存成功、放弃、scope 切换和关闭成组采纳或清空；CAS conflict 测试证明旧 draft 不覆盖并发 R2。
- 本地检查：cloud-only checker、selftest、Trellis task validate、`git diff --check origin/main...HEAD` 与当前记录变更的 `git diff --check` 均通过。
- 接受的偏移或风险：无产品行为、兼容性或 AC 偏移。接受已披露的剩余验证缺口：未执行真实桌面 UI 与真实 bridge/credential 组合人工验收；自动化已覆盖编辑器状态、运行时 failover、bridge prepare、SSE/非流、CAS 与合同路径。
- 日期：2026-08-14。

### Round 3

- 结论：返工。候选 head `0280474d747ad9255d9aae6a48cbd02796d3becc` 已关闭 `F-002`～`F-005`，Ready 状态、固定 head、工作树归属和同一 head 必需 CI 均满足；前端组合编辑器的 cross revision 与 dirty draft 未绑定，新增 required finding `F-006`。
- 审查范围：Round 2 的 17 个变更文件、`F-002`～`F-005` 的实现和新增测试、实时 PR #137 状态、最新 CI，以及 provider editor 保存时序中的 ordinary/cross query cache、baseline、revision 和后端 CAS 交互。
- 候选 base：`bd2535796fdf847008b7b55789572367d3e615e9`；接管前工作树干净，本地、远端分支和 PR head 一致，PR 可合并且无冲突。
- CI 证据：[run 31766673811](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31766673811)、[CodeQL run 31766673812](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31766673812)、[`pr-title` run 31766673832](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31766673832)、[`ci-gate` job 94667044227](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31766673811/job/94667044227)；frontend、Rust tests/Clippy/audit/generated drift、docs/support、CodeQL、`pr-title` 和 `ci-gate` 均通过且 `headSha` 均为本候选。
- 已关闭 finding：override off 不再暴露 runtime cross policy 且开/关/开测试通过；ordinary cache 广播/取消旧请求与 clean revision 同步成立；global/provider/share 写入拒绝 unknown/cross-only 字段且历史读取 fail-open；v53 事务清洗、bounded projection、幂等与更新失败回滚成立。
- 新 finding：见 `findings.md` 的 `F-006`。dirty cross draft 保留旧内容却从最新 `routingPolicyView` 取 revision，provider upsert 触发的同 scope refetch 可让旧 draft 以新 revision 覆盖并发 writer。
- 接受的偏移或风险：本轮不接受 CAS 绕过。真实桌面 UI 和真实 bridge/credential 组合仍是已披露人工验证缺口，但不是 `F-006` 的替代证据。
- 日期：2026-08-14。

### Round 2

- 结论：返工。候选 head `2e7a8e284ff3b3e60678150eec0b07768f4db3a2` 已满足 F-001、Ready 状态和同一 head 必需 CI 全绿，但产品 diff 验收发现四项必须修复的问题，见 `findings.md` 的 `F-002`～`F-005`。
- 审查范围：完整 PRD/设计/实施/交付材料、实时 PR #137 diff、覆盖开关到 gateway 的数据流、普通策略 query/form 缓存、普通/全局/share 写入边界、v52 -> v53 迁移，以及运行时 failover/session binding 的独立只读复核。
- 候选 base：`bd2535796fdf847008b7b55789572367d3e615e9`；工作树干净，本地、远端分支和 PR head 一致，PR 可合并且无冲突。
- CI 证据：[run 31759041962](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31759041962)、[CodeQL run 31759042000](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31759042000)、[`ci-gate` job 94644619335](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31759041962/job/94644619335)；frontend、Rust tests/Clippy/audit/generated drift、docs/support、CodeQL、`pr-title` 和 `ci-gate` 均通过。
- finding 摘要：关闭 provider override 后 cross policy 仍参与 runtime；普通策略跨 mode cache/延迟请求可回显旧 revision；global/provider/share ordinary 写入静默忽略 cross-only 字段；v53 没有持久化清洗旧非法 effort。
- 已确认无阻断项：一次性 B 调度、Ready cap/processed 去重、B 失败模型恢复，以及 SSE/非流 `session_binding_allowed=false` 的主路径实现未发现新的 required finding。
- 接受的偏移或风险：本轮不接受上述 AC 偏差。真实桌面 UI 和真实 bridge/credential 组合仍是已披露人工验证缺口，但不是本轮四项确认缺陷的替代证据。
- 日期：2026-08-14。

### Round 1

- 结论：返工。候选 head `bbe3e8bb96ef09cdff6b791b7ee4d1d9c29b9f4d` 的 frontend 与 `ci-gate` 失败，AC-10 未满足；尚未进入产品 diff 的最终验收。
- 审查范围：实时 PR #137 状态、候选 head/base、执行 session 终态、工作树归属、失败 CI 根因和 main 基线修复。
- CI 证据：[run 31736969410](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31736969410)；frontend job `94570993792` 因 `nanoid 3.3.17 / GHSA-2v37-7h3g-55p8` 失败，Rust 和其余合同检查通过，`ci-gate` job `94577125786` 因 frontend 失败而失败。
- 基线处置：main 已通过 PR #140 修复 `nanoid 3.3.18`，实际 merge commit 为 `a0f6823be399b2a2a31fb27b9fdfb0b6cb60e885`；最终基线 CI [run 31752575312](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31752575312) 全绿。
- finding：见 `findings.md` 的 `F-001`。执行 session 只需普通 merge 最新 `origin/main`、重新取得同一新 head 的完整 CI、更新 `delivery.md` 并标记 Ready；不把基线依赖修复重复提交到功能分支，不改变产品行为。
- 接受的偏移或风险：当前不接受 CI 缺口；真实 UI/真实供应商人工验证仍可作为新交付 head 的剩余风险报告，但不能替代必需 CI。
- 日期：2026-08-14。

### Round 0

- 结论：尚未验收（规划阶段）
- 审查范围：规划材料结构和依赖门
- 审查候选 head：尚未提交
- `ci-gate`：未触发
- AC 与人工验证：产品 AC 未开始
- 接受的偏移或风险：无
- 结论与证据：待规划提交后由 main 更新
- 日期：2026-08-13

## main 收尾

- 最终结果：Round 4 验收通过；等待本验收记录 head 的自动检查后合并功能 PR
- 功能 PR 与验收候选：PR #137，已接受候选 `74523fe49213f148e4979ce01bc6b1f7d8707368`
- main 合并提交：无
- 收尾记录 PR：无
- 知识库与合同：待实现并验收
- PENDING 去向：无未解决条目
- 归档：保持活动
- worktree 与分支清理：合并和收尾记录完成前保持
- 遗留风险：真实桌面 UI 与真实 bridge/credential 组合尚未人工验证；无未关闭 required finding
