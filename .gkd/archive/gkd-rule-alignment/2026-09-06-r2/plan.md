> 归档快照：gkd-rule-alignment，PLAN/execution r2。记录各阶段当时事实，不是活动指令；本机目录已替换为逻辑路径。

# AIO 项目规则对齐 GKD 与验证分级修正 PLAN

## 1. 当前方案 r2

- 任务：`gkd-rule-alignment`。
- 日期：2026-09-05。
- 状态：用户先批准“开始按 PLAN 执行”，随后明确要求“按 gkd main automatic 流程开始施工”；r2 获准采用 `delegated/automatic`，main 启动一个命名的 `gkd_execute` 执行 session。
- 已完成的拟案产物：本文件，以及 GKD 仓库 `docs/reports/2026-09-05-aio-rules-gkd-issues.md` 问题报告。
- 已批准第 5 节实施范围、第 8 节验证和第 9.3 节完整交付边界，并已明确选择自动施工；发布、远端设置与 GKD Skills 修改仍未授权。
- 已执行 `git fetch origin`，任务基线确认为 `bc891afbb80b70efc81b628a1c48b1bd0a7051da`；沿用分支 `chore/gkd-rule-alignment` 和执行 worktree `../worktrees/gkd-rule-alignment`，将其中的 `.gkd/execution.md` 更新为 r2。启动前 worktree 无实现改动，只有 main 创建的交接文件。
- 主工作树：当前 AIO `main` checkout。调查 HEAD 为 `193767510ef647193ce5f16390bc1f663c3dffb0`，本地 `origin/main` 为 `bc891afbb80b70efc81b628a1c48b1bd0a7051da`；未 fetch，不能把该远端跟踪值当作服务器最新状态。
- 调查开始时 AIO 工作树干净。`main` ahead 26，但与本地 `origin/main` 的文件差异仅为根级 `progress.md`、`review.md`。保留现场，不重置历史。

## 2. 目标与优先级

让 AIO 的规则只承担项目自身约束，并与 GKD 通用工作流一致。资料按改动触发读取，验证与受影响行为匹配，任务成功条件能用证据判定，已有授权不重复申请。

本次用户明确指定以下处理顺序：

1. 在系统、开发者和用户明确指令允许的边界内，GKD 是通用任务生命周期、执行路线、角色、授权、验收与收尾的优先依据。
2. AIO 的 `AGENTS.md`、规范、README、操作文档或检查器重复定义流程并与 GKD 冲突时，修正 AIO；不以项目例外绕过 GKD。
3. 零产物本地环境、Git remote、PR 门禁、squash 同步和发布来源属于项目适配信息，继续由 AIO 维护；它们不重新定义 GKD 的角色或授权。
4. 确认属于 GKD 自身的缺陷写入 GKD 目录问题报告。本任务不修改 GKD Skills、角色、手册、安装副本或其现有施工记录。
5. 其他通用 Skills 的命令范例不能替代本项目验证入口，也不能覆盖 GKD 路由。系统全局 AGENTS 与非 GKD 用户级 Skills 不在本任务修改范围。

不以模型升级为删减依据。保留先 PLAN 后执行、材料性变化确认、主代理审查、delegated 独立验收、明确 CI 目标、单 writer、未知现场保护和必要质量门。

## 3. 事实、归属与处置

以下行号来自拟案时快照，实施以章节和关键内容定位。

