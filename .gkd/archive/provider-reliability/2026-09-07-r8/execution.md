> 归档时点：2026-09-07 20:58:18 +08:00。来源：r8-exec7，原 writer 已停止。
> 这是本任务记录的脱敏快照；个人绝对路径及执行角色句柄改为逻辑标识，原有判断和各轮作者事实保留。
> 历史阶段中的“当前”“未完成”等表述保持原观察时点；最终事实以 [收尾摘要](summary.md) 和 [审查记录](review.md) 的“最终源码决定与收尾交接”为准。SOURCE STATIC PASS 不等于编译、CI 或运行验收通过。

# 供应商可靠性修复执行交接

- Revision：r8-exec7，来源为 main 已获批 PLAN r8；本轮仅修并发回归的数据库 fixture。
- 用户批准：开始执行 PLAN。
- 路线：delegated/automatic；执行角色 gkd_execute，是本 worktree 唯一 writer。
- 执行目录：`<执行工作树 provider-reliability>`
- 分支：`fix/provider-reliability`
- Baseline：`a35f2aa4ea469f6e4066582b2e969f1ec44fca2e`，已从更新后的 origin/main 建立。
- 主工作树独有历史和另一个 gkd-project-rules worktree 均非本任务范围，不触碰。

## 本轮返工目标

### r8-exec7 唯一当前返工

上一 writer 和独立 accept 已停止。当前 HEAD `d22f88956a90669075bbeb2f348534f728375cd2`，17 个提交保留，产品工作区清洁。最后两项产品实现已通过针对性源码复查，无确定新产品 bug；现在只修一项会导致回归无法到达目标阶段的测试设施。

P2：`accepted_oauth_commit_orders_real_reads_without_blocking_admission_or_slots` 用 `db::init_for_tests`，其 pool 固定 max_size(1)。旧 CAS 在 oauth_accepted 暂停时独占连接，新 flight 的配置加载无法取连接，自然到不了 oauth_read_waiting；后续普通 WAL SELECT 同样无法在旧 commit 前取连接。不能删除通知/并发/33轮断言或先放旧 commit 回避。请为此测试提供真正可同时容纳旧 CAS、新 flight 配置/只读、超时消费与 WAL 对照的独立连接。

main 已核对范围遗漏，本轮额外允许 `src-tauri/src/infra/db/mod.rs` 中 **cfg(test)** 的 `init_for_tests` 初始化路径，增加显式池容量的测试入口，复用原迁移/连接配置；原 init_for_tests 保持默认单连接，生产 init/open_read_only/configuration 完全不变。该辅助只用于确需并发的本任务回归，不建立通用 fixture 框架。选择足够但小的固定池大小（例如4，按实际同时占用数确定）且不要通过修改真实环境/生产容量实现。

在 `app/provider_availability_probe_runtime.rs` 本任务新增的持数据库事务暂停/并发消费回归中，静态核对哪些也需要独立连接并最小接入该 fixture；尤其 oauth 接受前后竞态、旧 commit 未释放的新读取/33轮、实际 reset 并发、observation 事务暂停、失败队列回归。只调整有明确依赖的用例。不要改变产品逻辑，禁止无关测试重构。回归须让新 flight 在旧commit仍未释放时走到真实 credential read gate，并让独立普通 WAL SELECT读旧值作对照；避免 test-only 先释放事务制造通过。

核对清楚每个通知的到达条件和连接/锁所有权，保留既有所有断言。按现有要求只编写并静态核对，不本地运行 Cargo/测试/生成器。完成消融、git diff --check、一个具体中文 Conventional Commit，progress记录fixture容量理由/受影响用例/未跑验证后停止。

### r8-exec6 当前两项返工

前 writer/accept 均停止；当前 HEAD `1946498c3b2a34f12d53b4c49c8f4f0ee0ef7149`，15 个任务提交完整保留，产品工作区清洁。r8-exec5 的 DB/manager 锁反转、OAuth 同步接受点、超时失败排队与 warnings 代码已完成；本轮仅处理剩余两项，顶部本节优先于历史清单。

