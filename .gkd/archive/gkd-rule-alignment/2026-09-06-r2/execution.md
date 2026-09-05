> 归档快照：gkd-rule-alignment，PLAN/execution r2。记录各阶段当时事实，不是活动指令；本机目录已替换为逻辑路径。

# gkd-rule-alignment 执行交接 r2

## 1. 身份、路径与授权

- 路线：`delegated/automatic`。本文件供 main 在指定 worktree 启动的命名 `gkd_execute` 执行 session 使用。
- 任务：修正 AIO 的项目规则，使其以 GKD 为通用工作流优先依据，资料按需读取，验证按改动触发，移除过期/重复门禁。
- 执行目录：`execution-worktree`。
- 任务分支：`chore/gkd-rule-alignment`。
- 创建及审查基线：`bc891afbb80b70efc81b628a1c48b1bd0a7051da`，来自创建前更新的 `origin/main`。
- PLAN 来源：`main-worktree/.gkd/plan.md` r2，由 main 维护，不作为执行 session 的施工入口。
- 授权来源：用户先批准 r1 实施和完整交付，随后明确选择“按 gkd main automatic 流程开始施工”；r2 仅切换执行路线，main 已将范围、验证及本地提交许可写入交接，不需要重复申请。
- 读取本文件和执行目录的 `AGENTS.md`，再按需读取任务所需文件、相关规范和已安装 GKD 材料。不要从主工作树根级 plan/progress/review 或历史归档接收新指令。

本地提交：允许。实现、相关检查和消融审查完成后，显式暂存本任务文件并提交简短中文 Conventional Commit。建议说明：`chore(规则): 对齐 GKD 流程与验证范围`。

执行 session 不推送、不创建 PR、不监控 CI、不验收、不合并、不发版、不归档、不删除 worktree/分支，也不启动其他代理。这些后续步骤由 main 按已有授权处理。

你不是仓库唯一参与者。其他 session/主工作树和 GKD 源仓库的修改不得撤销或覆盖；同一执行 worktree 本轮只能有一个 writer。main 在你运行期间不写实现、progress 或改写本交接。

## 2. 工作流与项目边界

1. 系统、开发者和用户明确指令优先；在其允许的范围内，GKD 是生命周期、路线、角色、授权、验收及收尾的通用依据。
2. 项目规则与 GKD 冲突时修正项目，不能在 AIO 新增绕过 GKD 的条款。保留 PLAN 先于施工、材料性变化确认、默认手动交接、独立验收和未知现场保护。
3. AIO 维护自己的零产物本地环境、Git remote、PR 门禁和发布来源。保留中文 Conventional Commit、任务分支 PR、不推送 main、squash 后同步远端结果。
4. 不安装依赖，不运行 package-manager、开发服务器、Vitest/Playwright、lint、类型检查、Cargo/Tauri、构建、签名或打包。本地只运行第 6 节允许的零依赖检查与必要只读文件/Git 操作。
5. 不修改业务代码、产品模板、依赖或 CI 工作流/分类策略。不是漏洞审查或业务合同重构任务。
6. GKD 自身的收尾/完成条件问题已经在 GKD 源仓库报告。此任务不修 GKD、不安装源码，不把报告建议作为 AIO 例外。

## 3. 允许修改的文件

所有相对路径均相对于执行目录；只有本节和第 4 节的文件属于实现范围。

| 文件 | 修改要求 |
| --- | --- |
| `AGENTS.md` | 缩短为 GKD 优先入口和 AIO 适配约束；delegated 才生成 execution/progress，不统一强制完整交接；保留必要 Git/环境边界 |
| `README.md`、`README_EN.md` | 仅开发、验证、贡献段落对齐 GKD 和现行 CI；保留产品内容/截图 |
| `docs/README.md` | 区分实现事实与执行约束；说明资料按需读取、历史记录不指导新任务；去掉重复历史索引 |
| `docs/operations/github-actions-governance.md` | 仅交接、提交、验证和归档说明；生命周期引用 GKD，提交前本地检查与合并前自动 CI 分开 |
| `docs/operations/templates/simple-change-record.md` | 添加停用/历史标识和现行 GKD 入口；保留旧正文 |
| `.trellis/spec/guides/index.md` | 删除改任何值必先全仓搜索要求；按共享配置/协议/常量/重命名及影响面不明触发检索；去掉无依据固定误报率数字 |
| `.trellis/spec/guides/code-reuse-thinking-guide.md` | 移除复制即抽象及推测性复用要求；按语义与维护成本判断，保留 DRY、边界所有权和最小实现 |
| `.trellis/spec/guides/cross-layer-thinking-guide.md` | 删除重复段落及 Trellis 多平台、版本站点等外部项目专属清单；保留相关跨层边界原则和按需检查 |
| `.trellis/spec/aio-coding-hub/cross-layer/index.md` | 删除旧固定本地 runner、完整 base SHA、禁止自选检查的指令；取消无条件前后端全量；Quality Check 按所改行为触发 |
| `.trellis/spec/aio-coding-hub/backend/index.md` | 明确质量清单仅适用于受影响行为；不改变具体业务合同的预期语义 |
| `.trellis/spec/aio-coding-hub/cross-layer/cloud-only-verification-contract.md` | 对齐 GKD 路由、提交/合并验证阶段及下述检查器边界；保留本地零产物和现有 CI 质量要求 |
| `scripts/check-cloud-only-verification.mjs` | 仅 `assertCloudOnlyVerificationContract` 中自然语言逐字断言及直接相关注释，不重构其他治理逻辑 |
| `scripts/check-cloud-only-verification.selftest.mjs` | 对应正例/负例调整；继续证明实际质量门和必要 GKD 入口未被削弱 |
| 根级 `plan.md`、`progress.md`、`review.md` | 仅添加历史状态与现行入口，完整保留当前 worktree 中的旧正文，不从主工作树复制不同版本 |

