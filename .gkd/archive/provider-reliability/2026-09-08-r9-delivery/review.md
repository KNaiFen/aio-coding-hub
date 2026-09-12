> 归档快照：2026-09-08，来源 主工作树 .gkd/review.md。原作者决定和各轮观察时点保留；本轮后续交付事实见 [summary.md](summary.md)。个人路径与会话角色句柄已改为逻辑标识。

# 供应商可靠性修复审查

> 本地收尾记录（2026-09-07）：完整脱敏快照与待续事项已保存到 [归档摘要](../2026-09-07-r8/summary.md)。原执行/复查角色已停止；下方正文按原作者与观察时点保留，不构成新施工交接。
> 最终已审 HEAD 为 `7ab1b06de258434145e5748559f5739005cecfbd`，SOURCE STATIC PASS；云端 canonicalize、CI、macOS 与新版本现场验证仍未完成。任务分支/worktree 和 18 个本地提交保留；后续外部交付与清理仍按 main 已有许可边界决定。
> 本段由收尾角色添加；原 PLAN、execution、review 决定及 progress 各轮事实不变。必要验证未结束，活动正文继续保留。

- 当前结论：PASS，允许按r9既有授权合并和发版。最终HEAD `845589f8bf72df70022a6c1e7a85132accd4b266` 已完成独立源码复查与全部必要PR CI；main签名候选和Release仍待收尾执行，本机现场验证仍待发布后完成。
- r9外部交付：用户已批准PR/CI/合并/发版。main直接核对六个版本/CHANGELOG文件，提交 `37513933403682f68aad0c135c0ca98220ed8fb1`，任务分支已推送；PR为 https://github.com/KNaiFen/aio-coding-hub/pull/192 。首轮自动CI待定，锁/格式/bindings尚待云端；版本目标0.60.59。该元数据提交不改变已审产品行为。
- 日期：2026-09-08。
- r9 CI：PR #192 head `37513933403682f68aad0c135c0ca98220ed8fb1`，run `34125500658` attempt 1，ci-gate失败/pr-title成功。contracts为新增macOS步单行run不符既有块脚本合同；frontend仅停用后的脏草稿显示断言失败（2865通过/1失败）；rust因生成漂移停止；observer-macos因lock未更新停止。
- main 核对来自该run的cloud-native-fixes，merge SHA `08f465d6a783a035c058f0ed8142976fb6b83be3`的两个父提交为baseline和PR head。补丁限定Rust格式、Cargo.lock三个0.60.59版本/两个macOS直接依赖、bindings两个可选模型字段；git apply --check和git diff --check通过，提交 `aff75018`。没有本地运行生成器或测试。
- r9-exec8 交接只修门禁脚本表示及路由草稿回归失败，保留原行为和成功条件，原writer均停止；运行检查继续由后续PR自动CI证明。
- r9-exec8 writer及独立accept均停止；本轮baseline `aff75018dbb627613717842eca8a03496218bde1`，HEAD `c1c002c5556054ec1b43ff2de22bb1dc863a824b`。提交9d74900b保留块脚本合同和命令缺失负例；c1c002c5确认停用时输入隐藏，保留实际提交/revision、错误/不关闭不成功，并经同成员重新启用refetch证明草稿仍在，生产逻辑未改。main采纳复查结论，三文件消融/静态检查已过，推进新head自动CI。
- 第二轮run `34127583409`，rust job `101759838184`：canonicalize无漂移，clippy在probe runtime尾部needless_return和provider_availability三个fixture的unused_io_amount失败。main读取必要错误及四处准确源码后，r9-exec9只修这两文件的等价控制流/fixture读取，不降低门禁或改变行为。monitor已终止；frontend/observer-macos尚未取得终态，后续继续取得证据。
- 第二轮最终frontend/contracts/observer-macos均成功，rust的四处clippy导致ci-gate失败。r9-exec9提交 `f6bbcc82720c1e75ef03b54a89a1a96de76458c0`，writer及accept均停止；main采纳独立复查无问题结论：正向budget条件与原尾部return等价，三fixture断言实际收到字节且保留全部响应验证；无豁免/门禁变更，git diff --check通过。继续推送并验证新head。
- 第三轮run `34128761259` attempt1，Rust job `101763609700`：canonicalize/clippy成功，Rust tests被1小时job限制取消；frontend/contracts/observer-macos/pr-title成功。必要日志显示snapshot的fixed/all复用与详情截断两项失败，probe runtime两个OAuth竞态回归失败；最后ok shared_deadline_releases...后约40分钟无新结果，按序下一项gateway/circuit等待回归疑似阻塞。main已读准确fixtures，确认第一个存在单连接重复借用，其他待修证据详见r9-exec10；不增加超时、不删断言或放宽门禁。独立monitor已停止。
- r9-exec10 writer/accept均已停止，HEAD `7d19a02e80c5dda9888b447f31b76951da6a296e`；b478b9ed修单连接释放、UUID与route ID，6b3c0dde将两个OAuth mock有效期提升到高于原3600秒刷新窗口，7d19a02e稳定锁回归准入并加测试诊断/失败释放。独立复查无确定问题，main采纳静态通过结论；原真实锁/超时/33轮/失败消费/半开等全部断言保留。仅两文件测试变化，无产品行为变化，静态空白通过；40分钟停顿仍未证明根因，新head必须CI验证。
- 第四轮run `34136912876` attempt1在生成漂移停止，artifact `cloud-native-fixes-75f8e96b9ad7b07892ba5784c573e81e8eb8efe2-1`仅633字节压缩包。main核对merge父提交含7d19a02e，完整两hunk仅catch_unwind闭包fmt换行和等价花括号；git apply --check与diff --check通过，提交 `2a01c75e`，无行为变化。继续新head自动CI，第四轮尚未运行Rust回归。
- 第五轮run `34137431217` attempt1/rust job `101791619602`完整结果2948通过/4失败/4ignored，788.81秒；frontend/macOS/contracts/pr-title/fmt/clippy成功，两个OAuth回归通过。失败：两snapshot候选缺少显式default成员（512!=513、[]!=[1]）；费用兼容分支初始DB仍complete（14!=15，原新SQL等价已过）；锁回归真实manager held时deliver timeout阶段失败。main核对准确fixture/候选/回填初始化与完成消费，r9-exec11限定相应三文件回归及有证据必要的原探测边界；预算/断言/门禁保持。
- r9-exec11 writer及accept停止，HEAD `845589f8bf72df70022a6c1e7a85132accd4b266`，三提交d162f97e/19b60325/845589f8只改测试：显式default成员、incomplete回填态、锁fixture同钟同deadline见证与1ms tick。独立复查无确定问题，main采纳静态通过，原矩阵/次数/真实锁/SQL断言保持；timer边界根因仍为推断，以新head自动CI为准。两次diff --check通过。
- 第六轮自动CI run `34141052395` completed/success，固定HEAD `845589f8bf72df70022a6c1e7a85132accd4b266`；只读monitor观察2026-09-08 00:23:04 +0800，ci-gate/contracts/frontend/observer-macos/pr-title/rust全部SUCCESS，无缺失/失败。main核对该run真实成功、任务产品区清洁、baseline至HEAD diff --check exit0；全部原writer/accept/monitor已停止。此前Rust四项回归已不再阻塞，源码/自动验证验收通过。
- 交付收尾决定：按r9许可启动本新增交付阶段唯一gkd_closeout，绑定上述HEAD squash PR #192，保留main独有历史与任务分支/worktree；等待真实merge SHA的main自动CI及签名候选后发布 `aio-coding-hub-v0.60.59`，核对12类预期制品。允许归档五类活动记录和summary至 `.gkd/archive/provider-reliability/2026-09-08-r9-delivery/`，成功交付后移除已归档的本任务独占活动正文。现场30分钟观察、实际用户测试、安装均未完成且无自动操作许可；不因CI成功宣称现场通过。
- PLAN：r8-plan；最终 execution：r8-exec7；独立技术复查至 r8-exec6，最后单一测试 fixture 差异由 main 针对性复核。
- Baseline：`a35f2aa4ea469f6e4066582b2e969f1ec44fca2e`。
- 最终已审 HEAD：`7ab1b06de258434145e5748559f5739005cecfbd`；首次已审 HEAD：`2e7e3745ee2b974600037b88abcb195c45ce1467`。
- 原 writer 初轮施工角色 已完成并停止；独立 初轮复查角色 已集中返回 findings 并停止。main 已抽查关键实现位置，采纳以下返工项。原范围、行为、验收及授权不变，不新增 PLAN revision。

