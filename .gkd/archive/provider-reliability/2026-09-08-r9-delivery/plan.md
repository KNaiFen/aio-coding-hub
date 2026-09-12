> 归档快照：2026-09-08，来源 主工作树 .gkd/plan.md。原作者决定和各轮观察时点保留；本轮后续交付事实见 [summary.md](summary.md)。个人路径与会话角色句柄已改为逻辑标识。

# 供应商真实测试、停用路由编辑与 TUI 问题

> 本地收尾记录（2026-09-07）：完整脱敏快照与待续事项已保存到 [归档摘要](../2026-09-07-r8/summary.md)。原执行/复查角色已停止；下方正文按原作者与观察时点保留，不构成新施工交接。
> 最终已审 HEAD 为 `7ab1b06de258434145e5748559f5739005cecfbd`，SOURCE STATIC PASS；云端 canonicalize、CI、macOS 与新版本现场验证仍未完成。任务分支/worktree 和 18 个本地提交保留；后续外部交付与清理仍按 main 已有许可边界决定。
> 本段由收尾角色添加；原 PLAN、execution、review 决定及 progress 各轮事实不变。必要验证未结束，活动正文继续保留。

- 日期：2026-09-07
- Revision：r9-delivery
- 状态：2026-09-08，r9版本/云端生成及范围内CI返工完成；最终HEAD `845589f8bf72df70022a6c1e7a85132accd4b266`，execution r9-exec11，独立复查与main审查通过，PR #192自动CI run `34141052395`全部必要检查成功。进入获准合并、main候选、0.60.59发布和归档收尾；新版本本机现场验证仍待发布后完成，历史正文保留原观察时点。
- 主工作树：`<主工作树>`
- 执行 worktree：`<任务工作树>`；分支 `fix/provider-reliability`；baseline `a35f2aa4ea469f6e4066582b2e969f1ec44fca2e`；执行交接 `.gkd/execution.md` revision r8-exec7（含原版与范围内返工）；最终源码审查见主工作树 `.gkd/review.md`。

## r9 交付授权与当前路线

- 用户已明确授权“PR。CI，合并，发版”，取代正文中此前未授权外部交付的历史边界。本版不变更三个问题的行为或验收，只推进已实现成果的云端验证和发布。
- 本次目标版本 `0.60.59`，标签 `aio-coding-hub-v0.60.59`；已只读核实最新发布0.60.58、origin/main仍为baseline、本任务尚无PR。main 直接修改 package.json、tauri.conf.json、三个工作区Cargo.toml版本与CHANGELOG，单独中文提交；Cargo.lock仍由云端生成。
- 允许推送任务分支、创建/更新PR、下载对应CI run/attempt的受限cloud-native-fixes补丁到仓库外临时目录、核对并应用格式/锁/bindings差异后本地提交与推送。自动PR检查为ci-gate和pr-title，必须覆盖所选前端/Rust/observer-macos及合同；不手动启动普通PR CI。
- 必要CI失败在原行为范围内持续修复，不删断言、不放宽成功标准或质量门禁；业务实现返工继续gkd_execute与针对性审查，纯生成/版本变更可由main直接核对。验证仍全部云端，本地仅已允许Git/源码与git diff --check，下载日志只读取必要错误片段。
- 最终PR checks通过后，由交付收尾角色使用squash和--match-head-commit绑定已验证head合并。fetch远端真实合并结果，保留本地main独有历史，不reset或把旧任务branch merge到本地main；可使用任务worktree detached远端merge SHA作为已同步发布现场。
- 版本变更合入main自动触发完整CI和签名候选，等待该merge SHA的成功候选与ci-gate。仅当自动candidate因已知机制缺失时可按现有main-only build_release_candidate=true入口补建，不重复常规PR CI。
- 对通过main CI的同一merge SHA创建并推送新标签，等待release工作流发布已验证候选，不本地打包、不覆盖既有标签/Release资产；核对release目标、桌面/TUI/签名/latest.json/SHA256SUMS资产完整性。允许发布必要的简短用户变更说明，不包含调试数据或未完成现场验收的成功断言。
- 已完成的本地收尾不因新授权失败而重开；本轮是新增授权的云端验证与外部交付阶段。只在代码/生成/PR验证完成后启动本阶段一个gkd_closeout，连续完成合并、main CI候选等待、发版、归档事实更新和已有许可的活动记录整理。
- 本阶段允许维护本任务.gkd活动记录与归档后续事实；尚无应用安装、用户配置变更、真实上游自动测试、分支/worktree删除许可，保留这些现场。三个30分钟观察与用户主动真实生成在发布后另行记录，不作为本次外部交付前的新增阻塞，也不能宣称已通过。
- 完成标准：PR实际合并，正确版本/同一SHA的main CI与候选成功，Release published且预期制品齐全，归档记录可续办；任何失败如实修复或记录外部阻塞，不预填成功。

## 需求与成功标准

1. AIO 停留在供应商页面时，TUI 应继续读取供应商信息；关闭主窗口但保留托盘运行后，读取仍正常。请求日志继续正常刷新。
2. 供应商在命名调用方案中被路由停用时，仍可编辑并保存普通供应商配置和普通模型路由；未配置跨供应商规则时同样可以保存。
3. 手动按钮和定时可用性测试必须验证指定供应商的指定模型能真实生成内容，不能因接口可达或收到错误响应就显示可用。

用户已确认关闭窗口后仍在托盘运行，两端使用最新版本。本机已安装 AIO 与 aio-tui 均核实为 `0.60.58`。用户进一步确认问题发生在本机：长时间停留供应商页后出现，切到首页立即恢复，刚进入供应商页不会立即出现。用户曾要求暂缓此问题，先讨论测试按钮和定时测试改为真实测试；本轮再次报告 TUI 显示“供应商状态暂不可用”，随后确认切回 AIO 首页后 TUI 恢复正常。

## 当前现场

- 已执行 `git fetch origin`，远端基线为 `a35f2aa4ea469f6e4066582b2e969f1ec44fca2e`。
- 本地 main 为 `193767510ef647193ce5f16390bc1f663c3dffb0`，工作区初始无未提交修改；与 origin/main 存在历史分叉。
- 已比较文件差异：本次相关 `src/`、`src-tauri/` 产品代码与 origin/main 相同。差异集中于项目规则、文档和云端验证脚本；保留本地历史，不重置或混入任务分支。
- 另有 `gkd-project-rules` worktree，非本任务所有，不修改或清理。
- 活动 `.gkd/plan.md` 原先不存在；不覆盖旧任务记录。