| ID | 位置与现状 | 影响 | 归属与本次处理 |
| --- | --- | --- | --- |
| A01 | `AGENTS.md:8`、运维文档 `:25` 把所有任务统一写成 worktree 交接 | 与 GKD direct-main 分支不一致 | AIO 修正为按 GKD 路由，delegated 才要求 execution 交接 |
| A02 | 跨层索引 `:242` 要求已移除的固定 local runner，禁止追加 checker/syntax/diff | 无法按现行工具完成验证 | AIO 删除旧入口，引用实际云端验证合同 |
| A03 | 跨层索引 `:257` 无条件要求前后端全量；Quality Check 混入多模块测试 | 与已实现的 CI 分级不一致，额外验证不按风险触发 | AIO 为场景加适用范围，保留相关核心合同 |
| A04 | guides/index `:82` 要求改任何值前搜索全仓 | 小改动触发宽搜索；没有利用既有定位结果 | AIO 改为影响范围不明或共享值变更时检索相关消费者 |
| A05 | code-reuse guide `:34` 见复制即要求共享抽象 | 与最小实现及消融原则冲突 | AIO 依据语义、维护成本和内部类型保证决定复用 |
| A06 | plugin-sdk/create-aio-plugin 的 20 个规范文件仍是空模板 | 无项目事实，诱导填表与不相关读取 | 删除逐项列出的空模板，不新增替代模板或项目 Skills |
| A07 | cross-layer thinking guide 含 Trellis 多平台、版本站点清单且有重复段 | 扩大阅读/验证范围，与 AIO 技术面不匹配 | 保留通用边界原则，移除外部项目专属和重复检查 |
| A08 | cloud-only checker `:512,544` 锁定 AGENTS/README 自然语言原句 | 语义不变的文档修订也可能阻塞 CI | 最小调整检查器及已有自测，保留必要入口和实际执行边界 |
| A09 | docs/README 的“事实源优先级”混合实现与指令权威 | 当前实现可能被误用来覆盖期望规则 | 区分实现证据与执行约束，注明 GKD 工作流归属 |
| A10 | 旧 simple-change-record 模板、根级 plan/progress/review 仍像活动记录 | 形成第二个新任务入口 | 旧模板标为停用，根级记录标为历史；不删历史正文 |
| G01-G04 | GKD closeout 的未提交改动、必需 cleanup commit、偏差粒度、完成措辞 | 可能阻塞收尾或混淆已完成阶段 | 仅写 GKD 问题报告，AIO 不添加绕过规则 |
| G05 | GKD direct-main 与 Git 分支约束组合说明不足 | 易把 main 代理与 Git main 分支混为一谈 | 报告为待澄清，不把推断写成新的项目路由 |
| D01 | 非 GKD Git Skill 要求提交前本地全套检查，review Skill 要求清理自身死代码前询问 | 与 AIO 云端验证、全局最小改动原则不匹配 | 项目明确自己的验证入口和 GKD 授权来源；不改这些全局 Skills |
| O01 | CI 合同步骤对所有源码/文档分类仍较宽 | 可能存在优化空间，但没有耗时证据 | 后续可单独分析；本次不更改 workflow、分类器或 required checks |

GKD 源仓库正在实施 `role-wait-boundaries` r12，且存在未提交修改。源 Skill 已补充复用已有授权等说明，安装副本尚有旧文案。报告分别记录仍存在的问题、待澄清事项和源码已改善事项，不将未提交源码宣称为已发布。

## 4. 修正设计

### 4.1 项目入口与生命周期

`AGENTS.md` 保留短入口、项目事实和不可绕过的项目约束：

- 普通任务加载 GKD 主流程，路线选择、交接、CI 监控、独立验收和收尾引用对应 Skill，不在项目复制完整流程。
- PLAN 仍先于施工，材料性变化仍按 GKD 处理；不新增一个免审批或自动接管路线。
- 已有授权从会话/PLAN 继承，项目不得在加载 Skill、角色交接、提交和清理时额外索要同一授权。
- `.gkd/plan.md`、`.gkd/review.md` 是主代理记录；`.gkd/execution.md`、`.gkd/progress.md` 的写入与使用按 GKD delegated 路线触发。direct-main 不为形式制造 execution/progress 或独立验收。
- 保留中文 Conventional Commit、任务分支 PR、remote/repository 限定、不推送 main、squash 后同步远端结果、保留未知改动和本地独有历史。
- 项目不提供旧 GKD runner、外部状态副本、角色替代实现或用户级 Skill 安装器。

README 的开发段落只保留项目环境和入口链接，操作文档记录 AIO 的 Git/CI/发布事实；不要求每次任务通读二者。

### 4.2 读取与检索按任务触发

