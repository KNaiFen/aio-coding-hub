# PR 前主线差异审查

## 审查范围

- 审查时间：2026-08-04（Asia/Shanghai）
- 功能开发基线：`dc311482c2af00177544c1b526dd173a2b7f20c9`
- 审查时 `origin/main`：`523256fc4108f03731bedb3962ff1d88acab01f4`
- 待提交功能分支：`codex/provider-metric-trends-ledger`

本审查在功能实现完成后、更新 PR 前执行。结论不以 Git 能否自动合并或 CI 是否通过为依据，而是核对主线新增行为与供应商趋势、日汇总投影、Provider 生命周期和 CI 门禁之间的业务语义。

## 主线漂移

基线之后主线合入 PR #31 至 #34，主要包含：

1. Codex SSE 流内部错误识别、脱敏证据、同供应商重试或切换供应商，以及对应设置和展示。
2. 按完整 PR/push 变更范围决定完整 CI 或文档检查的 scope-aware CI。
3. 上述两项的 CI 漂移修复、验证证据和 Trellis 归档。

选择性上游集成的 7 个父分支提交在 `origin/main@523256fc` 上完成 rebase 演练，无文本冲突。`ProviderChainView` 同时保留主线的流错误证据、circuit 字段和本分支的稳定供应商身份展示。

## 已确认交叉与修补

### 数据库 schema 版本冲突（已修补）

主线已经用 `v45_to_v46.rs` 和 schema v46 持久化流内部错误重试默认策略。本功能原先也占用 v46；直接合并会覆盖主线迁移，或让已安装 v46 的用户永久漏跑日汇总迁移。

处理：完整保留主线 v45→v46，将日汇总迁移顺延为 v46→v47，并同步 dispatcher、latest/max version、fresh schema、ensure 和迁移测试。两套迁移不能压成同一个 v46。

### 流式请求终态与统计语义（兼容）

主线会把终端 SSE 错误从 HTTP 200 表象中识别出来，并把最终请求归类为失败、重试后成功或全部失败。该变化会改变最终写入 `request_logs`/`usage_ledger` 的状态、错误、耗时和 Provider 归属，也会有意改变趋势口径。

- 提交前可重试流错误只写 attempt，不写请求级 ledger；重试成功或耗尽后只产生一条最终 ledger。
- 假 200、未完成的提交后错误和重试耗尽归一为 `502/error_present=true`。
- 已产生有效完成输出后的尾部错误仍是 `200/error_present=false`，但 circuit 记录 Provider 失败，错误证据仅进入 attempt/error details。日汇总不得从证据反推 `error_present`。
- Claude pending 行会按同一 `trace_id` 终态 UPSERT；触发器只把旧/新日期标脏，随后整日重建，不会重复累计。

日汇总只聚合最终 ledger 事实。UPDATE trigger 覆盖成功判定、耗时、TTFB、Token、CLI、Provider、名称、时间和排除标记，因此迟到终态会使旧投影失效。主线六个流式终态文件在集成分支中保持原样。

非阻断口径：重试成功时 `duration_ms` 是整个用户请求耗时，TTFB 和 Token 来自最终成功尝试；输出速率会把前次失败和退避耗时归给最终 Provider。这符合现有“请求级事实 + 最终 Provider”合同，不等于单次 Provider 尝试性能。

### CI 架构冲突（已修补）

旧性能门禁仅比较 `HEAD^1..HEAD` 的少量文件，在 rebase、合并提交和多提交 PR 中可能漏跑；直接覆盖主线工作流也会破坏新的 CI 分级合同。

处理：在 `ci-change-scope.mjs` 的统一分类结果中增加 `provider_trend_benchmark`。它依据 PR merge-base/push before SHA 的完整变更范围，在趋势 SQL、日汇总、ledger 和 retention 相关文件变化时运行；分类失败、空 diff 或手动触发时保守运行。工作流只消费该输出，并由 self-test 锁定触发与不触发样例。

### Provider 展示与清空统计（已保留双方语义）

选择性集成在清空 Provider 统计后失效 Usage 查询，并改进供应商身份展示；主线在同一组件增加流内部错误证据。集成结果同时保留身份格式化、流错误证据面板、circuit 字段和 Usage 缓存失效，未用任一分支整文件覆盖另一方。