## 问题一：TUI 供应商信息不可用

状态：已捕获故障时的接口响应，已确认直接故障路径。r6 写入后用户再次遇到异常，并实际完成“托盘唤到前台即恢复”的对照；最新结论见 r7 小节。两个只读探索代理已结束；此前 interrupted 代理没有恢复。尚未修改产品代码。

### 已有证据

- TUI 只有在供应商视图请求 `include_providers=true`，请求日志和状态视图不请求该投影。见 `src-tauri/crates/aio-tui/src/client.rs` 的 `snapshot_with_providers` / `fetch_snapshot`，以及 observer contract。
- `src-tauri/src/app/observer/snapshot.rs:315` 单独加载供应商投影；`load_provider_observations` 读取供应商、可用性时间线、限额与 OAuth 缓存。
- `build_snapshot` 在包含供应商时续订共享账户用量 runtime 的 TUI 租约并读取 `display_snapshots`；桌面供应商页使用同一 runtime。没有证据证明此处已经发生锁等待。
- observer 使用独立单连接只读数据库池，TUI HTTP 超时为 3500 ms，数据库投影超时为 1500 ms；这些是现状，不预先决定调大。
- 本机只读 GET 诊断中，请求视图 HTTP 200，耗时 287 ms；供应商视图 HTTP 200、available=true、10 个供应商，耗时 477-903 ms；`all` 范围也返回 10 个供应商。此结果仅证明采样时正常，不能证明故障已修复。
- 诊断仅输出状态、耗时、字段名和数量，没有记录 bearer token、供应商名称、金额或完整响应。
- Computer Use 在读取应用状态及列举应用时均返回配置错误 `invalid type: map, expected a boolean in features`，未能操作供应商页面，也未更改用户配置。
- 本轮错误原文对应 `src-tauri/crates/aio-tui/src/ui.rs:812` 的 `provider_empty_message`：在没有连接陈旧提示、没有待加载状态时，收到 `providers.available=false` 才显示“供应商状态暂不可用”。据此可以把排查重点收窄到后端供应商投影不可用，不能直接等同为供应商模型调用失败。
- `src-tauri/src/app/observer/snapshot.rs:677` 根据 `provider_details_available` 输出 unavailable；此值来自 `load_provider_observations` 的结果，或整个 `load_db_projection` 不可用的默认值。初期候选包括数据库未就绪、共享 blocking 队列等待/查询合计超过 1500 ms、阻塞任务失败，以及供应商行/本地限额/OAuth 快照读取错误；下方 r5 现场采样已将本次直接故障定位为整体投影超时。
- 账户用量 runtime 的 `touch`、`display_snapshots` 与结果更新锁内未发现网络 await；上游请求在锁外执行。账户用量查询失败本身不会将 `provider_details_available` 置 false。当前证据不支持直接断言该 runtime 死锁。
- observer 虽有独立单连接只读数据库池，`observer_snapshot` 仍使用 `src-tauri/src/shared/blocking.rs` 的全局 blocking 并发限制；独立数据库池不能单独排除桌面任务引起的排队。现场已确认 observer DB 通道排队，但没有单独测出全局 blocking 等待占比，不将两者混为同一证据。
- 恢复后本机 `cli=codex&history_limit=20` 的两次只读 GET：不含供应商 HTTP 200、377 ms；含供应商 HTTP 200、490 ms、`providers.available=true`。仅记录状态、耗时和结构字段，没有保存凭据或响应正文。
- 对当日日志作有限模式筛选，未发现 observer 数据库不可用、可用性时间线不可用、阻塞任务 panic/cancel、连接池或数据库锁相关条目。`load_db_projection` 的超时/错误和供应商投影错误目前未保留具体诊断，日志无匹配不能排除这些分支。

### r5 本机故障采样与定位