- 首先阅读适用 AGENTS 和当前路线要求的 GKD 材料。
- 需要项目规范时，从索引定位所改行为对应的合同；读取相关章节及其必要上下文，跨章节语义无法判断时才扩大。
- 执行 session 的任务指令来源仍是 execution；完成任务所需代码、合同、类型和检查入口可以按需读取，不能把别的计划或历史记录当作施工指令。
- 阅读指南不要求每改一行重复检索。共享配置、协议字段、公共常量、重命名或影响面不明时，查消费者；已有定位且无新风险时复用结果。
- 保留边界校验、错误可见、DRY 和代码所有权原则；去掉“看到复制即提取”“可能有人需要就抽象”等机械条件。
- 在各 Quality Check 入口明确其清单只适用于触及对应行为的修改；不要求每个任务执行整个索引中的全部测试场景。

系统全局要求不在项目中暗中降级。全局“奠基性文档完整阅读”等问题只保留为范围外观察，本项目修订不能声称已经修复它。

### 4.3 提交、验证与风险匹配

保留本地不安装依赖、不运行 package-manager、开发服务器、测试框架、lint、类型检查、Cargo/Tauri、构建或打包的政策。

| 任务类型 | 本地证据 | 云端证据 | 新增覆盖原则 |
| --- | --- | --- | --- |
| PLAN、报告、过程记录 | 范围/引用/差异审阅；需要时零依赖检查 | 只有进入 PR 后才依现有 process 分类 | 不为文案新增单元测试 |
| README、AGENTS、现行规范 | 相关引用、cloud-only 合同、diff | 现有 checked-documentation 合同和 PR 门禁 | 检查规则变化才调整相关已有自测 |
| 单一前端或 Rust 行为 | 相关静态证据和明确运行限制 | 当前分类器选中的 frontend 或 Rust job | 真实工作流、核心行为或既有覆盖缺口需要时补回归 |
| shared、生成绑定、CI/工具脚本、未知路径 | 计划中允许的无依赖检查 | 现行 complete CI，不降低分类 | 根据实际影响面保留跨域验证 |
| 发布、签名、候选制品 | 对应任务的元数据检查 | 发布任务需要的精确提交与候选证据 | 不作为普通 PR 的新增要求 |

“提交前”和“合并前”分开：先用允许的检查产生可审查提交，再通过 PR 的自动 CI 验证；不得要求先通过只能由提交触发的云端检查才能提交。不要额外 dispatch 常规 ci。

本次将修改治理脚本，因此实施 PR 按当前政策运行完整 CI 是合理的验证成本；本方案不通过修改扩展名、分类白名单或拆出绕过门禁的提交规避它。

### 4.4 文档合同检查器的最小调整

仅修改 `assertCloudOnlyVerificationContract` 内自然语言要求及其已有自测：

1. 保留 `$gkd-main` 和四个 `.gkd` 文件入口、禁止恢复旧命令、禁止本地 package/native 示例的检查。
2. 去除对 `Keep the local checkout zero-artifact.`、普通 PR 中文整句、README 中英文禁止重复 CI 整句的逐字锁定；正文仍清晰表达这些规则。
3. 保留实际 package script Actions guard、Tauri hook 边界、CI 自动/手动 gate 区分、job 选择、必要检查命令和 PR 打包边界检查。
4. 在既有 selftest 增加或调整一个正例：仅将以上说明改写为等义文案，合同仍通过。保留缺失 GKD/execution 入口、恢复禁止命令、拆掉实际质量门的负例。
5. 不引入 Markdown/YAML 解析框架、机器状态或新的元数据文件。零依赖检查继续只读。

这是对门禁行为的验证，属于必要治理回归，不是给普通文档补实现镜像测试。

### 4.5 资料整理

- 20 个文件经复核仍为空模板时按第 5 节明确清单删除；发现新增有效项目事实则保留并在 review 说明，不能为满足删除数量破坏内容。
- 旧 `simple-change-record.md` 顶部标明已停用、新任务使用 GKD；保留正文作为历史模板，避免新增一套轻量生命周期。
- 根级 `plan.md`、`progress.md`、`review.md` 只添加历史状态与现行入口，不改原发布结论，不搬移或删除。
- `docs/README.md` 区分现行索引、历史资料及执行约束，去掉重复历史索引项。
- 不改 `.trellis/tasks/archive/**`、`.gkd/archive/**` 旧正文、PENDING 或其他历史文件。

### 4.6 GKD 问题报告

