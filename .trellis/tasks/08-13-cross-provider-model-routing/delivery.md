# 交付报告：跨供应商模型路由

> 本文件记录执行 session 已完成的产品实现和当前阻塞状态。main 验收与收尾区仍由 main 填写；任何新 push 都会使旧 head 的 CI 证据不再代表最新 PR head。

## 交付状态

- 结果：阻塞。范围内实现、Rust/合同验证已完成；必需 frontend job 被 `origin/main` 已存在的 `nanoid 3.3.17` 高危公告阻断，因而 `ci-gate` 未通过，PR 保持 Draft。
- PR：[KNaiFen/aio-coding-hub#137](https://github.com/KNaiFen/aio-coding-hub/pull/137)（Draft）
- 分支：`feat/cross-provider-model-routing`
- 初始派生 base：`main` @ `875ff441c5ba9f1a7f235ad95dadb945a41bba61`；实时 PR base：`main` @ `70f103467c5770c7a7a29f564b7a5620409fff5a`
- 最后安全实现 head：`8365ccad51fdce9faf4fb363dbe275d0c6701561`
- 规划提交：`c6d59507c7a1de46abdb07427aa8bc153c69739c`
- `ci-gate`：未通过，[run 31734427079](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31734427079) / [job 94568983595](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31734427079/job/94568983595)，绑定最后安全实现 head；gate 输入明确为 `FRONTEND_RESULT=failure`、`RUST_RESULT=success`。
- 其他检查：Rust、change-scope、docs-contract、support-contract、pr-title、JS/Rust CodeQL 通过；frontend 失败；候选构建按 PR 范围跳过。
- 交付时间：2026-08-14 03:33 CST（实现 head 的 CI 终态时间）
- 执行 session：本阻塞记录提交并等待其最新 head 检查终态后暂停；不会标记 Ready for review。

## 阻塞快照

- 证据：[frontend job 94562654985](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31734427079/job/94562654985) 在 `Dependency audit` 报告 `nanoid / GHSA-2v37-7h3g-55p8` 后失败，lint、typecheck、前端 tests 与 build 均未执行。
- 最后安全提交：`8365ccad51fdce9faf4fb363dbe275d0c6701561`
- 工作树状态：实现提交已推送；写入本记录前工作树干净。
- 受影响的 AC/范围：AC-03、AC-04、AC-08、AC-09 的前端云端测试证据缺失；AC-10 的必需 CI 全绿与 Ready 状态未满足。产品实现和 Rust 验证可保留。
- 需要的决定：依赖升级不在本任务允许范围内；main 需安排 base 依赖修复或确认其他合规处置，本执行 session 不把锁文件升级混入功能 PR。
- 恢复条件：`main` 修复 `nanoid` 基线漏洞并使 PR #137 最新 head 的 frontend 与 `ci-gate` 通过；随后执行 session 才能更新最终候选并标记 Ready。

## 实现摘要

### 用户可见结果

- 供应商覆盖编辑器将普通规则和当前命名调用方案绑定的跨供应商规则分区编辑；Default、非成员、dirty draft、失效目标、严格 effort 下拉与模型目录建议均有明确状态。
- 推理请求可按固定优先级匹配来源模型/标准 effort，并最多一次从当前候选 A 临时跳转到同 CLI、当前命名方案中的启用成员 B；B 失败后恢复原始 A -> C 基线。
- 请求日志/Home/Observer 使用最终成功供应商、模型和费用，并保留有界 cross marker 与 attempts；没有修改 TUI formatter、TTFB 或“切/重”文案。

### 内部实现

- SQLite schema v53 新增稳定 mode UUID identity 与成员 cross policy；组合 domain/query/IPC 以 mode/provider UUID 和 owner revision 读写，坏 JSON fail-open。
- 五个协议入口只提取八项标准来源 effort；matcher 固定为“跨精确 > 跨通配 > 普通精确 > 普通通配”，Gemini 出站只写 `thinkingLevel`。
- failover 使用完整 `ProviderForGateway` 快照构造 `CrossTemporary` work item，复用既有 gate、凭据、bridge、retry、circuit 和 Ready 上限；B 禁止 session binding 且不链式匹配。
- bundle v5 保留方案/provider UUID 与成员策略；v1-v4 兼容导入，provider share 剥离 cross policy，duplicate 仅复制普通策略且不复制方案成员。

## 验收标准对应

| 标准 | 结果 | 证据 |
|---|---|---|
| AC-01 数据模型与向后兼容 | 实现完成，Rust 验证通过 | `settings/types.rs` sanitizer、`domain/sort_modes.rs` fail-open/CAS 测试、生成 TS bindings；Rust job 通过 |
| AC-02 方案持久化 | 实现完成，Rust 验证通过 | `v52_to_v53.rs`、baseline v25、迁移测试覆盖 UUID 回填/不可变/级联/回滚，方案 CRUD 测试覆盖改名与删除 |
| AC-03 编辑器与候选 | 实现及前端测试代码完成，云端执行被阻塞 | `ModelRoutingPolicyFields.tsx`、provider form/query；`ProviderRoutingEditor.test.tsx` 覆盖 Default/非成员/失效目标/dirty draft/目录建议 |
| AC-04 强度提取与匹配 | Rust 验证通过；前端测试未运行 | `model_inference.rs` 覆盖五入口与预算排除，`configured_model_route.rs` 覆盖精确/通配/大小写及 Gemini 写入 |
| AC-05 运行时跳转 | Rust 验证通过 | `failover_loop/mod.rs` 与 `gateway/routes.rs` 覆盖 B 非流/SSE 成功、B 失败 A 基线恢复、bridge prepare 失败恢复及 processed 去重 |
| AC-06 资格与安全边界 | Rust 验证通过 | effective mode snapshot、inference-only gate、managed alias/非推理排除、bounded marker/candidate DTO 测试 |
| AC-07 会话与审计 | Rust 验证通过 | attempt 级 `session_binding_allowed=false`；request-end/stream finalize、cost/Observer/Home marker 投影测试通过 |
| AC-08 导入/分享/复制 | Rust 验证通过；前端服务测试未运行 | config v5/v1-v4 导入测试、share/duplicate 测试；最新 Rust suite 修复并通过单连接池 duplicate 路径 |
| AC-09 测试与合同 | 部分通过 | Rust 2895 项中 2890 passed/5 ignored；六份现行合同、三份 shipped template、docs/support contract 通过；frontend tests 被依赖审计前置阻断 |
| AC-10 交付门 | 未通过 | PR/实现/推送/delivery 已完成，但 frontend 与 `ci-gate` 失败，故保持 Draft、未标记 Ready |

## 主要代码位置

| 文件或符号 | 变更 | 设计原因 |
|---|---|---|
| `infra/db/migrations/v52_to_v53.rs`、`domain/sort_modes.rs` | mode UUID、成员策略、组合 DTO/CAS | 把跨规则绑定稳定命名方案成员，而不污染普通 provider policy |
| `gateway/proxy/handler/middleware/model_inference.rs`、`gateway/configured_model_route.rs` | 标准 effort 提取、纯 matcher、Gemini 写入和 marker | 集中协议边界并保持匹配顺序确定 |
| `gateway/proxy/handler/failover_loop/mod.rs` | `CrossTemporary` 一次性 B work item | 复用原 failover 基线与公共资格/重试状态机，不创建第二套路由器 |
| `gateway/routes.rs` | effective mode/member snapshot 和端到端回归 | 请求开始时冻结完整目标快照，避免运行中重读 active 指针 |
| `ModelRoutingPolicyFields.tsx`、`useProviderEditorForm.ts` | 普通/跨规则 UI 与草稿状态 | 在既有供应商编辑器内实现锁定交互，不扩展全局规则 |
| `infra/config_migrate/*`、`domain/providers/share.rs`、`app/provider_service.rs` | bundle v5、share/duplicate 边界 | 保持旧 bundle 兼容且不把方案成员隐式带入单供应商操作 |
| `gateway/request_end.rs`、`app/observer/snapshot.rs`、`requestLogPresentation.ts` | attempts、marker、最终 provider/model/cost 投影 | 审计完整，同时避免把失败 B 当最终结果 |

## 与计划的偏移

- 产品行为、兼容性、范围和 AC 无偏移；严格按阶段 0 -> 8 推进。
- CI 首轮发现四条本任务 Clippy 警告，使用参数 DTO、返回类型别名和 boxed 完整 provider 快照消除；没有用 `allow` 掩盖。
- CI 次轮发现本任务抽取的 duplicate helper 在单连接池内嵌套取连接；缩短 source 读取连接作用域后，最新 Rust suite 通过。该修复不改变复制语义。
- 两轮云端 rustfmt artifact 均先核对文件范围后原样应用；最终 head 的 generated-file drift 检查通过。

## 验证结果

### 本地检查

| 命令 | 结果 | 说明 |
|---|---|---|
| `node scripts/check-cloud-only-verification.mjs` | 通过 | 仓库云端验证合同通过 |
| `node scripts/check-cloud-only-verification.selftest.mjs` | 通过 | 全部断言通过 |
| `python3 ./.trellis/scripts/task.py validate .trellis/tasks/08-13-cross-provider-model-routing` | 通过 | 12 implement / 11 check entries |
| `git diff --check` | 通过 | 已提交实现无 whitespace error |
| `git diff --name-only origin/main...HEAD` | 通过 | 已审查 PR 文件范围；当前 PR diff 不含 `src-tauri/crates/aio-tui/src/format.rs` |
| `node --check <修改的 .mjs>` | 不适用 | 本任务未修改 `.mjs` 文件 |

未在本地运行 pnpm、Cargo、rustfmt、Clippy、测试、构建或绑定生成；这些命令按仓库合同仅由 GitHub Actions 执行。

### GitHub CI 与编译

| Workflow / Job | 结果 | 链接或说明 |
|---|---|---|
| `change-scope` | 通过 | [job 94562524729](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31734427079/job/94562524729)，`scope=full` |
| `rust` | 通过 | [job 94562654975](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31734427079/job/94562654975)：格式/绑定无 drift、Clippy、Rust tests、cargo audit 通过；主 lib 2890 passed/5 ignored |
| `frontend` | 失败（base 漂移） | [job 94562654985](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31734427079/job/94562654985)：`nanoid 3.3.17 / GHSA-2v37-7h3g-55p8`；本 PR 未改 package/lock/workspace 文件 |
| `docs-contract` / `support-contract` | 通过 | jobs [94562581505](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31734427079/job/94562581505) / [94562581449](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31734427079/job/94562581449) |
| `pr-title` / CodeQL | 通过 | pr-title 与 JS/TS、Rust CodeQL 均绑定实现 head 通过 |
| `ci-gate` | 失败 | [job 94568983595](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31734427079/job/94568983595)，仅 `FRONTEND_RESULT=failure` 阻止 full gate |

### 人工验证

- 未运行桌面应用或开发服务器；项目合同禁止在常规 checkout 启动。UI 和真实供应商请求仍需 main 在可用候选环境中人工验收。

## 测试、文档与合同

- 测试：新增/更新 Rust 迁移、domain/CAS、五协议 matcher、SSE/非流 failover、session/marker/cost、bundle/share/duplicate 测试，以及前端 editor/query/service/projection 测试。
- 现行文档：已同步 configured routing、gateway failover、config bundle、provider share、local observer/TUI、settings ownership 六份 `.trellis/spec` 合同。
- 类型或机器合同：SQLite schema v53、Rust DTO、IPC registry、`src/generated/bindings.ts` 与 config bundle schema v5 已同步。
- 模板：只同步仓库中已存在的 config migration、gateway failover、settings ownership shipped templates；未补造缺失模板。

## 兼容性、风险与回滚

- 兼容性：旧三字段/标准 effort 普通规则继续命中；旧 bundle v1-v4 可导入；非法 effort 规则丢弃并 fail-open；未知目标 UUID 保留为失效项。
- 数据与配置：v52 -> v53 是事务迁移，新增 identity 表和 nullable member policy；升级后的数据库 schema 不应直接交给只支持 v52 的旧二进制，回滚需先恢复数据库备份或使用兼容迁移版本。
- 安全与隐私：candidate DTO 与 marker 有界且不包含 body、URL、header 或凭据；B 复用既有 auth/gate/circuit 边界。
- 回滚方式：代码可回退本 PR；配置 bundle 可用 v5 备份恢复。数据库降级不能只回退二进制，需配套恢复升级前备份。
- 剩余风险：frontend tests/build 尚未在最新 head 执行；真实 UI、真实跨供应商 bridge/credential 组合未人工验证；PR 当前落后于实时 main，main 验收前需评估同步策略。

## 未完成项与阻塞

- 必需 frontend 与 `ci-gate` 未通过，不能标记 Ready。阻塞根因是 base 锁定的 `nanoid 3.3.17`，不属于本任务范围；恢复后须在同一最新 head 重跑完整 scope。
- delivery 记录提交会生成新的 PR head；执行 session 将等待该 head 的自动检查终态并在最终报告中提供其完整 SHA/链接，不再修改本文件造成自引用循环。
- 未进行桌面 UI/真实上游人工验收，交由 main 在 CI 恢复后的候选上完成。

## 建议 main 重点审查

- `failover_loop/mod.rs`：CrossTemporary 是否始终只跳一次、占 Ready budget、processed 去重，并在所有 B 失败分支恢复 A 原始模型。
- `domain/sort_modes.rs`：组合策略 transaction/CAS、失效目标保留、Default/非成员 fail-open 与 owner revision 语义。
- `config_migrate`：v5 UUID 映射、旧 schema 导入、share/duplicate 不携带 mode-scoped policy。
- `request_end`/stream finalize：B 终端错误 marker、最终 provider/model/cost 和 session binding 在 SSE/非流路径是否一致。

## main 验收记录

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

- 最终结果：尚未开始
- 功能 PR 与验收候选：无
- main 合并提交：无
- 收尾记录 PR：无
- 知识库与合同：待实现并验收
- PENDING 去向：无未解决条目
- 归档：保持活动
- worktree 与分支清理：保持，执行 session 尚未启动
- 遗留风险：见“未完成项与阻塞”