- 本机运行响应与 descriptor 均为 `0.60.58`；`git diff --stat aio-coding-hub-v0.60.58 HEAD -- src src-tauri` 无差异，调查源码与发布标签的产品代码一致。
- 初轮诊断曾按 Rust snake_case 访问部分 JSON 字段；协议使用 camelCase，早先的 null 不能用作分区失败证据。本轮已按 `recentRequests`、`lastRequest`、`preferredProvider`、`dominantProvider` 等协议字段重新采样，参数与 TUI 一致：`cli=codex&history_limit=50&include_providers=true`。
- 正常阶段（2026-09-07 13:11:37 至 13:16:36，本机时区）：72 次供应商请求全部成功；相邻 12 次不含供应商的请求快照全部成功。未命中缓存时，供应商快照约 490-550 ms，不含供应商约 240-280 ms。较长 HTTP 延迟可包含等待其他投影完成，不能全部算为本次 SQL 时间。
- 故障阶段（13:23:25 至 13:28:25）：49 次供应商请求中 48 次 HTTP 200 且所有数据库分区 unavailable，1 次 HTTP 429；相邻 49 次请求日志快照全部成功。第二故障采样段（13:29:57 至 13:34:59）同样为 49 次供应商请求全部失败或繁忙，相邻 45 次请求日志快照全部成功。
- 失败快照中 `providers`、`recentRequests`、`lastRequest`、`preferredProvider`、`dominantProvider`、`today` 的 `available` 同时为 false，只有内存分区 `activeRequests.available=true`。非缓存失败响应的生成至返回约 1503-1508 ms，紧贴 `DB_SNAPSHOT_TIMEOUT=1500 ms`。不含供应商的快照在故障期间仍完成，其生成至返回约 1130-1280 ms，含排队的 HTTP 总耗时常为 1800-2300 ms。
- 由同进程正常的日志投影、持续贴合 1500 ms 的失败时刻和 unavailable 字段组合，已定位直接故障为供应商快照的整体 DB 投影超时。局部供应商查询错误不能解释所有数据库分区一起失败。
- 超时后 `spawn_blocking` 闭包仍执行，observer 的独立 DB permit 在闭包结束才释放（`snapshot.rs:254`）；后续请求最多等待 1600 ms，超过则 `429 OBS_BUSY`。现场已捕获该 429，确认持续慢查询还会影响后续请求排队。
- 故障期间通过 macOS `proc_pidinfo(PROC_PIDTBSDINFO)` 只读查询进程标志：`suppressed=true`、`darwin_background=false`、`external_background=false`、`resource_throttled=false`；AIO 非最前台应用，调度优先级为 4。`PROC_FLAG_SUPPRESSED=0x800000` 由本机 SDK `mach/task_policy.h` 确认。线程优先级后的 `T` 仅表示分时调度，不能当作抑制证据。
- Apple 官方 App Nap 文档说明后台抑制可降低 CPU 优先级并限制 I/O/定时器，置前应用会退出 App Nap：<https://developer.apple.com/library/archive/documentation/Performance/Conceptual/power_efficiency_guidelines_osx/AppNap.html>。本机 suppressed 状态和进程内查询变慢符合此机制；r6 时前台对照尚未完成，r7 已新增用户实际操作的恢复证据，见下方。
- 只读数据库聚合信息：10 个供应商，5 个配置本地费用限额，无 OAuth 供应商；历史账本约 22.7 万行，backfill complete。独立进程在正常和故障期间都能读取：供应商基础行约 0-1 ms、72 小时时间线约 1-2 ms、过期 5h 窗口解析约 2 ms。排除当前现场因 OAuth 读取、基础行或 5h 过期补查单独拖慢的解释。
- 当前 `load_provider_candidates` 与 `load_provider_observations` 对相同 CLI 重复调用 `provider_limit_usage::list_v1`（`snapshot.rs:355`、`:419`）。单次历史费用汇总在独立只读进程约 226-287 ms；现有查询计划物化 `usage_events` 后再关联，扫描历史账本并建立临时索引，未先按所需供应商缩小。
- 只读 SQL 方案对照：保持相同只读事务与窗口参数，先物化所需供应商的必要费用字段，再关联汇总，现场 5 行金额结果与原查询完全一致，耗时由 241 ms 降到 133 ms。普通 inner join 或单纯窄字段子查询未改善，不能以这些无效改法收尾。该实验使用 Node 自带 SQLite，与 Rust bundled SQLite 版本不同，只证明候选改法与现场金额一致，不能代替发布版本性能验收。
- 桌面供应商卡片账户用量每 5 秒轮询且允许后台运行（`src/query/providers.ts:636`）；命中 runtime 缓存前仍读配置（`commands/providers/account_usage.rs:43`）。本机 7 个账户用量配置有效，定时周期 300 秒。隐藏到托盘保留页面，切首页卸载页面查询；这是可确认的额外压力，未证明它是触发系统抑制或 SQL 减速的唯一原因。网络请求不跨越数据库连接或 runtime 锁持有期。
- 短时系统线程采样无完整用户日志、凭据或请求内容落盘；发布二进制符号被裁剪，无法据匿名地址精确归属 Rust 函数。`launchctl procinfo` 因需要 root 不可用，未提权或改系统配置。Computer Use 仍因同一配置错误不可用，未以其他方式操作 UI。

### r7 前台恢复对照与实施边界

- 2026-09-07 用户补充：TUI 再次获取供应商状态异常，从托盘将 AIO 切到前台后，TUI 立即恢复。按本次对照的操作描述，恢复动作是置前 AIO，没有报告再切首页；该证据来自用户现场操作，本轮未同步采集置前前后的系统标志或接口耗时。
- r6 的“长时间未再复现，最后一个对照未完成”保留为当时事实，现已由本次实际恢复对照补充，不再作为方案待决项。无需用户重复完成同一对照。
- 已确定的直接故障是包含供应商的整体 DB 投影超过 1500 ms。结合此前故障时 suppressed=true、进程内查询变慢及本次唤到前台立即恢复，macOS App Nap/后台抑制是目前最有力的触发因素，足以进入正式修复范围；尚不声称已排除所有唤醒伴随行为或证明唯一系统机制。
- 重复历史费用汇总与 SQL 扫描是已验证的主要耗时来源，页面后台轮询是额外压力。不能再将“供应商页面自身卡住，只能切首页才能恢复”作为根因结论。A-D 保留并与 E 同批实施，同时解决慢查询余量和 TUI 正在使用时被视为闲置应用的问题。

### A. 单次快照复用费用汇总

- 在 `collect_db_projection` 中按本次投影所需范围计算一次本地费用限额数据，候选供应商与供应商详情消费同一份结果。
- 固定 CLI 且包含供应商时，只执行一次该 CLI 的 `provider_limit_usage::list_v1`；不含供应商时只计算候选所需 CLI。`all` 且包含供应商时计算全范围一次，再按供应商 ID 分配给候选与详情，不能漏掉其他 CLI 的供应商。
- 复用范围仅限一个请求，不创建跨请求缓存。候选计算仍覆盖实际路由所需供应商，不能被详情 512 行展示上限截断。
- 错误由原有受影响分区明确呈现；费用查询失败不能被当作“无限额”或零费用成功返回。保持原有限额窗口、金额、候选资格和路由顺序语义。

### B. 优化历史费用 SQL

- 在 `aggregate_costs_for_providers` 保留候选窗口 CTE，先按该批供应商 ID 筛选 `usage_events`，仅物化 `final_provider_id`、`created_at`、`cost_usd_femto` 等汇总必要字段和符合现行统计条件的行，再与候选窗口做 LEFT JOIN。
- 保留 5h、日、周、月、总额的现有计算，尤其不能以截断历史的方式加速总额；保持没有匹配费用的供应商仍返回零值。
- 保留 `usage_events` 对完整回填与兼容日志的视图语义，不直接改读某一张表，不新增数据库迁移或索引。
- 用 Rust 实际 bundled SQLite 验证查询计划及结果等价。已有 Node SQLite 的 241 ms 到 133 ms 对照只作为选型证据，不作为 Rust 性能通过证明。

### C. 隐藏页面暂停展示轮询