### 多 session 隔离

正式集成期间，一个临时演练克隆出现了其他 session 的 Tray/路由未提交改动。该目录被原样保留，没有 reset、checkout 或提交这些内容；最终候选从已提交的集成 HEAD 重新克隆到唯一目录，再应用本任务补丁，避免误带或覆盖并行工作。

## 验证要求

1. schema v47 覆盖 fresh install、v45→v46→v47、已处于主线 v46 的升级，以及 ensure 修复缺表/缺触发器后的投影失效。
2. 混合查询继续在同一 SQLite read transaction 中选择覆盖日并读取 raw/rollup，避免并发刷新双计或漏计。
3. Provider `clear_usage_stats=true` 在同一事务删除 ledger 和对应 rollup；普通删除继续保留历史。
4. 本地只运行 Node、TypeScript、前端测试、lint 和 Vite build。Rust 格式、Clippy、原生测试、迁移测试和百万行门禁由 GitHub Actions 验证。
5. PR 合并前再次 fetch `origin/main`。若 SHA 前进，必须重复代码审查、修补和完整 CI，不沿用本次 SHA 的结论。

## 第二轮主线漂移

修补期间 `origin/main` 从 `523256fc` 前进到 `a0db6c20cfbae0d2b3cb64fbf868eed4110979b0`，新增 PR #35 的限额前置路由和 Tray 计数布局修复。人工审查覆盖 30 个变更文件：它们与本候选 99 个变更文件的真实路径交集为 0。

运行时影响仍需记录：主线现在会在 Session 偏好和 failover 前排除已知 OAuth/消费限额供应商，并在发送前复检。全部候选已限额时请求会以无 Provider attempt 的 `GW_NO_ENABLED_PROVIDER` 收口；对应 ledger 可能没有最终 Provider，因此不会进入 Provider 趋势。混合候选时 ledger 继续只记录实际获胜或最终失败的 Provider。这是主线路由合同的有意变化，日汇总应继承最终 ledger，而不能重新加入被预过滤的供应商。

本候选不修改候选解析、Session 绑定、限额计算、发送前 gate、attempt 收口或 Tray。主线代码原样保留；Provider 趋势查询和日汇总只消费其最终持久化结果。第二轮 rebase 后必须重跑允许的本地验证和云端原生 CI。

第二轮 rebase 已完成：`origin/main@a0db6c20` 是候选 HEAD 的 merge-base。PR #35 的 30 个文件、六个流式终态文件和主线 `v45_to_v46.rs` 均与 `origin/main` 逐字一致；仓库无冲突标记，提交范围 `git diff --check` 通过。

## 本地验证证据

- Node 22（与 CI 主版本一致）：310 个 Vitest 文件、2738 个测试通过。
- TypeScript typecheck、ESLint、目标 Prettier、spec link、gateway error-code sync 和 Vite production build 通过。
- CI change-scope self-test 通过；趋势目录、日汇总、ledger/retention 文件会触发百万行门禁，分类失败和手动触发会保守执行。
- Trellis 上下文验证通过：implement/check 各 4 条。
- Node SQLite 烟测通过：v47 的 3 个表和 3 个 trigger 可创建；同值 ledger UPDATE 保持 `complete`，真实状态变化标记 `dirty`；迁移设置 schema 47。
- 首次全量 Vitest 曾在替代执行器的 Node 24 下因其未配置文件的实验性全局 `localStorage` 产生 225 个 setup/teardown 连锁失败；同一现象在旧候选可复现。切换到 Node 22 后原复现文件和全量套件全部通过，因此该轮不作为产品失败。

依仓库政策未在本地运行 Cargo、Rust tests、rustfmt、Clippy、Specta 或 Tauri。迁移/Rust 编译、原生测试和百万行 release benchmark 仍是 PR CI 的强制门槛。

## 当前结论

`origin/main@a0db6c20` 与本功能业务目标兼容；schema v47、scope-aware 性能门禁和同文件业务语义已经按上述决定修补。第二轮 rebase 和本地允许检查已完成；云端完整 CI 通过并在合并前再次复核最新 main 后方可合并。