## 返工项

1. P1：完整共享任务的 60 秒预算未覆盖同步 OAuth 数据库读取，以及 finish_probe 在共享锁内的数据库写入/恢复证据操作。必须确保超时结果交付、协调状态释放与后续准入不被这些同步操作拖住，同时保持 generation/recovery epoch 校验、迟到结果隔离及时间线/恢复一致性。不能仅在 HTTP 外再套 timeout 而保留同步阻塞。
2. P2：Gemini 的 gemini_text 未检查 content.role，明确 role=user 的文字可在 JSON/SSE 中成为成功证据。拒绝明确非 model 角色；缺省 role 的兼容可保留；两种响应路径均补负例。
3. P2：result.error 仅含 PROBE_NO_TEXT 等内部代码。保留固定脱敏代码，提供可理解的简短中文原因，覆盖无有效回答、未完成、读取失败、响应超限、超时及既有认证/HTTP/结构类别。没有回答只代表本次未验证成功，不描述为模型永久不可用。所有现有消费者应得到一致原因。
4. P2：补真正完整入口回归：API Key/OAuth/固定桥接经过探测入口和 mock 上游，断言认证、实际模型、100 token/已知 OAuth 例外及有效成功响应；真实触发共享任务超时后释放与再测；活动租约到期无需主动 sync 即自动撤销，以及有效/无效 snapshot、缓存、手动工作、停止等路由生命周期接入。不得继续通过直接注入 finish_probe 错误或手动 sync 冒充完整路径覆盖。