- `useProviderAccountUsageQuery` 复用现有 `useDocumentVisibility`，仅在调用方启用、配置有效且文档可见时启动每 5 秒的自动刷新；移除强制后台轮询。
- 文档恢复可见时补一次正常读取，继续使用 runtime 缓存及既有定时调度，不强制向上游重新拉取账户用量。可见但未聚焦时维持现有刷新节奏，不擅自改成焦点控制。
- 切换首页或隐藏到托盘后，桌面展示消费者不再靠轮询续订租约；既有 15 秒租约自然到期。TUI 自己的消费者租约、用户配置的账户用量周期及定时可用性测试独立保持原行为。
- 已在执行的合法账户用量请求可完成；不因隐藏页面取消共享任务或清空结果。

### D. 超时任务收束与受控诊断

- 保持 observer DB 投影 1500 ms、DB permit 等待 1600 ms、TUI 普通快照请求 3500 ms 的现有预算，不增加重试或全局 blocking 并发数。
- 将同一个单调时钟截止时间带入投影闭包，在开始及各主要阶段交界检查。已经过期时不再启动后续独立查询；正在执行的 SQLite 调用返回后才释放 DB permit，不提前释放 permit 导致过期任务与新任务重叠。
- 不加入跨线程强制中断共享连接、专用线程池或常驻监控器。此处只减少超时后无用的后续工作，不能声称能够强行终止正在执行的 SQL。
- 记录 DB permit 等待、blocking 调度等待、主要投影阶段和总体耗时；明确区分 permit 繁忙、整体截止时间、blocking 失败及局部查询错误。只有失败或慢投影输出诊断，正常快照不逐次写日志；慢投影阈值为现有预算的 75%。
- 诊断只保留阶段标识、耗时、CLI 范围、是否包含供应商、数量与错误代码，不记录 SQL 参数、供应商名称、账户数值、凭据或完整响应。
- 整体失败继续返回 unavailable；不把过期缓存、空列表或仍正常的内存分区冒充供应商查询成功。

### E. TUI 使用期间声明 macOS 用户活动

- 在 AIO 的 observer 生命周期内调用 `NSProcessInfo.beginActivity(options:reason:)`，选用 `NSActivityUserInitiatedAllowingIdleSystemSleep`，在用户通过 TUI 使用 AIO 时告知系统这是需要及时完成的用户工作。固定 reason 为 `AIO TUI observation`，不含用户数据。结束时以同一个 token 调用 `endActivity` 并释放所有权。
- Apple 官方依据：<https://developer.apple.com/library/archive/documentation/Performance/Conceptual/power_efficiency_guidelines_osx/PrioritizeWorkAtTheAppLevel.html> 与 <https://developer.apple.com/documentation/foundation/processinfo/activityoptions/userinitiatedallowingidlesystemsleep>。该选项抑制 App Nap 对用户工作的延后，同时允许空闲系统休眠；不用会阻止系统/显示器休眠的额外选项，不全局关闭 App Nap、不修改系统偏好或用户启动方式。
- observer 启动时不自动持有活动。每次通过现有认证和参数校验的 snapshot 请求，在缓存与 DB 排队之前申请或续订进程内共用的 15 秒活动租约；包括所有 CLI、所有 TUI 视图、缓存命中、后续 DB 不可用或繁忙的有效读取。不能等供应商投影成功后才续期，否则故障期间无法唤醒。
- health、未认证、参数不合法、未知路径请求不申请或续期；活动状态不依赖供应商数量、账户用量配置、是否有网络请求或桌面当前页面。多个 TUI 客户端共用一个活动 token，最后一次有效读取决定到期时间，不增加客户端登记协议。
- TUI 手动模型测试通过原有校验与并发准入后，在本次 observer 测试等待期间持有同一活动的工作引用；等待结束、错误、超时或取消时释放引用。最后一次 snapshot 后 15 秒且无此类在途工作时结束活动；仅有测试等待时不超过本版 65 秒端点预算。既有定时测试不因这一用户活动机制新增租约。
- 使用单调时钟与一个 observer 所有的可重置到期等待任务；续期只更新时间并唤醒该任务，不为每次读取创建一个常驻任务。无活动时不定时唤醒。有新读取与到期并发时串行更新状态，旧到期不能释放已续订活动，begin/end 必须成对且同一时刻最多一个有效 token。
- 活动 token 的创建、持有及释放按 Objective-C 类型/线程要求封装在一个小平台模块；复用现有 Tauri 主线程调度在需要时执行原生操作，不通过裸指针或 `unsafe impl Send` 绕过对象约束。计时与活动状态更新不占用 DB 连接，也不依赖全局 blocking 配额。
- `stop_best_effort`、服务器异常结束及启动取消都关闭该控制器、停止到期任务并释放 token；停止后的迟到请求不能重新申请。正常退出完成清理；进程被系统结束时由 OS 回收。Windows/Linux 路径无此原生活动、无新增到期任务。
- 正常 begin/end 仅在状态变化时记录一次脱敏诊断，方便观察租约与系统抑制对应关系；原生调用或主线程派发失败必须有错误代码，不能伪报活动已建立或把失败转换为供应商成功。
- 依赖只增加 macOS 条件下直接使用的 `objc2` 0.6.3、`objc2-foundation` 0.3.2 及所需最小 features，复用当前 Cargo.lock 中已有版本，不引入新的平台运行时或自行手写 Objective-C ABI。Cargo.lock 的直接依赖归属更新由既有云端 canonicalize 核对，不本地安装依赖或运行 Cargo。
- 权衡：TUI 持续读取期间 AIO 可能比原来受 App Nap 抑制时耗电更多，这是及时提供用户数据所需的活动；15 秒空闲释放与 A-C 减少无用查询共同控制影响。不能为节能重新允许活跃 TUI 的查询持续超时。

### 文件与符号范围

