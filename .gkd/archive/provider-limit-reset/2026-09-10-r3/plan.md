# 供应商周期限额手动重设 PLAN

- Revision: r3，2026-09-10；用户确认后续周期沿新起点持续递推，并强调四种周期重设互不影响。
- 状态: r3 已批准执行。用户在本轮回复“可以，开始吧”，批准按此方案实施、云端验证、独立审查、合并 main 及任务归档清理。
- 执行现场: `../provider-limit-reset`，分支 `feat/provider-limit-reset`，baseline `25b4203165e709f9c9b0d8f18c0290b12c296b4a`；执行前 fetch 和源码差异检查确认基线未变。主树其他任务差异保持原样。
- 目标仓库: `KNaiFen/aio-coding-hub`，目标分支 `main`。
- 路线: `delegated/automatic`，涉及前端交互、IPC、持久化与网关实际限额判断。

## 目标与用户决定

用户要求 AIO 支持手动清除本地供应商限额，明确范围是 5 小时、日、周、月，累计不用处理；入口在供应商配置页原有限额设置旁，使用刷新图标，四个周期分别操作。

成功标准是用户在现有供应商编辑界面重设某个周期后，该周期已记录的用量清零，整个周期从本次操作时刻重新起算。用户的验收示例：原周窗口 9/1–9/8，在 9/7 重设后立刻变为 9/7–9/14，窗口内累计为 0。数据库事务提交后，运行中的网关下一次限额判断和界面查询立即采用新窗口，不需要保存供应商表单、重启应用或重启网关。持久化只保证以后重启仍保留结果，不是生效条件。

四种周期的起点、终点、周期内累计及重设分界彼此独立。重设周限额只清零周用量并更新周周期，月、日、5h 的周期与累计均不改变；其他任意周期的重设同理。总累计限额及累计用量、金额上限、历史请求和费用统计均保留。选中周期的新起点替代其原自然周期边界，后续按该新起点持续递推；其他周期仍可独立阻止供应商被选中。

## 当前事实

- 本地主工作树 HEAD 为 `4ae9431f3c61f4b2b90ed3fe044431195524a341`；远端 main 经 `git ls-remote` 确认为 `25b4203165e709f9c9b0d8f18c0290b12c296b4a`。`git diff HEAD origin/main -- src src-tauri` 为空，可复用当前源码调查；执行从获批时再次确认的远端 main 创建工作树。
- 主树存在既有归档修改及旧 `.gkd/plan.md` 删除。旧 CI 调度任务已合并为 PR #195，主树 `.gkd/archive/ci-scheduling-cost/2026-09-08-r1/summary.md` 明确原活动计划已归档并移除。本任务复用空出的活动计划位置，不纳入其他任务差异。
- `src/pages/providers/LimitsSection.tsx:16` 提供原有限额设置；四个周期使用 `LimitCard`，累计位于独立区域。`ProviderEditorDialog` 区分 create/edit；`ConfirmDialog` 有现成确认与处理中交互。
- `src-tauri/src/domain/provider_limit_usage.rs:239` 按 `usage_events` 聚合周期用量与累计费用；`resolve_5h_starts` 在存储窗口过期/缺失时会查询旧事件，单纯置空窗口会把旧用量重新选回。
- `src-tauri/src/gateway/proxy/handler/failover_loop/prepare/provider_limits.rs:606` 执行实际限额判断：5h 固定窗口，daily 为 fixed/rolling，周/月按本机自然日历。网关在 ledger 回填前后分别用 request_logs/usage_ledger，保持索引查询路径。
- `usage_events.id` 在两种来源均是 request_log_id；`request_logs` 使用 AUTOINCREMENT，日志与 ledger 同事务写入，pending 的终态更新沿用同一 ID。秒级时间本身不能区分同秒重设前后的请求。
- schema 当前为 54。迁移和 always-run ensure 惯例见 `src-tauri/src/infra/db/migrations/mod.rs`、`ensure.rs`。历史保留与回填语义由 `.trellis/spec/aio-coding-hub/cross-layer/request-log-usage-ledger-pagination-contract.md` 约束。
- 远端规则实际要求 `ci-gate`、`pr-title`，严格更新基线。仓库 PUBLIC。远端 #195 已将选中的 frontend/rust/observer-macos 改为等待 contracts 成功；执行沿用远端工作流。

