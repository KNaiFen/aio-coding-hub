# 供应商周期限额重设审查

## 当前结论

- **通过实施审查，准予收尾。** PLAN r3 / execution e2，最终已验证实现 head `4226ed5d648e3d1b783187d1007f5ef51315676b`；baseline `25b4203165e709f9c9b0d8f18c0290b12c296b4a`。原 writer 与独立 accept 已停止；两项 P2、连续四周期证据遗漏及后续 CI 兼容/fixture 问题均解决。独立审查与 main 受影响部分复查无剩余阻断 finding；消融审查通过。
- 最终实现 CI `34500544358` attempt 1 success，PR 测试 merge SHA 为 `2372b5ab8e3d2eb6d555c990966e11ea6c9316c9`，不是 PR head。监控于 `2026-09-10T16:37:24Z` 返回 required + expected 全成功：contracts/frontend/rust/observer-macos/ci-gate/pr-title，无缺失。main 补查 run metadata 与关键测试日志：前端 313 个测试文件通过、build 通过；Rust lib 2983 passed / 0 failed / 4 ignored，全部 workspace tests、Clippy、依赖审计通过；5 项此前失败用例均显示 ok。release benchmark 本 run 按现有工作流 skipped，未增设或绕过门禁。
- 同一数据库连接与同一网关配置对象的真实评估测试证明事务提交后立即采用独立窗口；它不是完整桌面应用人工实测。本地仅差异检查和 Git/gh，无安装、测试、编译、格式生成或开发服务器。未发布、打包或安装。
- 收尾授权沿用 PLAN：单 `gkd_closeout` 保存五类记录和 summary，推送最终归档 head 并验证，正常合并 PR #196 至 main、等待实际 merge SHA 自动 CI，随后清理本任务独占活动记录/临时 worktree/任务分支。主工作树既有其他任务修改及本地独有历史保留，禁止为本任务重置 main。

## 已解决的接续问题

- 2026-09-11 接续：head `761b8619d1f4364c862a3c124ee73e8c2fb733b3` 的 CI run `34483572457` attempt 1 已失败，实际测试 merge SHA `6504c6061ede766de9b40fe3308f4e504602d294`。frontend（含 unit/build）、contracts、observer-macos、pr-title 全成功；Rust fmt/bindings 无漂移、clippy 成功，Rust lib tests 为 2978 passed / 5 failed / 4 ignored，后续 benchmark/audit 未运行。原 CI 代理连接异常不等于 CI 结果，本次直接取已终态 run/日志恢复证据。
- 5 项失败均发生在新增测试调用 `list_enabled_for_gateway_in_mode` 后读取空列表，尚未到达行为断言。main 核对默认路由查询实际 JOIN `default_route_providers`，共享 fixture 只创建 providers 记录；已在 5 项测试初始配置中调用现有 `default_route_set_order`，在持有连接与加载网关配置之前加入路由。只补测试前置条件，所有限额、隔离、DST、即时生效断言保留；待新 head 云端通过。
- `761b8619` 已应用第二轮精确来源的格式修正，并以 `readonly cause` 成员修复 Error constructor 的 TypeScript 兼容；该 head 的 frontend 完整通过证明此项解决。

## 前序结论

- 第二轮 CI: run `34482402998` attempt 1，head `3feb92c8920a613173a2979ad3e0e1d1f6d04f58`。contracts/observer/pr-title 成功，frontend 的依赖审计、lint和全部unit tests成功，build在 `ProviderLimitRefreshError` 使用Error第二参数时TS2554失败；main局部修复为类成员保存cause，保留原消息/类型语义。Rust仅一处14行格式漂移，artifact `10154391168` / `cloud-native-fixes-3f4bd6bf50184489b8f0e776673d5fd6d7ab8488-1`，875 bytes；该3f4bd6bf是实际测试merge SHA，监控报告将它与head混同，已按artifact/工作流日志纠正。待应用并在新head验证Rust tests/build。
- e2 复查补记: 当前实现 head `3feb92c8920a613173a2979ad3e0e1d1f6d04f58`，writer 已停止。main 只复查 e2 受影响差异：provider upsert 已 await 限额 key invalidation；固定 daily start/end 已让显示与网关复用原网关计算；新增同一数据库/同一 provider 连续四周期完整快照与 marker 比较，原先月后周保留。两项 P2 和测试遗漏在代码层已解决，待新 head 云端验证；不重复全量技术审查。
- 云端格式与绑定已应用，js-yaml 4.3.2 patch 升级提交 `3729f71c`，metadata 来源与原依赖边已检查。e2 推送到 PR #196，最终 CI 尚未通过。
- PLAN r3，execution e1；baseline `25b4203165e709f9c9b0d8f18c0290b12c296b4a`，已审实现 `bf989bb33cbba2fd73547b75778cbe546de2d96a`。writer 已停止，独立 gkd_accept 已返回。当前不通过，须修复下列范围内遗漏并复查受影响部分；最终 CI 仍待通过。
- main 已按生成来源应用格式/binding patch，提交 `724c722f`；补丁来自 PR #196 CI run `34479239040` attempt 1，实际测试 merge SHA `30fb8e082fc487256773117191fac5145c857913`、PR head `bf989bb33cbba2fd73547b75778cbe546de2d96a`，artifact `10153129357`，大小 10992 bytes。

## Findings 与处理

1. P2: `src/query/providers.ts::useProviderUpsertMutation` 未使本地限额缓存失效，默认 staleTime 5 分钟。修改日调度保存后后端锚点已停用，重开编辑器仍可能显示旧日期/用量。增加保存成功后的限额缓存刷新及生产 staleTime 行为测试；这是 PLAN 原要求的接口联动遗漏，e2 纳入该函数及现有 query tests。
2. P2: `domain/provider_limit_usage.rs` 使用已换算的起点加一天推导 fixed daily end，DST 不存在时间调整会带入次日；网关按原配置计算，结果不一致。修正为按 daily_reset_time 计算实际下一边界，补同一时区显示/用量/网关一致性测试。
3. 验证遗漏: e1 四周期矩阵每轮新库，未完全证明同一 provider 连续重设四周期保存其他已有锚点。补齐连续重设和完整快照对比，保留先月后周用例。

main 抽查对应 query 保存路径、LimitsSection 和两条 Rust daily 边界源码，认可这些 finding。未重复全量技术审查。独立审查确认其余周期隔离、累计保留与锚定递推未见阻断问题；不以测试代码冒称通过证据。

## CI 事实

- PR https://github.com/KNaiFen/aio-coding-hub/pull/196
- 初轮 https://github.com/KNaiFen/aio-coding-hub/actions/runs/34479239040 ，attempt 1 failure。
- contracts、observer-macos 成功；Rust format/export 与 patch 上传成功，随后因 drift 按设计失败，clippy/tests 未运行。
- frontend 在依赖审计阶段失败：现有固定 js-yaml 4.3.1 被新公告阻止，后续 lint/tests/build 未运行。
- main 根据官方修复版本和 npm 精确元数据做最小 4.3.1→4.3.2 override/lock 同步，不新增依赖、不改变审计门槛。范围补记到 plan-changes，e2 不需执行依赖安装。

## 下一步

实施及必要 CI 已通过；由单收尾角色准备归档、验证最终 head、正常合并并等待 main CI 后清理。本地仍仅差异与 Git/gh 轻量检查。
