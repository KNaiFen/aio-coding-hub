> 归档观察：2026-09-07 02:54:17 +08:00。来源：主工作树 `.gkd/plan.md`；记录作者：main。正文保留原作者的状态和观察时点，本机路径与账号已脱敏；后续收尾事实见同目录 `summary.md` 及收尾返回。

# 项目规则以 GKD 为准

- 日期：2026-09-07
- 版本：r1
- 状态：r1 已批准并完成实施；独立技术验收和 main 审查通过，待获准本地归档与提交收尾。
- 主工作树：`<主工作树>`

## 目标与用户决定

清理仓库现行规则中与用户级 `$gkd-main` 冲突、重复或另设门槛的开发工作流要求，只保留 AIO 项目信息和必要环境约束。生命周期、路线、角色、授权、提交安排、验证安排、验收和收尾由 GKD 及具体任务的获批方案决定。

用户明确要求：

1. 方案阶段仅沟通需求和出 PLAN；用户随后明确批准执行本 r1，现进入实施阶段。
2. 清理范围包括项目自定的提交方式、提交前必须运行什么、固定手动交接和固定 worktree 等规则。
3. 不修改 CI、PR 的 GitHub Actions 工作流。
4. 保留“禁止本地运行会生成大量文件产物或持续占用 CPU 的检查”这一约束。轻量、短时检查按 GKD 方案确定，不再扩大为一律禁止所有本地检查。

## 现状证据

- 本地主工作树 `main` 为 `193767510ef647193ce5f16390bc1f663c3dffb0`，调查开始时无未提交修改。
- 本地已知 `origin/main` 为 `a35f2aa4ea469f6e4066582b2e969f1ec44fca2e`。该提交已做过一次规则对齐；本地与远端跟踪分支既有历史分叉，也有实际文件差异。本轮没有 fetch，以上不代表已核实 GitHub 最新 head。
- 本地 `AGENTS.md:3`、`:8` 固定 manual-first 和独立 worktree；`:24` 固定中文 Conventional Commit 与 PR 集成；`:35` 一律禁止多种本地工具。远端跟踪版本仍指定“默认手动交接，自动执行须由用户明确选择”，并重复任务分支、提交、验收与收尾约束。
- `README.md:193`、`README_EN.md:199`、`docs/README.md:42`、`:68`、`docs/operations/github-actions-governance.md:25` 重复执行、提交和验证规则。远端跟踪版本仍含 Before committing / 提交前要求。
- 本地 `.trellis/spec/aio-coding-hub/cross-layer/index.md:242` 要求使用固定本地 runner，且不允许单独选择 checker、语法或 diff 检查；远端版本已去掉 runner，但仍规定提交前检查时机。backend 索引的远端版本也新增了同类提交前文字。
- `.trellis/spec/guides/` 保存通用开发/审查清单和 Trellis 任务流程；本地版本还包含外部项目模板、重复章节和固定提交前清单。它们不属于 AIO 产品或模块合同。
- `docs/operations/templates/simple-change-record.md` 是第二套任务计划/执行/收尾模板；远端已标记停用，本地尚未标记。
- `.trellis/spec/plugin-sdk/` 和 `.trellis/spec/create-aio-plugin/` 本地共有 20 份待填通用模板，远端跟踪版本已删除。
- `scripts/check-cloud-only-verification.mjs:513` 起强制 AGENTS 包含 GKD 入口和四个交接文件名，并拒绝特定旧流程词；selftest 将缺少交接文件名判为失败。本地还要求零产物和手动 CI 的固定句子，远端已移除固定句子断言。
- root/workspace package scripts 实际带有 `scripts/require-github-actions.mjs` 环境 guard；本次保留此工具行为。`.github/ci-scope.json` 将 `scripts/*.mjs` 判为 shared source，因此修改检查器后的 PR 会按现有规则运行两端 CI。
- 当前没有 `.gkd/` 活动 PLAN/review/execution/progress；根级 `plan.md`、`progress.md`、`review.md` 是 0.60.58 已结束发布记录，不复用。

## 改法与文件范围

### 1. 精简项目入口

修改 `AGENTS.md`，保留一条 GKD 权威入口和必要项目事实：仓库身份、远端信息、项目知识库/模块合同入口、工具环境限制和本地资源约束。

删除项目自行规定的 manual-first、固定路线、强制 worktree、分支建立/提交格式/提交时机/合并方式、执行者和验收者分工、记录文件清单、归档与清理步骤，以及重复的授权门禁。不得把旧清单换成另一套自制 GKD 清单，也不复制 Skill 正文。