## 用户行为

1. 仅在编辑已保存供应商时，在 5 小时、每日、每周、每月消费上限标题旁分别显示一个固定尺寸的 `RefreshCw` 图标按钮。新增/复制尚未保存的供应商没有可重设的历史，暂不显示。累计卡片不添加按钮。
2. 按钮有对应周期的 tooltip 和 accessible name，例如“重设每周限额窗口”。这是独立即时命令，不能提交或修改金额表单。
3. 点击打开现有 ConfirmDialog，标明供应商与周期，说明“当前周期用量清零，周期从现在重新开始”，历史费用保留。确认成功即生效；取消不写数据库。原编辑器里的未保存配置保持原样，重设也不替代保存；取消编辑不会撤销已确认的重设。
4. 重设期间禁止重复操作、保存和关闭编辑器；成功后立即刷新对应供应商的周期起止时间与用量，并使 `providerLimitUsageKeys.all` 失效，已挂载的其他限额视图自动重新查询，不依赖下一次定时轮询或重新打开页面。失败显示真实错误并允许重试。并发状态采用现有组件/hook 惯例，不新增全局操作队列。
5. 重设时刻为 T，新的首个窗口是 `[T, T+5h)`、`[T, T+24h)`、`[T, T+7天)` 或 `[T, T+1个月)`，只有所选周期发生变化。日限额原为 rolling 时，手动重设也必须建立从 T 起的完整 24h 周期，不能让重设前的历史请求影响新额度。所有窗口使用半开区间，边界时刻的请求属于下一周期。
6. 后续周期以此次 T 为新锚点持续递推；例如周窗口 9/7–9/14 后是 9/14–9/21，不在 9/8 恢复原自然周；9/7 重设月限额得到 9/7–10/7，之后是 10/7–11/7。用户已明确确认这种延续方式。5h/日/周分别按 5h/24h/168h 固定时长递推；月按本机时区的日历月同日同时刻递推，目标月不存在该日期时取月末，每次从原锚点推导，避免 1/31→2/28 后永久漂移到 28 日。月边界遇夏令时重复时间取较早者，缺失时间按时差顺延。未手动重设的周期仍用原规则。
7. 每日配置的 fixed/rolling 与重置时间原值保留；存在手动日锚点时由新 24h 周期覆盖其运行窗口，并显示真实周期起止。之后用户显式修改并保存每日模式或固定时间时，结束日锚点对窗口的覆盖，恢复新配置，已保存的 cutoff 继续排除重设前记录；仅改金额或其他字段不撤销锚点。保存未更改的表单不撤销重设。
8. 未设置金额的周期也可预先清除已有用量，以便随后设置金额；金额 0 保持既有“达到 0 即受限”规则。累计或其他周期仍超限时，不能承诺整个供应商恢复可用。OAuth 上游配额和熔断状态不由本操作更改。

## 技术方案

### 持久化与分界

新增一个仅保存本地运行状态的小表 `provider_limit_resets`，以 `(provider_id, period)` 为主键；period 只支持 `5h`、`daily`、`weekly`、`monthly`，不支持 total。保存可空的 `reset_at` Unix 秒及 `request_log_id_cutoff`；重设总是写入真实时间，之后显式改变每日调度时仅把 daily 的 reset_at 置空以停用其窗口覆盖，保留 cutoff。provider 外键按项目惯例级联删除；复制供应商不复制运行状态，现有配置导入/分享不暴露该表。新增 v54->v55 迁移并接入 ensure，空表等价于现有行为，不做历史重算或遍历。

重设在一个 IMMEDIATE 事务中完成：验证供应商存在和周期合法，读取 request_logs 的 AUTOINCREMENT 分配高水位，仅 upsert `(provider_id, period)` 对应的一行；仅选中 5h 时同时更新 providers.window_5h_start_ts=reset_at，然后提交。其余周期记录保持不变，不使用供应商级共用 reset_at/cutoff。不得通过 MAX(usage_ledger.id) 推定所有历史边界，避免回填/保留删除使旧记录回流。无日志的初始高水位为 0。

