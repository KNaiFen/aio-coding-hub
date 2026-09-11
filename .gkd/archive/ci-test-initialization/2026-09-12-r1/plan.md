# CI 等待与测试初始化优化 PLAN

- Revision: r1，2026-09-12。
- 状态: r1 已批准，`delegated/automatic` 执行中。
- 当前授权: 用户在计划形成后明确要求“开始执行此PLAN”，批准本计划第 6 节的实现、本地提交、分支推送、PR、正常合并、云端验证、归档及本任务默认清理。此前 plan-only 阶段只保存计划，未实施或触发 CI。
- 目标仓库: `KNaiFen/aio-coding-hub`，目标分支 `main`。
- 远端基线: `b03b5ab4d57df5b39348744d5e65d6cc835fbe1b`，2026-09-12 拟定计划时通过 commits/main API 再次确认。
- 执行路线: `delegated/automatic`。main 已从再次确认的远端基线创建 sibling worktree `../ci-test-initialization`、分支 `ci/test-initialization`；执行交接为该工作树 `.gkd/execution.md` e1。远端 required 仍为 `ci-gate`、`pr-title`，strict=true。

## 1. 目标与成功标准

本任务处理调查中可直接落地的四项：交付前已知归档与最终实现合批推送、减少测试固定规则的重复初始化、补齐一次非确定失败的诊断、按准确检查名交接监控。

1. 规则初始化优化仅存在于两个模块的 `#[cfg(test)] mod tests`，产品编译、运行配置、加载行为与公开接口不变。
2. 在一个 Rust lib 测试进程内，两组共享初始化各执行一次；服务层另外保留一次独立真实加载。服务、缓存容器、插件配置和请求结果仍逐测试独立。
3. 既有测试名称、数量、断言、输入与忽略状态保持；服务真实加载、临时规则文件、缓存预热/裁剪，以及官方插件/运行时集成测试继续执行原加载路径。
4. 既有网关测试仍严格要求上游调用一次；失败时提供足以区分路由提前返回、已记录的传输错误和 mock 已结束的上下文。本任务交付“诊断增强”，不将未知根因写成已修复。
5. 最终待验证 head 推送前完成可知记录与迁移。允许必要云端生成修正和真实返工，不为已知归档、记录自身提交号或 CI 结果额外制造一次成功后的源码 PR 验证。
6. 现行 required `ci-gate`、`pr-title` 及分类、失败传播、主干集成验证保持。监控绑定准确 head 和实际检查名，所需检查失败被观察到后返回，缺失检查不能通过。
7. 最终 head 的必要 Actions 验证成功、独立审查通过，完成获准交付与本任务归档/清理。速度效果复用自然 CI 观察，样本不足不追加无限等待或试跑。

## 2. 基线、原因与证据

### 2.1 运行与 session 样本