GKD 仓库的独立报告已在拟案阶段形成；它不替换 GKD `.gkd/plan.md`。记录 G01-G04 的证据、触发场景、影响、建议和可判定验收，以及 G05 待澄清事项。源码已改善、安装版本尚未同步的问题单独标注。

报告只要求后续 GKD 维护者审议，不授权修复或安装。其他任务的 GKD dirty worktree 不清理、不提交、不纳入本 AIO PR，也不成为 AIO 文件修正的前置。

## 5. 文件级范围

### 5.1 AIO 修改清单

| 文件 | 允许改动 |
| --- | --- |
| `AGENTS.md` | GKD 优先的流程入口、项目差异、按需资料读取与验证阶段；不复制 GKD 实现 |
| `README.md`、`README_EN.md` | 开发/验证/贡献段落对齐；不改产品功能、截图或营销文案 |
| `docs/README.md` | 指令与事实来源、现行/历史入口、移除重复项 |
| `docs/operations/github-actions-governance.md` | 仅任务交接、提交、验证、归档段落；不修改 Actions 权限和发布算法 |
| `docs/operations/templates/simple-change-record.md` | 添加历史/停用说明和新入口；保留旧正文 |
| `.trellis/spec/guides/index.md` | 改值检索触发、按需阅读、去除无依据固定误报率数字 |
| `.trellis/spec/guides/code-reuse-thinking-guide.md` | 最小复用条件、限定检索、按实际需求抽象 |
| `.trellis/spec/guides/cross-layer-thinking-guide.md` | 去除重复和外部项目专属清单，保留边界原则与按需场景 |
| `.trellis/spec/aio-coding-hub/cross-layer/index.md` | 移除旧 runner/完整 base SHA 要求，质量清单按行为触发，引用 CI 分级 |
| `.trellis/spec/aio-coding-hub/backend/index.md` | 明确 Quality Check 按受影响合同触发，不改变业务合同要求 |
| `.trellis/spec/aio-coding-hub/cross-layer/cloud-only-verification-contract.md` | 对齐项目验证阶段、GKD 路由和文档检查器边界 |
| `scripts/check-cloud-only-verification.mjs` | 仅自然语言逐字断言及其直接相关注释 |
| `scripts/check-cloud-only-verification.selftest.mjs` | 对应正例/负例调整，保持已有实质门禁覆盖 |
| `plan.md`、`progress.md`、`review.md` | 仅历史状态横幅与现行入口，不覆盖其任务证据 |

实施中的 `.gkd/plan-changes.md`、`.gkd/execution.md`、`.gkd/progress.md`、`.gkd/review.md` 及本任务 `.gkd/archive/gkd-rule-alignment/` 按 GKD 实际阶段创建，不预填通过结论，不复制第二份活动 PLAN。

### 5.2 拟删除的 20 个空模板

批准实施包括以下逐项删除范围；实施前只复核这些文件的内容和引用：

| 目录 | 文件 |
| --- | --- |
| `.trellis/spec/plugin-sdk/backend/` | `index.md`、`directory-structure.md`、`database-guidelines.md`、`error-handling.md`、`quality-guidelines.md`、`logging-guidelines.md` |
| `.trellis/spec/plugin-sdk/frontend/` | `index.md`、`directory-structure.md`、`component-guidelines.md`、`hook-guidelines.md`、`state-management.md`、`quality-guidelines.md`、`type-safety.md` |
| `.trellis/spec/create-aio-plugin/frontend/` | `index.md`、`directory-structure.md`、`component-guidelines.md`、`hook-guidelines.md`、`state-management.md`、`quality-guidelines.md`、`type-safety.md` |

不以 `plugin`、`skill`、`template` 关键词删除产品文件。`src/templates/**` 是业务资产，明确不在本次修改范围。

### 5.3 明确非目标

- 所有业务实现、UI、API、数据库、迁移、生成绑定、插件运行时及产品模板。
- `package.json`、workspace manifests、依赖、锁文件、Tauri 配置。
- `.github/workflows/**`、`.github/ci-scope.json`、CI 分类器、GitHub Ruleset 与 Actions 配置。
- GKD Skills/角色/手册/现有任务记录、系统全局 AGENTS、非 GKD 用户级 Skills、安装目录。
- 发布、签名、版本号、Release、旧分支/worktree、Git 历史重写、其他任务的归档与现场。
- 逐字文档门禁以外的治理检查器重构、通用 YAML/Markdown parser、新工作流状态或代理体系。