1. **P1 完成锁等待耗尽全局 blocking 配额**：finish_probe 在 blocking::run 闭包内同步锁 completion_gate，此时全局 permit 已持有；长期堵塞一个供应商、多次60秒超时后持久化 owner 会各占一个槽等同一锁，最终饿死其他供应商、observer、配置。将 completion_gate 改为现有 tokio async mutex，先异步取得 owned guard，再启动 blocking::run，guard 移入实际闭包保持消费序。等待 gate 时不得持全局 blocking permit，取消仍未进闭包的任务正确释放，已运行闭包的 gate 随实际闭包结束才释放。保持每供应商队列/Arc 身份/超时失败顺序，不增加全局线程池。补实际共享入口多轮连续超时，同时另一供应商能正常生成/完成的回归，轮数足以覆盖原全局槽上限或通过有限独立配额证据明确证明没有占槽等待。不能只测单次超时。
2. **P2 OAuth 读取越过已接受未提交写**：当前 ProbeBudget 接受点与失效排序已成立，但新 flight 会在 WAL 下普通 SELECT 读到旧凭据。把生产探测所用实际 transport provider 的凭据读取与该 transport 已接受未提交 CAS 排序；新 flight 可以准入，等待凭据时仍受自己的60秒预算，不得使用旧已轮换 refresh token，不可通过降低断言或把测试读取改成 IMMEDIATE 而生产仍普通 SELECT。优先局部、每 transport 的异步凭据协调，等待发生在申请 blocking permit 之前；实际已接受 CAS 的 owned guard/完成通知必须随 commit/rollback 真正结束释放，异步调用取消不能提前解锁。不要复用会挡住新 flight 准入的 mutation gate，不增加通用刷新/事务框架。normal OAuth 刷新/CAS 协议、token fallback 行为不改；若为复用必须让同一凭据解析入口共享该局部 gate，限于原已允许 queries.rs 函数及必要私有辅助与导出，属于已授权必要范围。

OAuth 回归应停在旧 oauth_accepted 后、tx.commit 前，触发旧 flight 超时，直接启动并放行新 flight 的真实生产读取，保持旧 commit 未完成，证明没有第二次 refresh/旧认证生成；再释放旧 commit，新 flight 用新凭据成功，模型和单生成限制继续满足。另验证新读取等待到自己的截止时准入会释放，不会长期占用 global blocking 槽。保留接受前失效回滚、接受后配置写的事务顺序、timeout失败消费和旧完成隔离回归。

本輪范围仍为已有 probe runtime/domain/provider queries 与必要测试；不改其他已关闭项。务必静态核对同步/异步 guard 的真正所有权生命周期和 cfg(test)/clippy 返回值。补回归但禁止本地运行；消融、git diff --check、分项中文提交，progress 写清两项证据和未验证事实后停止。

### r8-exec5 唯一当前返工清单

前 writer 与独立验收均停止。当前 HEAD `7ad71ec2c2a96c43ae86aa4b8c001a3c901fd10a`，11 个任务提交完整保留，产品工作区清洁。下方 r8-exec4/3/2 为历史设计与允许范围依据，冲突时以本节当前返工要求为准。原 Gemini 角色、中文失败说明源码已通过；入口/活动/router 回归已有，不重做。只修以下集中 findings：

1. **P1 锁顺序反转**：app/provider_availability_probe_runtime.rs::finish_probe 在 SQLite IMMEDIATE 事务内等待 gateway manager，而现有 app/gateway_control.rs -> gateway/control_service.rs::{circuit_reset_provider,circuit_reset_cli} 在 manager 下写同一 DB。并发会互等直到 busy timeout，造成重置已改内存却持久化失败。消除 probe 完成路径的反向持锁（例如在 DB 事务之前取得拥有所有权的 circuit handle 并立即释放 manager），全面核对本次涉及的 manager/circuit/config/health/backlog/inner/DB 顺序，不能只是把反向等待移到另一个锁。保持现有重置行为，控制层作为只读依据；在原允许 runtime 测试中调用真实重置路径做并发回归，证明没有 DB busy failure/manager 阻塞导致操作失败。
2. **P2 OAuth 最后检查与 commit 竞态**：queries.rs 的 budget.checkpoint("oauth_write") 返回后、tx.commit 之前可发生 timeout 或 begin_mutation 失效。probe 专用入口没有 mutation guard，所以 current AtomicBool 的单次读取不足。为 OAuth CAS 设一个与 timeout/配置失效互斥或原子排序的明确接受点，覆盖最后校验之后的窗口。已经在有效代际/预算内同步接受的更新可按原 DB 事务顺序完成；失效先发生的更新必须回滚，不能再提交。不要用持全局协调锁或供应商准入 gate 跨 SQL commit 换取排序从而破坏 60 秒释放。补接受前、接受后到 commit 前两侧的可控竞态，明确 accepted-before-invalidation 与 invalidated-before-acceptance 的不同结果，并验证新 flight/配置写入的读取与 CAS 顺序；不能仅在检查前暂停。保持正常 OAuth refresh/CAS 原行为，不复制逻辑。
3. **P2 超时未进入统一失败消费**：共享 owner 的 timeout 分支只 complete_flight，未写失败时间线且未消费半开失败。半开已有 2 次成功，本次实际超时必须打断成功累积，不能下一次单次成功就 close。等待方/flight 按 60 秒及时释放；当前配置代际的 timeout 失败事实必须进入现有时间线和 circuit/recovery 消费，不能在该分支默默跳过。DB/锁阻塞时允许已接受失败的持久化稍后完成，但必须保留与同供应商随后成功之间的顺序，不能迟到失败反过来污染新配置或覆盖后续已接受状态。沿用单一结果消费，不新建一般事件框架/常驻 watcher/数据库迁移；使用最小的 per-provider/flight 已有状态实现必要顺序。移除“超时观察数恒为零”的错误断言，新增 half-open 两次成功 -> 真实60秒超时 -> 下一次成功仍不错误关闭、失败观察恰好一次、配置变更隔离、超时后新 flight 准入、旧成功无消费回归。配置加载前超时没有模型元数据也不能产生虚构成功，按可确认的 provider/generation 处理失败事实。
4. **P2 测试门禁错误**：shared/circuit_breaker/tests.rs 中 apply() -> Option<bool> 不可直接丢弃，按意图断言或显式 let _；app/provider_availability_probe_runtime.rs 新共享超时环境变量测试的同步环境 guard 跨 await，按项目现有 scoped env test 模式精确添加 clippy::await_holding_lock 处理或缩短锁范围。不得广泛屏蔽模块 warnings。静态核对本轮所有相同新写法，避免 CI -D warnings 失败。