- `src-tauri/src/app/observer/snapshot.rs`：`load_db_projection`、`collect_db_projection`、`load_provider_candidates`、`load_provider_observations` 及同文件必要回归。
- `src-tauri/src/app/observer/mod.rs`：DB permit 等待诊断、snapshot/手动测试活动接入、服务启动/停止清理及相关回归；不修改 HTTP 授权、协议版本或刷新策略。
- `src-tauri/src/app/observer/activity.rs`（新增）：observer 专用活动租约、工作引用、到期调度和 macOS 原生 token 的小范围封装；同文件覆盖生命周期回归，不扩展成通用电源管理服务。
- `src-tauri/Cargo.toml`、`src-tauri/Cargo.lock`：仅 macOS 条件依赖与已有锁定版本的直接依赖关系，禁止无关升级。
- `.github/workflows/ci.yml`、`scripts/check-ci-quality-gates.mjs`、`scripts/check-ci-quality-gates.selftest.mjs`：补充下方 macOS 专用回归 job 及其门禁消费，不重构工作流或改发布职责。
- `src-tauri/src/domain/provider_limit_usage.rs`：`aggregate_costs_for_providers` 的 SQL 与现有测试区域；`list_v1` 外部数据语义不变。
- `src/query/providers.ts`、`src/query/__tests__/providers.test.tsx`：账户用量展示查询的可见性与恢复流程。
- `src/components/providers/__tests__/ProviderAccountUsageInline.test.tsx`：仅在 query 层现有测试不足以覆盖隐藏/恢复用户流程时补充。
- `.trellis/spec/aio-coding-hub/cross-layer/local-observer-tui-contract.md`：补充单次费用复用、过期任务、不可用诊断及 macOS 活动租约/释放语义。
- `.trellis/spec/aio-coding-hub/cross-layer/provider-account-usage-query-contract.md`：将桌面租约的心跳条件明确为已挂载且文档可见，保留 TUI 与远端调度的既有语义。
- `src/hooks/useDocumentVisibility.ts`、`src-tauri/src/shared/blocking.rs`、`src-tauri/src/app/provider_account_usage_runtime.rs`、数据库视图迁移为复用/审查依据，当前不计划修改。

### 验收条件

1. 同一固定 CLI 供应商快照只做一次费用汇总；`all`、不含供应商、空列表、展示截断场景仍覆盖正确的供应商范围。费用查询错误仍明确表示对应分区不可用。
2. SQL 回归使用合成账本覆盖完整回填/兼容视图、零费用、窗口边界、被排除记录、失败记录、多 CLI、多批供应商；五类金额与原实现一致。查询计划证明减少宽表物化和无关供应商参与，不能只靠金额单例测试或固定机器毫秒阈值声称优化成功。
3. CI 回归证明隐藏后不再自动发起桌面账户用量查询，恢复可见后及时读取；TUI 租约仍维持其账户用量刷新，定时可用性测试不受页面可见性影响。
4. 可控延迟回归证明过期闭包不再启动后续阶段、permit 随实际工作结束释放、后续正常请求可恢复；错误类别和阶段可诊断，不暴露真实用户数据。
5. 修改后的应用在本机相近数据规模下，分别执行供应商页可见、保持供应商页但 AIO 位于后台、关闭到托盘三个至少 30 分钟的观察段。正常采样包含供应商且 `history_limit=50`，使用 TUI 正常轮询节奏，不额外提高请求频率制造压力。
6. 上述观察段的有效采样中，供应商及请求日志分区持续可用，没有整体 DB 超时或 `OBS_BUSY`；每段至少切换一次 TUI 供应商/日志视图验证刷新。供应商列表与账户用量状态分别记录，账户用量上游错误不得误算成整个供应商列表不可用。
7. 记录非缓存投影耗时及后台状态，验证 1500 ms 内完成；不能拿缓存 HTTP 耗时替代 DB 耗时。复现出同类错误则该项不通过，依据新增阶段诊断继续处理；窗口观察通过仅代表指定时长和环境通过，不承诺已排除所有系统调度影响。
8. 本机运行验收依赖可运行的新版本及用户正常打开/隐藏页面，不在本地构建或安装候选应用；交付时分别报告 CI 验证与本机观察完成情况。尚未具备新版本时记为待验证，不以旧版本“长时间未复现”替代。
9. 可控时间回归覆盖首次 snapshot 申请、缓存/所有视图续期、多个客户端合并、15 秒到期释放、到期与续期竞态、手动测试超过 15 秒的工作引用、取消/超时释放、服务停止/异常退出与迟到请求；认证或参数失败不能续期。各流程断言 begin/end 次数，证明没有重复 token 或泄漏。
10. macOS 云端回归实际编译原生绑定并验证 begin/end 可调用；本机三个观察段还需确认活动存在时不会持续处于导致快照超时的后台抑制状态。TUI 全部退出且没有在途测试后，15 秒租约到期即撤销活动；系统可以正常空闲休眠。未看到活动清理证据不能仅凭前台可用认定通过。

## 问题二：停用路由时保存失败

### 已确认的原因

- `src/pages/providers/useProviderEditorForm.ts:222` 将无配置的跨供应商规则初始化为 `{ enabled: false, rules: [] }`。
- `adoptCrossRoutingView` 在来源成员存在时，即使成员停用，也把后端 `cross_policy=null` 转成该对象，并把它视为未修改的初始草稿。
- `saveRoutingPolicies` 在普通保存时仍提交该对象。
- `src-tauri/src/domain/sort_modes.rs:1178` 允许停用成员保存普通规则，但要求跨供应商规则保持原值。后端 `None` 与传入空对象不相等，因此拒绝保存。
- `runProviderEditorSave` 先保存供应商信息，再保存路由策略，因此用户可能看到保存失败但部分普通字段已经写入。此次应消除上述错误触发，不扩大为重构整个保存事务。

### 最小修法

- 跨供应商草稿未修改时，保存使用当前后端视图的原始 `cross_policy`，保留 `null`、空对象或已有规则的真实表示；仅在草稿确实修改时提交编辑对象。
- 编辑器仍可为启用成员显示空规则表单，不把表单显示默认值等同于持久化修改。
- 保留后端 UUID/revision 检查、事务回滚、停用来源成员不能改跨供应商策略，以及目标成员资格校验。
- 复核同一成员停用状态更新、重新拉取和切换方案时的草稿基线；如需调整，限定在相同表单状态范围内。

### 拟修改范围

- `src/pages/providers/useProviderEditorForm.ts`：`saveRoutingPolicies` 及其依赖，必要时调整相关草稿基线。
- `src/pages/providers/__tests__/ProviderRoutingEditor.test.tsx`：在已有集成测试中覆盖用户保存流程。
- `.trellis/spec/aio-coding-hub/cross-layer/configured-model-routing-contract.md`：补充停用成员普通保存与未变跨供应商配置的语义。
- `src-tauri/src/domain/sort_modes.rs` 为后端证据与审查范围；当前不需要放宽该处限制。