本地资源约束保留为：禁止本地执行会产生大量依赖、缓存或构建文件，或持续高 CPU 占用的检查；依赖安装、完整测试/覆盖率、编译、打包和长时性能检查继续使用现有云端环境。GKD 方案可选择必要的轻量、短时检查。保留 package scripts 仅可在 GitHub Actions 运行这一事实，不绕过其 guard，不为资源约束新增监控脚本或数值阈值。

### 2. 清理现行文档中的重复流程

允许修改：

- `README.md`、`README_EN.md`：贡献和验证部分仅引用 GKD，去掉提交格式、固定分支/合并步骤和交接文件说明；保留功能、安装、支持矩阵、Actions/发布事实及资源约束入口。
- `docs/README.md`：明确项目信息与 GKD 流程的关系，移除项目自定的提交前要求和任务记录步骤；保留知识库分类、现行合同、历史资料入口和文档状态信息。
- `docs/operations/github-actions-governance.md`：仅清理“提交与发版”中混入的代理计划、提交、worktree、归档步骤。保留现行 Actions 触发、job、required checks、PR 标题校验、版本标签与候选制品晋升事实；保留现有锚点或同步修正受影响链接。
- `.trellis/spec/aio-coding-hub/cross-layer/cloud-only-verification-contract.md`：删除代理生命周期及提交前规则，改为项目工具/云端验证合同；保留包 guard、Tauri hook、CI job、候选制品和漂移处理的实际合同。
- `.trellis/spec/aio-coding-hub/backend/index.md`、`.trellis/spec/aio-coding-hub/cross-layer/index.md`：移除固定 runner 和通用提交前门禁；保留 AIO 模块行为、不变量及相关回归场景。必要验证由 GKD 根据受影响行为选择，不能将模块清单变成每个任务必跑的全量检查。
- `docs/plugins/developer-guide.md`、`docs/plugins/runtime/README.md`：仅整理仓库本地验证限制的入口文字，保持插件作者工具用法、SDK/API 和现有 CI 说明。
- `.trellis/spec/aio-coding-hub/cross-layer/codex-config-contract.md`、`config-migration-skill-bundle-contract.md`：仅在本地验证入口存在重复流程文字时精简引用，不改业务合同和生成绑定职责。

### 3. 移除无项目信息的通用流程材料

- 删除 `.trellis/spec/guides/index.md`、`code-reuse-thinking-guide.md`、`cross-layer-thinking-guide.md`、`upstream-merge-scope-guide.md`。其中 AIO fork 与 upstream 的项目事实由已有 README、产品文档和 Actions 治理文档保留；不保留 Trellis follow-up、强制阅读、通用审查或合并任务流程。
- 删除 `docs/operations/templates/simple-change-record.md`，不再维护另一套任务模板。
- 不恢复远端已删除的 `.trellis/spec/plugin-sdk/` 与 `.trellis/spec/create-aio-plugin/` 20 份空模板；若最终获批基线仍含这些模板，仅删除确认无项目事实的模板。
- 更新受影响的现行相对链接。历史正文、历史 JSONL 中的旧路径保留为当时事实，不批量改写任务历史，不为旧流程建立兼容入口。

### 4. 解除检查器对代理流程文本的绑定

仅修改 `scripts/check-cloud-only-verification.mjs` 与 `scripts/check-cloud-only-verification.selftest.mjs` 的文档流程断言：

- 移除 AGENTS 必须列出四个交接文件名、旧生命周期词黑名单、固定零产物句子及其他代理流程字串断言；不新增 GKD 文案白名单或自制流程解析器。
- 不再要求检查器读取 AGENTS 来校验代理生命周期；无其他用途时移除该 fixture 字段。
- 删除与上述规则一一对应的旧 selftest。用一个聚焦正例确认仅由 GKD 决定交接文档不会触发云端合同错误；复用其余既有负例证明包 guard、Tauri 和 Actions 门禁仍有效。
- README/active specs 对不可用 package/native 命令的说明仍须符合保留的工具事实，不借此恢复重型本地检查。
- CI workflow、分类器、required checks、job 选择、测试/构建命令和发布校验均保持原行为。这个脚本改动只解除项目规则文本与外部 GKD 实现的耦合。

## 非目标

- 不修改 `.github/**`、package manifests、锁文件、Tauri 配置、工具链、依赖、业务源码或发布版本。
- 不调整 Actions 工作流、PR 标题规则、CI 分类/触发、签名、候选制品、发布或远端保护设置。
- 不修改用户级 Skills、Codex 配置、全局 AGENTS 或其他仓库。
- 不清理历史任务、根级旧发布记录、既有归档、来源不明的分支/worktree/文件。
- 不整理本地主分支历史，不 reset/rebase/强推，不把 ahead 数量当作未交付功能数量。
- 不为本次文档清理新增通用校验框架、任务模板、运行状态或持久脚本。