## 验证及交付现状

- 已有 5 个范围内本地提交；产品工作区清洁，执行 .gkd 材料未跟踪并保留。
- `git diff --check a35f2aa4..HEAD` 已由 writer 通过；独立验收仅静态审查，未重跑测试。
- Rust 格式、Cargo.lock 直接依赖归属与生成 bindings 仍待云端 canonicalize，未生成即不能声称已具备编译通过条件。
- 前端/Rust/macOS CI、三个至少 30 分钟的新版本现场观察、活动撤销/休眠与用户主动真实调用均未执行。
- 当前终点沿用 PLAN：实现、分项本地提交、独立代码验收、可审阅交付材料。未授权推送、PR、合并、发布、安装或删除分支/worktree；不把未完成运行验证记为通过。

## r8-exec2 返回及后续决定

- writer 施工角色 r2 已完成并停止，HEAD `5a167826c921a32c3f0ee513b55b4b5036dd6d3f`。本轮提交 `e9e3c7d5`、`ec439b20`、`5d269c49`、`5a167826`，实现 Gemini 角色和中文错误，补真实入口成功矩阵、活动自动到期/工作释放回归；未运行回归。
- Findings 2/3 待最终针对性核验；finding 4 仅部分补齐。Finding 1 完整预算，以及 finding 4 真实共享超时和 observer 路由集成仍未关闭。
- main 抽查确认：`domain/providers/queries.rs::resolve_effective_transport_credential_inner` 的同步 OAuth 读/CAS 写在原允许范围外；需要移入已有 blocking 调用以使异步预算生效。仅补该函数及必要导出/相关回归，保持原刷新/CAS 行为。
- main 抽查确认：observer 固定 Wry 的调用链经过 `app/provider_account_usage_runtime.rs` 的 touch_tui/touch/scheduler/perform_refresh 和 `commands/providers/account_usage.rs::fetch_account_usage_uncached`；`ensure_db_ready` 已泛型。为使用项目现有 MockRuntime 测完整 handler，允许这些私有/内部函数最小 Runtime 泛型传递；公开 IPC 签名与账户调度/请求行为不变。
- 恢复证据还有 gateway manager 同步锁等待。允许按实际需要在 `app/gateway_state.rs` 的必要访问函数、`gateway/runtime.rs::record_availability_probe_outcome` 及其直接内部调用边界做最小截止时间/锁外准备接入，禁止更改熔断算法、事件或普通转发语义。
- 上述为独立验收揭示的实现范围遗漏，目标/用户行为/接口/验收/授权保持 r8；按 gkd-main 以 r8-exec3 补充执行范围，不重新申请既有施工批准。出现无法在这些边界内满足预算/旧结果隔离的证据再返回 main。