### 验收条件

- 来源供应商全局启用、当前方案成员停用、跨供应商配置为 null：修改普通字段并保存成功，提交的跨供应商值仍为 null。
- 停用成员原有非空或显式空规则：普通保存保留原值，不创建、清空或覆盖跨供应商规则。
- 停用后仍保留的跨供应商脏草稿不会被静默丢弃或当成保存成功。
- Default、非方案成员和正常启用成员保存继续正常；启用成员修改跨供应商规则仍按实际草稿提交。
- 旧 revision 仍触发并发更新错误，不覆盖其他编辑者的更新。

## 问题三：手动与定时真实测试

- 用户明确确认：不限定接口名称，只要是真实有效的模型请求；只测试供应商本身，不能显示可用而实际没有成功调用模型。r8 用户进一步指定 100 token 预算，能输出有意义内容即可视为可用。这是行为要求确认，不是施工批准。

### 现状证据

- 桌面按钮、TUI 测试和定时任务通过 `ProviderAvailabilityProbeRuntimeState` 共用 `provider_availability::test_provider_availability`。
- 现状是向上游发送极小模型请求，但 `is_probe_available_status` 使用 `status < 500 && !looks_like_auth_failure`；模型错误、限流等部分 400/404/429 也会被计为成功。
- 响应体读取失败被替换为空内容，成功判定不验证模型输出或完整完成。直连 Codex 当前使用 `/v1/chat/completions`，不一定覆盖实际 Codex Responses 请求路径。
- 所有现有普通探测请求输出上限为 1 token，转译探测 `gateway::build_translated_bridge_probe` 也使用该上限。
- 当前只有 Codex 的 `availability_test_model` 覆盖在表单和持久化中生效；其他 CLI 使用各自现有默认值/配置。真实测试必须明确本次实际测试的模型，不能把其他模型的成功当作指定模型的成功。
- 探测结果既写入可用性时间线，也被现有半开熔断恢复流程读取；收紧成功条件必须同步核验这些下游，不能只修改按钮提示。

### 已对齐的行为与验收标准

- 测试固定一个供应商及测试模型，绕过当前路由方案、普通模型路由和跨供应商规则，不允许换供应商或自动改用另一个模型来取得成功。
- 使用该供应商认证方式与协议支持的真实模型生成请求，复用现有传输/转译能力；接口选择由实现负责，不新增让用户选择接口的模式。
- 对转译供应商，其显式绑定的源连接与必要协议转换属于该供应商配置；不能通过本机网关的动态供应商选择取得替代成功。没有固定源供应商的动态网关桥接不能宣称已验证指定供应商模型。
- 发起简短文本生成，支持输出上限的路径统一限制为 100 token，不新增自动重试或自动提高预算；不以输出长度或回答是否逐字匹配提示衡量能力。
- 成功必须同时满足：HTTP 2xx、符合该协议的响应结构、存在有效的非空白模型回答文本、协议明确以正常完成或仅达到输出 token 上限结束，且无其他错误。只有握手、响应头、心跳、usage、reasoning 或空对象不能证明生成成功。
- 非流式必须完整读取并解析响应；流式必须收到回答文本及对应协议的结束证据。达到模型输出 token 上限与传输损坏分别判断：前者已有回答可通过；读取失败、64 KiB 响应截断、提前 EOF、错误事件、非 token 上限原因的 incomplete 仍失败。不在收到首个文字后提前报成功。
- 400/401/403/404/429/5xx、200 错误对象、200 HTML、空响应、模型不存在、限流、配额不足、连接失败和超时均不计为成功。错误要有可理解的原因，不能静默替换为空响应。
- 结果只代表本次指定供应商与该模型的调用结果，不泛化为该供应商全部模型可用。保留本次测试模型信息以便用户核对。
- 桌面按钮、TUI 手动测试、定时测试与既有恢复探测共用严格判定；失败不得写成成功时间线或成功恢复证据。
- 定时开关、间隔与同一供应商并发合并规则继续沿用；收紧判定不额外自动启用测试或路由。

### 请求、模型与预算

- 输入为一次固定短文本请求 `Reply with the single word OK.`，不包含用户会话、工具或附件；成功不要求逐字等于 OK，要求符合下方真实文本及完成条件。一次测试最多发送一个模型生成请求，配置/认证失败时不发送；沿用必要的 OAuth 凭据刷新，不借失败自动换接口、模型或供应商。
- Codex 直连使用 `/v1/responses`；模型来源保持供应商 `availability_test_model` > 全局 `codex_provider_test_model` > 现有代码默认值。普通 API Key 路径请求非流式，Codex OAuth 路径复用现有 ChatGPT 后端兼容规则，包括 `/responses` 路径、账户头、`store=false` 和流式要求。
- Claude 直连使用 `/v1/messages`，测试模型保持当前 `claude-sonnet-4-6`；Gemini API Key 直连保持当前 `gemini-2.0-flash:generateContent`；Grok 使用现有 effective preferences 的 `model_id` 和 API backend。此次不新建其他 CLI 的模型选择设置，也不查询模型目录后自动挑选另一个模型，结果明确显示实际测试模型。
- 各 OAuth 路径使用项目现有适配器的传输认证和必要请求/响应兼容。Claude 使用 OAuth 适配头；Gemini 使用现有 Code Assist 项目包装/解包；Grok 保留 OAuth 头与配置的 backend。不能仅把 OAuth token 当普通 API Key 拼入请求。
- 显式桥接沿用绑定源连接与已有 model mapping，向该固定源发请求并依据实际目标协议验证结果；这类静态映射属于所选供应商配置。记录输入模型与最终发出的模型，不能把映射后的成功显示成输入模型未经映射的成功。无固定源的动态桥接返回明确失败，不访问本机动态路由获取替代成功。
- 普通支持 token 上限的协议请求设为 100 输出 token，使用该协议既有的输出预算字段，并确保静态桥接后的最终请求保留同一预算；这不包含输入 token，协议若将 reasoning 计入输出预算则沿用其计数语义。不主动打开额外思考模式，不为获得成功自动提高预算。100 token 全用于思考而没有可见回答时，返回“未获得有效回答”，不称为永久不可用。
- Codex OAuth 兼容层会移除输出上限字段，该路径仍不能承诺服务端 100 token 硬上限；沿用单请求、短输入、45 秒 HTTP 总时长与 64 KiB 响应上限。不得用字符数估算 token 或声称断开连接必然停止服务端生成。到达本地时间/响应大小边界仍判失败，此已知兼容限制需向用户明确，不伪报所有路径均受 100 token 限制。
- 连接超时保持 8 秒；HTTP 总时长改为 45 秒，覆盖响应体读取及流式完成；共享实际测试工作从加载配置到得到结果最多 60 秒，超时后释放本次协调状态。observer 测试端点等待预算设 65 秒，TUI 使用协议预算加既有 1 秒宽限。普通快照的 3500 ms 超时不变。
- 64 KiB 响应上限沿用现有值，边界必须正确区分恰好读完与仍有未读字节；截断或读取失败不能进入成功分支。耗时记录到最终完成或失败，不再仅测量响应头到达。
- 定时测试会产生真实模型用量，单次消耗可能高于旧 1 token 探测；用户原有定时开关、频率和并发合并保持不变，不批量自动触发测试。

