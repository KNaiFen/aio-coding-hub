# 供应商周期限额手动重设执行交接

- Execution revision: e2；对应获批 PLAN r3，2026-09-10。e1 已实施并独立审查，当前仅修复下节明确范围；其余正文保留既定行为与限制。
- 唯一执行目录: `<task-worktree>`。
- 分支: `feat/provider-limit-reset`；baseline: `25b4203165e709f9c9b0d8f18c0290b12c296b4a`。
- 主工作树: `<main-worktree>`，仅 main 管理；禁止执行角色写入。
- 用户已批准: “可以，开始吧”。本角色可实施、维护本 worktree `.gkd/progress.md`、完成消融审查并进行简短中文本地提交；禁止推送、开 PR、合并、发布、归档或清理工作树，由 main/closeout 后续处理。
- 本角色是范围内唯一 writer，但不独占代码库；保护所有他人改动，遇到变化按实际内容适配，不回退。禁止嵌套子代理。读取本 execution 和适用规则后按本文执行，无需读主树 PLAN 或旧归档。完成本地实施与记录后停止，返回 head 和真实未验证项。

## e2 当前现场与返工

当前起始 head `3729f71c5235ac3108066afcc6ad66c0f8a9f0c2`：e1 实现 bf989bb3，main 应用云端生成补丁 724c722f，main 最小 js-yaml patch 版本升级 3729f71c。后两者属于已完成工作，不撤销或重复处理。PR #196 的上次远端 head 仍 bf989bb3；你只本地提交，main 合批推送。

e1 独立审查确认以下两项 P2 和一个测试缺口，main 已核对来源。本轮完成它们，禁止追加无关重构：

1. `src/query/providers.ts::useProviderUpsertMutation` 的 onSuccess 不会刷新 providerLimitUsageKeys，生产 staleTime=5分钟。重设日窗口→改变日调度并保存→立即重开，后端锚点已失效但 UI 可显示旧值。允许新增该文件对应成功回调的最小限额 invalidation，并扩展 `src/query/__tests__/providers.test.tsx` 或该 mutation 已有实际对应 tests；用生产 staleTime/真实 query adapter 验证保存前 fresh cache 在保存后失效并读取新日窗口。不要新建平行 save 流程。
2. `src-tauri/src/domain/provider_limit_usage.rs::list_v1` 原 e1 664行 fixed daily end 使用 local_window_end(ts_daily,"+1 day")，DST缺失时刻转换后的时间偏移会带入次日。网关 compute_daily_fixed_bounds 按配置原 reset_time 算 next，二者不一致。按配置算真实 fixed daily start/end；可以复用/共享现有两处已经等价的 fixed daily 边界计算以消除本次重复，但不要扩散改无关窗口。补 America/New_York 春季 02:30 固定每日在 DST 当天与次日显示/用量/网关一致性；现有测试子进程模式可复用，禁止实际运行测试。
3. 当前网关四周期矩阵每次新 DB；计划要求同一 provider 连续重设四周期并完整对比其他已存在锚点。本轮补一条真实 SQLite 同一连接、同一固定时间、同一 provider 的四周期连续 reset，逐步断言只所选起止/用量/标记变化、其余各周期包括已经reset的标记保持、总累计/金额不变。保留“先月后周”与原矩阵，勿降低断言。

已有云端事实：run 34479239040/attempt1，测试 merge 30fb8e082fc487256773117191fac5145c857913，PR head bf989bb33cbba2fd73547b75778cbe546de2d96a。contracts/observer成功；Rust成功运行fmt/export然后因drift中断，clippy/tests未跑。main 已下载并应用此run小patch artifact10153129357，src/generated/bindings.ts 现在完整。Frontend由于js-yaml4.3.1依赖审计中止，main按npm官方4.3.2精确integrity和相同argparse依赖更新override/lock，未安装，待下一云端验证；你不改依赖。

保留后文环境、权限及功能要求。先按已格式化源码阅读，再用 apply_patch 局部修改；不要运行本地rustfmt、编译、测试或npm/pnpm。测试与可能格式漂移由后续云端负责。更新 progress 追加 e2 实际变更/检查/未验证项，用中文本地提交，完毕停止。不要改 execution 或主树 review。

## 已确定用户行为

在供应商配置页面的原有限额设置中，5 小时、每日、每周、每月旁各加独立刷新图标。只在编辑已保存供应商时出现；create/duplicate 未保存供应商没有动作；累计卡片不加。使用既有样式、lucide RefreshCw、tooltip、accessible name 和固定尺寸。

点击确认后只做所选周期的两件事：周期内已记录累计清零、整个周期从当前时刻 T 重新起算。不是清除历史，也不是保留原自然周期终点。用户示例：原周窗口 9/1–9/8，在 9/7 重设后立刻变为 9/7–9/14，用量为 0；后续为 9/14–9/21。四个周期的起点、终点、累计、重设分界全部独立；重设周不改变月/日/5h，用量之后正常分别计入各窗口。总累计及总限额、金额配置、历史请求/费用保持原样，其他限额仍超限时继续拦截。

窗口规则:

