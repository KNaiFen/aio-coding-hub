# 代码库健康度与工程质量审计

> 本文件是本次审计的唯一事实源。除本文件外，审计过程不修改、格式化或生成任何产品代码、配置或测试产物。

## 0. 审计元数据

- 审计状态：`revalidated-planning`
- 开始日期：2026-08-03（Asia/Shanghai）
- 完成日期：2026-08-03（Asia/Shanghai）
- 基线提交：`9d1fb9664b4a783622d937a84381cdd103f7bcc2`
- 初始分支：`main...origin/main`
- 初始工作区：无已跟踪文件改动；存在审计开始前的未跟踪目录/文件 `.impeccable/`、`.playwright-cli/`、`.trellis/tasks/08-02-tui-summary-local-time/`、`.trellis/workspace/KNaiFen/`、`PRODUCT.md`、`upgrade-tui.command`。这些内容由用户或另一 Session 所有，本审计不修改。
- 并行开发说明：用户确认另一个 Session 正在进行前端小版本修改并计划通过 PR 合并发布。审计以以上提交和实际读取到的工作树为证据；结束时重新检查工作区，审计期间发生变化的文件将列为验证盲点，不据其瞬时状态下确定结论。
- 审计期工作区漂移：扫描期间另一 Session 将当前分支切换为 `codex/tui-polish-release` 并修改 TUI、托盘、resident、Provider availability、入口、样式、测试、配置与 Trellis/PENDING 文件。该 Session 随后完成提交并推送；审计结束 HEAD 为 `a322ba15a258f71fd59bf402f92e122eee03ecc4`，相对基线共 31 个文件变化。规则变更仅调整已完成 pending 条目的归档位置，不改变本审计约束。上述变化文件始终以基线提交版本形成确定性审计证据，并统一登记为合并/发布后复核对象。
- 结束工作区：分支 `codex/tui-polish-release...origin/codex/tui-polish-release`，已跟踪文件无未提交改动；仍存在用户/其他 Session 所有的未跟踪 `.impeccable/`、`.playwright-cli/`、`.trellis/workspace/KNaiFen/`、`PRODUCT.md`、`upgrade-tui.command`，以及本报告。审计未修改这些其他内容。
- 本地执行约束：不运行 Cargo、rustfmt、Clippy、Rust 测试、Specta 绑定生成、`pnpm exec tauri` 或任何间接调用它们的脚本；仅允许只读静态检查，以及必要时运行 Node.js、TypeScript、前端测试和 Vite 构建。
- Trellis 说明：原始审计阶段只产出本报告、不创建任务；后续经用户确认进入实施后，已按批次建立 `.trellis/tasks/08-04-*` 任务，并持续以本报告作为问题状态与验证结果的事实源。
- 上一轮复核日期：2026-08-04（Asia/Shanghai）
- 上一轮复核提交：`86a3071090a4a1fa1c33567d488ae3ea12148b53`；其代码树与当时的 `origin/main` 等价，分支仅保留不同提交拓扑。
- 上一轮工作区：父工作树除本报告和审计 Trellis 文件外无未提交产品代码；`PENDING.md` 无未解决条目。PR #40 已于 2026-08-04 合并为 `cec2353f`；原 draft PR #41 至 #50 已关闭并由单一 Ready PR #51 替代。
- 上一轮复核范围：逐项重查全部 51 个原始问题和 5 个假设的现行代码、`origin` 历史与已有测试；不读取 `upstream`，不运行本地 Rust/Cargo 工具链。
- 首批规划任务：父任务 `.trellis/tasks/08-04-codebase-health-next-batch`，子任务分别处理 `AUD-019/AUD-020`、`AUD-004`、`AUD-036`。三个子任务均已完成定向验证并建立 draft PR；#40、#41、#42 的必需检查均已通过。
- 上一轮前端异常恢复批次：父任务 `.trellis/tasks/08-04-codebase-health-frontend-recovery`。`AUD-024` 和 `AUD-041` 已完成局部修复、验证并建立 draft PR #43/#44，两者必需检查均已通过；当时对 `AUD-052` 的无生产调用者判断已由 2026-08-05 复核更正。
- 供应链与 MCP 边界批次：父任务 `.trellis/tasks/08-04-codebase-health-ci-mcp-safety`，子任务分别处理 `AUD-042`、`AUD-048`、`AUD-049`。三项均已重新追到当前生产调用链、已有测试和 `origin/main@fef05dec`，完成最小修复、定向验证与提交前主线门，并建立 draft PR #45/#46/#47；没有主线冲突或搁置项。
- 运行时合同与并发状态批次：父任务 `.trellis/tasks/08-04-codebase-health-runtime-contracts`，子任务分别处理 `AUD-009`、`AUD-011`、`AUD-014`。三项均完成失败优先回归、最小修复、定向验证、差异审查与 PR 前 `origin/main@fef05dec` 主线门，分别进入 draft PR #48/#49/#50；没有主线冲突或搁置项。#48 必需检查全部通过，#49 frontend 通过且 rust 运行中，#50 frontend/rust 运行中。
- PR 整合交付计划（`planned`）：用户要求把尚未合并的 #41 至 #50 合并为一个 PR。已先撤销 #41 自动合并并恢复为 draft；以已包含 #40 的最新 `origin/main@cec2353f` 新建整合分支，按各 PR 的最终 head 提交顺序纳入全部 10 项改动，不改写产品实现。实施后逐项核对原 PR 文件/提交覆盖，联合运行原有定向前端/Node/TypeScript 检查、隔离 Vite build 与 `git diff --check`；Rust/native 仍仅由新 PR 的 GitHub Actions 验证。提交前再次 fetch `origin/main` 并检查功能、实现和效果冲突；通过后建立单一 Ready PR、开启 CI 通过后 squash 自动合并，再关闭 #41 至 #50 并登记替代关系。若主线或整合产生只能二选一的语义冲突，则停止该整合 PR、在本报告登记后继续后续审计项。
- PR 整合交付结果（`pr_open`）：整合分支 `codex/audit-health-consolidated` 从 `origin/main@cec2353f` 建立，按最终 head 无冲突重放 #41 至 #50 的 12 个提交；最终 head `25637824` 的 33 个文件逐项与原 PR 内容一致，正好等于十个 PR 的路径并集，无额外文件。联合验证通过 20 个前端测试文件/192 tests、脚手架 33 tests、pnpm audit 语法与 selftest、四项插件合同、根与包级 TypeScript、全前端 ESLint、目标 Prettier、隔离 Vite build 和 diff 检查。Ready PR #51 已开启 CI 通过后自动 squash 合并；原 #41 至 #50 已关闭并逐项注明替代关系。停止持续监控时 frontend、support-contract、docs-contract、change-scope 和 pr-title 均通过，rust 仍在运行；因此十项状态仍为 `pr_open`，不能提前转为 `resolved`。遗留风险是本地按仓库规则未运行 Rust/native 工具链，最终合并仍取决于 #51 rust 终态；Actions 同时报告既有 Node 20 action 弃用提醒，本批不顺手升级 action。
- 本次接续复核日期：2026-08-05（Asia/Shanghai）。已 fetch 并以 `origin/main@eeccf64dc2d60698d0df48ff3fcbcd2aafd24688` 的独立只读 worktree 为准，逐项重新核验既有 52 个问题和 5 个假设；用户新增的供应商页当前路由展示问题经同一主线核验后登记为 `AUD-053`。当前工作分支 HEAD `86a3071090a4a1fa1c33567d488ae3ea12148b53` 相对主线 10 个提交领先、3 个提交落后，不把当前工作树的旧代码当作现状。
- 主线交付事实：PR #51 已通过全部必需检查并以 `02b9980d8ce7e5fc61af6837ab06c587e50fd7b0` 合并，原 10 个 `pr_open` 项转为 `resolved`；随后 PR #52 以 `eeccf64dc2d60698d0df48ff3fcbcd2aafd24688` 合并请求观测、TUI、Provider 状态与相关迁移。#52 未重新引入已解决根因，也未解决本批四项；其更丰富的 Provider 时间线会增大 Observer 快照，放大 `AUD-030` 的既有缓存风险。
- 本次高收益边界批次：`AUD-001`、`AUD-021`、`AUD-053`、`AUD-037` 已分别由 PR #53/#54/#55/#56 合并为 `891c9eb3`、`ef41e6da`、`ed72549b`、`db92a480`，全部转为 `resolved`。本批没有待决策冲突。
- 质量恢复批次：在 `origin/main@ef41e6da` 复核后选择 `AUD-005/AUD-006/AUD-044/AUD-052`。ready PR #57/#58/#59 已分别以 `0062c907`/`5b13683b`/`62574e22` 合并，四项全部转为 resolved；#59 的标准 CI、四平台 dev-build、下载后 mode 与架构检查均通过。父任务为 `.trellis/tasks/08-05-codebase-health-quality-recovery`。
- 下一批布局恢复：重新读取 `PENDING.md`（无未解决条目）并在 `origin/main@0062c907` 核验后选择 `AUD-015`；候选最终无冲突重放到 `origin/main@62574e22`，以 `4d1d720a` 通过全部检查并由 ready PR #60 squash 合并为 `d12dbfe3`。任务只让 AppLayout 为启动横幅和 Outlet 分配剩余高度，并增加结构与真实小视口滚动回归。
- 插件 SDK 合同批次：再次读取 `PENDING.md`（无未解决条目）并核验后选择 `AUD-051`；候选由 `c62c4725` 无功能变更重放为 `29c2139e` 到 `origin/main@d12dbfe3`，ready PR #61 的全部必需检查通过后 squash 合并为 `ba06dabb`。SDK、合同和文档表面已同步，未修改原生运行时行为。
- TUI Provider 探测期限批次：再次读取 `PENDING.md`（无未解决条目）并在 `origin/main@5b13683b` 重新核验 `AUD-029`。候选最终重放为 `13abfea7` 到 `origin/main@ba06dabb`，7/7 源合同与 diff 复验通过，Actions `30995513871` 全绿，ready PR #62 squash 合并为 `c2e4db25`；未放宽快照轮询或引入取消重构。
- 长会话双向窗口批次：在报告先转为 planned 后实施 `AUD-038`；#62 合并后候选无冲突重放为 `f7d6fc17` 到 `origin/main@c2e4db25`，Actions `30997519757` 全绿，ready PR #63 squash 合并为 `e57acb54`。保留十页窗口、page size、query key 与 IPC，只补双向取页、真实边界文案和响应式操作控件；28 项定向测试、静态门、Vite、diff 与 1024px 浏览器回归通过。
- 普通设置串行 patch 批次：在 `origin/main@ba06dabb` 重新核验 `AUD-012`，#63 合并后候选无冲突重放为 `f6d7d2d4` 到 `origin/main@e57acb54`。Actions `30999335471` 全绿，最终主线门确认 base/merge-base 未漂移，ready PR #64 squash 合并为 `5c756edc`。原生专用设置组所有权屏障已消除旧报告中的跨 writer 覆盖面；本批只统一前端普通设置写入的串行 patch，不改原生 schema、专用 writer 或设置语义。
- Observer 快照缓存边界批次：在 `origin/main@c2e4db25` 重新核验 `AUD-030`，#64 合并后实现无冲突重放到 `origin/main@5c756edc`；三轮 Actions（含云端 rustfmt artifact 与 Clippy 最小修正）全部通过，ready PR #65 squash 合并为 `405a545f`。#63/#64 只改会话分页和前端设置，与 Observer 缓存无文件交集。本批只增加 TTL 访问清理、64 项硬上限和最旧项淘汰，不改协议或 snapshot 内容。
- 发布签名私钥作用域批次：重新读取 `PENDING.md`（无未解决条目）并在 `origin/main@405a545f` 复核 `AUD-043` 后，提交 `1fcc687d` 创建 ready PR #66。规范化私钥改写 runner-temp 0600 文件，仅由签名 Tauri Action 的 step-scoped 路径读取，紧邻 `always()` 删除；静态合同/self-test 锁定 secret 位置、跨步骤 command-file、顺序和清理。Actions `31005579029` 全绿，最终主线门无漂移，PR #66 squash 合并为 `d5c9cfe0`；不改版本、制品或 promotion。
- Responses 连续性缓存字节预算批次（`resolved`）：在 `origin/main@d5c9cfe0` 复核 `AUD-017` 后完成 P1 单文件修复；候选 `4de2889b` 的 Actions `31012064253` 全绿，最终主线门确认 base/merge-base 无漂移且只有该 PR 开放。Ready PR #67 squash 合并为 `0854d830`，每条 1 MiB、全局 32 MiB 最终 JSON 载荷预算已进入主线。
- 插件预览内容绑定批次（`resolved`）：候选 `4ce877b6` 的 Actions run `31015354600` 全绿，最终主线门确认 base/merge-base 仍为 `0854d830`、只有自身 PR 开放且 CLEAN/MERGEABLE；Ready PR #68 squash 合并为 `e94c83bd`。本地安装/更新确认现在绑定预览 checksum，缓存写入同一份已验证字节。
- 图片生成响应下载扇出批次（`resolved`）：候选 `f703c863` 的 Actions run `31017816818` 全绿，最终主线门确认 base/merge-base 仍为 `e94c83bd`、只有自身 PR 开放且 CLEAN/MERGEABLE；Ready PR #69 squash 合并为 `9a280136`。
- 前端内存诊断共享预算批次（`resolved`）：候选 `35491b78` 基于 `origin/main@9a280136`，10 个 service 测试文件/52 tests、TypeScript、目标 lint/format、隔离 Vite、diff、两轮独立审查与 Actions `31020581604` 全绿；Ready PR #70 已 squash 合并为 `5d4906c5`。
- Observer OAuth gate 批量查询批次（`resolved`）：候选 `f92d5190` 的 Actions `31023947314` 全绿；最终主线门确认 `origin/main@5d4906c5` 未漂移、PR CLEAN/MERGEABLE，且 #72 仅改无交集的插件管线。Ready PR #71 已 squash 合并为 `7c395d15`。
- 插件 fail-open header patch 合同批次（`resolved`）：逻辑提交 `2a1878ba` 加云端格式提交 `0a5bd769` 的 Actions `31026666018` 全绿；最终主线门确认 `origin/main@7c395d15` 未漂移，#73 的观测/日志批次与 `pipeline.rs` 无交集。Ready PR #72 squash 合并为 `d26524f2`。
- Extension Host 主动空闲回收批次（`resolved`）：在 `origin/main@d26524f2` 复核 `AUD-046` 后完成三文件最小修复；逻辑提交 `139c8432`、云端格式提交 `a50ec5be` 的 Actions `31030917177` 全绿，Ready PR #74 squash 合并为 `94da784b`。最终主线门确认 #73 无目标文件或同功能实现，合并树与候选三文件一致。
- Homebrew Cask 正式资产合同批次（`resolved`）：候选 `007e612d` 在 #73 合并后无冲突重放 `origin/main@2a79978c`，同步 0.60.49 示例 tag；第二轮 Actions `31036360425` 全绿，最终主线门无漂移且 PR CLEAN/MERGEABLE。Ready PR #75 squash 合并为 `ff09a81a`，合并后三目标文件树与候选一致。
- FormField 标签关联合同批次（`resolved`）：候选 `90230c56` 的 Actions `31039483396` 全绿，Ready PR #76 squash 合并为 `9e83772c`；25 个唯一主控件字段使用 render prop，9 个复合控件使用 group，生产 60 个调用均满足类型合同。重放后 143/143 定向、2814/2814 全量前端单测、TypeScript、ESLint/Prettier、Vite build、AST 与 diff 全过。
- Provider 自环发送防护批次（`resolved`）：最终候选 `b7f5378c` 基于 `origin/main@60b12aa4`，Ready PR #78 的 Actions `31047727848` 全绿后 squash 合并为 `ecd82606`。合并前重新 fetch 并确认唯一开放 PR 为自身、base/merge-base 未漂移且无同功能实现；合并后两目标文件树与候选一致。
- 插件版本不可变性批次（`resolved`）：最终候选 `e89efd3c` 的 Actions `31052820007` 全绿；合并前 `origin/main`/base/merge-base 均为 `ecd82606`，#80 仅改 CI、审计配置和锁文件，无功能或路径重叠。Ready PR #79 CLEAN/MERGEABLE 后 squash 合并为 `cab1229a`，合并后 `plugin_service.rs` 与 `repository.rs` 的 blob 与候选一致。
- RustSec 审计豁免移除批次（`resolved`）：云端 artifact 只把 `plist/quick-xml/wayland-scanner` 更新为 `1.10.0/0.41.0/0.31.11`；最终候选删除临时 update、保留 plain `cargo audit` 并移除 `audit.toml`。Actions `31054953851` 全绿；合并前 `origin/main`、base 与 merge-base 均为 `cab1229a`，没有其他开放 PR 或相关主线实现。Ready PR #80 squash 合并为 `b0698f57`，三个目标路径与候选 `2f499dd0` 一致。
- 插件 fail-closed 日志持久化批次（`resolved`）：在 `origin/main@cab1229a` 复核确认 fail-closed error/invalid payload/circuit-open 仍可把原 request log 入库，未知 policy 仍静默 fail-open，且 circuit 跨 hook 共享。现有 redactor 无法完整无条件兜底；候选以 enqueue 前 drop-log 屏障、hook 级 circuit 与快照替换隔离、严格 policy 校验和七份直接合同收口。首轮 Actions `31060135548` 仅报云端 rustfmt 漂移，精确单文件 artifact 提交为 `27213728`；第二轮 `31060862654` 全绿。合并前 `origin/main`、base 与 merge-base 均为 `b0698f57`，只有自身 PR 开放且 CLEAN/MERGEABLE；Ready PR #81 squash 合并为 `871b84dc`。
- 插件上下文字段大小写兼容批次（`resolved`）：重新读取 `PENDING.md`（无未解决条目）并在 `origin/main@b0698f57` 核验 `AUD-010`。现行 wire 仍输出 snake_case，SDK/v1 contract 仍声明 camelCase，真实 runtime fixture 仍读取 `body_truncated`。候选 `1ff29726`、云端格式提交 `8d5ef669` 让 wire 只输出 canonical camelCase，在 QuickJS 解析后为旧插件安装同值 snake_case alias，并补齐 SDK/合同遗漏的 truncation flags；#81 合并后已核对四个重叠文件可兼容，Ready PR #82 的 Actions `31063487534` 全绿后 squash 合并为 `e6cf04d3`。合并后 `origin/main` 八个目标文件树与候选一致。
- 插件 QuickJS 上下文容量边界批次（`resolved`）：逻辑候选 `cc8ab625`、云端格式 head `a891a038` 严格七文件；第二轮 Actions `31069274373` 全绿，合并前 `origin/main`/base/merge-base 均为 `e6cf04d3` 且无漂移。Ready PR #83 squash 合并为 `4ee5faa8`，合并后七文件树与候选一致。
- 插件配置与 Storage 原子合并批次（`resolved`）：最终候选 `c669b522` 严格五文件；Actions `31073434744` 的 frontend、Rust format/bindings、Clippy、Rust tests、依赖审计及全部合同门全绿。合并前 `origin/main`、base 与 merge-base 均为 `4ee5faa8`，只有自身 PR 开放且 CLEAN/MERGEABLE；Ready PR #84 squash 合并为 `4800bc87`，合并后五个目标文件树与候选一致。
- 插件 hook 单次调用绝对截止时间批次（`resolved`）：逻辑候选 `1417c045`、云端格式 head `27efd051` 严格四文件；同一 absolute deadline 覆盖 gate、单插件队列、清理、cold start、activation 与 RPC，warm/cold/LRU 超时实例按身份摘除并强制终止。Actions `31077183327` 的 frontend、Rust format/bindings、Clippy、Rust tests、依赖审计及全部合同门全绿；合并前 `origin/main`、base 与 merge-base 均为 `4800bc87`，只有自身 PR 开放且 CLEAN/MERGEABLE。Ready PR #85 squash 合并为 `735cec12`，合并后四个目标文件树与候选一致。
- 最终治理批次基线：2026-08-06 以已核验的 `origin/main@735cec12` 建立隔离候选。加入新问题前，索引的正确现状为 48 项 `resolved`、5 项 `confirmed`；旧的 47 resolved / 5 confirmed / 1 pr_open 统计未吸收 PR #85 终态，现已纠正。新增 `AUD-054` 至 `AUD-056` 后，用户锁定按 054、055、056、016、008、002、035、033 的顺序逐项实施，八项全部转为 `planned`。
- 最终治理批次本地约束：禁止依赖安装、dev、类型检查、Lint、测试、构建、Cargo、Tauri 及任何会生成 Node/Rust 产物的仓库脚本；本地只运行零依赖 Node 源码合同/解析检查和 `git diff --check`。完整前端、rustfmt、bindings、Clippy、Rust tests、audit 与原生制品验证由 GitHub Actions 负责。

## 1. 扫描进度与覆盖模块

状态含义：`pending` 未开始；`scanning` 正在扫描；`complete` 已扫描且证据已写入本报告；`blocked` 因约束无法完成。

| 模块 | 主要范围 | 状态 | 完成证据 |
| --- | --- | --- | --- |
| A. 规则、文档、依赖与构建基线 | `AGENTS.md`、`README*`、`PENDING.md`、`.trellis/workflow.md`、架构文档、根配置、CI/脚本 | `complete` | 覆盖规则、README/架构、依赖、15 个 Node 检查脚本和全部 Actions workflow；确认发布、CI gate、依赖审计及文档漂移问题 |
| B. 前端应用壳与页面层 | `src/app`、`src/pages`、`src/components`、`src/ui`、`src/hooks` | `complete` | 覆盖应用壳、查询/hooks、设置/插件/MCP/会话/用量页面、通用 UI；确认状态竞争、数据可达性、输入边界与无障碍问题 |
| C. 前端服务、状态与 IPC 边界 | `src/services`、`src/generated`、`src/types`、`src/test` | `complete` | 覆盖 services/utils/constants/schemas/generated/test；确认 `AUD-021` 至 `AUD-024`，未发现该层循环依赖 |
| D. Rust 应用核心与持久化 | `src-tauri/src` 中配置、数据库、命令、应用生命周期、CLI 管理（不含网关/插件） | `complete` | 覆盖 command→app/domain/infra 调用链、事务与资源生命周期；确认初始化重试、双写补偿、数据重置与配置完整性问题 |
| E. Rust 网关、路由与流式代理 | `src-tauri/src/gateway` | `complete` | 覆盖监听/路由、认证、转发、重试、流式收口、缓存、日志与 OAuth 身份边界；确认安全与资源问题 |
| F. 插件系统与扩展宿主 | Rust/前端插件模块、`packages/plugin-sdk`、`packages/create-aio-plugin`、`docs/plugins` | `complete` | 覆盖安装/更新、SDK/脚手架、manifest、Extension Host、hook pipeline、失败策略与资源生命周期；确认合同、隔离、并发、预算和快照一致性问题 |
| G. TUI 与观测协议 | `src-tauri/crates/aio-tui`、`src-tauri/crates/aio-observer-protocol` 及生产端投影 | `complete` | 以基线版本覆盖 TUI/client/observer 投影；确认超时错配、缓存、N+1 和全量会话扫描问题 |
| H. 测试体系、自动化与发布链路 | 全仓测试布局、`scripts`、`.github/workflows`、覆盖率与发布合同 | `complete` | 覆盖 CI、发布、dev-build、同步与静态检查；确认自动发布、制品、密钥暴露、审计 gate 和测试缺口 |
| I. 横向架构、依赖方向与重复实现 | 全仓导入/模块关系、跨层合同、重复逻辑、死代码候选 | `complete` | 结合模块调用链核对 IPC/插件/observer 合同；未发现需要单列的可确认循环依赖，跨层合同问题已并入相关 AUD 项 |
| J. 安全、数据完整性与资源/性能横切面 | 密钥/日志/文件/网络边界、并发与资源生命周期、无界增长风险 | `complete` | 横向复核认证、秘密、状态快照、缓存、并发写入、流式 hook 与输入限制；结果已归并入 `AUD-002`、`AUD-008`、`AUD-012`、`AUD-016` 等 |

## 2. 简要架构与关键数据流

### 2.1 架构图

```text
React 19 页面 / 组件
        |
        v
TanStack Query + 前端 services
        |
        v
generatedIpc / Tauri invoke + desktop events
        |
        v
Rust Tauri commands / 应用状态
   |           |              |
   v           v              v
SQLite      Axum 本地网关    插件 Extension Host
               |
               v
      路由 / 重试 / 协议桥 / 流式转发
               |
               v
       Anthropic / OpenAI / Gemini 等上游

Rust 运行时状态 / 请求日志
        |
        v
本地 observer protocol -> 独立 aio-tui
```

### 2.2 已确认的关键数据流

1. 配置与控制流：React 页面通过前端 service 和生成的 IPC 合同调用 Tauri command；Rust 负责持久化配置并更新运行时状态。
2. 请求转发流：本地 CLI 请求进入 Axum 网关，经供应商/模型路由、重试与熔断选择上游；协议桥负责请求/响应转换，流式管道负责转发、用量与请求结束收口。
3. 观测流：网关尝试与请求日志进入持久化/运行时投影，再通过本地观测协议供桌面前端和独立 TUI 消费。
4. 插件流：插件清单与权限在宿主边界校验，扩展宿主参与请求/响应钩子；SDK 与脚手架对外暴露合同。

### 2.3 健康度结论

- 当前索引共 56 项：`P1` 20 项、`P2` 35 项、`P3` 1 项；其中 48 项 `resolved`、8 项 `planned`，没有 `confirmed`、`pr_open` 或 `not_recommended`。加入新问题前的基线为 48 resolved / 5 confirmed；现有八项均已获得用户决策并进入固定顺序实施。原 5 项假设中，3 项仍需运行时/策略验证，`HYP-004` 已被现行修复取代，`HYP-005` 的机制已晋升为生产可达的 `AUD-052`。
- 代码目录和主要层次仍具可辨识的模块边界，前端 service/query 与 Rust command/domain/infra 的常规路径没有发现需要单列的循环依赖；问题更集中在跨层合同没有单一事实源或原子语义，例如 Plugin SDK/Host、TUI/Observer、UI cache/backend DTO、manifest/runtime policy。
- 当前最高风险不是一般可维护性，而是几组可直接影响用户的工程缺口：非回环网关缺少可信调用者边界；插件 fail-closed/timeout 没有端到端兑现；诊断链可能泄露秘密；上游同步仍缺少人工 fork 语义审查门。Release tag 与资产不可变性已由 #40 解决，不再列为现存根因。
- 数据完整性的共性根因是“基于旧快照的整文档写回”：全局设置、插件 config/storage、模型别名和多个前端 mutation 都缺少 revision/CAS、字段级原子 patch 或统一串行化。
- 资源治理的共性根因是各层分别设置局部上限，却没有端到端总预算：网关 body、QuickJS heap、插件队列/stream、observer 查询、缓存、图片下载和图表数据点之间均存在预算断层。

## 3. 问题索引

| ID | 状态 | 优先级 | 模块 | 摘要 | 证据位置 |
| --- | --- | --- | --- | --- | --- |
| AUD-001 | `resolved` | `P1` | CI / 上游同步治理 | 定时同步以 Git 可合并性代替 fork 语义审查，可无人审核更新 `main` | PR #53 已合并：`891c9eb3`；同步 PR 人工审查合同与 self-test |
| AUD-002 | `planned` | `P2` | Rust 数据完整性 | SQLite 与 CLI/技能文件双写依赖补偿事务，但多个失败路径吞掉补偿失败 | `.trellis/tasks/08-06-filesystem-recovery-journal`；`src-tauri/src/domain/prompts.rs:490-503` 等 |
| AUD-003 | `resolved` | `P2` | 发布 / Homebrew | Cask 生成器与文档仍依赖正式 Release 不产出的 Intel 桌面包，并虚构自动 tap 同步 | PR #75 已合并：`ff09a81a`；ARM-only Cask 与真实手动 tap 流程 |
| AUD-004 | `resolved` | `P1` | Rust 启动恢复 | 数据库初始化错误被永久缓存，UI 的“重试启动”会重复返回同一旧错误 | PR #51 已合并：`02b9980d`；`src-tauri/src/app/app_state.rs`、`src-tauri/src/app/startup_tasks.rs` |
| AUD-005 | `resolved` | `P2` | CI / Rust 正确性门 | 已知会导致 `Instant` 下溢 panic 的静态防线只在可选 precommit 中，CI 与 prepush 均未执行 | PR #57 已合并：`0062c907` |
| AUD-006 | `resolved` | `P2` | 插件脚手架 / CI | `create-aio-plugin` 声明了严格 typecheck，但根聚合门与 CI 只运行其 Vitest | PR #57 已合并：`0062c907` |
| AUD-007 | `resolved` | `P2` | Rust 依赖治理 | 两个 CVSS 7.5 `quick-xml` 公告被无期限全局忽略，依赖可达性变化后 CI 仍会绿灯 | PR #80 已合并：`b0698f57`；Actions `31054953851` 全绿 |
| AUD-008 | `planned` | `P1` | Rust 数据重置 / 资源生命周期 | 数据重置未停止长期持有 DB pool 的后台任务，就按非原子顺序删除设置与 SQLite 文件 | `.trellis/tasks/08-06-cross-restart-data-reset`；`src-tauri/src/commands/data_management.rs:61-83` 等 |
| AUD-009 | `resolved` | `P1` | 插件脚手架 / 运行时合同 | 三个官方脚手架模板读取错误的 hook 参数层级，生成插件会静默跳过核心逻辑 | PR #51 已合并：`02b9980d`；三模板行为回归与公开文档修正 |
| AUD-010 | `resolved` | `P1` | 插件 SDK / 序列化合同 | Hook 可见上下文运行时使用 snake_case，公开合同与 SDK 却声明 camelCase | PR #82 已合并：`e6cf04d3`；Actions `31063487534` 全绿 |
| AUD-011 | `resolved` | `P1` | 前端启动状态 | 旧的 GET/重试响应可覆盖更新的启动事件，隐藏真实失败或让成功状态回退 | PR #51 已合并：`02b9980d`；启动 store、监听/bootstrap/Banner 与定向回归 |
| AUD-012 | `resolved` | `P1` | 前端设置 / 数据完整性 | 多个设置 mutation 用旧缓存生成整份配置，并发保存会静默覆盖不相交修改 | PR #64 已合并：`5c756edc`；共享串行 scope 与 changed-key patch |
| AUD-013 | `resolved` | `P2` | 前端 Provider 缓存 | 删除 Provider 后未失效默认路由与排序方案缓存，页面可继续提交已删除 ID | `src/query/providers.ts:332-380` 等 |
| AUD-014 | `resolved` | `P2` | 前端 CLI 代理状态 | 多个 CLI 的异步冲突检查共用单个 pending 槽位，状态和确认内容可互相覆盖 | PR #51 已合并：`02b9980d`；`useCliProxyControls`、Sidebar 与定向回归 |
| AUD-015 | `resolved` | `P2` | 前端布局 / 故障恢复 | 启动失败横幅与 `h-full` 页面叠加超过容器高度，设置页底部可被裁掉 | PR #60 已合并：`d12dbfe3`；AppLayout 剩余高度容器与 21 项回归 |
| AUD-016 | `planned` | `P1` | 网关安全 / 观测完整性 | 非回环监听没有入站认证，且信任客户端可伪造的内部 header，允许借用已存凭据并逃逸日志/用量 | `.trellis/tasks/08-06-gateway-lan-bearer-token`；`src-tauri/src/gateway/routes.rs:36-118` 等 |
| AUD-017 | `resolved` | `P1` | 网关性能 / 内存 | Responses 连续性缓存只限条目数量、不限字节数，可长期复制并保留接近请求上限的大型 JSON | PR #67 已合并：`0854d830`；1 MiB/32 MiB 最终 JSON 载荷预算 |
| AUD-018 | `resolved` | `P1` | 发布自动化 | tag 触发的 Release 对 annotated tag 会 fetch 同名本地 ref 后再推送，导致 tag clobber 失败 | `.github/workflows/release.yml:48-52` 等 |
| AUD-019 | `resolved` | `P1` | 发布供应链 / 不可变性 | Release 可从同 SHA 的“最新”候选重取资产并用 overwrite 覆盖既有 tag 文件，制品不是不可变的 | PR #40 已合并：`cec2353f`；`.github/workflows/release.yml`、`scripts/release-promotion.mjs` |
| AUD-020 | `resolved` | `P2` | 发布并发控制 | 自动 tag 与手动 dispatch 使用不同 concurrency key，可同时写同一 Release | PR #40 已合并：`cec2353f`；`.github/workflows/release.yml:18-20` |
| AUD-021 | `resolved` | `P1` | 前端日志 / 隐私 | 分散的字符串与错误清洗可将 API key、认证头、prompt、URL query 等写入诊断和原生日志 | PR #54 已合并：`ef41e6da`；共享 redactor 与 native 二次清洗 |
| AUD-022 | `resolved` | `P2` | 前端性能 / 诊断 | 内存诊断对每一个 query 都同步遍历最多 20 万节点，没有整次快照预算 | PR #70 已合并：`5d4906c5`；共享节点/query 预算与有界 top-20 |
| AUD-023 | `resolved` | `P2` | 图片生成 / 资源控制 | 上游图片 URL 响应数组无数目和聚合下载预算，可触发无界串行下载 | PR #69 已合并：`9a280136`；请求 n/硬上限 10 |
| AUD-024 | `resolved` | `P3` | 更新器异常恢复 | 畸形 percent-encoded Release tag 在 catch 外解码，导致完整更新检查拒绝 | PR #51 已合并：`02b9980d`；updater 与定向测试 |
| AUD-025 | `resolved` | `P2` | 网关递归保护 | 自指 Provider URL 不会得到内部跳数标记，递归 guard 永远不触发并耗尽连接/超时预算 | PR #78 已合并：`ecd82606`；发送前 runtime self-target 拒绝与 DNS 别名防护 |
| AUD-026 | `resolved` | `P1` | 插件安全 / 失败策略 | fail-closed 日志脱敏会被入口、未知策略和熔断跳过原文绕过 | PR #81 已合并：`871b84dc`；Actions `31060862654` 全绿 |
| AUD-027 | `resolved` | `P1` | 插件运行时 / 资源边界 | 网关可接受的 body 大小远高于 QuickJS heap，官方 fail-closed 请求钩子会对合法大请求系统性阻断 | PR #83 已合并：`4ee5faa8`；Actions `31069274373` 全绿 |
| AUD-028 | `resolved` | `P2` | 网关 OAuth / 身份完整性 | 来访 `chatgpt-account-id` 可覆盖已选 Provider 的账户头，与本次注入的 OAuth token 混搭 | `src-tauri/src/gateway/proxy/handler/failover_loop/prepare/codex_chatgpt.rs:62-85` 等 |
| AUD-029 | `resolved` | `P2` | TUI / Observer 可用性 | TUI 总超时 3.5 秒低于服务端/上游探测期限，4-15 秒成功的探测必报失败 | PR #62 已合并：`c2e4db25`；共享 deadline、请求级 timeout 与超时分类 |
| AUD-030 | `resolved` | `P2` | Observer 缓存 / 内存 | 快照缓存没有容量或过期删除，合法参数组合可长期保留数百个大快照 | PR #65 已合并：`405a545f`；TTL 访问清理与 64 项上限 |
| AUD-031 | `resolved` | `P2` | Observer 查询性能 | 每个快照对 OAuth Provider 逐一查询 gate，活动轮询形成 N+1 SQLite 压力 | PR #71 已合并：`7c395d15`；分批 display snapshot 查询 |
| AUD-032 | `resolved` | `P2` | 插件失败策略合同 | fail-open 插件的非法/保留 header patch 仍无条件中断请求，与公开 fail-open 语义冲突 | PR #72 已合并：`d26524f2`；事务 patch + request/response policy 回归 |
| AUD-033 | `planned` | `P2` | 插件生命周期合同 | `activationEvents` 只验证/展示而不驱动激活，连续运行时失败也不会持久 quarantine | `.trellis/tasks/08-06-plugin-activation-quarantine`；`src-tauri/src/domain/plugins.rs:760-784` 等 |
| AUD-034 | `resolved` | `P2` | 插件配置 / 数据完整性 | 插件 storage 与 UI 配置都对整份 JSON 无 CAS 地 read-modify-write，并发会丢更新 | PR #84 已合并：`4800bc87`；Actions `31073434744` 全绿 |
| AUD-035 | `planned` | `P2` | TUI / Observer I/O | `history_limit=0` 的状态栏仍读 500 条日志并全量遍历 Codex session 文件树，每 500ms 重复 | `.trellis/tasks/08-06-observer-zero-history-query`；`src-tauri/src/app/observer/snapshot.rs:24,264-285` 等 |
| AUD-036 | `resolved` | `P1` | 前端插件状态 | 切换到慢加载插件详情时，旧插件的配置/版本可作用于新插件的保存或回滚目标 | PR #51 已合并：`02b9980d`；插件详情 identity guard 与定向测试 |
| AUD-037 | `resolved` | `P1` | 定价配置 / 数据完整性 | 模型价格别名读取失败被伪装为空配置，保存会覆盖原有别名规则 | PR #56 已合并：`db92a480`；严格编辑读取、v2 对齐与错误态回归 |
| AUD-038 | `resolved` | `P1` | 前端会话历史 | 十页窗口淘汰较早页面后没有反向取回入口，页面还把窗口起点误标为会话开始 | PR #63 已合并：`e57acb54`；双向取页、真实边界与响应式操作控件 |
| AUD-039 | `resolved` | `P2` | 前端无障碍 / 表单 | `FormField` 对直接 ReactNode 未注入 id，却生成 htmlFor，多个可见标签无法关联控件 | PR #76 已合并：`9e83772c`；control/group 标签关联合同 |
| AUD-040 | `resolved` | `P2` | 插件安装安全 | 本地包预览结果只绑定路径，确认安装前可被替换，形成审核与安装之间的 TOCTOU | PR #68 已合并：`e94c83bd`；checksum 内容绑定 |
| AUD-041 | `resolved` | `P2` | 前端错误恢复 | `attempts_json` 只校验顶层数组，`[null]` 可在详情渲染时击穿全局 ErrorBoundary | PR #51 已合并：`02b9980d`；共享 parser、链路视图和三组定向测试 |
| AUD-042 | `resolved` | `P2` | CI / 依赖安全 | `pnpm audit` 的未知或畸形成功响应会被脚本当成无阻断漏洞 | PR #51 已合并：`02b9980d`；严格响应 validator 与 selftest |
| AUD-043 | `resolved` | `P2` | CI / 签名密钥 | 更新器私钥写入 job 级 `$GITHUB_ENV`，后续脚本和 Action 都可读取 | PR #66 已合并：`d5c9cfe0`；runner-temp 文件与作用域静态合同 |
| AUD-044 | `resolved` | `P2` | Dev 制品可用性 | macOS/Linux 开发制品直接上传，下载后丢失可执行权限 | PR #59 已合并：`62574e22`；四平台下载验证通过 |
| AUD-045 | `resolved` | `P1` | 插件运行时 / 可用性 | hook 的超时不覆盖单插件锁等待、启动和整条流，且没有队列/流级预算 | PR #85 已合并：`735cec12`；Actions `31077183327` 全绿 |
| AUD-046 | `resolved` | `P2` | 插件运行时 / 资源泄漏 | 声称的 Extension Host idle recycle 没有生产调度，最后一次调用后的子进程可永久常驻 | Ready PR #74；Actions `31030917177`；合并 `94da784b` |
| AUD-047 | `resolved` | `P2` | 用量分析 / 性能 | 缓存命中率趋势接受无界日期范围与 Provider 数，前后端共同形成 `Provider × 日期` 的无界工作量 | `src-tauri/src/domain/usage_stats/trend_common.rs:7-9, 55-75, 412-420` 等 |
| AUD-048 | `resolved` | `P2` | MCP / 前端状态一致性 | 编辑已启用的 workspace MCP Server 会把缓存中的 `enabled` 覆盖为 `false` | PR #51 已合并：`02b9980d`；精确 workspace list invalidation 与 query 回归测试 |
| AUD-049 | `resolved` | `P2` | MCP / 输入资源边界 | 超过 1 MiB 的 MCP JSON 被 service 拒绝后仍会回退到无界浏览器 `JSON.parse` | PR #51 已合并：`02b9980d`；共享字符上限与 dialog 回归测试 |
| AUD-050 | `resolved` | `P2` | 插件安装 / 版本完整性 | 重复导入同 ID/版本会替换代码目录，但版本快照以 `INSERT OR IGNORE` 保留旧内容 | PR #79 已合并：`cab1229a`；Actions `31052820007` 全绿 |
| AUD-051 | `resolved` | `P2` | 插件 SDK / 合同漂移 | Host 和文档已公开 storage/diagnostics，SDK 的 `PluginApi` 类型却没有声明 | PR #61 已合并：`ba06dabb`；SDK、跨层合同与 Actions 全绿 |
| AUD-052 | `resolved` | `P2` | 前端编辑器 / 异常恢复 | `CodeEditor` 首次动态 import 失败后永久复用 rejected Promise，当前会话无法恢复 | PR #58 已合并：`5b13683b` |
| AUD-053 | `resolved` | `P2` | 供应商页 / 路由状态 | “调用顺序”始终以 Default 初始化，未默认展示当前实际活动路由 | PR #55 已合并：`ed72549b`；活动路由初始化回归 |
| AUD-054 | `planned` | `P1` | CI / 本地环境治理 | 文档与受控脚本仍引导安装依赖、dev、测试和前端构建，无法保证本地零产物 | `.trellis/tasks/08-06-cloud-only-zero-artifact-contract`；`AGENTS.md:10-11`、`README.md:193-202` |
| AUD-055 | `planned` | `P2` | Provider Sync / 磁盘治理 | 快照扫描活动与归档 sessions、SQLite/global state，并保留五代 managed backup | `.trellis/tasks/08-06-provider-sync-session-snapshot`；`src-tauri/src/infra/codex_provider_sync.rs:571,1003-1228` |
| AUD-056 | `planned` | `P1` | 请求/运行日志 / 磁盘治理 | 请求日志默认永久，运行日志只有日龄没有容量上限，清空请求日志还自动 VACUUM | `.trellis/tasks/08-06-request-runtime-log-retention`；`src-tauri/src/infra/settings/defaults.rs:65-70` 等 |

优先级口径：

- `P0`：可直接导致广泛数据泄露、任意代码执行、不可恢复数据损坏或核心功能普遍不可用，需立即阻断发布。
- `P1`：高概率或高影响的正确性、安全、数据完整性问题，或关键路径缺少有效恢复；应进入最近治理批次。
- `P2`：触发条件明确、影响有限但真实，或显著增加故障概率/维护风险；应排期修复。
- `P3`：收益明确但紧迫性较低的工程债；仅在有证据且修复成本合理时记录。

## 4. 详细问题证据

### AUD-001：定时上游同步可绕过 fork 语义审查直接更新 `main`

- 状态：`resolved`
- 优先级：`P1`
- 判断依据：该路径拥有 `contents: write`，可将未经人工语义审查的上游行为变更写入默认分支并进入后续 CI/候选制品链。触发依赖上游出现新提交，且分支保护可能降低实际落地概率，因此不定为 `P0`。
- 文件和行号：`.github/workflows/sync-upstream.yml:3-6, 14-16, 125-160, 183-205`；治理约束见 `AGENTS.md:8-9`。
- 证据与触发路径：
  1. 工作流每日定时执行，并获得内容与 PR 写权限。
  2. 当 `origin/main` 是 `upstream/main` 祖先且上游未修改 workflow 文件时，工作流执行 `git merge --ff-only` 后直接 `git push origin HEAD:${TARGET_BRANCH}`。
  3. 当历史分叉或上游修改 workflow 时，工作流创建 PR，但只轮询 GitHub 的 `mergeable` 字段；一旦为 `true` 即执行 `gh pr merge`，没有在脚本内要求 CI 成功或人工 review。
  4. Git 的可快进和 GitHub 的文本可合并性都不能判定上游变更是否与 fork 特有产品行为语义冲突，而仓库规则明确要求此类冲突暂停并由用户选择。
  5. 审计期只读查询 `origin` 的 main ruleset（`20130768`）显示已强制 PR 与 strict `ci-gate`，不要求 approval（`required_approving_review_count: 0`），且 workflow 无 bypass。因此 fast-forward 的直接 push 会被拒绝并没有 PR fallback；分叉路径仍会在 checks 后由 workflow 自动合并 PR。
- 实际影响与根因：直接 push 分支是确定的失败死路径；更关键的是非文本冲突的上游改动仍可经无人 review 的自动合并 PR 静默改变 fork 功能、配置或发布行为。根因是同步自动化把拓扑/文本兼容性误当成 fork 语义批准。
- 最小修复建议：保留定时 fetch、差异检测以及创建/更新同步 PR；删除 fast-forward 分支中的直接 `git push` 和 PR 分支中的 `gh pr merge`。让同步 PR 必须通过 required checks，并由人工完成 fork 语义审查后合并。
- 验证及回归测试：以临时 origin/upstream 构造（a）可快进、（b）历史分叉但文本可合并、（c）文本冲突三种情形；前两者应只创建或更新未合并 PR，目标分支 SHA 保持不变，第三种应留下明确人工处理状态。再核对默认分支保护确实强制 required checks 与 review。
- 剩余盲点：远端 ruleset 可变，合并前仍应重新读取；当前规则会阻止 direct push，但不会替代 fork 语义人工批准。
- 2026-08-05 当前主线复核：`origin/main@eeccf64d` 的 fast-forward 路径仍直接 `git push origin HEAD:${TARGET_BRANCH}`，分叉路径仍轮询 `mergeable` 后执行 `gh pr merge`；#51/#52 没有修改该 workflow 或同步治理合同。根因、生产触发和 P1 优先级均成立。
- 计划：子任务 `.trellis/tasks/08-05-upstream-sync-review-gate` 将两类“上游含新提交”路径统一为只创建/更新 open PR，删除目标分支 push、mergeability 轮询和自动 merge；用依赖无关 Node 合同/self-test 覆盖 direct-push、auto-merge 反例、no-op 与冲突路径，并接入现有完整 CI。真实远端行为只由 Actions 和人工审查验证。
- 2026-08-05 实施结果：分支 `codex/audit-upstream-sync-review` 的 `7463b597` 经 PR #53 以 `891c9eb3` squash 合并。同步 workflow 的领先路径现在只创建或更新 open PR，不再直推目标分支或自动合并；合同 checker/self-test 已接入 support-contract。全部必需检查通过，合并前 `origin/main` 无相关漂移或语义冲突。遗留风险是远端 ruleset 与实际人工审查质量仍属于 GitHub 侧治理，仓库合同只能阻止自动绕过。

### AUD-002：跨 SQLite/文件系统双写吞掉补偿失败，可能留下状态分裂

- 状态：`planned`
- 优先级：`P2`
- 判断依据：触发需要“原操作失败 + 恢复写也失败”的复合条件，概率低于常规错误路径；但一旦发生，数据库中的启用状态与 CLI 实际配置、manifest 或技能目录会分裂，且当前返回值和日志无法指出恢复失败，用户也没有可靠的自动修复入口。
- 文件和行号：
  - Prompt：`src-tauri/src/domain/prompts.rs:490-503, 568-580, 654-666, 695-715`
  - MCP：`src-tauri/src/domain/mcp/backups.rs:13-27, 33-48`；调用点 `src-tauri/src/domain/mcp/db.rs:577-615`
  - Skills：`src-tauri/src/domain/skills/ops.rs:309-348, 386-425`；`src-tauri/src/domain/skills/local.rs:491-525`
- 证据与触发条件：
  1. 这些流程在 SQLite 事务提交前修改外部 CLI 文件、manifest、符号链接或 SSOT 目录，随后在同步失败或 `tx.commit()` 失败时执行补偿恢复。
  2. Prompt 的 `restore_target_bytes` / `restore_manifest_bytes` 返回 `Result`，但所有上述失败分支均以 `let _ = ...` 丢弃结果。
  3. MCP 的 `SingleCliBackup::restore` 和 `restore_all` 返回 `()`，内部同样丢弃两个恢复结果；调用方随后只返回原始同步/提交错误。
  4. Skills 安装、启停和本地导入在失败时忽略删除目录、删除链接、恢复链接及事务内清理的错误。
  5. 可复现触发例：完成第一次文件写后让目标目录变为只读或占用路径，同时注入后续同步/提交失败；补偿写/删除失败，但 API 只报告原始错误，SQLite 会回滚而文件系统保留新状态，或反向留下数据库状态与缺失链接不一致。
- 实际影响与根因：Prompt/MCP 可能显示未启用但 CLI 配置仍生效，或显示启用但 CLI 文件已恢复/损坏；Skills 可能残留孤立 SSOT、丢失受管链接或错误 marker。根因是跨资源事务采用“尽力回滚”，却没有把补偿失败提升为独立故障、持久化恢复意图或提供重放式对账。
- 最小修复建议：先统一所有补偿函数返回并聚合 `Result`，任何恢复失败都返回稳定的 `*_ROLLBACK_FAILED` 错误并保留快照；同时写入一个有界的“需要对账”标记，在下次启动或显式同步时按 SQLite 权威状态重放 Prompt/MCP/Skills 配置。不要在补偿尚未确认成功时仅返回原始错误。
- 验证及回归测试：为 Prompt、MCP、Skills 的文件适配器增加故障注入点，分别覆盖首个外部写失败、第二个写失败、SQLite commit 失败、恢复目标失败和恢复 manifest/链接失败；断言成功补偿时 DB/文件一致，补偿失败时错误码包含原始错误与恢复错误、快照未删且对账标记存在；重启对账后再次断言两侧恢复一致。
- 2026-08-06 最新主线复核：`origin/main@4ee5faa8` 仍在 prompts、MCP backups/db/import、workspace switch 与 Skills ops/local 的失败分支以 `let _ =` 吞掉恢复/清理错误；现有测试只覆盖“业务失败但恢复成功”，没有恢复失败注入或 `RECOVERY_REQUIRED` 断言。AUD002 仍成立，但至少跨 Prompt/MCP/workspace/Skills 多个写路径；仅把单点 `?` 化会提前停止补偿，完整修复需要错误聚合与持久化对账/journal，故本批不 planned。
- 2026-08-06 最终治理计划：任务 `.trellis/tasks/08-06-filesystem-recovery-journal`，在 AUD-008 maintenance coordinator 合并后实施。所有外部副作用前以独立事务提交 journal，启动时以已提交 SQLite 为权威阻断重放；Prompt/MCP/Skills/workspace switch 的补偿失败不再吞掉。Skills 不可由 metadata 重建的 SSOT 内容使用 journal 专属、带 ownership/hash 的临时 staging/backup，缺失或校验失败时保持维护态。journal 与错误摘要禁止保存正文、env/header、token/secret 或原始敏感路径。

### AUD-003：Homebrew Cask 合同与真实 Release 资产矩阵分裂

- 状态：`resolved`
- 优先级：`P2`
- 判断依据：不会影响当前 Windows/macOS ARM 正式发布本身，但按仓库提供的工具或文档生成 Cask 时会得到指向不存在 Intel 桌面资产的 URL，且所谓自动 tap 更新实际不会发生，直接影响该分发渠道的可用性与版本新鲜度。
- 文件和行号：
  - 正式矩阵：`scripts/support-matrix.mjs:7-24`；`.github/workflows/ci.yml:247-267`；`.github/workflows/release.yml:114-149`
  - 漂移实现：`scripts/support-matrix.mjs:216-260`；`scripts/support-matrix.homebrew-cask.selftest.mjs:27-57, 85-96`
  - 漂移文档：`docs/release-homebrew.md:3-16, 24-45`
  - 当前产品决策：`README.md:207-209`
- 证据与触发路径：
  1. `RELEASE_TARGETS`、`main` CI 候选矩阵以及 Release 的资产校验只包含 Windows x64 与 macOS ARM 桌面包；README 也明确 macOS Intel 只属于开发制品，不进入正式 Release/updater。
  2. Cask 生成器却强制接收 ARM 与 Intel 两个 SHA，并生成 `aio-coding-hub-macos-#{arch}.zip` 双架构 URL；自测把这一遗留假设和旧仓库 `FingerCaster/aio-coding-hub` 固化为成功结果。
  3. `docs/release-homebrew.md` 声称 release workflow 会读取两个 macOS zip、生成并推送 Cask；实际 `release.yml` 在校验 ARM 资产后直接发布 Release，全文没有 Cask、tap 或 `HOMEBREW_TAP_TOKEN` 步骤。
- 实际影响与根因：Intel 用户按生成的 Cask 安装会请求正式 Release 中不存在的 `aio-coding-hub-macos-intel.zip`；即使 Release 成功，tap 也不会自动升级。根因是正式支持矩阵已收缩为 ARM 桌面包后，Cask 生成器、自测和发布文档仍各自维护旧的重复矩阵。
- 最小修复建议：以当前已明确的正式支持矩阵为准，让 Cask 生成逻辑从 `RELEASE_TARGETS` 派生并改为 ARM-only，移除 Intel 哈希必填和旧仓库默认样例；将文档改成真实的手动推送流程，或在 `release.yml` 中明确接入 KNaiFen 的 tap 同步（二者择一，不能继续声称不存在的自动化）。
- 验证及回归测试：新增合同测试从同一 `RELEASE_TARGETS` 推导 Cask 架构，逐个断言 Cask URL 对应的文件同时存在于 CI 候选与 Release `Verify candidate files`；对测试 tag 生成 Cask，并对每个 URL 做 HEAD/资产清单校验；文档测试确认所述 workflow 步骤真实存在。
- 2026-08-06 最新主线复核：`origin/main@94da784b` 的 `RELEASE_TARGETS` 仍只有 Windows x64 与 macOS ARM64，CI/Release 同时晋升 `aio-coding-hub-macos-arm.zip`，README 中英文继续明确 Intel 只属开发制品。Cask 生成器与自测仍要求 ARM/Intel 两个 SHA、输出 `#{arch}` 双架构 ZIP URL；`docs/release-homebrew.md` 仍要求 `FingerCaster` tap/token 并声称 release workflow 自动生成推送，但当前 workflow 没有 Homebrew/tap 步骤。相关生成器、自测和文档自报告形成后无主线修改；开放 #73 最新 head `30b16337` 不含三文件，没有重复、覆盖或接口冲突。根因确定成立，现有正式支持矩阵已给出 ARM-only 决策，无需新增产品选择。
- planned 实施：只修改 `scripts/support-matrix.mjs`、`scripts/support-matrix.homebrew-cask.selftest.mjs`、`docs/release-homebrew.md`。在既有 macOS ARM release target 上声明 Homebrew Cask 的 `arm64` 架构与正式 ZIP 资产名，生成器从该 target 选出唯一 Cask target，只接受 ARM SHA，输出单一 `sha256`、精确 `aio-coding-hub-macos-arm.zip` URL 和 `depends_on arch: :arm64`；显式拒绝遗留 Intel SHA 参数，避免旧调用静默被忽略。自测锁定 KNaiFen 仓库、无 Intel stanza/URL、ARM 依赖、缺少 ARM SHA/遗留 Intel 参数失败，并确认生成资产名同时存在于 CI 候选与 Release 校验。文档只描述 Release 发布后手动生成、验证和推送 tap，不新增 secret、workflow、版本、资产或依赖。
- planned 验证：先用目标 ARM-only CLI 合同证明旧实现因缺少 Intel SHA 失败，并证明旧文档仍声称自动推送；修复后运行 Homebrew self-test、CLI 正反例、Node 语法、目标 Prettier、CI/Release 资产交叉合同与 `git diff --check`。不运行本地 Rust/native 工具链；PR 前与合并前重新 fetch `origin/main` 并核对 Release/Cask 文件和开放 PR。遗留风险是 tap 仍需发布者手动更新，且当前没有联网 HEAD 测试真实尚未发布的测试 tag；这两点会在文档中明确，不伪装为自动化或实时资产验证。
- 2026-08-06 本地实施：failure-first 目标合同在旧实现上仅 1/5 通过，ARM-only 调用、精确正式 ZIP、ARM 架构依赖和真实手动文档均失败。修复严格限于上述三个文件：macOS ARM target 增加 Cask 元数据，生成器要求唯一 Cask target 并输出单 SHA/精确 ARM ZIP/`depends_on arch: :arm64`，遗留 Intel 参数明确失败；self-test 覆盖 stdout/output、缺参、遗留参数和 CI/Release 资产交叉合同；文档明确 tap 为 Release 后人工发布。目标 5/5、`pnpm check:homebrew-cask`、`check:release-promotion`、`check:release-source`、`check:tui-release-contract`、两份 Node 语法、目标 Prettier 与 `git diff --check` 均通过。完整差异复核未发现扩大范围或行为回归；PR 前最新主线门与 Actions 仍待执行，tap 人工更新及未发布 tag 不做联网 HEAD 的遗留风险不变。
- 2026-08-06 提交前主线门与 PR：再次 fetch 后 `origin/main`、分支 base 与 merge-base 均为 `94da784b`；主线没有基线后提交，唯一开放 #73 最新 head `30b16337` 不含三个目标文件或同功能实现。候选 `f1861d5a` 严格只有三个目标文件，工作树干净；Ready PR #75 已创建，等待 Actions。没有重复、覆盖、根本冲突或待决策项。
- 2026-08-06 #73 合并后漂移计划：#75 首轮 Actions `31034183720` 的 frontend、rust、support/docs contract 与 ci-gate 全绿；最终合并门查询时 #73 刚合并为 `origin/main@2a79978c`，因此未直接合并。`94da784b..2a79978c` 不修改 Cask 三文件或 CI/Release 资产合同，但把正式版本从 0.60.48 升为 0.60.49，使 Homebrew 手动步骤中的示例 tag 过期。计划将候选无冲突重放最新 main，只同步该示例 tag，重跑 Homebrew、相邻发布合同、Node/Prettier/diff 与三文件范围检查，再强推 #75 交由新一轮 Actions；无根本冲突或待决策项。
- 2026-08-06 #73 漂移整合结果：候选已无冲突重放到 `origin/main@2a79978c`，仅将文档示例 tag 同步为 0.60.49；新候选 `007e612d` 的 base、merge-base 均为 `2a79978c`，仍严格只有三个目标文件。Homebrew self-test、release promotion/source、TUI release contract、目标 Prettier、Node 语法与 diff 复验全过，已 force-with-lease 更新 Ready PR #75；第二轮 Actions `31036360425` 运行中。无重复、覆盖、根本冲突或待决策项。
- 2026-08-06 合并结果：第二轮 Actions `31036360425` 的 frontend、rust、Clippy、依赖审计、support/docs contract 与 `ci-gate` 全绿。合并前再次 fetch，`origin/main`、PR base 与 merge-base 均为 `2a79978c`，head `007e612d` 严格三文件、CLEAN/MERGEABLE，且只有 #75 开放。Ready PR #75 squash 合并为 `ff09a81a`；合并提交父节点为 `2a79978c`，三目标文件树与候选完全一致。无待决策冲突；遗留风险仍为 tap 需人工更新、未发布测试 tag 不做联网 HEAD。

### AUD-004：数据库初始化错误被缓存，启动重试无法恢复瞬时故障

- 状态：`resolved`
- 优先级：`P1`
- 2026-08-04 复核：现行 `DbInitState` 仍缓存 `Option<AppResult<Db>>`，`ensure_db_ready` 在 `src-tauri/src/app/app_state.rs:14-21` 无论成功失败都写入；基线后无针对性提交。计划任务：`.trellis/tasks/08-04-db-init-retry-recovery`。
- 2026-08-04 实施：提交 `979c6cfb` 已推送至 `codex/audit-db-init-retry`，draft PR #41。`DbInitState` 仅保存成功 `Db`；私有初始化 seam 让失败返回后保持 `None`，同时继续在同一 async mutex 内完成检查、初始化和成功写入。启动管线只提取原有 DB 阶段用于测试，没有改变阶段、错误文本或后续任务顺序。
- 修改文件：`src-tauri/src/app/app_state.rs`、`src-tauri/src/app/startup_tasks.rs`。
- 测试结果：`git diff --check` 通过；定向测试覆盖首次失败后第二次执行成功、成功缓存复用、并发成功只初始化一次，以及 `Failed/InitializingDb` 经 retry 进入 `ReadingSettings`。首轮 Actions run `30902714293` 在 cloud rustfmt 前解析阶段发现 `startup_tasks.rs` 的 match 绑定缺失分号；已由后续提交 `90fed48e` 只补该分号并推送，新一轮 CI 运行中。遵循仓库规则，未在本地运行 Cargo、rustfmt、Clippy 或 Rust 测试。
- 遗留风险：持续性的权限、目录或迁移错误仍会在用户显式重试时再次执行并失败；本项不增加自动重试或退避。`AUD-008` 的后台 DB pool 所有者与数据重置原子性仍未处理；并发测试证明初始化器只执行一次，但没有额外用 barrier 断言等待时序。
- 判断依据：数据库是几乎全部后端功能的共同前置条件；一次瞬时初始化失败会使本进程内所有后续命令和用户可见的“重试”持续失败。虽然重启应用可能恢复，因此不定为 `P0`。
- 文件和行号：`src-tauri/src/app/app_state.rs:7-21`；`src-tauri/src/app/startup_tasks.rs:9-32`；`src-tauri/src/commands/app.rs:108-119`；`src-tauri/src/app/startup_state.rs:70-76`。
- 证据与触发路径：
  1. `DbInitState` 缓存 `Option<AppResult<Db>>`，`ensure_db_ready` 无论成功或失败都把结果写入缓存。
  2. 缓存非空后直接 clone 返回；因此第一次 `db::init` 的 `Err` 在本进程生命周期内不会再次执行初始化。
  3. 启动管线把 DB 失败状态标为 `can_retry = true`，前端暴露重试按钮；但 `app_startup_retry` 只重新 spawn 同一管线，没有清除 `DbInitState`。新管线立刻取得缓存的旧 `Err`。
- 实际影响与根因：临时文件锁、瞬时权限/目录问题或一次性迁移外部条件恢复后，用户点击重试仍无法启动网关、设置、供应商、日志等依赖数据库的功能，只能重启进程或走数据重置。根因是初始化门同时承担并发去重和永久结果缓存，却没有区分可缓存的成功结果与应重试的失败结果。
- 最小修复建议：只缓存成功的 `Db`，初始化失败时把状态恢复为 `None`；保留互斥锁来串行化并发初始化。若担心命令风暴，可增加短暂退避，但显式 `app_startup_retry` 必须绕过/清除失败退避。
- 验证及回归测试：为 DB 初始化注入“第一次返回错误、第二次成功”的适配器；第一次启动应进入 `Failed/InitializingDb`，调用 `app_startup_retry` 后必须再次执行初始化并进入后续阶段/`Ready`。并发发起多个首次命令时仍应只执行一次成功初始化；已成功缓存的 DB 不应重复创建；数据重置后应继续允许重新初始化。

### AUD-005：已知 Rust panic 静态规则未进入强制 CI

- 状态：`resolved`
- 优先级：`P2`
- 判断依据：仓库已经为一个明确的运行时 panic 模式建立专用检查，说明该风险不是主观风格；但贡献者只要未主动运行 precommit，就能在 CI 全绿的情况下合入违规代码。该问题是防线缺口而非当前已发现的违规实例，因此为 `P2`。
- 文件和行号：`scripts/check-no-instant-now-sub.mjs:4-6, 26-35, 43-69`；`scripts/run-checks.mjs:17-59`；`.github/workflows/ci.yml:74-108, 110-196`。
- 证据与触发路径：专用脚本递归扫描 `src-tauri/src`，明确禁止 `Instant::now() - <Duration>`（下溢会 panic）并要求使用 checked/saturating 形式；该检查只存在于 `PRECOMMIT_SRC`，`PREPUSH_STATIC` 未包含，CI 的 frontend/support 与 Rust job 也均未调用。Clippy 和未覆盖该时序边界的单元测试不能等价替代专用合同。
- 实际影响与根因：未来代码若用较长 duration 从较短进程 uptime 的 `Instant::now()` 相减，运行时可 panic，而 PR 仍通过全部强制门。根因是 CI 与本地聚合清单分别手工维护，已知正确性合同未被提升为仓库强制门。
- 最小修复建议：把 `no-instant-now-sub` 加入 `PREPUSH_STATIC`，并在 CI 的轻量 support-contract 或 Rust job 依赖前置中执行 `pnpm check:no-instant-now-sub`；让 CI 直接复用聚合清单或增加清单一致性测试，避免再次漂移。
- 验证及回归测试：为 checker 增加临时 fixture/self-test，违规表达式必须 exit 1，`checked_sub`/`saturating_duration_since` 必须 exit 0；在 CI 日志中确认该门实际运行，并用仅含违规 fixture 的测试分支确认 PR 被阻断。
- 2026-08-05 当前主线复核：`origin/main@ef41e6da` 仍只把 `no-instant-now-sub` 放入 `PRECOMMIT_SRC`；`PREPUSH_STATIC` 与 support-contract/frontend/rust job 均未执行它，且 checker 没有独立负向 self-test。PR #53/#54 未触及该质量矩阵，根因仍成立。
- 计划：与 `AUD-006` 共用 `.trellis/tasks/08-05-ci-static-contract-gates`。只把现有 checker 提升到 prepush 与 support-contract，并增加临时仓库 fixture self-test；不修改 Rust 产品代码或扩大扫描规则。验证现仓通过、违规表达式失败、安全写法通过、聚合清单与 CI 均真实调用该门。
- 2026-08-05 执行结果：ready PR #57 在 #55/#56 合并后无冲突重放到 `origin/main@db92a480`，头提交 `400491b8`；Actions run `30979684244` 的 support-contract、frontend、rust 与 ci-gate 等必需检查全部成功。合并前再次 fetch，确认主线仍为 `db92a480`、PR 可合并且头提交未漂移，随后 squash 合并为 `0062c907`。`scripts/check-no-instant-now-sub.mjs` 改为整文件扫描，支持空白、行注释及嵌套块注释后的减法识别；新增 self-test 覆盖同一行、跨行、注释绕过、安全写法和缺失目录。`package.json`、`scripts/run-checks.mjs` 与 `.github/workflows/ci.yml` 将门接入 prepush 和无依赖的 support-contract；结构化导出的实际 `CHECKS/STAGES` 由独立 CI 质量矩阵合同保护。两轮独立审阅发现的跨行/注释绕过与死文本接线问题均已修复。本地 CI 合同、Instant 合同、plugin-hardening、脚手架 33 tests、根 TypeScript/ESLint、Prettier 和 diff 全部通过。遗留风险仅为静态规则覆盖未来新增语法形态的天然边界，不影响本项既定合同。

### AUD-006：公开插件脚手架包未被严格 TypeScript 门覆盖

- 状态：`resolved`
- 优先级：`P2`
- 判断依据：脚手架是对外插件开发入口，类型错误可能进入 main；现有测试能转译 TypeScript，但不能替代 `tsc --noEmit`。影响限定在该 workspace 包，故为 `P2`。
- 文件和行号：`packages/create-aio-plugin/package.json:6-12`；`package.json:9-10, 32-35`；`tsconfig.json:23-29`；`scripts/run-checks.mjs:17-59`；`.github/workflows/ci.yml:92-105`。
- 证据与触发路径：`create-aio-plugin` 自己声明 `typecheck: tsc -p tsconfig.json --noEmit`，但根脚本只有 `create-aio-plugin:test`；根 `tsconfig.json` 只 include `src`；prepush/plugin-hardening 以及 CI 都只对脚手架运行 Vitest，没有运行包级 typecheck。Vitest/esbuild 可在存在严格类型错误时继续完成转译和测试。
- 实际影响与根因：`cli.ts`、`devtools.ts`、`scaffold.ts` 中未被测试路径执行的类型契约回归可随主分支发布，造成脚手架命令在消费者环境构建失败或 SDK 类型错配。根因是新增 workspace 包的独立质量脚本未接入根质量矩阵。
- 最小修复建议：新增根 `create-aio-plugin:typecheck`，加入 prepush、plugin-hardening 与 CI；最好用 workspace 级递归 typecheck 作为唯一入口，避免每新增包都再次遗漏。
- 验证及回归测试：CI 明确执行 `pnpm --filter create-aio-plugin typecheck`；用一个只触发 `tsc`、不会让 Vitest 失败的负向 fixture 验证门禁确实阻断；保留现有脚手架行为测试。
- 2026-08-05 当前主线复核：`origin/main@ef41e6da` 的包级 `typecheck` 仍存在，但根 `package.json`、`run-checks.mjs` 的 prepush/plugin-hardening 和 CI frontend job 都只运行脚手架 Vitest。根 TypeScript 仍只覆盖 `src`，因此类型缺口未被其他门间接消除。
- 计划：与 `AUD-005` 共用 `.trellis/tasks/08-05-ci-static-contract-gates`。新增根脚手架 typecheck 入口并接入 prepush、plugin-hardening 和 CI；用一个包内临时类型错误 fixture 证明 Vitest 可过而 `tsc --noEmit` 必须失败，再运行包测试、包类型检查、矩阵清单/self-test 和 workflow 静态合同。
- 2026-08-05 执行结果：同一 PR #57，头提交 `400491b8`，合并提交 `0062c907`。新增根 `create-aio-plugin:typecheck` 并接入 prepush、plugin-hardening 与 frontend CI；负向 self-test 在隔离工作区保留真实 `packages/create-aio-plugin`/`packages/plugin-sdk` 相对布局、复用真实 tsconfig 与 `--noEmit`，证明 SDK 路径别名可解析且纯类型错误由 TS2322 阻断。脚手架 33 tests、包与根 TypeScript、根 ESLint、plugin-hardening、两类合同 self-test、目标 Prettier、diff 以及 Actions frontend/rust/support-contract/ci-gate 全部通过；#55/#56 合并后的主线新增只涉及 Provider/模型价格，无相关漂移或竞争实现。遗留风险是 CI 增加一次脚手架 tsc 与轻量 Node 合同耗时，属于预期质量成本。

### AUD-007：高危 RustSec 公告豁免无 owner、期限与可达性约束

- 状态：`resolved`
- 优先级：`P2`
- 判断依据：当前锁文件确实含受影响版本，两个公告均为 CVSS 7.5 的拒绝服务问题；但本轮尚未证实产品把不可信 XML 送入受影响 API，因此不将其描述为当前可远程利用漏洞。已确认的问题是 CI 对未来可达性变化也永久静默。
- 文件和行号：`.github/workflows/ci.yml:191-196`；`src-tauri/.cargo/audit.toml:1-7`；`src-tauri/Cargo.lock:3594-3604, 3760-3767, 6396-6405`。
- 外部权威证据：[RUSTSEC-2026-0194](https://rustsec.org/advisories/RUSTSEC-2026-0194.html) 与 [RUSTSEC-2026-0195](https://rustsec.org/advisories/RUSTSEC-2026-0195.html) 均标为 HIGH/CVSS 7.5，修复版本为 `quick-xml >= 0.41.0`；前者影响默认属性重复检查的二次复杂度，后者影响 `NsReader` 的无界命名空间分配。
- 证据与触发路径：锁文件固定 `quick-xml 0.39.2`，由 `plist 1.9.0` 和 `wayland-scanner` 间接引入；CI 命令行和 `audit.toml` 同时全局忽略两个 ID。注释只有“上游可升级时删除”，没有 owner、到期日、允许的反向依赖或可达 API 断言。以后新增直接 XML 解析或依赖内部改用受影响 API时，`cargo audit` 仍会成功。
- 实际影响与根因：风险接受无法过期，也无法在依赖图/可达性发生变化时自动失效，安全门会把新的实际暴露与原先认为不可达的路径一并掩盖。根因是 advisory ignore 被当成永久版本配置，而不是带条件、owner 和期限的风险接受记录。
- 最小修复建议：优先升级/patch 到不再解析到受影响版本；暂时无法升级时，为每个豁免记录 owner、到期日、锁定的反向依赖与可达性结论，并增加 CI 脚本：期限到达、依赖来源变化或应用源码直接引入 `quick-xml` 即失败。只保留一个 ignore 来源，避免双重配置漂移。
- 验证及回归测试：CI 打印并断言实际反向依赖；到期日期和新增直接依赖的 fixture 必须使门禁失败；依赖升级后删除 ignore 并确认无豁免的 `cargo audit` 通过。实际 XML 可达性验证见 `HYP-001`。
- 2026-08-06 当前主线复核：`origin/main@ecd82606` 仍锁定 `quick-xml 0.39.2`，workflow 与 `audit.toml` 共四处 ignore；明细原 `resolved` 没有提交、PR、合并或测试证据且与索引/代码矛盾，现纠正为 `planned`。当前兼容发布 `plist 1.10.0` 与 `wayland-scanner 0.31.11` 均要求 `quick-xml ^0.41`，上层版本约束允许精确更新；唯一开放 PR #79 只改插件服务/repository，无重叠。
- 计划：任务 `.trellis/tasks/08-06-rustsec-exception-removal`。首轮在 cloud canonicalize 临时精确更新两个传递包以生成 `Cargo.lock` artifact；只接受可解释的锁漂移，应用后删除临时命令。最终删除 `audit.toml`、Dependency audit 改为 plain `cargo audit`，不改 manifest/产品代码或升级其他依赖。定向验证覆盖 failure-first 源合同、零公告 ID、最终版本、精确三路径、diff、独立审查和完整 Actions；本地不运行 native 工具链。
- 2026-08-06 实施结果：首轮 `5df86904` 的 Actions `31054009003` 按计划在 drift gate 失败；artifact 只更新 `plist 1.9.0→1.10.0`、`quick-xml 0.39.2→0.41.0` 与 `wayland-scanner 0.31.10→0.31.11`。应用后删除临时命令，最终候选 `2f499dd0` 只修改 `.github/workflows/ci.yml`、删除 `src-tauri/.cargo/audit.toml` 并同步 `src-tauri/Cargo.lock`。最终源合同、YAML、diff、独立审查及 Actions `31054953851` 的 frontend、格式/绑定、Clippy、Rust tests、无豁免依赖审计与 `ci-gate` 全部通过。合并前 `origin/main`、base 与 merge-base 均为 `cab1229a`，仅自身 PR 开放且 CLEAN/MERGEABLE；Ready PR #80 squash 合并为 `b0698f57`，合并后三目标路径与候选一致。遗留风险是 XML 运行时可达性仍属 `HYP-001`，本批不建立自动依赖升级机制。

### AUD-008：应用数据重置未收口 DB 后台所有者，且删除过程可部分提交

- 状态：`planned`
- 优先级：`P1`
- 判断依据：这是用户主动触发的破坏性操作，但命令可在已经删除设置后因数据库文件仍被占用而失败，或让旧后台任务继续操作已删除的数据库 inode；结果可能是明确的数据丢失/状态分裂。影响跨设置、数据库和网关生命周期，且当前没有事务性恢复。
- 文件和行号：
  - 重置编排：`src-tauri/src/commands/data_management.rs:61-83`；`src-tauri/src/app/app_state.rs:24-31`
  - 非原子删除顺序：`src-tauri/src/infra/data_management.rs:187-212`
  - 长期 DB 所有者：`src-tauri/src/app/startup_tasks.rs:20-33, 59-70`；`src-tauri/src/infra/request_logs.rs:476-495`；`src-tauri/src/domain/provider_availability.rs:1048-1058`；`src-tauri/src/infra/usage_ledger.rs:811-854`；observer 独立只读池 `src-tauri/src/app/observer/mod.rs:459-492`、`src-tauri/src/infra/db/mod.rs:232-256`
  - 前端失败行为：`src/pages/settings/useSettingsSidebarController.ts:235-265`
- 证据与触发路径：
  1. 启动管线把 `db.clone()` 移入两个无限 interval 任务和一个可能长期重试的 blocking 回填任务；这些函数不返回 handle/cancellation token，无法由重置命令 join。
  2. 重置命令只获取网关生命周期锁、停止网关、恢复 CLI proxy，再从 `DbInitState` 取走一份缓存；这不能 drop 其他任务持有的 pool clone，也没有停止 observer/其他 DB 消费者。observer 在首次快照时还会把 `open_read_only` 创建的独立 `Db` 永久缓存于 `ObserverDbState.db`，不依赖主 `DbInitState`。
  3. 删除函数先依次删除 settings tmp/bak/main，再删除 SQLite wal/shm/main；任一步失败就立即返回，已删除的前序文件不会恢复。
  4. Windows 上仍打开的 SQLite pool 可能阻止删除 DB，从而在 settings 已删除后返回失败；Unix 允许 unlink 时，旧 pool 仍可写旧 inode，随后新 `ensure_db_ready` 可在同一路径创建另一数据库。前端仅在命令成功后延迟退出，失败时保持当前进程运行。
- 实际影响与根因：失败后可能出现“设置已重置但业务数据库保留”、网关已停、缓存已清空、旧/新 DB pool 并存，用户再次操作会面对不可预测的持久化状态；Unix 即使 unlink 成功，observer 仍可能持续展示旧 inode 中的数据。根因是后台任务与 observer 生命周期没有统一归属，数据重置把清空单个缓存误当成所有 DB 句柄已关闭，并直接执行多文件不可回滚删除。
- 最小修复建议：首选把重置改成“持久化 reset marker → 完整 cleanup/退出 → 下次启动在任何 DB 打开前原子 rename 整个数据目录到 tombstone → 初始化新目录 → 异步删除 tombstone”。若必须进程内完成，则所有 DB 后台任务必须有集中注册的 cancellation token/handle，重置前 cancel+join、停止 observer/账户用量/插件 DB 消费者并确认 pool 全部 drop；多文件删除先 rename 到隔离目录，全部成功后再清除。
- 验证及回归测试：在 Windows/macOS/Linux CI 做真实命令集成测试：启动所有后台任务并确保 pool 有活跃连接后执行 reset，断言不会部分删除、旧任务全部退出、同一路径只存在一个新 DB、设置和 DB 同时为空；分别注入 settings rename、WAL/DB rename、后台 join 超时失败，断言要么完整回滚，要么下次启动可根据 marker 完成恢复。前端失败时必须明确要求退出/重启，不能继续在半重置进程内工作。
- 2026-08-06 最新主线复核：`origin/main@ff09a81a` 仍只停止网关并取走 `DbInitState` 缓存；observer、两个 retention task 与 usage backfill 继续持有独立或 clone 的 DB pool，删除仍按 settings 后 SQLite 的不可回滚顺序执行。#76 仅修改前端 FormField 文件，无相关实现。没有可独立保证安全的窄修复：退出式 marker、进程内统一 cancel/join 或暂时禁用入口会改变产品生命周期合同，保持 `confirmed`，等待用户统一决策。
- 2026-08-06 最新主线复核：以 `origin/main@4ee5faa8` 逐点确认 `app_data_reset` 仍只锁 gateway/DbInitState；startup retention、provider availability、usage backfill、observer 独立只读 pool 和已分发的 Db clone 都可能继续存活，文件删除仍逐个 `remove_file` 且首错即返。基础测试不覆盖真实 IPC、后台 task、in-flight clone 或跨平台失败。AUD008 没有不改变生命周期契约的安全窄修复，继续 `confirmed`。
- 2026-08-06 最终治理计划：任务 `.trellis/tasks/08-06-cross-restart-data-reset`。reset IPC 只原子持久化 marker 后走禁止 `ensure_db_ready` 的专用退出；下次进程在 DB、observer、gateway、retention、usage backfill 和前端后台任务前执行幂等清理。任一删除失败保留 marker，应用进入仅允许 retry/exit 的 maintenance 状态；全部成功才清 marker并首次正常启动。该 coordinator 随后作为 AUD-002 的共享阻断 gate。

### AUD-009：脚手架生成的三个 hook 示例读取了不存在的顶层上下文

- 状态：`resolved`
- 优先级：`P1`
- 判断依据：这是公开插件开发的默认入口；三个可选模板均能安装和执行，却会在真实宿主中静默返回 `pass`，让提示词修改、敏感信息脱敏和响应保护全部失效。对 redactor 模板而言还会造成用户以为已启用的安全控制实际不生效。
- 文件和行号：宿主封装 `src-tauri/src/app/plugins/extension_host_registry.rs:343-354`、`src-tauri/src/app/plugins/extension_host_worker.rs:746-753`；错误模板 `packages/create-aio-plugin/src/scaffold.ts:97-112, 142-160, 194-203`；正确参考 `src-tauri/resources/plugins/official/privacy-filter/dist/extension.js:16-42`；同类错误文档 `docs/plugins/developer-guide.md:170-176`、`docs/plugin-manifest-v1.md:270-282`、`docs/plugins/reference/sdk.md:119-129`。
- 证据与触发路径：Registry 把 `GatewayVisibleHookContext` 序列化后构造 `{ hook, traceId, config, context: context_value }`，worker 将整个对象作为唯一参数传给插件 handler；内置 Privacy Filter 因而读取 `payload.context.request`。脚手架生成的 prompt-helper、redactor、response-guard 却分别读取 `context.request`、`context.log`、`context.response`。这些表达式在真实 payload 上得到空值，模板随后走无修改的 `pass` 分支，不会抛错或暴露配置失败。
- 实际影响与根因：使用官方生成器创建的插件在 fixture/文本测试中看似有效，装入产品后核心功能不执行；redactor 会让原本预期被替换的 token/password 继续进入请求或日志。根因是脚手架以内部可见上下文而不是公开 `PluginHookContext` 作为 handler 参数模型，且测试没有穿过生产 Registry/worker 边界。
- 最小修复建议：三个模板和相关文档统一使用 `payload.context.request/response/log`，参数命名改为 `payload` 以避免歧义；把模板行为测试改为使用 Registry 产生的真实 payload fixture，并让生成插件经同一 worker 调用路径执行。
- 验证及回归测试：分别生成三个模板并在真实宿主测试夹具中执行命中与未命中案例；断言 prompt-helper 输出 `requestBody`，redactor 同时修改请求与日志，response-guard 输出 `responseBody`，且未命中时才为 `pass`。另加合同测试，断言 handler 唯一参数顶层键固定为 `hook/traceId/config/context`。
- 2026-08-04 当前主线复核：三种模板、三份公开示例仍读取错误层级；Registry/worker 与 SDK 均明确传入外层 payload，官方 privacy-filter 已使用 `payload.context`。计划只修生成器、行为测试和点名文档，不修改宿主、SDK 或 AUD-006 的 CI 接线。
- 2026-08-04 执行结果：提交 `c8aeffb5`、draft PR #48；四个生成 handler 与三份公开示例改用 `payload.context`，VM 回归以生产外层 payload 执行命中和未命中路径。Create-aio-plugin 33 tests、包级 TypeScript、模板源码 ESLint、Prettier、四项插件合同检查、隔离 Vite build、diff 与 PR 前主线门通过。遗留风险是旧版本已生成插件不会自动迁移；Actions 待终态。

### AUD-010：插件可见上下文的多词字段与公开 camelCase 合同不一致

- 状态：`resolved`
- 优先级：`P1`
- 判断依据：受影响的是所有依赖 trace、CLI、模型、截断标记或规范化消息的第三方插件；权限校验可以成功，但插件在运行时会收到 `undefined` 并做出错误判断。合同版本标为公开 v1，修复还需要考虑兼容迁移。
- 文件和行号：运行时结构 `src-tauri/src/gateway/plugins/context.rs:257-320`；直接序列化 `src-tauri/src/app/plugins/extension_host_registry.rs:343-354`；SDK `packages/plugin-sdk/src/index.ts:190-228`；JSON 合同 `docs/plugins/plugin-api-v1-contract.json:183-220`。
- 证据与触发路径：`GatewayVisibleHookContext` 及 request/response/stream/log 子结构派生 `Serialize`，但没有 `#[serde(rename_all = "camelCase")]`；只有嵌套的 `GatewayNormalizedMessage` 有该属性。Registry 没有字段转换，直接 `serde_json::to_value(&context)`。因此运行时产生 `hook_name`、`trace_id`、`cli_key`、`body_truncated`、`normalized_messages`、`requested_model`、`message_truncated`，而 SDK/JSON 合同要求 `traceId`、`cliKey`、`normalizedMessages`、`requestedModel` 等 camelCase 字段。
- 实际影响与根因：插件按 SDK 编译后读取 `payload.traceId` 仍可取到 Registry 单独复制的顶层 trace，但读取 `payload.context.request.cliKey/requestedModel/normalizedMessages` 以及截断标记都会得到 `undefined`；依赖这些字段的审计、路由或防护逻辑可静默失效。根因是 Rust 内部结构直接成为外部 JSON DTO，却没有由合同驱动的序列化测试。
- 最小修复建议：为四个公开可见上下文结构添加 camelCase 序列化规则，并明确是否在一个兼容窗口同时接受/发出旧 snake_case 别名；把顶层 `hook_name/trace_id` 是保留、删除还是公开也纳入版本决策，避免第二套未文档化字段。
- 验证及回归测试：从各 hook 构造字段全量非空的真实上下文，做 JSON golden test并逐键对照 `plugin-api-v1-contract.json` 与 `PluginHookContext`；断言输出没有未公开的 snake_case 多词键。再用 JS 插件读取所有 SDK 字段完成端到端回归，并为旧插件兼容策略增加版本测试。
- 2026-08-06 最新主线复核：`origin/main@b0698f57` 的 request/response/stream/log 子 context 仍无 `serde(rename_all = "camelCase")`，Registry 仍直接 `serde_json::to_value`；SDK 与 v1 JSON contract 使用 camelCase，但 SDK 还遗漏 `bodyTruncated`、`normalizedMessagesTruncated`、`chunkTruncated` 和 `messageTruncated`。真实 `runtime_executor` Extension Host fixture 读取 `body_truncated`，因此破坏式改名会回归现有插件。唯一开放 PR #81 head `27213728` 不修 context casing，但与本项共享 SDK index/test、v1 contract JSON 和 hook reference；目标可兼容，文件不能并行改写。
- 2026-08-06 候选实施：#81 合并为 `871b84dc` 后重新 fetch，四个重叠合同文件与相邻 context/worker 实现没有重复修复或语义冲突；从该主线建立独立 worktree。候选严格限定八个文件：四个子 context 以 camelCase 序列化，内部 root `hook_name/trace_id` 不再进入 wire；worker 在 `JSON.parse` 后为 root 及四类多词字段安装 getter/setter snake_case v1 alias，读取和写入均指向 canonical JS value，不向 JSON-RPC payload 写双份 body/normalized messages。SDK、Rust hook matrix、JSON contract 与 hook reference 补齐 canonical truncation flags；不改 hook/权限/failure policy/timeout/budget/manifest/协议版本或依赖，也不处理 AUD-027/033/045。
- 2026-08-06 本地验证与 PR：全字段 Rust JSON golden、真实 Extension Host canonical/legacy alias（含 `normalizedMessages === normalized_messages`）、SDK 类型回归已加入；`plugin-hardening`（SDK 30 tests、两包 TypeScript）、`plugin-system-docs`、目标 Prettier、alias 静态合同、精确八文件范围与 `git diff --check` 全部通过。本机按规则不运行 Rust/native；PR 前重新 fetch 确认 `origin/main`、base 与 merge-base 均为 `871b84dc`，基线后无提交、开放 PR 为空且八路径没有重叠。
- 2026-08-06 合并结果：候选逻辑提交 `1ff29726` 在云端 rustfmt artifact 后为 `8d5ef669`。Ready PR #82 的 Actions `31063487534` 中 change-scope、pr-title、docs-contract、support-contract、frontend、Rust（含格式/绑定、Clippy、Rust tests、依赖审计）及 `ci-gate` 全部成功。合并前重新 fetch 确认 `origin/main`、PR base 与 merge-base 均为 `871b84dc`，唯一开放 PR 为自身且 CLEAN/MERGEABLE；PR #82 squash 合并为 `e6cf04d3`。合并后 `origin/main@e6cf04d3` 的八个目标文件与候选树一致。
- 遗留风险：snake_case alias 作为 v1 插件的运行时兼容表面继续存在，canonical wire 仍以 camelCase 为准；QuickJS heap 与网关 body 的端到端容量边界不属于本项，继续由 `AUD-027` 处理。

### AUD-011：启动状态的旧命令响应可以覆盖更新的事件状态

- 状态：`resolved`
- 优先级：`P1`
- 判断依据：启动失败横幅是数据库、网关等启动故障的主要恢复入口；竞态可隐藏真实失败和重试按钮，或在成功后回退为旧状态。触发依赖异步时序，但 GET、事件订阅和重试本来就是并发入口。
- 文件和行号：`src/app/startupStatusStore.ts:31-47`；`src/app/useAppEventListeners.ts:8-20`；`src/app/useAppStartupTasks.ts:9-10`；`src/components/app/AppStartupStatusBanner.tsx:31-46`。
- 证据与触发路径：应用启动时事件监听和 `appStartupStatusGet()` 独立异步执行。若 GET 在后端读取旧状态后暂未返回，随后事件把 store 更新为 `failed` 或 `ready`，最后 GET 在 `syncAppStartupStatusSnapshot` 中无条件 `set`，新状态会被旧快照覆盖；订阅真正 ready 前还存在“GET 已读、终态事件已发”的丢事件窗口。横幅内 `appStartupRetry()` 的响应也无条件写 store，可同样覆盖更晚的事件。
- 实际影响与根因：真实启动失败可能不显示，用户失去恢复入口；成功重试也可能被旧失败/初始化状态覆盖。根因是命令快照与事件作为无序多写者共享一个无版本 store，没有 revision、generation 或来源优先级。
- 最小修复建议：把“事件监听 ready + 初始快照同步”放进单一编排；前端至少记录请求开始时的 event generation，只在期间没有事件更新时接纳 GET/重试响应。长期方案是在后端状态中加入单调 revision，store 拒绝较旧 revision。
- 验证及回归测试：用 deferred Promise 固定“旧 GET 未决 → 新 failed/ready 事件 → 旧 GET 完成”和“retry 未决 → ready 事件 → retry 返回旧状态”两种顺序，断言 store 不回退；再覆盖监听 ready 之前发生终态切换的初始化测试。
- 2026-08-04 当前主线复核：初始 GET、事件 callback 和 Banner retry 仍是三个无序写入者，bootstrap 只并列挂载两个独立 effect；现有测试没有 deferred 逆序。计划先完成订阅，再以仅由事件递增的 generation 仲裁 GET/retry，不改 Rust 状态合同。
- 2026-08-04 执行结果：提交 `d4691401`（`fix(startup): prevent stale status responses`），draft PR #49。修改 `src/app/startupStatusStore.ts`、`src/app/useAppEventListeners.ts`、`src/app/useAppStartupTasks.ts`、`src/components/app/AppStartupStatusBanner.tsx`、`src/__tests__/app.bootstrap.test.tsx`、`src/app/__tests__/startupStatusStore.test.ts`、`src/components/app/__tests__/AppStartupStatusBanner.test.tsx`。监听注册完成后才启动初始 GET；事件、GET、retry 通过 generation 仲裁，活动订阅 token 使 StrictMode/卸载后的旧 GET 失效；初始 GET 失败保留 listener 并记录 warning。
- 2026-08-04 定向验证：失败优先 deferred GET/retry 及 listener ready 回归先复现旧实现覆盖，再由 6 个相关测试文件共 25 tests 通过；根 TypeScript、目标 ESLint、Prettier、Vite production build 和 `git diff --check` 均通过。独立差异审查发现并补齐“旧订阅清理与当前 GET 同代际”竞态，新增取消订阅和旧清理不影响当前订阅用例。
- 2026-08-04 PR 前主线门：重新 `git fetch origin main`，`origin/main` 与 merge-base 均为 `fef05dec20341d365aa685c3d7aee5d3a0f71c7a`，目标文件和直接调用合同无漂移；随后再次运行 25 tests、TypeScript、Prettier 与 `git diff --check origin/main...HEAD`，均通过。
- 遗留风险：前端 generation 只能依据已观察到的事件/订阅顺序仲裁，后端 `AppStartupStatus` 仍没有单调 revision；PR 尚未合并，Rust/Actions 结果待云端终态。

### AUD-012：设置 patch 实际是基于旧缓存的整文档覆盖，并发时会丢修改

- 状态：`resolved`
- 优先级：`P1`
- 判断依据：会静默回滚用户已成功保存的独立设置，界面还可能因各 mutation 的 `onSuccess` 短暂显示成功；设置包含代理、通知、网关和 WSL 等关键行为，影响横跨多个页面/卡片。
- 文件和行号：`src/query/settings.ts:76-100`；`src/services/settings/settings.ts:271-283`；多个独立 mutation 入口 `src/pages/cli-manager/useCliManagerPageDataModel.ts:107-117`；代表性并发控件 `src/components/cli-manager/GeneralTab.tsx:438-456`、`src/components/cli-manager/WslSettingsCard.tsx:515-516`。
- 证据与触发路径：每个 `useSettingsPatchMutation` 实例在请求开始时读取共享 Query cache；`createSettingsSetInput` 用该快照和局部 patch 合并后序列化几乎全部设置字段，后端收到的不是稀疏 patch。不同 hook/mutation 实例没有共同串行 scope 或版本检查。两个不相交修改从同一旧快照启动时，后完成的整份设置会把先完成字段写回旧值；各实例的 pending 状态也不能互相禁用。
- 实际影响与根因：例如较慢的 WSL 保存与通用通知/缓存设置同时进行，最终持久化结果只保留后完成请求所携带的那一项新值。根因是前端把缓存快照模拟成 patch 语义，而服务合同实际执行整文档 last-write-wins。
- 最小修复建议：近期让所有整文档 settings 写入共享同一 mutation scope/队列，并在真正执行时重新读取最新已确认缓存；根治方案是后端提供原子稀疏 patch 或 revision/ETag 冲突检测，避免客户端承担合并权威。
- 验证及回归测试：从两个独立 hook 实例同时提交不相交字段，控制两种完成顺序，断言最终后端和 cache 均保留两项修改；再加入相同字段冲突的明确策略测试，以及请求失败后不把另一成功写入回滚的测试。
- 2026-08-05 当前主线复核：`origin/main@ba06dabb` 的 `SettingsUpdate` 已排除 rectifier、circuit notice、session reuse、Codex completion、Image Gen 和 Grok 等专用 owner 字段，普通保存也在 `settings::update` 写锁内对 latest 应用 owned patch；原生测试 `ordinary_save_preserves_dedicated_owner_fields_under_lock_barrier` 已锁定这部分并发安全，因此旧报告的跨专用 writer 覆盖面已失效。剩余根因仍可确定触发：`useSettingsPatchMutation` 每个实例都没有 `scope`，执行时用共享 cache 展开全部普通字段；CLI 管理页的 common settings 与 `WslSettingsCard` 各自创建独立 hook。设置页虽在组件内串行，但 `buildPersistedSettingsMutationInput` 仍把 changed keys 扩成 20 个字段，跨页面未与 patch hook 共用队列。
- planned 实施：让普通设置 mutation 共享稳定的 TanStack scope；设置页改用同一个 patch mutation，并把 runner 变量收窄为仅含本轮 `changedKeys` 的 `AppSettingsPatch`。排队任务真正执行时继续由现有 `createSettingsSetInput` 从最新已确认 Query cache 合成后端兼容 payload，从而保留既有必填字段、validation、auto-start 显式意图、proxy password preserve 与 runtime result 语义。专用 writer 不进入该 scope，因为原生已按字段所有权隔离；不改 Rust、generated binding、IPC schema、设置默认值或依赖。
- planned 验证：先增加两个独立 `useSettingsPatchMutation` 的 deferred 失败优先回归，证明旧实现会同时发出两个基于同一缓存的整份 payload；修复后断言第二个请求在首个 success/cache sync 后才执行，最终 input/cache 同时保留两个不相交修改。模型测试断言设置页只产生 changed-key patch，并覆盖 auto-start 与失败释放队列；随后运行 query/settings、settings persistence model/runner/page 定向 Vitest、TypeScript、目标 ESLint/Prettier、隔离 Vite build 和 `git diff --check`。遗留风险是串行 scope 仅保护当前 QueryClient 内的官方前端写入；外部或未来原生普通 writer 仍需 revision/CAS，但不影响本项现有生产入口的确定性修复。
- 2026-08-05 实施与提交前门：修改 `src/query/settings.ts`、对应 query 测试、设置 persistence model/runner/hook 及两个定向测试文件，共 7 个文件。失败优先回归在旧实现上按预期出现第二个 `settingsSet` 提前发送；修复后普通设置 mutation 共享 scope，设置页只交付本轮 snake_case changed-key patch，真正执行时从最新已确认 cache 合成兼容 payload。#62 只新增三个 Observer/TUI Rust 文件，与本项无功能或文件重叠；候选无冲突重放为 `8ad4b33b` 到 `origin/main@c2e4db25`。重放后 6 个测试文件/82 tests、TypeScript、目标 ESLint/Prettier、Vite production build、diff 与完整差异审查通过，ready PR #64 已创建，Actions 运行中。
- 2026-08-05 #63 后主线门：#63 只改四个会话分页文件，与本项 7 个 settings 文件、功能目标和接口行为没有重叠；候选无冲突重放为 `f6d7d2d4` 到 `origin/main@e57acb54`。重放后 6 个测试文件/82 tests、TypeScript、目标 ESLint/Prettier、Vite build 与 diff 再次通过，ready PR #64 新一轮 Actions 运行中。
- 2026-08-05 合并结果：Actions `30999335471` 的 frontend、rust、合同与 `ci-gate` 全部成功；合并前再次 fetch，`origin/main`、PR base 与 merge-base 均为 `e57acb54`，PR 头仍为 `f6d7d2d4` 且 `CLEAN/MERGEABLE`。ready PR #64 squash 合并为 `5c756edc`，没有相关主线竞争实现或待决策冲突。
- 遗留风险：当前修复只串行化同一前端 QueryClient 内的官方普通设置 writer；跨进程、外部或未来直接调用原生普通设置写入的竞争仍需 revision/CAS。专用 writer 已由现行原生 ownership barrier 隔离，不纳入本项。

### AUD-013：删除 Provider 后相关路由缓存继续引用已删除记录

- 状态：`resolved`
- 优先级：`P2`
- 2026-08-04 复核：提交 `47509d90` 已在删除时取消、过滤并失效 Provider 列表、default route 和 sort-mode 缓存，现行实现在 `src/query/providers.ts:332-380`；回归覆盖位于 `src/query/__tests__/providers.test.tsx:1382-1513`。本轮不再修复。
- 判断依据：删除动作本身成功，但同一会话会留下可见的幽灵行，并可能让后续排序/路由写入继续携带已删除 ID；刷新或重新拉取可恢复，影响不及持久化配置丢失。
- 文件和行号：`src/query/providers.ts:314-337`；独立缓存键 `src/query/keys.ts:44-52`、`src/query/sortModes.ts:20-27`；消费路径 `src/pages/providers/hooks/useProvidersViewDataModel.ts:315-317, 603-610`；显示行为 `src/pages/providers/ProvidersView.tsx:500-520`。
- 证据与触发路径：`useProviderDeleteMutation.onSuccess` 只同步主 Provider 列表，并清理账户用量与模型目录；它没有失效 `providersKeys.defaultRoute(cliKey)` 或 `sortModes/providers/...`。这些查询仍挂载并参与路由行生成；主列表中找不到 ID 时界面明确渲染 `未知 Provider #<id>`，相关行仍可进入排序交互。
- 实际影响与根因：删除后用户会看到未知供应商，且在缓存刷新前的后续排序操作可能向后端提交含已删除 ID 的列表，导致二次错误或取决于后端容错的配置漂移。根因是 Provider 删除只按实体归属清缓存，未按查询依赖关系清理所有引用方。
- 最小修复建议：删除成功后失效该 CLI 的 default-route 与所有相关 sort-mode provider rows；同时按实际引用关系核对 circuit status/OAuth 等派生缓存。若后端删除会级联路由，应让重新查询成为完成删除 UI 状态的前置条件。
- 验证及回归测试：预置 Provider list、default route 和多个 sort-mode rows，删除被引用 Provider 后断言各引用查询被失效并重新获取；页面不得出现未知行，后续 reorder payload 不得包含已删除 ID。

### AUD-014：CLI 代理冲突检查用一个槽位表示多请求并发状态

- 状态：`resolved`
- 优先级：`P2`
- 判断依据：可导致仍在检查中的 CLI 提前解除禁用、重复 IPC，且两个冲突结果只能保留后写入者；需要用户快速操作多个 CLI，但入口本身允许这种并发。
- 文件和行号：`src/hooks/useCliProxyControls.ts:31-78`；现有测试缺口 `src/hooks/__tests__/useCliProxyControls.test.tsx:52-87`。
- 证据与触发路径：所有 CLI 共用 `checkingCliProxyCliKey: CliKey | null`，只阻止当前同 key 重入。启动 Codex 检查后再启动 Gemini 会覆盖槽位；Codex 若先完成，其 `finally` 无条件清空槽位，即使 Gemini 仍未完成，Gemini 可再次点击。两个请求都发现冲突时也都写入单个 `pendingCliProxyEnablePrompt`，后返回者覆盖先返回者。
- 实际影响与根因：同一 CLI 可产生重复冲突检查/启用请求，用户看到并确认的冲突可能对应另一 CLI，先发现的冲突没有任何提示即丢失。根因是状态模型允许多请求，却用单值槽位表达集合和队列。
- 最小修复建议：若产品允许并发，pending 改为按 `CliKey` 的集合/记录并给每个请求 generation；冲突 prompt 按 CLI 排队。更小的替代是显式全局串行化，在任一检查完成前禁用所有 CLI 开关。
- 验证及回归测试：让两个 deferred 请求按相反顺序完成，断言各自 pending 生命周期独立、同 key 不可重复发起，且两个冲突均可被逐一确认或取消；覆盖一个失败、一个冲突的组合。
- 2026-08-04 当前主线复核：同一 render 内不同 CLI 的两个调用均可穿过 state guard，后写入者覆盖 checking/prompt，任一 finally 又可清空另一请求状态。计划用同步 ref 全局串行化“启用预检 → 冲突确认”，保持关闭路径可用且不引入队列。
- 2026-08-04 执行结果：提交 `dacb7518`（`fix(cli): serialize proxy conflict checks`），draft PR #50。只修改 `src/hooks/useCliProxyControls.ts`、`src/hooks/__tests__/useCliProxyControls.test.tsx`、`src/ui/Sidebar.tsx`、`src/ui/__tests__/Sidebar.test.tsx`。同步 ref 在同一 render 内原子获取跨 CLI 锁；冲突 prompt 保持锁至取消/确认，无冲突和检查异常在 finally 释放；Sidebar 暴露全局 enable busy，只禁用新的启用入口和修复按钮，已启用代理仍可关闭。
- 2026-08-04 定向验证：失败优先运行得到 5 条失败，其中跨 key 同一 act 实际启动 2 次 `envConflictsCheck`，prompt/busy 与 Sidebar 禁用断言均不成立；修复后 hook、Sidebar、底层 hook/query/service 共 6 个测试文件 42 tests、根 TypeScript、目标 ESLint、Prettier、Vite production build 和 diff 检查通过。确认/取消后再次发起另一 CLI 的测试证明 ref 与 UI state 同时释放，第二请求不能覆盖首个 prompt。
- 2026-08-04 PR 前主线门：重新 `git fetch origin main`，`origin/main` 与 merge-base 均为 `fef05dec20341d365aa685c3d7aee5d3a0f71c7a`，4 个目标文件和直接调用合同无漂移；随后再次运行 42 tests、TypeScript、目标 ESLint/Prettier 与 `git diff --check origin/main...HEAD`，均通过。
- 遗留风险：busy 期间的新启用请求按设计直接拒绝而非排队，用户需在当前检查或 prompt 结束后重试；后端 IPC 本身仍不提供跨调用串行保证，正确性依赖所有 UI 启用入口继续通过该 hook。PR 尚未合并，frontend/rust Actions 运行中。

### AUD-015：启动失败横幅会把全高页面底部推出可视容器

- 状态：`resolved`
- 优先级：`P2`
- 判断依据：发生在用户最需要进入设置排障时，底部操作可能不可达；属于确定的布局约束矛盾，但影响随视口和页面内容而变化。
- 文件和行号：`src/layout/AppLayout.tsx:36-50`；代表页面 `src/pages/settings/SettingsPage.tsx:20-23`；现有布局测试 `src/layout/__tests__/AppLayout.test.tsx:23-44`。
- 证据与触发路径：外层内容区为 `overflow-hidden`；`main` 只声明 `flex-1 min-h-0`，自身不是纵向 flex 容器。启动失败时先渲染有实际高度/边距的 Banner，再渲染 `Outlet`。Settings 等页面根节点使用 `h-full`，因此 Outlet 仍按完整 main 高度布局，二者总高度超过 main，超出的底部被祖先裁切。
- 实际影响与根因：设置页内部滚动区的末端内容/操作无法滚到可见区域，其他 `h-full` 页面同样受影响。根因是临时横幅被插入固定全高子页面之前，却没有重新分配剩余高度。
- 最小修复建议：将 `main` 设为 `flex min-h-0 flex-col`，用单独的 `min-h-0 flex-1` 容器包住 Outlet，Banner 保持不伸展；不要依赖各页面自行减去横幅高度。
- 验证及回归测试：固定桌面小视口注入 failed 状态，在 Settings/Home 等全高页面放置底部 sentinel，断言可滚动到可见且横幅不遮挡；同时覆盖无横幅时页面尺寸不变。
- 2026-08-05 当前主线复核与计划：`origin/main@0062c907` 的 `AppLayout` 仍是 `main.flex-1.min-h-0` 下依次渲染 Banner/Outlet，Settings 根仍为 `h-full`，祖先仍 `overflow-hidden`；现有 `AppLayout` 测试只覆盖壳组件和主题。#58/#59 分别只改 CodeEditor 与 CI/dev-build，无文件、接口或最终布局冲突。任务 `.trellis/tasks/08-05-app-layout-startup-banner` 只把 `main` 变为纵向 flex，并用 `min-h-0 flex-1` 容器包住 Outlet；不改 Banner、页面高度或全局 overflow。先加结构失败测试，再运行 AppLayout/启动横幅相关 Vitest、TypeScript、目标 ESLint/Prettier、隔离 Vite build 和 diff；真实小桌面视口注入 failed 状态，验证设置页底部 sentinel 可滚动到视口且无横向溢出，无 Banner 时内容尺寸不回归。
- 2026-08-05 本地执行状态：只修改 `src/layout/AppLayout.tsx` 与 `src/layout/__tests__/AppLayout.test.tsx`。结构测试在旧实现上 1/15 失败，明确显示 main 缺少 `flex flex-col`；修复后 main 统一分配 Banner/Outlet 高度，Outlet 由 `min-h-0 flex-1` 容器承接。AppLayout 与真实 Banner 2 files / 21 tests、TypeScript、目标 ESLint/Prettier、隔离 Vite build 和 diff 通过。Playwright 在 1024x600 Settings 页面验证：failed Banner 高 70px、Outlet 高 474px，内部滚动区 `402/1742` 并滚至 `scrollTop=1340` 后内容底部恰为 580px；无 Banner 对照 Outlet 高 560px；两者 `scrollWidth=clientWidth=1024`。遗留风险仅为其他非全高 Outlet 对新 flex item 的尺寸响应，生产构建和路由主题测试已覆盖基础合同；Actions 与 PR 前最新主线门仍待完成。
- 2026-08-05 PR 状态：初始提交 `93e00021` 在 #58 合并后无冲突重放为 `a1b863c2`，最终基线和 merge-base 均为 `origin/main@5b13683b`；重放后 2 files/21 tests、TypeScript、ESLint、Prettier、隔离 Vite build 与 diff 再次通过。ready PR #60 已建立，Actions run `30984490225` 运行中。#58 只新增 CodeEditor 实现、#59 只修改 CI/dev-build，均无布局竞争实现或最终效果冲突。遗留风险保持为其他非全高 Outlet 的 flex 尺寸响应，待 Actions 和合并前最终主线门关闭。
- 2026-08-05 合并结果：#59 合并后候选再次无冲突重放为 `4d1d720a`，最终合并前 `origin/main`、merge-base 与 PR base 均为 `62574e22`，目标布局文件没有新主线实现。重放后 2 files/21 tests、TypeScript、ESLint、Prettier、隔离 Vite build、diff 和 1024x600 failed/ready 浏览器对照全部通过，Actions run `30990128465` 的 frontend、rust、support-contract 与 ci-gate 全绿；ready PR #60 squash 合并为 `d12dbfe3`。修改文件仅为 `src/layout/AppLayout.tsx`、`src/layout/__tests__/AppLayout.test.tsx`。遗留风险仅是其他非全高 Outlet 对 flex 剩余尺寸的响应，本次真实设置页与无横幅路径均未回归。

### AUD-016：网关缺少可信调用者边界，非回环模式会暴露带凭据代理并允许观测逃逸

- 状态：`planned`
- 优先级：`P1`
- 判断依据：攻击者不需要读取磁盘中的密钥即可让应用使用已保存的供应商凭据发起请求，造成配额/费用与数据外发；还可让请求不进入日志和用量账本。风险需要用户选择 LAN/通配地址或本地恶意进程，因此不定为 `P0`。
- 文件和行号：监听地址 `src-tauri/src/gateway/binder.rs:43-69`；公开路由 `src-tauri/src/gateway/routes.rs:36-118`；Provider 绕过 `src-tauri/src/gateway/proxy/handler/middleware/cli_proxy_guard.rs:15-26`；真实凭据注入 `src-tauri/src/gateway/proxy/handler/failover_loop/attempt/attempt_auth.rs:160-184`；可伪造标记 `src-tauri/src/gateway/proxy/mod.rs:42-43, 101-130`；请求收口跳过 `src-tauri/src/gateway/proxy/request_end.rs:932-940, 985-993`；已锁定行为的测试 `src-tauri/src/gateway/routes.rs:7185-7236`；UI 警告 `src/components/cli-manager/NetworkSettingsCard.tsx:187-225`。
- 证据与触发路径：
  1. LAN 模式绑定 `0.0.0.0`，Custom 也允许通配地址；`build_router` 在 `/v1/*`、`/:cli_key/*` 和 `/:cli_key/_aio/provider/:provider_id/*` 上直接进入代理，没有认证/授权 layer。
  2. 普通路由只检查相应 CLI proxy 是否启用；带 Provider ID 的路由通过 `forced_provider_id.is_some()` 直接跳过该检查。供应商尝试阶段会把数据库中保存的 effective credential 写入真实上游认证 header，客户端提交的占位认证不是访问控制。
  3. `x-aio-gateway-forwarded: aio-coding-hub` 只按固定明文比较，没有来源证明。非 Claude 请求携带它仍被转发，但 `compute_observe_request` 返回 false；request-end 随后在 `!observe` 时直接返回，不写请求日志/usage ledger。现有集成测试明确断言这种 Codex 请求成功且查不到 trace 日志。
  4. UI 只提示用户自行保证防火墙与访问控制，没有提供应用层访问令牌；防火墙也不能区分同一允许主机上的合法 CLI 和滥用请求，更不能阻止本机进程伪造内部 header。
- 实际影响与根因：同一局域网/允许网段中的进程可枚举常见路径和 Provider ID，借用用户已存的 API/OAuth 凭据调用上游，产生费用或处理攻击者数据；带伪造 header 的 Codex/Gemini/Grok 调用不会出现在请求历史、统计和 Provider 成本限制依据中。根因是网关把“能连接 socket”当成已授权客户端，并把安全敏感的内部身份放在外部可控 HTTP header 中。
- 最小修复建议：所有非回环监听强制启用高熵网关访问令牌（按客户端/设备可轮换更佳），在路由最外层、读取 body 和选择 Provider 之前统一鉴权；Provider 强制路由至少需要同等或更高权限。入口无条件移除客户端的内部 marker，内部递归状态改为进程内 request extension、不可伪造的通道，或经认证的短期签名。启用 LAN/通配地址时阻止无令牌配置，而不是只显示警告。
- 验证及回归测试：在真实 socket 上分别绑定 loopback 与 `0.0.0.0`；无 token、错误 token、过期 token访问所有三类代理路由均应在任何上游请求/DB 日志前返回 401/403，正确 token 才可代理。用正确 token但伪造内部 header 的请求仍必须被记录；只有宿主内部构造的递归 hop 可被识别。加入上游计数器断言未授权请求绝不注入/使用 Provider 凭据，并验证轮换撤销旧 token。
- 2026-08-06 最新主线复核：`origin/main@4ee5faa8` 仍让 LAN 绑定 `0.0.0.0`，路由无入站认证；公开 provider 专用路径会注入 forced provider 并绕过 CLI enable guard，客户端仍可伪造固定 `x-aio-gateway-forwarded` 以跳过观测。删除 bypass 会改变 Claude Terminal 现有启动路径，marker 清理也涉及递归/观测兼容性。AUD016 需先决定 LAN token/代理信任模型、provider 专用路由权限和 marker 语义，继续 `confirmed`。
- 2026-08-06 最终治理计划：任务 `.trellis/tasks/08-06-gateway-lan-bearer-token`。loopback 保持兼容；基于 Axum `ConnectInfo<SocketAddr>` 的真实 peer 对全部非回环 route（含 `/health`）执行最外层 Bearer 鉴权。高熵 token 只展示一次、磁盘只存摘要，旧 LAN/custom 配置自动生成，未确认即退出则下次轮换；认证后剥离 Authorization 与转发/provider 身份头。删除 provider 专用路由、全部 forced-provider 数据流和 Claude Terminal 入口。WSL 非回环连接同样带 token，一次性明文仅在生成/轮换时直接同步，失败必须可见。

### AUD-017：Responses 连续性缓存没有字节预算，可由少量大请求耗尽内存

- 状态：`resolved`（PR #67，`0854d830`）
- 优先级：`P1`
- 判断依据：缓存常驻全局进程、TTL 为 10 分钟，单项理论体积接近 128 MB 默认请求上限（环境变量还可提高到 500 MB）；条目上限 2,000 无法提供 OOM 保护。需要启用特定协议桥并得到带 tool-call 的成功响应，但这是正常产品路径而非异常内部调用。
- 文件和行号：请求上限 `src-tauri/src/gateway/util.rs:12-32`；请求整体验证 `src-tauri/src/gateway/proxy/handler/middleware/body_reader.rs:24-40`；缓存实现 `src-tauri/src/gateway/proxy/protocol_bridge/response_cache.rs:10-97, 115-168`；桥接输入 clone `src-tauri/src/gateway/proxy/protocol_bridge/inbound/openai_responses.rs:90-103`；启用条件 `src-tauri/src/gateway/proxy/handler/failover_loop/prepare/bridge_preparation.rs:113-129`；非流式写入 `src-tauri/src/gateway/proxy/handler/failover_loop/response/success_non_stream.rs:459-475`；流式写入 `src-tauri/src/gateway/proxy/protocol_bridge/stream.rs:307-349`。
- 证据与触发路径：
  1. 缓存只限制 `2,000` 个 key 和每 key `200` 个 `serde_json::Value`，没有单项/单 key/全局序列化字节预算；一个 Value 可以包含接近整个请求上限的字符串或嵌套对象。
  2. 默认请求体允许 128 MB，最大配置允许 500 MB。Responses 桥接先把 `body.input` clone 成 `expanded_input`；响应包含任一 tool-call context item 时，`cache_completed_response` 又逐项 clone 并保留 10 分钟。
  3. 非流式成功响应和流式 `response.completed` 都进入相同缓存；`get` 还会 clone 整个 `Vec<Value>` 后再拼接新 input，使连续调用产生额外峰值副本。
  4. 可复现条件：配置 `codex_to_openai_responses` bridge，发送包含单个大型 replayable message 的合法请求，让上游返回带 `function_call` 的成功响应并使用唯一 response ID；重复少量会话即可让常驻缓存达到数百 MB/GB，远早于 2,000 项上限。
- 实际影响与根因：桌面应用可能出现内存突增、系统交换、进程被 OOM 终止；在 `AUD-016` 的非回环暴露条件下还可成为远程局域网拒绝服务。根因是缓存容量按逻辑条目计数，而缓存值由外部可控、尺寸差异极大的 JSON 构成，且多次深拷贝。
- 最小修复建议：改用按序列化字节计费的 weighted LRU，同时设置保守的单 item、单 entry 和全局预算；超过单项预算时不要缓存该 response，并对需要 previous-response 工具上下文的后续请求返回明确的可恢复错误。缓存值尽量使用共享不可变字节/Arc 或受限持久化，避免 set/get 的重复深 clone；停止/重置网关时清空缓存。
- 验证及回归测试：构造少量大 Value 与大量小 Value，断言实际估算/序列化总字节始终不超过全局预算，超大单项不写入，逐出顺序稳定；分别覆盖非流式与流式写入以及 previous-response 读取。做受控 RSS 压测，连续提交接近上限的 tool-call 请求后进程峰值应被预算封顶，网关仍能处理健康检查。
- 2026-08-05 最新主线复核：`origin/main@d5c9cfe0` 的 `response_cache.rs` 仍只有 10 分钟 TTL、2,000 key 和每 key 200 item 限制，没有单条或全局序列化字节计费；#66 只改 workflow 与 CI 合同脚本，当前无开放 PR或同功能实现。`PENDING.md` 无未解决条目。选择最小边界为单文件 Rust 实现与同文件测试，不改协议/请求上限/依赖。
- planned 实施：保留现有 replayable item、TTL、namespace 和 previous-response miss 语义；先对最后 200 个可回放借用项做 1 MiB 受限预检，超限时在深拷贝前退出；`CacheEntry` 持久化最终 JSON 的受限 `Box<[u8]>`，不再常驻完整 `Vec<Value>` 并仅记录逻辑长度。写锁内清理过期项、扣除同 key 旧载荷并按最早 `created_at` 淘汰，直到全局 32 MiB 载荷预算可容纳，否则放弃写入。超限不截断，避免静默发送不完整上下文。
- planned 验证：failure-first 源码探针先确认旧实现没有字节预算；Rust 单测覆盖最终字节存储、真实 `set` 路径填满全局预算、单条超限、全局逐出、替换计费、TTL 清理、namespace 隔离和既有 item 截断。允许的本地验证只做源合同、目标格式与 `git diff --check`；Rust format/Clippy/tests/audit 交由 Actions，PR 前重新 fetch 并核对相关主线文件。
- 2026-08-05 执行结果（`resolved`）：候选 `4de2889b` 仅修改 `src-tauri/src/gateway/proxy/protocol_bridge/response_cache.rs`；最终 JSON 以 `Box<[u8]>` 常驻，写入前借用预检，替换扣费与最早创建项淘汰均由真实 1 MiB 数据路径测试覆盖。Actions `31010223216` 的 rustfmt artifact 以 `d60cc100` 精确应用，`31011383445` 的 Clippy 发现以 `4de2889b` 最小修正；最终 run `31012064253` 的 frontend、生成漂移、Clippy、Rust tests、依赖审计与 ci-gate 全绿。合并前重新 fetch 后 `origin/main`、base 与 merge-base 均为 `d5c9cfe0`，唯一开放 PR 为 #67，差异只有目标文件且 CLEAN/MERGEABLE；随后 squash 合并为 `0854d830`。
- 遗留风险（计划锁定）：32 MiB 约束覆盖持久 JSON 载荷，不包含 key、HashMap 和分配器元数据；`get` 需要反序列化最多 1 MiB 载荷并产生短时扩张，过期 idle 项仍可保留至 TTL，但持久 JSON 载荷与条目数量均有硬上限。

### AUD-018：自动 tag 发布对 annotated tag 的本地引用处理会失败

- 状态：`resolved`
- 优先级：`P1`
- 2026-08-04 复核：提交 `3620595f` 已改为将远端 tag fetch 到 `FETCH_HEAD` 并立即 peel commit，现行 workflow 见 `.github/workflows/release.yml:48-52`；`scripts/release-source.selftest.mjs:101-124` 覆盖 annotated tag 与同名本地 ref 冲突。本轮保留该合同，不再修改 tag 解析。
- 判断依据：annotated tag 是常见且受保护的发布触发方式；该工作流的 tag push 路径无法完成 Release，已在连续自动运行中发生，而手工 dispatch 可绕过，属于正式发布可用性缺陷。
- 文件和行号：`.github/workflows/release.yml:34-56, 72-96`。
- 证据与触发路径：tag push 触发时 checkout 已取得 tag 对象；工作流随后把远端 `$TAG_NAME` fetch 到同名本地 ref 再解析提交。对 annotated tag，该更新会和本地同名 tag ref 冲突，Git 拒绝 `would clobber existing tag`。审计期读取 Actions 记录，自动运行 `30736266456`、`30746603313`、`30762093164` 均在此路径失败，而 workflow_dispatch 成功。
- 实际影响与根因：维护者按正常 annotated tag 发布会得到失败工作流，必须改走手动分支或临时规避；自动发布和文档承诺的 tag 语义不一致。根因是把远端 tag fetch 到已存在的本地同名 tag ref，而不是解析已 checkout 的不可变 commit 或使用临时 ref。
- 最小修复建议：tag 触发时直接解析 `refs/tags/$TAG_NAME^{commit}`；需要远端核验时 fetch 到唯一临时 ref 后 peel 到 commit SHA，绝不覆盖同名本地 tag。下游构建保持只接收该 SHA。
- 验证及回归测试：在隔离仓库各推一次 lightweight 与 annotated tag，断言两个 `push tags` 触发均解析同一目标 commit 并创建 Release；再跑 workflow_dispatch 指向相同 tag，断言不会尝试更新本地 tag ref。

### AUD-019：同一 Release tag 的资产可以被重跑候选制品静默替换

- 状态：`resolved`
- 优先级：`P1`
- 2026-08-04 复核：workflow 仍按 `updated_at` 选择同 SHA 的最新成功候选，并在发布时设置 `overwrite_files: true`（`.github/workflows/release.yml:72-101, 139-149`）。计划任务：`.trellis/tasks/08-04-release-asset-immutability`。
- 判断依据：Release tag 是用户和更新器的信任锚；同一 tag 下二进制、校验和、元数据可在不改变 tag 的情况下变更，破坏可复现性和供应链审计。当前未发现历史被覆盖，问题在可达工作流路径。
- 文件和行号：`.github/workflows/release.yml:72, 79, 96, 139-149`；`.github/workflows/ci.yml:512-518`。
- 证据与触发路径：Release 依据同一 commit SHA 选择“最新成功”的 candidate，而 candidate `latest.json` 包含每次运行时的当前时间；Release 上传使用 `overwrite_files: true`。重新运行 candidate 或 Release 即可令同一 tag 选到另一份资产并覆盖既有 ZIP、SHA256SUMS 和更新元数据。审计期只读 GitHub API 显示 tag ruleset 禁止更新/删除 tag，但仓库 `immutable-releases` 设置为 `enabled: false`，不保护 Release asset 覆盖。
- 实际影响与根因：用户下载相同 tag 名可能获得不同二进制和校验和，排查、回滚、签名/哈希比对都失去稳定锚点，updater 还可能因版本字符串不变而不重新安装。根因是把“最新成功运行”作为发布输入且允许覆盖，而不是在首次发布时固定并验证 artifact run/digest。
- 最小修复建议：首次创建 Release 时持久化唯一 candidate run/artifact digest；后续同 tag 发布若输入不完全相同就失败，恢复必须发新 patch tag。移除 `overwrite_files`，或仅允许字节 hash 完全相同的幂等重试；启用 GitHub immutable releases 后再把其作为附加防线。
- 验证及回归测试：为同一 SHA 构造两个 candidate（不同 `latest.json`/zip digest），Release 必须拒绝歧义或只接受首次锁定 digest；对同一 digest 的重试应幂等，对不同 digest 上传必须失败。API 检查确认 Release asset 的更新被拒绝或工作流不再请求覆盖。
- 2026-08-04 实施：`caeaf348`（draft PR [#40](https://github.com/KNaiFen/aio-coding-hub/pull/40)）改为收集全部 eligible run 后要求全局恰好一个未过期 candidate；候选 checksum 清单必须覆盖所有将上传资产。已有同 tag Release 时下载其 `SHA256SUMS.txt`，仅在完整资产名集合和 digest 映射完全一致时 no-op，其他状态均失败；上传显式关闭 overwrite。
- 修改文件：`.github/workflows/release.yml`、`.github/workflows/ci.yml`、`scripts/release-promotion.mjs`、`scripts/release-promotion.selftest.mjs`、`package.json`、`.trellis/spec/aio-coding-hub/cross-layer/release-promotion-contract.md`。
- 本地验证：`node scripts/release-promotion.selftest.mjs`、`node scripts/release-source.selftest.mjs`、YAML parse、`node scripts/check-spec-links.mjs`、`node scripts/check-tui-release-contract.mjs`、定向 Prettier 和 `git diff --check` 均通过；GitHub Actions 的真实 Release 行为待 PR CI。
- PR 前主线核对：提交后再次 `git fetch origin main`，`origin/main` 仍为基线 `fef05dec` 且是 `caeaf348` 的祖先，无功能、实现或效果冲突。
- 遗留风险：GitHub 首次创建 Release 时若平台中断，仍可能留下部分资产；后续运行会 fail closed，需人工判断或新 patch tag。

### AUD-020：自动 tag 发布与手动发布没有共享并发锁

- 状态：`resolved`
- 优先级：`P2`
- 2026-08-04 复核：concurrency key 仍为 `release-${{ github.ref }}`，tag push 与手动 dispatch 对同一最终 tag 使用不同 ref（`.github/workflows/release.yml:18-20`）。与 `AUD-019` 同一任务实施，先锁定不可变输入，再按最终 tag 串行化。
- 判断依据：该竞态单独会导致相同 tag 的两条发布路径同时操作 Release；与 `AUD-019` 的覆盖能力结合会放大为不可预测资产，触发需要并发操作，因此为 P2。
- 文件和行号：`.github/workflows/release.yml:17-21, 34-40`。
- 证据与触发路径：concurrency group 使用 `github.ref`。tag push 的值是 `refs/tags/<tag>`，workflow_dispatch 的值是被选择分支（通常 `refs/heads/main`），即使两个运行最终解析同一个 `inputs.tag`，GitHub 也不会互斥，二者可并发下载、校验、创建/更新同一 Release。
- 实际影响与根因：同一发布可出现资产上传交错、一个运行读取另一个尚未完成的 Release 状态或覆盖对方文件。根因是锁按触发 ref 而不是按最终 release tag 建模。
- 最小修复建议：concurrency group 统一为 `${{ inputs.tag || github.ref_name }}`，并设置 cancel-in-progress 为 false，使同 tag 操作串行；`AUD-019` 的不可变输入校验仍必须保留。
- 验证及回归测试：对同一 tag 并发触发 push 和 dispatch，断言第二个 job 在第一个完成后才进入资产操作；不同 tag 仍可并发，且两者都不共享产物目录/Release ID。
- 2026-08-04 实施：与 `AUD-019` 同一提交/PR；顶层 concurrency group 改为 `release-${{ inputs.tag || github.ref_name }}`，保留 `cancel-in-progress: false`，使同 tag push 与 dispatch 排队而非取消。自测锁定 group、非取消语义及暂存 guard 在 source checkout 前可用。
- 遗留风险：同 tag 串行和真实 GitHub 调度只能由 PR CI/实际 Release 验证；本地静态合同和纯函数 fixture 已覆盖配置回归。

### AUD-021：前端诊断链对敏感字符串和原始异常的清洗不闭合

- 状态：`resolved`
- 优先级：`P1`
- 2026-08-04 复核：局部 IPC/console 清洗已有部分防护，但 `frontendErrorReporter` 仍把原始 message/stack/details/href 送往 console/native，Rust 边界仍只截断后持久化；主风险仍成立，故不改为 resolved。本批因跨前端、bindings 与 native tracing，未选入三个最小独立 PR。
- 判断依据：明确存在从 API key 草稿到诊断日志的可达路径，且全局 error reporter 会把原始 message/stack/details/完整 URL 再写到 native tracing；泄露的是可重用认证材料和用户内容，属于高影响隐私/凭据风险。
- 文件和行号：IPC 清洗与重抛 `src/services/generatedIpc.ts:24-138, 180-208`；第二套清洗 `src/services/consoleLog.ts:135-195`；可达剪贴板路径 `src/services/desktop/clipboard.ts:5-16`、`src/pages/providers/useProviderEditorActions.ts:7-16`；全局上报 `src/services/frontendErrorReporter.ts:112-176`、`src/services/app/frontendErrorReport.ts:45-71`、`src-tauri/src/commands/app.rs:123-140`。
- 证据与触发路径：复制 API key 草稿时，desktop clipboard IPC 将 `{ text: normalizedText }` 传给 `invokeGeneratedIpc`；失败后 `text` 不是敏感键，字符串值仅截断不脱敏，因此明文进入前端 Console。`invokeGeneratedIpc` 随后重抛原始错误；若进入未处理 rejection，global reporter 又将原始 message/stack/details 和含 query/hash 的 `location.href` 上报，Rust command 只截断并写 `tracing::error!`。相同共因还覆盖完整 TOML、prompt、MCP headers/env、插件配置/命令参数、CSV 和带签名 query 的 URL。
- 实际影响与根因：API key、Bearer/Authorization、密码、用户 prompt 或带凭据 URL 可留在前端诊断、桌面日志或 crash 采集链路，扩大本机其他用户、支持导出和日志上传时的暴露面。根因是多套 denylist 只按字段名清洗，任意字符串/raw blob 不识别 secret 模式，且“记录已清洗副本”和“继续抛出原始错误/全局上报”是分离路径。
- 最小修复建议：建立唯一的结构化诊断 redactor，规范化键名并保守清洗字符串中的 `Authorization/Bearer/api[_-]?key/password/secret` 模式；已知敏感调用只记录长度、稳定 ID 或 hash，不传原值；所有 console、IPC 和 frontend-error 出口共用该 redactor，href 去除 query/hash，禁止 native 持久化未清洗错误。
- 验证及回归测试：对 clipboard reject、TOML/MCP header/env、插件 config、`Authorization: Bearer sentinel` 的 unhandled rejection，以及 URL query/hash 放入唯一 sentinel；断言所有前端 log payload 与 Rust command 输入均不含 sentinel。保留嵌套对象、循环对象和普通错误的可诊断字段回归。
- 2026-08-05 当前主线复核：`origin/main@eeccf64d` 的 `generatedIpc.ts`、`consoleLog.ts` 仍各自维护不一致的局部清洗，任意字符串主要只截断；`frontendErrorReporter.ts` 仍把原始 message/stack/details 与完整 `location.href` 送入 Console/IPC，native `app_frontend_error_report` 只截断后写 tracing。#52 扩展观测功能但没有建立统一秘密边界，根因和生产调用面仍成立。
- 计划：子任务 `.trellis/tasks/08-05-diagnostic-secret-redaction` 新增无依赖共享 redactor，收口 generic IPC、Console、全局错误和已知 raw-text adapter，并在 native tracing 前二次保守清洗；诊断副本改变但原错误 identity、返回值和用户提示保持不变。以随机 sentinel 覆盖嵌套/循环、Bearer/header、clipboard/API-key、rejection 与 URL query/hash；本地运行定向 Vitest、TypeScript、ESLint、Prettier、隔离 Vite build，Rust 测试、格式和 Clippy 仅由 Actions 执行。
- 2026-08-05 实施结果：分支 `codex/audit-diagnostic-secret-redaction` 经三轮 CI 修正云端 rustfmt drift 和一个 obsolete helper Clippy 告警，最终 head `d9f5ae8b` 由 PR #54 以 `ef41e6da` squash 合并。共享 redactor 已覆盖 generic IPC、Console、全局错误与已知 raw-text adapter，native tracing 前再次清洗；原错误 identity 和命令行为保持不变。Actions frontend、rustfmt、Clippy、Rust tests、依赖审计和 ci-gate 全部成功。遗留风险是没有标记、无上下文的任意自然语言秘密无法可靠识别，故已知 raw blob 仍必须保持 metadata-only。

### AUD-022：内存诊断在内存压力时按 query 数量放大主线程遍历

- 状态：`resolved`
- 优先级：`P2`
- 判断依据：这是人工触发的 Console 诊断，非正常请求路径；但它恰在用户遇到内存异常时执行，最坏情况为 `query_count × 200,000` 次同步节点遍历并额外分配诊断对象，足以冻结 renderer。
- 文件和行号：`src/services/app/memoryDiagnostics.ts:6, 48-91, 128-159, 188-196`；触发入口 `src/pages/ConsolePage.tsx:414`。
- 证据与触发路径：`estimateValueSize` 每次调用都重置 `nodes` 和 `WeakSet`，最多走 200,000 个节点；`collectFrontendDiagnostics` 对 Query cache 的每一条 query 都调用它，既没有全局节点/字节/时间预算，也没有 yield。大量复杂查询会让总工作量线性叠加，且 `topQueries`/groups 继续分配诊断数据。
- 实际影响与根因：在最需要排障时界面可能长时间无响应，恶化内存压力。根因是预算被放在单个值估算函数中，而业务操作是对整份快照扫描。
- 最小修复建议：让所有 query 共享总节点、估算字节和 wall-clock budget，耗尽后停止并标记全局截断；必要时按批 `queueMicrotask`/scheduler yield。保留 top-N 但避免先完整构建无界中间数组。
- 验证及回归测试：预置大量深层 query，断言总访问节点不超过共享 budget、返回 `global_truncated` 且聚合字段可用；模拟慢设备/时间预算，断言调用可让出事件循环。
- 2026-08-05 最新主线复核：`origin/main@e94c83bd` 的 `estimateValueSize` 仍在每次 query 调用时新建 `WeakSet` 并重置 200,000 节点额度，`collectFrontendDiagnostics` 仍同步扫描 `getAll()` 的每一项、先构建完整 diagnostics/groups 再对完整数组排序取 top-20；Console 生产按钮仍直接调用该路径。#68 只改插件安装链，开放 PR #69 只改 Image Gen adapter 与测试，均无功能、接口或文件重叠；现有测试没有整次快照预算回归，根因仍成立且有确定修复价值。
- planned 实施：保留同步 API、现有估算规则、query 总数和正常小缓存输出；让整次快照共享 200,000 节点预算，并增加最多 2,000 条 query 的扫描上限。任一预算耗尽后停止后续 query，结果新增 `scanned_query_count` 与 `scan_truncated`，估算字节和 groups 明确代表已扫描部分；top query 改为最多保留 20 个候选后再排序，避免完整中间数组。只修改 `src/services/app/memoryDiagnostics.ts` 及其定向测试，不改 Console、backend diagnostics、IPC、依赖或其他 query 行为。
- planned 验证：先加入 failure-first 回归，证明旧实现会让多个复杂 query 各自获得 200,000 节点额度且没有截断元数据；再覆盖共享节点预算在第二项耗尽后不扫描第三项、2,000 query 上限、top-20 排序/数量和正常小缓存聚合不变。随后运行目标 Vitest、TypeScript、目标 ESLint/Prettier、Vite build 与 `git diff --check`。遗留风险是同步调用仍没有 wall-clock deadline 或事件循环 yield，且 `getAll()` 自身会返回完整引用数组；硬预算已消除按 query 数乘算的最坏遍历与诊断对象无界增长，异步分批需另行评估 UI 生命周期和取消合同。
- 2026-08-05 合并结果：仅修改 `src/services/app/memoryDiagnostics.ts` 和 `src/services/app/__tests__/memoryDiagnostics.test.ts`。旧实现新增两项预算回归为 2 failed / 2 passed；独立审查发现宽对象 `Object.entries` 会预算前预读全部值，补充 getter 回归后旧枚举按预期抛错，改为预算感知逐属性读取后目标 5/5。整次快照现共享 200,000 节点、最多 2,000 query，返回总数/已扫描数/截断标记，稳定 top-20 始终有界。10 个 service 测试文件/52 tests、TypeScript、目标 ESLint/Prettier、隔离 Vite build、diff、两轮独立审查与 Actions `31020581604` 全绿。#69 只改 Image Gen 两文件并以 `9a280136` 合并，候选无冲突重放为 `35491b78`，base/merge-base 均为该主线；最终 Ready PR #70 于无相关开放 PR 竞争时 squash 合并为 `5d4906c5`。遗留风险是 `QueryCache.getAll()` 仍复制完整引用数组，同步 API 仍无 wall-clock deadline/yield；不在本批扩展 TanStack 或 UI 生命周期合同。

### AUD-023：图片生成响应可驱动无界 URL 下载 fan-out

- 状态：`resolved`
- 优先级：`P2`
- 判断依据：恶意或异常上游可在单个允许的 JSON 响应中返回任意长度 `data`，前端会为每个 URL 串行请求后端下载；单图 32 MiB 和 600 秒默认超时不能约束总请求数或总字节。
- 文件和行号：`src/services/image-gen/gptImageAdapter.ts:176-240`；UI 请求数范围 `src/pages/image-gen/ImageGenParamsPanel.tsx:187-198`；单次后端限制 `src-tauri/src/domain/image_gen/transport.rs:9-20, 153-194`。
- 证据与触发路径：`parseImagesResponse` 无条件遍历所有 `data`，每遇 URL 都 `await fetchImageByUrl`；生成调用没有把 `req.n` 或硬上限传给该解析器。上游只需返回大量 URL 项，就能使一次 UI 生成触发大量最长 600 秒、各最多 32 MiB 的下载；已有 SSRF 校验只能限制目标地址，不限制 fan-out。
- 实际影响与根因：renderer 长时间被占用，后端网络连接、带宽与内存被耗尽，用户请求取消也没有成为下载集合的明确 cancellation boundary。根因是把不可信响应的数组基数当作已请求的图片数量。
- 最小修复建议：解析前后都按 `min(req.n, 10)` 拒绝超量 data，在首次下载前建立总请求数/总下载字节预算与 AbortSignal；若保留部分成功语义，显式返回被丢弃数量而不是静默继续。
- 验证及回归测试：返回大于请求数和大于 10 的 URL 数组时，断言零下载调用并返回可诊断错误；覆盖聚合字节预算、取消、混合 b64/URL 顺序及正常 `n=1..10`。
- 2026-08-05 最新主线复核：`origin/main@0854d830` 的 `parseImagesResponse` 仍遍历任意长度 `record.data`，每个 URL 都串行调用 `fetchImageByUrl`，`generate` 仍未传入 `req.n`；Rust `fetch_image` 仍只提供单图 32 MiB、单请求 timeout、重定向和 SSRF 边界。`eeccf64d..0854d830` 没有 Image Gen 路径变化，唯一开放 PR #68 也只有插件文件，因此没有重复、覆盖或接口冲突。
- planned 实施：在 adapter 定义响应图片硬上限 10；`parseImagesResponse` 接收本次请求数量，先验证为正安全整数并取 `min(req.n, 10)`，在任何 Base64 收集或 URL 下载前整体拒绝超过该数量的 `data`，不静默截断或返回部分成功。`generate` 显式传入 `req.n`。现有 Rust 单图 32 MiB 边界由此形成最多 `min(req.n, 10) × 32 MiB` 的确定总下载上界；本批不修改 Rust IPC、timeout、SSRF 或依赖。
- planned 验证：先加入 failure-first 回归，覆盖默认硬上限 10、请求 `n=1` 却返回两项、超量响应零下载，以及正常混合 Base64/URL 的顺序；增加 adapter 级测试证明 `generate` 把 `req.n` 传入该门。随后运行目标 Vitest、TypeScript、目标 ESLint/Prettier、Vite build 与 `git diff --check`。遗留风险是已开始的单图下载仍没有跨 IPC AbortSignal，且合法 `n=10` 仍可能达到 320 MiB/较长串行时延；这两项是有界但偏高的产品预算，需另行设计批量下载/取消合同，不能在本批顺手扩大。
- 2026-08-05 本地实施结果：候选提交 `b8303703` 仅修改 `src/services/image-gen/gptImageAdapter.ts` 与其测试。旧实现 failure-first 为 45 tests 中 3 项失败：11 项响应、`n=1` 两项响应和 adapter 生产路径均会开始下载；修复后 Image Gen 8 个测试文件共 223 tests、目标 adapter 48 tests、TypeScript、目标 ESLint/Prettier、Vite build、源合同与 `git diff --check` 全部通过。超量门在首次 Base64/URL 处理前执行且下载调用为 0；尚未创建 PR，待 #68 合并后按最新 main 重放。
- 2026-08-05 合并结果：#68 以仅含插件文件的 `e94c83bd` 合并后，候选无冲突重放为 `f703c863`，base/merge-base 均为该主线，仍只修改两个 Image Gen 文件。重放后 8 files / 223 tests、TypeScript、目标 ESLint/Prettier、Vite build 与 diff 再次通过；Actions run `31017816818` 的 frontend、rust、合同与 ci-gate 全绿，最终主线无漂移且 PR CLEAN/MERGEABLE。Ready PR #69 squash 合并为 `9a280136`。遗留风险保持为单图下载缺少跨 IPC 取消，以及合法 10 图仍可能达到 320 MiB。

### AUD-024：畸形 percent-encoded Release tag 会让更新检查整体失败

- 状态：`resolved`
- 优先级：`P3`
- 判断依据：触发需要 updater 返回特定畸形 fallback URL，影响限于更新说明二次获取，原始更新检查可恢复，因此为 P3；但异常是确定的且修复极小。
- 文件和行号：`src/services/app/updater.ts:27-58, 90-105`。
- 证据与触发路径：`new URL` 的异常被捕获，但 `decodeURIComponent(encodedTag)` 在 try/catch 之外。fallback body 为 `See release: https://github.com/KNaiFen/aio-coding-hub/releases/tag/%ZZ` 时，解码抛出 `URIError`，`resolveGitHubReleaseFallbackBody` 在其 fetch try/catch 之前拒绝，`updaterCheck()` 整体 reject。
- 实际影响与根因：有效的 updater 结果会因为可选 release body 解析失败而丢失，UI 不能显示更新。根因是外部文本解析只保护 URL 构造，未保护 percent decoding。
- 最小修复建议：将 decoding 放入同一 try/catch，失败返回 null/保留原 `update`；或避免 decode/re-encode 路径。
- 验证及回归测试：`%ZZ`、截断 UTF-8、合法编码 tag 三种输入分别断言前两者保留原结果且不 fetch，后者生成正确 API URL。
- 2026-08-04 当前主线复核：`origin/main@fef05dec` 的 `decodeURIComponent(encodedTag)` 仍在 `new URL` 异常边界之外，现有 updater service 测试未覆盖畸形编码；根因和最小修复路径均成立。
- 计划：子任务 `.trellis/tasks/08-04-updater-fallback-decode-guard` 仅收口 tag 解码异常；失败优先覆盖两类畸形编码与合法编码，随后运行 updater 定向 Vitest、TypeScript、ESLint、Prettier、Vite build、PR 前主线漂移门和 GitHub Actions。
- 执行结果：提交 `7fbfd6c5`，draft PR #43；`decodeURIComponent` 失败返回无 fallback，调用者沿既有 fail-soft 路径保留 updater 结果。新增 `%ZZ`、截断 UTF-8 与合法编码 tag 回归；10 个 updater tests、3 个关联 query tests、TypeScript、ESLint、目标 Prettier、Vite build 和 diff 检查通过。PR 前再次 fetch 的 `origin/main` 仍为 `fef05dec`，updater、query/hook 调用者与测试无主线漂移；GitHub Actions 正在运行。

### AUD-025：递归 guard 没有生产跳数标记，自指 Provider 会反复回入网关

- 状态：`resolved`
- 优先级：`P2`
- 判断依据：错误配置一个 API-key Provider 的 base URL 即可触发，造成连接/超时资源耗尽；需要用户配置错误而非外部默认攻击，因此为 P2。
- 文件和行号：标记读取/guard `src-tauri/src/gateway/proxy/mod.rs:42-43, 101-109`、`src-tauri/src/gateway/proxy/handler/middleware/recursion_guard.rs:15-37`；上游发送 `src-tauri/src/gateway/proxy/handler/failover_loop/attempt/attempt_executor.rs:203, 351-402, 483-488`；Provider URL 校验 `src-tauri/src/domain/providers/validation.rs:160-199`；仅 proxy URL 的自环检查 `src-tauri/src/gateway/http_client.rs:217`。
- 证据与触发路径：全仓生产代码没有插入 `x-aio-gateway-forwarded: aio-coding-hub`，只存在读取、测试和指纹逻辑；Provider URL 校验只允许 HTTP(S)，不拒绝当前网关地址。把 Provider base URL 设为本机 gateway `/v1` 后，首跳不含 marker，网关再把请求选回同 Provider，guard 始终 continue，直到 socket/超时资源耗尽。
- 实际影响与根因：一个误填的 Provider 可以让每个匹配请求递归创建嵌套转发，拖慢或耗尽本地网关。根因是循环保护依赖从未写入的外部 HTTP marker，同时缺少在发送边界对 self target 的确定性拒绝。
- 最小修复建议：在每次 provider send 前用 gateway 的实际监听/自检上下文拒绝目标为本机 gateway 的 URL；若仍需跨代理跳数机制，采用不可伪造且有限跳的内部 context，并在入站剥除控制 header。
- 验证及回归测试：配置自指 Provider 后请求必须在一次受控错误内结束，上游/网关连接计数不增长；覆盖 loopback、LAN base URL、尾随 `/v1` 与不同大小写/端口规范化。
- 2026-08-06 最新主线复核：`origin/main@ff09a81a` 的 Provider 保存仍只校验 HTTP(S)，所有生产出站路径仍未写入递归 header；`GatewaySelfCheckContext` 已在网关启动前同步实际 port 与 loopback/LAN/custom host，可直接复用。唯一开放 #76 只改 14 个 FormField 前端文件，无文件或功能重叠，也不存在必须二选一的主线冲突。
- 计划：Trellis 子任务 `.trellis/tasks/08-06-provider-self-loop-guard` 仅修改 `gateway/http_client.rs` 与 `attempt_executor.rs`。先以失败测试锁定当前完整 target URL 未被拒绝，再将现有 host/port 匹配收敛为可复用的 URL 自检，并在 `build_target_url` 后、fingerprint/发送前沿既有 URL-build failure 路径拒绝当前实例；使用 `port_or_known_default` 覆盖监听 80/443 的省略端口，不复活 header、不改 Provider schema、路由选择或 attempt budget。
- 验证计划：定向 Rust 单元/路由测试覆盖 loopback、大小写、LAN/custom host、IPv6、尾随 `/v1`、默认端口、不同端口和外部 host，并证明 self target 没有 upstream send 且后备 Provider 仍可接管；执行 diff/静态合同检查，原生编译、格式、Clippy 与 Rust 测试仅在 GitHub Actions 运行。PR 前和合并前重新 fetch `origin/main` 并核对相关漂移。
- 2026-08-06 本地实施与提交前门：严格只修改 `src-tauri/src/gateway/http_client.rs` 和 `src-tauri/src/gateway/proxy/handler/failover_loop/attempt/attempt_executor.rs`。共享 URL 判定复用当前 runtime context，并以 `port_or_known_default` 保持 proxy 旧用途同时覆盖 Provider 的 80/443 省略端口；attempt 在完整 target URL 构造后立即验证，失败沿既有 `UrlBuildFailed → SwitchProvider`，位于 body finalize、fingerprint、`upstream_sent` 和网络发送之前。回归覆盖 loopback、host 大小写、LAN/custom host、IPv6、尾随路径/default port、同 host 不同端口和外部 host。按规则未本地运行 Rust；`git diff --check`、两文件范围和发送顺序源合同通过。
- 2026-08-06 最新主线整合、验证与合并：初始候选无冲突重放 #76 合并后的主线，并在两轮独立差异审查后补齐任意本机 DNS/hosts 别名、自定义监听 hostname 展开、750ms 解析期限、30s/5s 正负缓存、128 项容量上限和 runtime context 变更清空。最终逻辑/格式/Clippy 修正 head 为 `b7f5378c`；严格只修改 `src-tauri/src/gateway/http_client.rs` 与 `src-tauri/src/gateway/proxy/handler/failover_loop/attempt/attempt_executor.rs`。`git diff --check`、两文件范围、发送顺序与同步锁不跨 await 的源合同通过；Actions `31047727848` 的 frontend、云端格式/绑定、Clippy、Rust tests、依赖审计和 `ci-gate` 全绿。
- 2026-08-06 合并门与结果：合并前再次 fetch `origin/main@60b12aa4`，PR base/merge-base 与远端一致，CLEAN/MERGEABLE，唯一开放 PR 为 #78；#77 仅归档 Trellis/发布证据，主线没有目标文件或同功能实现。Ready PR #78 squash 合并为 `ecd82606`，合并后 `origin/main` 的两目标文件与候选树完全一致。遗留风险：发送前 DNS 校验不能彻底阻止校验后、连接前的恶意 DNS 重绑定；配置保存期仍不提前提示，错误沿用既有 URL failure code。彻底消除 DNS 重绑定需连接层地址固定，超出本次 P2 最小修复边界。

### AUD-026：插件 fail-closed 安全合同在日志入口和 manifest 校验处可静默降级

- 状态：`resolved`
- 优先级：`P1`
- 判断依据：官方 Privacy Filter 明确将 `log.beforePersist` 标为 fail-closed；插件超时、崩溃或格式错误后原始日志仍可入库。另一个 typo 即可让磁盘 manifest 的预期 fail-closed 退化为 fail-open，直接影响隐私/安全 hook。
- 文件和行号：官方声明 `src-tauri/resources/plugins/official/privacy-filter/plugin.json:27-44`；pipeline 策略/熔断 `src-tauri/src/gateway/plugins/pipeline.rs:276-302, 535-561, 767-793, 969-995, 1011-1089, 1163-1168, 1195-1228, 1386-1397`；日志入口 `src-tauri/src/gateway/proxy/logging.rs:320-369`；安装校验 `src-tauri/src/domain/plugins.rs:1070-1079`；SDK 校验 `packages/plugin-sdk/src/index.ts:588-619`；设计承诺 `docs/plugin-system-rfc.md:139-151`。
- 证据与触发路径：pipeline 对一个真正的 fail-closed log hook 错误返回 `Err`，但 logging 层捕获后只 warning 并以原 payload 继续持久化。与此同时，Rust 只拒绝 `timeoutMs=0`，SDK 只要求正整数，未知 `failurePolicy` 在运行时默认映射为 `FailOpen`；原始 `plugin.json` 写 `failclosed` 或极大 timeout 即可触发错误策略降级或长时间请求占用。更严重的是 circuit 只按 `plugin_id` 计数：任一 hook 三次失败后打开 30 秒，request/response/stream/log 四个入口都无视其自身 failurePolicy、直接 skip 并沿用原数据；一个 log hook 故障也能绕开同插件的 request privacy hook。
- 实际影响与根因：含 request body、attempt/error、special settings 等敏感字段可能在官方脱敏插件失效时原样外发或落库；第三方安全插件配置拼写错误也不会在安装期失败。根因是 manifest、pipeline circuit 和最终持久化出口没有共享的封闭策略枚举、最大 timeout、hook 维度隔离与 fail-closed 终态语义。
- 最小修复建议：安装/更新阶段只接受 `fail-open|fail-closed` 并强制公共 timeout 上限；日志 fail-closed 错误必须走宿主红线脱敏或丢弃敏感日志，绝不能回落原文；circuit open 对 fail-closed 必须保守拒绝/终止，至少按 `(plugin_id, hook)` 隔离。将 SDK/Rust/运行时约束生成自同一合同。
- 验证及回归测试：针对官方 Privacy Filter 的 crash、timeout、context truncation、非法输出分别注入 sentinel secret，断言持久化和上游都不含 sentinel；连续失败达到 circuit 阈值后，request/log/response/stream 四条路径不得原样继续（stream 必须终止而非仅发送错误 marker）；policy typo 和超大 timeout 均被拒绝或明确夹紧。
- 2026-08-06 当前主线复核：初始 `origin/main@cab1229a` 的三类日志绕过、未知 policy 降级和跨 hook circuit 均仍成立；当时 #80 只改 CI/audit/lock。#80 随后以 `b0698f57` 合并，仍与本项 12 个目标文件及功能零重叠，远端无其他开放 PR。AUD-021 native 清洗器只处理前端错误字符串，前端 recursive redactor 不在 Rust 网关，Extension Host privacy service 又依赖可能正失败的插件，均不能完整覆盖 request-log 字段，故不作为 fallback。
- 计划：任务 `.trellis/tasks/08-06-plugin-fail-closed-persistence`。本批以既有 fail-closed 声明直接推出“不持久化未经成功处理的原文”：error/invalid payload/自身 circuit-open 在 request-log channel/write-through 前停止，插件诊断保留；fail-open 保持原行为。circuit 改为 `(plugin_id, hook)`，四类 hook 的 fail-closed circuit-open 返回封闭终态；快照替换后旧快照的在途执行不得重新写入已清理的 circuit。Rust/SDK 拒绝未知 policy，运行时 legacy 未知值按 closed。同步七份直接合同文档。公共 timeout 上限、排队/冷启动/流总 deadline 明确保留在 AUD-045，不在本批扩张。
- 2026-08-06 实施状态：候选将所有 16 个 success/failure circuit 写入绑定取得插件列表时的 snapshot 身份，以固定 snapshot→circuit 锁顺序阻止替换前在途 hook 清理后回写；新增 hook 隔离、移除 hook、在途刷新、fail-closed log timeout/circuit/invalid payload 与 policy typo 回归。logging 在 `RequestLogInsert`、channel reserve 和 write-through 前统一早停；SDK 30 tests、两包 TypeScript、plugin docs/API contract、七文档 Prettier、静态源合同、精确 12 文件和 diff 通过。初始提交 `4255a154` 在 #80 合并后无冲突重放为 `fd5ab186` 到 `origin/main@b0698f57`；Rust/native 验证等待 Actions。
- 2026-08-06 云端与合并结果：首轮 Actions `31060135548` 的 frontend、docs/support contract 均通过，Rust 仅在 generated-file drift 门失败。artifact 只有 `pipeline.rs` 的 rustfmt 换行（59 insertions/50 deletions），没有逻辑、绑定或额外文件；原样提交为 `27213728` 后，plugin-hardening 4/4（SDK 30 tests）、plugin-system-docs、两包 TypeScript、目标 Prettier、单文件 artifact 范围与 diff 再次通过。第二轮 Actions `31060862654` 的 frontend、格式/绑定、Clippy、Rust tests、百万行基准、依赖审计与 `ci-gate` 全绿。合并前 fetch 确认 `origin/main`、PR base 与 merge-base 均为 `b0698f57`，只有 #81 开放且 CLEAN/MERGEABLE；Ready PR #81 squash 合并为 `871b84dc`。

### AUD-027：插件上下文 body 预算与 QuickJS heap 不相容，合法大请求会被 fail-closed 钩子阻断

- 状态：`resolved`
- 优先级：`P1`
- 判断依据：网关默认允许 128 MiB、可配置到 500 MiB，而 Extension Host 的 QuickJS heap 仅 32 MiB且完整 JSON parse context；官方隐私插件的请求 hook 是 fail-closed，因此达到阈值的合法请求会稳定不可用。
- 文件和行号：网关大小 `src-tauri/src/gateway/util.rs:12-33`；插件 context 预算/投影 `src-tauri/src/gateway/plugins/context.rs:22-32, 117-153`；序列化 `src-tauri/src/app/plugins/extension_host_registry.rs:343-354`；worker 传输/heap/parse `src-tauri/src/app/plugins/extension_host_worker.rs:15-24, 241-243, 746-754`；请求权限 `src-tauri/src/gateway/plugins/contract.rs:65-125`。
- 证据与触发路径：请求 body 在网关可完整读入并进入可见插件 context，再整体 JSON 序列化/传输/`JSON.parse`；transport 行上限甚至约为网关限额的六倍，而 QuickJS 堆固定 32 MiB。任何超过实际 JS 表示能力的合规 body 在含 `request.body.read` 的官方 fail-closed hook 上会使 worker 错误，继而阻断网关请求，并产生多次大对象复制。
- 实际影响与根因：大上下文、附件或工具输出请求的成功率会随是否安装官方隐私插件而变化，内存峰值也被序列化链放大。根因是网关 body、插件 context、进程间传输和 JS heap 分别定义上限，没有端到端容量合同。
- 最小修复建议：引入低于 QuickJS 可用堆的独立插件可见 body byte cap，统一截断标记并禁止截断 body 的危险 mutation；协调所有 transport/heap/body 上限，必要时对超限 body 跳过非必要 hook 并按 fail-closed 产品决策返回专门错误。
- 验证及回归测试：在 QuickJS 实际 heap 下做阈值边界请求，断言小请求仍可被隐私 hook 修改，大请求按已文档化策略完成或返回稳定错误且不 OOM；测量序列化峰值与截断标记，覆盖 request/response/log 三种 context。
- 2026-08-06 最新主线复核：重新读取 `PENDING.md`（无未解决条目）并 fetch `origin/main@e6cf04d3`。网关 `max_request_body_bytes()` 仍为默认 128 MiB、环境变量 1 至 500 MiB；`GatewayPluginContextBudget` 与 `GatewayPluginMutationBudget` 的 body 上限仍直接取该值。Extension Host worker 的 QuickJS heap 仍固定 32 MiB，JSON-RPC 行上限却按网关 body 的六倍加 1 MiB 派生；context 经 Rust value/string、worker `JSON.parse` 与结果 `JSON.stringify` 形成多份表示。官方 Privacy Filter 的请求及日志安全 hook 仍为 fail-closed，现有真实大 body 回归仅覆盖 300 KiB。远端无开放 PR，也没有主线同功能实现或文件竞争。
- planned 实施：任务 `.trellis/tasks/08-06-plugin-context-quickjs-budget`。在 `context.rs` 定义独立 1 MiB 插件可见 body 上限，在 `mutation.rs` 对齐插件 body 输出上限，两者均不再继承可配置网关上限；保留 stream/log 64 KiB 与 normalized message 预算。`pipeline.rs` 在四类直接可见内容（request body、response body、stream chunk、log message）被截断且当前 hook 为 fail-closed 时，于调用 executor 前返回既有稳定码 `PLUGIN_CONTEXT_TRUNCATED`，写入 `budgetRejected/context_budget` audit/report，且不计入 circuit failure。只出现 `normalizedMessagesTruncated` 而完整 body 未截断时不拒绝。fail-open hook 继续接收有标记的有界 context，截断字段 mutation 继续由现有规则拒绝；不静默跳过安全 hook，不扩大 QuickJS heap 或 transport 行上限。
- planned 文件边界：`src-tauri/src/gateway/plugins/context.rs`、`src-tauri/src/gateway/plugins/mutation.rs`、`src-tauri/src/gateway/plugins/pipeline.rs`、`src-tauri/src/app/plugins/runtime_executor.rs`、`docs/plugins/plugin-api-v1-contract.json`、`docs/plugins/reference/hooks.md`、`docs/plugins/examples/privacy-filter.md`。测试内联在上述 Rust 文件；不改网关 body 配置、worker 协议、官方插件实现、manifest、SDK 字段、hook timeout/队列、generated binding、依赖或其他审计项。
- planned 验证：先增加默认预算与 cap-1/cap/cap+1 失败优先合同；用真实 Extension Host 在固定 32 MiB heap 下执行上限内 request，证明 canonical/legacy context 与 300 KiB Privacy Filter 回归保持可用。pipeline 覆盖 request/response/stream/log 的 fail-closed 截断均在 executor 前稳定拒绝、带 audit/report、不开 circuit、不暴露原内容；另覆盖 fail-open 仍执行且不能修改截断字段、normalized-only 截断不误拒绝。运行 `plugin-hardening`、`plugin-system-docs`、目标 JSON/Markdown Prettier、静态源合同、精确七文件范围与 `git diff --check`；本机不运行 Rust/native，格式/绑定、Clippy、Rust tests、依赖审计和 `ci-gate` 交由 GitHub Actions。PR 前和合并前重新 fetch `origin/main` 并核对功能、接口、最终效果与开放 PR。
- planned 遗留风险：1 MiB 是针对当前 32 MiB heap 及多份 JSON 表示的保守边界，云端真实 Worker 测试证明可执行但不替代长期内存峰值遥测。安装官方 fail-closed Privacy Filter 时，超过该上限的 body 将被稳定拒绝而不是完整脱敏后继续；若产品必须接受并完整扫描更大 body，需要后续把官方过滤迁到受控 Rust/流式路径。JSON-RPC 行上限和其他 command/config/storage payload 仍需各自容量合同，但不再决定本项生产 hook context 的输入大小。
- 2026-08-06 合并结果：首轮 Actions `31068400547` 仅报四个计划 Rust 文件的 rustfmt 漂移，云端 artifact 原样提交为 `a891a038` 后，本地允许的 plugin-hardening、SDK 30 tests、两包 TypeScript、plugin docs、Prettier、静态合同、JSON 与 diff 全部复验通过。第二轮 Actions `31069274373` 的 frontend、Rust、docs/support contract、change-scope、pr-title 与 `ci-gate` 全绿。合并前重新 fetch，`origin/main`、PR base 与 merge-base 均为 `e6cf04d3`，没有主线漂移或同功能实现且 PR CLEAN/MERGEABLE；Ready PR #83 squash 合并为 `4ee5faa8`，合并后七个目标文件树与候选一致。

### AUD-028：Codex OAuth 尝试可混用来访账户 ID 与已选 Provider token

- 状态：`resolved`
- 优先级：`P2`
- 2026-08-04 复核：提交 `e5d758b6` 已在每个 attempt 无条件移除来访 `chatgpt-account-id`，仅注入当前 Provider 派生 ID；现行实现与回归测试见 `src-tauri/src/gateway/proxy/handler/failover_loop/prepare/codex_chatgpt.rs:62-85, 311-344`。
- 判断依据：会造成认证失败、错误的账户/配额归因和 failover 熔断污染；是否能导致跨账户计费仍需真实上游验证，因此不提升为 P1。
- 文件和行号：Provider account 解析 `src-tauri/src/gateway/proxy/handler/failover_loop/prepare/provider_iterator.rs:196-200`；来访头保留 `src-tauri/src/gateway/proxy/request_context.rs:243-251`；attempt header `src-tauri/src/gateway/proxy/handler/failover_loop/attempt/attempt_executor.rs:203`；注入逻辑 `src-tauri/src/gateway/proxy/handler/failover_loop/attempt/attempt_auth.rs:34-61`、`src-tauri/src/gateway/proxy/handler/failover_loop/prepare/codex_chatgpt.rs:76-84`。
- 证据与触发路径：每个 attempt 从已选 OAuth Provider 提取 account ID 并注入 Bearer token，但 `chatgpt-account-id` 仅在来访 headers 不含该字段时才写入；request context 保留了来访字段。因此恶意/遗留客户端 header 可与本次 Provider B token 配对，尤其在 Provider A→B failover 中更明显。
- 实际影响与根因：上游可能拒绝 token/account 不匹配，错误还会进入 provider health/circuit；如果上游按 header 归属，影响范围需隔离账号验证。根因是当前 attempt 的身份没有成为该身份 header 的唯一权威。
- 最小修复建议：每个 ChatGPT backend attempt 无条件移除来访 `chatgpt-account-id`，只以 selected Provider 的 account ID 覆盖；无法派生时不转发该身份头并返回可诊断错误。
- 验证及回归测试：预置伪造 account header 并强制 Provider A→B failover，断言每个 outbound 请求的 token/account 对来自同一 Provider；无法派生 account ID 和正常 OAuth 路径分别覆盖。上游跨账户影响保留为 `HYP` 验证。

### AUD-029：TUI 的 3.5 秒 HTTP 总超时小于服务端允许的 Provider 探测时限

- 状态：`resolved`
- 优先级：`P2`
- 判断依据：用户显式运行的“测试当前供应商”在 4–15 秒成功时必报 Unreachable，服务端却仍继续占用探测许可；这是确定的假阴性，而不是一般网络慢。
- 文件和行号：基线 TUI client `src-tauri/crates/aio-tui/src/client.rs:49-55, 87-115`；observer deadline `src-tauri/src/app/observer/mod.rs:26-33, 370-401`；上游探测期限 `src-tauri/src/domain/provider_availability.rs:15-16, 643-654`。
- 证据与触发路径：所有 TUI HTTP 调用共用 3.5 秒 client timeout，provider test 直接复用它；observer 给 handler 20 秒，实际 provider connect/total 预算为 8/15 秒。上游在 4 秒完成时，TUI 已断连并显示失败，后台请求仍可到 15 秒才结束。
- 实际影响与根因：用户会错误判断 Provider 不可用并重复触发探测，浪费外部请求/费用和 observer 并发许可。根因是状态读取和具有真实网络时限的 probe 共用单一短 client timeout，且断连没有取消服务端工作。
- 2026-08-05 当前主线复核：`origin/main@5b13683b` 仍由 `ObserverClient::new` 建立 3.5 秒全局 timeout，`test_provider_availability` 没有请求级覆盖；observer route 仍以 20 秒包裹探测，domain 仍使用 8 秒 connect/15 秒总预算。该操作在 TUI spawn task 中运行，不阻塞输入线程，但会提前返回错误；#59/#60/#61 均不触及本项三个目标文件或相邻协议合同，没有重复、覆盖或冲突实现。
- planned 最小修复：在 `aio-observer-protocol` 定义 20 秒 Provider probe deadline 作为跨层事实源；observer 从该常量构造服务端 timeout；TUI 仅在手动探测 request 上覆盖为 deadline + 1 秒传输余量，保留快照 client 的 3.5 秒总超时；新增明确 `Timeout` 离线原因并让 HTTP 504/reqwest timeout 映射到它。修改范围限于 protocol、observer route 与 TUI client。
- planned 验证：先增加期限关系、请求级覆盖和 504 映射的 Rust 单元回归，再实施最小代码；本地只做源合同、格式静态检查和 diff，不运行 Cargo/Rust 工具链，编译、rustfmt/Clippy 与 Rust tests 交由 GitHub Actions。提交 PR 前重新 fetch 最新 `origin/main`、核对同功能实现并重跑允许的定向门。
- 遗留风险：本批不实现客户端断开后的服务端 cancellation；异常退出可能让既有 probe 最多继续占用到统一的 20 秒 deadline。该风险有界且与本次假阴性修复可独立，后续若有真实资源压力证据再单独处理。
- 2026-08-05 实施与提交前门：先加入协议 deadline、Observer 关系、TUI request override 与 504 回归，并用源检查确认旧生产代码缺少四个合同符号；随后在 protocol 导出 20 秒常量，Observer 从其派生 timeout，TUI probe 使用 21 秒请求级覆盖并新增 `Timeout`。独立差异审查发现响应体阶段仍会把 reqwest timeout 当作无效响应，已统一 send/body timeout 分类并增加本地延迟 HTTP body 回归。按规则未本地运行 Rust 工具；7/7 源合同与 diff 通过。候选在 #59 合并后重放为 `2f15e136`，云端 rustfmt 漂移补丁后的 `0836f772` 已通过 Actions `30990899964`；#61 合并后再次核对主线，新增内容仅为 AppLayout 和五个 SDK/文档/合同文件，与本项三文件及功能目标无重叠，候选无冲突重放为 `13abfea7` 到 `origin/main@ba06dabb`。重放后 7/7 源合同、三文件边界与 diff 复验通过，ready PR #62 新一轮 Actions 运行中；当前无重复实现、接口冲突或待决策事项。
- 2026-08-05 合并结果：Actions `30995513871` 的 frontend、rust、合同与 `ci-gate` 全部通过；合并前再次 fetch，`origin/main`、PR base 与 merge-base 均为 `ba06dabb`，没有新增主线漂移。ready PR #62 squash 合并为 `c2e4db25`。异常断连后的服务端 cancellation 仍可能占用到既有 20 秒上限，属于本项记录的有界遗留风险。

### AUD-030：Observer 快照缓存不驱逐过期键，长期运行会保留大量大对象

- 状态：`resolved`
- 优先级：`P2`
- 判断依据：合法 query key 组合最多约 510 个，单个包含 provider 时可带 512 Provider 及每项 12 buckets；不是无限 key，但无容量和过期删除会让桌面长驻进程保留数百份不再可用的大快照。
- 文件和行号：key/state `src-tauri/src/app/observer/mod.rs:50-86`；写入 `src-tauri/src/app/observer/mod.rs:251-321`；只读不删的 TTL `src-tauri/src/app/observer/mod.rs:494-510`；snapshot 上限 `src-tauri/src/app/observer/snapshot.rs:24-29, 363-470`。
- 证据与触发路径：每个合法 `CliScope × history_limit(0..50) × include_providers` 请求无论结果都 insert；`cached_snapshot` 对过期项仅返回 None，没有 remove 或定期 sweep。一个持有 token 的本地消费者逐个请求合法参数即可让过期快照永久留在 `HashMap`。
- 实际影响与根因：内存随历史查询组合累积，并在 provider 多/请求频繁时增加 GC/clone 压力。根因是 TTL 被用于 freshness 判断而不是生命周期管理，缓存没有容量策略。
- 最小修复建议：固定容量的 LRU/weighted cache 加过期 sweep，或只缓存 TUI 实际会用的 key；写入前/后清除 expired entries。
- 验证及回归测试：遍历全部合法 key 和超时场景，断言 map 长度与加权字节不超过上限、expired entry 被移除，热门 TUI key 仍命中。
- 2026-08-05 最新主线复核：`origin/main@c2e4db25` 已包含 AUD-029 的共享 probe deadline，但没有改变 `CacheKey`、`CachedSnapshot`、400ms/1500ms TTL 或 `cached_snapshot` 的只读不删行为。协议仍允许 5 个 `CliScope`、`history_limit=0..50` 和 `include_providers` 两态，共 510 组合法 key；Provider status 查询仍限制为 512 项，单个大快照的边界仍存在。#63/#64 分别只改前端会话分页和 settings writer，与 `src-tauri/src/app/observer/mod.rs` 无功能或文件重叠，因此没有重复实现或冲突。
- planned 实施：在 Observer 模块内封装现有 HashMap 的 get/insert 生命周期；每次访问先按现有 active/idle TTL 删除过期条目，新 key 插入前维持 64 项硬上限并淘汰 `created_at` 最早项，替换已有 key 不额外淘汰。保留 key、TTL、snapshot DTO、API 路由、并发 limiter、数据库查询顺序和依赖，不增加后台清理任务。
- planned 验证：先加入同模块 Rust 单测覆盖 active/idle 过期实际删除、64 项容量淘汰、新 key 命中和已有 key 替换；本地按仓库规则只运行源合同核对与 `git diff --check`，不运行 Cargo/rustfmt/Clippy/Rust tests；PR Actions 负责 rustfmt、Clippy、Rust tests、support-contract 与 ci-gate。遗留风险是容量按条目而非字节加权，完全空闲时最多保留 64 个过期条目直到下一次访问，后续可在有内存测量后单独设计 weighted cache。
- 2026-08-05 #64 后主线门：#64 只改 7 个 settings 文件，与本项唯一产品文件 `src-tauri/src/app/observer/mod.rs` 无重叠；候选无冲突重放为 `296f38a2` 到 `origin/main@5c756edc`。重放后 6/6 源合同与 diff 通过，独立差异审查未发现阻塞问题；按仓库规则未在本地运行 Rust 工具链，等待 PR Actions 验证编译、rustfmt、Clippy 与 Rust tests。
- 2026-08-05 PR 状态：最终 fetch 确认 `origin/main` 与 merge-base 均为 `5c756edc`，唯一产品变更仍为 `src-tauri/src/app/observer/mod.rs`；ready PR #65 已由候选 `296f38a2` 创建，Actions 运行中。
- 2026-08-05 云端格式修正：首轮 Actions `31000946748` 的 frontend 成功，rust 在生成/格式步骤上传 725 字节 `cloud-native-fixes.patch` 后按预期阻止漂移。该 artifact 只含同一测试模块三处 rustfmt 排版，无语义或生成绑定变化；原样应用为提交 `2403b31e` 后，6/6 源合同、diff 与 `origin/main@5c756edc` merge-base 再次通过，已推送触发新一轮 Actions。
- 2026-08-05 云端 Clippy 修正：第二轮 Actions `31001414671` 的 frontend 与格式/生成绑定步骤成功，Clippy 只要求测试辅助函数将 `index % 2 == 0` 改为 Rust 1.90 的 `index.is_multiple_of(2)`；一行修正提交为 `5f249948`。修正后 6/6 源合同、diff、`origin/main@5c756edc` 与 merge-base 再次通过，未改变 CacheKey 生成语义或生产代码。
- 2026-08-05 合并结果：第三轮 Actions `31001992510` 的 frontend、rust、合同与 `ci-gate` 全部成功；合并前再次 fetch，`origin/main`、PR base 与 merge-base 均为 `5c756edc`，PR 头为 `5f249948` 且 `CLEAN/MERGEABLE`。ready PR #65 squash 合并为 `405a545f`，没有相关主线竞争实现或待决策冲突。条目上限非字节权重、空闲过期项等遗留风险保持记录。

### AUD-031：Observer 每次快照为 OAuth Provider 执行 N+1 gate 查询

- 状态：`resolved`
- 优先级：`P2`
- 判断依据：活跃状态的缓存 TTL 只有 400 ms、TUI 500 ms 轮询；路由中 OAuth Provider 数量增加时查询数线性上升并与网关共享 SQLite，能超过 observer 1.5 秒 deadline。
- 文件和行号：snapshot 查询链 `src-tauri/src/app/observer/snapshot.rs:172-177, 329-360`；无 LIMIT 的 route candidates `src-tauri/src/domain/providers/queries.rs:1099-1157`；单 Provider gate SQL `src-tauri/src/domain/provider_oauth_limits.rs:408-417`；TTL `src-tauri/src/app/observer/mod.rs:29, 494-510`；TUI polling `src-tauri/crates/aio-tui/src/main.rs:454-465`（基线）。
- 证据与触发路径：即使 `include_providers=false`，snapshot 仍调用 `preferred_provider` 并加载完整 enabled route；随后对每个 OAuth Provider 逐一调用 `gate_snapshot/read_snapshot`。N 个 OAuth Provider 产生基础查询加 N 条 SQL；活动时接近每秒两轮。
- 实际影响与根因：大型路由下 observer 延迟/超时、TUI 不稳定，并增加网关请求与 SQLite 的竞争。根因是批量快照接口内部循环调用单实体数据访问，而非预加载所需 gate 状态。
- 最小修复建议：用 `IN` 分批一次读取所有 OAuth gate snapshot（可复用 display snapshot 策略），内存中关联 Provider；`include_providers=false` 只计算真正需要的最小字段。
- 验证及回归测试：构造数百 OAuth Provider，统计 SQL 次数与 p95 snapshot 时间，断言按 batch 数而非 Provider 数增长；覆盖 active 500 ms polling 和 `include_providers=false`。
- 2026-08-05 最新主线复核：`origin/main@9a280136` 的 `load_provider_candidates` 仍先加载无 LIMIT 的 active route，再为每个 `auth_mode == "oauth"` Provider 复用同一 connection 调用一次 `gate_snapshot/read_snapshot`；该候选投影不受 `include_providers` 控制。现有 `list_display_snapshots` 已去重 IDs、最多接受 512 项并按 300 个 SQL 参数分块，用同一 `gate_for_snapshot` 生成 `limited`，而 `load_provider_observations` 已在相邻路径批量使用它。#69 只改 Image Gen 两文件，开放 PR #70 只改前端内存诊断两文件，没有重复、覆盖或接口冲突；根因仍成立且无需产品决策。
- planned 实施：`load_provider_candidates` 收集 OAuth IDs，按 `MAX_DISPLAY_PROVIDER_IDS` 分组调用 `list_display_snapshots`，只把返回项中 `limited=true` 的 Provider ID 合并进既有 spend-limit 集合；未返回的快照继续等价于 `OAuthLimitGate::Allow`。将批量 API 的 512 输入上限收窄为 `pub(crate)` 常量供调用点复用，API 内仍按现有 300 参数执行 `IN` 查询。保留所有 Provider 候选、顺序、名称截断、spend limit、错误传播和 preferred-provider 行为；不改协议、TTL、DB schema、网关 gate 或依赖。
- planned 验证：先用源码 failure-first 断言候选函数不再含逐项 `gate_snapshot` 且必须调用批量 API；旧实现应失败。把现有 provider projection 回归扩展为两个 OAuth Provider，覆盖有 exhausted snapshot 的 Provider 被限制、缺失 snapshot 的 OAuth Provider 仍可用，并保留 spend-limit/顺序断言；源合同另锁定调用点按 512 上限分组，避免极端 route 因批量 API 输入上限回归。按仓库规则本地只运行源合同与 `git diff --check`，不运行 Cargo/rustfmt/Clippy/Rust tests；Actions 负责 rustfmt、Clippy、Rust tests、support-contract 与 ci-gate。遗留风险是每 512 个 OAuth 候选仍会打开一次连接并按 300 参数拆成查询，route candidates 本身仍无 LIMIT；本批消除逐 Provider SQL，不改变既有路由规模合同。
- 2026-08-05 本地实施与 PR 状态：候选 `ccfb0f4d` 仅修改 `src-tauri/src/app/observer/snapshot.rs` 与 `src-tauri/src/domain/provider_oauth_limits.rs`。旧实现 failure-first 同时报出逐项 `gate_snapshot`、缺批量 API、缺外层上限三项；修复后候选合同 4/4、OAuth 源合同 3/3、`git diff --check` 与独立差异审查通过。候选 IDs 现在按共享 512 上限分组，批量 API 内保留 300 参数 SQL 分块；现有 projection 回归把 ready Provider 改为缺失 snapshot 的 OAuth，锁定 exhausted 被限、missing Allow、spend limit 和顺序。官方 route writer 当前也硬限制 512 项，因此未添加绕过生产写入合同的 513 项重型数据库测试；外层分组作为旧库/异常数据防御由源码合同锁定。最终 fetch 的 `origin/main` 与 merge-base 均为 `9a280136`，唯一开放 PR #70 只有无交集的前端诊断两文件；Ready PR #71 已创建并进入 Actions `31022477938`。按规则未本地运行 Rust 工具链；遗留风险保持为每 512 IDs 的连接/最多两条 SQL 与 route query 无 SQL LIMIT。
- 2026-08-05 CI 漂移修正计划（`planned`）：Actions `31022477938` 的唯一 Rust 失败为生成/格式漂移；上传 artifact `cloud-native-fixes-53b38221e0de929f68896754a035437439b90bd6-1` 经检查仅含 `snapshot.rs` 中 OAuth ID 分组循环的 rustfmt 换行，无生成绑定、逻辑或测试变化。先将候选以 `origin/main@5d4906c5` 重放，精确应用该 patch；随后重跑 4/4 候选合同、3/3 OAuth 源合同与 `git diff --check`，核对 #70 的新主线没有功能/文件重叠，再推送并由新 Actions 验证 rustfmt、Clippy、Rust tests、support-contract 与 ci-gate。保留原有两文件范围和遗留风险。
- 2026-08-05 CI 漂移修正结果：#70 仅改前端内存诊断两文件并合并为 `5d4906c5`，与本项无功能、接口或文件重叠；候选无冲突重放为逻辑提交 `df4dcdca`，再原样应用 artifact 为格式提交 `f92d5190`。重放后候选合同 4/4、OAuth 源合同 3/3、`git diff --check` 与 merge-base 检查通过，工作树干净；PR #71 已用 `--force-with-lease` 更新，base/head 为 `5d4906c5`/`f92d5190`，新 Actions `31023947314` 运行中。第一次 OAuth 合同脚本因把 `pub(crate) fn` 误匹配为 `pub fn` 而自检失败，修正检查器后 3/3 通过，不涉及产品代码。
- 2026-08-06 Actions 与合并结果：Actions `31023947314` 的 frontend、rust、support-contract、change-scope、pr-title 与 ci-gate 全部通过。最终 fetch 的 `origin/main` 仍为 `5d4906c5`，PR #71 base/head 精确为 `5d4906c5`/`f92d5190` 且 CLEAN/MERGEABLE；唯一并行 PR #72 仅改 `pipeline.rs`，与本项无功能、接口或文件重叠。Ready PR #71 squash 合并为 `7c395d15`；遗留风险仍是每 512 IDs 的连接/最多两条 SQL，以及 route candidates 无 SQL LIMIT。

### AUD-032：fail-open 插件的非法 header patch 仍会无条件中断请求

- 状态：`resolved`
- 优先级：`P2`
- 判断依据：公开合同称 fail-open 在无效输出时继续原路径；一个可选插件返回保留/非法 header 即可导致请求或响应失败，实际可用性与合同相反。
- 文件和行号：合同 `docs/plugins/reference/hooks.md:7-23`；策略分支 `src-tauri/src/gateway/plugins/pipeline.rs:318-411`；无条件 header 错误 `src-tauri/src/gateway/plugins/pipeline.rs:414-444, 624-683`；reserved header 校验 `src-tauri/src/gateway/plugins/pipeline.rs:1642-1676`；反向测试 `src-tauri/src/gateway/plugins/pipeline.rs:2725-2817`。
- 证据与触发路径：executor/mutation 错误会进入 failure policy，但 `apply_header_patch` 失败直接返回 `Err`。默认 fail-open 插件若返回 `x-aio-provider-id` 或无效 header，gateway 仍终止而不是忽略 patch/保持原请求。
- 实际影响与根因：一个插件 bug 或恶意配置可以中断正常流量，用户无法依合同通过 fail-open 把影响限制在插件功能。根因是 header patch 在统一策略处理之后以非事务方式应用。
- 最小修复建议：先在 header 副本上事务性应用 patch，失败时按统一 failurePolicy 处理；fail-open 保留原 headers/body，fail-closed 拒绝。
- 验证及回归测试：请求和响应各覆盖 reserved/非法 header，断言 fail-open 成功转发且内容不变、fail-closed 得到稳定拒绝及审计记录。
- 2026-08-06 最新主线复核：`origin/main@5d4906c5` 中 `apply_header_patch` 仍直接逐项写入共享 `HeaderMap`，遇到后续 reserved/非法 header 才报错，因此既无条件让 request/response 两个调用点 `return Err`，又可能在报错前留下按 `BTreeMap` 顺序已写入的合法 header。默认测试 helper 和公开 `docs/plugins/reference/hooks.md` 均把 failure policy 定义为 fail-open，但现有 request/response reserved-header 测试反而期待默认插件失败，锁定了错误行为。相关实现自 `7088dcf4` 后无后续修正；唯一开放 PR #71 只改 Observer/OAuth 两文件，没有重复、覆盖或接口冲突。
- planned 实施：只修改 `src-tauri/src/gateway/plugins/pipeline.rs`。`apply_header_patch` 先克隆当前 headers，在副本完成全部 reserved/name/value 校验和插入，成功后一次替换原 map；request/response 调用点在记录现有 failure/audit/report 后读取既有 `failure_policy`，fail-closed 保持带诊断返回错误，fail-open `continue` 到下一插件并丢弃该插件的 headers/body/block 整份 mutation。保留错误码、circuit 计数、audit、execution report、权限/预算门和成功 hook 顺序，不改公开合同、SDK、manifest 或依赖。
- planned 验证：先运行源合同 failure-first，证明当前两个 header 错误分支无 policy 判断且 helper 会部分写入；再把现有 request/response reserved-header 拒绝测试显式设为 fail-closed，并新增 request reserved 与 response 非法 header 的 fail-open 回归，均同时返回一个排序更早的合法 header 和 body mutation，断言输出保留进入该插件前的 headers/body、无部分 mutation、记录失败并继续。修复后重跑源合同与 `git diff --check`；按仓库规则不在本地运行 Cargo/rustfmt/Clippy/Rust tests，完整原生验证交由 GitHub Actions。遗留风险是 fail-open 失败仍计入 plugin 级 circuit，且 `AUD-026` 的 fail-closed 日志/circuit 共因不在本批扩大。
- 2026-08-06 本地实施与 PR 状态：候选 `6ebd09e5` 只修改 `src-tauri/src/gateway/plugins/pipeline.rs`。旧实现 failure-first 的事务 helper、request policy、response policy 三项均失败；修复后非空 patch 在 cloned `HeaderMap` 完整校验/插入后一次提交，两个调用点在保留 failure/audit/report 与 circuit 计数后按 policy 继续或返回。新增 request reserved 与 response invalid 的 fail-open 原子回归，同时带排序更早的合法 header 和 body mutation；现有三组 request/response reserved 测试显式设为 fail-closed。7/7 源合同、`git diff --check`、单文件范围与主线程差异审查通过。最终 fetch 的 origin/main 与 merge-base 均为 `5d4906c5`，唯一开放 #71 只改 Observer/OAuth；Ready PR #72 的 base/head 为 `5d4906c5`/`6ebd09e5`，Actions `31025383330` 运行中。按规则未本地运行 Rust 工具链；plugin 级 circuit 与 `AUD-026` 保留。
- 2026-08-06 CI 漂移修正计划（`planned`）：Actions `31025383330` 的 frontend 与合同检查通过，Rust 只因云端格式/生成文件漂移失败。artifact `cloud-native-fixes-06797bfbd1ccb9d129663ba7cd0a2ea9b49bad7b-1` 经 `git apply --check` 和逐行审查，只有 `pipeline.rs` 两个新增测试的 rustfmt 换行（9 行增、13 行减），没有逻辑、绑定或依赖变化。#71 已以 `7c395d15` 合并且只改 Observer/OAuth；先把 #72 无冲突重放到该最新主线，再精确应用 artifact，重跑 7/7 源合同、`git diff --check`、范围与 merge-base 检查，随后推送新 head 并由 Actions 重验 rustfmt、Clippy、Rust tests、support-contract 与 ci-gate。原有单文件边界和遗留风险不变。
- 2026-08-06 CI 漂移修正结果：#71 只改 Observer/OAuth 并合并为 `7c395d15`，与本项无功能、接口或文件重叠。逻辑提交无冲突重放为 `2a1878ba`，artifact 原样提交为 `0a5bd769`；7/7 源合同、`git diff --check`、单文件范围、完整差异复核和 merge-base 检查通过，工作树干净。PR #72 已用 `--force-with-lease` 更新，base/head 精确为 `7c395d15`/`0a5bd769` 且 MERGEABLE，新 Actions `31026666018` 运行中。
- 2026-08-06 Actions 与合并结果：Actions `31026666018` 的 frontend、rustfmt、Clippy、Rust tests、依赖审计、support-contract 与 ci-gate 全部通过。最终 fetch 的 `origin/main` 仍为 `7c395d15`，PR #72 base/head 精确为 `7c395d15`/`0a5bd769` 且 CLEAN/MERGEABLE；新开放 #73 只涉及观测、日志、用量与协议文件，没有 `pipeline.rs` 或插件 failure-policy 实现。Ready PR #72 squash 合并为 `d26524f2`；plugin 级 circuit 与 `AUD-026` 的遗留风险保持记录。

### AUD-033：插件激活与隔离生命周期只存在于合同，运行时未实现对应状态转换

- 状态：`planned`
- 优先级：`P2`
- 判断依据：声明 `onStartup` 的插件不会启动，未声明 hook/command event 的插件却仍执行；重复崩溃/timeout 仅短暂内存熔断而不持久隔离，直接偏离对外插件安全与可用性合同。
- 文件和行号：activation 验证 `src-tauri/src/domain/plugins.rs:760-784`、`packages/plugin-sdk/src/index.ts:428-445`；无条件 activation `src-tauri/src/app/plugins/extension_host.rs:205-264, 639-774`；内存 circuit `src-tauri/src/gateway/plugins/pipeline.rs:1216-1228`；审计/状态 `src-tauri/src/gateway/plugins/audit.rs:20-102`、`src-tauri/src/app/plugin_service.rs:2008`；公开合同 `docs/plugins/plugin-api-v1-contract.json:21-26`、`docs/plugins/architecture/security.md:15`。
- 证据与触发路径：`activationEvents` 只作格式验证，没有 onStartup dispatcher，也不在 command/hook 执行前检查声明；`Quarantined` 只在市场撤销路径写入，运行时失败只更新内存 circuit，冷却后会再次执行。
- 实际影响与根因：插件作者无法依声明预测何时运行，反复失败的 fail-closed 插件会周期性阻断用户请求，并在重启后重新发生。根因是 manifest 生命周期被当作文档 metadata，而状态机没有在 host/pipeline 持久化实现。
- 最小修复建议：实现 event dispatcher 与贡献-事件校验，或删除/拒绝未实现事件；持久化失败窗口/阈值，达到阈值原子切换 Quarantined 并提供显式恢复动作。
- 验证及回归测试：覆盖 onStartup exactly-once、未声明 event 不执行、连续 timeout/crash 跨重启仍 Quarantined、手动恢复后成功执行。
- 2026-08-06 最新主线复核：`origin/main@4ee5faa8` 的 `activationEvents` 仍只有 manifest 校验，没有 dispatcher；运行时 circuit 只在内存中短暂 cooldown，重启后清零，`Quarantined` 仅用于显式市场撤销且没有 revalidate/recover 转换。公开文档仍承诺 repeated failure 可隔离并恢复。AUD033 的阈值、窗口、事件派发和恢复语义必须先定产品合同，本批不实施，继续 `confirmed`。
- 2026-08-06 最终治理计划：任务 `.trellis/tasks/08-06-plugin-activation-quarantine`。显式 activation event 只接受精确 `onStartup`、`onCommand:*`、`onGatewayHook:*`；空/缺失数组保持 legacy，废弃 ProviderEditor/ProtocolBridge 事件拒绝并把历史安装迁移为有原因的 disabled。host crash、runtime error 或 timeout 在任意 startup/command/hook 的 600 秒窗口累计三次时，以单一事务持久 quarantine；第三次请求仍保持既有 fail-open/fail-closed，随后刷新 gateway snapshot 并释放 host。revalidate 成功只恢复 disabled，不自动启用。

### AUD-034：插件 storage 与 UI 配置保存对同一 JSON 的并发写入会丢失更新

- 状态：`resolved`
- 优先级：`P2`
- 判断依据：插件 runtime 与用户界面是独立并发写者，任何后完成的旧快照都会覆盖对方字段；配置可包含路由/隐私行为，属于真实数据完整性问题。
- 文件和行号：host storage `src-tauri/src/app/plugins/extension_host.rs:393-433`；repository upsert `src-tauri/src/infra/plugins/repository.rs:463-530`；UI command/service `src-tauri/src/commands/plugins.rs:675-687`、`src-tauri/src/app/plugin_service.rs:2226-2245`；表单 identity 重置 `src/pages/plugins/PluginConfigSchemaForm.tsx:76-83, 148-300`；页面 identity `src/pages/PluginsPage.tsx:433-437`。
- 证据与触发路径：`api.storage.set` 读取 `detail.config`，修改 storage 子字段后全量保存；UI config 保存也把整份 JSON replace。两者并发时都基于相同旧 document，后提交者将前者的独立配置或 storage 变化覆盖。相同的“服务端确认回写全量配置”根因还会在 UI 中丢失提交后的编辑：保存尚未完成时字段仍可修改；保存令 `updated_at` 变化后，identity 改变会把草稿重置为服务端快照。
- 实际影响与根因：插件内部状态和用户设置可无错误提示地回退，造成行为与 UI 不一致。根因是不同逻辑域复用单一 JSON blob 却没有独立存储、原子 JSON update 或版本 CAS。
- 最小修复建议：storage 使用独立 KV 表，或对 config 用 SQLite 原子 JSON patch/版本 CAS 加 retry/合并；不要让 runtime storage 与用户 schema 表单共享同一全量写入单元。
- 验证及回归测试：确定性交错 `storage.set` 与 UI 保存，分别测试两种完成顺序，断言两个不相交字段均保留；保存请求 pending 时再编辑另一字段，确认表单被锁定或后续草稿不被响应重置；覆盖冲突时明确报版本错误而非 last-write-wins。
- 2026-08-06 最新主线复核：审计报告 worktree 的 tracked source 停在历史 `86a30710`，已显式弃用该源码视图并改在基于 `origin/main@e6cf04d3` 的 worktree 点验。当前 `storage_set` 仍先 `get_plugin` 再全量 `save_plugin_config`；用户保存同样先读 detail/defaults 再全量覆盖；表单只禁用 submit，所有输入在 pending 期间仍可改变，且 identity 仍含 `updated_at`。#83 严格七文件，与本批五文件零重叠。
- planned 实施：repository 新增 runtime storage 与 config 两个 `TransactionBehavior::Immediate` 专用入口及可复用 transaction helper；前者在事务内更新单 key 并执行既有 64 KiB 限额，后者在事务内把最新保留 `storage` 合并回非 runtime config。Extension Host、UI save、官方默认配置、local package update 与 rollback 改用对应入口/helper，版本/权限/状态流程不变；表单用 disabled fieldset 冻结 pending 编辑。严格只改 repository、Extension Host、plugin service、配置表单及其测试，不改 IPC/schema/DTO、公开 storage 形态、依赖或生命周期状态机。
- planned 验证：failure-first Rust 回归覆盖 runtime→UI、UI→runtime 与 storage 超限零提交；前端回归覆盖 pending 时输入/select/checkbox/Switch/保存均不可操作。运行定向前端测试、TypeScript、目标 ESLint/Prettier、静态事务/范围合同和 diff；Rust format/Clippy/tests/audit 交 Actions。遗留风险是绕过 UI 并发提交两份用户 config 仍 last-write-wins，未来多窗口/多设备编辑需单独 revision/CAS 公共合同。
- 2026-08-06 候选实施：任务 `.trellis/tasks/08-06-plugin-config-storage-atomicity`，候选 `fed31c67` / Ready PR #84 严格五文件。repository 用 `TransactionBehavior::Immediate` 在读取前取得写者所有权：runtime 只更新单个 storage key 并先执行 64 KiB 检查，config helper 把事务内最新保留 `storage` 合并回 UI、官方默认配置、local package update 与 rollback 的输入；Extension Host 仅委托。表单用 disabled fieldset 冻结 pending 期间所有控件。独立审查发现并补齐官方/update/rollback 同域写者；既有文档已将顶层 `storage` 定义为保留字段，本批不新增 schema validator 或独立表。
- 2026-08-06 本地验证与 PR 前门：failure-first 前端用例在旧实现按预期失败，实施后目标 11 tests、TypeScript、目标 ESLint/Prettier、Vite build、静态 transaction/call-site 合同、精确五文件范围与 diff 全过。按规则未本地运行 Rust/native。#83 Actions `31069274373` 全绿并合并为 `4ee5faa8` 后，候选无冲突重放该最新主线并复跑全部允许验证；PR 前最后 fetch 确认 `origin/main`/merge-base 均为 `4ee5faa8`，没有其他开放 PR、重复实现或冲突。
- 2026-08-06 Actions 与合并结果：首轮只产生云端 rustfmt artifact，精确提交为 `05d317fe`；第二轮 Clippy 发现仅测试使用的 legacy helper，补 `#[cfg(test)]` 为最终候选 `c669b522`。Actions `31073434744` 的 frontend、Rust format/bindings、Clippy、Rust tests、依赖审计及全部合同门全绿。最终 fetch 确认 `origin/main`、base 与 merge-base 均为 `4ee5faa8`，只有 #84 开放且 CLEAN/MERGEABLE；PR squash 合并为 `4800bc87`，合并后五个目标文件与候选逐字节一致，远端无开放 PR。直接多 IPC 用户配置仍是 last-write-wins，保留为 revision/CAS 后续风险。

### AUD-035：`history_limit=0` 的 TUI 状态刷新仍投影完整历史并全量扫描会话文件树

- 状态：`planned`
- 优先级：`P2`
- 判断依据：状态栏与 `status --once` 明确请求零条历史，却固定触发 500 条日志和会话目录解析；活跃请求期间 500 ms 轮询会把一次不必要的重 I/O 放大为持续负载，可能超过 observer 的 1.5 秒预算。
- 文件和行号：基线 TUI 请求与轮询 `src-tauri/crates/aio-tui/src/main.rs:84-94, 97-152, 454-465`；快照投影 `src-tauri/src/app/observer/snapshot.rs:24, 158-167, 264-285, 779-819`；Codex 文件发现 `src-tauri/src/domain/cli_sessions/codex.rs:48-97, 675-687`。
- 证据与触发路径：TUI 每次调用 `snapshot(scope, 0)`；活跃时刷新间隔为 500 ms。observer 的 projection 没有用 `history_limit=0` 剪掉历史读取，仍以 `HISTORY_SCAN_LIMIT=500` 读取请求日志，并为 active/历史 session 解析 folder。Codex folder lookup 递归遍历 `$CODEX_HOME/sessions` 下所有 `.jsonl`，没有索引或缓存。
- 实际影响与根因：大型会话目录或慢盘会使本不需要历史的状态栏持续占用磁盘、CPU 和 SQLite，observer timeout 后又会使 TUI 显示离线。根因是“快照完整性”与“消费者请求的投影范围”没有分层，先做全量昂贵工作再丢弃结果。
- 最小修复建议：按 `history_limit` 在查询前裁剪日志和会话解析；只为最终可见条目查 folder；给 session-folder lookup 加带失效策略的索引/缓存。
- 验证及回归测试：构造大量 Codex session 文件，在 `history_limit=0` 下断言没有全树扫描、没有 500 条历史读取；活跃 500 ms 轮询的 p95 快照时延低于 observer deadline。
- 2026-08-05 当前主线复核：`origin/main@62574e22` 中 `history_limit=0` 只约束 `recent_requests` 数量，不能等同于“不需要日志”。同一批最多 500 条日志仍用于 `last_request`、`dominant_provider`、活动请求终态去重，以及 `CliScope::All` 的首选 CLI/Provider 判断；TUI 的默认和可选状态项会直接展示这些字段。直接在零历史请求下跳过日志读取会改变现有协议与状态栏行为，因此旧计划中的“没有 500 条历史读取”不是可保持行为的最小修复验收条件。
- 当前决定：性能问题仍成立，但本轮不实施。后续应先把“recent 列表上限”与“摘要统计/终态去重所需窗口”拆成明确合同，再选择专用聚合查询、只为实际投影的 active/last/recent 条目解析 folder，或建立可失效 session-folder 索引；需用大 session 树基准验证收益和一致性。本项保持 `confirmed`，不按旧报告建议直接裁剪。
- 2026-08-06 最新主线复核：`origin/main@4ee5faa8` 仍无条件读取最多 500 条日志，并用其派生 last request、dominant provider、active 去重、`scope=all` 首选 provider 与 folder lookup；TUI 仍以 `snapshot(scope, 0)` 请求。直接把零 history 变成零扫描会改变状态栏/协议语义，且没有现成摘要查询或扫描量回归。AUD035 继续 `confirmed`，先定义摘要窗口与历史列表分离合同，再做专用聚合/索引和大树基准。
- 2026-08-06 最终治理计划：任务 `.trellis/tasks/08-06-observer-zero-history-query`。保留 last 为当前 scope 最新 terminal inference、dominant 为最近最多十条且平手优先较新、active/all-scope/Claude 可见性以及 recent ready-empty 语义；分别使用受限查询，`history_limit=0` 跳过 500-row recent 投影。folder lookup 只接收实际渲染的 active/last/recent 键，并在 Observer state 使用 `(source, session_id)` 隔离、容量与正/负 TTL 均受限的缓存。

### AUD-036：插件详情占位数据可被提交到新选中的插件

- 状态：`resolved`
- 优先级：`P1`
- 2026-08-04 复核：`usePluginQuery` 仍使用 `keepPreviousData`（`src/query/plugins.ts:90-101`），页面仍把旧 `detailQuery.data` 与新 `effectiveSelectedPluginId` 组合为保存/回滚操作（`src/pages/PluginsPage.tsx:501-508, 684-701`）。计划任务：`.trellis/tasks/08-04-plugin-detail-identity-guard`。
- 2026-08-04 实施：提交 `05f62ad2` 已推送至 `codex/audit-plugin-detail-identity`，draft PR #42。页面只接受 `detail.summary.plugin_id === effectiveSelectedPluginId` 的详情；不匹配的 placeholder 在查询期间进入加载态，保存、更新和回滚入口不可达。保存与回滚 mutation 的目标进一步改为已通过身份校验的详情 ID。
- 修改文件：`src/pages/PluginsPage.tsx`、`src/pages/__tests__/PluginsPage.test.tsx`。
- 测试结果：新增 A -> 慢 B 的确定性竞态测试，未修复代码确实显示 A 详情并失败；修复后验证等待期间 A 配置/回滚操作不可见且 mutation 为零，B 返回后配置和回滚目标均为 B。`PluginsPage.test.tsx` 34/34、根 TypeScript、`eslint src/ --no-cache`、目标 Prettier、Vite production build 与 `git diff --check` 全部通过；Vite 仅有既有 Browserslist 和 chunk-size 提示。PR #42 GitHub Actions 运行中。
- 遗留风险：本项不改变全局 `keepPreviousData`，其他消费者仍需各自绑定实体身份；不解决同一插件内 runtime storage 与 UI config 的并发覆盖，`AUD-034` 保留。查询失败后的详情区沿用现有空态/刷新指示，没有新增错误恢复 UI。
- 判断依据：一个普通的慢查询时序即可让 A 的配置或版本以 B 的 `pluginId` 保存/回滚，属于直接跨实体写入，而无需权限绕过或罕见故障。
- 文件和行号：查询 placeholder `src/query/plugins.ts:90-101`；当前选中 ID 与操作目标 `src/pages/PluginsPage.tsx:501-508, 672-701`。
- 证据与触发路径：详情 query 使用 `keepPreviousData`。用户从插件 A 切到慢加载的 B 时，标题和操作 target 已改为 B，但 `detailQuery.data` 仍可为 A；页面把这份数据交给配置表单和 rollback 操作，保存/回滚 mutation 使用 `effectiveSelectedPluginId`。
- 实际影响与根因：A 的配置可覆盖 B，A 的历史版本号可请求回滚 B；操作结果看似成功但破坏另一插件的设置/版本。根因是实体详情的 placeholder 没有与当前选中实体做身份绑定。
- 最小修复建议：仅在 `detail.summary.plugin_id === effectiveSelectedPluginId` 时渲染详情并启用保存、回滚；其他情况显示 loading，或对详情 query 禁用 `keepPreviousData`。
- 验证及回归测试：使 B 的详情查询延迟，在返回前断言 A 字段不显示、保存/回滚按钮禁用且不会以 B 为 target；B 返回后只允许 B 的配置与版本操作。

### AUD-037：模型价格别名读取错误被伪装为空配置，随后的保存会抹掉原规则

- 状态：`resolved`
- 优先级：`P1`
- 判断依据：别名文件损坏、不可读或超限后，编辑接口明确返回默认空对象且 UI 可正常保存；成功写入后会删除备份，因此是可触发的持久配置丢失。
- 文件和行号：command `src-tauri/src/commands/model_prices.rs:77-90`；fail-open read `src-tauri/src/infra/model_price_aliases.rs:208-225`；编辑/保存 UI `src/components/settings/ModelPriceAliasesDialog.tsx:462-520, 550-610`。
- 证据与触发路径：`model_price_aliases_get` 把 `read_fail_open` 包装为成功 `Ok`；任意 `read` 失败都会返回 `ModelPriceAliasesV1::default()`。前端无法区分合法空配置和读取失败，仍允许保存；保存流程完成后移除旧备份。
- 实际影响与根因：已有别名规则会被空文档覆盖，模型价格匹配/计费归因永久变化，用户没有错误提示或恢复入口。根因是“显示用宽容读取”被错误用于编辑和写回路径。
- 最小修复建议：编辑 command 改用严格 `read` 并传播错误；只把缺失文件视为合法空状态；加载失败时 UI 显示恢复建议并禁用编辑/保存。每次原子替换保留可恢复版本。
- 验证及回归测试：分别注入损坏 JSON、权限拒绝、超限和不存在文件；前三种必须显示错误且不发 save，最后一种可创建默认配置；保存后可恢复上一版本。
- 2026-08-05 当前主线复核：`origin/main@eeccf64d` 已有严格 `model_price_aliases::read`，且缺失文件会合法返回默认值；但编辑 command 仍调用 `read_fail_open`，`ModelPriceAliasesDialog` 仍用空数据构造默认 draft 并保持 mutation 可达。#51/#52 未改变该读写边界，数据覆盖路径仍成立。
- 2026-08-05 实施前追加核验：`origin/main@891c9eb3` 的 Rust 默认值、v1 迁移和写回响应均已使用 schema v2（来源提交 `2deb7e82`），但前端 `normalizeModelPriceAliases` 仍只接受并输出 v1。仅切换严格读取会让“文件不存在”的合法默认响应和正常保存响应都被前端拒绝，因此 schema 对齐是本项验收的直接前置，不是无关扩项。
- 计划：子任务 `.trellis/tasks/08-05-model-price-alias-read-safety` 将编辑 command 切换到严格 `read`，保留运行时成本计算的 `read_fail_open`；前端 adapter 兼容 v1/v2 输入并统一输出当前 v2；UI 将 query error 显示为局部错误与重试状态，并阻断新增、编辑、删除和保存。验证不存在、损坏、不可读、超限、v1 迁移、v2 默认/保存、零 save、重试恢复和正常编辑；本地运行定向 Vitest、TypeScript、ESLint、Prettier、隔离 Vite build，native 定向测试与 Rust 质量门只由 Actions 执行。本批不改变 DTO、生成绑定或既有备份策略。
- 2026-08-05 实施状态：`codex/audit-model-price-alias-read` 最终 head `acd55808` 在 `origin/main@ed72549b` 完成主线门，PR #56 的 frontend、rust、support-contract 与 ci-gate 全部通过，已 squash 合并为 `db92a480`。旧实现失败优先得到 3 个目标失败；修复后 query/service/dialog 18/18、全前端 312 files / 2789 tests、TypeScript、ESLint、Prettier、Vite build 和 diff 检查通过。独立审查确认编辑 command 严格传播错误，request log 与成本回填两处仍 fail-open。修改文件为严格读取 command/native fixture、model price adapter/query/dialog 及其定向测试。遗留风险是既有成功写入后删除 `.bak` 的恢复策略未改变；本地按规则未运行 Rust，native 结果由 #56 Actions 验证。

### AUD-038：十页消息窗口淘汰较早页面后没有反向取回入口

- 状态：`resolved`
- 优先级：`P1`
- 判断依据：大于 500 条消息的合法会话会确定性丢失已访问的最早页，页面又没有重新获取方向；用户看到“会话开始”却无法回到真实开头。
- 文件和行号：分页配置 `src/query/cliSessions.ts:26-27, 83-107`；页面加载与布局 `src/pages/SessionsMessagesPage.tsx:574-584, 610, 686-702`。
- 证据与触发路径：query 使用 `pageSize: 50` 和 `maxPages: 10`，只有 `getNextPageParam`，没有 `getPreviousPageParam`。会话从末端反向加载十页后再 load-more，TanStack Query 淘汰最早缓存页；UI 只提供继续加载方向，同时仍把可见区域当作会话开头。
- 实际影响与根因：排障、审计和历史浏览无法读取长会话的早期内容，且没有刷新/深链替代路径。根因是缓存窗口策略与单向浏览语义不兼容。
- 最小修复建议：提供 previous-page 参数和“加载更早”入口，或取消窗口淘汰/按锚点重取；明确 UI 的当前数据边界。
- 验证及回归测试：构造 11 页以上会话，连续加载后确认第 1 页仍可访问或可从“加载更早”重新获取；断言页面不会把窗口起点标示为会话起点。
- 2026-08-05 当前主线复核：`origin/main@62574e22` 的页面已固定使用 `fromEnd:false`，从 page 0 顺序加载，因此旧报告中的“从末端反向加载”描述不再准确；但 query 仍配置 `maxPages: 10` 且只有 `getNextPageParam`。加载第 11 页后 TanStack Query 会丢弃 page 0，`hasPreviousPage` 永远为 false；组件虽按首个缓存 page 重新计算全局序号，却仍固定显示“会话开始”，也没有取回较早窗口的操作。#60/#61/#62 分别只改布局、Plugin SDK 和 Observer/TUI Rust 文件，与本项无重复实现或文件冲突。
- planned 实施：保留现有 10 页内存上限、50 条 page size、query key、后端 IPC 和顺序加载语义；为无限查询增加基于首个缓存 page 的 `getPreviousPageParam`，在页面接入 `hasPreviousPage` / `fetchPreviousPage` / loading 状态，提供明确的“加载更早”操作；仅在 page 0 仍在窗口时显示“会话开始”，否则标示已加载窗口边界。向后加载继续使用现有“加载更晚”路径，任一方向取页都允许 TanStack 按既有上限淘汰另一端。
- planned 验证：先用 11 页以上的 hook 回归证明旧实现淘汰 page 0 后无法 `fetchPreviousPage`，再断言修复后可取回 page 0 且缓存仍不超过 10 页；页面回归覆盖更早/更晚按钮、loading/disabled 状态、真实会话开始与窗口边界文案、全局消息序号。随后运行定向 Vitest、TypeScript、目标 ESLint/Prettier、隔离 Vite build、`git diff --check` 和桌面/窄视口浏览器验证。遗留风险是窗口仍有意只保留 10 页，反向取回较早页时最晚端会被淘汰，但两端都可再次获取。
- 2026-08-05 实施与提交前门：修改仅限 `src/query/cliSessions.ts`、对应 query 测试、`src/pages/SessionsMessagesPage.tsx` 和对应页面测试。失败优先证明第 11 页后 page 0 淘汰且 `hasPreviousPage=false`；修复后 query 从 `[1..10]` 取回 page 0 得到 `[0..9]`，页面分别管理更早/更晚 loading 与禁用状态，并依据首末缓存页显示真实会话或窗口边界。候选 `0714e83b` 无冲突重放为 `93a350bd` 到 `origin/main@ba06dabb`；2 files/28 tests、TypeScript、目标 ESLint、Prettier、隔离 Vite build 与 diff 通过。1024px 真实浏览器在 `1-500/550` 与 `51-550/550` 间双向切换正确，边界标记和禁用状态同步，文档宽度等于视口宽度，三个 38px 操作控件不重叠。ready PR #63 已创建，Actions 运行中。遗留风险：窗口仍有意只保留 10 页；390px 下现有固定侧栏会压缩全局主内容，本项未扩大范围修改既有 App shell。
- 2026-08-05 #62 后最终主线门：#62 只改 `aio-observer-protocol`、TUI client 与 Observer route，与本项四个前端文件、功能目标和接口行为没有重叠；候选无冲突重放为 `f7d6fc17` 到 `origin/main@c2e4db25`。重放后 2 个文件/28 tests、TypeScript、目标 ESLint/Prettier、Vite build 与 diff 再次通过，ready PR #63 的新一轮 Actions 运行中。
- 2026-08-05 合并结果：Actions `30997519757` 的 frontend、rust、合同与 `ci-gate` 全部通过；合并前再次 fetch，`origin/main`、PR base 与 merge-base 均为 `c2e4db25`，没有新增主线漂移。ready PR #63 squash 合并为 `e57acb54`。窗口仍有意只保留十页，390px 固定侧栏压缩属于既有 App shell 遗留风险。

### AUD-039：`FormField` 的直接子节点没有得到 `id`，可见标签与控件断开关联

- 状态：`resolved`
- 优先级：`P2`
- 判断依据：该 primitive 被至少二十处生产表单直接使用，影响键盘/读屏用户的控件名称、点击聚焦和提示关联；属于真实可访问性功能缺失而非视觉风格问题。
- 文件和行号：primitive `src/ui/FormField.tsx:17-33`；示例调用 `src/pages/PromptsView.tsx:345-358`、`src/pages/WorkspacesPage.tsx:667-668`、`src/components/settings/ApiKeySection.tsx:75-76`。
- 证据与触发路径：`FormField` 总为 label 生成 `htmlFor`，但仅在 children 是 render function 时传入生成的 id。普通 `ReactNode` child（`Input`/`Textarea`）没有 id 注入，因此 label 的 `htmlFor` 指向不存在元素，hint 也无法可靠关联。
- 实际影响与根因：点击可见标签不能聚焦控件，读屏器无法以标签名识别控件，表单错误/提示上下文丢失。根因是组件 API 同时允许两种 children 形态，却只实现其中一种的无障碍合同。
- 最小修复建议：强制 render-prop API；或限制为一个可克隆的表单控件并注入 `id` 和 `aria-describedby`，对不支持的 child 明确失败。
- 验证及回归测试：以可见 label 查询所有代表性控件，点击 label 后必须聚焦对应 input；用自动化无障碍测试断言 label 与 hint/error 关联完整。
- 2026-08-06 最新主线复核：`origin/main@94da784b` 的 `FormField` 仍允许 render function 与直接 ReactNode 两种 children，却总生成 `label[for]`，只有前者接收 control/hint id。结构化 TSX 扫描确认当前生产有 34 个直接 child：20 个直接 Input/Textarea，5 个外层含辅助按钮或样式容器但仍有唯一 Input/Switch 主控件，9 个为 Base URL、TabList、RadioButtonGroup、OAuth 连接等真正复合内容；现有单测还把无 id 的直接 child 固化为成功。报告形成后的 Provider 提交只迁移少数调用为 render prop，未收口 primitive 或其余调用。开放 #73/#75 均不含 FormField、十个目标调用文件或同功能实现；根因确定成立且不需要产品选择。
- planned 实施：只扩展既有 `FormField` 类型与渲染分支并迁移现存直接调用。默认 control 模式仅接受 `(id, hintId) => ReactNode`；`group: true` 模式接受复合 ReactNode，标题改用稳定 id 的非 label 元素，内容容器以 `role=group`、`aria-labelledby` 和可选 `aria-describedby` 建立关系。25 个有唯一主控件的字段改用 render prop，并把 id/hintId 精确交给 Input/Switch 等主控件；9 个真正复合控件增加 group 声明；既有 29 个 render-prop 调用签名不变。不新增组件/依赖，不改字段值、handler、validation、disabled 或视觉 class。
- planned 验证：先增加直接 Input 的可见 label 查询与点击聚焦测试，证明旧实现失败；修复后覆盖 control 的 accessible name/description/focus、group 的 name/description 和无悬空 `htmlFor`。运行 FormField 三个测试文件、受影响调用方定向测试、完整前端单测、TypeScript、目标 ESLint/Prettier、Vite build、结构化生产调用扫描与 `git diff --check`。PR 前和合并前重新 fetch 并核对相关文件与开放 PR。遗留风险是自动化 DOM 语义不能替代 VoiceOver/NVDA 人工读屏，且 group 模式增加一层语义 wrapper；完整测试、构建和代表性页面回归用于约束布局风险。
- 2026-08-06 本地实施：失败优先新增可见标签查询、提示描述与点击聚焦回归，旧实现得到 1 failed / 6 passed；修复严格限于 `FormField`、三个既有测试文件和十个生产调用文件。primitive 改为 TypeScript 可判别的 control/group 联合类型，control 以原生 `label[for]` 关联唯一主控件，group 以稳定标题 id 和 `role=group` 建立 name/description；25 个字段迁移 render prop，9 个复合字段声明 group。结构化扫描得到 60 个生产调用（51 control、9 group、0 invalid）；8 个定向文件 143/143、全量 312 个文件 2811/2811、TypeScript、目标 ESLint/Prettier、Vite production build 与 `git diff --check` 全过。初始候选 `f22f8f16` 已提交且工作树干净；未改业务值、handler、validation、视觉 class、依赖或后端。真实 VoiceOver/NVDA 尚未人工验证，最新主线整合与 PR 门仍待执行。
- 2026-08-06 最新主线整合计划：`origin/main` 已由 #73/#75 推进至 `ff09a81a`；`94da784b..ff09a81a` 不修改 14 个目标文件，也没有同类 FormField 标签合同实现，只有无交集的观测/日志、版本与 Homebrew 发布改动。计划将初始候选 `f22f8f16` 无冲突重放最新 main，重新执行 143 项定向测试、2811 项全量前端单测、TypeScript、目标 ESLint/Prettier、Vite build、60 调用 AST 合同、diff/范围检查，再提交 Ready PR。无根本冲突或待决策项，尚未执行重放。
- 2026-08-06 最新主线整合结果：初始候选已无冲突重放 `origin/main@ff09a81a` 为 `90230c56`，merge-base 与 base 均为 `ff09a81a`，仍严格 14 个计划文件且工作树干净。重放后 8 个定向文件 143/143、全量 312 个文件 2814/2814、TypeScript、目标 ESLint/Prettier、Vite production build、60 调用 AST 合同（51 control、9 group、0 invalid）与 diff 全过；仅有既有 Recharts/JSDOM、Browserslist 与大 chunk 警告。没有重复、覆盖、根本冲突或待决策项；PR 前远端门仍待执行。
- 2026-08-06 提交前主线门与 PR：再次 fetch 后 `origin/main`、branch base 与 merge-base 均为 `ff09a81a`，当前没有开放 PR；候选 `90230c56` 严格 14 个目标文件且工作树干净。分支已推送并创建 Ready PR #76，GitHub 显示 MERGEABLE，Actions `31039483396` 运行中。没有重复、覆盖、根本冲突或待决策项。
- 2026-08-06 合并结果：Actions `31039483396` 的 frontend、rust、合同与 `ci-gate` 全部通过。合并前再次 fetch，`origin/main`、PR base 与 merge-base 仍为 `ff09a81a`，仅 #76 开放且 CLEAN/MERGEABLE；squash 合并为 `9e83772c`。合并后的 14 个目标文件树与候选 `90230c56` 完全一致，远端不再有开放 PR。真实 VoiceOver/NVDA 人工读屏仍未执行，作为自动化语义验证之外的遗留风险。

### AUD-040：本地插件预览与最终安装仅绑定路径，存在审核-安装 TOCTOU

- 状态：`resolved`（PR #68，`e94c83bd`）
- 优先级：`P2`
- 判断依据：插件包拥有代码和权限语义；用户批准的是预览的 A，但确认阶段会重新从同一路径读取 B，任何可写该路径的本地进程都能改变实际安装内容。
- 文件和行号：预览状态和确认 `src/pages/PluginsPage.tsx:482-486, 524-567`；前端 service `src/services/plugins.ts:118-158`；后端预览/安装入口 `src-tauri/src/commands/plugins.rs` 中对应 `plugin_preview_from_file` / `plugin_install_from_file`。
- 证据与触发路径：页面只保存 `{ filePath, preview }`；预览完成后确认调用 `installMutation.mutateAsync(filePath)`。两个 IPC command 的唯一关联是 path，没有内容摘要、文件标识、一次性 token 或已审核字节缓存。
- 实际影响与根因：最终安装的 manifest、代码、权限和版本可能与用户看到的预览不同，破坏插件审查边界。根因是审核结果没有绑定不可变内容。
- 最小修复建议：预览返回包 digest 和短期一次性 token，安装时在后端重新算 digest 并拒绝不一致；或由后端安全缓存预览的字节并从该副本安装。
- 验证及回归测试：预览包 A 后原子替换为包 B，确认安装必须返回 `PACKAGE_CHANGED_SINCE_PREVIEW`；未变更文件仍可成功安装，token 不能重复使用。
- 2026-08-05 最新主线复核：`origin/main@d5c9cfe0` 仍由页面保存 `{ filePath, preview }`，确认安装只提交 path；同构的本地更新确认也只提交 path。`PluginInstallFromFileInput` 没有 checksum，命令使用默认 `LocalPackageInstallPolicy`；服务虽已支持 `expected_checksum` 并从首次读取的 `package_bytes` 校验，却在安装验证后又 `fs::copy(package_path)` 到缓存，留下第二次路径读取窗口。`405a545f..d5c9cfe0` 只含 #66 的 CI 三文件，开放 PR #67 只改 `response_cache.rs`，无文件或行为冲突；`PENDING.md` 无未解决条目。
- planned 实施：前端本地安装/更新 mutation 改为提交 `{ filePath, expectedChecksum }`，checksum 分别取预览 `preview.trust.checksum` 与本地更新 `diff.trust.checksum`；IPC 在 `PluginInstallFromFileInput` 增加可选 `expected_checksum` 保持旧调用兼容，两个命令把它传入现有 `LocalPackageInstallPolicy`。安装端继续用现有 SHA-256 校验和 `PLUGIN_CHECKSUM_MISMATCH`，并将已验证的 `extracted.package_bytes` 写入缓存，不再重读原路径。签名、权限、developer mode、远程安装及版本更新语义不变。
- planned 验证：failure-first 证明当前确认只提交路径且安装缓存二次读取；前端 service/query/page 测试断言预览 checksum 逐层传入安装与本地更新，Rust 测试覆盖 A 预览后替换为 B 时在文件/DB 副作用前拒绝、未变包成功且缓存字节等于已验证字节。运行允许的 Vitest、TypeScript、ESLint、Prettier、Vite、源合同和 diff；Rust format/Clippy/tests/bindings drift 交由 Actions，PR 前和合并前重新 fetch。
- 2026-08-05 本地执行状态：分支 `codex/audit-plugin-preview-install-content-binding` 的初始候选 `f775b76f` 修改 9 个文件。失败优先的 3 个前端测试文件先得到 7 failed / 38 passed，精确证明 service、query 和页面确认仍只传路径；修复后 45/45 通过。`PluginInstallFromFileInput` 增加可选 checksum，安装/更新命令接入现有 policy；安装缓存改写已验证的 `package_bytes`。Rust 回归源码覆盖安装与更新的 A/B 替换拒绝、旧 IPC 字段兼容和缓存字节一致性。本地 TypeScript、目标 ESLint/Prettier、Vite production build、8/8 源合同和 diff 检查通过。
- 2026-08-05 PR 状态：#67 合并后重新 fetch，确认 `d5c9cfe0..0854d830` 只修改无交集的 `response_cache.rs`；候选无冲突重放为 `834876d0`，base/merge-base 均为最新 `origin/main@0854d830`。重放后 3 个前端文件 45/45、TypeScript、目标 ESLint/Prettier、Vite build、8/8 源合同和 diff 再次通过，Ready PR #68 已创建；按仓库规则未运行 Rust 工具，等待 Actions 的生成漂移、格式、Clippy、Rust tests 与依赖审计。
- 2026-08-05 执行结果（`resolved`）：首轮 Actions `31014368791` 的 frontend、云端格式和绑定生成均通过，唯一 Clippy 失败是旧无策略安装包装函数只剩测试调用；候选以 `4ce877b6` 增加 `#[cfg(test)]` 最小收口。第二轮 run `31015354600` 的 frontend、format/bindings、Clippy、Rust tests、依赖审计和 ci-gate 全绿。合并前再次 fetch，确认 `origin/main`、base 与 merge-base 仍为 `0854d830`，只有 #68 开放，9 个文件无竞争实现且 CLEAN/MERGEABLE；随后 squash 合并为 `e94c83bd`。
- 遗留风险（计划锁定）：本批用 SHA-256 内容身份而不新增一次性 preview token；同一份字节重复确认仍沿用既有行为，重复导入同 ID/版本的快照一致性由 `AUD-050` 单独处理。路径被删除或变为不可读会明确失败，不会安装未审核内容。

### AUD-041：损坏的 `attempts_json` 元素会在请求详情渲染时击穿全局错误边界

- 状态：`resolved`
- 优先级：`P2`
- 判断依据：后端历史数据、迁移或插件输入只需存入 `[null]` 这一合法 JSON 数组，即可让用户打开请求详情后落入应用级错误页。
- 文件和行号：解析 `src/services/gateway/attemptsJson.ts:1-42`；直接解引用 `src/components/ProviderChainView.tsx:70-95`；全局边界 `src/main.tsx:19-21`。
- 证据与触发路径：parser 只检查顶层是数组，随后把元素强制断言为 attempt 类型；`[null]`、空对象或字段类型错误均会通过。`ProviderChainView` 直接读取元素字段，抛出的 render error 只能由根 `ErrorBoundary` 捕获。
- 实际影响与根因：单条损坏日志可替代整个应用主界面，用户无法继续排障或删除该记录。根因是反序列化边界没有逐项 schema 验证，且局部详情没有错误隔离。
- 最小修复建议：对每个元素做 runtime type guard/schema parse，跳过或明确标记损坏条目；详情组件增加局部错误态，避免全局崩溃。
- 验证及回归测试：覆盖 `[null]`、`[{}]`、错误字段类型与混合数组，断言详情保持可用并显示损坏记录提示；合法 attempts 仍完整渲染。
- 2026-08-04 当前主线复核：共享 parser 虽新增 `stream_internal_error` 局部校验，但非对象元素仍被强制断言为 `AttemptJsonEntry`，`ProviderChainView` 会直接读取其字段；`[null]` 的崩溃路径仍存在。整数组 fail closed 可避免过滤元素后破坏 attempt 索引归因。
- 计划：子任务 `.trellis/tasks/08-04-attempts-json-entry-validation` 校验所有进入渲染/聚合的标量字段，任一元素损坏时返回 `null`；无兼容 logs 时显示局部错误态，有 logs 时保留现有回退。验证覆盖 parser、组件和错误摘要调用者以及完整前端质量门。
- 执行结果：提交 `22240539`，draft PR #44；新增逐项对象、必需标量和可选标量守卫，任一坏元素令整数组返回 `null`，不改变数组位置。无兼容 logs 时显示 `role=alert` 的局部解析失败状态，有 logs 时保持兼容链路和原有失败提示。3 个定向文件、49 tests，TypeScript、ESLint、目标 Prettier、Vite build、diff 和差异审查均通过；PR 前重新 fetch 的 `origin/main` 和 GitHub API 均确认基线仍为 `fef05dec`，无主线冲突。首次 Git HTTPS/REST 写入遇到 SSL/EOF，重试后分支与 PR 已成功创建；GitHub Actions 待终态。

### AUD-042：依赖审计脚本对未知/畸形 `pnpm audit` 成功响应 fail-open

- 状态：`resolved`
- 优先级：`P2`
- 判断依据：脚本是供应链安全 gate；当 audit 服务的 JSON schema 漂移、返回顶层 error 或 advisory 格式变化时，CI 会把无法解释的结果视为“无阻断漏洞”。
- 文件和行号：解析/跳过逻辑 `scripts/check-pnpm-audit.mjs:92-120, 288`；把容忍行为固化为预期的 selftest `scripts/check-pnpm-audit.selftest.mjs:119`。
- 证据与触发路径：检查器对非数组、畸形条目和未知 severity 静默跳过，只验证顶层为 object。已用顶层 `error`、critical object 和未知 severity 的模拟成功响应验证，均不会产生阻断结果。
- 实际影响与根因：新的 high/critical 公告格式或 audit 失败伪装为成功时可随 CI 绿灯进入 release。根因是安全检查把“不能确定安全”当成“没有发现漏洞”。
- 最小修复建议：严格验证顶层、advisory 字段和 severity 枚举；未知、畸形、顶层 error、非零 exit 或不可解析内容一律失败，并把解析版本固定在测试 fixture 中。
- 验证及回归测试：为三类 payload 加负向 selftest，确保都非零退出；正常 npm/pnpm audit fixture 仍给出可读的高危列表。
- 2026-08-04 当前主线复核：`extractSeverityCounts`、`blockingAdvisoryEntries` 仍对非数组、非对象和未知 severity 静默跳过，主入口只校验顶层为 object；frontend CI job 会执行 `pnpm audit:deps`。计划仅收紧 response JSON 边界，不改 high/critical 阈值、例外、registry 或 workflow。
- 2026-08-04 执行结果：提交 `43532603` / draft PR #45。单一 validator 在统计和例外判断前拒绝顶层 error、非数组列表、非对象/缺失 severity 条目与未知 severity；保留大小写归一、high/critical 阈值和 GHSA 例外。失败优先 selftest、两份 Node 语法、目标 Prettier、diff 和安全差异审查通过；PR 前 `origin/main` 与 merge-base 均为 `fef05dec`。本机无 `pnpm`，真实 registry 路径等待 Actions。

### AUD-043：更新器私钥在候选构建后提升为 job 级环境变量

- 状态：`resolved`（PR #66，`d5c9cfe0`）
- 优先级：`P2`
- 判断依据：更新器私钥可签署受客户端信任的更新；把其值放入 `$GITHUB_ENV` 会使后续所有仓库脚本和第三方 Action 获得读取机会，明显扩大密钥暴露面。
- 文件和行号：`.github/workflows/ci.yml:319, 339, 387`。
- 证据与触发路径：CI 为处理换行把 `TAURI_SIGNING_PRIVATE_KEY` 写入 `$GITHUB_ENV`。GitHub Actions 的该机制使变量对同一 job 随后的步骤可见，直至 job 结束；签名构建后仍存在打包、脚本和上传步骤。
- 实际影响与根因：任一后续依赖、Action 或被注入的脚本一旦失陷，可能读取并外传用于伪造 updater payload 的私钥。根因是密钥作用域按 job 扩散，而不是按单一签名步骤最小化。
- 最小修复建议：将规范化后的密钥写入 `$RUNNER_TEMP` 的受限文件，仅在签名 build step 以路径或 step-scoped env 传入，随后删除；更强方案是独立隔离签名 job。
- 验证及回归测试：构建仍生成可验证签名；后续步骤探针确认环境变量不存在、临时文件已删除；审查所有后续 Action 不再接触私钥。
- 2026-08-05 最新主线复核：`origin/main@405a545f` 的 `build-release-candidate` 仍在 `Validate updater signing secrets` 将规范化私钥追加到 `$GITHUB_ENV`，随后 `Prepare updater assets`、portable ZIP 与上传步骤均继承该 job 环境。#65 只改 `src-tauri/src/app/observer/mod.rs`，当前没有开放 PR 或同功能实现。Tauri v2 合同允许 `TAURI_SIGNING_PRIVATE_KEY` 接收私钥文件路径，因此根因可在不改 Action pin、CLI 版本或制品合同的前提下局部修复。
- planned 实施：保留现有 secret 非空校验、换行规范化和 signer probe；将规范化 key 写入固定 `$RUNNER_TEMP/tauri-updater.key` 并 chmod 600，签名 Action 仅以 step-scoped env 接收 `${{ runner.temp }}/tauri-updater.key` 路径，紧随其后的 `if: always()` 步骤删除该明确文件。新增独立 Node 静态合同，锁定 validation/build/cleanup 顺序、禁止 `$GITHUB_ENV` 和 direct secret 传递、清理后的后续不可达性及 support-contract 接线。
- planned 验证：failure-first 仓库扫描先拒绝当前 `$GITHUB_ENV`；self-test 覆盖直接 secret、缺失/提前/仅 echo 清理、后续引用和接线断开。修复后运行新合同、现有 CI quality/dev-build/release promotion 合同、目标 Prettier、diff 与 PR Actions。真实签名 candidate 构建仍由下一次正常 main candidate run 验证，不为当前已发布版本制造额外同版本候选制品。
- 2026-08-05 执行结果（`resolved`）：提交 `1fcc687d` 只改 `.github/workflows/ci.yml`，并新增 `scripts/check-release-signing-secret-scope.mjs` 与 `.selftest.mjs`。私钥不再写入 job `$GITHUB_ENV`，而是经 `umask 077` 写入 `$RUNNER_TEMP/tauri-updater.key`、chmod 600，并只向 `Build signed Tauri candidate` 传入路径；`Delete updater signing key` 以紧邻的 `if: always()` 删除。新合同同时禁止 `GITHUB_ENV`、`GITHUB_OUTPUT`、`GITHUB_PATH`、`GITHUB_STATE`、`GITHUB_STEP_SUMMARY` 跨步骤传播，并要求唯一的私钥 secret 引用位于 validation step。
- 本地验证：failure-first 先拒绝旧 `$GITHUB_ENV`；新 self-test 覆盖五类 command-file、direct secret、secret 错位、workspace 路径、权限、缺失/提前/仅 echo cleanup、后续引用和接线断开。`check-release-signing-secret-scope`、CI quality、dev-build artifact、release source/promotion、TUI release、change-scope 合同均通过；目标 Prettier、`git diff --check` 与 staged diff 通过。独立差异审阅发现并关闭 `GITHUB_OUTPUT` 绕过。
- 远端交付：PR 前和合并前两次 fetch 后 `origin/main` 与 merge-base 均为 `405a545f`，没有新主线提交、其他开放 PR 或相关文件漂移；Actions `31005579029` 的 support/frontend/Rust/ci-gate 全绿，PR #66 squash 合并为 `d5c9cfe0`。
- 遗留风险：runner 内签名 Action 与其直接构建进程按功能仍必须读取私钥文件和密码；本项消除的是签名完成后对无关步骤的暴露，不把签名拆成独立 job 或外部 KMS。

### AUD-044：macOS/Linux 开发制品直接上传后丢失可执行权限

- 状态：`resolved`
- 优先级：`P2`
- 判断依据：dev-build 的交付物下载后不能直接运行，影响测试人员和发布前验证；这是 Actions artifact 已知的权限语义，不是推测。
- 文件和行号：`.github/workflows/dev-build.yml:45-55, 118-122`；上传 Action 固定为 `actions/upload-artifact@v4.6.2`。
- 证据与触发路径：workflow 将 `.app` 目录和 AppImage 直接上传。`upload-artifact@v4` 下载时不保留 Unix mode，文件会成为 `0644`；macOS `Contents/MacOS/*` 和 Linux AppImage 因此不可执行。
- 实际影响与根因：成功的 macOS/Linux dev build 交给测试者后还需手工 chmod，常被误判成制品损坏或无法验收。根因是将需要保留 mode 的文件直接交给不保留 mode 的 artifact 格式。
- 最小修复建议：macOS 使用 `ditto`/zip，Linux 使用 tar 包装可执行制品，上传归档文件而不是原始目录/文件；下载脚本负责解包。
- 验证及回归测试：下载新制品并解包，断言 macOS 主二进制和 AppImage 通过 `test -x`，再执行最小启动 smoke。
- 2026-08-05 当前主线复核：`origin/main@ef41e6da` 的 dev-build 仍把 `bundle/**` 原样交给 `upload-artifact@v4.6.2`；macOS `.app` 和 Linux AppImage 没有先归档，workflow 也没有权限保真合同或下载后 `test -x`。PR #53/#54 与该 workflow 无交集，根因仍成立。
- 计划：子任务 `.trellis/tasks/08-05-dev-build-executable-artifacts` 只调整 dev-build 的平台输出准备与上传路径：Windows 保持现有文件，macOS 用 `ditto -c -k --sequesterRsrc --keepParent` 归档 `.app`，Linux 用 tar 保留 AppImage mode；增加 Node 静态合同/self-test 锁定平台分支、归档命令和上传白名单。真实解包后的可执行位与启动 smoke 由手动 dev-build Actions 验证。
- 2026-08-05 实施前复核：最新 `origin/main@ed72549b` 只新增 #55 的 Provider 路由 data model/测试，与 dev-build 无文件或行为交集；workflow 仍直接上传 `bundle/**`。精化计划为：Windows 准备目录只复制唯一 MSI 与唯一 EXE；macOS 唯一 `.app` 经 `ditto` zip；Linux 唯一 AppImage 与至多一个 deb 经 `cp -p` 后 tar；三个分支均在上传前验证唯一/非空与可执行位，upload-artifact 只读取统一白名单目录。Node 合同/self-test 同时锁定四目标映射、平台命令、唯一上传步骤及 support-contract 接线。PR #57 会在同一 `ci.yml` support-contract 邻近位置增加独立步骤，预判为可兼容的顺序追加；其若先合并，本分支须重基后重跑合同。
- 2026-08-05 实施中审查补充：当前 checker 对 macOS/Linux 关键动作仅做子串存在检查，`echo 'cp -p ...'`、赋值或其他不执行归档的文本可绕过合同；修订为在对应命名 step 内匹配真实命令形态，并增加 `echo`/死文本负例。与 PR #57 的 `ci.yml` support-contract 是同位置的可整合文本冲突，合并时保留两组步骤、重跑双方 checker/self-test，不构成待决策冲突。
- 2026-08-05 最新主线整合计划补充：已在 `origin/main@0062c907` 解决与 #57 的预期文本冲突并保留三项 support-contract。最终差异审阅发现 dev-build 合同自身的 CI 接线仍可被 `run: echo 'node ...'` 子串绕过；PR 前将其收紧为精确单行 `run` 命令并新增死文本负例，再重跑双方合同、前端静态门、YAML/Bash、真实 tar/ditto fixture、Prettier 与 diff。
- 2026-08-05 执行状态：原实现与审阅修补重放为 `f4a3e0e1`、`d120f229`，ready PR #59 最终基线和 merge-base 均为 `origin/main@0062c907`。`ci.yml` 冲突按计划保留 CI 矩阵、Instant 防线和 dev-build 合同三项步骤；没有功能或接口二选一。Windows 仍准备唯一 MSI/EXE；macOS 用 ditto zip 并解包检查 `Contents/MacOS` 可执行位；Linux 用 `cp -p` 和 tar 并解包检查 AppImage；上传仅读取统一准备目录。dev-build、CI 矩阵、Instant、plugin-hardening、脚手架 33 tests/typecheck、根 TypeScript/ESLint、Node/YAML/Bash、Prettier、diff 和真实 tar/ditto mode fixture 全部通过。遗留风险：本机没有 `pwsh`/`actionlint`，Windows PowerShell 解析及四平台真实构建、下载和最小 smoke 由 Actions 验证；归档文件名变化已写入 workflow summary。
- 2026-08-05 最新主线重放：#58 合并后无冲突重放到 `origin/main@5b13683b`，PR #59 新头为 `2e519e51`。#58 的 CodeEditor 文件与本项无交集；重放后 dev-build/CI 矩阵/Instant/local-build 合同、plugin-hardening、脚手架 33 tests/typecheck、根 TypeScript/ESLint、Node/YAML/Bash、Prettier、diff 和 tar/ditto mode fixture 再次通过。Actions run `30984325182` 的 support-contract 已成功，frontend/rust 运行中；标准 CI 全绿后从该最终头触发四平台 dev-build。
- 2026-08-05 合并结果：标准 CI run `30984325182` 全绿；同一头提交的 macOS x64/arm64、Windows x64、Linux x64 dev-build runs `30986450817`/`30986450918`/`30986450908`/`30986450875` 均成功。下载后解包确认两个 macOS 主二进制为 `rwxr-xr-x` 且分别为 Mach-O x86_64/arm64，Linux AppImage 在 tar 与解包后均为 `rwxr-xr-x`/ELF x86_64，Windows 为 PE32+ x86_64。合并前 `origin/main` 仍为 `5b13683b`、PR head 为 `2e519e51` 且 CLEAN/MERGEABLE，随后 #59 squash 合并为 `62574e22`，无相关主线竞争实现或待决策冲突。macOS arm64 bundle 的 Info.plist 与二进制 ad-hoc 签名可读，但开发包没有完整 bundle resource seal；这是既有 unsigned dev-build 签名属性，本项不扩展为签名治理。

### AUD-045：插件 hook 只有 RPC 单次超时，没有覆盖排队、冷启动和整条流的端到端预算

- 状态：`resolved`
- 优先级：`P1`
- 判断依据：所有经过同一插件的网关请求可在单插件 mutex 上无界排队；一个每次 4.9 秒但未超时的 hook 会使第 N 个请求等待约 `N x 4.9s`。流式响应还对每个 chunk 串行等待全部插件，N 又不受流级上限约束。
- 文件和行号：registry 锁与冷/热启动 `src-tauri/src/app/plugins/extension_host_registry.rs:355-390`；RPC timeout 边界 `src-tauri/src/app/plugins/extension_host_process.rs:174-220`；runtime executor `src-tauri/src/app/plugins/runtime_executor.rs:100-121`；stream hook 循环 `src-tauri/src/gateway/plugins/pipeline.rs:744-940, 1216-1238`；upstream chunk backpressure `src-tauri/src/gateway/streams/plugin_chunk.rs:143-170`。
- 证据与触发路径：registry 在拿到 per-plugin async mutex 后才执行 worker RPC，manifest `timeoutMs` 仅包 write/read，调用方没有外层 `tokio::timeout`。多请求/stream 共用该 lock，且没有 queue cap。stream 每读一个 upstream chunk 都 await `apply_plugin_chunk_hooks`；慢但未超时的 handler 会 `record_success`，不会打开 circuit，因此可对任意数量 chunk 持续增加延迟。
- 实际影响与根因：一个慢插件可以把所有受影响请求、SSE 连接、内存和 upstream socket 长时间占住，同时 fail-open/fail-closed 的行为只在实际 RPC 超时后才生效。根因是为了保护单一 QuickJS state 的串行锁没有配套 admission control、总 deadline 和流级资源合同。
- 最小修复建议：在进入 registry 前建立 invocation absolute deadline，把锁等待、host 启动、activation 和 RPC 全纳入剩余预算；按 plugin 配置有界 semaphore/queue；为 stream 定义累计 wall-time 和 processed-chunk 上限，超限按明确 policy 终止或绕过并只记录一次摘要。
- 验证及回归测试：用并发 deferred handler 验证排队请求在自己的预算到期，不是依次延长；使用慢但成功的多 chunk SSE 验证端到端延迟、连接释放和上限行为；覆盖 fail-open 与 fail-closed。
- 2026-08-06 最新主线复核：PR #84 已 squash 合并为 `4800bc87`；在该最新 `origin/main` 中 gateway pipeline 仍只把 `hook_timeout` 传给 registry。registry 在 deadline 外依序等待 operation gate、plugin lock、warm/cold 分支，冷启动又包含 ready/handshake/module load/activation。底层 JSON-RPC timeout 仅包 write/read 并会 kill child；若粗暴在 registry 外层套 timeout，warm future 被取消后实例仍在 map，未消费 response 可污染下一次调用。stream 仍为每个 chunk 重开预算，故根因完整成立。#84 的 storage helper 与本批 cancellation/timeout 只在相邻 host 方法处共存，无功能或接口冲突；候选从 `origin/main@4800bc87` 新建。
- planned 实施：任务 `.trellis/tasks/08-06-plugin-hook-invocation-deadline`。本批只交付单次 gateway hook invocation 的 absolute deadline：同一 `timeout_at` 截止时间覆盖 gate、plugin lock、旧实例清理、factory start、activation 与 warm/cold RPC；新增取消专用 abort，warm timeout 先取消 RPC future，再按 key/`Arc` 身份摘除实例并 `start_kill` child，统一返回 `PLUGIN_EXTENSION_HOST_TIMEOUT`。严格限定 `extension_host_registry.rs`、`extension_host.rs`、`extension_host_process.rs` 和 `runtime_executor.rs` 测试，不改 timeout 数值、manifest/SDK/DTO、command 路径或 pipeline policy。
- planned 验证：failure-first 源合同证明旧代码缺少 absolute deadline/abort；Rust 回归覆盖 operation gate、同插件排队、cold start、warm RPC timeout 与下一次冷启动，真实 Extension Host 覆盖 activation/module load 超时后无协议残留。运行静态源合同、精确四文件范围、`git diff --check` 与独立差异审查；本机不运行 native，format/Clippy/Rust tests/audit 交 Actions。遗留队列容量和整条 stream 累计 wall-time/chunk 预算仍保持 `confirmed` 子范围，需另定耗尽后的 bypass/终止与审计语义。
- 2026-08-06 候选实施：任务 `.trellis/tasks/08-06-plugin-hook-invocation-deadline`，候选 `1417c045` / Ready PR #85 严格四文件。registry 在入口建立单一 Tokio absolute deadline，覆盖 context 后检查、operation gate、plugin-lock map、单插件 mutex、旧实例清理、factory、activation/RPC、缓存插入与 LRU 清理；process/host/registry 新增不发送 deactivate 的取消专用 abort，warm/cold/LRU 超时均按 key/`Arc` 身份摘除并 `start_kill` child，统一返回 `PLUGIN_EXTENSION_HOST_TIMEOUT`。
- 2026-08-06 本地验证与 PR 前门：failure-first 四项源码合同在旧实现按预期失败；实施后 gate/queue/warm/cold factory/cold execution/LRU 与真实 activation recovery 七类回归已写入。`pnpm check:plugin-hardening`（SDK 30 tests、API/SDK/脚手架类型合同）、`pnpm check:gateway-error-codes`（40 codes）、静态 deadline/abort/LRU 合同、精确四文件范围与 diff 通过。三轮独立审查发现并修复 LRU cleanup 超时保留新实例及 activation 测试上界过宽；按规则未本地运行 Rust/native。PR 前 fetch 确认 `origin/main`/merge-base 均为 `4800bc87`，无开放 PR、主线漂移、重复实现或冲突。queue 容量与 stream 累计预算仍不在本批。
- 2026-08-06 首轮 Actions 与云端格式修正：run `31076737365` 的 change-scope、pr-title、support-contract 通过，Rust 只在 generated-file drift 门失败，Clippy/Rust tests/audit 因此尚未执行。artifact SHA-256 `0408082c4c3b969801813cf188509351b8dc0d0bbdd5a74261d2030b25f818c9` 只重排 `extension_host_registry.rs` 与 `runtime_executor.rs`，原样提交为 `27efd051`；plugin hardening、40 错误码、deadline/abort/LRU 源合同、范围与 diff 复跑通过。推送前 `origin/main`/merge-base 仍为 `4800bc87`，唯一开放 PR 为 #85；第二轮 Actions `31077183327` 已触发，终态见下一条。
- 2026-08-06 最终验证与合并：第二轮 Actions `31077183327` 的 frontend、云端 format/bindings、Clippy、Rust tests、provider 百万行 release benchmark、依赖审计和 `ci-gate` 全绿。合并前再次 fetch，确认 `origin/main`、PR base 与 merge-base 均为 `4800bc87`，主线在四个目标文件上没有新提交，唯一开放 PR 为 CLEAN/MERGEABLE 的 #85，无重复、覆盖或冲突实现。Ready PR #85 squash 合并为 `735cec12`；合并后 `origin/main` 指向该提交，四文件与候选 head `27efd051` 一致，远端无开放 PR。单次 invocation 的 deadline 根因已闭合；queue capacity 与 stream 跨 chunk 累计 wall-time/chunk 预算仍需独立产品策略，不在本项已解决边界内。

### AUD-046：Extension Host 的 idle recycle 没有生产调度，空闲子进程可永久常驻

- 状态：`resolved`
- 优先级：`P2`
- 判断依据：注册表明确声明 120 秒 idle recycle、子进程又有 30 秒 `recycle_if_idle`，但生产没有任何 timer 调度它们；最后一次调用后内存/进程资源会持续到下一次请求、生命周期变更或退出。
- 文件和行号：idle 常量与状态 `src-tauri/src/app/plugins/extension_host_registry.rs:24-52`；仅在下次执行时机会式清理 `src-tauri/src/app/plugins/extension_host_registry.rs:280-290, 366-376`；子进程 recycle `src-tauri/src/app/plugins/extension_host_process.rs:47-50, 224-230`；QuickJS heap `src-tauri/src/app/plugins/extension_host_worker.rs:241-243`。
- 证据与触发路径：全仓 `dispose_idle`/`recycle_if_idle` 的生产调用不存在，只有各自测试；registry 的 `remove_idle_locked` 只在新的 command/hook 到来时运行。一次插件调用后若再无流量，worker 不会在 30/120 秒边界退出。
- 实际影响与根因：虽然每 registry warm instance 有数量上限，多个插件仍会常驻多个最高 32 MiB heap 的 QuickJS 子进程，浪费内存/文件描述符和进程资源，并延长卸载前的非预期驻留窗口。根因是回收逻辑实现为被动检查，却没有定时触发者。
- 最小修复建议：注册周期性 idle sweep，或在创建实例时安排可取消 deadline task；卸载/禁用时统一 cancel 和 dispose。
- 验证及回归测试：以可控时间推进，在最后一次请求后断言 registry 实例数归零、子进程退出；有新请求时验证 deadline 正确重置且不误杀活跃 worker。
- 2026-08-06 最新主线复核：`origin/main@d26524f2` 中 command 路径由 `ExtensionHostRuntimeState::registry` 创建一个 registry，gateway 路径由 `RuntimeGatewayPluginExecutor::with_db` 创建另一个 registry；两者都只构造 `Arc`，没有启动回收任务。registry 仍只在下一次 command/hook 冷启动前调用 `remove_idle_locked`，公开 `dispose_idle` 只有测试调用；`ExtensionHostChildProcess::recycle_if_idle` 仍未被 `ExtensionHostInstance` 或 registry trait 转发，故 30 秒 child 和 120 秒 registry 两层合同均无生产触发。#72 只改插件 pipeline 并合并为该主线；开放 #73 的观测/日志/用量文件不含 `extension_host.rs`、`extension_host_registry.rs` 或 `runtime_executor.rs`，没有重复、覆盖或接口冲突。根因确定成立且无需产品决策。
- planned 实施：只修改 `src-tauri/src/app/plugins/extension_host.rs`、`extension_host_registry.rs`、`runtime_executor.rs`。为 `ExtensionHostInstance` 和 registry 内部 process trait 转发既有 `recycle_if_idle`；registry 提供 command/gateway 共用的生产 shared 构造器，构造 `Arc` 时只启动一个持 `Weak` 的 5 秒 Delay sweeper。每 tick 先通过实例 process mutex 调用 30 秒 child recycle，再以写屏障调用现有 120 秒 `dispose_idle`，避免移除正在执行的实例；单个 child recycle 错误只记录并保留 120 秒 registry dispose 兜底。RuntimeState 与 RuntimeGatewayPluginExecutor 都改用 shared 构造器；不改插件协议、manifest、超时值、heap、DB、依赖或公开 DTO。
- planned 验证：先运行 failure-first 源合同，证明旧代码没有生产 shared 构造/sweeper、没有 child recycle 转发、两个生产构造点仍直接 `Arc::new`；旧实现应失败。新增缩短 sweep/idle 间隔的 Rust 回归：一次 command 后不再发请求，仍应观察 child recycle check、registry dispose 和实例数归零；drop 最后一个 registry `Arc` 后弱引用任务应退出。修复后重跑源合同、`git diff --check`、三文件范围和差异审查；按仓库规则不在本地运行 Cargo/rustfmt/Clippy/Rust tests，完整原生验证交给 Actions。遗留风险是 5 秒粒度使回收最多晚约一个 tick，registry 存活期间空闲 sweeper 仍每 5 秒唤醒；异常 child recycle 由 120 秒整实例回收兜底。
- 2026-08-06 本地实施结果：从 `origin/main@d26524f2` 建立 `codex/audit-extension-host-idle-sweeper` 独立 worktree。failure-first 的 7 项生产源码合同在旧代码上 0/7 通过；最小实现后 7/7 通过，`git diff --check` 通过且差异严格为上述 3 个文件。两个缩短间隔的 `tokio::test` 分别覆盖无后续调用时的 child check/registry dispose/弱任务退出，以及 timer 等待活跃 command 后再移除。独立差异审查未发现生产 P0/P1；发现测试释放信号使用 `notify_waiters` 可能在 waiter 注册前丢失的 P2，已改为可保留 permit 的 `notify_one`，并给 command join 增加 1 秒超时后复验 7/7。按仓库规则未本地运行 Rust 工具链，当前仍等待 PR 前最新主线门和 Actions。
- 2026-08-06 首轮 Actions 修补计划：run `31030242037` 的 change-scope、PR title 与 support-contract 已通过；Rust 只在云端生成/格式漂移门失败，尚未进入 Clippy/Rust tests。artifact `cloud-native-fixes-f76ffd2b582a50b7150101c609a89d985b653528-1` 仅含 `extension_host_registry.rs` 的 rustfmt 折行（3 insertions/12 deletions），没有逻辑、绑定或额外文件。先 fetch 并核对最新 `origin/main` 与开放 PR；无根本冲突时原样应用该单文件 artifact，重跑 7/7 源合同、diff、三文件范围和 merge-base，单独提交格式修补并重新交 Actions。若主线出现相关实现则先整合，不能共存则保留候选并登记待决策。
- 2026-08-06 合并结果：云端 artifact 原样形成格式提交 `a50ec5be`，第二轮 Actions `31030917177` 的 frontend、rustfmt、Clippy、Rust tests、依赖审计、support-contract 与 `ci-gate` 全绿。合并前 `origin/main`、PR base 与 merge-base 仍为 `d26524f2`，head 为 `a50ec5be` 且 CLEAN/MERGEABLE；#73 无相关漂移。Ready PR #74 squash 合并为 `94da784b`，合并后三文件树与候选一致。遗留风险仅为回收最多晚一个 5 秒 tick、空 registry 存活期间周期唤醒，以及 child recycle 错误由 120 秒整实例回收兜底；未引入待决策项。

### AUD-047：缓存命中率趋势允许无界日期和 Provider 笛卡尔积，WebView/SQLite 工作量没有上限

- 状态：`resolved`
- 优先级：`P2`
- 2026-08-04 复核：`d18da74e` 至 `fcc2a4d9` 已建立最多 10 个 Provider、120 个时间桶和 1200 行的共同硬预算，`limit: null` 也会规范化为 Top 10；现行边界见 `src-tauri/src/domain/usage_stats/trend_common.rs:7-9, 55-75, 412-420`。大型历史库的 p95 仍需运行时基准，但原“Provider × 日期无界”结论已失效。
- 判断依据：用户可选择任意长日期范围，后端又接受不限 Provider 的查询，前端按天构建并为每个 Provider 渲染 series；数据量随 `Provider x 日期` 同时增长，足以阻塞本地 WebView。
- 文件和行号：日期输入 `src/hooks/useCustomDateRange.ts:35-51`；按日循环/series `src/components/usage/UsageProviderCacheRateTrendChart.tsx:191-205, 332-360, 495-505`；调用不设 limit `src/pages/usage/useUsagePageDataModel.ts:107-110`；后端不限结果 `src-tauri/src/domain/usage/cache_rate_trend_v1.rs:46-50`。
- 证据与触发路径：custom range 没有最大跨度，query 传 `limit: null`，后端将其解释为无限。图表为每个 Provider 构造每个日期的点并分别渲染折线；数据获取、JS 分配和 SVG/canvas work 都没有 shared budget。
- 实际影响与根因：长保留期、较多 Provider 或异常历史数据会让图表页面卡顿、内存上升，并放大 SQLite 查询成本。根因是前端展示限制和后端查询限制都缺失，任一层都无法保护整体工作量。
- 最小修复建议：前后端共同限制日期跨度和 Provider/数据点总数；对超额 Provider 聚合“其他”或要求过滤，必要时对日粒度降采样。
- 验证及回归测试：覆盖极长范围和大量 Provider，断言接口拒绝/夹紧、图表数据点不超过上限，测量渲染与查询 p95；正常短范围保持精确数据。

### AUD-048：编辑 MCP Server 会把 workspace 作用域的 `enabled` 缓存状态覆盖为 `false`

- 状态：`resolved`
- 优先级：`P2`
- 判断依据：编辑一个当前 workspace 已启用的 Server 后，UI 立即显示未启用；下一次开关会重复发送 enable，用户无法按预期关闭，属于可复现的状态一致性错误。
- 文件和行号：前端缓存 replacement `src/query/mcp.ts:38-65`；不带 workspace 的 SQL 投影 `src-tauri/src/domain/mcp/db.rs:272`；upsert 返回该投影 `src-tauri/src/domain/mcp/db.rs:557`。
- 证据与触发路径：mutation 直接把 upsert 返回对象写回 query cache。该 Rust 查询固定返回 `0 AS enabled`，因为它没有 workspace context；前端因而用非作用域投影覆盖了 workspace list 中原本 `enabled: true` 的行。
- 实际影响与根因：UI 与实际 workspace 状态分叉，用户后续 toggle 产生错误请求或无法关闭已启用 Server。根因是 mutation 的返回 DTO 缺少必要 scope，却被当作同一缓存实体。
- 最小修复建议：upsert 后失效并重取 workspace-scoped list；或只更新不依赖 workspace 的字段并保留缓存中的 enabled。长期应让后端返回显式 workspace-scoped DTO。
- 验证及回归测试：缓存预置 `enabled:true`，模拟 edit/upsert 后仍应显示 true；下一次 toggle 必须发 disable，而不是再次 enable；覆盖非当前 workspace。
- 2026-08-04 当前主线复核：`useMcpServerUpsertMutation` 仍将 upsert DTO 整体替换 `mcpKeys.serversList(workspaceId)` 行；后端 `get_by_id` 固定投影 `0 AS enabled`，而页面 toggle 直接对缓存值取反。计划改用该模块现有 import mutation 的精确 workspace-key invalidation，不改 Rust DTO。
- 2026-08-04 执行结果：提交 `18b983e5`、draft PR #46；只修改 `src/query/mcp.ts` 与对应 query test。失败优先场景确认旧实现覆盖作用域状态；修复后只失效当前 workspace list 且不直接替换缓存。MCP query/view 共 16 tests、TypeScript、ESLint、Prettier、隔离 Vite build、diff 与 PR 前主线门均通过；遗留风险为保存后增加一次权威列表 IPC 重取，Actions 待终态。

### AUD-049：MCP JSON 的 1 MiB service 限制可被 UI 回退解析绕过

- 状态：`resolved`
- 优先级：`P2`
- 判断依据：服务层明确定义 1 MiB 输入上限，但前端捕获所有异常后仍在主线程对同一大文本执行无界 `JSON.parse`；合法超大 JSON 能稳定绕过该资源保护。
- 文件和行号：service 限制 `src/services/workspace/mcp.ts:20, 99-101`；浏览器回退 `src/pages/mcp/components/McpServerDialog.tsx:285, 810-842`。
- 证据与触发路径：dialog 先走受限 service parse，catch 不区分超限、结构错误或 IPC 错误，随后调用本地 `JSON.parse`。因此超过 1 MiB 的合法 JSON 不会停止，而是在 WebView UI thread 分配/解析。
- 实际影响与根因：大文本可造成 UI 卡顿与内存峰值，服务层容量合同形同虚设；错误语义也从“输入超限”变为行为不一致。根因是不同解析路径没有共享同一边界验证，fallback 捕获范围过宽。
- 最小修复建议：在任何解析之前统一检查 byte length；只对允许本地恢复的 syntax/shape 错误 fallback，超限/IPC 错误直接显示明确提示。
- 验证及回归测试：超限 JSON 不得调用本地 parser 且显示大小错误；限额内的结构 fallback 仍正确；测量主线程在大输入时没有长任务。
- 2026-08-04 当前主线复核：现有常量实际为 `MCP_PARSE_JSON_MAX_CHARS`，service 在 IPC 前检查 trim 后字符数；唯一生产调用者 `McpServerDialog.fillFromJson` 会在任意 service reject 后执行无界 local fallback。计划在入口复用既有字符上限，特意不把本批扩大为 byte-length 合同变更。
- 2026-08-04 执行结果：提交 `919c48a9`、draft PR #47；只修改 `McpServerDialog.tsx` 与对应测试，在任何 service/local parse 前复用现有字符上限。失败优先回归确认旧实现会解析并填充超限 JSON；修复后 dialog/service 共 17 tests、TypeScript、ESLint、Prettier、隔离 Vite build、diff 与 PR 前主线门通过。遗留风险是 textarea 仍可持有大文本，本批只封住用户点击导入后的解析路径；Actions 待终态。

### AUD-050：重复导入同 ID/版本会让已安装代码与版本快照不一致

- 状态：`resolved`
- 优先级：`P2`
- 判断依据：用户可通过普通导入重复安装相同插件；文件目录被替换，但同版本数据库快照不更新，后续回滚/诊断面对的是不同内容，属于明确的版本完整性问题。
- 文件和行号：已有状态预览 `src-tauri/src/app/plugin_service.rs:1005-1034`；安装与目录替换 `src-tauri/src/app/plugin_service.rs:1542-1624, 2472-2495`；版本记录 `src-tauri/src/infra/plugins/repository.rs:244-313`。
- 证据与触发路径：preview 能查到现有插件，但普通 install confirmation 没有阻断同 ID；安装先替换当前代码目录和插件记录，版本表却用 `INSERT OR IGNORE`。若同版本包字节不同，版本行/回滚快照仍保留旧 manifest 或内容引用。
- 实际影响与根因：插件可被静默禁用、降级或替换为与版本历史不一致的代码；回滚结果和审计记录不能证明实际执行内容。根因是安装、更新、版本快照三种状态转换没有统一不可变版本约束。
- 最小修复建议：普通 install 遇到已有 ID 必须拒绝并要求专用 update 流；版本唯一键同时绑定内容 digest，同版本不同内容一律拒绝；目录替换和数据库版本写入用可恢复的 staged transaction。
- 验证及回归测试：覆盖重复 ID、同版本同字节、同版本不同字节和低版本包；拒绝路径不改变现有目录/状态，合法 update 与 rollback 后代码 digest、manifest 和版本记录必须一致。
- 2026-08-06 最新主线复核：`origin/main@ecd82606` 仍在 install/update 中先删除 `installed/<id>/<version>` 再写主记录，而 `plugin_versions` 对同键继续 `INSERT OR IGNORE`；rollback 随后会组合旧 manifest 与被替换目录。PR #68 的 preview checksum 已完整绑定确认字节，但不约束历史版本身份；其后主线没有安装、更新、回滚、repository 或插件 UI 相关提交，远端无开放 PR，因此没有重复实现或主线冲突。
- 计划：Trellis 任务 `.trellis/tasks/08-06-plugin-version-immutability` 仅修改 `src-tauri/src/app/plugin_service.rs` 与 `src-tauri/src/infra/plugins/repository.rs`。Repository 增加“当前或历史版本是否已记录”的精确查询；本地/远端 install/update 的预览与执行都对已记录 `(plugin_id, version)` 返回稳定 blocker/error，官方 install 在资源物化前执行同一检查，历史版本切换继续使用现有 rollback。包状态变更在单进程内串行化，目录 promotion 拒绝覆盖现有版本目录且失败清理只删除本次新建目录。保留不同全新版本的普通导入、全新低版本的 downgrade warning、现有 config/permission/audit 语义；不新增 digest/schema 迁移，不处理旧数据库中已存在的不一致快照。
- 验证计划：failure-first 覆盖普通 install 和 update 的当前版本/历史版本重写、官方同版本重新物化、目录字节与 plugins/plugin_versions/config/permissions/audit 不变量、v1→v2 后拒绝再次导入 v1 且 rollback 仍恢复原 v1、全新版本和全新低版本不回归、目标目录已存在时不删除。执行 `git diff --check`、精确两文件范围和源合同检查；本地不运行 Rust 工具链，云端必须通过格式/绑定漂移、Clippy、Rust tests、frontend、依赖审计和 `ci-gate`。PR 前及合并前重新 fetch `origin/main` 并核对相关漂移。
- 2026-08-06 本地实施与 PR 状态：最终候选 `e89efd3c` 严格只有两目标文件。Repository 查询同时覆盖当前和历史版本；预览与执行复用 `PLUGIN_VERSION_ALREADY_INSTALLED`，官方安装在资源物化前阻断；安装、更新、官方安装和 rollback 使用同一进程内锁，目录 promotion 不再删除既有目标。新增当前/历史重复包、官方同版本重装、全新版本兼容、repository 查询与 rollback 回归。`git diff --check`、精确范围、12/12 源合同和两轮独立审查通过。提交前 `origin/main`、分支 base 与 merge-base 均为 `ecd82606`，远端无开放 PR；首轮 Actions `31052415102` 只报 rustfmt 漂移，原样应用云端制品后第二轮 `31052820007` 运行中。按规则未本地运行 Rust 工具链。
- 2026-08-06 合并结果：第二轮 Actions `31052820007` 的 frontend、云端格式/绑定、Clippy、Rust tests、依赖审计和 `ci-gate` 全绿。最终主线门确认 `origin/main`、PR base 与 merge-base 均为 `ecd82606`；新开放 #80 只改 CI/audit/lock，无功能或文件重叠。#79 CLEAN/MERGEABLE 后 squash 合并为 `cab1229a`，合并后两目标文件 blob 与候选一致。

### AUD-051：Plugin SDK 的公共类型没有覆盖 Host 已暴露的 storage/diagnostics API

- 状态：`resolved`
- 优先级：`P2`
- 判断依据：这是正式 SDK 与运行时的跨层合同漂移；插件作者必须使用 `any` 或私有声明才能调用文档化功能，使严格类型检查无法保护 API 兼容性。
- 文件和行号：SDK `PluginApi` `packages/plugin-sdk/src/index.ts:273-277`；worker 暴露 API `src-tauri/src/app/plugins/extension_host_worker.rs:528-570`；Host 实现 `src-tauri/src/app/plugins/extension_host.rs:352-458`；公开 storage 文档位于 `docs/plugins`。
- 证据与触发路径：worker 注入并 Host 实现 storage/diagnostics 方法，文档也把 storage 作为可用能力；SDK 的公共 `PluginApi` 类型只声明较小集合，没有这两个命名空间。用 SDK 严格编译的插件不能类型安全访问实际支持的 API。
- 实际影响与根因：官方/第三方插件会绕过类型系统，Host 参数或返回值漂移只能在运行时暴露；脚手架 typecheck 即使进入 CI 也无法覆盖这些真实 API。根因是 Host、SDK、文档分别手工维护，没有生成或双向合同测试。
- 最小修复建议：补全 SDK 类型与 JSDoc，从共享 schema/IDL 生成 Host/SDK 表面；把官方示例和脚手架作为严格 TypeScript 合同测试。
- 验证及回归测试：官方示例在不使用 `any` 的情况下调用 storage/diagnostics 并通过严格 typecheck；Host 删除/变更方法时合同测试必须失败。
- 2026-08-05 当前主线复核：fetch 后的 `origin/main` 仍为 `5b13683bd2a44699cd8c99e7aeffc317bcc19674`。SDK `PluginApi` 仍只声明 commands/gateway/privacy；worker 仍按 `storage.plugin` 注入同步 `get/set`、按 `diagnostics.read` 注入同步 `getRuntimeReports`；Host 分别返回 JSON/null、void 包装和 camelCase `PluginExtensionExecutionReport[]`。既有 v1 合同只记录 capability/storage 边界，合同脚本没有把这三种方法与 SDK/worker/Host 同时锁定。
- 计划与边界：只补 SDK 的 storage/diagnostics/运行报告类型、SDK 参考文档、v1 capability-to-API 映射和既有合同脚本/类型测试；不改 Rust、授权、持久化、限额、错误或诊断保留语义。先在旧 SDK 上取得 typecheck/合同失败，再运行 SDK、插件合同/文档、脚手架、根 TypeScript、目标 lint/format 与 Host 删除负例。#59/#60 文件与功能均无交集；PR 前仍按最新主线门复核。
- 2026-08-05 实施与验证：在 `codex/audit-plugin-sdk-host-api-contract` 以 `c62c4725` 提交并创建 ready PR #61，随后无功能变更重放为 `29c2139e` 到 `origin/main@d12dbfe3`。旧 SDK 的 typecheck/合同门先按预期失败；修复后 SDK 29 tests、SDK typecheck、plugin-hardening、plugin-system-docs/completion、脚手架 33 tests/typecheck、根 TypeScript、目标 ESLint/Prettier、Node 语法、Vite build 与 diff 均通过。临时删除 Host diagnostics route 后合同门按预期失败，恢复后正例通过。合并前 PR 为 `CLEAN/MERGEABLE`，差异仅为五个 SDK/文档/合同文件且与主线新功能无重叠；Actions `30993065410` 的必需检查全部成功，#61 squash 合并为 `ba06dabb0229e1f83ef34d468f54ef1d37f39f16`，`git ls-remote origin refs/heads/main` 已复核远端同值。本地未运行 Rust/native 工具链；既有测试/脚本未使用变量的 ESLint 报错已在未改动基线复现。遗留风险为未来 Host API 漂移，现由 SDK/worker/Host 合同门阻断。

### AUD-052：`CodeEditor` 首次动态加载失败后无法在当前会话恢复

- 状态：`resolved`
- 优先级：`P2`
- 来源：由原 `HYP-005` 在 2026-08-04 当前代码复核中晋升。
- 文件和行号：全局 import Promise `src/ui/CodeEditor.tsx:29-47`；消费 effect `src/ui/CodeEditor.tsx:75-160`。
- 证据与触发路径：`loadMonaco()` 把 `Promise.all(import(...))` 缓存在模块级变量中；effect 只有成功回调，没有 catch、失败态或清空缓存。首次 chunk/import reject 后，全局变量永久保留 rejected Promise，后续重新挂载或打开编辑器只会复用同一拒绝结果，不会再发起加载。
- 实际影响与根因：一次瞬时 chunk、网络或 WebView 资源加载失败会使当前会话所有 `CodeEditor` 保持不可用，用户只能重启应用。根因是把动态 import Promise 当成只会成功的永久单例。
- 最小修复建议：加载失败时仅清除仍由本次调用拥有的缓存 Promise，组件展示局部错误和显式重新加载动作；浏览器会缓存失败的 ES module 解析结果，因此不得把同一文档内再次调用相同 `import()` 伪装成可恢复重试。
- 验证及回归测试：单元测试固定首次 reject，断言失败缓存按代次安全清除、错误态尺寸稳定且重新加载动作可达；浏览器测试让首个 chunk 返回 503，确认同一文档内重试仍失败，再解除拦截并重新加载页面，重新打开编辑器后恢复。该项收益明确，但未选入当前批次。
- 2026-08-04 当前主线复核：模块级 Promise 仍只赋值不清理，effect 仍只有成功回调；现有 18 个 CodeEditor 单测均走成功加载，没有失败或重试覆盖。根因未变化。
- 2026-08-04 可达性复核（已被下条更正）：当时的调用面搜索漏掉经 CLI 管理页接入的生产调用者，因而得出的 `not_recommended` 结论无效；实验性改动已撤销这一历史事实不变。
- 2026-08-05 更正：`src/components/cli-manager/tabs/CodexTab.tsx:59,1523` 在生产 UI 中导入并渲染 `CodeEditor`。动态 import 的 rejected Promise 复用机制仍存在，因此恢复为 `confirmed`，且有明确修复价值。当前批次按优先级、改动面和独立验证成本未选中它，不修改产品代码；后续应以失败后显式 retry 和真实浏览器 chunk 故障回归单独处理。
- 2026-08-05 当前主线复核：`origin/main@ef41e6da` 的 `loadCodeMirrorBundle()` 仍永久缓存 `Promise.all(import(...))`，effect 仍只有成功处理器；现有 21 个 `CodeEditor` 测试全部走成功 mock。`CodexTab` 的生产 lazy import 和 JSX 调用仍存在，刚合并的诊断改动未覆盖该恢复路径。
- 计划：子任务 `.trellis/tasks/08-05-code-editor-production-load-retry` 先用可控 loader 证明首次 reject 后旧组件为空且重新挂载仍复用拒绝；随后仅在失败 Promise 仍拥有缓存时清空它，并为组件增加稳定尺寸的局部错误和显式页面重新加载。验证旧失败不会清除新一代 Promise、成功缓存仍复用、卸载不更新状态，以及现有生命周期/值同步/语言/只读/尺寸回归；再执行 Playwright 浏览器级失败后恢复。
- 2026-08-05 实施前复核：最新 `origin/main@ed72549b` 仅新增 #55 的 Provider 路由实现，`CodeEditor.tsx`、21 个测试与 `CodexTab` 生产调用均未变化；#56/#57 与 AUD-044 的候选改动也分别位于模型价格、CI 与 workflow，接口和文件无交集。初版计划曾尝试清缓存后在同一文档内重试；真实 Vite 页面以首轮 CodeMirror 请求 503 验证后，即使解除拦截仍保持 `alerts=1、editors=0`，证明浏览器 module map 会复用失败结果。修订计划保持现有 props/依赖/调用页不变，只增加 identity-safe reject 清理、卸载保护、同一 `height/minHeight` 的局部 `role=alert` 错误态和可靠的页面重新加载动作；失败优先测试覆盖 rejected cache 清理、旧失败不清新 Promise、卸载、成功缓存和 reload，现有 21 项行为继续回归。
- 2026-08-05 实施状态：初始提交 `b64289f9`、ready PR #58；#57 合并后无冲突重放到 `origin/main@0062c907`，更新头提交为 `c4f17111`，两个编辑器文件和调用合同与主线新增质量门无交集。修改 `src/ui/CodeEditor.tsx` 与 `src/ui/__tests__/CodeEditor.test.tsx`：失败 Promise 仅由自身清缓存，组件捕获拒绝并显示稳定尺寸、可访问的错误态，刷新图标按钮重新加载当前页面；窄视口隐藏说明文本但保留 alert 标签和 tooltip。重放后定向测试 25/25、TypeScript、目标 ESLint/Prettier、Vite build 与 diff 再次通过；新 Actions run `30981668718` 的 support-contract/frontend 已成功，rust 运行中。Playwright 在 1280x900 和 390x844 证明首轮 503 进入错误态、同文档重试无效，解除故障并重新加载后 URL 不变、`alerts=0`、`editors=1`，且无水平溢出。遗留风险：页面重新加载会重置未持久化页面状态，但这是清除浏览器失败 module map 的可靠边界；调用方自身 `React.lazy` chunk 失败仍由应用级错误边界负责。
- 2026-08-05 合并结果：Actions run `30981668718` 的 support-contract、frontend、rust 与 ci-gate 全部成功；合并前 fetch 确认 `origin/main@0062c907` 未漂移、PR 头仍为 `c4f17111` 且 CLEAN/MERGEABLE，随后 ready PR #58 squash 合并为 `5b13683b`。没有相关主线竞争实现或待决策冲突。遗留风险保持为页面 reload 会重置未持久化状态，以及调用方自身 `React.lazy` chunk 失败需应用级错误边界处理。

### AUD-053：供应商页没有默认选中当前实际活动路由

- 状态：`resolved`
- 优先级：`P2`
- 来源：用户于 2026-08-05 追加的明确需求，经最新主线调用链核验后登记。
- 判断依据：正常的异步加载即可稳定出现“活动模板带（当前）标记，但下拉仍选中 Default、成员列表也展示 Default”的自相矛盾状态；不影响网关实际选路，因此定为 P2。
- 文件和行号：无条件默认初始化 `src/pages/providers/hooks/useProvidersViewDataModel.ts:117-127`；活动值只用于当前态 `:561-598`；用户草稿选择 `:1098-1106`；下拉绑定与当前标记 `src/pages/providers/ProvidersView.tsx:133-134, 425-437`。
- 证据与触发路径：`createProviderUiState()` 每次都把 `routeDraftSelection` 设为 Default。`useSortModeActiveListQuery()` 从 `sort_mode_active` 返回当前 CLI 的活动模板，网关也读取同一表决定新请求路由，但前端只用该值计算 `currentRouteActive` 和“（当前）”标签，从未同步下拉 value。
- 实际影响与根因：用户进入供应商页或切换 CLI 后会先看到错误的编辑对象，可能误以为 Default 正在生效；根因是“实际活动路由”和“正在查看的草稿”缺少一次性初始化关系。后台 refetch 又不能无条件持续同步，否则会覆盖用户主动查看的非活动模板。
- 最小修复建议：活动路由与模板查询可判定后，按 CLI 只初始化一次草稿；活动值为 null 时选择 Default，活动模板存在时选择该模板。用户手动选择后标记草稿已拥有，后续 refetch 不覆盖；切换 CLI 时重新初始化。未知/已删除 mode ID 保守回退 Default。
- 验证及回归测试：以 deferred active query 复现旧页面仍选 Default；修复后断言模板 value、标签和成员一致。再覆盖活动值 null、手动选择后 refetch 不抢回、切换 CLI 各自初始化，以及初始化阶段零 `sortModeActiveSet` 调用。
- 2026-08-05 当前主线复核：`origin/main@eeccf64d` 的供应商页 hook、view、sort-mode query 与活动路由持久化均保持上述路径；#52 只调整托盘供应商展示，没有修改根因文件。首页路由控件已正确绑定活动值，托盘没有可选下拉，因此本项只处理供应商页。
- 计划：子任务 `.trellis/tasks/08-05-provider-active-route-default` 只修改供应商页 data model 与定向测试，不改后端、IPC、生成绑定或依赖；运行定向 Vitest、TypeScript、ESLint、Prettier、隔离 Vite build 和 diff 检查。
- 2026-08-05 实施状态：`codex/audit-provider-active-route-default` 最终 head `28f4b99f` 通过全部必需检查，PR #55 已 squash 合并为 `ed72549b`。草稿按 CLI 在活动路由和模板数据齐备后只初始化一次；Default、有效模板、未知模板回退、手动选择后 refetch、切换 CLI 和零 active-set mutation 均有回归。修改供应商页 data model 与定向测试；目标 42/42、TypeScript、全量 ESLint、Prettier、Vite build、diff 与 Actions 全部通过。一次全量并行测试曾触发与本项无关的 Prompt mutation 时序失败，隔离复跑通过。遗留风险是活动 mode 已删除时按既定策略保守展示 Default。

### AUD-054：云端验证与本地零产物合同未覆盖所有受控入口

- 状态：`planned`
- 优先级：`P1`
- 判断依据：当前工程约束只禁止 Cargo、Tauri 和部分 native 工具，却仍把依赖安装、Vite dev、TypeScript/Lint/前端测试和 build 描述为可在本地执行；本批要求将本地验证收窄为零依赖合同检查，避免在旧 worktree 或新候选中产生不可追踪的 Node/Rust 产物。
- 文件和行号：`AGENTS.md:10-11`；`README.md:193-202,261-265`；`README_EN.md:190-199,247-251`；`package.json:18-47`；`scripts/run-checks.mjs:19-35`；`scripts/check-local-build-entrypoints.mjs:1-38`；`.github/workflows/ci.yml:128-301`；活跃 `.trellis/workflow.md`、agent 模板和 cross-layer spec。
- 证据与触发路径：README 仍给出 `pnpm install`/`pnpm dev`；聚合检查器把 `format-check`、lint、typecheck、unit coverage 和 build 作为本地 stages；现有入口 checker 主要识别 `cargo/rustc/tauri`，不阻止 `pnpm install`、dev、test、lint 或 build。CI 已有 frontend、Rust format/bindings、Clippy、Rust tests、audit 和 `ci-gate`，但仓库规范与脚本没有形成唯一的本地/云端合同。
- 实际影响与根因：开发者可能在错误 worktree 运行受控脚本，产生 `node_modules`、Vite cache、`src-tauri/target*` 或 generated drift，随后把本地状态误当作验证证据；根因是“禁止 native”与“允许本地 frontend”之间的规则分裂，且静态 checker 没有覆盖完整脚本图。
- 最小修复建议：增加零依赖本地入口合同，静态拒绝依赖安装、dev、typecheck、lint、test、build 及其间接调用；README/AGENTS/活跃 Trellis 规范和模板只描述该入口与 Actions workflow_dispatch/dev-build。保留 CI 全量质量门，不把桌面打包设为每个 PR 必需任务。历史任务/归档只作为历史证据不改写。
- 验证及回归测试：对 package/workspace 脚本图、README 命令块、workflow 触发器和入口脚本做 Node AST/文本合同检查；构造允许的 `node --check`/`git diff --check` 与禁止命令 fixture；确认 `ci.yml` 仍运行完整 frontend/native gates，`dev-build` 仍按需 dispatch。

### AUD-055：Provider Sync 备份扫描归档会话并保留五代快照

- 状态：`planned`
- 优先级：`P2`
- 判断依据：正常 Provider Sync 会扫描和改写活动、归档 sessions，并为 SQLite/global state 建立备份；受管备份按五代保留，空间增长与本次 session-only 恢复目标不匹配。
- 文件和行号：`src-tauri/src/infra/codex_provider_sync.rs:18,151-201,454-472,571-610,1003-1114,1180-1234` 及其测试模块。
- 证据与触发路径：`collect_session_changes` 对 `sessions` 和 `archived_sessions` 使用同一递归 rollout 收集；`SyncChangeSet` 还包含 SQLite 与 global-state change；v1 manifest 的 `managed_by`/`version` 标记由 `create_backup` 写入；`prune_managed_backups` 只按创建时间排序并 `skip(PROVIDER_SYNC_KEEP_COUNT)`，其中 keep count 为 5。当前分类不足以区分 v1/v2 ownership，也没有保证归档字节不被触碰。
- 实际影响与根因：大体量归档会话被重复读写和复制，失败回滚面扩大；多代 backup 长期占用用户磁盘，且按目录/时间清理若证据不足可能误删非受管内容。根因是同步变更集、备份 manifest 和清理策略没有围绕“活动 sessions + 一代 managed backup”建立明确格式合同。
- 最小修复建议：引入显式 v2 session-only manifest；只枚举活动 sessions 与必要 config，按 manifest 精确分类 v2 managed、可迁移 v1 managed 和 unmanaged。成功建立 v2 后删除旧 v1 managed，最多保留一代 v2；无/损坏 manifest、marker 不匹配、symlink 或非受管目录一律不删除。保留现有失败快照与回滚语义。
- 验证及回归测试：云端 Rust 覆盖 archived session 不扫描/不改写、v1→v2 迁移、单代保留、替换与回滚；故意制造缺失/损坏 manifest、symlink 和非受管目录，断言原样保留；任一 config/session 写失败时断言活动文件恢复且归档不变。

### AUD-056：请求日志永久留存与运行日志无容量软上限

- 状态：`planned`
- 优先级：`P1`
- 判断依据：运行日志清理已有七天日龄，但请求日志默认仍为 `0=永久`；运行日志按日龄删除而不限制总容量；数据管理清空路径会自动 VACUUM，无法让用户观察或控制 SQLite 可回收空间。
- 文件和行号：`src-tauri/src/infra/settings/defaults.rs:65-70`；`src-tauri/src/infra/settings/migration.rs:760-772,1558-1559`；`src-tauri/src/infra/settings/persistence.rs:268-295,372-383`；`src-tauri/src/infra/request_logs.rs:406-570`；`src-tauri/src/app/logging.rs:101-210`；`src-tauri/src/infra/data_management.rs:90-110,170-185`；前端 settings validation/fixtures。
- 证据与触发路径：`DEFAULT_REQUEST_LOG_RETENTION_DAYS` 为 0，purge_expired 在 0 时直接禁用，settings validation 也允许 0；usage ledger 覆盖检查已存在。运行日志 cleanup 只按 modified time 和 retention days 逐文件删除，没有 256 MiB 总量、活动文件保护或超额保守记录。`clear_request_logs`/相关清理路径仍执行 `VACUUM`。
- 实际影响与根因：原始 request/error/retry 明细和运行日志可无限增长，长期桌面用户磁盘占用不可预测；自动 VACUUM 还会在用户未明确请求时长时间持有数据库资源。根因是明细 retention、永久聚合 ledger 和物理压缩没有分开建模。
- 最小修复建议：默认与历史 0 配置迁移为 7 天，所有写入口拒绝 0；保留 usage_ledger/统计聚合永久可查，只删到期原始明细。运行日志增加 256 MiB 软上限，超额只回收最旧已关闭滚动文件，活动文件永不删。清理后通过 `freelist_count * page_size` 展示可回收空间，移除自动 VACUUM，手动压缩才执行。
- 验证及回归测试：云端 Rust/前端覆盖 settings migration、ledger 覆盖、明细删除、上月统计、运行日志日龄/容量/活动文件、SQLite freelist 展示和手动压缩；确保不重构现有 request-log JSON 字段。

## 5. 待验证假设

### HYP-001：`quick-xml` 公告在当前产品路径中的可利用性

- 状态：`needs-verification`
- 现有证据：锁文件中的受影响版本来自 `plist` 与 `wayland-scanner`；初步静态扫描未发现应用源码直接调用 `quick-xml`，也未确认运行时依赖使用 `NsReader` 或默认 attributes 解析不可信 XML。
- 不能下结论的原因：构建期 `wayland-scanner` 与运行期 `plist` 的真实调用路径、上游 crate 内部实现及平台条件尚未形成完整可达图。
- 验证方法：在 CI 中使用 `cargo tree -i quick-xml@0.39.2 -e normal,build` 区分构建/运行依赖；检查锁定 crate 源码的 reader 类型和 attributes 用法；对任何可接收外部 plist/XML 的产品入口投递公告 PoC，并在隔离环境测量 CPU/内存上限。

### HYP-002：正式桌面制品是否必须具备平台发布者签名

- 状态：`needs-verification`
- 现有证据：`.github/workflows/ci.yml:298, 332, 435` 的 macOS 正式候选使用 ad-hoc identity；当前 release 配置只能确认 Tauri updater 签名，没有 Apple notarization、Windows Authenticode 或 TUI 可执行物平台签名步骤。
- 不能下结论的原因：代码能确认缺少平台身份签名，但仓库没有明确公开分发对象与支持政策；如果制品仅供已知用户手工放行，其严重度与公开消费者下载场景不同。
- 验证方法：锁定正式分发策略与支持平台；从实际 Release 下载制品，在 macOS 运行 `codesign --verify --deep --strict`、`spctl --assess`、`stapler validate`，在 Windows 检查 Authenticode 发布者和时间戳。若面向普通终端用户，应转为 confirmed 治理项。

### HYP-003：MDX 编辑器静态进入主入口是否造成显著启动/内存回归

- 状态：`needs-verification`
- 现有证据：`src/components/app/UpdateDialog.tsx:1-8` 静态导入 `@mdxeditor/editor`，`src/layout/AppLayout.tsx:54` 常驻挂载，`src/main.tsx:7` 全局导入其 CSS；现有构建产物可见 MDX/Lexical 标记且没有独立 MDX chunk。
- 不能下结论的原因：静态依赖存在，但没有稳定基线上的 bundle analyzer 与冷启动/内存对照，尚不能量化真实收益。
- 验证方法：在并行前端 PR 合并后的固定 SHA 上输出 bundle composition，测量未打开更新对话框的冷启动和 resident memory；再做动态 import 原型对照。达到项目性能阈值后再转 confirmed。

### HYP-004：OAuth token/account 头混搭是否会导致跨账户计费或授权

- 状态：`superseded`
- 现有证据：`AUD-028` 已确认来访 `chatgpt-account-id` 会覆盖选中 Provider 的派生账户头，从而与 bearer token 不匹配。
- 当前结论：提交 `e5d758b6` 已消除产品中的 token/account 混搭路径；每个 attempt 先移除来访账户头，再仅写入当前 Provider 派生 ID。外部服务对历史错误组合的行为仍未知，但已不构成当前代码修复前置或候选项。
- 验证方法：如需验证外部服务历史行为，只能使用两个完全隔离的测试账户，记录每次 outbound token/account 对、上游响应、配额和账单归属；不得使用生产账号或真实敏感数据。

### HYP-005：`CodeEditor` 动态加载首次失败后是否无法在当前会话恢复

- 状态：`confirmed-promoted`
- 现有证据：`src/ui/CodeEditor.tsx:29-47, 75-160` 缓存全局 import Promise，仅有成功处理，没有 catch 后清空 rejected Promise 或可重试 UI。
- 当前结论：标准 Promise 语义足以确认模块级 rejected Promise 会被永久复用，现行 effect 又没有失败恢复分支；该缺陷已晋升为 `AUD-052`。真实 chunk 注入仍作为实施后的浏览器回归，不再作为确认缺陷的前置。

任何其他缺少可复现路径、运行证据或完整调用链的怀疑项均在此记录，不进入问题索引。

### 5.1 2026-08-04 上一轮状态复核（历史）

本节记录 2026-08-04 当时的判断，已由 5.2 的最新主线复核取代。该轮以当时磁盘代码和 `origin` 历史为准，不把旧报告行号或提交标题直接当作结论；未运行本地 Rust/Cargo 命令。

| 当前分类 | 项目 | 复核结论 |
| --- | --- | --- |
| `resolved` | `AUD-013, AUD-018, AUD-019, AUD-020, AUD-028, AUD-047` | AUD-019/AUD-020 的 PR #40 已通过全部必需检查并以 `cec2353f` 合并；其余四项由既有修复完整覆盖。本批不再修改。 |
| `pr_open` | `AUD-004, AUD-009, AUD-011, AUD-014, AUD-024, AUD-036, AUD-041, AUD-042, AUD-048, AUD-049` | 原 #41 至 #50 已关闭，最终内容统一进入 Ready PR #51；#51 已开启 CI 通过后自动 squash 合并。停止监控时 frontend 与四个合同/范围 job 通过，rust 仍在运行，合并前不转为 `resolved`。 |
| `not_recommended` | `AUD-052` | 根因代码仍在，但 CodeGraph 的唯一调用者是测试，全量 `src` 无生产 JSX、导入或调用页面；没有可达用户界面或可验证收益，实验性改动已撤销。 |
| `confirmed` | `AUD-001, AUD-002, AUD-003, AUD-005, AUD-006, AUD-007, AUD-008, AUD-010, AUD-012, AUD-015, AUD-016, AUD-017` | 现行实现和基线后历史未消除根因；保留原优先级，未选入当前批次。 |
| `confirmed` | `AUD-021, AUD-022, AUD-023, AUD-025, AUD-026, AUD-027, AUD-029, AUD-030, AUD-031, AUD-032, AUD-033, AUD-034, AUD-035` | `AUD-021` 只有局部清洗改善，主泄露链仍在；其余根因仍存在。 |
| `planned` | 无 | 当前已选修复均已实施并进入 #40/#51；下一批尚未写入 planned。 |
| `confirmed` | `AUD-037, AUD-038, AUD-039, AUD-040, AUD-043, AUD-044, AUD-045, AUD-046, AUD-050, AUD-051` | 根因仍存在且有明确修复价值，但未选入当前 planned 批次。AUD-039 需迁移 35 个调用点并逐一处理复杂控件语义，留待单独批次。 |
| 假设 | `HYP-001, HYP-002, HYP-003` | 代码证据仍不足以决定可利用性、平台签名政策或 MDX 性能收益，不进入修复。 |
| 失效假设 | `HYP-004` | `AUD-028` 已移除当前产品中的混搭路径；不再作为修复候选。 |
| 不进入修复 | `HYP-005 / AUD-052` | Promise 根因成立，但当前没有生产调用路径，不能按“确定有修复价值”标准进入产品变更。 |

复核中确认但未选入本批的项目不是“不推荐修复”，而是受以下边界约束暂缓：`AUD-016/026/027/045` 需先锁定认证、fail-closed、超限和端到端 deadline 产品合同；`AUD-008/012/034` 需先锁定生命周期或原子 patch/CAS 语义；假设项仍缺运行证据或产品决策。`AUD-009/011/014` 因明确生产触发路径、局部合同和确定性测试进入当前批；`AUD-005/006/044` 保留给随后的 CI/制品批次，`AUD-043` 需先确认签名 Action 的 step-scoped 密钥合同；`AUD-039` 需迁移 35 个调用点，保留给单独可访问性批次；`AUD-052` 因无生产调用路径标记为 `not_recommended`。

### 5.2 2026-08-05 `origin/main` 全量复核

本轮先 fetch `origin/main`，再在 `eeccf64dc2d60698d0df48ff3fcbcd2aafd24688` 的独立 worktree 中按领域重新追踪既有 52 个问题和 5 个假设的现行符号、生产调用者、测试与 #51/#52 历史；随后以同一基线核验用户新增需求并登记 `AUD-053`。没有读取或操作 `upstream`，没有运行本地 Rust/Cargo 工具链，也没有修改产品代码。

| 当前分类 | 项目 | 最新结论 |
| --- | --- | --- |
| `resolved` | `AUD-001, AUD-004, AUD-005, AUD-006, AUD-009, AUD-011, AUD-012, AUD-013, AUD-014, AUD-015, AUD-017, AUD-018, AUD-019, AUD-020, AUD-021, AUD-022, AUD-023, AUD-024, AUD-028, AUD-029, AUD-030, AUD-031, AUD-032, AUD-036, AUD-037, AUD-038, AUD-040, AUD-041, AUD-042, AUD-043, AUD-044, AUD-047, AUD-048, AUD-049, AUD-051, AUD-052, AUD-053` | #40/#51/#53/#54/#55/#56/#57/#58/#59/#60/#61/#62/#63/#64/#65/#66/#67/#68/#69/#70/#71/#72 均已合并并通过必需检查；最新 merge commit 为 `d26524f2`。 |
| `pr_open` | 无 | 当前审计候选均已合并；开放 PR #73 属于独立观测/日志任务，不计入本报告候选。 |
| `planned` | `AUD-046` | `origin/main@d26524f2` 的 command/gateway 两个生产 registry 均无 idle timer；计划以共享构造器启动单一弱引用 sweeper，转发现有 child recycle 并保留 registry dispose。开放 #73 无相关文件。 |
| `confirmed` P1 | `AUD-008, AUD-010, AUD-016, AUD-026, AUD-027, AUD-045` | 根因仍存在。认证、插件 failure policy/heap/deadline 与生命周期类项目需要先锁定产品合同。 |
| `confirmed` P2 | `AUD-002, AUD-003, AUD-007, AUD-025, AUD-033, AUD-034, AUD-035, AUD-039, AUD-050` | 根因均仍存在且有修复价值；`AUD-035` 的零历史旧验收会改变摘要语义，需先重设计查询/索引。其余项目依赖未决产品合同、风险或改动面高于当前批次。 |
| 待验证假设 | `HYP-001, HYP-002, HYP-003` | 仍缺运行时可利用性、平台签名政策或 bundle/内存收益证据，不进入修复。 |
| 已取代假设 | `HYP-004, HYP-005` | `HYP-004` 的旧触发路径已由 `AUD-028` 修复；`HYP-005` 的机制已确认并由生产可达的 `AUD-052` 承接。 |

本轮没有发现必须二选一的主线冲突，也没有候选 PR 需要登记为“待决策”。未选项目不等于不推荐：确定有价值但依赖未决合同或成本更高的项目继续保持 `confirmed`，假设项则继续保留为待验证，不实施。

## 6. 治理批次

原始审计不实施修复；后续执行按以下收益、风险和前置依赖分批推进，同一批次内部仍拆为可独立验证的 Trellis task/PR。

| 批次 | 目标与范围 | 项目 | 前置/退出条件 |
| --- | --- | --- | --- |
| 0. 发布前安全与供应链阻断 | 建立网关调用者边界、兑现插件失败策略与总预算、修复自动发布并收紧签名密钥作用域 | `AUD-001, 016, 019, 021, 026, 027, 043, 045` | 先冻结相关发布/非回环暴露；以未认证 LAN、sentinel secret、并发慢 hook 和不可变资产集成测试退出 |
| 1. 数据完整性与状态版本化 | 消除旧快照整文档写回、恢复初始化/重置原子性、修复跨实体与版本历史错误 | `AUD-002, 004, 008, 011, 012, 034, 036, 037, 038, 048, 050` | 先定义 revision/CAS 或字段级 patch 合同；故障注入与逆序并发测试证明无丢更新、可恢复 |
| 2. 插件跨层合同收敛 | 让脚手架、SDK、manifest、Host 和运行时从同一 schema 生成并兑现激活/失败语义 | `AUD-006, 009, 010, 032, 033, 040, 051` | 依赖批次 0 的 failurePolicy/timeout 决策；SDK 严格 typecheck、官方样例和 Host 合同测试成为 CI gate |
| 3. 资源预算与性能上限 | 为缓存、observer、下载、图表、插件进程和 MCP 输入建立总预算/容量/淘汰策略 | `AUD-017, 022, 023, 029, 030, 031, 035, 046, 049` | 先定义可观测指标和合理数据规模；压力测试证明字节、条目、查询数、deadline 和进程数均有硬上限 |
| 4. CI、发布操作与依赖治理 | 把缺失/可绕过的检查变为真实合并门，修正文档和 dev artifact 合同 | `AUD-003, 005, 007, 020, 042, 044` | 依赖批次 0 的发布模型；反例 fixture、候选制品、下载权限和依赖审计在 PR CI 可验证 |
| 5. 产品正确性、恢复与可访问性 | 修复异步单槽、错误页、递归配置、标签关联、坏历史数据隔离、路由展示和动态加载恢复 | `AUD-014, 015, 024, 025, 039, 041, 052, 053` | 可与批次 1/3 并行；以慢请求、损坏数据、键盘/读屏、自指路由、活动路由初始化和 chunk 重试回归测试退出 |

关键依赖关系：`AUD-026` 与 `AUD-045` 的 policy/deadline 模型应先于 `AUD-032/033/051`；`AUD-019` 的不可变 release 模型先于 `AUD-020`；后端 revision/atomic patch 合同先于 `AUD-012/034/037/048` 的前端保护，避免只用 UI 串行化掩盖跨写者竞争。`AUD-036` 是独立的跨实体身份错误，不依赖该原子写入合同。

### 6.1 首批已进入 PR 的范围

| 顺序 | Trellis 子任务 | 报告项 | 选择理由 | 主要风险 |
| --- | --- | --- | --- | --- |
| 1 | `.trellis/tasks/08-04-release-asset-immutability` | `AUD-019, AUD-020` | `pr_open`：`caeaf348` / draft PR #40；两个问题共享同一 workflow 和不可变输入模型。 | 远端首次发布中断可能留下部分 Release；必须 fail closed，不自动覆盖或删除。 |
| 2 | `.trellis/tasks/08-04-db-init-retry-recovery` | `AUD-004` | `pr_open`：PR head `460845a0` / draft PR #41；DB 是全后端共同前置，修复集中在初始化门。 | 持续性错误会被再次执行；本批不引入自动重试或退避。 |
| 3 | `.trellis/tasks/08-04-plugin-detail-identity-guard` | `AUD-036` | `pr_open`：`05f62ad2` / draft PR #42；页面边界 identity gate 已由确定性慢查询回归验证。 | 不解决同一插件内部的并发配置写入，`AUD-034` 仍保留。 |

#### PR 1：Release 候选不可变与同标签串行

- 预期文件：`.github/workflows/release.yml`、release promotion Node self-test/helper、必要的 `package.json` 检查入口和 `.trellis/spec/aio-coding-hub/cross-layer/release-promotion-contract.md`。
- 实施：全局只接受一个与 source SHA 绑定的未过期候选；发布前核对同 tag 已有资产名称和 checksum；完全一致为只读幂等成功，任何差异在上传前失败；关闭 overwrite；concurrency key 改为最终 tag。
- 定向验证：0/1/多候选 fixture、既有 Release 同/异 digest、缺失/多余资产、tag push/dispatch 同 tag 锁、不同 tag 并行，以及现有 annotated-tag collision self-test。局部 Node 检查可本地运行，真实 Actions/Release 行为由 GitHub Actions 验证。
- 遗留风险：部分发布不自动修复；需人工判断或新 patch tag。

#### PR 2：DB 初始化失败后可重试

- 预期文件：`src-tauri/src/app/app_state.rs`，以及同模块或启动管线的定向 Rust 测试；除非测试证明必要，不修改其他启动代码。
- 实施：共享状态只缓存成功 `Db`，错误返回后保持未初始化；mutex 继续串行化首次初始化和 reset guard。
- 定向验证：首次失败/第二次成功、并发成功只初始化一次、成功缓存复用、reset guard，以及启动状态从 `Failed/InitializingDb` 经 retry 进入后续阶段。所有 Rust 验证只在 GitHub Actions 运行。
- 遗留风险：持续错误会在显式重试时再次发生；不处理 `AUD-008` 的后台 DB 所有者和重置原子性。
- 执行结果：功能提交 `979c6cfb`、CI 解析修补 `90fed48e`、云端 rustfmt 修补 `460845a0`，draft PR #41；实际只修改 `app_state.rs` 与 `startup_tasks.rs`，PR 前 `origin/main` 仍为分支基点 `fef05dec`，三个相邻路径无主线漂移。GitHub Actions 的必需检查均已通过。

#### PR 3：插件详情身份保护

- 预期文件：`src/pages/PluginsPage.tsx`、`src/pages/__tests__/PluginsPage.test.tsx`。
- 实施：只有详情自身 `plugin_id` 与当前选择 ID 相同才传给详情面板；不匹配时显示加载态并阻断保存、更新/回滚，不改变全局 query placeholder 策略。
- 定向验证：deferred A -> B 逆序时序中，A 配置/版本不可见且 mutation 为零；B 返回后内容和 target 均为 B；运行定向 Vitest、typecheck、lint 和 Vite build。
- 遗留风险：不处理 `AUD-034` 的 runtime storage/UI config 并发写入。
- 执行结果：提交 `05f62ad2`，draft PR #42；实际只修改 `PluginsPage.tsx` 与对应测试。失败优先回归、页面 34 tests、typecheck、lint、Prettier、Vite build 和 diff 检查通过；PR 前 `origin/main` 仍为基点 `fef05dec`，页面/query 无主线漂移；GitHub Actions 的必需检查均已通过。

#### 主线漂移与 PR 门

三个任务确认实施后均从当时最新 `origin/main` 建独立 worktree/branch。提交 PR 前再次 fetch `origin/main`，检查任务基线以来触及文件及相邻合同的功能、实现和效果变化；可共存则整合主线、修补当前提交并重跑全部定向验证。若只能二选一，则把任务标为 `blocked-main-conflict`，在本报告记录主线提交、文件、行为冲突与可选方案，不创建该 PR，并继续后续任务。

### 6.2 前端异常恢复批次结果

| 顺序 | Trellis 子任务 | 报告项 | 选择理由 | 主要风险与边界 |
| --- | --- | --- | --- | --- |
| 1 | `.trellis/tasks/08-04-updater-fallback-decode-guard` | `AUD-024` | `pr_open`：`7fbfd6c5` / draft PR #43；确定性异常、修复面仅一个解析边界，能恢复有效更新结果。 | 只保留原 updater 结果，不改变 GitHub fetch、超时或发布语义。 |
| 2 | `.trellis/tasks/08-04-attempts-json-entry-validation` | `AUD-041` | `pr_open`：`22240539` / draft PR #44；单条损坏历史可击穿全局 UI，已由共享 parser 和局部错误态收口。 | 整数组 fail closed，避免过滤后重排 attempt 索引；不修复后端历史数据。 |
| 3 | `.trellis/tasks/08-04-code-editor-load-retry` | `AUD-052` | 2026-08-04 历史结论：当时误判为无生产调用者，未实施。 | 该结论已被 2026-08-05 的 `CodexTab` 生产调用证据推翻；项目现为 `confirmed`。 |

#### PR 4：Updater fallback tag 解码保护

- 预期文件：`src/services/app/updater.ts`、`src/services/app/__tests__/updater.test.ts`。
- 实施：把 `decodeURIComponent` 纳入局部异常边界；解码失败返回无 fallback，使调用者沿现有路径保留原 updater 结果。
- 定向验证：`%ZZ`、截断 UTF-8、合法编码 tag；断言畸形输入不 fetch 且不拒绝，合法输入 API URL 正确；再跑 TypeScript、ESLint、Prettier、Vite build、主线漂移门和 Actions。
- 遗留风险：GitHub API 本身失败仍只保留 fallback 文本，这是既有 fail-soft 合同。
- 执行结果：`7fbfd6c5` / draft PR #43；10 个 updater tests、3 个关联 query tests、TypeScript、ESLint、Prettier、Vite build 和 diff 检查通过。PR 前 `origin/main` 仍为 `fef05dec`，无 updater/query/hook 或测试合同漂移；GitHub Actions 的必需检查均已通过。

#### PR 5：attempts_json 逐项校验与局部降级

- 预期文件：`src/services/gateway/attemptsJson.ts`、对应 parser 测试、`src/components/ProviderChainView.tsx` 与对应组件测试。
- 实施：逐项验证所有用于渲染/聚合的标量字段；任一损坏元素令整数组解析失败；只有损坏 JSON 时显示局部错误态，有兼容 logs 时继续现有回退。
- 定向验证：`[null]`、`[{}]`、错误字段类型、混合数组、兼容 logs 回退、合法链路和错误摘要；再跑完整前端质量门、主线漂移门和 Actions。
- 遗留风险：损坏数组不做部分展示或自动修复，原始 JSON 仍可在 raw tab 用于排障。
- 执行结果：`22240539` / draft PR #44；失败优先回归确认修复前 `[null]` 分别在链路视图和错误摘要抛错。修复后 3 个定向文件、49 tests，TypeScript、ESLint、Prettier、Vite build、diff 检查通过；重新 fetch 与 `main` API SHA 均为 `fef05dec`，无冲突。首次 Git HTTPS/REST 写入遇到 SSL/EOF，重试后分支与 PR 已成功创建；GitHub Actions 的必需检查均已通过。

#### AUD-052：2026-08-04 暂不修复结论（已更正）

- 当时证据：调用面搜索漏掉了 `CodexTab` 的生产导入与 JSX 使用，因而“只有测试调用者”的判断不成立。
- 结果：失败优先实验确实复现旧实现的空白容器，但没有可达产品入口，所有实验性代码/测试变更已经撤销，worktree 无差异。
- 当前决定与遗留风险：`AUD-052` 已恢复为 `confirmed`，本批仅因优先级和独立验证成本未选中。后续单独修复时需验证真实 WebView 对失败 module URL 的缓存语义，并提供显式 retry。

### 6.3 供应链与 MCP 边界计划

| 顺序 | Trellis 子任务 | 报告项 | 选择理由 | 主要风险与边界 |
| --- | --- | --- | --- | --- |
| 1 | `.trellis/tasks/08-04-pnpm-audit-fail-closed` | `AUD-042` | `pr_open`：`43532603` / draft PR #45；安全 gate 的已证实 fail-open 已由严格 JSON 边界和 selftest 收口。 | 服务端未来 schema 会从错误绿灯改为明确失败；真实 registry 由 Actions 验证。 |
| 2 | `.trellis/tasks/08-04-mcp-upsert-cache-refresh` | `AUD-048` | `pr_open`：`18b983e5` / draft PR #46；精确 query invalidation 已阻止非作用域 DTO 覆盖 workspace enabled 状态。 | 保存后增加一次列表 IPC 重取；不改变后端 DTO 或其他 workspace。 |
| 3 | `.trellis/tasks/08-04-mcp-json-input-boundary` | `AUD-049` | `pr_open`：`919c48a9` / draft PR #47；入口共享字符上限已阻止 service/local fallback 解析超限文本。 | 保持字符数而非字节数合同；不改变限额内 fallback 兼容性。 |

实施与验证计划：每项先写失败优先回归；AUD-042 验证顶层 error、非数组条目与未知 severity 均失败，保留正常 severity/豁免；AUD-048 验证 `enabled:true` 缓存不被 `enabled:false` 非作用域 DTO 覆盖且只失效当前 key；AUD-049 验证超限 JSON 零 service/fallback 调用、限额内 fallback 不回归。随后运行相应定向 Node/Vitest、TypeScript、ESLint、Prettier、Vite build 和 diff 检查。提交前重新 fetch `origin/main`，核对触及文件和直接调用合同；可共存才创建独立 draft PR，冲突不可共存则登记并继续下一项。

AUD-042 执行结果：只修改 `scripts/check-pnpm-audit.mjs` 与 `scripts/check-pnpm-audit.selftest.mjs`，提交 `43532603`，draft PR #45。五组畸形响应与既有正常/例外路径 selftest、Node 语法、目标 Prettier、diff 和安全差异审查均通过；主线门无漂移，GitHub Actions 必需检查全部通过。遗留风险是未来 registry schema 漂移会明确阻断 CI。

AUD-048 执行结果：只修改 `src/query/mcp.ts` 与 `src/query/__tests__/mcp.test.tsx`，提交 `18b983e5`，draft PR #46。失败优先回归确认旧实现会覆盖 `enabled:true` 缓存；修复后只失效 `mcpKeys.serversList(currentWorkspaceId)`，缓存留待权威 query 重取。MCP query/view 共 16 tests、TypeScript、ESLint、Prettier、隔离 Vite build、diff、主线门与 GitHub Actions 必需检查通过；遗留风险为成功保存增加一次列表 IPC。

AUD-049 执行结果：只修改 `src/pages/mcp/components/McpServerDialog.tsx` 与对应 dialog test，提交 `919c48a9`，draft PR #47。失败优先回归确认旧实现会调用 service 后在 fallback 解析并填充超限 JSON；修复后超限输入零 service 调用、零字段填充，限额内 fallback 与 service 边界测试保持通过。Dialog/service 共 17 tests、TypeScript、ESLint、Prettier、隔离 Vite build、diff、主线门与 GitHub Actions 必需检查通过；遗留风险为 textarea 本身仍可接收大文本。

### 6.4 运行时合同与并发状态计划

| 顺序 | Trellis 子任务 | 报告项 | 选择理由 | 主要风险与边界 |
| --- | --- | --- | --- | --- |
| 1 | `.trellis/tasks/08-04-plugin-scaffold-payload-contract` | `AUD-009` | `pr_open`：`c8aeffb5` / draft PR #48；三种生成代码已由生产外层 payload 行为回归验证。 | 不修既有已生成插件，不改变 Host/SDK payload，也不混入 AUD-006；Actions 必需检查已通过。 |
| 2 | `.trellis/tasks/08-04-startup-status-order-guard` | `AUD-011` | `pr_open`：`d4691401` / draft PR #49；事件、GET、retry 和订阅清理逆序均有定向回归。 | 前端 generation 只保证事件优先于并发响应，不替代后端 revision；frontend 通过，rust 运行中。 |
| 3 | `.trellis/tasks/08-04-cli-proxy-conflict-serialization` | `AUD-014` | `pr_open`：`dacb7518` / draft PR #50；同步 ref 单飞、prompt 所有权和释放路径均有定向回归。 | 不引入 prompt 队列；busy 期间拒绝新的启用预检，但保持关闭路径；frontend/rust 运行中。 |

实施与验证计划：AUD-009 先以生产形态外层 payload 执行三种生成代码，修复后验证 request/log/response 命中 patch 与未命中 pass；AUD-011 用 deferred GET/retry 和 listener ready 验证事件 generation 仲裁；AUD-014 用两个跨 key deferred checks 验证同 render 只有首个 IPC、prompt target 不被覆盖且所有退出路径释放锁。每项分别运行定向 Vitest/包测试、TypeScript、目标 ESLint/Prettier、相关合同检查、隔离 Vite build 和 diff 检查。提交前重新 fetch `origin/main` 并核对直接合同；可共存才创建独立 draft PR，严重冲突则登记并继续下一项。

AUD-009 执行结果：只修改三个生成模板、对应 scaffold test 和三份包含同错示例的公开文档，提交 `c8aeffb5`，draft PR #48。失败优先 VM 回归确认三类旧模板均静默 `pass`；修复后四个 handler 的命中 patch 和未命中 pass 均以生产外层 payload 执行。完整 package 33 tests、包级 TypeScript、模板源码 ESLint、Prettier、四项插件合同检查、隔离 Vite build、diff 与主线门通过；遗留风险为既有已生成插件需手工迁移，Actions 必需检查已通过。

AUD-011 执行结果：提交 `d4691401`，draft PR #49。只修改启动状态 store、监听/bootstrap/Banner 和对应测试；监听注册完成后才发起初始 GET，generation 与活动订阅 token 拒绝事件、卸载/StrictMode 旧订阅后的 GET/retry 覆盖。失败优先逆序回归、6 个相关测试文件 25 tests、根 TypeScript、目标 ESLint/Prettier、Vite production build、diff 检查和 PR 前 `origin/main@fef05dec` 主线门均通过；遗留风险为后端仍无单调 revision，frontend 通过，rust 运行中。

AUD-014 执行结果：只修改 CLI proxy controls hook、Sidebar 和对应两份测试，提交 `dacb7518`，draft PR #50。失败优先确认同一 act 会启动 2 次跨 key IPC，且 prompt/busy/UI 无全局所有权；修复后同步 ref 串行化预检并让冲突 prompt 持锁，取消、确认、无冲突、异常均释放。相关 6 个测试文件 42 tests、TypeScript、目标 ESLint/Prettier、隔离 Vite build、diff 与 `origin/main@fef05dec` 主线门通过；遗留风险为 busy 请求不排队且所有 UI 入口必须继续走统一 hook，frontend/rust Actions 运行中。

本批结果汇总：AUD-009/AUD-011/AUD-014 均在修改前完成当前主线复核和 planned 计划，随后分别以 `c8aeffb5`/`d4691401`/`dacb7518` 建立 draft PR #48/#49/#50；三项共触及 16 个有明确归属的产品/测试/公开文档文件，没有无关重构、依赖升级、主线冲突或搁置项。三项各自的失败优先证据、定向测试、静态检查、隔离构建、差异审查和提交前主线门均已落盘；#48 必需检查通过，#49/#50 等待剩余云端终态。

### 6.5 2026-08-05 高收益边界批次计划

父任务：`.trellis/tasks/08-05-codebase-health-high-impact-guardrails`。本批选择三个经最新主线确认的 P1，并纳入用户新增且已证实的 P2 `AUD-053`；用户确认前不运行 `task.py start`，不建立产品分支/worktree，不修改产品代码。

| 顺序 | Trellis 子任务 | 报告项 | 最小实施边界 | 定向验证与遗留风险 |
| --- | --- | --- | --- | --- |
| 1 | `.trellis/tasks/08-05-upstream-sync-review-gate` | `AUD-001` | 两类上游领先路径统一只创建/更新 open PR；移除目标分支 push、自动 merge 与 mergeability 轮询；新增依赖无关合同检查并接入完整 CI。 | 反例 self-test 锁定 direct push/auto merge 不可回归，覆盖 no-op、可快进、分叉与冲突；YAML、CI scope、Prettier、diff 本地验证。真实权限/ruleset 行为只由 GitHub 验证。 |
| 2 | `.trellis/tasks/08-05-diagnostic-secret-redaction` | `AUD-021` | 建立共享前端 redactor，收口 IPC/Console/全局错误/raw-text adapter；native tracing 前二次清洗，不改原错误 identity、命令结果或依赖。 | 随机 sentinel 覆盖嵌套/循环、clipboard/API-key、Bearer/header、rejection 与 URL query/hash；运行定向 Vitest、TypeScript、ESLint、Prettier、Vite build。无法识别完全无标记的任意自然语言秘密，已知 raw blob 必须 metadata-only。 |
| 3 | `.trellis/tasks/08-05-model-price-alias-read-safety` | `AUD-037` | 编辑 command 使用严格 `read`；adapter 兼容 v1/v2 并统一输出当前 v2；加载失败显示局部错误/重试并阻断所有 mutation；运行时成本计算继续 fail-open。 | 覆盖缺失、损坏、不可读、超限、v1 迁移、v2 默认/保存、零 save、重试恢复与正常编辑；运行定向 Vitest、TypeScript、ESLint、Prettier、Vite build。既有成功写入后删除 `.bak` 的策略不在本批改变。 |
| 4 | `.trellis/tasks/08-05-provider-active-route-default` | `AUD-053` | 当前 CLI 的活动路由查询只初始化草稿一次；Default/模板与实际活动值一致，用户手动选择后不被 refetch 覆盖。 | 覆盖 deferred 初始加载、Default、活动模板、refetch 不抢占、切换 CLI 与零 active-set mutation；运行定向 Vitest、TypeScript、ESLint、Prettier、Vite build。未知活动模板保守显示 Default。 |

四个子任务从实施时最新的 `origin/main` 建立独立 branch/worktree，先写失败优先回归，再做最小修复。每项完成本地验证和五轴差异审查后，可把提交、PR、CI 与合并跟进委派给独立子代理，主线程继续下一项；报告由主线程统一维护，避免并发冲突。

每个 PR 前必须重新 fetch `origin/main`，记录基线与最新 SHA，并核对触及文件及相邻合同在功能目标、实现方式、接口行为和最终效果上的变化。能够兼容时先整合主线并重跑全部定向验证；只能二选一时保留 branch、commit 和 worktree，把任务、基线、冲突文件、不可共存原因、方案影响和建议登记为“待决策”，随后继续其他未受影响任务。只有合并并验证后，报告项才从 `planned` 转为 `resolved`。

### 6.6 2026-08-05 质量门与恢复批次计划

父任务：`.trellis/tasks/08-05-codebase-health-quality-recovery`。本批只选择在 `origin/main@ef41e6da` 仍能直接证明、合同明确且不依赖未决产品决策的四项；`PENDING.md` 当前无未解决条目。

| 顺序 | Trellis 子任务 | 报告项 | 最小实施边界 | 定向验证与遗留风险 |
| --- | --- | --- | --- | --- |
| 1 | `.trellis/tasks/08-05-ci-static-contract-gates` | `AUD-005, AUD-006` | `resolved`：头提交 `400491b8` / ready PR #57 已通过 Actions 并 squash 合并为 `0062c907`，最终合并前基线 `origin/main@db92a480`。Instant checker 已加跨行/注释 self-test 并接入 prepush/support-contract；根脚手架 typecheck 已接入 prepush/plugin-hardening/frontend CI；实际 `CHECKS/STAGES` 用结构化合同防漂移。未改 Rust 或脚手架产品实现。 | 失败优先反例、两轮独立审阅、重放后本地全门及 Actions support-contract/frontend/rust/ci-gate 均通过。遗留风险仅为静态规则未来语法覆盖边界与少量 Node/tsc 时间。 |
| 2 | `.trellis/tasks/08-05-dev-build-executable-artifacts` | `AUD-044` | `resolved`：提交 `2e519e51` / ready PR #59 已通过标准 CI 和四平台 dev-build，并 squash 合并为 `62574e22`；最终合并前基线 `origin/main@5b13683b`。macOS `.app` 用 ditto zip、Linux AppImage 用 tar、Windows 保持 MSI/EXE 布局，统一上传白名单。 | 下载解包后 mode、Mach-O/ELF/PE 架构与 macOS plist 通过；GUI 启动未在自动会话执行，dev bundle 无完整 resource seal，签名治理不在本项。 |
| 3 | `.trellis/tasks/08-05-code-editor-production-load-retry` | `AUD-052` | `resolved`：头提交 `c4f17111` / ready PR #58 已通过 Actions 并 squash 合并为 `5b13683b`，最终合并前基线 `origin/main@0062c907`。失败 Promise 仅由自身清缓存；组件展示稳定局部错误与页面重新加载。 | 25/25 Vitest、TypeScript、ESLint、Prettier、Vite build、diff、桌面/窄视口 Playwright 与 Actions 全部通过。遗留风险是页面 reload 重置未持久化状态。 |

三个子任务分别从实施时最新 `origin/main` 建立 branch/worktree。每项先取得旧实现失败证据，再做最小修复；PR 前重新 fetch、比较相邻实现和最终行为、重跑验证。可兼容漂移先整合；只能二选一时保留全部成果并登记待决策，不覆盖主线。

### 6.7 2026-08-05 启动故障布局恢复批次计划

`PENDING.md` 无未解决条目。本批只选择在 `origin/main@0062c907` 可直接证明、无需产品语义决策且与当前 PR 文件集无交集的 `AUD-015`；认证、插件失败策略、数据生命周期、CAS 与依赖豁免等项目继续保留为 `confirmed`，不在本批顺手处理。

| 顺序 | Trellis 任务 | 报告项 | 最小实施边界 | 定向验证与遗留风险 |
| --- | --- | --- | --- | --- |
| 1 | `.trellis/tasks/08-05-app-layout-startup-banner` | `AUD-015` | `resolved`：候选 head `4d1d720a` / ready PR #60，最终基线 `origin/main@62574e22`；只让 AppLayout main 成为纵向 flex，并用 `min-h-0 flex-1` 包住 Outlet；squash 合并为 `d12dbfe3`。 | 失败优先结构测试、重放后 2 files/21 tests、TypeScript、ESLint、Prettier、Vite build、diff、1024x600 failed/ready 浏览器对照与 Actions 全部通过。风险限于其他非全高 Outlet 的 flex 尺寸响应。 |

实施前从最新 `origin/main` 建立独立 worktree；PR 前再次 fetch 并核对 #58/#59 及任何新主线布局实现。可兼容漂移先整合并重验；根本冲突则保留分支、提交和 worktree，登记待决策后继续其他任务。

### 6.8 2026-08-05 Plugin SDK Host API 合同批次计划

`PENDING.md` 无未解决条目。本批选择在 `origin/main@5b13683b` 可直接证明、属于向后兼容类型补全且不需要产品决策的 `AUD-051`。Host/worker 是现行行为所有者，本批不修改原生执行、授权、storage 持久化或 diagnostics 数据策略。

| 顺序 | Trellis 任务 | 报告项 | 最小实施边界 | 定向验证与遗留风险 |
| --- | --- | --- | --- | --- |
| 1 | `.trellis/tasks/08-05-plugin-sdk-host-api-contract` | `AUD-051` | `resolved`：候选由 `c62c4725` 重放为 `29c2139e`，最终基线 `origin/main@d12dbfe3`；导出 storage、diagnostics 与完整运行报告类型，接入可选 `PluginApi` 命名空间；让 v1 合同和既有 checker 同时约束 SDK、worker 与 Host 的三个现行方法；补 SDK 参考文档。ready PR #61 已 squash 合并为 `ba06dabb`。 | 旧 SDK 的严格类型/合同失败优先，修复后 SDK test/typecheck、插件合同/文档/完成度、脚手架 test/typecheck、根 TypeScript、目标 ESLint/Prettier、Node 语法、Vite、diff 与 Host 删除负例通过；Actions `30993065410` 全绿。未来 Host/报告字段变化会由合同门显式阻断。 |

实施从最新 `origin/main` 建立独立 worktree。#59 只触及 dev-build workflow/checker，#60 只触及 AppLayout，当前没有重复实现或文件冲突；最终 PR 前已再次 fetch 并按功能目标、接口行为和最终效果核对主线漂移，重放后重验且完成合并。根本冲突则保留分支、提交和 worktree 并登记待决策。

### 6.9 2026-08-05 TUI Provider 探测期限合同批次计划

`PENDING.md` 无未解决条目。本批选择在 `origin/main@5b13683b` 可直接证明、接口目标明确且改动可限制在三个 Rust 模块的 `AUD-029`。状态快照继续使用短 client timeout；本批只修复用户显式手动探测的跨层 deadline 错配。

| 顺序 | Trellis 任务 | 报告项 | 最小实施边界 | 定向验证与遗留风险 |
| --- | --- | --- | --- | --- |
| 1 | `.trellis/tasks/08-05-tui-provider-probe-timeout-contract` | `AUD-029` | `resolved`：候选 `13abfea7` / ready PR #62，最终基线 `origin/main@ba06dabb`；squash 合并为 `c2e4db25`。protocol 导出 20 秒共享 deadline；observer 从其派生 route timeout；TUI 的 provider probe request 单独使用 deadline + 1 秒余量，并新增 `Timeout` 显示原因。 | 失败优先源检查、共享期限、request override、504 与真实 body timeout 回归、7/7 源合同、diff 和独立审查通过；Actions `30995513871` 全绿。断连后的服务端 cancellation 不在本批，最坏继续到既有 20 秒上限。 |

实施前从最新主线建立独立 worktree，先落测试再改代码。仓库规则禁止本地 Cargo/rustfmt/Clippy/Rust tests，因此原生编译和测试以 PR Actions 为准；提交 PR 前再次 fetch 并核对 #59/#60/#61 合并后的最新 `origin/main`。兼容漂移先整合并重新验证；根本冲突则保留分支、提交和 worktree，登记待决策后继续其他任务。

### 6.10 2026-08-05 长会话双向窗口批次计划

`PENDING.md` 无未解决条目。本批选择在 `origin/main@62574e22` 仍可确定复现、无需修改后端协议或扩大内存预算、且与 #60/#61/#62 无文件交集的 P1 `AUD-038`。旧报告的反向加载描述已纠正；当前实际问题是顺序加载第 11 页后 page 0 被淘汰，却没有反向取回能力。

| 顺序 | Trellis 任务 | 报告项 | 最小实施边界 | 定向验证与遗留风险 |
| --- | --- | --- | --- | --- |
| 1 | `.trellis/tasks/08-05-session-message-bidirectional-window` | `AUD-038` | `resolved`：候选 `f7d6fc17` / ready PR #63，最终基线 `origin/main@c2e4db25`；squash 合并为 `e57acb54`。在现有 infinite query 增加 previous page param；页面接入双向操作和真实边界。 | 2 个测试文件/28 tests、TypeScript、ESLint、Prettier、Vite、diff 与 1024px 浏览器双向取页/边界/无溢出验证通过；Actions `30997519757` 全绿。窗口十页与 390px 既有侧栏问题保留。 |

本批先写报告与 Trellis 计划，再从实施时最新 `origin/main` 建独立 worktree。PR 前已再次 fetch，并确认 #60/#61 与本项没有功能或文件重叠；兼容漂移无冲突整合到 `ba06dabb` 后已重跑全部验证并创建 #63。#62 合并后仍执行最终主线门；根本冲突则保留分支、提交和 worktree 并登记待决策。

### 6.11 2026-08-05 普通设置共享串行 patch 批次计划

`PENDING.md` 无未解决条目。本批选择在 `origin/main@ba06dabb` 仍有两个同页生产 writer、可用现有 TanStack scope 和 patch helper 局部修复、且优先级为 P1 的 `AUD-012`。当前主线已保护专用设置 owner，本批边界据此从“全设置 CAS”收窄为官方前端普通设置写入。

| 顺序 | Trellis 任务 | 报告项 | 最小实施边界 | 定向验证与遗留风险 |
| --- | --- | --- | --- | --- |
| 1 | `.trellis/tasks/08-05-settings-patch-serialization` | `AUD-012` | `resolved`：候选 `f6d7d2d4` / ready PR #64，最终基线 `origin/main@e57acb54`，squash 合并为 `5c756edc`。普通 settings mutation 共享同一 scope；设置页 runner 仅传本轮 changed-key patch，执行时复用 `createSettingsSetInput` 从最新 cache 构造兼容 payload。只改 7 个计划文件。 | 两独立 hook 的并发失败优先与顺序/合并断言、changed-key、auto-start、失败释放均通过；6 个测试文件/82 tests、TypeScript、ESLint、Prettier、Vite、diff、差异审查及 Actions `30999335471` 全绿。外部/native 普通 writer 的 revision/CAS 仍是后续架构风险。 |

实施前从最新主线建立独立 worktree，并核对 #62/#63 文件集；当前两者分别只改 Observer/TUI Rust 与会话分页文件，没有重复实现或文件冲突。PR 前仍须重新 fetch、整合兼容漂移并重跑全部定向验证；根本冲突则保留候选并登记待决策。

### 6.12 2026-08-05 Observer 快照缓存边界批次计划

`PENDING.md` 无未解决条目。本批在最新 `origin/main@c2e4db25` 重新核验后选择 `AUD-030`：根因仍能由生产代码直接证明，且 #62 已合并、#63/#64 均无 Observer 文件交集。当前缓存最多可积累 510 个合法 key，TTL 只用于命中判断；本批用固定条目上限和访问时清理解决长驻累积，不引入后台任务或协议变化。

| 顺序 | Trellis 任务 | 报告项 | 最小实施边界 | 定向验证与遗留风险 |
| --- | --- | --- | --- | --- |
| 1 | `.trellis/tasks/08-05-observer-snapshot-cache-bounds` | `AUD-030` | `resolved`：当前头 `5f249948` / ready PR #65，最终基线 `origin/main@5c756edc`，squash 合并为 `405a545f`；仅修改 `src-tauri/src/app/observer/mod.rs`，访问时清理过期项，新 key 超过 64 项时淘汰最早创建项，已有 key 替换不额外淘汰。 | Rust 单测覆盖 active/idle 过期删除、容量淘汰、命中与替换；三轮 Actions `31000946748`/`31001414671`/`31001992510`、frontend/rust/合同/ci-gate、6/6 源合同、diff 与独立差异审查均通过。条目上限不等于字节预算，完全空闲最多保留 64 个过期项直到下一次访问。 |

实施前从最新主线建立独立 worktree，并核对 #63/#64 文件集；兼容漂移先整合并重验，根本冲突则保留候选并登记待决策。

### 6.13 2026-08-05 发布签名私钥作用域批次计划

`PENDING.md` 无未解决条目。本批在最新 `origin/main@405a545f` 重新核验后选择 `AUD-043`：根因仍可由 workflow 直接证明，当前没有开放 PR，且刚合并的 #65 只有 Observer 文件。Tauri 已支持私钥文件路径，因此无需升级依赖或改变制品/发布合同。

| 顺序 | Trellis 任务 | 报告项 | 最小实施边界 | 定向验证与遗留风险 |
| --- | --- | --- | --- | --- |
| 1 | `.trellis/tasks/08-05-release-signing-key-scope` | `AUD-043` | `resolved`：`1fcc687d` 只修改 `ci.yml` 并新增 checker/self-test。validation 写 runner-temp 0600 key；Tauri Action step-scoped 接收路径；紧邻 `always()` cleanup；禁止五类跨步骤 command-file；后续步骤不再可达。 | failure-first、10 类负向 mutation、现有 CI/release 合同、Prettier、diff 与 Actions `31005579029` 全绿；PR #66 合并为 `d5c9cfe0`。真实签名 Action/构建进程仍需读取 key；不额外运行已发布版本的 signed candidate。 |

实施前从最新主线建立独立 worktree；PR 前重新 fetch 并核对 main/open PR。兼容漂移先整合并重验，根本冲突则保留分支、提交和 worktree，登记待决策。

### 6.14 2026-08-05 Responses 连续性缓存字节预算批次计划

`PENDING.md` 无未解决条目。本批在最新 `origin/main@d5c9cfe0` 重新核验后选择 `AUD-017`：根因仍可由 `response_cache.rs` 直接证明，#66 只改 CI 三文件，当前无开放 PR或相关主线实现。该项为 P1 且可限制在单一 Rust 文件，不改变协议、请求上限或依赖。

| 顺序 | Trellis 任务 | 报告项 | 最小实施边界 | 定向验证与遗留风险 |
| --- | --- | --- | --- | --- |
| 1 | `.trellis/tasks/08-05-responses-continuity-cache-byte-budget` | `AUD-017` | `resolved`：候选 `4de2889b` / PR #67，最终基线 `origin/main@d5c9cfe0`；squash 合并为 `0854d830`。仅修改 `response_cache.rs`，持久化最终 JSON 字节并实施 1 MiB/32 MiB 载荷预算。 | failure-first、真实预算填充、两轮独立审查及 Actions `31012064253` 的 format/bindings、Clippy、Rust tests、audit、ci-gate 全绿。预算不含 key/HashMap 元数据；`get` 反序列化与 TTL idle 保留风险已记录。 |

实施前从最新主线建立独立 worktree，并核对 AUD-017 相关文件和开放 PR。兼容漂移先整合并重验，根本冲突则保留分支、提交和 worktree，登记待决策。

### 6.15 2026-08-05 插件预览与安装内容绑定批次计划

`PENDING.md` 无未解决条目。本批在最新 `origin/main@d5c9cfe0` 重新核验后选择 `AUD-040`：审核与确认仍只由可变路径关联，安装校验后还会二次读取该路径；当时主线新增和开放 PR #67 均无相关文件或实现。#67 后已合并为 `0854d830`，其唯一变更仍是无交集的响应缓存；AUD-040 候选已重放到该最新主线并进入 PR #68。该项修复目标明确，不依赖 `AUD-026/027/045` 的插件运行时产品合同。

| 顺序 | Trellis 任务 | 报告项 | 最小实施边界 | 定向验证与遗留风险 |
| --- | --- | --- | --- | --- |
| 1 | `.trellis/tasks/08-05-plugin-preview-install-content-binding` | `AUD-040` | `resolved`：候选 `4ce877b6` / Ready PR #68，最终基线 `origin/main@0854d830`；本地安装/更新确认携带预览 SHA-256，安装缓存写已验证字节；squash 合并为 `e94c83bd`。 | failure-first 7 failed / 38 passed；修复后 45/45、TypeScript、目标 ESLint/Prettier、Vite、8/8 源合同通过；Actions `31015354600` 全绿。无一次性 token，重复同版本行为留给 AUD-050。 |

候选从已核对的 `origin/main@d5c9cfe0` 建立独立 worktree；#67 合并后已 fetch 并确认 `d5c9cfe0..0854d830` 仅改无交集的响应缓存，候选无冲突重放。首轮 Actions 的单一 Clippy dead-code 已最小修复，第二轮全绿；最终主线无漂移或竞争实现，PR #68 已 squash 合并为 `e94c83bd`，无待决策冲突。

### 6.16 2026-08-05 图片生成响应下载扇出边界批次计划

`PENDING.md` 无未解决条目。本批在最新 `origin/main@0854d830` 重新核验后选择 `AUD-023`：不可信 `data` 仍可绕过 UI 的 `n=1..10` 约束触发任意数量 URL 下载；Image Gen 路径自 `eeccf64d` 后无主线改动，开放 PR #68 也只有插件文件。该项可用单一前端 adapter 边界和定向测试确定修复，无需先决定认证、插件失败策略或数据生命周期合同。

| 顺序 | Trellis 任务 | 报告项 | 最小实施边界 | 定向验证与遗留风险 |
| --- | --- | --- | --- | --- |
| 1 | `.trellis/tasks/08-05-image-gen-response-fanout-bound` | `AUD-023` | `resolved`：初始候选 `b8303703` 在 #68 合并后无冲突重放为 `f703c863` 到 `origin/main@e94c83bd`；Ready PR #69 只修改 adapter 与其测试，下载前按请求 `n`/硬上限 10 整体拒绝超量 `data`，squash 合并为 `9a280136`。 | failure-first 旧实现 45 tests 中 3 项失败；重放后 Image Gen 8 文件/223 tests、TypeScript、目标 ESLint/Prettier、Vite、diff 与 Actions `31017816818` 全部通过。最坏总下载有界为 320 MiB；跨 IPC 取消和更低聚合字节预算保留。 |

实施前从最新主线建立独立 worktree；PR 前再次 fetch 并核对 Image Gen 文件、主线提交与开放 PR。兼容漂移先整合并重验，根本冲突则保留分支、提交和 worktree，登记待决策。

### 6.17 2026-08-05 前端内存诊断共享扫描预算批次计划

`PENDING.md` 无未解决条目。本批在最新 `origin/main@e94c83bd` 重新核验后选择 `AUD-022`：Console 生产入口仍同步调用逐 query 重置的 200,000 节点估算器，且完整构建/sort 全量诊断；#68 已合并的插件文件和开放 PR #69 的 Image Gen 文件均无重叠。该项为 P2，可用两份前端文件局部限制最坏同步工作量，不需要产品合同决策。

| 顺序 | Trellis 任务 | 报告项 | 最小实施边界 | 定向验证与遗留风险 |
| --- | --- | --- | --- | --- |
| 1 | `.trellis/tasks/08-05-frontend-memory-diagnostics-shared-budget` | `AUD-022` | `resolved`：候选 `35491b78` / Ready PR #70，最终基线 `origin/main@9a280136`；整次快照共享 200,000 节点、最多扫描 2,000 query，返回截断元数据并维护有界稳定 top-20。只改诊断 service/test。 | 两轮 failure-first、宽对象预读修正、10 个 service 文件/52 tests、TypeScript、目标 lint/format、隔离 Vite、diff、两轮独立审查与 Actions `31020581604` 通过；squash 合并为 `5d4906c5`。同步 wall-clock/yield 和 `getAll()` 完整引用数组保留。 |

实施前从最新主线建立独立 worktree；PR 前重新 fetch 并核对诊断文件、#69 终态与其他开放 PR。兼容漂移先整合并重验，根本冲突则保留分支、提交和 worktree，登记待决策。

### 6.18 2026-08-05 Observer OAuth gate 批量查询批次计划

`PENDING.md` 无未解决条目。本批在最新 `origin/main@9a280136` 重新核验后选择 `AUD-031`：候选投影仍在每轮快照对 OAuth Provider 逐项查询 gate；相邻 provider detail 路径已有可复用的去重、512 输入上限、300 SQL 参数分块和同语义 gate 计算。#69 已合并的 Image Gen 文件与 #70 的前端诊断文件均无交集；#70 后续已合并为 `origin/main@5d4906c5`，仍无相关实现。该项为 P2，可用两份 Rust 文件局部消除 N+1，不需要修改协议或产品合同。

| 顺序 | Trellis 任务 | 报告项 | 最小实施边界 | 定向验证与遗留风险 |
| --- | --- | --- | --- | --- |
| 1 | `.trellis/tasks/08-05-observer-oauth-gate-batching` | `AUD-031` | `resolved`：候选 `f92d5190` / Ready PR #71，最终基线 `origin/main@5d4906c5`；OAuth IDs 按共享 512 display 上限分组，复用去重、300 参数 `IN` 查询与 gate 语义，只合并 limited IDs，缺失快照仍 Allow。首轮 CI 的一处 rustfmt 换行漂移已精确应用。 | failure-first、候选合同 4/4、OAuth 源合同 3/3、diff、独立审查与 Actions `31023947314` 全绿；最终门无漂移且 #72 无交集，squash 合并为 `7c395d15`。每 512 IDs 仍有连接/最多两条 SQL，route query 无 SQL LIMIT 保留。 |

实施前从最新主线建立独立 worktree；PR 前重新 fetch 并核对 Observer/OAuth 文件、#70 终态与其他开放 PR。兼容漂移先整合并重验，根本冲突则保留分支、提交和 worktree，登记待决策。

### 6.19 2026-08-06 插件 fail-open header patch 合同批次计划

`PENDING.md` 无未解决条目。本批在最新 `origin/main@5d4906c5` 重新核验后选择 `AUD-032`：公开合同与默认 helper 都是 fail-open，但 request/response header patch 错误仍无条件失败，且逐项直接写共享 headers 会留下部分 mutation。相关 pipeline/docs 自 `7088dcf4` 后没有同类修复，开放 PR #71 仅含 Observer/OAuth 文件。该项为 P2，可用一个 Rust 文件对齐既有 failure-policy 语义，不需要新增产品决策。

| 顺序 | Trellis 任务 | 报告项 | 最小实施边界 | 定向验证与遗留风险 |
| --- | --- | --- | --- | --- |
| 1 | `.trellis/tasks/08-06-plugin-fail-open-header-patch` | `AUD-032` | `resolved`：逻辑提交 `2a1878ba`、云端格式提交 `0a5bd769` / Ready PR #72，最终基线 `origin/main@7c395d15`；`apply_header_patch` 在副本全量校验并一次提交，request/response 报错后 fail-open 丢弃该插件整份 mutation 并继续，fail-closed 保持拒绝。只改 `pipeline.rs`。 | failure-first 3 项失败；修复及 artifact 后 7/7、diff、完整差异、范围与 Actions `31026666018` 全绿；最终门确认 #73 无交集，squash 合并为 `d26524f2`。plugin 级 circuit 与 AUD-026 保留。 |

实施前从最新主线建立独立 worktree；PR 前重新 fetch 并核对 plugin pipeline、公开合同、#71 终态与其他开放 PR。兼容漂移先整合并重验，根本冲突则保留分支、提交和 worktree，登记待决策。

### 6.20 2026-08-06 Extension Host 主动空闲回收批次计划

`PENDING.md` 无未解决条目。本批在最新 `origin/main@d26524f2` 重新核验后选择 `AUD-046`：command 与 gateway 使用两个独立 registry，但两个构造点都没有生产 idle timer；30 秒 child recycle 未向 registry 转发，120 秒 registry dispose 只在后续调用或测试触发。#72 只改已合并的 plugin pipeline，开放 #73 不含三个目标文件。该项为 P2，可在既有生命周期边界内兑现已经声明的资源回收合同，不需要修改产品协议或新增决策。

| 顺序 | Trellis 任务 | 报告项 | 最小实施边界 | 定向验证与遗留风险 |
| --- | --- | --- | --- | --- |
| 1 | `.trellis/tasks/08-06-extension-host-idle-sweeper` | `AUD-046` | `resolved`：逻辑提交 `139c8432`、云端格式提交 `a50ec5be` / Ready PR #74 严格修改 `extension_host.rs`、`extension_host_registry.rs`、`runtime_executor.rs`；shared 构造器在 command/gateway registry 创建时启动单一弱引用 5 秒 sweeper，转发现有 child recycle，并以写屏障调用 registry dispose。 | failure-first 0/7、修复后及 artifact 后 7/7、diff、三文件范围、差异审查与 Actions `31030917177` 全绿；最终主线门通过，squash 合并为 `94da784b`。5 秒粒度、空 registry 周期唤醒和 child 错误的 120 秒兜底保留。 |

实施前从该主线建立独立 worktree；PR 前重新 fetch 并核对 Extension Host 三文件、#73 终态与其他开放 PR。兼容漂移先整合并重验，根本冲突则保留分支、提交和 worktree，登记待决策。

### 6.21 2026-08-06 Homebrew Cask 正式资产合同批次计划

`PENDING.md` 无未解决条目。本批在最新 `origin/main@94da784b` 重新核验后选择 `AUD-003`：正式支持矩阵、CI 候选、Release 校验和 README 均只把 macOS Apple Silicon 纳入正式桌面发布；遗留 Cask 生成器/自测仍要求 Intel SHA，文档仍描述不存在的自动 tap 同步。#73 不含三个目标文件。该项为 P2，可仅收敛发布辅助合同与文档，不修改实际 Release workflow、资产、版本或依赖。

| 顺序 | Trellis 任务 | 报告项 | 最小实施边界 | 定向验证与遗留风险 |
| --- | --- | --- | --- | --- |
| 1 | `.trellis/tasks/08-06-homebrew-cask-release-contract` | `AUD-003` | `resolved`：候选 `007e612d` / Ready PR #75 只改 `support-matrix.mjs`、Homebrew self-test 与 `docs/release-homebrew.md`；Cask target 从现有 macOS ARM release target 派生，生成 ARM-only SHA/精确 ZIP URL/`depends_on arch: :arm64`，拒绝遗留 Intel 参数；文档改为真实手动发布后流程。 | failure-first 旧实现 1/5、修复后 5/5；本地合同复验与 Actions `31036360425` 全绿。最终基线 `2a79978c` 无漂移，PR #75 squash 合并为 `ff09a81a`；tap 仍需人工更新，未发布测试 tag 不做联网 HEAD。 |

实施前将只读 worktree 切换为 `codex/audit-homebrew-cask-release-contract`；PR 前重新 fetch 并核对支持矩阵、Cask、Release workflow、#73 终态与其他开放 PR。兼容漂移先整合并重验，根本冲突则保留分支、提交和 worktree，登记待决策。

### 6.22 2026-08-06 FormField 标签关联合同批次计划

`PENDING.md` 无未解决条目。本批在最新 `origin/main@94da784b` 重新核验后选择 `AUD-039`：FormField 仍允许直接 child 却不给真实控件注入 id/hintId，当前生产 34 个该路径中有 25 个字段具备唯一主控件，9 个是真正复合控件。#73/#75 不含目标文件或同功能实现。该项为 P2，修复价值明确，可在前端类型、语义和调用迁移内闭环，不依赖产品策略或后端合同。

| 顺序 | Trellis 任务 | 报告项 | 最小实施边界 | 定向验证与遗留风险 |
| --- | --- | --- | --- | --- |
| 1 | `.trellis/tasks/08-06-form-field-label-contract` | `AUD-039` | `resolved`：14 文件候选 `90230c56` / Ready PR #76 在 `origin/main@ff09a81a` 使用可判别 control/group 模式；25 个唯一主控件字段迁移 render prop，9 个复合控件声明 group。不改字段业务或视觉 class；squash 合并为 `9e83772c`。 | failure-first 旧实现 1 failed / 6 passed；重放后 143/143 定向、2814/2814 全量前端单测、TypeScript、ESLint/Prettier、Vite build、60 调用 AST（0 invalid）、diff 与 Actions `31039483396` 全过。合并后 14 文件树与候选一致；真实 VoiceOver/NVDA 未人工执行。 |

实施前将只读 worktree 切换为 `codex/audit-form-field-label-contract`；PR 前重新 fetch 并核对 FormField、十个调用文件、#73/#75 终态与其他开放 PR。兼容漂移先整合并重验，根本冲突则保留分支、提交和 worktree，登记待决策。

### 6.23 2026-08-06 Provider 自环发送防护批次计划

`PENDING.md` 无未解决条目。本批在最新 `origin/main@ff09a81a` 重新核验后选择 `AUD-025`：Provider 保存仍只校验 HTTP(S)，生产发送路径从不写入 recursion header，自指 base URL 可反复回入当前网关。现有 `GatewaySelfCheckContext` 已随真实监听端口同步 loopback、LAN/custom host 与本机地址；开放 #76 仅改 FormField 前端文件。该项为 P2，可复用既有自检并在发送边界局部拒绝，无需产品策略、schema 或依赖变更。

| 顺序 | Trellis 任务 | 报告项 | 最小实施边界 | 定向验证与遗留风险 |
| --- | --- | --- | --- | --- |
| 1 | `.trellis/tasks/08-06-provider-self-loop-guard` | `AUD-025` | `resolved`：两文件最终候选 `b7f5378c` / Ready PR #78 在 `origin/main@60b12aa4` 复用 runtime self-check；完整 target URL 在 fingerprint/发送前验证，self target 沿既有 URL failure 切换 Provider。补齐本机 DNS/hosts 别名、解析期限与有界短期缓存；不复活 header、不改 schema、路由选择或 attempt budget。Squash 合并为 `ecd82606`。 | failure-first/修复回归覆盖 loopback、大小写、LAN/custom host、IPv6、DNS alias、尾随路径、默认端口、不同端口和外部 host；diff/范围/发送顺序/锁合同与 Actions `31047727848` 全过，合并树一致。遗留恶意 DNS 重绑定窗口、配置保存期提示和专用错误码。 |

实施前从该主线建立独立 worktree；PR 前和合并前重新 fetch 并核对两个目标文件、#76 终态和其他开放 PR。兼容漂移先整合并重验，根本冲突则保留分支、提交和 worktree，登记待决策。

### 6.24 2026-08-06 插件版本不可变性批次计划

`PENDING.md` 无未解决条目。本批在最新 `origin/main@ecd82606` 重新核验后选择 `AUD-050`：预览 checksum 已阻止确认前换包，但 install/update 仍会删除已记录版本目录，随后由 `INSERT OR IGNORE` 静默保留旧快照。主线在 PR #68 后没有相关提交，远端无开放 PR。该项为 P2，触发路径明确，可在两份 Rust 文件内闭环；为避免撤销当前用户导入全新版本的能力，本批只锁定“已记录版本不可再次写入”，不扩大为所有已有 ID 都必须走当前并不普遍可达的专用更新 UI。

| 顺序 | Trellis 任务 | 报告项 | 最小实施边界 | 定向验证与遗留风险 |
| --- | --- | --- | --- | --- |
| 1 | `.trellis/tasks/08-06-plugin-version-immutability` | `AUD-050` | `resolved`：最终候选 `e89efd3c` / Ready PR #79 只改 `plugin_service.rs` 与 `repository.rs`，增加当前/历史版本存在查询、preview/execution 双重 blocker、官方 install 物化前阻断、单进程包变更锁和只允许新目录的 promotion；历史切换走 rollback。Squash 合并为 `cab1229a`。 | 12/12 源合同、diff、精确范围、两轮独立审查与 Actions `31052820007` 全过；合并后两文件 blob 一致。遗留既有坏快照不自动修复，进程崩溃的跨 FS/DB 瞬时窗口与跨进程外部篡改不在本批。 |

实施前已从 `origin/main@ecd82606` 建立 `codex/audit-plugin-version-immutability` 独立 worktree；PR 前和合并前重新 fetch 并核对两目标文件、插件导入/更新调用合同和其他开放 PR。兼容漂移先整合并重验，根本冲突则保留分支、提交和 worktree，登记待决策。

### 6.25 2026-08-06 RustSec 审计豁免移除批次计划

`PENDING.md` 无未解决条目。本批在最新 `origin/main@ecd82606` 重新核验后选择 `AUD-007`：报告索引的 `confirmed` 与当前代码一致，明细旧 `resolved` 没有任何交付证据，已纠正。两处全局配置仍共同忽略 `RUSTSEC-2026-0194/0195`，锁文件仍为受影响的 `quick-xml 0.39.2`。当前兼容的两个直接传递包版本已具备修复依赖，因此无需延续风险接受，也无需修改 manifest 或产品代码。唯一开放 PR #79 修改插件服务与 repository，无文件、目标或效果重叠。

| 顺序 | Trellis 任务 | 报告项 | 最小实施边界 | 定向验证与遗留风险 |
| --- | --- | --- | --- | --- |
| 1 | `.trellis/tasks/08-06-rustsec-exception-removal` | `AUD-007` | `resolved`：云端 artifact 只更新三个预期锁项，临时 update 已移除；最终候选 `2f499dd0` 仅为 plain audit、删除 `audit.toml` 和修复后的 `Cargo.lock`。Ready PR #80 squash 合并为 `b0698f57`。 | failure-first、最终源合同、YAML、精确三路径、diff、独立审查及 Actions `31054953851` 全过；合并前主线门与合并后目标树验证通过。遗留 XML 运行时可达性研究仍属 `HYP-001`，本批不建立自动依赖升级机制。 |

实施前已从 `origin/main@ecd82606` 建立 `codex/audit-rustsec-exception-removal` 独立 worktree。PR 前和合并前重新 fetch 并核对 workflow、审计配置、锁文件、#79 终态和其他开放 PR；兼容漂移先整合并重验，根本冲突则保留分支、提交和 worktree，登记待决策。

### 6.26 2026-08-06 插件 fail-closed 日志持久化批次计划

`PENDING.md` 无未解决条目。本批最初在 `origin/main@cab1229a` 重新核验后选择 `AUD-026`：官方 Privacy Filter 已声明 `log.beforePersist: fail-closed`，但 executor error、非法最终 payload 和自身/其他 hook 打开的 circuit 都能让原 request log 入库；未知 policy 也会静默变成 fail-open。现有 native/frontend/plugin redactor 均不能在该失败路径中无条件覆盖所有 request-log 字段，因此本批不伪造 host-redaction 保证，而以不落原 request log 作为可证明的封闭终态。当时唯一开放 PR #80 只改 CI/audit/lock；其合并为 `b0698f57` 后仍零文件、目标或效果重叠，远端已无开放 PR。

| 顺序 | Trellis 任务 | 报告项 | 最小实施边界 | 定向验证与遗留风险 |
| --- | --- | --- | --- | --- |
| 1 | `.trellis/tasks/08-06-plugin-fail-closed-persistence` | `AUD-026` | `resolved`：候选 `27213728` 基于 `origin/main@b0698f57`，pipeline circuit 改为 `(plugin_id, hook)`，快照身份阻止替换前在途 hook 回写；四类 fail-closed circuit-open 返回封闭错误，log error/circuit/invalid payload 在 channel/write-through 前停止 request log 并保留 diagnostics；Rust/SDK 严格校验 policy，同步七份直接合同文档。严格 12 文件，Ready PR #81 squash 合并为 `871b84dc`。 | failure-first、SDK 30 tests、两包 TypeScript、plugin docs/API contract、七文档 Prettier、静态源合同、精确范围、diff 和独立审查通过；首轮 CI 仅 rustfmt drift，精确 artifact 后 Actions `31060862654` 全绿。遗留故障期 request-history 缺口；metadata tombstone 需另行定义。timeout ceiling、排队/冷启动/流总预算明确留在 AUD-045。 |

实施前已从 `origin/main@cab1229a` 建立 `codex/audit-plugin-fail-closed-persistence` 独立 worktree。PR 前和合并前重新 fetch 并核对 pipeline/logging/manifest/SDK/docs、#80 终态和其他开放 PR；兼容漂移先整合并重验，根本冲突则保留分支、提交和 worktree，登记待决策。

### 6.27 2026-08-06 插件上下文字段大小写兼容批次计划

`PENDING.md` 无未解决条目。本批在 `origin/main@b0698f57` 重新核验后选择 `AUD-010`：公开 Plugin API v1 与 SDK 均声明 camelCase，但 Registry 仍把 Rust snake_case 子 context 原样序列化；同时现有真实 Extension Host fixture 已依赖 `body_truncated`，不能直接破坏式改名。PR #81 不包含 context/worker 修复但共享四个 SDK/合同文件；其合并为 `871b84dc` 后已核对最新内容，failure-policy/log-persistence 变更与本项可兼容。

| 顺序 | Trellis 任务 | 报告项 | 最小实施边界 | 定向验证与遗留风险 |
| --- | --- | --- | --- | --- |
| 1 | `.trellis/tasks/08-06-plugin-context-casing-compatibility` | `AUD-010` | `resolved`：八文件边界；wire context 只输出 canonical camelCase，内部 root 调度字段不序列化；worker 在 parse 后提供同 JS value 的 v1 snake_case alias；SDK/Rust/JSON/docs 补齐 truncation flags。候选 `1ff29726` 经云端格式提交为 `8d5ef669`；Ready PR #82 squash 合并为 `e6cf04d3`。 | 全字段 JSON golden、真实 Extension Host canonical/legacy 同值、SDK/plugin contract/Prettier/静态 alias、精确八文件与 diff 通过；Actions `31063487534` 全绿，合并后八文件树与候选一致。v1 alias 仍是运行时兼容表面；AUD-027 的 QuickJS heap/body budget 不在本批，但禁止双份 wire payload 以免放大它。 |

已在 #81 合并后重新 fetch，并逐项核对 SDK、contract、hook docs、context 和 worker 的最新内容；没有主线重复实现或语义冲突。后续 PR 前与合并前仍按最新主线门处理兼容漂移；若出现无法与 alias 方案共存的 context 版本策略，则保留计划/候选并登记待决策，不覆盖主线。

### 6.28 2026-08-06 插件 QuickJS 上下文容量边界批次计划

`PENDING.md` 无未解决条目。本批在最新 `origin/main@e6cf04d3` 重新核验后选择 `AUD-027`：插件可见与 mutation body 预算仍继承网关 128 至 500 MiB 上限，Extension Host QuickJS heap 固定 32 MiB且需经历多份 JSON 表示；官方 Privacy Filter 的安全 hook 为 fail-closed。Ready PR #82 已合并，远端无开放 PR或同功能实现。

| 顺序 | Trellis 任务 | 报告项 | 最小实施边界 | 定向验证与遗留风险 |
| --- | --- | --- | --- | --- |
| 1 | `.trellis/tasks/08-06-plugin-context-quickjs-budget` | `AUD-027` | `resolved`：逻辑候选 `cc8ab625`、云端格式 head `a891a038`；七文件内实现独立 1 MiB 可见/输出 body 预算，四类直接内容的 fail-closed 截断在 Worker 前以 `PLUGIN_CONTEXT_TRUNCATED` 拒绝并记录 budget report，不开 circuit；Ready PR #83 squash 合并为 `4ee5faa8`。 | plugin-hardening、SDK 30 tests、两包 TypeScript、文档/Prettier、静态合同、diff 与两轮审查通过；Actions `31069274373` 全绿，最终主线无漂移，合并后七文件树一致。完整大 body 脱敏仍需后续 Rust/流式方案。 |

实施前只更新本报告和 Trellis 计划；随后从 `origin/main@e6cf04d3` 创建独立 `codex/audit-plugin-context-quickjs-budget` worktree。PR 前和合并前重新 fetch 并核对 context/mutation/pipeline/runtime/docs 与所有开放 PR；兼容漂移先整合并重验，根本冲突则保留分支、提交和 worktree，登记待决策。

### 6.29 2026-08-06 插件配置与 Storage 原子合并批次计划

`PENDING.md` 无未解决条目。本批在真正的最新 `origin/main@e6cf04d3` 重新核验后选择 `AUD-034`：runtime storage 与用户配置仍是两个无事务所有权的整份 JSON 写者，保存表单在 pending 期间也仍可编辑。Ready PR #83 的七文件与本批零重叠，没有同功能实现。

| 顺序 | Trellis 任务 | 报告项 | 最小实施边界 | 定向验证与遗留风险 |
| --- | --- | --- | --- | --- |
| 1 | `.trellis/tasks/08-06-plugin-config-storage-atomicity` | `AUD-034` | `resolved`：最终候选 `c669b522` / Ready PR #84 / squash merge `4800bc87`；五文件内新增 SQLite `IMMEDIATE` runtime/config 入口与 transaction helper，runtime storage 原子更新保留域，UI/官方默认配置/update/rollback 保存均保留最新 storage，pending 表单整体禁用。 | 两种写入顺序、官方更新、local update/rollback、64 KiB 超限零提交与 pending 全控件回归已写入；11 项前端测试、TypeScript、ESLint/Prettier、Vite build、静态范围/diff 及 Actions `31073434744` 全绿。直接多 IPC 用户配置并发仍 last-write-wins。 |

实施前从最新主线创建独立 `codex/audit-plugin-config-storage-atomicity` worktree。PR 前和合并前重新 fetch 并核对五个目标文件、#83 终态及所有开放 PR；兼容漂移先整合并重验，根本冲突则保留候选并登记待决策。

### 6.30 2026-08-06 插件 hook 单次调用绝对截止时间批次计划

`PENDING.md` 无未解决条目。本批在 `origin/main@4ee5faa8` 重新核验剩余 6 个 confirmed 项后选择 AUD045 的无歧义子范围：单次 gateway hook 的 timeout 仍不覆盖 gate、单插件锁、cold start、handshake/module load 与 activation。AUD002/008/016/033/035 分别依赖恢复 journal、reset 生命周期、LAN 认证、quarantine/activation 或 observer 摘要投影合同，不进入本批。Ready PR #84 只在 `extension_host.rs` 有相邻 storage 改动，必须等其合并后从最新主线建立候选并复核整合。

| 顺序 | Trellis 任务 | 报告项 | 最小实施边界 | 定向验证与遗留风险 |
| --- | --- | --- | --- | --- |
| 1 | `.trellis/tasks/08-06-plugin-hook-invocation-deadline` | `AUD-045` | `resolved`：逻辑候选 `1417c045` / 云端格式 head `27efd051` / Ready PR #85；四文件内建立单一 Tokio absolute deadline，覆盖 gate/lock/清理/factory/activation/RPC；warm/cold/LRU timeout 按实例身份摘除并强制终止 child，错误稳定为 `PLUGIN_EXTENSION_HOST_TIMEOUT`。 | gate/queue/cold factory/cold execution/warm/LRU/真实 activation 回归已写入；plugin hardening、40 错误码合同、源合同、四文件范围、diff 与三轮审查通过。Actions `31077183327` 全绿，PR #85 squash 合并为 `735cec12`，合并后四文件与候选一致。queue 容量和 stream 累计预算仍需独立产品策略。 |

实施前只更新本报告与 Trellis 计划。PR #84 已在 `4800bc87` 合并；现已重新 fetch `origin/main`、核对四个目标文件、所有开放 PR 和同功能实现并建立候选 worktree。兼容漂移先整合并重验，根本冲突则保留候选、提交和 worktree 并登记待决策。

### 6.31 2026-08-06 最终修复与本地零产物批次计划

本批以已核验的 `origin/main@735cec12` 为唯一候选基线。`PENDING.md` 已新增 `AIO-PENDING-016` 至 `AIO-PENDING-023`，八项均为 `planned`；父任务为 `.trellis/tasks/08-06-final-hardening-zero-artifact`，每项使用独立短分支和独立 PR。首个 AUD-054 PR 同时纳入本报告、PENDING 和本轮 Trellis 任务；第八项合并后另建一个纯文档 PR，记录全部 PR、提交和 CI 证据，再把条目迁入 `PENDING_COMPLETED.md`。

| 顺序 | Trellis 任务 / 分支 | 报告项 | 实施边界 | 云端门与交接 |
| --- | --- | --- | --- | --- |
| 1 | `.trellis/tasks/08-06-cloud-only-zero-artifact-contract` / `codex/cloud-only-zero-artifact-contract` | `AUD-054` | 规则、README、活跃 spec/template、package/workspace 脚本和零依赖合同；不改历史归档。 | 本地只做 Node 源码合同/解析与 `git diff --check`；精确 `ci.yml` workflow_dispatch 全量验证，合并后核验并清理仓库级已知产物。 |
| 2 | `.trellis/tasks/08-06-provider-sync-session-snapshot` / `codex/provider-sync-session-snapshot` | `AUD-055` | sessions-only v2 manifest、v1 managed 迁移、单代 backup、回滚和非受管保护。 | 云端 Rust tests/format/bindings/Clippy；下一候选先补 AUD-054 的 PR/CI/提交证据。 |
| 3 | `.trellis/tasks/08-06-request-runtime-log-retention` / `codex/request-runtime-log-retention` | `AUD-056` | 7 天请求/运行日志、ledger 永久保留、256 MiB 软上限、freelist 可见、不自动 VACUUM。 | 云端完整跨层门；下一候选补 AUD-055 证据。 |
| 4 | `.trellis/tasks/08-06-gateway-lan-bearer-token` / `codex/gateway-lan-bearer-token` | `AUD-016` | 真实 peer 的全路由 token、一次性展示/轮换、header 脱敏、WSL 同步和 provider 专用路径移除。 | 云端 socket/路由/日志/WSL 覆盖；下一候选补 AUD-056 证据。 |
| 5 | `.trellis/tasks/08-06-cross-restart-data-reset` / `codex/cross-restart-data-reset` | `AUD-008` | durable marker、专用退出、启动前清理、失败 maintenance retry/exit gate。 | 云端跨平台生命周期门；为 AUD-002 提供共享 coordinator。 |
| 6 | `.trellis/tasks/08-06-filesystem-recovery-journal` / `codex/filesystem-recovery-journal` | `AUD-002` | prepare-first journal、SQLite 权威 replay、Skills 受管 artifact、补偿错误聚合。 | 云端故障注入/重启/脱敏/并发门；下一候选补 AUD-008 证据。 |
| 7 | `.trellis/tasks/08-06-observer-zero-history-query` / `codex/observer-zero-history-query` | `AUD-035` | last/dominant/recent 受限查询和 source-aware 有界 folder cache，保持摘要语义。 | 云端查询 spy/缓存/协议回归；下一候选补 AUD-002 证据。 |
| 8 | `.trellis/tasks/08-06-plugin-activation-quarantine` / `codex/plugin-activation-quarantine` | `AUD-033` | 精确 activation policy、startup/command/hook gate、600 秒三次严重故障 quarantine、revalidate。 | 云端跨重启/并发/fail-open-close/snapshot 门；合并后只开纯文档收口 PR。 |

本地不运行 `pnpm`、Cargo、Tauri、安装器、dev server、类型检查、Lint、测试、构建或任何间接产物命令；不删除全局 `~/.cargo`、pnpm store 或其他项目文件。每次 PR 创建/合并前重新 fetch `origin/main`，检查功能、接口、实现和效果重叠；兼容漂移先重放并重验，根本冲突保留候选并登记待决策。保留现有 `workflow_dispatch` 全量 CI 与按需 `dev-build`，不把桌面打包升级为每个 PR 必需任务。

## 7. 未覆盖区域与验证盲点

- Rust 原生格式、Clippy、测试、依赖审计、绑定生成与桌面打包只能由 GitHub Actions 验证，本地审计不会运行。
- 审计期并行提交 `9d1fb966..a322ba15` 触及 31 个 TUI、托盘、resident、配置、入口、样式、测试和 Trellis/PENDING 文件；这些文件只以起始基线形成结论。对应 PR 合并/发布后应在稳定 SHA 重新跑 TUI/Observer、tray mini、provider probe 和 macOS resident 回归审计。
- 本机没有可用 `pnpm`，验证直接调用仓库现有 `node_modules/.bin` 工具；后续前端修复均把 Vite 产物写入 `/private/tmp` 隔离目录，仍未运行 Tauri/native build 或覆盖率采集。
- 原始审计阶段未推送分支或执行 Actions；后续修复已推送 draft PR #40 至 #50 并记录对应 Actions 终态/运行态，但没有创建 tag/Release 或下载跨平台制品，ruleset、environment 和远端治理配置仍可能变化。
- 未做真实 LAN 攻击、外部 OAuth 双账户、插件恶意包竞争、慢 SSE/高并发、磁盘故障、超大数据库或长期内存 soak；相应结论来自完整静态调用链，影响上限仍需隔离环境验证。
- macOS/Windows/Linux 的签名、权限、通知、托盘与更新器行为未在对应平台制品上运行；平台身份签名保留在 `HYP-002`。
- 未进行 git 历史级死代码演化、全量 bundle composition 或生产遥测分析；静态导入成本与外部依赖真实可达性分别保留在 `HYP-003` 和 `HYP-001`。

## 8. 验证记录

| 时间 | 命令/方法 | 结果 | 说明 |
| --- | --- | --- | --- |
| 2026-08-06 | `origin/main@735cec12`、`PENDING.md`、审计索引/明细与八项只读调用链复核 | planned | 基线纠正为 48 resolved / 5 confirmed；加入 AUD-054/055/056 后总计 56 项，八项全部转 planned。已建立父任务与 8 个顺序子任务、隔离候选 `/private/tmp/aio-final-hardening`；尚未修改产品代码、提交、推送或创建 PR。 |
| 2026-08-06 | AUD-002、AUD-035、AUD-033 并行只读探索与 Trellis 计划补齐 | 通过 | 分别确认 recovery journal 的 Skills artifact 边界、zero-history 摘要/查询拆分和 activation/quarantine 事件/状态模型；结果已写入三个 PRD、design、implement、source/check manifests。未运行 Cargo、pnpm、Tauri、测试或构建。 |
| 2026-08-06 | 本地零产物合同范围审查 | planned | 仅计划 Node AST/源码合同、JSON/Markdown 结构检查与 `git diff --check`；完整前端、Rust、bindings、Clippy、tests、audit 和制品交 GitHub Actions，待 AUD-054 实施后执行。 |
| 2026-08-06 | AUD-054 零产物合同、独立差异审查与本地允许验证 | 本地允许阶段通过；Actions 待执行 | 新 checker/self-test 覆盖受控 scripts、README、活跃 Trellis 指引、真实 CI `run:` steps、dev-build manual-only、candidate main-only 与非主线 skipped 结构；反例含注释、`with.run`/`env.run` 死文本、条件放宽和全部 frontend/Rust 门缺失。已通过 Node syntax、checker/self-test、CI gate checker/self-test、spec links、相关 JSON 解析和 `git diff --check`；未运行 pnpm、Cargo、Tauri、格式化、测试或构建。 |
| 2026-08-06 | AUD-045 云端格式修正、最终 Actions、合并前最新主线门、PR #85 squash 合并与合并后树验证 | 通过并合并 PR #85 | 首轮 run `31076737365` 只在 generated-file drift 门失败；artifact `0408082c...18c9` 严格重排 registry/runtime_executor 两文件并原样提交为 `27efd051`。第二轮 run `31077183327` 全绿。合并前 `origin/main`、base/merge-base 均为 `4800bc87`，只有自身 PR 开放且 CLEAN/MERGEABLE；squash 合并为 `735cec12`，合并后四个目标文件与候选一致，远端无开放 PR。 |
| 2026-08-06 | AUD-045 failure-first、四文件实施、三轮差异审查、本地允许验证、PR 前最新主线门与 Ready PR #85 | 本地允许阶段通过；Actions 首轮只报格式漂移 | 逻辑候选 `1417c045` 基于 `origin/main@4800bc87`。gate/queue/warm/cold factory/cold execution/LRU/真实 activation recovery 回归已写入；plugin hardening（SDK 30 tests 等）、40 错误码合同、deadline/abort/LRU 源合同、精确范围与 diff 通过。审查修复 LRU cleanup timeout 残留和测试上界；PR 前无主线漂移、开放 PR、重复实现或冲突，本机未运行 native。 |
| 2026-08-06 | AUD-034 云端格式/Clippy 修正、最终 Actions、合并前最新主线门、PR #84 squash 合并与合并后树验证 | 通过并合并 PR #84 | 云端 rustfmt artifact 提交 `05d317fe`，仅测试可达 helper 的 `#[cfg(test)]` 修正形成最终候选 `c669b522`；Actions `31073434744` 全绿。合并前 `origin/main`、base/merge-base 均为 `4ee5faa8`，只有自身 PR 开放且 CLEAN/MERGEABLE；squash 合并为 `4800bc87`，合并后五个目标文件与候选一致，远端无开放 PR。 |
| 2026-08-06 | 剩余 6 个 confirmed 项最新主线复核、`PENDING.md`、AUD045 deadline/cancellation 调用链与下一批计划 | planned | `origin/main@4800bc87`：AUD002/008/016/033/035 根因仍成立但无无歧义窄修；AUD045 可先闭合单次 invocation deadline。已建立四文件 Trellis 计划，明确 warm abort/remove、gate/queue/cold/activation 验证以及 queue/stream 残留范围；候选 `/private/tmp/aio-aud045` 与该 SHA 对齐，尚未修改 AUD045 产品代码。 |
| 2026-08-06 | AUD-034 failure-first、五文件实施、独立审查、重放最新主线、本地允许验证与 Ready PR #84 | 本地允许阶段通过；Actions 运行中 | 候选 `fed31c67` 基于 `origin/main@4ee5faa8`。目标组件 11 tests、TypeScript、目标 ESLint/Prettier、Vite build、静态 transaction/call-site 合同、精确范围与 diff 通过；审查补齐官方默认配置、local update/rollback 的 storage 保留。PR 前最后 fetch 无漂移且无其他开放 PR；本机未运行 native。 |
| 2026-08-06 | AUD-027 第二轮 Actions、合并前最新主线门、PR #83 squash 合并与合并后树验证 | 通过并合并 PR #83 | Actions `31069274373` 的 frontend、Rust、docs/support contract、change-scope、pr-title 与 `ci-gate` 全绿。合并前 `origin/main`、base 与 merge-base 均为 `e6cf04d3`，无新增主线漂移且 PR CLEAN/MERGEABLE；squash 合并为 `4ee5faa8`，合并后七个目标文件树与候选一致。 |
| 2026-08-06 | AUD-027 首轮 Actions 失败归因、云端 rustfmt artifact、复验、提交与 PR #83 更新 | 本地允许阶段通过；第二轮 Actions 运行中 | run `31068400547` 的 frontend、docs/support、范围与标题检查通过；Rust 只在 generated-file drift 门失败。artifact `c5b37051...aac9` 只含 `runtime_executor.rs`、`context.rs`、`mutation.rs`、`pipeline.rs` 排版，原样提交为 `a891a038`；plugin-hardening、SDK 30 tests、两包 TypeScript、plugin docs、Prettier、静态路径计数与 diff 重验通过。推送前 `origin/main`/merge-base 仍为 `e6cf04d3`，仅 #83 开放。 |
| 2026-08-06 | AUD-034 `PENDING.md`、真正最新主线 repository/storage/service/form、历史源码偏差与 PR #83 文件集复核 | planned | 报告 worktree 的 tracked source 为历史 `86a30710`，已切换到 `origin/main@e6cf04d3` 候选 worktree重新点验；根因仍成立。已锁定五文件 `IMMEDIATE` 原子合并 + pending 表单冻结计划，#83 零重叠，尚未修改 AUD-034 产品代码。 |
| 2026-08-06 | AUD-027 实施、两轮审查、本地允许验证、两次 PR 前主线门与 Ready PR #83 | 本地允许阶段通过；Actions 运行中 | 候选 `cc8ab625` 严格七文件；审查发现并修复非法 UTF-8 replacement expansion 绕过，补齐 request/response/stream 精确 cap 和零预算回归。plugin-hardening、SDK 30 tests、两包 TypeScript、plugin docs、Prettier、静态路径/源合同、JSON 与 diff 全过；两次 fetch 均确认 `origin/main`/merge-base 为 `e6cf04d3`，无开放 PR 或相关漂移。 |
| 2026-08-06 | AUD-027 最新主线、`PENDING.md`、网关/context/mutation/Worker/pipeline/官方 Privacy Filter、测试与开放 PR 复核 | planned | `origin/main@e6cf04d3` 根因完整成立，远端无开放 PR；已写入 6.28，锁定独立 1 MiB body budget、fail-closed Worker 前拒绝、七文件范围及真实 Worker/四类管线验证，尚未修改产品代码。 |
| 2026-08-06 | AUD-010 第二轮 Actions、合并前最新主线门、PR #82 squash 合并与合并后树验证 | 通过并合并 PR #82 | 云端 rustfmt artifact 形成 `8d5ef669`；Actions `31063487534` 的 change-scope、pr-title、docs/support contract、frontend、Rust（格式/绑定、Clippy、Rust tests、依赖审计）和 `ci-gate` 全绿。合并前 `origin/main`、base/merge-base 均为 `871b84dc`，仅自身 PR 开放且 CLEAN/MERGEABLE；squash 合并为 `e6cf04d3`，合并后八个目标文件树与候选一致。 |
| 2026-08-06 | AUD-010 实施、本地允许验证、PR 前主线门与 Ready PR #82 | 本地允许阶段通过；Actions 等待终态 | 候选 `1ff29726` 严格 8 文件；全字段 serialization golden、真实 Extension Host canonical/legacy alias、SDK 30 tests、两包 TypeScript、插件合同/文档、目标 Prettier、alias 静态合同、范围和 diff 均通过。PR 前 `origin/main`/base/merge-base 均为 `871b84dc`，无开放 PR 或路径/功能重叠。 |
| 2026-08-06 | AUD-026 第二轮 Actions、合并前最新主线门、PR #81 squash 合并与 AUD-010 基线切换 | 通过并合并 PR #81 | Actions `31060862654` 全绿；合并前 `origin/main`、base/merge-base 均为 `b0698f57`，唯一开放 PR 为自身且 CLEAN/MERGEABLE。#81 squash 合并为 `871b84dc`；AUD-010 随后从该最新主线建立独立 worktree，四个重叠合同文件可兼容。 |
| 2026-08-06 | AUD-010 最新主线、`PENDING.md`、context/Registry/worker、SDK/contract、legacy fixture 与 PR #81 重叠复核 | planned | `origin/main@b0698f57` 根因仍成立；wire snake_case、公开 camelCase 与真实 legacy consumer 同时存在。已建立 `.trellis/tasks/08-06-plugin-context-casing-compatibility` 并写入 6.27，锁定 canonical wire + parse 后同值 alias + truncation flags；因 #81 共享四文件，尚未修改产品代码。 |
| 2026-08-06 | AUD-026 首轮 Actions 失败归因、云端 rustfmt artifact、复验、提交与 PR #81 更新 | 本地允许阶段通过；Actions `31060862654` 运行中 | `31060135548` 只在 generated-file drift 门失败；artifact 仅含 `pipeline.rs` 排版，原样提交为 `27213728`。plugin-hardening 4/4、SDK 30 tests、两包 TypeScript、plugin docs、Prettier、范围和 diff 通过；第二轮已触发。 |
| 2026-08-06 | AUD-026 实施、差异审查、补充竞态/文档修正、PR 前最新主线门与 Ready PR #81 | 本地允许阶段通过；Actions 等待终态 | 初始候选 `4255a154` 经审查补齐旧 snapshot 在途 circuit 写回、两份遗漏文档和 fail-closed log timeout/circuit 回归；#80 合并后与 12 个目标文件零重叠，无冲突重放为 `fd5ab186` 到 `origin/main@b0698f57`。SDK 30 tests、两包 TypeScript、plugin docs/API contract、Prettier、16 个 snapshot-scoped circuit 写入源合同、早停顺序、精确范围和 diff 全过；Ready PR #81 已创建。 |
| 2026-08-06 | AUD-026 最新主线、`PENDING.md`、日志持久化、policy/circuit、redactor 可复用性、合同文档与开放 PR 复核 | planned | `origin/main@cab1229a` 根因完整成立；现有 redactor 不足以安全 fallback，显式 fail-closed 可用 drop-log 封闭终态闭环。已建立 Trellis 任务和独立 worktree，计划限定 pipeline/logging、Rust/SDK validator、相邻测试与五份直接文档；#80 无重叠，尚未修改产品代码。 |
| 2026-08-06 | AUD-050 第二轮 Actions、合并前最新主线门与合并后树验证 | 通过并合并 PR #79 | Actions `31052820007` 全绿；合并前 `origin/main`/base/merge-base 均为 `ecd82606`，#80 仅改 CI/audit/lock 且无重叠，#79 CLEAN/MERGEABLE。PR squash 合并为 `cab1229a`，两目标文件 blob 与候选一致。 |
| 2026-08-06 | AUD-007 failure-first、云端锁同步、最终静态门、Actions、合并前主线门与合并后树验证 | 通过并合并 PR #80 | 首轮 `31054009003` 产出只含三个预期版本变化的 artifact；最终零公告 ID、无临时 update、精确三路径、YAML、diff 与独立审查通过。Actions `31054953851` 全绿；合并前 `origin/main`/base/merge-base 均为 `cab1229a` 且无其他开放 PR。候选 `2f499dd0` squash 合并为 `b0698f57`，三目标路径与候选一致。 |
| 2026-08-06 | AUD-050 实施、两轮差异审查、提交前主线门与 Ready PR #79 | 本地允许阶段通过；第二轮 Actions 运行中 | 初始候选 `bbbc6cf9` 只改两目标文件；12/12 源合同与 diff 通过。首轮 `31052415102` 只报 rustfmt 漂移，按云端制品精确修补为 `e89efd3c`；第二轮 `31052820007` 运行中。提交前 `origin/main`/base/merge-base 均为 `ecd82606`，无开放 PR 或相关漂移。 |
| 2026-08-06 | AUD-050 最新主线、`PENDING.md`、安装/更新/回滚/repository 调用链、PR #68 后历史与开放 PR 并行复核 | planned | `origin/main@ecd82606` 根因仍成立，远端无开放 PR；已建立 `.trellis/tasks/08-06-plugin-version-immutability` 和独立 worktree，只写入实施/验证计划，尚未修改产品代码。 |
| 2026-08-06 | AUD-025 两文件实现、差异审查、三轮 CI 修补、最终远端门与合并后树验证 | 通过并合并 PR #78 | 最终候选 `b7f5378c` 严格两文件；diff/范围/发送顺序/锁源合同通过。Actions `31047727848` 的 frontend、云端格式/绑定、Clippy、Rust tests、依赖审计和 `ci-gate` 全绿；合并前 `origin/main`、base 与 merge-base 均为 `60b12aa4`，仅自身 PR 开放且 CLEAN/MERGEABLE。PR squash 合并为 `ecd82606`，两目标文件树与候选一致。 |
| 2026-08-06 | AUD-039 Actions、合并前最新主线门与合并后树验证 | 通过并合并 PR #76 | Actions `31039483396` 全绿；合并前 `origin/main`、PR base/merge-base 均为 `ff09a81a`，仅 #76 开放且 CLEAN/MERGEABLE。PR squash 合并为 `9e83772c`，14 个目标文件树与候选 `90230c56` 完全一致，远端无开放 PR。 |
| 2026-08-06 | AUD-008/AUD-025 最新主线、`PENDING.md`、生产调用链、自检上下文与开放 PR 复核 | AUD-008 保持 confirmed；AUD-025 planned | `origin/main@ff09a81a`：AUD-008 没有无需产品生命周期决策的安全子集；AUD-025 可复用实际监听 self-check，在两文件内阻止出站自环。#76 与两项均无文件或功能重叠；已建立 Trellis 任务并写入 6.23，尚未修改 AUD-025 产品代码。 |
| 2026-08-06 | AUD-039 PR 前远端门、推送与 Ready PR #76 | 本地允许阶段通过；Actions 运行中 | `origin/main`、base 与 merge-base 均为 `ff09a81a`，无其他开放 PR；候选 `90230c56` 严格 14 文件且工作树干净。PR #76 MERGEABLE，Actions `31039483396` 运行中。 |
| 2026-08-06 | AUD-039 最新主线重放与完整前端复验 | 本地允许阶段通过；PR 前远端门待执行 | 候选无冲突重放 `origin/main@ff09a81a` 为 `90230c56`；14 文件、工作树干净。143/143 定向、312 文件 2814/2814 全量、TypeScript、ESLint/Prettier、Vite build、60 调用 AST（0 invalid）与 diff 全过。 |
| 2026-08-06 | AUD-039 #73/#75 合并后最新主线漂移核对与整合计划 | 兼容漂移待整合 | `origin/main@ff09a81a`；`94da784b..ff09a81a` 与 14 个目标文件零交集且无同类实现。已先登记重放、完整前端复验、范围检查和 PR 门计划，尚未改写候选。 |
| 2026-08-06 | AUD-003 第二轮 Actions、最终主线门与 Ready PR #75 | 通过并合并 PR #75 | Actions `31036360425` 全绿；最终 `origin/main`、PR base 与 merge-base 均为 `2a79978c`，head `007e612d` CLEAN/MERGEABLE，且只有 #75 开放。PR squash 合并为 `ff09a81a`，合并后三文件树一致；无待决策冲突。 |
| 2026-08-06 | AUD-039 失败优先、14 文件最小实现与本地完整前端门 | 本地允许阶段通过；最新主线门待执行 | 旧实现可见标签回归 1 failed / 6 passed；修复后 8 文件 143/143、全量 312 文件 2811/2811、TypeScript、目标 ESLint/Prettier、Vite build、`git diff --check` 全过。AST 扫描 60 个生产调用为 51 control + 9 group + 0 invalid；无依赖、业务或视觉 class 改动。 |
| 2026-08-06 | AUD-003 #73 漂移整合、重放与 PR #75 更新 | 本地复验通过；第二轮 Actions 运行中 | 候选无冲突重放 `origin/main@2a79978c` 并同步 0.60.49 示例 tag；新 head `007e612d` 严格三文件，Homebrew/相邻发布合同、Node、Prettier、diff 全过，Actions `31036360425` 运行中。 |
| 2026-08-06 | AUD-003 首轮 Actions、#73 合并后最新主线漂移核对与整合计划 | Actions 全绿；兼容漂移待整合 | Run `31034183720` 全绿；合并门捕获 #73 已合并为 `2a79978c`。Cask/CI/Release 合同无重叠，仅 package version 变为 0.60.49，使文档示例 tag 过期；已先登记重放 main、同步 tag、重验和更新 #75 的计划，尚未改写候选。 |
| 2026-08-06 | AUD-039 `PENDING.md`、`origin/main@94da784b`、FormField/70 个调用/测试/历史与开放 PR 复核 | planned | 生产仍有 34 个直接 child（25 个有唯一主控件、9 个真正复合控件），根因成立；#73/#75 无目标文件或同功能实现。已建立 Trellis 任务并写入 6.22，计划以可判别 control/group 合同收口全部调用，尚未修改产品文件。 |
| 2026-08-06 | AUD-003 最新主线门、提交推送与 Ready PR #75 | 本地允许阶段通过；Actions 运行中 | `origin/main`、branch base 与 merge-base 均为 `94da784b`；#73 head `30b16337` 无三文件或功能交集。候选 `f1861d5a` 严格只含三个目标文件并已推送，Ready PR #75 等待云端终态。 |
| 2026-08-06 | AUD-003 failure-first、三文件最小实现、Homebrew/相邻发布合同、Node/Prettier/diff 与完整差异复核 | 本地允许阶段通过；PR 前主线门待执行 | 旧实现目标合同 1/5，修复后 5/5；`check:homebrew-cask`、release promotion/source、TUI release contract、两份 Node 语法、目标 Prettier 与 diff 全部通过。只改生成器、自测和发布文档；按规则未运行本地 Rust/native 或真实 `brew style`。 |
| 2026-08-06 | AUD-003 `PENDING.md`、`origin/main@94da784b`、Release/Cask/README/历史与开放 PR 复核 | planned | 正式发布仍为 macOS ARM-only，生成器/自测仍强制 Intel SHA，文档仍虚构自动 tap 同步；#73 无三文件交集。已建立 Trellis 任务并写入 6.21，计划只改生成器、自测和文档，尚未修改目标文件。 |
| 2026-08-06 | AUD-046 第二轮 Actions、最终主线门与 Ready PR #74 | 通过并合并 PR #74 | Actions `31030917177` 全绿；最终 `origin/main`、PR base 与 merge-base 均为 `d26524f2`，head `a50ec5be` CLEAN/MERGEABLE，#73 无交集。PR squash 合并为 `94da784b`，远端 main 与候选三文件树一致；无待决策冲突。 |
| 2026-08-06 | AUD-046 云端 rustfmt artifact 应用、复验、提交与 PR #74 更新 | 本地允许阶段通过；Actions `31030917177` 运行中 | 应用前 `origin/main`/merge-base 仍为 `d26524f2`，#73 无交集；artifact 原样形成单文件提交 `a50ec5be`。7/7、diff、三文件总范围与 merge-base 通过；PR base/head 为 `d26524f2`/`a50ec5be`，新 run 已启动。 |
| 2026-08-06 | AUD-046 Actions `31030242037` 失败归因与云端 drift artifact 审查 | planned | Rust 只在 generated-file drift 门失败；artifact 仅含 `extension_host_registry.rs` 的 3+/12- rustfmt 折行。已先登记 fetch 最新 main/开放 PR、原样应用、重跑 7/7 + diff + 范围/merge-base、单独提交并重新交 Actions 的计划，尚未应用补丁。 |
| 2026-08-06 | AUD-046 最新主线门、提交推送与 Ready PR #74 | 本地允许阶段通过；Actions `31030242037` 运行中 | 最新 `origin/main`、PR base 与 merge-base 均为 `d26524f2`，远端 head 为 `139c8432` 且文件集严格为 3 个目标文件。#73 最新 head `f8ddfdfa` 仍无目标文件或同功能实现；PR #74 MERGEABLE，等待必需检查。 |
| 2026-08-06 | AUD-046 failure-first、三文件最小实现、源码合同、diff 与独立差异审查 | 本地允许阶段通过；PR 前主线门待执行 | 旧代码 7 项合同为 0/7，修复后为 7/7；只改 Extension Host 3 个既定文件并新增无后续请求/活跃调用/弱任务退出回归。`git diff --check` 与完整差异复核通过；独立审查发现的 `Notify` 丢信号 P2 已改为 `notify_one` 并加 1 秒 join timeout。按规则未本地运行 Rust。 |
| 2026-08-06 | AUD-046 `PENDING.md`、`origin/main@d26524f2`、command/gateway registry、child recycle、历史与开放 PR 复核 | planned | 根因仍成立；两个生产 registry 构造点均无 timer，child recycle 未转发，registry dispose 只有机会式/测试调用。已建立 Trellis 任务并写入 6.20，计划只改三个运行时文件，先做 failure-first 与无后续请求 timer 回归，尚未修改产品代码。 |
| 2026-08-06 | AUD-032 Actions、最终主线门与 Ready PR #72 | 通过并合并 PR #72 | Actions `31026666018` 全绿；最终 `origin/main` 仍为 `7c395d15`，PR base/head 为 `7c395d15`/`0a5bd769` 且 CLEAN/MERGEABLE，新开放 #73 无插件管线交集；squash 合并为 `d26524f2`。 |
| 2026-08-06 | AUD-032 #71 后主线重放、云端 rustfmt artifact、7/7 合同与 PR 更新 | 本地允许阶段通过；新 CI 运行中 | 逻辑提交无冲突重放为 `2a1878ba` 到 `origin/main@7c395d15`，artifact 原样提交为 `0a5bd769`；7/7、diff、单文件范围、完整差异复核和 merge-base 通过。PR #72 base/head 已更新为 `7c395d15`/`0a5bd769`，Actions `31026666018` 运行中。 |
| 2026-08-06 | AUD-032 Actions `31025383330` 失败归因、artifact 审查与 #71 后主线复核 | planned | frontend 与合同检查通过；Rust 只报云端 drift。artifact 仅含 `pipeline.rs` 两个新增测试的 rustfmt 换行。#71 已合并为 `7c395d15` 且无交集；已先登记重放最新主线、精确应用 patch、重跑 7/7 + diff + 范围/merge-base、再交 Actions 的计划，尚未修改候选。 |
| 2026-08-06 | AUD-031 Actions、最终主线门与 Ready PR #71 | 通过并合并 PR #71 | Actions `31023947314` 全绿；最终 `origin/main` 仍为 `5d4906c5`，PR base/head 为 `5d4906c5`/`f92d5190` 且 CLEAN/MERGEABLE，#72 仅改无交集的插件管线；squash 合并为 `7c395d15`。 |
| 2026-08-06 | AUD-032 failure-first、事务 patch、request/response policy 回归、提交前主线门与 Ready PR | 本地允许阶段通过；PR #72 | 旧实现源合同 3/3 按预期失败；修复后 7/7、diff、单文件范围与差异审查通过。候选 `6ebd09e5` 的 origin/main/merge-base 均为 `5d4906c5`，#71 仅有无交集的 Observer/OAuth 文件；Ready PR #72 已触发 Actions `31025383330`。 |
| 2026-08-06 | AUD-032 `PENDING.md`、`origin/main@5d4906c5`、header patch/failure policy/公开合同、历史与开放 PR 复核 | planned | 根因仍成立；默认 fail-open 与现有无条件 Err/部分 mutation 明确冲突。已建立 Trellis 任务并写入 6.19，计划只改 `pipeline.rs`，先做 failure-first 和 request/response 原子回归，尚未修改产品代码。 |
| 2026-08-05 | AUD-031 #70 后主线重放、云端 rustfmt patch、源合同与 PR 更新 | 本地允许阶段通过；新 CI 运行中 | 候选无冲突重放为 `df4dcdca` 到 `origin/main@5d4906c5`，artifact 原样提交为 `f92d5190`；4/4 候选合同、3/3 OAuth 合同、diff 与 merge-base 通过。PR #71 base/head 已更新，新 Actions `31023947314` 运行中。 |
| 2026-08-05 | AUD-031 Actions `31022477938` 失败归因与云端 drift artifact 检查 | planned | frontend、support-contract 等通过；Rust 仅因生成/格式漂移失败。artifact 只有 `snapshot.rs` 一处 rustfmt 换行，已先登记重放 `origin/main@5d4906c5`、精确应用 patch、重跑 4/4 + 3/3 源合同与 diff、再交 Actions 验证的计划，尚未应用产品补丁。 |
| 2026-08-05 | AUD-022 Actions、最终主线门与 Ready PR #70 | 通过并合并 PR #70 | Actions `31020581604` 的 frontend、rust、合同与 ci-gate 全绿；最终候选 `35491b78` 基于 `origin/main@9a280136` 且无相关开放 PR 竞争，squash 合并为 `5d4906c5`。修改仍限 memory diagnostics service/test，遗留风险保持记录。 |
| 2026-08-05 | AUD-031 failure-first、批量 gate、缺失 OAuth 快照、独立审查与 Ready PR | 本地允许阶段通过；PR #71 | 旧实现源合同报出 3 项缺口；修复后候选 4/4、OAuth 3/3、diff 通过。候选 `ccfb0f4d` 仅改 Observer/OAuth 两文件，base/merge-base 为 `origin/main@9a280136`；#70 无交集。按规则未本地运行 Rust，等待 Actions `31022477938`。 |
| 2026-08-05 | AUD-031 `PENDING.md`、`origin/main@9a280136`、候选投影/批量 OAuth API/轮询与 #69/#70 文件集复核 | planned | 根因仍成立且无相关主线或开放 PR 实现；已建立 Trellis 任务并写入 6.18。计划只复用现有批量 display snapshots、暴露 crate 内输入上限并补定向回归，尚未修改产品代码。 |
| 2026-08-05 | AUD-022 failure-first、宽对象预算、#69 后主线重放、定向门、两轮独立审查与 Ready PR | 本地阶段通过；PR #70 | 旧实现新增预算回归 2 failed / 2 passed，`Object.entries` getter 负例随后按预期失败；修复后目标 5/5、service 10 files / 52 tests、TypeScript、目标 ESLint/Prettier、隔离 Vite 与 diff 通过。候选 `35491b78` 的 base/merge-base 为 `origin/main@9a280136`，创建前无其他开放 PR；仅改目标两文件，等待 Actions `31020581604`。 |
| 2026-08-05 | AUD-022 `PENDING.md`、`origin/main@e94c83bd`、生产入口/预算/聚合与 #68/#69 文件集复核 | planned | 根因仍成立且无相关主线或开放 PR 实现；已建立 Trellis 任务并写入 6.17。计划只增加共享 20 万节点/2,000 query 预算、截断元数据和有界 top-20，尚未修改产品代码。 |
| 2026-08-05 | AUD-023 failure-first、下载前数量门、#68 后主线重放、定向门、Actions 与 Ready PR | 通过并合并 PR #69 | 旧实现 45 tests 中新增 3 项按预期失败；候选从 `b8303703` 无冲突重放为 `f703c863` 到 `origin/main@e94c83bd`。重放后 Image Gen 8 files / 223 tests、TypeScript、目标 ESLint/Prettier、Vite、源合同、diff 与 Actions `31017816818` 全绿；最终主线无漂移，squash 合并为 `9a280136`。 |
| 2026-08-05 | AUD-040 failure-first、checksum 贯通、两轮 Actions、最终主线门与 Ready PR | 通过并合并 PR #68 | 旧实现 3 files / 45 tests 为 7 failed / 38 passed；候选 `4ce877b6` 最终基线 `origin/main@0854d830`。首轮 `31014368791` 唯一 Clippy dead-code 已最小修正，第二轮 `31015354600` 全绿；最终 CLEAN/MERGEABLE 且无竞争 PR，squash 合并为 `e94c83bd`。 |
| 2026-08-05 | AUD-017 failure-first、最终 JSON 字节存储、三轮 Actions、最终主线门与 Ready PR | 通过并合并 PR #67 | `origin/main@d5c9cfe0` 旧实现无字节预算；候选只修改 `response_cache.rs`。run `31010223216` 的 rustfmt drift 以 `d60cc100` 精确应用，`31011383445` 的 Clippy 发现以 `4de2889b` 最小修正；最终 `31012064253` 全绿。合并前 base/merge-base 无漂移且无竞争 PR，squash 合并为 `0854d830`。 |
| 2026-08-05 | AUD-043 failure-first、10 类 mutation、发布合同、差异审阅、最终 origin/main 门与 ready PR | 通过并合并 PR #66 | 旧 workflow 的 `$GITHUB_ENV` 先被新 checker 拒绝；提交 `1fcc687d` 改为 runner-temp 0600 key、step-scoped 路径和紧邻 cleanup。独立审阅发现 `GITHUB_OUTPUT` 绕过后扩展为五类 command-file 防线；10 项 Node 合同、Prettier、diff 与 Actions `31005579029` 全绿。最终 base/merge-base 仍为 `405a545f`，squash 合并为 `d5c9cfe0`。 |
| 2026-08-05 | AUD-043 `PENDING.md`、`origin/main@405a545f`、workflow/history/open PR、Tauri 私钥路径合同复核 | planned | 根因仍成立且无相关漂移/开放 PR；已建立 Trellis 任务并写入 6.13。计划只用 runner-temp 文件、step-scoped 路径、紧邻 cleanup 和静态合同收窄作用域，尚未修改产品 workflow。 |
| 2026-08-05 | AUD-012 失败优先、共享 scope、changed-key patch、最终主线门与 ready PR | 通过并合并 PR #64 | 旧实现第二个独立 hook 会提前发送旧缓存 payload；候选 `f6d7d2d4` 最终基线 `origin/main@e57acb54`，6 个测试文件/82 tests、TypeScript、目标 ESLint/Prettier、Vite、diff、差异审查及 Actions `30999335471` 全绿。PR #64 squash 合并为 `5c756edc`；外部/native 普通 writer 的 revision/CAS 保留。 |
| 2026-08-05 | AUD-030 CacheKey/TTL/insert 生命周期、#64 漂移整合、独立差异审查、三轮 CI 与 ready PR | 通过并合并 PR #65 | 候选无冲突重放到 `origin/main@5c756edc`；唯一产品文件为 Observer 模块。三轮 Actions `31000946748`/`31001414671`/`31001992510` 最终全绿（首轮 rustfmt artifact、第二轮 Clippy 一处测试写法均按最小范围修正），当前头 `5f249948` squash 合并为 `405a545f`；条目上限非字节权重和访问时清理遗留风险保留。 |
| 2026-08-05 | AUD-012 `origin/main@ba06dabb`、前端 settings writers、原生 ownership barrier 与 TanStack scope 复核 | planned | 专用 owner 竞态已由主线消除；common/Wsl 两个普通 patch hook 仍可并发从同一 cache 展开整份输入，设置页也未进入共享 scope。计划只统一普通前端串行 patch，不改 native/schema，尚未修改产品代码。 |
| 2026-08-05 | AUD-038 失败优先、双向窗口、最新主线门、定向/浏览器验证与 ready PR | 通过并合并 PR #63 | 旧实现第 11 页后淘汰 page 0 且无 previous 能力；候选 `f7d6fc17` 最终基线 `origin/main@c2e4db25`，2 个测试文件/28 tests、TypeScript、目标 ESLint/Prettier、Vite、diff 与 1024px 双向浏览器验证通过。Actions `30997519757` 全绿，ready PR #63 squash 合并为 `e57acb54`；十页窗口与 390px 侧栏问题保留。 |
| 2026-08-05 | AUD-029 失败优先、共享 deadline、请求级 override、timeout 分类、独立审查与提交前主线门 | 通过并合并 PR #62 | 旧生产源缺少四项合同；修复后 7/7 源检查、diff 与真实延迟 body timeout 回归源码通过。候选 `13abfea7` 最终基线 `origin/main@ba06dabb`；Actions `30995513871` 全绿，ready PR #62 squash 合并为 `c2e4db25`。按规则未本地运行 Rust；异常断连 cancellation 保留为有界风险。 |
| 2026-08-05 | AUD-029 `origin/main@5b13683b`、`PENDING.md`、protocol/observer/TUI deadline 与并行 PR 文件集核验 | planned | 根因仍成立且现行手动探测在后台 task 中执行；已建立 Trellis 任务并写入 6.9。计划仅共享 20 秒 deadline、增加 request-level 余量和明确 timeout 分类，尚未修改产品代码。 |
| 2026-08-05 | AUD-051 失败优先、SDK/跨层合同、最新主线门与 ready PR | 通过并合并 PR #61 | 旧 SDK typecheck/合同门分别按预期失败；候选 `c62c4725` 重放为 `29c2139e`，最终基线 `origin/main@d12dbfe3`。SDK 29 tests、plugin-hardening、docs/completion、脚手架 33 tests/typecheck、根 TypeScript、目标 lint/Prettier、Node、Vite、diff 与 Host route 删除负例通过；Actions `30993065410` 全绿，#61 squash 合并为 `ba06dabb`，远端 main 已由 `git ls-remote` 复核。 |
| 2026-08-05 | AUD-015 失败优先结构回归、最新主线重放、相关前端门、真实小视口与 Actions | 通过并合并 PR #60 | 旧实现 1/15 失败；候选最终重放为 `4d1d720a` 后 2 files/21 tests、TypeScript、ESLint、Prettier、Vite build、diff 通过。1024x600 下 failed Outlet 474px、滚动区 402/1742 且末端可达；ready Outlet 560px；两者无水平溢出。Actions 全绿并 squash 合并为 `d12dbfe3`。 |
| 2026-08-05 | AUD-015 `origin/main@0062c907`、`PENDING.md`、AppLayout/Settings/现有测试与 #58/#59 文件集核验 | planned | 根因仍成立，当前 PR 无交集；已建立 Trellis 任务并写入 6.7，只规划 main flex/Outlet 剩余高度与定向/浏览器验证，尚未修改产品代码。 |
| 2026-08-05 | AUD-044 最新主线整合、标准 CI、四平台 dev-build、下载解包与最终合并门 | 通过并合并 | 提交重放为 `2e519e51`，最终基线 `origin/main@5b13683b`。标准 CI `30984325182` 与四平台 runs `30986450817`/`30986450918`/`30986450908`/`30986450875` 全绿；下载后 macOS/Linux mode 与三平台架构通过，#59 squash 合并为 `62574e22`。 |
| 2026-08-05 | AUD-052 真实 chunk 失败、差异审阅、最新主线重放、Actions 与最终合并门 | 通过并合并 | 提交由 `b64289f9` 重放为 `c4f17111`；25/25 tests、TypeScript、ESLint、Prettier、Vite build、diff 与桌面/窄视口 Playwright 通过，Actions run `30981668718` 全绿；最终基线 `origin/main@0062c907`，PR #58 合并为 `5b13683b`。 |
| 2026-08-05 | AUD-005/006 失败优先、自测绕过复核、两轮独立差异审阅、最终本地门、Actions 与合并前 `origin/main` 门 | 通过并合并 | 头提交 `400491b8`；跨行/行注释/嵌套块注释的 Instant 反例、死文本 prepush 伪接线及隔离 TS2322 负例均先失败后通过。CI 矩阵/Instant 合同、根 lint/typecheck、plugin-hardening、脚手架 33 tests、目标 Prettier、diff 与 Actions run `30979684244` 全部通过；合并前主线仍为 `db92a480`，PR #57 squash 合并为 `0062c907`，无待决策冲突。 |
| 2026-08-05 | PR #53/#54 终态、合并前两次 `origin/main` 门与合并后 SHA 验证 | 通过 | AUD-001/AUD-021 分别以 `891c9eb3`/`ef41e6da` 合并；两项必需检查成功，无等待期主线漂移、重复实现或待决策冲突。 |
| 2026-08-05 | AUD-037/AUD-053 失败优先、定向/全量前端验证、独立差异审查、最终主线门与云端终态 | 通过并合并 | AUD-053 PR #55 合并为 `ed72549b`，AUD-037 PR #56 在其上重放后合并为 `db92a480`；两项 frontend/rust/support-contract/ci-gate 通过，无待决策冲突。 |
| 2026-08-05 | 下一批 `origin/main@ef41e6da` 只读复核 + `PENDING.md` + 精确质量矩阵/workflow/生产调用核验 | planned | AUD-005/006/044/052 根因仍成立，已建立一个父任务和三个子任务并写入 6.6；尚未修改下一批产品/workflow 代码。 |
| 2026-08-05 | fetch `origin/main` + PR #51/#52 终态 + 8 个领域只读复核 + 精确主线 worktree/CodeGraph 核验 | 通过 | 既有 52 项和 5 个假设均在 `origin/main@eeccf64d` 重新核验；16 项 resolved、33 项 confirmed，AUD-001/021/037 进入 planned。更正 AUD-052 的生产调用面；用户新增供应商页需求经同一基线确认并登记为 planned `AUD-053`。 |
| 2026-08-05 | 报告结构 Node 校验 + 5 个 Trellis 任务 `task.py validate` | 通过 | 53 个索引 ID 与 53 个详细 ID 一一对应，无重复、遗漏或状态错位；状态为 confirmed=33、planned=4、resolved=16，优先级为 P1=18、P2=34、P3=1。四个实施子任务均有真实 spec/研究/报告上下文，父任务无直接实现。 |
| 2026-08-05 | 规划阶段变更范围检查 | 通过 | 仅新增/更新 `CODEBASE_HEALTH_AUDIT.md` 与 `.trellis/tasks/08-05-*` 规划文件；未修改产品代码、测试或 workflow，故未运行产品测试。 |
| 2026-08-04 | 第二批当前主线复核 + CodeGraph 调用面 + 精确文件/测试复核 | planned | `origin/main@fef05dec` 仍存在 `AUD-024/AUD-041/AUD-052` 根因；报告和四个 Trellis 任务已记录实施、验证、依赖、风险与 PR 前主线门，尚未修改产品代码。 |
| 2026-08-04 | 9 组只读领域核验 + CodeGraph/精确文件复核 + `git log 9d1fb966..86a30710` | 通过 | 逐项复核 51 个原始问题和 5 个假设；确认 4 resolved、4 planned、43 个原问题仍 confirmed，另将 `HYP-005` 晋升为 `AUD-052` |
| 2026-08-04 | 报告结构 Node 校验 | 通过 | 52 个索引 ID 与 52 个详细 ID 一一对应，无重复/遗漏；状态为 confirmed=44、planned=4、resolved=4，优先级为 P1=18、P2=33、P3=1 |
| 2026-08-04 | `task.py validate` 检查父任务与三个子任务 | 通过 | 三个实施子任务的 `implement.jsonl`/`check.jsonl` 均包含真实 spec/报告上下文；父任务无直接实现，未配置 context manifest |
| 2026-08-04 | `git diff --check` | 通过 | 报告与 Trellis 规划文件无空白错误；本阶段未修改产品代码，未运行产品测试 |
| 2026-08-04 | Release promotion 定向验证与 PR 前主线门 | 通过，draft PR #40 | 本地 promotion/source self-test、YAML parse、spec/TUI contract、Prettier 与 diff 检查通过；`origin/main` 无冲突；GitHub Actions run `30898805002` 的 ci-gate、frontend、rust、docs/support contract 均成功。 |
| 2026-08-04 | DB 初始化重试定向验证与 PR 前主线门 | 通过，draft PR #41 | 定向测试覆盖失败后重试、成功缓存、并发去重和启动阶段恢复；首轮 Actions run `30902714293` 的缺失分号由 `90fed48e` 修复，第二轮云端 rustfmt drift 由 `460845a0` 精确应用。按规则未本地运行 Rust 工具链，GitHub Actions 必需检查已通过。 |
| 2026-08-04 | 插件详情身份保护定向验证与 PR 前主线门 | 通过，draft PR #42 | 失败优先竞态回归与页面 34 tests、TypeScript、ESLint、Prettier、Vite build、diff 检查均通过；`origin/main` 仍为 `fef05dec`，无页面/query 漂移；GitHub Actions 必需检查已通过。 |
| 2026-08-04 | Updater fallback tag 解码保护定向验证与 PR 前主线门 | 通过，draft PR #43 | `%ZZ`、截断 UTF-8 和合法编码 tag 回归通过；10 个 updater tests、3 个关联 query tests、TypeScript、ESLint、Prettier、Vite build、diff 检查通过；PR 前 `origin/main@fef05dec` 无相邻调用合同漂移；GitHub Actions 必需检查已通过。 |
| 2026-08-04 | attempts_json 元素校验定向验证、差异审查与主线门 | 通过，draft PR #44 | 失败优先 `[null]` 可复现链路视图/错误摘要抛错；修复后 3 files、49 tests、TypeScript、ESLint、Prettier、Vite build、diff 检查通过。重新 fetch 与 GitHub API 均确认 `main@fef05dec`；GitHub Actions 必需检查全部通过。 |
| 2026-08-04 | AUD-052 可达性复核与实验回退 | 不处理 | CodeGraph 仅发现测试调用者，`src` 没有生产 JSX、导入或调用页面；失败优先实验已完全撤销且 worktree 无差异。没有生产入口，故未执行 Playwright 浏览器验证。 |
| 2026-08-04 | AUD-042/048/049 当前主线复核与批次计划 | planned | 已 fetch `origin/main@fef05dec`，无基线后主线提交；四个独立探索分别确认 AUD-042/048/049 存在生产触发路径与最小修复，AUD-039 因 35 个调用点/复杂语义迁移暂不混入本批。已建立父任务和三个子任务，尚未修改产品代码。 |
| 2026-08-04 | pnpm audit 响应 fail-closed 定向验证、安全审查与主线门 | 通过，draft PR #45 | 失败优先顶层 error 反例确认修复前不抛错；修复后五组畸形响应 fail closed，既有 severity/例外路径保留。两份 Node 语法、selftest、目标 Prettier、diff 与差异审查通过；PR 前 `origin/main@fef05dec` 无漂移；GitHub Actions 必需检查全部通过。 |
| 2026-08-04 | MCP upsert workspace 缓存刷新定向验证与主线门 | 通过，draft PR #46 | 失败优先 enabled 分叉回归确认旧实现不触发 invalidation；修复后 query/view 共 16 tests、TypeScript、ESLint、Prettier、隔离 Vite build 和 diff 检查通过；PR 前 `origin/main` 与 merge-base 均为 `fef05dec`，GitHub Actions 必需检查全部通过。 |
| 2026-08-04 | MCP JSON 输入边界定向验证、审查与主线门 | 通过，draft PR #47 | 失败优先合法超限 JSON 反例确认旧实现仍本地解析并填充；修复后 dialog/service 共 17 tests、TypeScript、ESLint、Prettier、隔离 Vite build 和 diff 检查通过；五轴审查零发现，PR 前 `origin/main` 与 merge-base 均为 `fef05dec`，GitHub Actions 必需检查全部通过。 |
| 2026-08-04 | AUD-009/011/014 当前主线复核与批次计划 | planned | `PENDING.md` 无未解决条目，`origin/main@fef05dec` 未漂移；五个独立只读核验面确认三项仍有生产触发路径、明确最小修复与失败优先测试。父任务和三个子任务已记录实施、验证、风险与 PR 前主线门；当时尚未修改产品代码。 |
| 2026-08-04 | 插件脚手架 payload 合同定向验证与主线门 | 本地通过，draft PR #48 | 失败优先 VM 回归确认三种模板均返回 pass；修复后 package 33 tests、TypeScript、模板源码 ESLint、Prettier、四项插件合同检查、隔离 Vite build 和 diff 检查通过；PR 前 `origin/main` 与 merge-base 均为 `fef05dec`。 |
| 2026-08-04 | 启动状态乱序响应定向验证、竞态审查与主线门 | 通过，draft PR #49 | 失败优先 deferred GET/retry、listener ready、卸载/StrictMode 旧订阅回归锁定根因；修复后 6 个相关测试文件 25 tests、TypeScript、目标 ESLint、Prettier、Vite production build、diff 检查通过。独立审查发现并补齐同代际并发 GET 清理竞态；PR 前重新 fetch，`origin/main` 与 merge-base 均为 `fef05dec`。 |
| 2026-08-04 | CLI proxy 启用预检串行化定向验证、审查与主线门 | 通过，draft PR #50 | 失败优先 5 条断言确认同 act 跨 key 启动 2 次 IPC、prompt/busy/UI 无所有权；修复后 6 个相关测试文件 42 tests、TypeScript、目标 ESLint、Prettier、Vite production build、diff 检查通过。PR 前重新 fetch，`origin/main` 与 merge-base 均为 `fef05dec`。 |
| 2026-08-04 | 运行时合同与并发状态批次汇总 | 本地完成，3 个 draft PR | AUD-009/AUD-011/AUD-014 分别进入 PR #48/#49/#50；16 个目标文件均有明确归属，无依赖升级、无主线冲突、无搁置项。#48 必需检查通过，#49 frontend 通过/rust 运行中，#50 frontend/rust 运行中。 |
| 2026-08-04 | #41 至 #50 单 PR 整合、联合验证与最终主线门 | 本地通过，Ready PR #51 自动合并等待中 | 从已包含 #40 的 `origin/main@cec2353f` 无冲突重放 12 个最终提交；33 个文件逐项等同原 PR 内容且无额外路径。20 个前端文件 192 tests、脚手架 33 tests、audit selftest、四项插件合同、两套 TypeScript、ESLint、Prettier、隔离 Vite build 和 diff 检查通过。原 #41 至 #50 已关闭并指向 #51；Actions run `30920988836` 停止监控时 frontend/合同/范围检查通过，rust 仍在运行。 |
| 2026-08-03 | `git status --short --branch` | 通过 | 记录初始分支和工作区；未修改既有未跟踪内容 |
| 2026-08-03 | 静态读取仓库规则、Trellis、README、`package.json`、`Cargo.toml`、Vite/Vitest 配置 | 通过 | 建立架构、测试和本地执行约束基线 |
| 2026-08-03 | 全部 `scripts/*.mjs` 执行 `node --check` | 通过 | 所有 Node 检查/发布辅助脚本语法有效；未执行会触发 Rust/Tauri 的聚合脚本 |
| 2026-08-03 | `./node_modules/.bin/tsc -p tsconfig.json --noEmit --incremental false` | 通过 | 当前终态工作树的根 TypeScript 类型检查通过 |
| 2026-08-03 | `./node_modules/.bin/tsc -p packages/plugin-sdk/tsconfig.json --noEmit --incremental false` | 通过 | Plugin SDK 严格类型检查通过；不否定 `AUD-051` 的 API 表面缺口 |
| 2026-08-03 | `./node_modules/.bin/tsc -p packages/create-aio-plugin/tsconfig.json --noEmit --incremental false` | 通过 | 脚手架严格类型检查通过；不否定 `AUD-006` 的 CI gate 缺失 |
| 2026-08-03 | `./node_modules/.bin/eslint src/ --no-cache` | 通过 | 按 `package.json` 实际 `lint` 范围验证当前前端源码 |
| 2026-08-03 | `./node_modules/.bin/eslint . --no-cache` | 非项目门；失败 | 全仓扩展尝试命中未跟踪 `.local/`、`coverage/` 和 package 测试/脚本的既有 unused/`any` 规则；根 lint 脚本只定义为 `src/`，故不把该结果伪装成产品 lint 回归 |
| 2026-08-03 | `./node_modules/.bin/vitest run --reporter=dot` | 工作区噪声；产品测试全过 | 306 个仓库测试文件/2668 tests 通过；4 个失败 suite 全来自未跟踪 `.local/codex-cli-reference` 缺少 Jest 依赖 |
| 2026-08-03 | `./node_modules/.bin/vitest run --exclude '.local/**' --reporter=dot` | 通过 | 306 个测试文件、2668 个测试全部通过；存在图表 SVG 的 jsdom stderr 警告，不影响结果 |
| 2026-08-03 | 报告结构 Node 校验 | 通过 | 51 个索引 ID 与 51 个详细 ID 一一对应、无重复/遗漏；统计为 P1=18、P2=32、P3=1，HYP=5 |
| 2026-08-03 | `git status`、`git rev-parse HEAD`、基线到终态 name-only/stat | 通过 | 终态 HEAD `a322ba15`，跟踪树干净；保留全部用户/并行 Session 内容，审计唯一新增文件为本报告 |

## 9. 2026-08-04 停止点与并行接续

- 用户要求停止当前 Session 的运行与 CI 监控，后续由多个新 Session 继续；停止时没有仍在运行的本地命令或监控进程。
- 主线事实：PR #40 已合并，merge commit 为 `cec2353fbedb74e96591aaf6b46c7e2c7832fd94`，因此 `AUD-019/AUD-020` 已转为 `resolved`。
- 待合并事实：Ready PR #51，head `256378240857de2548adc980d65496f8bcef9f79`，base `cec2353fbedb74e96591aaf6b46c7e2c7832fd94`；自动合并已于 `2026-08-04T14:50:19Z` 启用，方式为 squash。原 #41 至 #50 全部关闭并留言指向 #51。
- #51 修改范围：33 个文件，正好由原十个 PR 的最终路径并集组成，包括 2 个 Rust 启动文件、3 份插件文档、2 个脚手架文件、2 个依赖审计脚本和 24 个前端源码/测试文件；无依赖或锁文件变更。
- #51 CI：Actions run `30920988836`；停止监控时 `pr-title`、`change-scope`、`support-contract`、`docs-contract`、`frontend` 已成功，`candidate-plan` 按条件跳过，`rust` 仍在安装系统依赖。既有 Node 20 action 弃用注释不属于本批，不顺手升级。
- 新 Session 首先应重新读取 `PENDING.md`、fetch `origin/main`，并查询 #51 的 state/head/merge commit 与 run `30920988836` 终态。若 #51 已合并，验证主线包含其 squash commit 后，将上述 10 个 `pr_open` 项及对应 Trellis 子任务转为完成并重新计算汇总；若失败，只在 #51 分支做最小 CI 修补，不重新打开旧 PR。
- 当前没有下一批 `planned` 项。此前只读初筛认为 `AUD-005/AUD-006` 可作为同一 CI 合同批次、`AUD-044` 可独立处理，`AUD-043` 仍需先确认签名 Action 的 step-scope 合同；这些只是候选，不是已落盘实施计划，任何新 Session 都必须基于当时最新 main 重新核验后再选批，避免并行重复开发。

## 10. 2026-08-05 接续点

- 第 9 节是 2026-08-04 的历史停止点；其“#51 待合并”和“没有 planned 项”已失效。最新事实以本节、5.2 和 6.5 为准。
- 最新已核对的 `origin/main` 为 `9e83772c536c270a26484e85b4b324ecb05c82c9`。AUD-031/AUD-032/AUD-046/AUD-003/AUD-039 已分别由 PR #71/#72/#74/#75/#76 合并为 `7c395d15`/`d26524f2`/`94da784b`/`ff09a81a`/`9e83772c`；AUD-025 候选 `aef7e458` 已进入 Ready PR #78。合计 40 `resolved`、12 `confirmed`、1 `pr_open`，尚未收口 13 项。
- AUD-015 候选 head `4d1d720a` 的 21 tests、TypeScript、ESLint、Prettier、Vite、diff、1024x600 failed/ready 浏览器对照与 Actions 全部通过，最终基线 `62574e22`，合并提交 `d12dbfe3`。
- 用户已授权后续自动逐批计划和执行；AUD-031/AUD-032/AUD-046/AUD-003 均已完成。AUD-003 的 Ready PR #75 已由全绿候选 `007e612d` squash 合并为 `ff09a81a`；AUD-039 候选 `90230c56` 已创建 Ready PR #76，Actions `31039483396` 运行中。当前没有待决策候选。
- `AUD-038` 已完成并由 PR #63 合并为 `e57acb54`；窗口淘汰和 390px 侧栏遗留风险已记录。
- `AUD-012` 候选 `f6d7d2d4` 已在 `e57acb54` 通过 82 tests、静态门、构建、差异审查与 Actions，PR #64 squash 合并为 `5c756edc`。原生专用 owner、IPC schema 和外部 writer CAS 不在本批。
- `AUD-030` 候选已在 `5c756edc` 通过三轮 Actions、6/6 源合同、diff、独立差异审查与最终主线门，PR #65 squash 合并为 `405a545f`。未选的 confirmed 项继续保留，不顺手处理。
