# 供应商周期限额重设实施记录

- Execution: e1 / PLAN r3；执行完成时间 2026-09-10 20:47 +0800。
- Worktree: `<task-worktree>`。
- Branch: `feat/provider-limit-reset`。
- Baseline: `25b4203165e709f9c9b0d8f18c0290b12c296b4a`。
- 本地提交: `bf989bb33cbba2fd73547b75778cbe546de2d96a`，`支持独立重设供应商周期限额`。
- 24 个实现/测试/文档文件，1267 insertions / 178 deletions；execution、progress 未提交。无推送、PR、dispatch、合并、归档、清理或嵌套代理。

## 实现事实

- 新增 v54->v55 与 ensure：`provider_limit_resets` 主键 provider/period，四值 CHECK，无 total，nullable anchor 和非负 cutoff，provider 外键级联删除。重设 IMMEDIATE 事务检查 provider、读取 `sqlite_sequence` 的 request_logs 已分配高水位，仅写所选周期；5h 同步原窗口字段。
- `domain/provider_limit_usage.rs` 提供小型共用 manual_window，固定 5h/24h/168h 从原锚点递推；月使用已有 chrono Months 月末钳制及 SQLite localtime 转换，从原始日期计算每期。重复本地时刻选择较早瞬间、缺失时刻按差额前移。月转换采样目标前后两天的系统时区偏移；未加依赖或通用日历模块。
- 显示聚合仍批量访问 usage_events，每 provider 传递四组 start/end/cutoff，总累计没有 reset 过滤。SQL 参数增为每 provider 13 个，单批由 300 改 75，低于 SQLite 999 参数基线。usage_events 暴露的稳定 ID 实际名为 `id`，查询以 `id AS request_log_id` 对接。
- 网关每次读取 reset 表，保留 ledger 完成前后 source switch、provider 局部索引和 NULL final_provider_id 兼容查询；行读取稳定 ID，按每周期 cutoff 累加。锚定窗口限额使用真实 end，日 rolling 仅无锚点时取带自身 cutoff 的 buckets。网关 gate 与预筛选使用评估时当前时刻，避免请求最初创建时间滞后于操作后周期。
- daily mode/time 比较后端持久值，保存真实调度变化时在原 upsert 事务里清空日 anchor，保留 cutoff；只改金额或不变表单保存不清空。配置结构及复制/分享/导入均未扩展运行状态。
- `provider_limit_reset` command 与 registry 已接入，前端使用预期 Specta 生成的 ProviderLimitPeriod 和 commands.providerLimitReset。四个 end 字段及 daily_manual_anchor 经 service 保留；首页使用实际边界、日锚点不再显示 rolling 标签。
- 编辑器保存供应商四周期独立 RefreshCw、tooltip、accessible name、确认框；create/duplicate/累计没有动作。处理中阻止重复、保存及关闭；表单不 reset。写入完成后取消旧读请求并等待所有已挂载限额 query 刷新。已提交但读数失败单独提示并只重试刷新，不再次 reset。未配金额仍可重设；首页隐藏仅含重设运行状态而无金额的空白卡。
- README 和 request-log/ledger 合同已补操作及历史保留、分界、时间窗口、即时生效规则。

## 测试代码覆盖（均尚未执行）

- `ProviderEditorDialog.test.tsx`：四 period 路由、取消不调用、双击只写一次、pending 保持确认框且禁用保存关闭、成功等待刷新、金额草稿保留、create/duplicate/累计无按钮、写失败重试、已提交读失败仅重试读取、真实月起止及用量更新。
- `providerLimitUsage.test.tsx`：所有挂载查询等待刷新、提交后刷新失败的独立异常、不自动重复写入、提交前初次读取的取消与替换。
- `providerLimitUsage.service.test.ts`：四枚举/ID 边界、void IPC 成功及实际失败传播、生成字段归一化保留。
- `HomeProviderLimitPanel.test.tsx`：真实月结束时间、日锚点标签、无金额仅重设状态不显示空卡；已有直接构造 fixture 已补字段。
- 网关 Rust tests：完整/未完整 ledger × fixed/rolling × 四周期矩阵；同一连接、同一配置对象、同一固定时间 reset 前超限，提交后立即允许，完整显示快照只改变选中周期，其他 provider/金额/总累计保留；同秒新记录重新达到限额、next_available 为 end、end 处半开切期及跨多个闲置周期；先月后周保持月用量/窗口/标记/拦截；零限额及总限额继续拦截；retained daily cutoff 应用于 rolling buckets 及 NULL provider 兼容分支。
- domain Rust tests：用户周示例 2025-09-01~09-08 原窗口于 09-07 重设为 09-07~09-14，旧边界不清零、到期递推；月同日/月末/闰年/跨年闲置；DST gap/repeat 用单独 test 子进程设置 America/New_York，运行生产 SQLite/chrono 路径；pending 后补费用、回填、删明细高水位、早于 anchor 晚落库不计、同秒新记录计入、重开 DB 持久化；非法 period/provider、未配金额和级联删除。
- migration/provider tests：v54 新表约束/幂等/历史保留、新库/ensure；只改金额不撤销日锚点，改 daily mode/time 撤销且保留 cutoff，复制的新 provider 无 marker。
- 既有聚合回归测试继续对比原 SQL 结果及 MATERIALIZED/索引规划；投影新增稳定 ID 后宽度断言从 3 更新 4，未删除断言或降低标准。