- 5h、日、周从 T 起分别按 5h、24h、168h 固定长度持续递推，半开区间 `[start,end)`。过期后从原 T 算出当前期，不在下一请求到来时才开始新周期。
- 月以本机时区同日同时刻的一个日历月递推，比如 9/7–10/7–11/7。目标月没有该日则取月末；从原锚点推导每期以防 1/31→2/28 后漂移到 28 日。夏令时重复时刻取较早者、缺失时刻按时差顺延。复用已有 SQLite 日期能力/chrono 依赖，禁止新增依赖或手写通用日历库。
- 未手动重设的周期保持现有 fixed/rolling/自然周期行为。
- 日限额无论原 fixed/rolling，手动重设后用锚定完整 24h 窗口；原配置值保留。用户以后显式修改并保存 daily_reset_mode/time 时结束日锚点覆盖并恢复新配置，但 cutoff 保留；仅改金额或保存未改的表单不撤销锚点。模式/时间的比较使用后端已有数据库值，非前端 dirty 猜测。

即时生效:

- DB 事务提交后任何新开始的运行中网关限额评估立即使用新状态，不需要保存表单、刷新配置、重新启用供应商、重启网关或应用。
- UI 成功后立即刷新用量与真实起止，等待相关 query 更新；已挂载其他限额视图通过 invalidation 更新，不依赖轮询或重新开页面。重启只是额外持久化保证。
- 独立即时操作不提交/清空未保存表单。确认框标明供应商/周期和“当前周期用量清零，周期从现在重新开始”，历史费用保留。取消不调用，确认期间禁止重复、保存、关闭编辑器。失败真实显示并可重试；区分重设写入失败和已经提交但读数刷新失败。
- 未配置金额的周期也可重设，以便稍后配置。金额 0 保留现有语义。OAuth 上游配额和熔断状态不变。

## 技术实施

### 存储

新增 `provider_limit_resets` 表，主键 `(provider_id,period)`，period 为仅含 `5h,daily,weekly,monthly` 的受约束枚举，无 total。记录 nullable reset_at（Unix 秒）和 request_log_id_cutoff，provider 级联删除。新增 schema v54→v55 迁移，接入 mod/ensure/迁移测试；空表等价原行为，不做历史遍历。复制/分享/配置导入不复制该运行状态，不扩展 ProviderSummary 配置类型。

reset 在单一 IMMEDIATE 事务中验证 provider、读取 request_logs AUTOINCREMENT 已分配高水位（无日志为 0）、只 upsert 选中行。只重设 5h 时同步 providers.window_5h_start_ts。不能只取 ledger MAX(id)，会受回填/明细保留影响。日调度以后显式改变时，同事务将 daily 标记 reset_at 置空，保留 cutoff。

事件计量需要 `id > period.cutoff` 且创建时间落入该周期实际窗口。总累计不受 cutoff 或手动窗口影响，禁止在所有聚合共用 WHERE 上过滤历史。重设前已落库 pending 即使随后费用补齐也不能回流；重设后落库按新时间窗口计。开始时间早于新起点的在途请求不会因晚落库进入新窗口；同秒且未落库的在途请求可能计入，禁止为此新增排空/暂停机制。

### 同步显示与限额判断

- 显示 `provider_limit_usage::list_v1` 仍批量处理，每个 provider 对应独立起止/cutoff，周/月不再共用全局自然窗口参数。源为 usage_events；累计仍聚合完整历史。保留批量 SQL，不按 provider 逐个聚合。
- 在现有 domain 模块中实现小型共用手动窗口计算，供显示与网关使用。只抽象实际重复的日期规则。5h 手动锚点过期不能通过 MIN(created_at) 重新拾回旧窗口。
- 网关 `evaluate_provider_limits` 每次读取重设标记；现有 ledger/request_logs 索引查询返回稳定 id，按周期决定是否累加。保留 ledger 回填前后切换与 NULL final_provider_id 的局部兼容路径，禁止热路径全面改用复杂 usage_events 视图。
- 锚定周期达到限额后的 next_available 直接使用当前真实 end。原 rolling daily 仅无锚点时使用滚动 buckets，且仍应用其 retained cutoff。不同周期的 cutoff 不能互用。
- `ProviderLimitUsageRow` 保留四个 start 字段、新增四个 end 字段和日窗口手动锚点有效的布尔信息。前端 service 归一化后保留这些字段；首页与编辑器从后端实际边界显示，不再用 start 盲算下月 1 日。未手动月窗口 end 保持真实下月初。
- `HomeProviderLimitPanel` 移除旧 getWindowEndTs 推断并使用真实 end；保持现有标签/显示惯例，日锚点时不能标成滚动。编辑器四张限额卡显示紧凑周期和用量，有真实起止时不再用“自然周/自然月”说明代替。

### IPC/UI

新增 `commands/provider_limit_usage.rs::provider_limit_reset(provider_id,period) -> Result<(),String>`，注册 commands/registry.rs，Specta 导出 TypeScript，复用 generated IPC service/query mutation。后端完成事务才返回，前端完成当前 query 刷新后展示完成状态；不能用失败时静默默认值或乐观清零掩盖读数错误。