## 第三轮主线漂移

更新 PR 前的最终核验发现 `origin/main` 又从 `a0db6c20` 前进到
`0f864415d56f3808d7bff1a344739b668224d7d2`。新增提交为
`69bb4bf8` 及其 PR #36 merge commit，只把 PR #35 的五个 Trellis
任务文件从活动目录移动到
`.trellis/tasks/archive/2026-08/08-04-tray-limit-routing-fix/`，并补齐
任务完成状态、CI、dev-build、merge 和合并后 main CI 证据。

人工审查确认本轮没有产品代码、配置、依赖或测试变更；五个归档路径与本功能
100 个变更路径交集为 0，也不触碰本功能的 `08-03-*` Trellis 任务。
业务影响仅为从活动任务列表移除已经完成的 PR #35，不改变供应商趋势、日汇总、
schema v47、路由、Session、attempt 或 request ledger 语义。因此无需业务补丁，
可以完整带入。

第三轮 rebase 无冲突完成。`origin/main@0f864415` 是候选 HEAD 的
merge-base；PR #36 的归档目录与主线逐字一致，功能范围和工作树
`git diff --check` 均通过。rebase 后再次验证：

- 310 个 Vitest 文件、2738 个测试通过。
- TypeScript typecheck、ESLint、Prettier、spec link、gateway error-code sync、
  CI change-scope self-test 和 Vite production build 通过。
- 依仓库政策仍未在本地运行 Cargo、Rust tests、rustfmt、Clippy、Specta 或
  Tauri；这些原生门禁必须由新 PR head 的 GitHub Actions 验证。

## 最终 PR 前结论

`origin/main@0f864415` 与候选业务语义兼容，且本轮主线漂移没有代码级
交叉或隐含回归。候选可以更新 PR；合并前仍必须再次核验 main SHA，若再次前进，
重复同样的逐提交业务审查、rebase 和完整 CI。

## 首轮 PR CI 编译修复

PR head `38785d9a` 的 CI run `30844374619` 中，frontend、文档合同、
支持合同、依赖审计和 PR 标题检查全部通过；Rust job `91789036552` 在
“Format Rust and generate bindings in the cloud”阶段编译导出绑定示例时失败，
因此 Clippy、Rust tests 和百万行 benchmark 尚未执行。

失败为 `trend_common.rs` 的 `E0282` / `E0308`：两条 SQLite
标量查询没有明确结果类型，且第二条查询直接返回项目 `AppError`，与函数
声明的 `Result<bool, String>` 不一致。修复保持现有公开错误合同和 SQL
不变，只把两条标量结果显式声明为 `bool`，并让第二条查询通过 `?`
使用既有 `From<AppError> for String` 转换后返回 `Ok(bool)`。

现有混合趋势测试已覆盖缺少汇总表时纯 raw、完整 schema 时混合查询，以及
ledger backfill 未完成时回退 raw；本修复没有新增分支或业务行为。下一轮 CI
必须重新通过云端编译、格式/绑定漂移、Clippy、Rust tests、依赖审计和百万行
benchmark 后才可合并。

第二轮 CI run `30845906061` 已成功完成 Rust 编译、格式化和绑定生成，
证明上述类型修复可编译；随后仅在 generated-file drift 门禁失败，Clippy、
Rust tests 和百万行 benchmark 仍因门禁顺序未执行。artifact
`cloud-native-fixes-e1116cbb516e6160f7b614874bcf14cea1edfe0f-1`
（ID `8868752325`，digest
`sha256:f8abd6fbf867142818deb3a1e62c3aba33b7aee22ddc656b1fd674cb01282fe0`）
已完整审查。

artifact 只修改 8 个 Rust 文件，内容均为 `rustfmt` 的换行、缩进和 import
排列；没有 SQL、控制流、字面量、业务断言、生成绑定或锁文件变化。补丁先通过
路径白名单与 `git apply --check`，随后原样应用。下一轮 CI 必须确认
generated-file drift 为零，并继续执行此前尚未运行的 Clippy、Rust tests、
依赖审计和百万行 benchmark。