## 实现路线与基线

用户已批准本 PLAN；2026-09-07 执行 `git fetch origin` 成功，`origin/main` 仍为 `a35f2aa4ea469f6e4066582b2e969f1ec44fca2e`，无新增范围变化。

使用获批的 `delegated/automatic`：规则跨多个文档入口，且需要移除 CI 调用的检查器中的流程断言，独立执行上下文与验收有实际收益。main 从获批 PLAN 生成 `.gkd/execution.md`，使用配置中的 `gkd_execute`；执行停止后使用 `gkd_accept`，main 决定审查结果，再交给一个 `gkd_closeout`。不继承仓库旧的 manual-first 限制。

执行目录为 sibling worktree `../gkd-project-rules`，任务分支 `chore/gkd-project-rules`，基线为上述已 fetch 的 `origin/main`。如果远端新增变化改变本方案范围或决定，main 说明影响后修订；普通定位和等价差异按 GKD 处理。

现有主工作树和本地 `main` 历史保留原状。基于远端已完成的清理不重复施工，不把本地旧文件恢复到任务分支。本次不顺带修复本地历史分叉；交付时明确项目规则位于任务分支/worktree 还是已合并远端。

## 验证与验收标准

在执行 worktree 使用系统现有 Node 和 Git，直接运行下列获批的短时、零依赖、不写产物的检查。

| 检查 | 通过条件 |
| --- | --- |
| `git diff --check` | 无本次引入的空白错误。 |
| `node scripts/check-cloud-only-verification.mjs` | 精简后的文档与保留的项目工具/云端合同兼容。 |
| `node scripts/check-cloud-only-verification.selftest.mjs` | 文档流程解绑正例及既有包/Tauri/Actions 负例通过。 |
| `node scripts/check-ci-quality-gates.mjs` | 现有 CI 门禁结构和命令合同继续通过。 |
| `node scripts/check-spec-links.mjs`、`node scripts/check-markdown-links.mjs` | 不新增断链；既有问题须以基线证据区分，不改无关历史内容来消除失败。 |
| Git diff 与定向文本复核 | 实现 diff 限于批准路径；`.github/**`、业务源码、包 guard 等受保护内容无改动；现行入口无固定手动路线、固定 runner、提交格式/时机或第二套任务模板。 |

任何检查若实际需要安装依赖、产生大量文件或持续高 CPU 占用，停止该检查并记录原因，交由现有云端路径验证，不在本地扩展执行。

验收需同时满足：

1. `AGENTS.md` 只包含 GKD 引用、AIO 项目信息和获准本地资源约束；没有复制 GKD 生命周期实现。
2. 现行文档不再规定提交格式、固定路线、固定工作树、固定提交前检查、角色分工或归档步骤；通用模板/指南不再作为项目规则存在。
3. 本地大量产物/持续 CPU 约束完整保留，轻量检查可按 GKD 方案选择，已有 Actions 环境 guard 保持有效。
4. 产品、模块、协议、插件、支持矩阵和发布事实保留；未将业务合同里的请求 commit / 事务 commit 误判为 Git 提交规则。
5. 云端合同检查不再强制项目复制 GKD 文件名或生命周期词汇；包与 Actions 真实行为的既有校验保持有效。
6. `.github/**` 和 CI 分类逻辑零变更，自动 CI 仍按现有分类运行；不以本地检查通过冒充远端 CI 通过。
7. main 审查含消融检查：移除本次新增但不必要的解释、抽象、模板或重复引用。

## 交付、授权与记录

- 用户已明确批准执行本 r1，许可创建上述任务 worktree/分支、执行范围内改动和轻量验证、GKD 验收/收尾与本任务记录归档整理。
- 实施终点：范围内改动、必要轻量验证、独立验收和 main 审查通过，完成本地任务分支提交与 GKD 记录归档，提供可审阅差异。
- 本地提交沿用用户“每个完成任务单独提交、简短中文说明”的会话规则。这是本次交付安排，不写回项目成为新的提交约束。
- 推送、创建 PR、合并、发布和删除任务 worktree/分支尚未授权，不擅自执行，也不作为本地交付的门禁。后续若授权 PR，等待现有自动 `ci-gate`、`pr-title` 及被选中的检查；不手动重复触发常规 CI。
- 收尾由一个 `gkd_closeout` 根据 main 交接，在任务分支归档本任务 PLAN/review/execution/progress 中实际产生的必要 Markdown 到 `.gkd/archive/project-rules-gkd/2026-09-07-r1/`，脱敏本机路径，不制造空记录。
- 归档确认后仅移出本任务已结束的活动正文；保留本次执行 worktree/分支供本地交付，保留原本地 `main` 与所有无关历史。