在 LimitCard 添加局部可选 action/当前窗口数据，LimitsSection 接入 edit ID、原查询和四个动作，ProviderEditorDialog/useProviderEditorForm 联动 pending 即可。复用 ConfirmDialog/Tooltip/toast，不加依赖、首页操作入口、批量 reset、日期选择器或通用配额框架。

## 允许文件范围

- `src/pages/providers/{LimitCard,LimitsSection,ProviderEditorDialog}.tsx`、`useProviderEditorForm.ts`。
- `src/services/providers/providerLimitUsage.ts`、`src/query/providerLimitUsage.ts`；`src/components/home/HomeProviderLimitPanel.tsx`。
- `src-tauri/src/domain/provider_limit_usage.rs`、`src-tauri/src/commands/provider_limit_usage.rs`、`commands/registry.rs`。
- `src-tauri/src/infra/db/migrations/{mod,ensure,tests}.rs` 和新增 `v54_to_v55.rs`。
- `src-tauri/src/gateway/proxy/handler/failover_loop/prepare/provider_limits.rs`。
- 必要联动: `src-tauri/src/domain/providers/queries.rs` 日调度显式变更时停用锚点、必要 provider 删除清理及已有对应测试；`src/components/home/HomeOverviewPanel.tsx` preview fixtures；ProviderLimitUsageRow 直接构造的 tests/mock；云端生成 `src/generated/bindings.ts`。
- 必要行为测试: `src/pages/providers/__tests__/ProviderEditorDialog.test.tsx`、`src/query/__tests__/providerLimitUsage.test.tsx`、`src/components/home/__tests__/HomeProviderLimitPanel.test.tsx`、`src/services/providers/__tests__/providerLimitUsage.service.test.ts`，上述 Rust 模块现有 tests。相同职责文件后缀/拆分差异在 progress 说明。
- 文档: README 使用流程简短增补，`.trellis/spec/aio-coding-hub/cross-layer/request-log-usage-ledger-pagination-contract.md` 补行为/历史保留/分界；本 worktree `.gkd/progress.md`。
- 不修改 CI、依赖、全局 skills、其他 GKD 记录、版本号。需要 material 越界时先返回 main，不能自增范围。

## 验证与环境

本机 Darwin arm64/macOS 26.5.1、Mac16,12、16 GiB、约245GB磁盘，低于512GB。批准的本地操作只有阅读、git diff --check、Git/gh 只读与提交。禁止任何包 scripts、安装、编译、测试运行、格式化/绑定生成、本地 dev server；不绕 guard，不用本地测试代替云端。编写测试代码允许。

必要行为测试（在已有测试集中覆盖，不为小 helper 镜像加用例）：

1. 四按钮正确 period、create/累计无动作、取消/处理中/失败/成功刷新/未保存表单保留；首页月真实 end 显示。
2. 同一运行中的 gateway/DB、同一配置对象，重设前超限，事务提交后立即新起止/选中用量0/限额结果；不重启、不重开 DB、不重建 gateway。之后同秒新请求正常累积。每种周期及原 daily fixed/rolling 均覆盖。
3. 用户周示例 9/7→9/14，9/8 原边界不能提前清零；9/14→9/21 递推。月份同日、月末、闰年、DST，跨多个空闲周期和新 end 半开边界。下一可用时间等于锚定窗口 end。
4. 固定同一时间逐周期 reset 对比完整快照，只有选中周期起止/累计/标记变化。特别先月后周保证月周期、月用量、月超限拦截均保留；总累计和金额不变。不同 provider 相互独立。
5. 回填前后、已清请求明细、pending 终态更新、重开 DB 持久化。后者不能代替实时验证。迁移/新库/provider删除/非法输入原子性，未重设既有行为保持。日调度显式改与只改金额区别。

云端将复用一个面向 main 的 PR 自动 CI，PUBLIC 仓库标准 GitHub runners，禁止自行 dispatch。本任务预期 contracts/frontend/rust/observer-macos，required ci-gate/pr-title。已有 workflow 负责 pnpm lint/plugin checks/test:unit:coverage/build，Rust fmt/lock/binding canonicalization、clippy --workspace --all-targets --locked -- -D warnings、cargo test --workspace --locked -- --test-threads=1 及其他既有检查。main/closeout 推送并等结果。

生成 binding 和 rustfmt 仅在云端；不得本地运行或手工假冒生成。首轮实现可以使用预期生成符号而暂缺 binding，明确报告需要对应 SHA/attempt 的小体积 cloud-native-fixes patch；main 将安排推送取证和范围内修复，不以此中止其余实现。不得从未知 SHA 应用生成差异。

## 完成与返回

在 progress 写清判断、实际变更、消融审查、执行过的轻量命令及结果，列明云端尚未验证和需生成补丁。只提交本任务实现/测试/文档（progress 可保持未提交交收尾），简短中文提交。不提交 execution、不生成归档。提交后停止返回完整 head、差异摘要、现有测试覆盖、未验证/阻塞项。不得声称通过未执行的测试，不自行开 PR/推送/合并。