范围继承 r8-exec4 的允许模块/函数与必要测试，不新增控制层行为。设计围绕真实可证明的同步接受点、锁顺序及失败顺序，不增加一般事务/取消框架；原 60 秒共享预算、generation/recovery epoch、每 flight 身份和旧结果隔离都保留。重点回归只编写，禁止本地执行。完成后消融、git diff --check、分项中文 Conventional Commit，progress 记清以上四项证据及实际限制后停止。不要把已生成字段/lock/fmt 或 CI 未完成当作本轮新源码 bug 已修。

### r8-exec4 当前现场与最小范围补齐

上一 writer 已停止，HEAD 仍为 `5a167826c921a32c3f0ee513b55b4b5036dd6d3f`。当前未提交的 5 个产品文件（observer/{mod,snapshot,activity}.rs、provider_account_usage_runtime.rs、commands/providers/account_usage.rs）是 r8-exec3 的有效工作草稿，完整保留并接续，不撤销、不从头重做；详情在 progress 的 r8-exec3 部分。先接通下列泛型边界完成实际 observer 路由回归并独立提交，再完成预算实现与回归。当前草稿已知因固定 Wry 下游无法编译，不能直接按完成提交。

在 r8-exec3 全部允许范围基础上，追加 main 已核实的原验收范围遗漏：

- `src-tauri/src/domain/cli_sessions/mod.rs::folder_lookup_by_ids`；`src-tauri/src/domain/cli_sessions/claude.rs::{folder_lookup_by_session_ids,claude_projects_dir,home_dir}`；`src-tauri/src/domain/cli_sessions/codex.rs::{folder_lookup_by_session_ids,scan_all_session_files}`。只做 6 个函数的 Runtime 类型参数传递，最终 app_paths/codex_paths 已泛型，不改扫描/路径/WSL/文件访问行为。observer 内保留 Wry 别名兼容 plugin_registry，不修改无关应用装配。
- `src-tauri/src/shared/circuit_breaker.rs`、`shared/circuit_breaker/types.rs` 及既有 `shared/circuit_breaker/tests.rs`：仅实现本次 probe 专属的锁取得后 flight/代际/deadline 校验与短内存提交入口及必要回归。复用已有 should_allow/record_success/record_failure 的算法，可将真正需要复用的短内存计算提为私有 helper，原入口继续调用，禁止复制熔断算法或重构全部锁体系。config/health 等锁取得后，变更之前重新检查本次有效性；closed circuit 仍不能消费 probe 清除失败历史，半开门槛/事件/持久化语义不变。
- `src-tauri/src/gateway/proxy/provider_router.rs` 或其现有模块文件中与 `record_success_and_emit_transition` / `record_failure_and_emit_transition` / `emit_circuit_transition` 直接有关的必要内部可见性或“已计算变更”事件复用，限于确需避免重复获取 circuit 锁时使用；不修改路由选择、fallback、计费或普通请求转发。若现有 gateway/runtime.rs 即可完成则不触碰。

以上补充实现原 PLAN 的完整预算及完整真实路径验证，无用户行为/公共协议/预算/授权变更，无需再次申请施工。不要因为需要在这些限定模块内新增一个局部私有函数或等价步骤再交回；仅当真正越过这些行为边界或现有机制无法完成时报告实际新事实。

完整预算不能只加一个表面 timeout。使用同一个单调 deadline，由共享任务拥有结果与超时协调，所有同步配置/认证/SQL 工作走现有 blocking，网络保持异步；全局 inner 不跨 DB 或潜在同步锁等待。每次 flight 独立身份/完成状态，旧 flight 超时后准入可恢复且迟到代码不能清理新 flight。同一配置 generation 未变化也必须隔离连续测试。恢复证据在实际取得内部锁后验证有效性；不要以“调用前已检查”代替。可在已有短内存临界区把有效完成与结果发布线性化，事件/持久化工作不占协调锁；明确记录同步 SQL 无法强制中断的实际边界，不能承诺中断 SQLite。保留配置失效排序，晚到成功不能污染新配置或让超时显示成功。实现最小有效办法并进行消融，不扩展通用取消/事务框架。