- 2026-09-11 调查最近 4 个主要 Codex session 及必要子会话；最近 100 次主 CI 中，2026-09-07 至 09-11 窗口有 33 次，读取该窗口全部 jobs。窗口内成功完整 PR 样本 12 次，墙钟中位数 23 分 26 秒，范围约 20 至 39 分钟；未发现明显长队列证据。
- [#196 实现 CI 34500544358](https://github.com/KNaiFen/aio-coding-hub/actions/runs/34500544358) 对 `4226ed5d` 成功后，追加六份归档得到 `2aa403c3`，再次运行 [CI 34503752926](https://github.com/KNaiFen/aio-coding-hub/actions/runs/34503752926)，新增 22 分 49 秒等待、约 32.02 job 分钟。合并后的 [main CI 34506391248](https://github.com/KNaiFen/aio-coding-hub/actions/runs/34506391248) 另需约 29 分钟，承担主干集成职责。
- #193 同样在实现通过后追加归档，[CI 34187295190](https://github.com/KNaiFen/aio-coding-hub/actions/runs/34187295190) 多用 33 分 50 秒。两例是同一任务 PR 新 head，不是额外归档 PR，也不是同 run 重跑。对应 session 为 `01a07e9c-139b-70b3-adef-27be836ee7a5` 和 `01a08965-778d-76f0-966c-ac4a2f8a8aaa`。
- #196 归档推送的原始证据位于 session `01a08c31-19d1-7720-a7f3-659a9ca2bdba` 的 JSONL 第 85、92、120 行；监控观察成功在第 178 行。上述两个归档额外 CI 墙钟合计约 56 分 39 秒，不把轮间代理/人工等待或并行 jobs 再相加。
- [#196 Rust job 102960835499](https://github.com/KNaiFen/aio-coding-hub/actions/runs/34503752926/job/102960835499) 已命中 Rust/pnpm 缓存，tests 步骤约 17 分 48 秒，其中 lib 实际执行 729.49 秒。上一绿色 head 的 [Rust job 102950088595](https://github.com/KNaiFen/aio-coding-hub/actions/runs/34500544358/job/102950088595) lib 执行 590.10 秒。两者同组 44 个相关插件测试的完成日志间隔合计分别约 548.55/423.85 秒，有 27 处约 16 至 20 秒慢间隔。这是串行日志归因，非精确 profiler 或可承诺节省量。
- [0.60.61 首轮 Rust job 102992281335](https://github.com/KNaiFen/aio-coding-hub/actions/runs/34513194799/job/102992281335) 失败在 `upstream_error_response_rule_rewrites_last_all_failed_attempt`：`routes.rs:1777` 计数实际 0、预期 1；2982 passed、1 failed、4 ignored。同 SHA attempt 2 成功，含候选构建的整次等待达约 75 分 57 秒，不能将这段时间全算成重复测试。
- #195 session `01a080ad-591d-7451-88c7-7d0cd0f11649` 第 108 行把 expected 写成 `CodeQL / Analyze (...)`，实际为 `codeql (...)`。检查完成到合并相隔 2 小时 07 分 38 秒，期间另有代理异常，不能全部归因为名称错误。

### 2.2 当前配置与定位

- 本地 main 相对本地 origin/main 为 ahead 32 / behind 3，并有旧任务归档修改及活动记录删除。远端已上线 #195 的合同前置、纯文档 CodeQL 跳过和归档合批规则；本地相关工作流/治理文档较旧。执行必须从远端基线起步，不能带回陈旧配置或主工作树独有历史。
- 旧 `.gkd/plan.md` / `plan-changes.md` 属于已交付的 `provider-limit-reset` r3，正文已保存在远端 `.gkd/archive/provider-limit-reset/2026-09-10-r3/`；当前活动路径为空。本轮只复用 plan 路径，保留其他用户改动和 plan-changes 的既有删除。
- `src-tauri/src/app/plugins/privacy_filter.rs:702` 的 `tests::filter()` 每次解析同一 include_str fixture，7 个测试各调用一次。`PrivacyFilter` 的实际过滤入口接收 `&self`，配置与匹配结果为调用局部值，可共享固定编译结果。
- `src-tauri/src/app/plugins/privacy_redaction_service.rs:581` 的 `privacy_filter_detail` 使用 packaged 官方规则；`:619` 的公共测试装配及其他直接装配反复创建 Default service。执行核对模块共15个测试，其中13个使用完整 packaged 规则，另有临时文件限额、预热/裁剪测试；修正初次调查多算1个的数字，不改变范围。
- 服务缓存使用 `privacy_filter_cache_key` (`:137`)，包含插件、版本、目录、更新时间。测试可在模块内部访问该缓存，以同一键预填不可变 `Arc<PrivacyFilter>`，无需生产构造器或跨模块注入。
- 网关 helper `run_codex_error_response_rule_route` (`routes.rs:1725`) 已持有环境锁、写入默认路由、限制尝试次数，并在返回地址前 bind listener；CI 使用单线程。当前没有证据支持补 sleep、放宽 timeout 或再次补默认路由。
- `gkd-ci-monitor/scripts/gkd-github-watch` 的 `summarize_checks` 已优先报告所需检查 failure/cancelled；PR 模式能在其他检查未完成时结束失败，run 模式等待 run 自身终态。名称错误属于交接错误，无需默认修改用户级 watcher。

命令摘要：Git status/show/diff/history、CodeGraph 定位、FastCtx 精读；只读 gh commits/main、ruleset、Actions runs/jobs/logs、PR commits/files；JSONL/SQLite 结构化读取。未运行项目测试、安装或构建。

## 3. 实现设计与最小范围

### 3.1 P1：归档与最终验证合批

沿用远端 `docs/operations/github-actions-governance.md:29-31` 的既有规则，不再新建归档分类器或重复规则文件。本计划第 6 节和执行/收尾交接给出具体顺序：

- writer 完成实现后，先审差异与已有证据，注明最终 CI 待定；独立审查通过后一次性交给 closeout。
- closeout 将已知 PLAN、execution、progress、review、说明和活动记录迁移合批到最终待验证 head，再推送并等必要 CI。可为多个本地提交，合批推送即可。
- 已有 PR 因云端生成修正或真实失败需要新的实现 head 时，最终修复应同时带上当时已知记录。失败 head 不冒充已验收。
- CI、合并和清理结果按真实时点保存为主工作树归档观察并引用 PR/Actions，不为回填记录自身结果另开提交/PR/CI。

收益是避免整轮仅因归档追加的源码验证；风险是过早记录成功，故所有未发生事实明确写待定。观察点为每次新 head 的真实原因，保留真实修复所需验证，不设“一律一次 CI”的门槛。

### 3.2 P1：两个测试模块复用固定初始化

只修改两个 `#[cfg(test)] mod tests`，不新增生产接口：

1. `privacy_filter.rs::tests` 增加模块私有 `LazyLock<PrivacyFilter>`，初始化保持原 include_str fixture 和真实解析入口。`filter()` 返回静态只读引用，调整对应借用写法，7 个测试的行为断言保持。加载统计测试仍断言真实编译所得规则数量和 skipped 数。
2. `privacy_redaction_service.rs::tests` 增加模块私有 `LazyLock<Arc<PrivacyFilter>>`，只加载该模块固定 packaged 规则，使用现有 `privacy_filter_detail` 和真实 `load_privacy_filter`；初始化失败直接报错。与低层 fixture 保留两个独立静态来源，不按当前文件字节相同强行合并。
3. 在同一测试模块增加一个重复使用的服务装配 helper：接受当前测试的 `&PluginDetail`，创建全新的 `PrivacyRedactionService` 和 Mutex/HashMap，以现有 `privacy_filter_cache_key` 插入共享规则的 Arc clone。每个测试继续独立持有缓存、插件、配置与输入。
4. 12 个只验证输出/配置的 packaged 规则测试使用该装配，包含 `execute_privacy_filter_request` 和现有直接 Default 装配；`privacy_redaction_service_redacts_before_send_request_bodies` 保留 Default 和真实加载路径。
5. 临时规则文件限额测试、预热/裁剪测试原样保留。`official.rs`、`runtime_executor.rs` 和 `plugin_service.rs` 的真实安装、加载与运行时集成测试全部原样保留。

源码装配口径：低层 7 次变为 1 次；服务层 13 次变为共享 1 次 + 保留真实加载 1 次，合计减少 17 次重复完整初始化。e1 实施时逐项核对发现初次数字多算1次，main 已确认按实际12个复用测试推进，纠正记录见 progress；行为、文件与验证范围不变。此数字是测试源码结构预期，不是云端耗时保证。LazyLock 只在同一 lib 测试进程共享，不跨 job 或测试二进制。

风险与验证：共享可变服务会造成串扰，因此仅共享只读规则结果；缓存和配置继续隔离。通过差异审查确认产品代码不变、既有测试集合不变，Actions 运行全部原测试。若出现共享状态/顺序问题或实测收益不抵新增复杂度，回退这两个测试模块的初始化复用，保留其他独立项。

### 3.3 P1：补齐网关偶发失败诊断

仅扩充 `routes.rs::tests::run_codex_error_response_rule_route` 原 `assert_eq!(call_count..., 1)` 的失败消息，使用已经存在的 `status`、`response`、`provider_id`、终态请求 `log` 和 `upstream_task.is_finished()`。保留原计数、响应状态、原上游状态审计等后续断言。

不新增请求、等待、重试、计数豁免或 mock 状态机；不改变生产路由、timeout、环境锁及默认配置。失败消息应包含 `error_code`、`attempts_json`、`provider_chain_json` 所在的现有终态日志，使自然失败可区分前置短路、传输错误与任务已结束。这里均为测试自产数据。

一次绿色运行只证明诊断改动无回归。若本任务 CI 再次出现该失败，读取新增上下文后再决定修复；只有根因明确且修复仍限于本 helper 测试装配时可在同范围纠正，涉及生产行为或新的模块须修订计划，不能以重跑转绿代替解释。

### 3.4 P1：准确交接监控与失败反馈

在现有 `docs/operations/github-actions-governance.md` 普通 PR 检查段落补充执行约定，不修改用户级 skill 或 watcher：

- 普通 PR 使用 PR 模式，required 仍为远端 `ci-gate`、`pr-title`。任务必要 job 作为 expected 可提供早期失败反馈；必须来自目标工作流与 `gh pr checks --json name,bucket,state,workflow` 实际名称，并说明必要性，不能根据 UI 标题拼接名字。
- 本任务是 Rust 测试与受检文档变更，PR expected 为 `contracts`、`rust`、`observer-macos`；这些 job 已被 ci-gate 要求，显式加入是为了下次查询即可观察失败。`frontend` 按此文件集合应 skipped，不加入 expected；若接受必要生成绑定修正导致最终分类选择 frontend，则同步加入该实际 job，不沿用旧选择。CodeQL 非 required 且本任务不改其工作流，不额外等待它。
- 同一 main merge SHA 包含代码，按现行合同两端执行；commit 监控 expected 为 `ci-gate`、`contracts`、`frontend`、`rust`、`observer-macos`。main 没有 pr-title，不错误添加。
- failure/cancelled、缺失检查待定、脚本 timeout、调用错误、代理异常分别记录。保留默认 240 秒查询/6 小时预算，不承诺故障瞬间被观察，不擅自改成短轮询；不自动删除缺失 expected 以得到绿色。
- 独立 CI 与收尾按现行 GKD 不同接续边界处理，异常不冒充 Actions 卡住或重新触发 CI 的理由。

回滚观察点：若交接名称/选择域错误，纠正为实际 job 与合同要求后按现有监控规则处理；不修改 required 设置、不放松缺失判定。

### 3.5 明确排除

- Rust 测试/benchmark 拆分、测试线程、构建并发、benchmark 触发与合并门槛调整沿用此前排除决定。
- 不改 workflow、分类器、缓存 key/save-if、runner 或工具链，不扩大缓存空间；TUI target 缓存隔离留作低优先级候选，现有数据不足以优先实施。
- 不新增 `cfg(test)` 跨模块注入构造器、不缓存任意规则输入、不改过滤规则/算法，不开展安全分析或漏洞工作。
- 不新增测试框架、性能采集平台、监控脚本或归档检查器；不改用户级 GKD skills、角色设置和其他项目。
- 不改版本，不发 tag/Release、不打包或安装，不整理本地 main 分叉和其他任务遗留文件。

## 4. 文件与覆盖边界

| 文件 | 允许改动 |
| --- | --- |
| `src-tauri/src/app/plugins/privacy_filter.rs` | 仅 tests 模块的固定 fixture 初始化和借用装配 |
| `src-tauri/src/app/plugins/privacy_redaction_service.rs` | 仅 tests 模块的固定 packaged 规则及独立服务装配 |
| `src-tauri/src/gateway/routes.rs` | 仅既有响应规则 helper 的失败诊断及根因证实后同 helper 装配纠正 |
| `docs/operations/github-actions-governance.md` | 监控模式、实际检查名与失败/缺失/异常边界，保留现有归档顺序 |
| 本任务 `.gkd/` Markdown | 计划、交接、进展、审查及必要归档 |

必要机械生成例外按第 5.2 节处理：仅接受该 SHA/attempt 云端 patch 对本任务源码格式、Cargo.lock 或 `src/generated/bindings.ts` 的直接修正；有无关漂移或语义变化时先查明归属，不整包应用。生成绑定变化须同步扩大实际验证与监控选择。

工作流 DAG 保持现状：

```text
change-scope -> contracts -> frontend / rust / observer-macos（按域选择，并行）
change-scope -> candidate-plan
版本 main: contracts + frontend + rust + plan -> 桌面/TUI候选 -> assemble
全部必要结果 -> ci-gate（always，严格判定）
```

| 变更类型 | 现行必要检查，保持 |
| --- | --- |
| 纯 `.gkd` Markdown/过程归档 | 分类 + gate；PR 另有 pr-title，重任务跳过 |
| 普通 docs、README、AGENTS、active spec Markdown | 分类 + 文档合同 + gate；PR 另有 pr-title |
| 纯前端源码 PR | 合同 + frontend + gate；PR 标题独立检查 |
| Rust 源码/Cargo 路径 PR | 合同 + rust + observer-macos + gate |
| shared、pnpm-lock、生成绑定、workflow/scripts、未知或混合源码 PR | 合同 + 两端 + observer-macos + gate |
| 文档 + 单域源码 PR | 对应单域和文档合同；本任务预计属于 Rust 域 |
| 含代码的 dev/main push | 两端及 macOS 检查；候选仍仅 main 满足版本/显式恢复条件 |
| 删除、跨域改名 | 删除按旧路径、改名按新旧路径并集，保留完整 PR 比较 |

PR 比较 merge-base 到 head，追加文档不抹去既有源码；异常或未知路径保持全量或明确失败。whole workflow 不增加 paths-ignore，不继承旧 head 的成功为新 head 兜底。[GitHub required checks 语义](https://docs.github.com/en/pull-requests/how-tos/merge-and-close-pull-requests/troubleshooting-required-status-checks)

## 5. 运行环境与必要验证

复用 2026-09-11 同机检测：Apple M4 MacBook Air、16 GB 内存、macOS 26.5.1、磁盘约 228 GiB 总容量/100 GiB 可用。遵守项目云端验证合同：本地只做读取、差异和短时无依赖检查；不安装、编译、测试、生成绑定、打包，不运行根/workspace package scripts，不绕过 guard。

公开仓库继续使用现有标准托管 runner：Rust `ubuntu-22.04` / Rust 1.90.0，macOS observer `macos-latest`，contracts 的 Node 22。不引入 larger/self-hosted runner。[GitHub 标准托管 runner 计费](https://docs.github.com/en/billing/concepts/product-billing/github-actions)

### 5.1 本地

- 当前计划仅验证自身 diff、引用与路径，执行 `git diff --check`，按用户规则单独中文提交，不触发 CI。
- 实施时使用 `git diff --check` 和范围内只读比较；审查确认修改全在原 tests 模块、共享值无业务可变状态、被保留的真实加载路径仍存在。
- 不为装配复用新增与实现重复的单元测试。现有行为测试及保留的加载/缓存测试构成必要回归覆盖；诊断只扩充原断言消息。

### 5.2 Actions

复用自然推送产生的自动 PR CI，不为同一 head dispatch 第二次 CI，不为优化效果造额外 PR、空提交或矩阵。Rust job 保留全部现有命令，重点取证：

```bash
cargo clippy --workspace --all-targets --locked -- -D warnings
CARGO_BUILD_JOBS=1 cargo test --workspace --locked -- --test-threads=1
cargo test --manifest-path src-tauri/Cargo.toml --locked --lib app::observer::activity -- --test-threads=1
```

前两个在 Rust job 的 `src-tauri` 工作目录运行；第三个为 macOS observer 的根目录命令。已有云端格式/锁/生成、依赖检查和 contracts 继续由原工作流执行。受检文档运行既有文档合同。本任务若只含上述文件，PR frontend 和 benchmark 按现有分类不执行，不将 skipped 写成通过了测试。

通过条件：最终 PR head 的 required 与第 3.4 节 expected 成功，选中 job 实际执行、未选中 job 符合分类；没有删测试、放宽断言或新增 ignore。记录 head、实际 checkout 测试 merge SHA、run/attempt、测试计数、缓存条件、jobs/关键步骤耗时和未覆盖项。

生成漂移时，只接受相同被测 SHA/attempt 的 bounded patch；实际格式/生成修正列为必要机械变更，经审查后随当时已知归档合批，若触及生成绑定会按现行分类扩大为两端。不得本地重生成。

合并后等待实际 main merge SHA 的自动 CI，不复用 PR SHA 冒充主干验证。测试初始化效果用自然 PR/main 日志和既有两个样本比较，分别报告初始化次数结构、lib 实际执行、步骤编译/执行与整轮墙钟；不同 runner/cache 条件只作有条件比较，不承诺百分比。若必要验证通过但趋势样本不足，交付注明“实现验证通过，稳定提速效果待观察”。

## 6. 批准后执行、交付与清理

本节已由用户“开始执行此PLAN”批准，执行与收尾在以下边界内连续推进。

1. main 复核远端基线与规则；从远端 main 建立本任务独立 worktree/分支，不混入本地主干的独有提交与旧任务文件。生成 execution，明确 e1、文件范围、环境、验证、中文本地提交许可和最终云端验证待定。
2. 用已配置 `gkd_execute`、`fork_turns="none"` 委派单 writer 实施。writer 完成范围内改动与 progress 后本地提交并停止；不承担推送、合并或归档。
3. writer 停止后由一次 `gkd_accept` 独立审查代码/已有证据，main 写 review 作最终决定。先审代码并标明最终 CI 待定，避免等待绿色后才准备归档。新问题仅复查受影响部分。
4. main 一次性交接 `gkd_closeout`。拟归档路径为 `.gkd/archive/ci-test-initialization/2026-09-12-r1/`；先保存已知内容、迁出本任务活动正文、合批最终 head，再推送并创建/更新一个面向 main 的任务 PR。不能把主工作树计划提交的父历史合入执行分支。
5. 拟议交付许可包括本任务本地提交、分支推送、PR 创建/更新、绑定完整 head 的正常 squash 合并、等待该 main merge SHA 的自动 CI。PR 标题使用 Conventional Commit 前缀；精确正文通过结构化参数或 body-file 提交，不用含 shell 替换的拼接文本。
6. CI 监控沿现有 skill，普通 PR 和 main commit 使用第 3.4 节实际名称。默认 240 秒查询、6 小时预算。主代理等待角色时按 `wait_agent(timeout_ms=3600000)` 连续等待，部分消息不接管现场；没有必要则不额外启动独立监控角色。
7. 正常路径不手动 dispatch、重跑或取消 CI。若自然 CI 失败，先提取证据并纠正范围内问题；真实修复产生的新 head 自然验证。同 SHA 重跑或其他恢复动作需根据实际原因及已有授权决定，不把本计划当作任意重跑许可。
8. 合并、main CI 和清理事实在主工作树同任务归档中按时间补记，引用 PR/Actions，不为回填自身结果再次推送。对归档前可知材料的遗漏不得借“后续事实”掩盖。
9. 默认清理仅含本任务新建临时 worktree、本地/远端任务分支和已保存的本任务活动记录；原 writer 停止、必要验证/交付完成、全部成果合并、无新增待保留内容时才删除。主工作树、main、其他任务归档与当前脏文件不属于清理范围。
10. 不请求发布标签、Release、桌面安装或平台设置变更。本计划获准后的终点为实现、云端验证、独立审查、正常合并及本任务归档/清理；外部异常如实保留材料与未完成项，不宣称已修复未知 flake。

## 7. 消融审查

- 收窄测试优化为两个模块私有静态实例和一个重复使用的测试装配 helper，免去 official/runtime_executor/plugin_service 的跨模块注入接口，保留端到端装配覆盖。
- 不优化所有慢点；17 次重复初始化是本轮清晰可控范围，其余真实集成加载仍执行。
- 偶发失败只补原断言上下文，不以未经证实的配置/同步修改冒充修复。
- 归档、监控复用既有规则与工具，只补项目内交接事实；不新增 watcher、CI 分类器、检查器或全局 GKD 规则。
- 不改工作流及外部设置，不引入新依赖，不执行效果专用真实演练。缓存及测试并行化排除，避免将一个有证据的改进扩展成全面 CI 重构。