## r8-exec3 返回及后续决定

- writer 施工角色 r3 已停止，HEAD 未变。5 个允许产品文件保留 observer 泛型和真实 handler 回归草稿，未提交；当前有固定 Wry 下游类型未接通，已知不能编译，不能通过验收。
- main 核对必要泛型范围为 `domain/cli_sessions/mod.rs::folder_lookup_by_ids`、`claude.rs::{folder_lookup_by_session_ids,claude_projects_dir,home_dir}`、`codex.rs::{folder_lookup_by_session_ids,scan_all_session_files}`；仅补 Runtime 参数传递，扫描/路径/WSL 行为不变。作为原路由回归接入遗漏纳入 r8-exec4。
- main 抽查 shared circuit 实现确认 snapshot/should_allow/record_success/record_failure 内部同步锁等待发生在调用前校验之后。允许 `shared/circuit_breaker.rs` 及其 types/tests 中本次 probe 专属的最小有效性校验/提交入口和必要已有纯内存逻辑复用，取得 config/health 后、实际修改之前检查当前 flight/deadline；transition/persist 等潜在等待分离，不改变普通转发算法。此为原 60 秒预算与迟到结果隔离要求的必要范围遗漏。
- 继续保持代码审查未通过；预算尚未修改。r8-exec4 继承上一轮草稿及全部 9 个提交，由单 writer 完成两项剩余工作。验证/授权与终点仍按 r8，未增加外部操作许可。

## r8-exec4 独立复查

- writer 施工角色 r4 与 复查角色 r2 均已停止。当前已审 HEAD `7ad71ec2c2a96c43ae86aa4b8c001a3c901fd10a`，11 个提交，产品工作区清洁。
- 原 Finding 2（Gemini 角色）、3（中文原因）源码可关闭；原 Finding 4 入口/自动到期/真实 router 回归已补，仍受下列问题影响。原 Finding 1 不通过。
- P1：finish_probe 先 SQLite IMMEDIATE 再 gateway manager；现有 circuit_reset_provider/cli 在 manager 下写 DB，形成反向等待。移除 DB 写事务跨 manager 等待，核对 circuit/persist 锁顺序并补实际重置并发回归。
- P2：OAuth budget 校验与 tx.commit 之间无失效线性化，可能在过期/新配置失效后提交旧 CAS。为 CAS 接受建立与 timeout/配置失效一致的同步点；补最后校验之后、提交之前的竞态，不能只验证检查前阻塞。
- P2：shared timeout 只 complete_flight，跳过失败观察及半开失败消费。当前超时后半开已有两次成功仍保留，下一次成功可错误关闭；新测试还断言观察数为零。超时必须在当前代际按统一结果消费，失败证据可与持久化异步完成但不能丢弃或晚于后续成功而打乱顺序；旧成功继续隔离，配置改变后不污染新状态。
- P2：shared/circuit_breaker/tests.rs 的 apply() 返回 Option 被丢弃触发 unused_must_use；共享超时回归的环境锁跨 await 未按项目既有模式处理 clippy::await_holding_lock。修正源码，不把 canonicalize 当作修复此类警告。
- main 抽查确认以上四处；r8-exec5 按现有允许范围继续修复和必要回归，不改用户行为、预算或授权。截止前已接受结果、随后最佳努力持久化本身可以保留，但不能据此放行不具备同步接受点的旧 OAuth CAS 或丢弃超时失败。

## r8-exec5 独立复查

- writer 施工角色 r5 与 复查角色 r3 已停止；已审 HEAD `1946498c3b2a34f12d53b4c49c8f4f0ee0ef7149`，15 个提交，产品工作区清洁。
- DB/manager 锁反转与 warnings 源码可关闭；OAuth 接受点有效但读序未完整，超时消费顺序已建立但等待方式占用共享资源，当前仍返工。
- P1：finish_probe 在 blocking::run 获得全局 permit 后同步等待 completion_gate。一个长堵塞供应商连续多轮超时的 owner 会累积占用全局 8-32 槽，影响 observer/配置/其他供应商。把消费者锁等待移到申请 blocking permit 之前，已获得锁的 owned guard 必须随实际闭包结束释放，不能 timeout 取消异步等待就提前放锁。补连续多轮超时的跨供应商可用性回归。
- P2：OAuth CAS 接受后允许稍晚 commit，但新 flight 普通 WAL SELECT 不等旧事务，能读旧已轮换凭据重复 refresh。现有回归在旧事务提交后才放新 oauth_read，避开了实际窗口。生产读取应等待本供应商已接受未提交的写完成，等待在 blocking 配额之外且受新 flight deadline 管理；保留准入释放。回归让新真实读取在旧 commit 前推进，证明不使用旧 token 重新 refresh。
- main 抽查确认两处，继续 r8-exec6；目标、预算、用户行为和授权不变。其余已关闭项不重复改动。