实际超时回归须涵盖共享入口的预算耗尽、所有等待方收到失败、同供应商再次准入，以及释放旧阻塞任务后新 flight/恢复状态不受影响；现有 OAuth CAS、同供应商合并、generation/recovery epoch 用例继续成立。路由草稿补齐后重点静态核对 MockRuntime 类型、paused-time 与跨 runtime 任务调度，不将尚未执行的回归记为通过。本地依然禁止运行测试/Cargo/生成器，最终只运行已批准 git diff --check 并提交完整范围内修改。

### r8-exec3 当前工作及新增必要范围

r8-exec2 writer 已停止，当前 HEAD `5a167826c921a32c3f0ee513b55b4b5036dd6d3f`。其四个提交实现 Gemini 角色、中文错误、入口成功矩阵和活动自动到期/工作释放回归，全部保留，事实在本 worktree progress。当前只需完成下列尚未关闭部分，避免重做已完成项：

- Finding 1 完整 60 秒预算、超时交付与共享协调状态释放，以及与其对应的真实截止/再次准入/迟到隔离回归。
- Finding 4 使用现有 MockRuntime 的实际 observer handler/路由生命周期回归，覆盖原交接的认证/参数失败不续、缓存/各视图续期、测试工作引用释放、停止/异常清理。

main 已核对前轮范围阻塞，按原目标所需最小范围补充如下；这些补充优先于下文旧版“仅复用/审查”边界：

1. `src-tauri/src/domain/providers/queries.rs::resolve_effective_transport_credential_inner`、其同功能入口和必要同文件回归，及 `domain/providers/mod.rs` 的必要导出。将同步 OAuth 详情读取/refresh 后 CAS 写入通过现有 blocking 能力分离；保持既有刷新、CAS 与配置失效排序，不复制 OAuth 刷新、不 block_on 网络占用 blocking 槽。domain/provider_availability.rs 自己的额外 OAuth 详情读取也要异步隔离。普通调用的认证与刷新语义保持不变。
2. `src-tauri/src/app/provider_account_usage_runtime.rs` 中从 touch_tui/touch 到 run_scheduler/perform_refresh 的内部 AppHandle Runtime 参数，以及 `src-tauri/src/commands/providers/account_usage.rs::fetch_account_usage_uncached`：仅允许最小泛型传递使既有真实 observer 路径可接受 MockRuntime，及必要相关回归。ensure_db_ready 已有 Runtime 泛型；现有 IPC 命令保持 Wry 签名，不更改账户调度、配置、租约、网络或缓存语义。不要引入假 handler、cfg(test) 绕开账户用量真实路径或通用测试服务。observer/activity.rs 的外层 Activity/Work 可按相同方式最小泛型化。
3. `src-tauri/src/app/gateway_state.rs` 必要的 gateway 访问函数、`src-tauri/src/gateway/runtime.rs::record_availability_probe_outcome` 及同文件直接内部调用边界：仅在完整共享预算要求确有需要时处理 manager 锁等待或证据准备/提交边界，不修改熔断算法、普通转发、事件语义或扩大为锁体系重构。优先在原 runtime 范围内使用已有能力；额外位置非必须则不改。

预算修复关键约束：把单调 deadline 从共享 Lead 工作起始贯穿配置加载、认证/生成、必要证据处理和完成交付；共享 inner 锁只保护短内存更新，不能跨 DB 或潜在长期同步锁等待。已开始同步调用不能强制中断时，其迟到完成必须经过当前 flight/代际/截止判定，不能撤销新 flight、污染新配置或把超时写成成功恢复。仅 generation 无法区分同一配置连续两次测试时，使用现有或最小的每次 flight 标识解决，不能假定一次超时后 generation 必然改变。清理协调状态后再次准入不能仍被旧工作的外围 gate 长期阻塞。模型在准备阶段已确定时保留失败模型信息。不得仅把整个 finish_probe 放进阻塞任务后无条件发布迟到成功。

测试在实际共享 probe 路径用已有可控时间/范围内测试注入模拟配置、OAuth 读、完成阶段阻塞，证明 waiters 及时得到超时、flight 清理、后续再测，以及旧任务完成不会清理/恢复新 flight。无需本地执行。对配置并发、恢复 epoch、同供应商合并保留原断言；不降低完成判定和预算。路由回归继续直接调用实际 handler/路由构造与真实状态，避免为了测试泛型化无关应用模块。

原四项完整要求如下，已完成部分只按新改动必要调整：

原 writer 已结束；原有五个任务提交完整保留，本轮从 `2e7e3745ee2b974600037b88abcb195c45ce1467` 继续。下文全部原行为、范围、验收与本地执行限制不变。只处理以下集中 findings，并更新 progress、分项本地提交后停止；不要重复全量实现。