## 6. 执行路线、目录和角色

本任务跨多份规则并修改 CI 合同检查器，用户现已明确选择 GKD `delegated/automatic`。main 使用 `agent_type=gkd_execute`、`fork_turns=none` 启动一个执行 session，不改为 direct-main 或通用 worker；角色不可用时保留现场并报告真实错误。

批准后的候选任务分支为 `chore/gkd-rule-alignment`，候选执行目录为主工作树旁 `../worktrees/gkd-rule-alignment`。main 先只读检查占用情况，从更新后的 `origin/main` 创建；命名占用时选择唯一后缀，既有 worktree 不复用、不清理。具体绝对路径和基线写入 execution，不进入长期归档。

main 维护本主工作树 PLAN/plan-changes/review，生成执行 worktree 的 execution。执行 session 只写第 5 节范围和其 progress；经本 PLAN 后续批准的本地提交许可写入 execution。main 不与执行 session 并行写实现、progress 或正在使用的 execution。

自动施工已获批；执行交接、验收和 CI 监控只使用 GKD 指定路线与角色，不以普通 worker/explorer 替代施工或验收。GKD 资源从当前安装位置加载，不从 AIO 假定存在 `.codex/agents` 或复制本地 Skill。

## 7. 执行顺序与更新点

1. main 复核已批准 revision、安装版 GKD、目标 Git 基线与占用；保留本地 main 独有历史。若 GKD 安装版变化，核对与本 PLAN 有关的差异，不自动安装源码。
2. main 复核已有任务 worktree，将 execution 更新为 r2，写清文件清单、命令、读取范围、本地提交授权和验证/交付边界，启动一个命名的 `gkd_execute`。启动成功只代表执行开始，不能称整个任务完成。
3. 执行 session 调整项目入口、运维/README 和索引，先消除 GKD 冲突和旧 runner 引用；在 progress 记录关键取舍和 GKD 报告关联。
4. 限定范围整理指南、空模板、历史标记；修改检查器的文案断言和相关已有自测。删除前复核模板内容，完成后检查活动引用。
5. 执行第 8 节本地验证，按结果修复范围内问题；完成消融审查，确保没有新抽象、第二套工作流或额外文档门禁。记录实际结果、未运行项与剩余风险。
6. 执行 session 按明确许可创建简短中文 Conventional Commit，完成后停止并交回 main。建议实现提交：`chore(规则): 对齐 GKD 流程与验证范围`。
7. main 审查 diff 与证据，按授权创建/更新任务 PR；普通自动 CI 由 `gkd_ci_monitor` 跟踪一个明确 PR，收到终态后继续。失败按 GKD 更新必要计划/交接，不能直接扩大范围。
8. 已批准 delegated 执行结束后，main 向 `gkd_accept` 交接主工作树 PLAN/变更记录、执行 worktree execution/progress、基线/head、CI 证据。main 根据验收结果写 review；返工必须完成再判断通过。
9. main 通过后加载 `gkd-closeout`，创建本任务脱敏归档，按批准范围准备、审查及提交实际清理差异；清理提交改变 PR head 后等待相应自动门禁，不把旧 head 的结果用于新 head。
10. 在第 9 节授权范围内完成 PR 合并、远端结果同步和符合 GKD 条件的本任务现场清理。若 GKD 未修复的收尾条件实际阻塞，只停止受阻步骤并报告证据，不在 AIO 增加绕过机制。

progress 更新点为：规则归属判断完成、模板清理与文案门禁调整完成、本地验证终态、交接阻塞或范围变化。主代理负责 CI、验收、集成与清理的最终记录。

## 8. 验证计划

下表是批准实施后的检查，不代表本轮已运行。所有本地 Node 检查使用现有入口、内建模块且不得写文件或启动被禁止工具；不通过 package-manager 包装执行。