## r8-exec6 独立复查

- writer 施工角色 r6 与 复查角色 r4 均停止；HEAD `d22f88956a90669075bbeb2f348534f728375cd2`，17 个提交，产品工作区清洁。
- 完成锁异步排队源码可关闭；OAuth 实际 transport 读写 gate、owned guard 生命周期/取消/接受点无确定新产品问题，仍有一项测试 fixture 问题。
- P2：新增 accepted_oauth_commit_orders_real_reads_without_blocking_admission_or_slots 使用 init_for_tests 单连接池。旧 CAS 暂停独占唯一连接，新 flight 配置加载无法到达 oauth_read_waiting；回归会等不到通知，后续 WAL 独立读取也不能推进。必须为此类并发回归提供显式多连接 fixture，保留生产普通 SELECT 与 33 轮断言。
- main 已核对 init_for_tests max_size(1)，允许 infra/db/mod.rs 的 cfg(test) 初始化增加显式池容量参数入口（原函数默认1不变）及本任务并发回归的最小接入；生产初始化/配置不改。顺带核对本任务同类持事务暂停测试是否同样依赖独立连接，不扩展到无关测试。r8-exec7 仅修测试设施与回归接入，预算/行为/授权不变。

## 最终源码决定与收尾交接

- 施工角色 r7 已停止；最终提交 `7ab1b06de258434145e5748559f5739005cecfbd`，共18个本地任务提交。main 完整核对本轮两文件 diff：原 init_for_tests 默认1保持，新 cfg(test) init_for_tests_with_pool_size 复用原配置/迁移；两个OAuth竞态fixture为4连接，真实reset为2连接，通知在获取连接后发送。生产代码不变。原测试全部关键断言保留，先前单连接阻断目标窗口的 finding 从源码层关闭。
- 结合历次独立复查与本轮唯一fixture修正，原四项及后续返工 findings 均无剩余已知源码阻塞。main 不重复已经审查的全量代码；此次消融保留只用于真实并发场景的显式测试池容量入口，不修改默认测试池和产品数据库容量。
- main 实际运行 `git diff --check a35f2aa4ea469f6e4066582b2e969f1ec44fca2e..HEAD` exit 0、无输出；实际 Git status 仅执行 execution/progress 未跟踪，产品区清洁。最终diff为40个文件、4348插入/612删除，含实现、合同、CI和必要回归。未运行任何本地测试/构建/Cargo/生成器。
- **源码静态通过不代表可编译交付完成**：Cargo.lock的macOS直接依赖归属、Rust fmt和两个可选模型字段bindings仍待云端canonicalize；当前生成类型未同步，不能宣称前端类型检查通过。全部CI/macOS及现场三个至少30分钟观察、活动释放/休眠、用户主动真实生成未执行。
- 当前获准本地终点已到：实现、分项提交、独立代码验收及可审阅材料；由一个gkd_closeout完成本任务Markdown归档/脱敏和交付摘要。允许归档到主工作树 `.gkd/archive/provider-reliability/2026-09-07-r8/`，保留待验证/权限边界，整理本任务活动记录为简短可继续入口或按worker准则移出已结束正文。
- 仍无推送/PR/合并/发布/安装或分支worktree删除许可，不启动这些动作，不新增CI等待目标。保留执行worktree和分支及原18个提交，主工作树独有历史与其他任务现场不动。归档不得包含真实用户数据、凭据、绝对个人路径或完整对话；使用逻辑工作树名及脱敏事实。归档/整理仅本任务拥有的Markdown，不能将主工作树文档混入执行产品分支提交。