1. **完整 60 秒预算与协调释放**：domain/provider_availability.rs 的超时 future 内还有同步 OAuth DB 读取；app/provider_availability_probe_runtime.rs 的 finish_probe 在共享 inner 锁内同步写时间线、读取供应商和消费恢复证据，且这些工作在预算外。将同步 DB 工作与异步协调分离，确保从共享实际工作加载开始至结果/超时交付和协调状态释放都受现有 60 秒预算控制。不得跨同步数据库工作长期持有全局协调锁，不得靠在仍有同步阻塞的 future 外套 timeout 声称解决。保留同供应商合并、generation/recovery epoch、配置失效排序，迟到任务不能消费成功恢复证据或清理新一代 flight。使用现有 blocking/任务能力，不能扩展通用框架、专用线程池或放宽预算；即使同步 SQL 本身不能强制取消，也要保证它的迟到结果与已完成协调状态隔离。模型已知的超时结果保留模型证据。若必须修改原允许文件之外的实现才能完成，记录证据返回 main，不自行越界。
2. **Gemini 回答角色**：protocol_bridge/probe.rs 的 gemini_text 只接受明确 model 角色或原兼容允许的缺省角色。明确 user/其他角色必须失败，JSON/SSE 均加负例，不能把用户文本计为回答。
3. **中文失败原因**：固定脱敏错误代码与简短中文解释一并经现有 error 传递，或复用现有适当展示机制统一映射，保持现有消费者一致。覆盖无有效回答（包括 100 token 仅 reasoning）、未结束、读取失败、64 KiB 超限、超时、HTTP/认证、模型配额、结构和上游错误；不记录原始响应全文，不把一次无回答称永久不可用。保持现有 DTO 兼容，不为文案添加新公共字段。
4. **完整入口回归**：补 API Key/OAuth/固定桥接经真实探测入口到 mock 上游的成功路径，断言认证/包装、模型及最终 wire 100 token（Codex OAuth cap 移除单独断言）。现有直接调用适配器的回归不能替代入口证据。共享超时测试应实际触发截止时间，验证 waiters 收到超时、flight 释放、下次可准入及迟到完成不污染，而非直接向 finish_probe 注入错误。macOS 活动到期测试不得在到期后主动调用 sync 来完成撤销；由到期任务自己驱动并断言 begin/end。补 observer 路由生命周期接入的必要回归，覆盖认证/参数失败不续、缓存/各视图续期、手动测试工作引用释放、停止/异常等，按现有测试方式避免建立额外通用测试框架。

本轮本地仅运行已允许的静态命令，不运行新增回归。Rust 格式/Cargo.lock/bindings 保持云端生成约定，准确记录未完成项。main 后续只复查本轮受影响部分及已有 findings 的关闭证据，不重复全量审查。

## 职责与边界

读取本交接、适用 AGENTS 和实现需要的源码/合同；无需读取主工作树 PLAN 或其他历史任务。按下列范围完整实施三项工作，遇材料性范围变化时记录并返回 main；不得降低验收标准以制造通过。你不是独自在代码库工作，不撤销他人编辑，只有本 worktree 是你的写入范围。不得派生子代理。

仅本地允许只读文件/Git 操作、`git diff --check`、手动代码编辑及本任务分项 Git 提交。不安装依赖、不运行 package-manager、开发服务器、lint、类型检查、测试、构建、Cargo、Tauri、签名、打包、Node 测试脚本。所有前端/Rust 测试及 bindings/lock 生成由 GitHub Actions 执行。你可编写必要回归但不得本地执行。不访问真实用户配置/账本，不发真实上游请求、不操作运行中的 AIO/TUI、不更改系统偏好。不记录真实凭据、对话、全量日志或用户金额。

进度写执行 worktree `.gkd/progress.md`，在重要判断、阶段完成、阻塞或验证事实时更新。实施完成做消融审查和 `git diff --check`，三个问题分别使用简短中文 Conventional Commit，例如 `fix(供应商): 保留停用成员未修改路由`、`fix(TUI): 修复后台供应商快照超时`、`fix(供应商): 使用真实生成验证可用性`。只提交明确的产品文件；execution/progress 保留为活动 Markdown，不混入产品提交。完成后汇报提交 SHA、变更范围、已有静态验证、未跑验证、必要风险并停止。不要验收、推送、创建 PR、合并、发布或清理工作树。

## 现场证据

