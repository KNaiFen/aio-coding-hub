# 供应商可靠性修复本地收尾摘要

- 归档时点：2026-09-07 20:58:18 +08:00；任务 provider-reliability；PLAN r8-plan；execution r8-exec7。
- 终点：实现、18 个分项本地提交、独立技术复查与 main 最终源码决定、可审阅材料已完成。SOURCE STATIC PASS；生成、编译、CI、运行和外部交付未完成。
- Baseline：`a35f2aa4ea469f6e4066582b2e969f1ec44fca2e`。
- 最终已审 HEAD：`7ab1b06de258434145e5748559f5739005cecfbd`；分支 `fix/provider-reliability`；执行工作树逻辑名 `provider-reliability`。
- 主工作树逻辑名 `main`，HEAD `193767510ef647193ce5f16390bc1f663c3dffb0`；本地独有历史按原现场保留，未变更。
- 原执行与独立复查角色均已停止。最后一次独立复查仅剩并发数据库 fixture finding；main 完整核对最终两个文件的 cfg(test) 差异后，从源码层关闭该项。收尾角色只归档已有事实，不另作验收。

## 保存材料

[plan.md](plan.md) 保存需求、范围、非目标、验收与授权；[execution.md](execution.md) 保存 r8-exec7 及其继承边界；[progress.md](progress.md) 保存 r8-exec1 至 r8-exec7 各轮实际作者事实；[review.md](review.md) 保存集中 findings、返工决定和 main 最终源码结论。本轮实际没有 plan-changes，未创建空文件。

归档位于主工作树 `.gkd/archive/provider-reliability/2026-09-07-r8/`。正文不是链接替身，后续可独立读取。个人绝对路径替换为逻辑工作树名，执行角色句柄替换为角色/轮次；未保存凭据、用户响应正文、完整对话或全量日志。源记录中已脱敏的采样耗时、数量和源码证据保留，不追加访问用户数据。

四份活动记录保留正文，并添加归档入口和待续状态：主工作树 `.gkd/{plan,review}.md`、执行工作树 `.gkd/{execution,progress}.md`。必要生成与运行验证未结束，保留正文符合 worker 的材料保留要求；没有删除活动记录或其他任务材料。归档和活动材料保持本地未跟踪 Markdown，没有新增提交，不混入 main 独有规则历史或产品提交。

## 完成事实及依据

- 三项实现已提交：停用方案成员保存时保留未修改的跨供应商配置；TUI 单次费用汇总、SQL 缩小扫描、隐藏展示轮询暂停、超时分阶段收束及 macOS 活动租约；指定供应商/固定源的真实模型请求、严格回答与终态判断、模型展示和统一失败消费。
- 预算及并发返工已提交：共享 60 秒结果交付/准入释放、迟到结果隔离、完成与重置锁顺序、OAuth 接受及凭据读取排序、超时失败先于后续成功消费，以及必要并发测试 fixture。
- main 在最终 HEAD 实际执行 `git diff --check a35f2aa4ea469f6e4066582b2e969f1ec44fca2e..HEAD`，exit 0、无输出。最终已审差异为 40 文件、4348 插入/612 删除；这是 main 的静态审查记录，收尾不重跑源码验收。
- 收尾只读 Git 核对确认最终 HEAD、18 个中文提交和两个工作树现场：执行产品区清洁，仅 execution/progress 未跟踪；主工作树仅 plan/review 未跟踪，main 与 origin/main 保留原 ahead 26 / behind 1 现场。未用历史数量推断未发布功能。
- 新增回归仅已编写并静态核对，未执行。本地没有运行依赖安装、Node、package-manager、Cargo、fmt、lint、类型检查、测试、构建、生成器、开发服务器、签名或打包。

## 必须继续的验证

1. 由 main 在获得后续交付授权并明确 GitHub 目标后，按项目既有云端 canonicalize 处理 Rust fmt、Cargo.lock 的 macOS 直接依赖归属和 `src/generated/bindings.ts`；`ProviderAvailabilityResult` 的 `requested_model`、`tested_model` 均为可选字段，预期 TS 为 `requested_model?: string | null`、`tested_model?: string | null`。当前生成类型未同步，不能宣称已可编译或前端类型检查通过。
2. 对 canonicalize 后的真实最终 HEAD 执行必要自动 PR 门禁：前端 coverage、workspace Rust/clippy、格式/bindings/质量门禁及新增 observer-macos 回归，等待 `ci-gate` 和 `pr-title`。普通 PR 不额外手动启动常规 CI，不用旧 HEAD 的结果替代新 HEAD。
3. 具备可运行新版本后，在本机相近数据规模下完成供应商页可见、供应商页后台、托盘三个各至少 30 分钟观察；正常 TUI 轮询，供应商/日志视图切换和刷新正常，分别记录列表及账户状态，验证非缓存投影在 1500 ms 内，无同类 timeout/OBS_BUSY。
4. 核验 macOS 活动建立、TUI 退出且无在途工作后 15 秒撤销，以及系统正常空闲休眠。仅看到前台正常不能替代释放证据。
5. 由用户在新版本主动进行真实模型生成成功验证，以及合法方式临时指定无效模型的失败验证；若现有设置不能临时指定无效模型，则该失败由 CI mock 覆盖。不得为验证自动发送上游请求或修改用户持久配置。