### 回答与结束判定

| 实际协议 | 需要的生成与结束证据 |
| --- | --- |
| Responses JSON | 合法 response，非空白 `output` 回答文本，`status=completed` 或明确因输出 token 上限而 incomplete，且无其他错误；不能只读顶层任意 text 字段 |
| Responses SSE | 合法事件中的非空白模型回答，并收到与本次响应对应的 completed 终态，或明确因输出 token 上限而 incomplete 的终态；其他错误、failed、未知 incomplete 原因或终态前 EOF 均失败 |
| Chat Completions JSON | 同一 choice 的非空白 assistant 回答及 `finish_reason=stop` 或 `length`；content_filter、工具调用或缺失结束原因仍失败 |
| Chat Completions SSE | 同一 choice 的非空白回答增量、`finish_reason=stop` 或 `length` 与终止标记齐全；只有 `[DONE]` 不够 |
| Anthropic Messages | 非空白 text block，合法 message 与正常 stop reason 或 `max_tokens`；流式还需相应 `message_stop`，只有 thinking/tool block 不算成功 |
| Gemini GenerateContent | 同一 candidate 的非空白模型回答及 `finishReason=STOP` 或 `MAX_TOKENS`；OAuth 先按现有 Code Assist 协议解包；thought-only、block 或错误结果失败 |

- 所有分支首先要求 HTTP 2xx，响应结构和事件类型必须匹配请求协议。非流式完整读取后解析；流式复用已有 SSE 分帧，在上述可接受的协议终态结束，不将单个内容增量当作整次成功。有效响应中的 usage 可缺省，不以是否收费字段判断成功。
- “有意义内容”在本次可用性测试中落实为正确协议的模型回答字段里存在非空白文本；不把思考、工具参数、错误消息当回答，不要求长回答或逐字等于 OK，不增加另一轮模型评分、关键词猜测或复杂语义评估。
- 不使用宽松转译器自动填入的默认 stop reason 来证明上游结束；在原始目标响应上检查结束事实与 token 上限原因。优先复用现有协议类型、SSE 分帧和错误处理，只增加本次测试必要的判定函数，不新建通用探测框架。
- 失败至少区分 HTTP/认证、模型或配额错误、无文本、结构不合法、未完成、读取失败、响应超限和超时；继续用现有 `error` 与受限脱敏预览展示，不把原始全文写入日志。

### 结果与既有消费者

- `ProviderAvailabilityResult` 和 observer 对应 DTO 增加可选 `requested_model`、`tested_model`，分别表示本次选择的输入模型与最终上游请求模型；observer JSON 延续 camelCase。请求准备前失败可为空，准备后失败也保留已确定的模型。
- 旧响应缺字段时仍可读取，新字段不提高 observer 协议版本；TUI/桌面成功与失败结果都显示实际测试模型，发生静态映射时同时显示输入与目标。旧服务端未提供模型时明确缺失，不猜测。
- 上游返回的模型标识若存在可用于核对，但别名可能返回具体版本，不以字符串不相等直接判失败；AIO 保证自己不替换请求中的模型，不能保证第三方内部实际部署的模型身份。
- 保持现有可用性时间线的存储形态，本次不回填历史成功记录、不增加数据库迁移。新测试从统一严格结果写入时间线并供半开恢复消费，不另建一个只改变按钮外观的成功判定。
- 配置在测试中途变化时，既有 generation/recovery epoch 机制仍阻止过期结果恢复新配置；失败不产生成功恢复证据。只测试供应商本身指不经过路由选择，既有时间线和恢复消费仍保留。

### 文件与符号范围

- `src-tauri/src/domain/provider_availability.rs`：请求构造、认证传输调用、有限读取、协议完成判定、结果字段及同文件 mock 上游回归。
- `src-tauri/src/gateway.rs`、`src-tauri/src/gateway/proxy/mod.rs`：为固定供应商测试暴露必要的既有适配/解析能力；修改 `build_translated_bridge_probe` 的请求预算与结果元数据。不得调用 failover 或动态供应商选择。
- `src-tauri/src/gateway/proxy/protocol_bridge/`、`src-tauri/src/gateway/proxy/gemini_oauth.rs`、`src-tauri/src/gateway/proxy/handler/failover_loop/prepare/codex_chatgpt.rs`：仅复用所需协议包装、SSE、完成信息与可见性调整；如需移动公共纯函数，现有网关调用一并复用，保持普通转发语义不变。
- `src-tauri/src/app/provider_availability_probe_runtime.rs`、`src-tauri/src/commands/provider_availability.rs`：统一 60 秒工作预算、同一结果的时间线/恢复消费和必要回归。
- `src-tauri/src/app/observer/mod.rs`、`src-tauri/crates/aio-observer-protocol/src/lib.rs`、`src-tauri/crates/aio-tui/src/client.rs`、`src-tauri/crates/aio-tui/src/ui.rs`：测试 DTO、65 秒协议等待预算、模型展示与兼容回归。
- `src/pages/providers/hooks/useProvidersViewDataModel.ts`、`src/pages/providers/__tests__/ProvidersView.test.tsx`、`src/services/providers/__tests__/providers.contract.test.ts`、`src/generated/bindings.ts`：真实结果和模型展示、既有测试 fixture 与 IPC 类型同步。生成 bindings 只由云端完成并核对必要差异。
- `.trellis/spec/aio-coding-hub/cross-layer/local-observer-tui-contract.md`：记录真实测试、模型字段、时间预算及失败语义；不新增重复的协议文档。