周期聚合使用该供应商、该周期的实际 `[start_ts, end_ts)`，并要求事件 request_log_id 大于该周期记录的 cutoff。有有效手动锚点时按照新起点推导当前窗口，不使用 `max(原自然周期起点, reset_at)`，该做法会在原自然边界过早清零。累计聚合完全沿用原始事件，因此同一个查询中不能在公共 WHERE 全局过滤 reset cutoff。不得在窗口过期后通过历史事件 MIN(created_at) 重新选择手动周期起点。

分界按“首次持久化 + 实际窗口”定义：重设前已落库请求（包括 pending）即使随后补齐费用也不计入该重设周期；重设后新落库请求按新时间窗口计量。请求开始时间早于新起点的在途请求不会因晚落库而进入新窗口；与重设同秒且尚未落库的在途请求可能计入，因此有正在请求时用量可迅速再次增长。无需取消、排空或重启网关。ID 分界区分同秒已记录的新旧数据，且在 ledger 回填切换及请求明细清除后保持有效。普通应用重启和未改变每日调度的编辑保存不会清除重设标记。

### 查询与网关

- `provider_limit_usage::list_v1` 批量加载每个供应商的四个标记，复用现有 provider_windows 批量聚合；为每个供应商传入实际周期起止及 cutoff，周/月不再只能用一组全局自然周期参数。只在对应 CASE 中应用这些条件，保留累计值，不能增加按供应商逐条聚合查询。
- 在现有限额 domain 内放置供显示查询和网关共用的手动周期边界计算；复用已有 SQLite 日历能力及已有 chrono 依赖，不引入新的时间库或通用调度框架。5h/日/周按锚点和整数周期数直接推导，月从原锚点按月份序号推导，不按经过的空闲周期逐个写库追赶。
- 网关 `evaluate_provider_limits` 每次评估读取同一份持久标记和实际边界；给现有成本行查询携带稳定 request_log_id，按周期分别决定是否累加。保留现有 ledger/request_logs 来源选择与 final_provider_id 索引分支，不把热路径换成复杂兼容视图扫描。
- 有手动锚点的周期达到限额时，下一可用时间使用该新窗口的真实结束时刻。原 rolling daily 只有没有生效的日锚点时才使用滚动 bucket 推断；不能对已重设的日周期继续按单条旧请求到期计算恢复时间。
- 重设事务提交是生效点；任何在其后开始的网关限额评估都读取新状态，无需重建路由缓存、重新启用供应商或重启。已完成的路由判断及正在传输的请求不追溯取消。原有同秒/在途记录归属按上文处理。
- `ProviderLimitUsageRow` 保留现有四个 start 字段并增加对应四个 end 字段，均由后端给出实际值；增加本地日窗口是否使用手动锚点的布尔信息，避免将重设后的固定 24h 仍标成 rolling。首页 `HomeProviderLimitPanel` 使用服务端起止值，移除手动推算月末/下月 1 日的逻辑；编辑器限额区域显示当前实际周期和用量，重设后立即更新。

### IPC 与前端

在现有 `commands/provider_limit_usage.rs` 增加 `provider_limit_reset`，参数为 `provider_id` 和受约束的 period 枚举，事务提交后返回确认（`Result<(), String>`）；随后 mutation 等待对应限额 query 重新读取，刷新成功后才展示完成状态。数据库重设错误与随后读取错误按实际阶段分别显示，不将“重设已提交但读数失败”冒称重设未生效。命令注册进入 `commands/registry.rs`，由现有 Specta 云端导出产生 TypeScript binding。前端在 `services/providers/providerLimitUsage.ts` 与 `query/providerLimitUsage.ts` 增加对应 service/mutation，不新增另一条手写 IPC 通道。

`LimitCard` 只增加可选的局部标题操作位置/重设 props 及当前周期简短数据；`LimitsSection` 接入四个按钮和已有本地限额 query。`ProviderEditorDialog`/`useProviderEditorForm` 只按需要接入 edit provider ID、确认与 pending 状态，复用现有 Dialog、Tooltip 和 toast。有手动窗口时不再展示“自然周周一/自然月 1 日”作为当前窗口规则，用实际起止日期替代。主界面风格沿用当前组件，不附带改版。

## 文件范围