## 实际命令与结果

所有项目命令均显式 workdir/cwd 指向本 worktree；代码读取使用 FastCtx 批量检查及 rg 定位。无本地测试、编译、安装、包 scripts、格式化工具、绑定生成或服务器。

| 命令 | 实际结果 |
| --- | --- |
| `git status --short --branch`（开始） | `feat/provider-limit-reset...origin/main`，仅 `.gkd/execution.md` 未跟踪 |
| `git rev-parse HEAD`（实施前最终确认） | baseline `25b4203165e709f9c9b0d8f18c0290b12c296b4a` |
| `rg --files -g AGENTS.md -g '!node_modules' -g '!.git' -g '.codegraph/**'` | 仅根 AGENTS.md；目录检查无 `.codegraph/`，跳过 CodeGraph |
| `rg -n ...` | 定位 allowed source、迁移、tests、types、usage_events ID、网关调用点、query keys、chrono 依赖；未运行代码 |
| `git diff --stat` / `--numstat` / `--name-only` 与范围内 `git diff -- ...` | 已复查文件范围、快照联动、单 writer 差异 |
| `git diff --check`（实施中及最后） | 全部执行 exit 0，无输出 |
| `git config --get core.hooksPath` | exit 1，无配置值；未改配置 |
| `git ls-files --others --exclude-standard`（提交前） | execution 和新迁移，仅这两个未跟踪项 |
| `git add` 明确列举 24 个实现/测试/文档路径 | exit 0；未 stage execution/progress |
| `git diff --cached --check` | exit 0，无输出 |
| `git commit -m "支持独立重设供应商周期限额"` | exit 0，提交 `bf989bb3`，24 files changed |
| `git rev-parse HEAD`（提交后） | `bf989bb33cbba2fd73547b75778cbe546de2d96a` |
| `git status --short --branch`（提交后、写 progress 前） | ahead 1，仅 `.gkd/execution.md` 未跟踪；代码干净 |
| `git show --format=fuller --stat HEAD` | 提交说明与文件列表符合本任务，2026-09-10 20:47:51 +0800 |

环境采用 execution 已记录事实：Darwin arm64/macOS 26.5.1、Mac16,12、16 GiB、约 245 GB 磁盘。没有重新运行资源调查或本地工具链命令。

## 消融审查与判断

- 保留已有窗口实现、批量 SQL、query keys、service helper、ConfirmDialog、Tooltip 和表单；只共享实际重复的手动窗口规则与 reset 读取。
- 不新增配置字段、依赖、热路径 usage_events 视图、通用限额框架、批量重设或额外入口。
- 发现并纠正 SQL 参数数、usage_events 稳定 ID 实名、测试池默认单连接、初始读数并发、reset-only 首页空卡等范围内问题。测试文件沿用交接已允许的对应 tests，没有材料性越界。
- 本地非生成实现完成并已提交；只读差异卫生检查通过。不能据此宣称编译、类型、测试或运行时 UI 通过。

## Main 接续与剩余风险

1. 首次云端必须在本 head 生成 Rust fmt 与 `src/generated/bindings.ts`，获取关联精确 SHA/attempt 的小体积 cloud-native-fixes patch。预期新增 `ProviderLimitPeriod`、`commands.providerLimitReset`，并扩展 ProviderLimitUsageRow 四个 end / daily_manual_anchor。当前 binding 故意未手工伪造，因此现有 head 前端类型/构建尚不完整。
2. 本地所有测试、类型、lint、build、rustfmt、clippy、Rust workspace tests、浏览器/桌面 UI 均为 **not run**，按 execution 资源限制留云端。需 main/closeout 按已批准单 PR 自动 CI 跑 contracts/frontend/rust/observer-macos 及 required gate；本角色未推送或 dispatch。
3. 时区 DST 子进程测试、SQL query plan、Tauri/Specta 生成、React Dialog pending 可访问性/渲染及共享类型兼容需云端证据；任何失败按涉及源码/生成 SHA 修复，不使用本地重任务。
4. 同秒尚未落库的在途请求可能计入新周期是交接已批准边界；重设前已落库 pending 和早于新 anchor 的请求由 cutoff/时间限制排除。

