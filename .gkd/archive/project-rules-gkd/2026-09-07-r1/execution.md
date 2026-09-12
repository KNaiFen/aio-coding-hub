> 归档观察：2026-09-07 02:54:17 +08:00。来源：执行工作树 `.gkd/execution.md`；记录作者：main。正文保留原作者的状态和观察时点，本机路径与账号已脱敏；后续收尾事实见同目录 `summary.md` 及收尾返回。

# 执行交接：项目规则以 GKD 为准

- execution revision：r1
- 来源：主代理 PLAN r1，用户已明确“批准执行”。
- 路线：delegated/automatic。
- 执行 worktree：`<执行工作树>`
- 分支：`chore/gkd-project-rules`
- 基线：`a35f2aa4ea469f6e4066582b2e969f1ec44fca2e`，2026-09-07 fetch 后的 `origin/main`。

## 授权和职责

你是唯一实施 writer。所有命令和路径显式指向本 worktree；主工作树另有独立 PLAN，不能把 cwd 当作本 worktree。本文件和适用 AGENTS 是施工来源，无需读取主工作树 PLAN、历史任务或用户级生命周期实现。

用户已批准清理项目规则，旧 AGENTS 的固定手动交接、分支/提交/验证步骤正是本次删除对象，不能用它们否定本交接的明确执行授权。保留系统/开发者指令和本交接约束。面向用户和 main 的报告使用中文；代码和英文文档保持原有语言。不得派生子代理。

只修改下述范围内实现和 `.gkd/progress.md`；不修改 execution、PLAN、review，不验收、不归档、不清理活动记录、不推送、不创建 PR、不合并、不发布。本阶段不作 Git 提交，实施差异保留给验收；main 审查通过后收尾角色统一归档并作本任务本地提交。

你并非独占整个仓库：主代理维护主工作树记录；不撤销其他人的修改，不改原本地 main 的历史或文件。已有工作与任务无关时保留，材料性冲突回报 main。

## 目标

现行项目规则仅保留 AIO 项目信息和必要环境约束，工作流以用户级 `$gkd-main` 为准。不再自行规定路线、角色、授权、提交格式/时机、分支/合并方式、提交前检查清单、固定 runner、交接文件清单或归档步骤，也不把旧清单换成自制 GKD 清单。

用户明确保留：禁止本地执行会生成大量文件产物或持续高 CPU 占用的检查。依赖安装、完整测试/覆盖率、编译、打包和长时性能检查继续使用现有云端环境；必要轻量、短时检查按 GKD 获批方案决定。保留 package scripts 仅在 GitHub Actions 可运行这一现有工具事实，不绕过环境 guard，不新增资源监控工具或数值阈值。

## 已核实的定位和技术方案

### 项目入口

精简 `AGENTS.md`：一条 GKD 权威引用，加项目仓库身份、默认 origin、GitHub 仓库 `<仓库所有者>/aio-coding-hub`、项目知识库/模块合同入口、工具环境事实和上述资源约束即可。保留必要 AIO 信息；移除通用流程、重复保护/授权和固定角色分工。远端基线仍有“默认手动交接，自动执行须由用户明确选择”等残留。

### 文档

允许修改这些文件中与本目标有关的段落：

- `README.md`、`README_EN.md`：清理贡献、验证和任务说明中的提交格式、固定分支/合并步骤与 GKD 交接细节，仅引用 GKD。保留产品、安装、支持矩阵、Actions/发布事实和资源约束入口。
- `docs/README.md`：保留知识库分类、事实与历史资料入口、文档状态；删除提交前要求、任务流程/文件清单和重复执行规则。
- `docs/operations/github-actions-governance.md`：仅删除“提交与发版”中代理计划、提交格式/时机、worktree 和归档规则。保留现行 Actions 触发、job、required checks、PR 标题校验、版本标签与候选制品晋升事实。保留已有标题锚点或修正允许范围内现行链接。
- `.trellis/spec/aio-coding-hub/cross-layer/cloud-only-verification-contract.md`：删除 GKD 生命周期、固定交接文件、提交前规则，成为项目工具/云端验证合同；保留资源约束、包 guard、Tauri hook、CI job、候选制品和生成漂移的实际合同。同步更正 selftest 所有权描述。
- `.trellis/spec/aio-coding-hub/backend/index.md`、`.trellis/spec/aio-coding-hub/cross-layer/index.md`：删除固定 runner 和通用提交前规则；保留模块行为、不变量及按受影响行为适用的回归场景。不要将模块检查清单变成所有任务全量必跑。
- `docs/plugins/developer-guide.md`、`docs/plugins/runtime/README.md`：仅整理仓库本地验证限制入口文字，保留插件作者工具用法、SDK/API 和既有 CI 说明。
- `.trellis/spec/aio-coding-hub/cross-layer/codex-config-contract.md`、`.trellis/spec/aio-coding-hub/cross-layer/config-migration-skill-bundle-contract.md`：仅在本地验证入口存在重复流程文字时精简引用，不改业务合同和生成绑定职责。

### 删除通用模板