| 检查 | 命令或方式 | 通过标准 |
| --- | --- | --- |
| 初始与最终范围 | `git status --short`、`git diff --name-status`、`git diff --stat`；提交后用审查基线比较 | 变更只在第 5 节及本任务 GKD 记录内，无业务/CI 策略/全局 Skill 改动 |
| 格式 | `git diff --check`，暂存后 `git diff --cached --check` | 无本任务引入的空白错误 |
| 治理脚本语法 | `node --check scripts/check-cloud-only-verification.mjs`；同命令检查 selftest | 两个文件语法有效，无执行副作用 |
| 实际 cloud-only 合同 | `node scripts/check-cloud-only-verification.mjs` | 新项目规则通过，实质守卫和 job 合同保持 |
| 治理回归 | `node scripts/check-cloud-only-verification.selftest.mjs` | 等义文案正例通过；必要入口缺失、禁止命令和实质门禁破坏仍失败 |
| spec 链接 | `node scripts/check-spec-links.mjs` | 删除模板后活动规范链接仍有效 |
| 本次文档链接 | 逐项检查第 5 节修改文件中的新增/修改本地链接与锚点 | 不产生新断链；不以本任务扩大为全仓历史链接修复 |
| 旧规则与引用 | 在第 5 节文件及其已知索引中定向 `rg` 检索旧 runner、完整 base SHA、ANY value、空模板路径及固定双域要求 | 活动指令不再要求已移除入口；历史标识不是可执行指令 |
| 条款场景走查 | 只读审查、简单 direct-main、delegated、文档改动、单域行为、shared 门禁变更、范围内修复、材料性偏差、已授权交付 | 每个场景能定位 GKD 路线、必要资料、实际检查和交付终点；项目不加重复审批 |
| GKD 报告 | 检查引用、引文、问题归属和源码/安装差异 | 报告可独立审阅，不把非 GKD 问题或在途修复误归属 |
| 自动 PR 门禁 | 由 GKD 监控明确 PR；普通 `ci-gate` 与 `pr-title` | 对应当前 PR head 的门禁成功；本次脚本改动应按现有策略跑完整 CI |

不运行本地 Vitest、Playwright、Cargo、lint/typecheck、构建或桌面打包；不新增业务单元测试、E2E，也不为规则修改创建若干仅用于演示分类的真实 PR。没有修改分类器，现有自动 CI 已提供本次应有的覆盖。

检查失败时先区分本次引入与既有问题。前者在批准范围内修复；后者记录具体证据及对 AC 的影响，不自动清理全仓。重新验证只覆盖新修复影响的检查；提交 head 改变后的 required checks 必须等待新的有效结果。

## 9. 成功标准、授权与收尾

### 9.1 可判定 AC

| AC | 成功条件 | 证据 |
| --- | --- | --- |
| AC-01 | 项目工作流入口优先引用 GKD，direct-main/delegated 不混用，不出现项目替代角色或授权例外 | AGENTS/运维/README diff 和场景走查 |
| AC-02 | 活动规范无固定旧 runner、完整 base SHA 绑定及无条件双域检查要求 | 定向扫描与合同检查 |
| AC-03 | 阅读、检索和质量清单按所改行为触发，保留跨层及高风险行为保障 | 两个业务规范索引和三个 guides 的 diff |
| AC-04 | 明确空模板已删除或因出现有效内容而说明保留；旧模板及根级发布记录不再充当新任务入口 | 删除清单、历史标记、引用检查 |
| AC-05 | 自然语言等义改写可通过 checker；实际本地/CI 边界及必要 GKD 入口的负例仍有效 | 本地 checker/selftest 和自动 CI |
| AC-06 | GKD 自身问题在其目录有独立报告，源码/安装版本与非 GKD 来源区分清楚 | GKD 问题报告；无 Skill/角色改动 |
| AC-07 | AIO 实施 diff 无业务代码、依赖、workflow/classifier/Ruleset 或旧历史正文变更 | 范围审查 |
| AC-08 | 本次允许的本地验证和当前 PR head 的 required checks 通过，主代理及 delegated 验收通过 | progress、CI 终态、review |
| AC-09 | 若批准下述完整交付范围，任务 PR 合并，归档完整，按当前 GKD 条件完成或如实报告受阻清理 | 实际 merge SHA、归档、Git 状态与保留项 |