上述项目均未完成，没有 CI/run/PR 目标或证据可供本轮等待。没有预填后续成功；未来事实应来源于 main 的明确授权和新交接、云端生成差异、对应实际 HEAD 的自动 CI 结果及用户新版本现场记录，并按当时范围追加事实。

## 保留边界与风险

- 未授权、未执行：推送、PR、合并、发布、候选安装、分支或 worktree 删除。本次没有新增 GitHub/CI 等待目标；未读取或运行 CI watcher。
- 保留任务分支、执行 worktree 和全部 18 提交，因为成果尚未外部交付且后续验证仍依赖当前源码。保留 main 独有历史及非本任务的 gkd-project-rules worktree，不修改其他任务档案。
- Codex OAuth 兼容层移除输出 token cap，不能承诺服务端 100 token 硬上限；该路径仍受单请求、短输入、45 秒 HTTP 与 64 KiB 本地响应边界约束。不能用字符估算或断连推断服务端停止生成。
- 已开始的同步 SQLite/blocking 工作不能被强制中断，已接受 SQL 可能在 60 秒交付后完成；本轮源码通过不证明实际并发、SQLite 性能、MockRuntime/macOS 原生活动及调度回归通过。
- 持续数据库错误时，已接受 timeout 事实可能暂留内存并等待原消费路径持久化，后续成功不能越过该失败；不保证永久错误下能落盘。
- 可继续步骤须由 main 处理生成与交付目标/授权，再对新 HEAD 验证；本归档不扩大原许可，也不将源码静态通过改写为可发布。

## 保留的本地提交

- `0ba20d5f7989e0a9219c6891a1bc1e9ce54b1f14` fix(供应商): 保留停用成员未修改路由
- `21412797fa7fd4fa047cfbc3a96c70a5e11227c3` fix(TUI): 修复后台供应商快照超时
- `09c2b01bcd2b20650fea01242defe03eebd263ce` test(供应商): 明确停用路由回归参数类型
- `493b6a947483f724711a5463866fc62c8950b19e` fix(TUI): 完善活动关闭与快照回归
- `2e7e3745ee2b974600037b88abcb195c45ce1467` fix(供应商): 使用真实生成验证可用性
- `e9e3c7d5217e6bf35ca05b133b02fbc4ead739f7` fix(供应商): 拒绝 Gemini 非模型回答角色
- `ec439b20b38de3490f68ef92c6186fdd4d50adfe` fix(供应商): 统一探测失败中文原因
- `5d269c49322ed5f6dbadb57d7e2e3f4dbe14eca4` test(供应商): 覆盖真实探测入口成功请求
- `5a167826c921a32c3f0ee513b55b4b5036dd6d3f` test(TUI): 验证活动自动到期与工作释放
- `1a6b9977ebf1e9e5138347ce0a2dc0c3ecc76ff3` test(TUI): 接通真实路由活动生命周期回归
- `7ad71ec2c2a96c43ae86aa4b8c001a3c901fd10a` fix(供应商): 贯通共享探测预算并隔离迟到结果
- `961de9ae923109ba57b25ea6361913c85347edc1` test(供应商): 修正探测回归警告门禁
- `ddae5b4965916f44bb9631f519f789874bda4713` fix(供应商): 消除探测完成与重置锁反转
- `3f69b9b721d5de72e5892f0c9e227709426d651b` fix(供应商): 线性化 OAuth 探测刷新接受点
- `1946498c3b2a34f12d53b4c49c8f4f0ee0ef7149` fix(供应商): 按序消费共享探测超时失败
- `2ca187c43e7181ede07854ec97dbed5c5d945671` fix(供应商): 避免完成锁等待占用全局阻塞配额
- `d22f88956a90669075bbeb2f348534f728375cd2` fix(供应商): 排序 OAuth 凭据读取与已接受提交
- `7ab1b06de258434145e5748559f5739005cecfbd` test(供应商): 为并发事务回归提供独立连接