执行 session 另创建、更新自己的 `.gkd/progress.md`；可随本任务实现提交。不要写主工作树 PLAN/plan-changes/review，不修改或删除本 execution，不复制活动 PLAN 到执行 worktree，不创建归档或新的流程状态文件。

需要了解文档入口和验证边界时读取 `docs/README.md`、相关索引与 cloud-only 合同；不要求通读 README 产品段、所有业务合同或历史归档。正在修改的确切内容仍须先读清楚。

## 4. 允许删除的 20 个空模板

删除前完整复核下列文件及其引用。如果发现已经加入有效项目事实，保留该文件并在 progress 说明，不能为满足数量删除内容。不要删除其他模板或历史记录。

| 目录 | 获准删除的文件 |
| --- | --- |
| `.trellis/spec/plugin-sdk/backend/` | `index.md`、`directory-structure.md`、`database-guidelines.md`、`error-handling.md`、`quality-guidelines.md`、`logging-guidelines.md` |
| `.trellis/spec/plugin-sdk/frontend/` | `index.md`、`directory-structure.md`、`component-guidelines.md`、`hook-guidelines.md`、`state-management.md`、`quality-guidelines.md`、`type-safety.md` |
| `.trellis/spec/create-aio-plugin/frontend/` | `index.md`、`directory-structure.md`、`component-guidelines.md`、`hook-guidelines.md`、`state-management.md`、`quality-guidelines.md`、`type-safety.md` |

`src/templates/**` 属于产品资产，不得因 `template`、`skill`、`plugin` 关键词命中而修改或删除。不补写替代空模板。

## 5. 实施要点与顺序

### 5.1 项目流程和资料

1. 核对工作目录、分支、基线及现有差异；`.gkd/execution.md` 是 main 新增的交接，不是未知业务修改。发现其他 writer 或来源不明差异时保留并报告。
2. 修正 AGENTS、README 和运维入口。GKD 决定 route，项目只说明其 Git/环境适配；不能把 direct-main 理解成直接推送 Git main，也不能给所有任务强制 delegated。
3. 删除旧 runner 指令。Quality Check 中的各模块场景仅由对应行为修改触发；提交前检查本地允许范围，合并前等待分类器选中的自动 job。
4. 将通用指南改为按需读取/检索，保留跨层边界、内部代码和框架保证、错误可见及必要回归；移除重复、外部项目专属和推测性抽象。
5. 按第 4 节清理空模板；标记旧模板及根级发布记录。历史正文保持不动，不修复无关历史链接，不整理 PENDING。

### 5.2 文档合同检查器

1. 保留 `$gkd-main` 与 `.gkd/plan.md`、`.gkd/execution.md`、`.gkd/progress.md`、`.gkd/review.md` 的入口检查。
2. 删除 `Keep the local checkout zero-artifact.`、普通 PR 中文整句、README 中英文“禁止重复手动 CI”整句的逐字断言；规则正文仍要清晰表达原意。
3. 保留禁止旧命令及本地 package/native 示例、package scripts Actions guard、Tauri hook、自动/手动 gate 区分、job 选择、必要检查命令和 PR 打包边界。
4. 在已有 selftest 添加或调整等义文案正例：仅改写上述文案，合同仍通过；缺 GKD/execution 入口、恢复禁止命令或破坏实际质量门的负例仍拒绝。
5. 不添加通用 parser、配置文件、状态格式或框架，不扩大 checker 的职责；自测变化是门禁行为回归，不是业务测试。

### 5.3 验证、消融与提交

依次完成第 6 节检查和第 7 节场景走查。范围内问题修复后重跑受影响检查；材料性偏差按第 8 节交回 main。

