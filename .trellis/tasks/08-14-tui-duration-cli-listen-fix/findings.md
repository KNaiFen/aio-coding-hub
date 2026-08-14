# 验收整改意见：TUI 请求时间与 CLI 监听切换修复

> main 负责写问题、证据和复验要求；执行 session 不删除或改写原始意见，只在“执行回应”中说明整改和证据。

## 当前结论

- 结论：需要整改，暂不可合并。
- PR：[#147](https://github.com/KNaiFen/aio-coding-hub/pull/147)（Draft）
- 审查轮次：Round 1
- 审查版本：`8e6ca2fbb35e92e3a68544b2b07da6d087d5325f`
- CI 状态：[run 31800876521](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31800876521) 在本轮验收时仍运行；上一完整功能候选的 [Rust job](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31798457041/job/94760734452) 因未修改的 Grok SSE route test 得到 `502` 而失败。
- 审查范围：完整任务；重点复核 page-level token owner、监听设置 draft、TUI 时间字段、gateway lifecycle lock 和回归测试。

## 总结

TUI 时间字段和 Rust lifecycle lock 没有发现阻断项，但前端仍有两个可达的异步时序缺陷：保存后的 token reveal 可能复用保存前的旧请求而丢失新令牌，保存期间到达的外部 settings 更新也可能被永久跳过。此外，任务分支仍落后于已完成测试清理和 CI 合同更新的 `main`，返工候选必须先集成最新基线并重新取得固定 head 的完整证据。

## 未解决问题

- [ ] F-001 保存成功可能复用旧 reveal flight，导致 LAN 新令牌不显示
- [ ] F-002 保存期间的外部 settings 更新可能被永久跳过
- [ ] F-003 集成最新 main 并重新取得完整交付证据

## Round 1

- 本轮整改候选 head：`8e6ca2fbb35e92e3a68544b2b07da6d087d5325f`
- `ci-gate`：本轮验收时尚未终态；上一完整候选失败，见 [job 94765660089](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31798457041/job/94765660089)。
- 本轮范围：首次完整验收。

### F-001：保存成功可能复用旧 reveal flight，导致 LAN 新令牌不显示

- 严重程度：阻塞
- 返工责任：执行 session
- 对应要求：PRD Locked Decision 4、6；AC-03、AC-05。
- 证据：`src/components/cli-manager/GatewayTokenDialog.tsx:23` 的 `revealPendingGatewayToken` 在 `revealInFlightRef.current` 非空时直接返回旧 Promise；`src/components/cli-manager/NetworkSettingsCard.tsx:141` 的 `commitListenMode` 在非本地保存成功后调用同一回调。现有 `src/pages/__tests__/CliManagerPage.test.tsx:388` 让初始 reveal 立即返回 `null`，没有覆盖保存与初始 reveal 重叠的时序。
- 当前行为：页面挂载可先按旧 `localhost` 设置发起 reveal。若该请求已在后端读取旧设置并将返回 `null`、但 Promise 尚未结算，用户此时成功保存为 `lan` 后，保存路径只等待这个旧 Promise，不会再针对新的 canonical 非回环设置发起 reveal。保存成功生成的一次性令牌因此不会在本次交互中出现。
- 影响：用户仍会遇到“切到 `0.0.0.0` 后没有接口密钥提示”的原始故障；AC-03 和 AC-05 未满足。
- 必须达到的结果：保存成功产生的 reveal 意图必须在任何保存前 flight 之后至少执行一次；同一阶段的重复调用仍应去重。若前一 flight 已经取得并展示令牌，不得重复消费；若前一 flight 返回 `null`，保存后的 queued reveal 必须实际访问后端并在同一次交互中展示新令牌。
- 修改边界：限 page-level token controller、General tab 保存回调及直接相关测试；保持一次性后端 reveal、acknowledge、rotate、digest、Bearer 鉴权、明文不落盘/不写日志和 tab 卸载后结果仍可展示的现有语义。
- 测试和文档：新增 deferred 时序测试：初始 reveal 未结算时完成 `localhost -> lan` 保存，初始请求返回 `null` 后必须发生一次后续 reveal并显示令牌；同时保留并验证重复点击去重、tab 卸载、copy、close、rotate、ack 流程。
- 复验方式：main 点验 controller 的 flight/queue 所有权和新增测试，确认旧 flight 返回 `null` 不会吞掉保存后的 reveal，随后核对最新 head 的 frontend CI。

**执行回应**

- 修改：
- 代码位置：
- 验证：
- 计划偏移：

**main 复验**

- 状态：待复验
- 复验候选 head：
- `ci-gate`：
- 结论与证据：
- 日期：

### F-002：保存期间的外部 settings 更新可能被永久跳过

- 严重程度：重要
- 返工责任：执行 session
- 对应要求：PRD Locked Decision 5；AC-04；前端 canonical settings 一致性。
- 证据：`src/components/cli-manager/NetworkSettingsCard.tsx:86` 的同步 effect 在判断 `applyingNetworkSettings` 前无条件更新 `lastSettingsSourceKeyRef.current`，随后只在未应用时 dispatch。`src/components/cli-manager/__tests__/NetworkSettingsCard.test.tsx:209` 只覆盖空闲状态的外部 rerender。
- 当前行为：若 query cache/refetch 或其他 settings writer 在本组件保存期间推送新 canonical settings，effect 会记录新的 source key 却跳过 draft reset。保存结束后同一 source key 被误判为已同步，draft 可永久停留在本次 mutation 返回的旧值。
- 影响：界面显示与真实 settings 不一致，后续切换可能基于陈旧监听模式继续写入；用户可能再次看到选项无法可靠切回或状态滞留。
- 必须达到的结果：只有 draft 实际采纳某个 settings source 后才能推进已同步标记；应用期间到达的新 canonical settings 必须被延迟保留，并在应用结束后按明确的 winner 规则同步到 draft。成功、`null`、error 与 custom address 路径都不得丢失更新。
- 修改边界：限 `NetworkSettingsCard` 的 draft/canonical 同步和直接相关测试；不改变 settings mutation、schema、listen mode 解析、gateway rebind 或 CLI proxy 语义。
- 测试和文档：新增应用期间 rerender 的 deferred 测试，覆盖外部 `gateway_listen_mode`/custom address 更新在 mutation 结算后被采纳；现有 pending、成功、`null`、error 和 LAN -> localhost 测试继续通过。
- 复验方式：main 点验 source key 或 deferred state 只在实际采纳时推进，并核对新增测试能在旧实现上失败、在新实现上通过，随后核对最新 head 的 frontend CI。

**执行回应**

- 修改：
- 代码位置：
- 验证：
- 计划偏移：

**main 复验**

- 状态：待复验
- 复验候选 head：
- `ci-gate`：
- 结论与证据：
- 日期：

### F-003：集成最新 main 并重新取得完整交付证据

- 严重程度：阻塞
- 返工责任：执行 session
- 对应要求：execution.md 并行任务停止条件；AC-07、AC-08；当前仓库 CI 合同。
- 证据：任务分支仍基于 `1b218897c09894cfb5aff796761eb8004ad6e53f`，当前 `origin/main` 为 `0ae7f03abaa37c7021fdf8718373e27fe61f62fd`；`main` 已归档测试清理任务并把 workflow 收敛为 `contracts` job，任务分支的 `.trellis/tasks/README.md` 仍保留旧活动索引且缺少该归档事实。PR 实时状态为 `BEHIND`。
- 当前行为：候选尚未吸收 PR #146/#148 的测试和 CI 合同变化，任务索引与最新 `main` 存在预期集成冲突；上一完整候选的 Rust/`ci-gate` 也不是绿色。
- 影响：即使前端缺陷修复，main 仍无法证明候选基于当前仓库事实、使用当前 workflow 拓扑并满足 AC-08。
- 必须达到的结果：在登记 worktree 中普通 merge `origin/main@0ae7f03abaa37c7021fdf8718373e27fe61f62fd`，不 rebase、不 force-push、不 cherry-pick；保留 `main` 的测试清理归档和 `contracts` workflow，保留本任务活动行并更新实时 PR/head/阶段。然后推送新的完整候选并等待所选 frontend、Rust、contracts、CodeQL、`pr-title`、`ci-gate` 全绿。
- 修改边界：合并中保留 `origin/main` 的全部非冲突变化；`.trellis/tasks/README.md` 只合并索引事实，不恢复已归档测试清理任务为活动状态；不得顺带修改范围外 `gateway/routes.rs` 或放宽 Grok SSE 测试。
- 测试和文档：更新 `execution.md`、`delivery.md` 和本文件的真实 merge/head/CI 证据。若同一个未修改的 Grok SSE test 在新完整 head 再次失败，保留响应和日志链接后暂停交 main，不猜测 route 修法。
- 复验方式：`git merge-base --is-ancestor 0ae7f03abaa37c7021fdf8718373e27fe61f62fd HEAD` 成功；worktree 干净；本地、远端与 PR head 一致；最新 head 的必需检查与选中的完整 jobs 全绿；任务索引和 delivery 与实时事实一致。

**执行回应**

- 修改：
- 代码位置：
- 验证：
- 计划偏移：

**main 复验**

- 状态：待复验
- 复验候选 head：
- `ci-gate`：
- 结论与证据：
- 日期：

## CI、编译或环境问题

| 检查 | 状态 | 证据 | 期望处理 |
|---|---|---|---|
| Rust / `ci-gate`（上一完整候选） | 失败 | [Rust job 94760734452](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31798457041/job/94760734452)：`2899 passed; 1 failed; 5 ignored`，未修改的 Grok SSE test 为 `502 != 200` | 先合并当前 main 并修复 F-001/F-002；新候选若重复同一失败则停止并交 main 归因，不修改范围外 route。 |

## 计划偏移需要处理

- 无需改变用户锁定行为、公共 API 或安全语义。
- 当前 delivery 对 AC-03/AC-05 的“通过”结论已被 F-001 推翻；执行 session 再次交付时必须按真实结果更新。

## 本轮返工边界

### 必须处理

- F-001
- F-002
- F-003

### 不要顺带处理

- `src-tauri/src/gateway/routes.rs`、Grok SSE route 行为或测试阈值。
- token 后端 IPC、一次性 reveal、digest、acknowledge、rotate、Bearer 鉴权和 settings schema。
- TUI 详情页、输出速率、observer 协议及已经通过审查的 lifecycle lock 分层。
- 依赖、release、candidate、签名、性能、CodeQL 或其他无关重构。

### 可以自主决定

- page-level controller 使用 queue、generation 或等价方式表达“保存后必须再检查一次”，只要满足 F-001 的可观察结果和安全边界。
- `NetworkSettingsCard` 使用延迟 source key 或 deferred canonical draft，只要满足 F-002 的 winner 与同步要求。

## 再次交付要求

- [ ] F-001 至 F-003 都有执行回应和代码/合并证据。
- [ ] `delivery.md` 已更新实现、偏移、验证、main 集成和 Round 1 返工记录。
- [ ] 新提交已推送，PR 最新 head 的必需 CI 和相关编译为绿色。
- [ ] 完整 PR head SHA 和对应 `ci-gate` 已写入 `delivery.md` 与本文件。
- [ ] PR 已转为 Ready for review，execution session 已暂停并通知 main 复验。

## 建议项

- 非阻断：现有 Rust timeout test 使用 `mock_app`，未覆盖真实网关运行时的完整 `settings_set -> rebind -> CLI 文件更新` 和重绑启动失败后的 rollback；可在后续具备可控 gateway harness 时补端到端测试，本轮不要扩大范围。
