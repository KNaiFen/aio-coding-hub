# 验收返工：跨供应商模型路由

## Round 3

- 验收候选 head：`0280474d747ad9255d9aae6a48cbd02796d3becc`
- PR：[KNaiFen/aio-coding-hub#137](https://github.com/KNaiFen/aio-coding-hub/pull/137)（Draft；main 已在确认返工结论后从 Ready 改回 Draft）
- 通过 CI：[run 31766673811](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31766673811)、[CodeQL run 31766673812](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31766673812)、[`pr-title` run 31766673832](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31766673832)、[`ci-gate` job 94667044227](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31766673811/job/94667044227)
- 验收结论：不通过。`F-002`～`F-005` 的实现与复验要求已关闭；前端组合编辑器仍存在 `F-006`，dirty cross draft 可借同 scope refetch 得到的新 revision 绕过 CAS 并覆盖并发写入。
- 返工责任：执行 session。main 仅写本轮验收记录与返工指导，不修改产品代码、测试、依赖或现行合同。

### F-006 [P1] dirty cross draft 使用 refetch 后的新 revision，绕过并发写入 CAS

**证据**

- `src/pages/providers/useProviderEditorForm.ts:553` 只在 `provider + mode` scope 改变时采纳 cross policy/draft/baseline；同 scope refetch 时，即使 cross draft 已 dirty，也不会采纳新的 cross policy。
- 同一个 effect 在 `src/pages/providers/useProviderEditorForm.ts:562` 无条件把 `routingPolicyView` 更新为 refetch 的新 view；`src/pages/providers/useProviderEditorForm.ts:1017`～`1021` 保存 cross draft 时却从这个最新 view 读取 `cross_policy_revision`，没有保存与当前 cross baseline 绑定的独立 revision。
- 正常保存路径在 `src/pages/providers/providerEditorSaveRunner.ts:50` 先等待 provider upsert；upsert 的 `src/query/providers.ts:334`～`338` 会等待该 provider 的 routing query invalidation/refetch，随后 `providerEditorSaveRunner.ts:51`～`52` 才保存 routing policy。因此“R1 上编辑 dirty cross draft -> 另一 writer 提交 R2 -> 本次 provider upsert refetch 到 R2 -> 以 R2 revision 提交 R1 派生 draft”是实际可达时序。

**影响**

- 后端收到最新 R2 revision 后会接受旧 draft，原本应成为 CAS loser 的本次保存反而静默覆盖另一 writer 的 R2 cross policy，违反设计的 owner-scoped CAS/并发写入保护并造成配置丢失。

**期望结果**

1. 为 cross draft 保存独立、与当前 adopted baseline 绑定的 revision；保存只能使用该 revision，不能使用任意后续 refetch 的 `routingPolicyView.cross_policy_revision`。
2. 同 scope refetch 时：cross draft clean 则可以原子采纳新 cross policy、baseline 和 revision；cross draft dirty 则必须同时保留旧 draft、旧 baseline 和旧 revision。此后保存若已有并发 R2，必须由后端返回 `PROVIDER_ROUTING_CONCURRENT_UPDATE`，不得覆盖 R2。
3. 保存成功、明确放弃草稿、provider/mode scope 切换和关闭编辑器时，cross policy、baseline、revision 必须一起更新或清空；Default/非成员的 `null` revision 语义保持不变。
4. 增加前端回归测试：R1 上编辑 dirty cross draft，同 scope refetch R2 后断言表单仍保留 draft 且保存发送 R1 revision；模拟后端 CAS conflict 后不出现成功状态且 draft 不丢失。另覆盖 clean draft refetch 会采纳 R2 policy/revision。

**复验标准**

- dirty cross draft 永远使用与其 baseline 对应的 revision；同 scope 延迟/refetch 不能替换该 revision。
- 并发更新场景稳定产生 CAS conflict，服务端 R2 与本地 dirty draft 均不被静默覆盖；clean draft 和 scope 切换测试继续通过。

## Round 2

- 验收候选 head：`2e7a8e284ff3b3e60678150eec0b07768f4db3a2`
- PR：[KNaiFen/aio-coding-hub#137](https://github.com/KNaiFen/aio-coding-hub/pull/137)（Draft；main 已在确认返工结论后从 Ready 改回 Draft）
- 通过 CI：[run 31759041962](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31759041962)、[CodeQL run 31759042000](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31759042000)、[`ci-gate` job 94644619335](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31759041962/job/94644619335)
- 验收结论：不通过。F-001 已关闭，固定 head 的必需 CI 和交付前置条件均满足；产品 diff 验收发现 `F-002`～`F-005`，涉及覆盖开关运行时语义、普通策略跨 mode 缓存一致性、普通/全局策略严格写入边界和 v53 迁移持久化清洗，必须修复后重新验收。
- 返工责任：执行 session。main 仅写本轮验收记录与返工指导，不修改产品代码、测试、依赖或现行合同。

### F-002 [P1] 关闭供应商覆盖后仍会执行已保存的跨供应商规则

**证据**

- `src-tauri/src/domain/sort_modes.rs:1057` 在 `provider_override_enabled=false` 时只把普通策略 JSON 写成 `NULL`，`src-tauri/src/domain/sort_modes.rs:1163` 仍原样保留方案成员的 cross policy，符合“关闭时保留配置”的持久化要求。
- `src-tauri/src/domain/providers/queries.rs:847` 无条件把成员 cross JSON 映射到 `ProviderForGateway`；`src-tauri/src/gateway/proxy/handler/failover_loop/mod.rs:282` 也没有以普通覆盖开关状态 gate `resolve_cross_plan`。
- 因此关闭覆盖只停止普通策略，已保存的 A -> B 仍可被调度，与 PRD 3.3.7、AC-02 的“关闭时保留但运行时不生效，重新开启后恢复”相反。

**影响**

- 用户已经明确关闭该供应商的模型路由覆盖，请求仍可能切换供应商，UI 状态与实际流量行为不一致。

**期望结果**

1. 保留数据库中的成员 cross JSON，但 selection/mapper 或 `cross_temporary_work_item` 必须以源供应商普通覆盖启用状态 gate 跨规则；关闭时按原始 baseline 执行，重新开启后恢复原 cross policy。
2. 不改变 source member disable、target member eligibility、Default 或普通策略的既有语义。
3. 增加 named-mode 路由回归测试，证明“启用时 A -> B、关闭后只走原 baseline 且 DB cross JSON 不变、重开后 A -> B 恢复”；覆盖至少一个实际 gateway 请求路径。

**复验标准**

- 对关闭覆盖的源 A，运行时不会创建 `CrossTemporary`，marker/attempt/final provider 均保持原 baseline；重开后同一规则恢复。
- 关闭与重开之间持久化的 cross policy 和 revision 没有被清空或静默改写。

### F-003 [P1] 普通策略更新没有同步同供应商的其他 mode 缓存

**证据**

- 普通策略是 provider 级共享字段，但 `src/query/sortModes.ts:298` 保存组合 DTO 后只 `setQueryData` 当前 `provider + mode` identity，并只失效当前 mode 的 candidates。
- `src/query/__tests__/sortModes.test.tsx:304` 还明确断言同供应商另一 mode 的 policy cache 保持旧值。
- provider upsert 会在 `src/query/providers.ts:334` 先失效该供应商的全部 routing editor query；保存流程随后才在 `src/pages/providers/providerEditorSaveRunner.ts:50` 保存组合策略。由 upsert 启动的旧普通策略请求可能在保存后回写。
- `src/pages/providers/useProviderEditorForm.ts:534` 只在 provider key 改变时应用 ordinary policy；同一 provider 的 mode 切换或旧缓存先显示后再刷新时，新 revision 不会重新同步 ordinary draft。

**影响**

- 在 mode A 保存普通规则后，切到已缓存或仍有旧请求的 mode B、或关闭再打开编辑器，可能显示旧普通规则；继续保存会产生错误草稿或无意义的 CAS 冲突。

**期望结果**

1. 组合策略保存前后取消或隔离该 provider 的旧 routing-policy 请求，避免旧响应覆盖保存结果。
2. 保存成功后，对同 provider 的所有 mode cache 同步普通策略及其 revision，或移除/失效这些 cache 并保证重新打开时不会先把旧普通策略锁进 form state；cross policy 仍按各 mode 独立保存。
3. form 同步逻辑要识别普通 policy revision 的真实变化，同时不能覆盖用户尚未保存的 dirty draft。
4. 增加 mode A 保存、mode B 已缓存，以及 mode B 延迟旧请求在保存后返回的测试；切换和关闭重开都必须看到最新普通规则/revision，B 的 cross policy 仍保持自身值。

**复验标准**

- 保存后的所有同 provider identity 不再暴露旧 ordinary policy/revision；延迟响应不能回滚当前 cache 或 form。
- 新测试在修复前可稳定复现，修复后通过，且 dirty draft 保护测试继续通过。

### F-004 [P2] 全局和普通策略写入会静默忽略跨供应商字段

**证据**

- `src-tauri/src/infra/settings/types.rs:251` 的 `ModelRoutingRule` 使用 `#[serde(default)]` 且没有严格未知字段校验；包含 `target_provider_uuid`、`target_reasoning_effort` 等 cross-only 字段的普通/全局规则会被 serde 静默丢弃这些字段。
- 全局写入在 `src-tauri/src/app/settings_service.rs:916`、provider ordinary 写入在 `src-tauri/src/domain/sort_modes.rs:1056` 只对已经反序列化的 `ModelRoutingPolicy` 做 normalize，无法再发现被丢弃的字段。
- provider share 外层虽然使用 `deny_unknown_fields`，但其嵌套 ordinary policy 仍复用上述宽松类型，因而也不能兑现 v2 的严格未知字段边界。

**影响**

- 错误客户端或导入数据以为保存了跨供应商目标，后端却把它静默降级成普通规则；这违反 AC-01 和设计 1.2/2.1 的范围隔离与“全局策略拒绝跨字段”要求。

**期望结果**

1. 在全局设置、provider ordinary-policy 与 provider-share 导入的写入边界检测并拒绝 cross-only/未知规则字段，返回稳定的 `SEC_INVALID_INPUT`；不要只依赖类型反序列化后再 normalize。
2. 历史数据库/设置启动读取仍保持 bounded、fail-open sanitizer，不能因为严格写入边界而把坏历史数据升级为启动失败。
3. 增加原始 JSON/IPC 边界测试，分别证明 global、provider ordinary 和 share v2 对 `target_provider_uuid`/cross-only 字段拒绝，而合法旧三字段与新增 `source_reasoning_effort` 继续接受。

**复验标准**

- 三个写入入口都显式报错且不会部分持久化；历史坏数据读取仍 fail-open。

### F-005 [P2] v52 -> v53 没有持久化清洗旧普通策略中的非法目标强度

**证据**

- `src-tauri/src/infra/db/migrations/v52_to_v53.rs:190` 的事务只创建/校验 mode identity、增加成员 cross 列并推进 `user_version`，没有读取或更新 `providers.model_routing_policy_json`。
- `src-tauri/src/domain/providers/queries.rs:70` 只在运行时读取时临时 sanitize 普通策略；原始数字预算、非法强度或坏 JSON 仍留在数据库，且没有迁移级 bounded invalid projection。

**影响**

- 升级后非法规则虽然暂时不执行，却会持续存在并在后续导出/写回路径被无证据地静默丢弃，不满足 AC-01 与设计 2.1/2.2 对“迁移/写入清洗、整条删除并记录有界失效投影”的要求。

**期望结果**

1. 在同一 v53 transaction 内 fail-open 解析所有现有 `providers.model_routing_policy_json`，删除目标或来源为非标准 effort 的整条规则，并把规范化结果持久化；坏 JSON 按设计落为安全的 disabled/empty 表示，不能阻断数据库升级。
2. 记录不含原始敏感内容、数量有界的 migration invalid projection/计数；迁移失败必须整体回滚且不得推进 `user_version`。
3. 增加 v52 fixture 测试，覆盖旧三字段、八项标准 effort、数字预算文本、非法字符串和 malformed JSON；断言升级后的原始数据库值已清洗、合法规则保留、重复迁移幂等、故障时回滚。

**复验标准**

- v53 完成后直接查询数据库不再包含非法普通规则；合法旧规则语义不变，迁移计数有界且不泄露原始规则内容。

## Round 1

- 验收候选 head：`bbe3e8bb96ef09cdff6b791b7ee4d1d9c29b9f4d`
- PR：[KNaiFen/aio-coding-hub#137](https://github.com/KNaiFen/aio-coding-hub/pull/137)（Draft）
- 失败 CI：[run 31736969410](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31736969410)
- 验收结论：未进入产品 diff 验收。AC-10 的同一 head 必需 CI 全绿与 Ready 状态未满足，先按本 finding 恢复交付门。
- 返工责任：执行 session。main 已通过独立 PR #140 修复仓库基线；本轮只要求同步基线、复验和更新交付记录，不要求重做产品实现。

### F-001 [P1] 同步已修复的 main 基线并重新取得完整交付证据

**证据**

- 候选 head 的 frontend job `94570993792` 在依赖审计阶段报告 `nanoid / GHSA-2v37-7h3g-55p8` 后失败，lint、typecheck、frontend tests 和 build 未执行；`ci-gate` job `94577125786` 因 `FRONTEND_RESULT=failure` 失败。
- PR #137 未修改 `package.json`、`pnpm-workspace.yaml` 或 `pnpm-lock.yaml`，该失败不是跨供应商路由 diff 引入的回归。
- main 已通过 [PR #140](https://github.com/KNaiFen/aio-coding-hub/pull/140) 把 `nanoid` 从受影响的 `3.3.17` 升到首个安全 3.x 版本 `3.3.18`；最终 head `abe221420d597a4afed873f85e75686ddb72fce7` 的 [CI run 31752575312](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31752575312) 已通过 frozen install、pnpm audit、frontend、Rust、docs/support contract、CodeQL、`pr-title` 和 `ci-gate`。实际 main merge commit 为 `a0f6823be399b2a2a31fb27b9fdfb0b6cb60e885`。
- 只读 `git merge-tree` 预检显示 `origin/main` 可普通 merge 到当前任务分支，不存在内容冲突。`.trellis/tasks/README.md` 是双方都修改但可自动合并的文件；`pnpm-workspace.yaml`、`pnpm-lock.yaml` 和 `docs/history/change-records/2026-08.md` 是 main 单边变更，必须完整保留。

**影响**

- AC-03、AC-04、AC-08、AC-09 缺少最新候选 head 的 frontend 云端测试/构建证据。
- AC-10 未满足；PR 不能标记 Ready，main 不能开始固定 head 的产品验收或合并。

**期望结果**

1. 在登记 worktree 和分支中重新做 preflight，确认路径、分支、`task.json.status=in_progress`、规划提交、工作树干净且本地/远端 head 一致。
2. `git fetch origin` 后使用普通 merge 同步 `origin/main`，不要 rebase、不要 force-push、不要 cherry-pick PR #140：

   ```bash
   git merge --no-edit origin/main
   ```

3. 保留 `origin/main` 的全部非冲突变化，尤其是 `nanoid 3.3.18` 的 workspace/lockfile 修复和 TUI 任务归档事实。若实际出现与只读预检不一致的内容冲突，或必须改变产品行为/现行合同才能继续，立即停止并报告 main。
4. 不修改跨供应商产品实现、测试逻辑、`src-tauri/crates/aio-tui/src/format.rs`、依赖版本或审计 allowlist；除任务 `delivery.md`/`execution.md`/`findings.md` 等交付记录外，本轮产品树变化只应来自合并 `origin/main`。
5. 运行 `execution.md` 允许的本地检查，推送新的完整 head，等待该 head 的自动 `change-scope`、frontend、Rust、docs/support contract、`pr-title`、CodeQL 和 `ci-gate` 终态；不要手动启动额外 `ci` run。
6. 若 CI 发现属于跨供应商路由 diff 的新失败，按原任务范围修复；若出现新的基线/基础设施失败且没有任务内安全修法，保留日志并停止报告 main。
7. 基于实际新 head 更新 `delivery.md`：删除已解除的 nanoid 阻塞表述，记录 merge commit、完整 PR head/base SHA、同一 head 的 `ci-gate` 链接、frontend/Rust/本地检查结果、剩余人工验证风险和 Round 1 返工结果。检查绿色后把 PR 标记 Ready for review，停止写入并通知 main 验收。

**复验标准**

- `git merge-base --is-ancestor a0f6823be399b2a2a31fb27b9fdfb0b6cb60e885 HEAD` 成功。
- worktree 干净，本地 `HEAD`、远端任务分支和 PR `headRefOid` 完全一致。
- 最新完整 head 的 required `ci-gate`、`pr-title` 及其选中的 frontend、Rust、docs/support contract 和 CodeQL 全部成功；没有沿用 `bbe3e8bb...` 的过期证据。
- `delivery.md` 与实时 PR/CI 一致，PR 已从 Draft 切换为 Ready for review；执行 session 已停止写入。
- main 将在新的固定 head 上重新进行产品 diff、AC、回归风险、测试与文档验收。此次基线修复绿色不预先代表 PR #137 的产品验收通过。