- 前端: `src/pages/providers/{LimitCard,LimitsSection,ProviderEditorDialog}.tsx`、`src/pages/providers/useProviderEditorForm.ts`；`src/services/providers/providerLimitUsage.ts`；`src/query/providerLimitUsage.ts`；`src/components/home/HomeProviderLimitPanel.tsx` 的真实周期显示。审查遗漏补齐允许 `src/query/providers.ts` 保存后的限额缓存刷新及现有 query tests。
- Rust: `src-tauri/src/domain/provider_limit_usage.rs`；`src-tauri/src/commands/provider_limit_usage.rs`、`commands/registry.rs`；`src-tauri/src/infra/db/migrations/{mod,ensure,tests}.rs` 和新增 `v54_to_v55.rs`；`src-tauri/src/gateway/proxy/handler/failover_loop/prepare/provider_limits.rs`。
- 仅必要联动: `src/generated/bindings.ts` 云端生成差异、`HomeOverviewPanel.tsx` 的 preview fixtures 和 `ProviderLimitUsageRow` 的直接构造测试数据；`domain/providers/queries.rs` 比较数据库已有和待保存的每日模式/时间，明确改变调度时在同事务把日标记的 reset_at 置空并保留 cutoff；若 provider 删除没有实际 FK cascade，则同事务清除该 provider 的 reset 标记并补现有删除测试。
- 验证: 扩展现有 `src/pages/providers/__tests__/ProviderEditorDialog.test.tsx`、`src/query/__tests__/providerLimitUsage.test.tsx`、`src/components/home/__tests__/HomeProviderLimitPanel.test.tsx`、`src/services/providers/__tests__/providerLimitUsage.service.test.ts`，以及上述 Rust 限额/迁移模块和 provider 配置保存已有测试。仅在现有测试结构要求时联动直接引用的新命令 mock。
- 文档: 在现有 request-log/usage-ledger 合同补充本地限额重设不删除历史及分界语义；README 使用流程增补简短入口说明；本任务 `.gkd/` Markdown。
- CI 真实失败的最小依赖修复: `pnpm-workspace.yaml` 和 `pnpm-lock.yaml` 现有 js-yaml 4.3.1 固定引用升级为已核对的 4.3.2，版本、integrity 及现有依赖边一致更新；不引入新依赖或变更审计门槛。

若目录中现有文件后缀或测试拆分不同，等价同职责文件定位可在 progress 记录后执行；出现额外产品范围、公共接口或语义改变时由 main 更新决定。

## 必要验证

本机已确认 Darwin arm64、macOS 26.5.1、Mac16,12、16 GiB RAM；磁盘总量约 245 GB、可用约 112 GB，低于 512 GB 档。依赖安装、包脚本、编译、完整测试/覆盖率、Rust 格式与绑定生成仅在标准 GitHub Actions。不得绕过环境 guard，不启动本地开发服务器。

本地仅运行 `git diff --check`、Git/gh 只读查询及阅读差异；计划检查只验证本任务文件，不要求主树全局干净。

必要行为证据在现有测试内组织，避免为小型 helper 逐个加测试：

1. 编辑器四个按钮分别提交正确 period；累计和 create 模式无按钮；取消不调用；确认中不重复、不误保存；成功后原页面的周期日期与用量立即更新，已挂载限额查询同步刷新，失败可重试，未保存表单值保留。首页月窗口使用后端真实 end，9/7 重设时显示 10/7，不显示 10/1。
2. 在同一个正在运行的网关/数据库测试现场，对四个周期及原 fixed/rolling daily 检查：重设前超限；重设事务返回后立刻读取新日期、选中周期为 0 并按新窗口判断放行；不重新创建网关、不重新打开数据库、不触发配置保存。之后同秒新请求重新累积，其他周期和总额完全不变。核心周示例固定验证 9/7 重设得到 9/7–9/14，9/8 不提前清零。
3. 验证新周期 end 前后及连续多个周期：费用不跨新边界累计，next_available 与真实 end 一致，跨过原自然日/周/月边界时窗口不变；月末、闰年、夏令时按已定语义，空闲跨多轮不把新窗口推迟到下一次请求才开始。未重设周期保留原 fixed/rolling/自然日历行为；仅改金额不撤销重设，明确修改日调度才恢复新配置。
4. ledger 回填前后、请求明细被保留清理后与数据库重新打开，结果一致；重设前 pending 稍后完成不回流。重新打开测试是持久化证据，不能替代前述同一运行实例的实时生效证据。
5. v54 升级和全新数据库可读写标记，provider 删除清理标记，未重设供应商保持原行为；非法 period/不存在 provider 不产生部分写入。
6. 固定同一评估时间，依次重设四个周期并比较完整快照：每次仅选中周期的起点、终点、累计与标记改变，其余周期（包括已有手动锚点的周期）及总累计不变。专门覆盖“先重设月、再重设周”，断言月窗口、月用量和月限额拦截结果保留；每个周期的金额值始终不变。以后新增请求正常计入各自窗口，不能因重设另一周期而免除其消费。