- 两端安装版为 0.60.58，产品代码与 baseline 对齐。AIO 隐藏到托盘仍运行，长时间供应商页/TUI 供应商视图会异常；用户已验证从托盘将 AIO 置前即立即恢复，不必切首页。
- 连续两段故障响应中包含供应商的快照所有 DB 分区同时 unavailable，内存 activeRequests 仍 available；非缓存生成耗时 1503-1508 ms 贴合 1500 ms DB 预算，且出现 429 OBS_BUSY。不包含供应商的快照仍成功，DB 耗时约 1130-1280 ms。
- 同期进程 suppressed=true；独立 SQLite 进程仍正常查询，约 22.7 万行历史账本、5 个配置限额供应商。候选和详情重复计算同一费用汇总，每次约 226-287 ms。先过滤供应商再物化必要费用字段的 SQL 实验与原 5 行结果一致，241 ms 降至 133 ms；Node SQLite 版本不同，不能当作 Rust 性能通过证明。
- 页面账户用量每 5 秒后台轮询，命中 runtime 缓存前仍查询配置；隐藏保留页面，切首页卸载；网络 await 不跨 DB 连接/runtime 锁，未证明死锁。
- 停用来源成员未改 cross_policy 时，前端把后端 null 初始化为空对象并提交，后端拒绝与原值不一致；普通配置先保存会造成局部成功假象。
- 当前真实探测判定 status<500 且非鉴权错误，部分模型错误/429 被记为成功；响应读错转为空体，1 token 且不验证回答。

## 一、TUI 快照与后台活动

### A. 费用只算一次

在 collect_db_projection 按所需范围一次加载 provider_limit_usage::list_v1，load_provider_candidates/load_provider_observations 消费同一结果。固定 CLI 含详情只算该 CLI 一次；无详情只算候选需要范围；all 含详情算全范围一次并按 ID 分配。候选不能受详情 512 行上限限制。只请求内复用，不跨请求缓存。费用失败必须让原本受影响分区明确 unavailable，不能视为零费用/无限额成功。保留候选资格、金额、窗口和排序语义。

### B. SQL

aggregate_costs_for_providers 保留候选窗口 CTE；先按该批 ID 筛选 usage_events，仅物化 final_provider_id、created_at、cost_usd_femto 等必要字段及现行合法统计行，再 LEFT JOIN 汇总。保留无费用零值、5h/日/周/月/总额、兼容/完成回填视图语义，不截断总额历史、不绕视图、不迁移/建索引。回归在 Rust bundled SQLite 验证结果等价及减少宽表/无关供应商物化的查询计划，不写固定机器毫秒断言冒充性能证据。

### C. 隐藏暂停展示轮询

useProviderAccountUsageQuery 复用 useDocumentVisibility；调用方启用、配置有效、文档可见时每 5 秒正常刷新，移除强制后台轮询。恢复可见补一次普通缓存读取，不 force 上游。可见但未聚焦仍正常刷新。隐藏/卸载不再续桌面 15 秒租约；TUI 租约、现有账户用量调度及定时可用性测试保持；允许在途共享请求完成，不取消或清空。

### D. 过期工作与诊断

保持 DB 1500 ms、permit 1600 ms、快照 HTTP 3500 ms 预算，不能加超时/重试/全局并发数。单调截止时间带入 blocking 闭包，在开始及主要阶段边界检查，过期不启动后续查询。正在执行的 SQL 返回后才释放 permit，不冒充可强制中断、不增加专用线程池或 watcher。记录 permit 等待、blocking 排队、阶段及总耗时，区分 busy/截止时间/blocking/局部 SQL 错误；仅错误或超过预算 75% 的慢投影输出。只记录阶段、CLI 范围、是否含详情、数量、耗时、错误代码。整体失败仍 unavailable，不能用空数据/旧缓存伪成功。

### E. macOS 活动

- observer 专用小模块 activity.rs，使用 NSProcessInfo beginActivity / endActivity，选项 UserInitiatedAllowingIdleSystemSleep，固定 reason `AIO TUI observation`；不阻止系统/屏幕休眠、不全局关闭 App Nap。
- 启动不自动建立活动；认证/参数校验后的有效 snapshot 在缓存与 DB 排队前申请/续 15 秒租约。覆盖所有视图/CLI/缓存/后续 busy 或 unavailable；不能等查询成功才续。health/无效认证参数/未知路由不续。
- 多 TUI 共一个 token，最后有效读取决定到期。手动测试通过原有校验/准入后持有同一活动的工作引用，结束/失败/超时/取消释放；无 snapshot 租约且无工作时 end。仅有测试最多本交接 65 秒 observer 端点预算，定时测试不新增用户租约。
- 一个 observer 所有、可重置的到期等待任务，使用单调时钟；不每个请求派常驻任务，无活动不轮询。状态串行化，旧到期不能释放新租约，begin/end 成对且最多一个 token。
- 原生对象按 Objective-C 线程/类型要求持有和释放，必要时复用 Tauri 主线程调度；不裸指针/unsafe impl Send 绕过约束，不占 DB/blocking 配额。失败记录错误代码，不伪报活动成功。
- observer 停止、异常结束、启动取消都关闭控制器、结束活动和任务，迟到请求不能重开。Windows/Linux 无原生活动及新增定时任务。诊断只在 begin/end 变化输出。
- macOS 条件直接依赖 objc2 0.6.3 / objc2-foundation 0.3.2 与最小 features，锁里已有版本，禁止无关升级。lock 归属和格式由既有云端 canonicalize 核对，不本地运行 Cargo。
- 官方依据：https://developer.apple.com/library/archive/documentation/Performance/Conceptual/power_efficiency_guidelines_osx/PrioritizeWorkAtTheAppLevel.html 及 https://developer.apple.com/documentation/foundation/processinfo/activityoptions/userinitiatedallowingidlesystemsleep 。