### 验收条件

1. CI 使用已有 mock 上游 server 捕获请求，证明每种 API Key/OAuth/固定桥接路径实际发送预定模型生成请求且认证/包装正确；支持输出上限的最终请求为 100 token，Codex OAuth 的已知移除行为单独断言。有效回答加正常完成或明确的 token 上限终态均返回成功。
2. 400/401/403/404/429/5xx、200 错误对象、HTML、空体、损坏 JSON、仅 usage/reasoning/tool、空白文本、响应大小超限截断、提前 EOF、流式错误与非 token 上限原因的 incomplete 全部返回失败；成功 HTTP 状态本身不能让测试通过。
3. 流式 fixtures 覆盖跨网络 chunk 分帧、UTF-8 拆分、心跳、只有终止标记、先文本后错误、正常完成，以及已有回答/无回答两种 token 耗尽结果；非流式同样覆盖各协议的 token 上限结束原因。未知 incomplete 原因不能按 token 上限放行，不需要向真实供应商发送请求来制造上述故障。
4. 当前方案成员停用、供应商全局停用、空跨供应商规则、路由指向其他供应商或其他模型时，手动测试仍只访问选定供应商本身或其静态绑定源；mock 的其他供应商接收次数为零。不会因测试自动开启路由或供应商。
5. 桌面、TUI、定时和恢复入口消费同一严格判定；同供应商并发仍合并，配置变化后的旧结果不回写新一代状态，超时后可以开始下一次测试。
6. TUI 的测试等待不会被普通快照 3500 ms 提前截断；完整结果耗时包含读取与解析，模型元数据对成功/失败均正确，旧协议 fixture 仍兼容。
7. 用本机正常配置供应商的合法模型做一次实际生成成功验证，再用明确无效模型做一次失败验证，可确认用户可见结果与实际请求一致；这两次由用户在具备新版本后主动操作，不在本轮自动发送、不修改用户持久化配置。若当前设置不能临时指定无效模型，则对应失败用 CI mock 验证，不为实战新增设置接口。
8. 真实调用结果只证明该时刻被请求模型的短文本生成，不代表所有模型、全部工具能力或未来请求；展示文案必须限制到本次模型。未完成新版本本机实测时如实标记待验证。

本轮未触发任何手动或定时上游测试，未修改测试代码、用户设置或供应商配置。

## 执行与验证安排

- 用户最新的“开始执行 PLAN”批准实施；已更新 origin/main 并建立上述独立 worktree。原先 plan-only/手动交接终点由本次明确开始执行指令替代，不再等待固定批准口令。
- TUI 按 r7 的 A-E 及上述验收执行。停用路由保存按已列最小修法；真实测试按 r8 的 100 token 预算、回答与结束判定执行，请求路径、模型来源和时间预算保持原方案。
- 采用 delegated/automatic：由 `gkd_execute` 单 writer 实施、必要记录和分项本地提交；结束后由 `gkd_accept` 独立验收，main 处理 findings 并写 review。施工角色不提交 PR、合并或发布。
- 本地不安装依赖、不运行 package-manager、测试、lint、类型检查、构建、Cargo 或开发服务器。
- 本轮文档仅作只读 Git/源码核对与差异空白检查。实施批准后，本地只允许 `git diff --check`、只读 Git/源码核对；当前没有列明或批准额外 Node 测试脚本，不把静态检查当作真实运行回归证据。
- 必要前端流程回归与 Rust 回归交由现有 PR 自动 CI。前端现行命令为 `pnpm test:unit:coverage`，Rust 为 `cargo test --workspace --locked -- --test-threads=1`，仅 GitHub Actions 执行。
- E 新增 macOS 原生代码，现有 Rust PR job 在 Ubuntu 运行，无法覆盖该分支。因此在自动 CI 增加 `observer-macos` job，使用现有 macOS runner/toolchain 约定，随 `rust_ci=true` 运行 `cargo test --manifest-path src-tauri/Cargo.toml --locked --lib app::observer::activity -- --test-threads=1`，验证生命周期及原生 begin/end；超时 30 分钟，不签名或打包。`ci-gate` 在需要 Rust 检查时要求此 job 成功，否则要求 skipped；同步现有门禁合同与负例测试，不能仅改校验摘要而移除旧断言。本地不执行该命令。
- PR 等待自动 `ci-gate` 与 `pr-title`；不额外手动启动常规 CI，不绕过失败门禁。
- 合并前完成消融审查，删除本次不必要的抽象和无证据兜底。
- 同一执行 worktree 保持一个 writer，三个问题按实际完成项分别使用简短中文 Conventional Commit，不混入本地 main 独有的规则/文档历史。PLAN 草稿留在主工作树，本轮不为文档创建施工分支或产品提交。
- 当前获准终点为实现、分项本地提交、独立代码验收和可审阅交付材料；按 gkd_execute -> gkd_accept -> main review 推进。必要 CI 与本机运行 AC 必须分别如实报告；未完成的运行验证不能以静态审查替代。
- 本任务尚无 PR 推送、合并、发布、新版本安装或分支/worktree 删除许可；这些均不以本次写 PLAN 自动推定。后续授权交付时，源码经任务 PR 和自动门禁交付，不推送 main；只有本任务拥有且事实已归档的活动记录、分支/worktree 可以进入清理清单。与此前版本发布任务的权限分开。
- 实施审查通过后按 gkd-closeout 路由归档本任务 Markdown 事实并衔接已授权的交付，当前不创建空 review/execution/progress 或归档副本。对未完成的本机观察与真实供应商调用分别保留待验证标记，不能以 CI 全绿宣称这些现场验收已通过。

## 非目标

- 不改变停用路由的转发资格，不自动启用供应商或方案成员。
- 不增加 TUI 管理接口或新的缓存层；真实测试若需增加模型结果字段，单独列入最终范围并保持协议兼容。
- 不重构全部供应商保存流程，不修整无关项目规则或本地历史。
- 不修改真实供应商配置、凭据、数据库记录或用户的进程启动方式。