执行角色到此停止；后续审查、云端验证、交付由 main 安排。

## e2 审查返工事实

- Execution e2 / PLAN r3；本轮提交时间 2026-09-10 21:22:05 +0800。
- 唯一执行目录沿用 `<task-worktree>`，分支 `feat/provider-limit-reset`。
- 起始 head `3729f71c5235ac3108066afcc6ad66c0f8a9f0c2`；本地提交 `3feb92c8920a613173a2979ad3e0e1d1f6d04f58`，说明 `修正限额缓存刷新和每日夏令时边界`。
- 仅修改 4 个获准实现/测试文件，437 insertions / 116 deletions。保留 main 已提交的云端 bindings/格式补丁及 js-yaml 4.3.2 修正；未改依赖、CI、生成文件或其他 GKD 记录。execution/progress 继续未提交。

### 实现与测试代码

1. `src/query/providers.ts::useProviderUpsertMutation` 在既有成功回调中等待 `providerLimitUsageKeys.all` invalidation。活跃限额查询刷新后保存才完成，未挂载缓存失效，重开时可读取新的日调度窗口。没有新增 save 流程或前端 dirty 判断。
2. `src/query/__tests__/providers.test.tsx` 新增真实 `useProviderUpsertMutation` 和 `useProviderLimitUsageV1Query` 联动用例，使用生产 staleTime 5 分钟，先放入 fresh 手动日锚点缓存，保存 `06:00:00` 调度后检查 active/未挂载缓存均失效、等待 deferred 读取完成、日锚点解除和起止更新、立即重开读取新窗口，以及全局限额查询重新挂载时刷新。仅 mock service 边界，没有 mock query adapter。
3. `domain/provider_limit_usage.rs` 复用原网关 `compute_daily_fixed_bounds` 的 SQL/时间解析逻辑，共享函数由 domain 提供，两端共同按原配置 wall time 分别解析当天、前一天、次日边界，防止 DST 缺失时刻的顺延偏移带到次日；日缓存保存 `(start,end)`。网关调用该函数，原解析函数的 3 个测试连同全部断言原样迁至 domain。滚动日算术仍是 `now-86400` 到 `now+1`；其他窗口未调整。
4. 网关新增 `fixed_daily_dst_bounds_match_display_usage_and_gateway`，复用独立 test 子进程方式设置 `TZ=America/New_York`，生产 SQLite 路径覆盖 2026-03-08 固定 `02:30:00` 的缺失时刻前、真实 `03:30` 切期、次日 `02:30` 切期及新消费；逐点比较真实 start/end、显示用量、网关 Allow/Limited 和 next_available。没有修改进程全局 TZ 的测试并发风险。
5. 网关新增 `consecutive_resets_preserve_all_other_period_snapshots_and_markers`：一个真实 DB、同一连接、同一配置对象、固定同一 now，以月、周、日、5h 顺序连续重设。每步完整 JSON 行快照只更新所选 start/end/usage（及每日 anchor 布尔），完整四标记数组只更新所选 marker；金额及总累计由完整快照保持。每步之后插入同秒新消费，既有 reset 周期用量非零，下一步继续保持原锚点、cutoff 和消费；第二步起月超限及真实月 end 仍拦截。原四周期/回填/fixed-rolling 矩阵和先月后周测试均保留。

### 实际检查、命令与结果

环境复用 execution 既有事实：Darwin arm64/macOS 26.5.1、Mac16,12、16 GiB、约 245 GB 磁盘。没有重新调查环境或执行工具链。项目文件读取、命令和修改均显式使用本 worktree 绝对路径/cwd/workdir；外部只读取适用 Git/验证 skill。

| 命令或检查 | 实际结果 |
| --- | --- |
| FastCtx inspect/glob/grep | 完整读取 execution、根 AGENTS、既有 progress、云端合同，定位并阅读改动源码及必要测试；仅根 AGENTS，没有 `.codegraph/` 文件；未执行代码 |
| `git status --short --branch`（开始） | ahead 2；仅 execution/progress 未跟踪 |
| `git rev-parse HEAD`（开始） | `3729f71c5235ac3108066afcc6ad66c0f8a9f0c2` |
| `git diff --stat`、`--numstat`、`--name-only`、范围内 `git diff -- ...` | 已检查全部差异及 4 文件范围；原断言迁移、原矩阵和月后周覆盖保留 |
| `git diff --check`（初次及最终） | 均 exit 0，无输出；最终仅补强测试断言后重查 |
| `git config --get core.hooksPath` | exit 1，无配置值，未修改配置 |
| `git add -- src/query/providers.ts src/query/__tests__/providers.test.tsx src-tauri/src/domain/provider_limit_usage.rs src-tauri/src/gateway/proxy/handler/failover_loop/prepare/provider_limits.rs` | exit 0，仅 stage 4 文件 |
| `git diff --cached --check` | exit 0，无输出 |
| `git commit -m "修正限额缓存刷新和每日夏令时边界"` | exit 0，提交 `3feb92c8`，4 files changed |
| `git rev-parse HEAD`（提交后） | `3feb92c8920a613173a2979ad3e0e1d1f6d04f58` |
| `git status --short --branch`（提交后） | ahead 3；仅 execution/progress 未跟踪，代码干净 |
| `git show --format=fuller --stat HEAD` | 提交时间、中文说明和 4 文件清单符合本轮 |