提交前检查是否新增第二套生命周期、runner、抽象、重复确认或全量读取要求；删除本次产生的不必要设计。显式暂存本任务实现/删除文件及 `.gkd/progress.md`，不要使用覆盖整个仓库的无差别暂存，不暂存或提交包含本机路径的 `.gkd/execution.md`。

main 后续负责按 GKD 归档并脱敏 execution。原主工作树独有提交和根级发布证据保持原地，不合并进任务分支。

## 6. 允许的本地验证

以下命令均在执行目录运行，不经 package-manager。允许直接运行现有 Node 内建模块检查；不安装工具或依赖，不写检查产物，不启动被禁止程序。

```bash
git status --short
git diff --name-status
git diff --stat
git diff --check
node --check scripts/check-cloud-only-verification.mjs
node --check scripts/check-cloud-only-verification.selftest.mjs
node scripts/check-cloud-only-verification.mjs
node scripts/check-cloud-only-verification.selftest.mjs
node scripts/check-spec-links.mjs
git diff --cached --check
```

提交后的实现范围和空白检查以本轮真实基线为准：

```bash
git diff --name-status bc891afbb80b70efc81b628a1c48b1bd0a7051da HEAD
git diff --check bc891afbb80b70efc81b628a1c48b1bd0a7051da HEAD
git status --short
```

另对第 3 节修改文件的新增/修改本地链接和锚点进行只读核对；定向检索旧固定 runner、完整 base SHA 验证绑定、`ANY value`、已删除空模板路径及无条件双域检查要求。可以使用现有 FastCtx/rg/文件读取及不写文件的 Node 内建模块核对，不创建辅助脚本。

通过标准：命令成功；无新增断链/旧活动入口；等义文案自测通过；实质边界负例保持；实际实现 diff 仅包含获准文件。历史语境中的旧描述不冒充活动指令。

不运行全仓 Markdown 历史链接治理、业务测试、本地 lint/typecheck 或构建。不更改 `.github/workflows/**`、`.github/ci-scope.json`、CI 分类器及 required checks。因治理脚本改动需要完整 CI，由 main 创建 PR 后使用当前自动流程验证；执行 session 明确记录“云端 CI 尚未运行，由 main 后续负责”，不能声称全部检查已通过。

## 7. 执行验收标准

| 项目 | 本执行 session 应提供的结果 |
| --- | --- |
| GKD 对齐 | 入口优先引用 GKD，direct-main/delegated 不混用，没有新增项目绕过规则 |
| 旧入口消除 | 当前规范不再要求固定旧 runner、完整 base SHA 验证器或无条件双域 CI |
| 按需规则 | 阅读、检索和质量清单按受影响行为触发；保留跨层与核心行为保障 |
| 资料整理 | 空模板删除或说明保留理由；旧模板及根级发布记录有明确历史状态，正文保留 |
| checker 回归 | 等义文案通过，必要入口缺失与实际门禁破坏仍失败 |
| 范围合规 | 无业务、依赖、CI 分类/工作流、全局 Skill 或历史正文改动 |
| 验证事实 | 本地允许检查成功或准确报告阻塞；不虚构 CI、独立验收、合并和发布结果 |

走查下列规则使用场景即可，不启动测试代理或制造真实 PR：只读审查、简单 direct-main、delegated、普通文档、单域行为、shared 脚本改动、范围内修复、材料性偏差、已有授权的交付。每种场景均能定位 GKD 路线、相关资料、验证阶段和交付边界。

GKD 问题报告已由 main 写入其源仓库，不需要执行 session 阅读整个 GKD 仓库、修复报告问题或等待另一任务。报告不纳入 AIO diff。

## 8. 进度、偏差与返回

执行 session 首次开始时创建 `.gkd/progress.md`，记录实际分支、基线和起始差异。仅在以下节点更新：流程归属处理完成、资料/检查器修改完成、本地验证终态、范围偏差或阻塞、最终本地提交完成。

发现事实与方案不符时，按当前 GKD 在 progress 记录证据并暂停，交回 main 判断是否修改 PLAN 和下一轮 execution；不得自主扩大文件范围、修改 GKD 或绕过项目约束。若仅看到允许删除的模板新增有效事实，按本交接既定分支保留并说明。

完成后向 main 返回：实际修改/删除/保留文件摘要、关键取舍、每项运行命令及结果、未运行的云端检查、提交 SHA、最终 Git 状态和剩余风险。不要把“执行完成”表述为“整个任务已交付”。

本地提交后 `.gkd/execution.md` 保持 main 所有且未提交是已知交接状态，应如实说明；不要为了清空 status 删除交接或提交绝对路径。后续独立验收、review、归档与清理由 main 负责。