### 范围与回归

允许：src-tauri/src/app/observer/{snapshot.rs,mod.rs,activity.rs(new)}；src-tauri/src/domain/provider_limit_usage.rs；src/query/providers.ts；src/query/__tests__/providers.test.tsx；必要时 src/components/providers/__tests__/ProviderAccountUsageInline.test.tsx；src-tauri/{Cargo.toml,Cargo.lock}；.github/workflows/ci.yml；scripts/check-ci-quality-gates{,.selftest}.mjs；.trellis/spec/aio-coding-hub/cross-layer/{local-observer-tui-contract,provider-account-usage-query-contract}.md。

useDocumentVisibility、shared/blocking、provider_account_usage_runtime、视图迁移仅复用/审查，不计划修改。覆盖固定/all/空/无详情/截断范围、费用错误、完整/兼容回填、时间边界/多 CLI/批次/失败与排除记录；隐藏/恢复和 TUI 独立续期；可控延迟过期停止/permit 释放；activity 首次/缓存/多客户端/到期竞态/长测试引用/取消/停止异常/迟到事件/无效请求不续。断言 begin/end 数量与原生 macOS 可调用。

新增自动 CI observer-macos job，随 rust_ci=true，在项目现有 macOS runner/toolchain 上运行 `cargo test --manifest-path src-tauri/Cargo.toml --locked --lib app::observer::activity -- --test-threads=1`，30 分钟超时，不签名打包；ci-gate 需要 Rust 时要求成功，否则 skipped；同步门禁合同与负例，不只换摘要/删旧断言。

## 二、停用成员普通保存

跨供应商草稿未修改时 saveRoutingPolicies 提交后端视图原始 cross_policy，保留 null/显式空对象/非空规则；只有真实脏草稿提交编辑对象。保留后端 UUID/revision/事务与停用来源禁止改 cross 策略、目标资格校验。复核状态刷新/切方案的草稿基线；不重构全保存事务、不放宽 sort_modes.rs。停用后的脏草稿不能静默丢弃或伪报成功。

允许：src/pages/providers/useProviderEditorForm.ts；src/pages/providers/__tests__/ProviderRoutingEditor.test.tsx；.trellis/spec/aio-coding-hub/cross-layer/configured-model-routing-contract.md。sort_modes.rs 仅审查依据。

回归覆盖全局启用+方案停用+null 普通字段保存成功且仍 null；显式空/非空保留；脏草稿不静默丢；Default/非成员/启用成员正常；启用成员真改按草稿；旧 revision 并发错误仍保留。

## 三、真实模型测试

### 请求约定

- 只测试指定供应商与模型，绕过普通/跨供应商/方案路由，不 failover。停用供应商/成员仍可手动测，不自动启用。
- 固定提示 `Reply with the single word OK.`，不带用户数据/工具；一次最多一个模型生成请求，不失败重试/换接口/模型。必要 OAuth 刷新沿用现有逻辑。
- Codex API Key 用 /v1/responses 非流式，模型优先 provider override > global setting > 当前默认；Codex OAuth 复用 ChatGPT /responses、账户头、store=false、stream=true 兼容。
- Claude /v1/messages 现有 claude-sonnet-4-6；Gemini API Key 当前 gemini-2.0-flash:generateContent；Grok effective preferences model/backend。此次不新增其他 CLI 模型设置或动态目录挑选。
- 各 OAuth 复用正确适配头/请求响应包装；Gemini Code Assist 项目包/解包；不能简单把 OAuth 当 API Key。
- 固定桥接只访问静态绑定源，用桥接 model mapping，记录输入/上游实际请求模型，按目标原始协议验证。无固定源的动态桥接明确失败，不走本机动态网关。
- 支持 token 上限的最终 wire 请求为 100 输出 token（含协议自定的 reasoning 计数），不主动开额外思考、不自动提预算。只有 reasoning 没回答失败，不能称永久不可用。Codex OAuth 兼容移除上限是已说明的例外，不能伪报 100 硬上限或用字符估算；受单请求/短输入/时间/体积上限约束。
- connect=8s，HTTP 总时长=45s 包括读取；共享实际测试工作从加载到结果最多 60s，超时释放协调状态；observer 等待=65s、TUI=协议+1s，快照 3500ms 不变。延迟直到最终结束而非响应头。
- 响应最大 64KiB，区分刚好完整与截断，读错不转空成功。不修改定时开关/频率/并发合并，不自行发真实请求。