删除以下无 AIO 项目信息的通用流程材料：

- `.trellis/spec/guides/index.md`
- `.trellis/spec/guides/code-reuse-thinking-guide.md`
- `.trellis/spec/guides/cross-layer-thinking-guide.md`
- `.trellis/spec/guides/upstream-merge-scope-guide.md`
- `docs/operations/templates/simple-change-record.md`

现有 README/产品文档/Actions 治理文档已保存 fork 与 upstream 项目事实，不保留 Trellis follow-up、通用合并流程/提交检查清单。现行相对链接有受影响者在允许范围内同步处理；范围外发现需改的链接先回报。历史正文和 JSONL 中的旧路径只作当时事实，不批量改写、不建兼容入口。已知历史 markdown 仅有一处代码格式路径引用，不是 markdown 超链接。

`.trellis/spec/plugin-sdk/` 与 `.trellis/spec/create-aio-plugin/` 的 20 份空模板在本基线已经删除，不重复处理、不从主工作树复制旧版本。其他历史目录、根级 plan/progress/review 和既有归档均保留。

### 检查器解绑

仅修改 `scripts/check-cloud-only-verification.mjs` 和 `scripts/check-cloud-only-verification.selftest.mjs` 中的文档流程断言：

1. 移除 AGENTS 的 GKD 入口/四个文件名字串强制、旧生命周期词黑名单及代理流程字串断言；README 中的旧生命周期词黑名单也属于本范围。无其他用途时移除 AGENTS fixture 读取/字段，不新增 GKD 文案解析器或白名单。
2. 删除对应的旧 selftest；用一个聚焦正例验证精简的 GKD 引用与云端合同兼容，不增加空洞或复制实现的测试。保留真实包/Tauri/Actions 合同的既有负例。
3. 保留 README/active specs 对不可用 package/native 命令的工具事实校验及其负例。固定零产物/手动 CI 句子断言已在本基线移除，不恢复。
4. 保留 CI workflow、分类器、required checks、job 选择、测试/构建命令、发布校验、包 guard、Tauri hook 的全部实际行为，不改其他 checker 或 selftest。

检查器从约 512 行开始校验 AGENTS；loadCloudOnlyVerificationFixture 和 assertCloudOnlyVerificationContract 包含该字段。先读即将修改的确切代码及已有测试；`.codegraph/` 若存在按适用规则先查询，否则无需创建索引。

## 不可越界

- 不改 `.github/**`、package manifests、锁文件、Tauri 配置、业务源码、版本、其他脚本、用户级 Skills/配置。
- 不重写 Git 历史，不 reset/rebase/强推，不改远端或现有 main。
- 不安装依赖，不运行 package manager、Cargo、Tauri、开发服务器、完整 lint/typecheck/test/build、签名、打包或长时性能检查。
- 不记录真实凭据、完整对话、全量日志或未脱敏用户数据。
- 不做安全漏洞调查，本任务只处理工作流规则。

## 验证环境、命令和通过条件

使用系统已有 Node、Git；以下是已批准的直接零依赖检查，允许不经 package manager 运行。先核对脚本的依赖/写入范围，若实际会产生大量文件或持续高 CPU 则停止并回报，不能改成重型本地替代方案。

| 命令 | 通过条件 |
| --- | --- |
| `git diff --check` | 无新增空白错误。 |
| `node scripts/check-cloud-only-verification.mjs` | 精简文档与真实工具/云端合同兼容。 |
| `node scripts/check-cloud-only-verification.selftest.mjs` | 流程解绑正例和既有包/Tauri/Actions 负例通过。 |
| `node scripts/check-ci-quality-gates.mjs` | 现有 CI 门禁结构和命令合同通过。 |
| `node scripts/check-spec-links.mjs` | 不新增规范链接损坏。 |
| `node scripts/check-markdown-links.mjs` | 不新增 Markdown 链接损坏。 |

另允许只读 Git diff/status、文本读取/定向检索核对范围、资源约束和流程残留。没有已运行的实现测试结果可复用。链接检查若报已有问题，使用基线文件/Git 证据说明，保留准确失败状态，不为消除失败修改无关历史。

已有测试成功后不重复全量运行；新增改动、失败或证据冲突时才重测受影响部分。本次不推送、不跑 GitHub CI；不能宣称远端 CI 成功。按现有分类器，脚本改动将来提 PR 时会选中两端 CI，这是保留行为，不是优化对象。

## 完成与 progress

完成前自行做消融检查：是否新增不必要解释/模板/抽象、是否又复制 GKD 细则、是否误把请求/事务 commit 当 Git 提交、是否误改产品或 Actions 合同。

在 `.gkd/progress.md` 保存 execution r1、基线/实际 HEAD、改动范围、关键判断、真实命令/结果/环境、未覆盖范围和剩余风险。命令输出只保存必要摘要。明确业务与 `.github/**`/包 guard 未变、未运行重型本地检查、未作提交。

完成后停止并向 main 集中报告变更、验证、未决事实和停止状态，供独立验收。执行角色不作最终验收或交付。