实现完成、CI 通过、验收通过、PR 合并和现场清理分别报告。GKD 自身报告只需落盘，不要求本任务修好 GKD；其他仓库的在途工作不纳入 AIO 完成条件。任何必要收尾条件实际未满足时，不宣称整个任务全部完成。

### 9.2 拟案阶段授权

- 只读调查、创建本 PLAN、在 GKD 目录新增独立问题报告。
- 拟案阶段不提交；批准后 AIO 本任务记录按第 9.3 节交付。GKD 报告与其其他在途改动仍不由本任务提交。

### 9.3 已批准的实施与完整交付范围

用户在审阅本 PLAN 后明确要求“开始按 PLAN 执行”，以下提案已转为本任务授权边界。按表执行，不在每次切换 Skill 或到达已授权阶段重新询问；表中明确不授权的动作保持不授权。

| 动作 | 授权边界 |
| --- | --- |
| AIO fetch、任务分支/worktree | 只从更新的 origin/main 创建本任务现场；保留原 main 独有历史 |
| 实施与本地提交 | 第 5 节范围、第 8 节零依赖检查；execution 明确执行 session 的本地提交许可 |
| 推送与创建/更新 PR | 只操作本任务分支，GitHub 使用显式仓库参数；不 push main，不自动扩大修复范围 |
| 自动施工、CI 监控与独立验收 | 自动施工已获用户明确选择，使用 GKD 命名执行角色；监控单一明确目标，不手动 dispatch 常规 ci；执行完成后才独立验收 |
| 合并 | 本任务 PR 当前 head 的 required checks、独立验收和 main 审查通过后，按项目 squash 合并；禁止管理员绕过 |
| 本任务归档与 cleanup commit | 仅本任务 GKD 记录和必要脱敏快照；实际清理差异先审查并走对应 PR 门禁 |
| 删除本任务现场 | 仅来源明确、已合并、停止写入且符合当前 GKD closeout 条件的任务分支/worktree；不删除其他对象 |
| GKD 问题报告 | 本轮已新增；不暂存/提交 GKD 的在途修改，不修 Skill、不安装 |
| 发布及远端设置 | 不授权；本任务不发版、不改 Ruleset/Secrets/Actions 设置 |

本任务现由 main 启动 execution session，不再等待用户手动开启施工会话。执行 session 停止后，main 继续已获批准的审查、CI、验收和交付；不因切换路线重新申请同一授权。

### 9.4 风险与停止条件

- GKD 源码与安装版存在差异：报告事实，不安装未完成源码；实际执行遵循当前可用版本和本次明确授权。
- GKD closeout 的未修复条件若阻塞实际收尾，记录问题编号、受阻动作及现有证据，保留现场，不以项目规则覆盖它。
- 模板中出现新内容、治理门禁需要修改额外符号、业务代码或 CI 分类策略时，main 判断材料性变化并按 GKD 修订 PLAN；不按旧清单强制删除。
- 当前 main 独有历史不能通过自动 reset 清除。合并后先比较文件差异；无法无损同步时保留原 checkout，报告远端已合并事实与本地保留状态。
- 发现并发 writer、来源不明修改、PR head 漂移、远端状态不明或授权实际不足时，停止受影响动作并保留证据；不伪造检查通过或全部完成。
- 不因暂未授权发布、没有桌面运行证据或其他仓库 dirty 状态，额外要求本次规则修正执行无关验证。

## 10. 消融审查

实施和验证结束后，main 逐项确认：没有第二套 GKD 生命周期、没有新本地 runner/解析器/状态文件、没有为了删模板而补空模板、没有因文案变更新增业务测试、没有把 GKD 问题修到 AIO、没有要求每次通读资料、没有新增重复确认点。

保留下来的每条强制规则应能说明适用任务、保护的实际行为和可观察证据。没有这三者且属于本次修改范围的新增要求应移除。

## 11. 拟案完成与当前阶段

拟案已完成并经用户批准；r2 只根据用户明确选择把执行路线切换为自动施工，不改变 r1 的技术方案、文件范围、验证和完整交付边界。main 启动执行 session，随后完成主代理负责的验证审查、独立验收与授权交付。