### 严格判定（r8）

要求 2xx、结构正确、正确回答字段非空白文本、明确正常终态或仅 token 上限终态，无其他错误。不做额外语义评分，不要求逐字 OK。thinking、usage、心跳、错误字段、tool 参数不算回答。

| 协议 | 接受的终态（均需已有有效回答） |
| --- | --- |
| Responses JSON/SSE | completed，或 incomplete 原因为输出 token 上限；SSE 必须收到对应响应的终态 |
| Chat JSON | 同一 choice 非空白 assistant 文本 + finish_reason stop 或 length |
| Chat SSE | 同一 choice 文本 + stop/length + 终止标记，只有 DONE 不够 |
| Anthropic | 合法 text block + 正常 stop reason 或 max_tokens，SSE 还需 message_stop |
| Gemini | 同一 candidate 非空白非 thought 文本 + STOP/MAX_TOKENS，OAuth 先解包 |

非流式完整读完解析；流式复用现有分帧，在合法终态结束。不要用转译器补的默认 stop 证明上游终态。无效结构/200 error或HTML/4xx（含429）/5xx/空/只有reasoning或工具/未知 incomplete/损坏/提前EOF/先文本后错误/本地64KiB截断/超时全失败。仅 token 限额结束且有回答可成功。错误至少区分 HTTP/认证/模型配额/无文本/结构/未结束/读取/超限/超时，不记原文日志，保留现有受限脱敏预览。

### 结果及消费

ProviderAvailabilityResult 与 observer DTO 增可选 requested_model、tested_model（选择输入与实际 wire 模型），observer camelCase，缺字段兼容旧协议不升版本。准备前失败可无模型，后续失败保留模型。桌面/TUI 成败都显示实际模型，映射时显示输入与目标；旧服务器无值不猜。上游别名可不同版本，不字符串硬判伪模型；不能保证第三方内部身份。

桌面/TUI/定时/恢复共一个严格结果，现有时间线形态不变不迁移历史；失败不变成功恢复证据。配置变化 generation/recovery epoch 仍阻旧结果回写，保留同供应商合并。只测供应商指绕过路由，不删除既有时间线/恢复消费者。

允许：src-tauri/src/domain/provider_availability.rs；src-tauri/src/gateway.rs；src-tauri/src/gateway/proxy/mod.rs；src-tauri/src/gateway/proxy/protocol_bridge/；src-tauri/src/gateway/proxy/gemini_oauth.rs；src-tauri/src/gateway/proxy/handler/failover_loop/prepare/codex_chatgpt.rs；src-tauri/src/app/provider_availability_probe_runtime.rs；src-tauri/src/commands/provider_availability.rs；src-tauri/src/app/observer/mod.rs；src-tauri/crates/aio-observer-protocol/src/lib.rs；src-tauri/crates/aio-tui/src/{client,ui}.rs；src/pages/providers/hooks/useProvidersViewDataModel.ts；src/pages/providers/__tests__/ProvidersView.test.tsx；src/services/providers/__tests__/providers.contract.test.ts；src/generated/bindings.ts；.trellis/spec/aio-coding-hub/cross-layer/local-observer-tui-contract.md。

协议能力仅做必要复用/可见性/纯函数移动，不改普通网关转发语义，不造通用引擎。生成 bindings 仅云端，若当前不能生成，记录准确类型变化交 main 获取 canonicalize patch，不能冒充已生成。

回归：所有认证/固定桥接 wire 请求、100预算/特殊移除、目标身份/无路由替代、上述成功失败矩阵、SSE chunk/UTF8拆分/心跳/终态/先文本后错/有无回答的token终态；定时/手动/恢复统一、并发合并、配置竞态、超时后再测；TUI预算与模型显示/旧DTO兼容。使用已有 mock 上游，禁止真实供应商来造故障。

## 验证与终点

本地仅 `git diff --check` 与只读源码/Git核对，不运行任何测试或构建。CI 应执行现有 `pnpm test:unit:coverage`、`cargo test --workspace --locked -- --test-threads=1`、上述 macOS 专项与门禁/格式/bindings合同；这些由后续获准 PR 自动触发，不由你启动。CI 未运行要明确。

后续 main 需独立验收后处理 CI/交付权限；你不要把静态审查当 runtime 成功。本机待新版本后验收：AIO 供应商页可见、供应商页后台、托盘三个至少30分钟正常节奏观察，各视图持续刷新且无1500ms超时/OBS_BUSY，分开列表/账户状态；活动建立/15s释放与正常休眠证据；用户主动用真实模型验证一次成功。任何未完成现场项保留待验证，不操作用户应用来完成这些步骤。

最终回复以事实为准：分项 commits、文件范围、静态命令和结果、未执行的 CI/runtime、必要返工/超范围事项、消融结果，然后停止等待 main。