### 消融判断与待验证

- 保留现有 QueryClient、成功回调、批量聚合、SQLite 日期解析及网关查询；仅共享本次真实重复的固定日边界。没有新增通用框架、依赖、窗口兜底或平行业务流程。
- 本轮未遇到材料性缺口；定位差异是当前格式化源码 `list_at` 中 e1 的固定日 end 已位于约 735 行，实际改动沿 e2 允许符号范围完成。
- 本地实现完成并提交；差异卫生检查通过。新增行为测试 **未运行**；TypeScript、lint、前端测试/覆盖率/构建、Rust fmt/clippy/workspace tests、DST 子进程和 UI 运行时均为 **not run**，受获准云端分工限制，不能以静态复查替代通过证据。
- 旧 run `34479239040/attempt1` 对应 PR head `bf989bb33cbba2fd73547b75778cbe546de2d96a`，其 contracts/observer 成功及 rust/frontend 中止事实沿用 execution；不是本 e2 head 的验证。前序 bindings 已由 main 应用，本轮未变更导出接口；新增 Rust 源码仍可能触发格式 drift，需 main 以本 head 对应 SHA/attempt 云端 artifact 接续。
- main 接续合批推送并等待批准的单 PR 自动 CI：contracts/frontend/rust/observer-macos 与 required ci-gate/pr-title；其中 frontend 仍需证明 js-yaml 4.3.2 override/lock 审计与 frozen 安装，Rust 仍需证明格式及本轮测试。执行角色没有推送、dispatch、PR、验收、合并、发布、归档、清理或派生代理。

e2 执行角色到此停止，交回 main。

## Main 接续 CI 修复，2026-09-11

- e2 后已应用 run `34482402998` attempt 1 / merge `3f4bd6bf50184489b8f0e776673d5fd6d7ab8488` 的 Rust 格式 patch，并将 `ProviderLimitRefreshError` 的 cause 保存在现有类成员中，提交 `761b8619d1f4364c862a3c124ee73e8c2fb733b3`。
- 对应 run `34483572457` attempt 1 / 实际测试 merge `6504c6061ede766de9b40fe3308f4e504602d294`：frontend 全部通过，contracts/observer-macos/pr-title 通过；Rust fmt/bindings 无漂移、clippy 通过，lib tests 2978 passed / 5 failed / 4 ignored，后续 benchmark/audit 未运行。
- 5 项新增网关测试均因 fixture 未加入默认路由而取到空列表，尚未执行行为断言。main 在各自创建测试供应商后、取得连接及固定网关配置之前调用 `providers::default_route_set_order`。单供应商加入自身，双供应商矩阵同时加入两者；没有修改生产代码、断言、被测网关读取流程或验证标准。
- 消融审查：5 个直接调用复用生产路由 API，不引入 helper，不扩大共享创建 fixture 的副作用。待提交后同一 PR 自动 CI 验证，不运行本地测试/编译/格式生成。

### 接续验证完成

- main 已执行 `git diff --check`、逐项 diff 阅读和 `git diff --cached --check`，均通过；仅 5 行测试前置调用。中文提交 `4226ed5d648e3d1b783187d1007f5ef51315676b`，已推送 `feat/provider-limit-reset`。
- PR #196 CI run `34500544358` attempt 1 全部必要检查 success，测试 merge SHA `2372b5ab8e3d2eb6d555c990966e11ea6c9316c9`。前端 313 个测试文件、构建成功；Rust lib 2983 passed / 0 failed / 4 ignored，其余 workspace tests、Clippy、依赖审计成功；此前 5 项失败用例全部通过。release benchmark 按 workflow 条件 skipped。监控于 `2026-09-10T16:37:24Z` 完成 required + expected 集合，无缺失检查。
- 同连接/配置的网关即时评估、四周期逐次完整快照、先月后周保持月拦截、固定日 DST、零/累计限额行为已有云端通过证据。本地桌面 UI 人工实测未运行；没有发布、安装或重启用户应用。
- main 审查结论已更新为通过，进入单角色收尾。当前实现树仅 execution/progress 未跟踪，远端 main 仍是 baseline；其他任务主树差异保留。