云端复用同一 PR 自动 CI，不额外手动 dispatch。同一 head 的必要检查：contracts、frontend、rust、observer-macos 成功，最终 `ci-gate`、`pr-title` 成功。前端沿用 `pnpm lint`、现有插件检查、`pnpm test:unit:coverage`（包含 src/e2e）和 `pnpm build`；Rust 沿用云端 fmt/lock/binding canonicalization、`cargo clippy --workspace --all-targets --locked -- -D warnings`、`cargo test --workspace --locked -- --test-threads=1` 及现有工作流其他必要步骤。继承既有自动扫描，不增加安全专项调查。

生成漂移只能使用对应完整 SHA、run attempt 的小体积 `cloud-native-fixes` patch 纠正，不在本机运行生成器。main 可在审批后的实现阶段推送任务分支并开 PR获取此必要反馈；执行角色本身不越权推送。记录最终 head、实际测试 merge SHA、run/attempt 与结果；真实修复的新 head 重新验证。不能以 mock UI 测试冒称桌面应用人工实测。

## 执行、交付与清理

以下动作已获用户明确批准，按实际阶段推进并记录事实：

1. main 从再次确认的远端 main 建立 sibling worktree `../provider-limit-reset` 和任务分支 `feat/provider-limit-reset`，记录实际 baseline；生成 `.gkd/execution.md`，以 `gkd_execute`、`fork_turns=none` 交接唯一 writer，允许范围内实现与中文本地提交。
2. writer 停止后 `gkd_accept` 独立审查代码和已有证据；main 处理 findings，记录 `.gkd/review.md`。允许先审代码并标注最终 CI 待定；范围内问题修复沿用批准。
3. main 按权限安排必要云端生成反馈；独立 CI 等待使用 `gkd-ci-monitor`。最终收尾使用一个 `gkd_closeout`，将交付前已知计划、计划变更、执行、审查与进展材料归档到 `.gkd/archive/provider-limit-reset/2026-09-10-r3/`，与实现合批推送最终待验证 head。
4. 拟批准交付为: 中文提交、推送本任务分支、创建/更新一个面向 main 的 PR、等待 required 和上述 expected CI、通过后按完整 head 正常合并，并等待实际 main merge SHA 的自动主 CI。不绕过保护。版本号、发布标签、Release、打包和安装不在本方案范围。
5. 归档保存在合并后仍可访问的位置；最终 CI、merge SHA 和清理事实按真实时点补记，不为回填自身结果递归制造提交。本地主树其他任务历史与脏文件保留，不推送主树独有提交。
6. 默认仅清理本任务工作树、本地/远端任务分支和已归档的本任务活动记录；须 writer 已停止、必要验证交付完成、成果实际合并且无新增待保留内容。受阻时保留成果和材料并报告未完成项。

终点为功能实现、必要云端验证、独立审查、正常合并及本任务归档清理完成。计划保存和角色启动均不等于功能已完成。

## 消融审查

- 只增加四个分周期按钮和一个已有模式的确认流程；不做首页快捷入口、批量操作、日期选择器或累计重设。
- 一个 period 枚举、一个命令和一个至多四行/供应商的运行状态表解决四种周期，避免四套持久化/接口逻辑；ID 高水位用于真实存在的同秒和异步费用更新，不新增任务调度或网关暂停机制。
- 仅共享显示与网关必须一致的周期计算；新增结束日期字段解决真实的月窗口显示错误，不让前后端各自推导日期，不通过重启或全局配置刷新实现生效。
- 复用现有查询、索引、组件和 CI；不删除账本、不改历史费用、不引入缓存总计，不扩展为配额管理框架。
