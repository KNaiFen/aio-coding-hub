> 归档快照：gkd-rule-alignment，PLAN/execution r2。记录各阶段当时事实，不是活动指令；本机目录已替换为逻辑路径。

# gkd-rule-alignment 执行进度

## 起始现场

- 执行交接：`.gkd/execution.md` r2，路线为 `delegated/automatic`。
- 实际分支：`chore/gkd-rule-alignment`。
- 实际基线：`bc891afbb80b70efc81b628a1c48b1bd0a7051da`，与交接一致。
- 起始 `git status --short --branch`：分支跟踪 `origin/main`，仅 `.gkd/execution.md` 未跟踪；该文件由 main 提供，不纳入本地提交。
- 已完整读取交接与执行目录 `AGENTS.md`；未发现嵌套 `AGENTS.md`，执行目录无 `.codegraph/`。
- 本轮仅修改交接列出的规则文档、治理脚本与经复核的空模板，按许可完成零依赖检查和本地中文提交。
- 云端 CI 尚未运行，由 main 后续负责；本轮不验收、不推送、不创建 PR、不归档、不合并、不清理。

## 流程归属处理完成

- 已按已安装 `$gkd-main` 的现行入口核对路线与角色；仅以执行交接 r2 施工，未读取主工作树 PLAN。
- AGENTS、两份 README、知识库入口及 Actions 运维说明已区分 GKD 生命周期和 AIO 项目约束：仅 delegated 生成 execution/progress，direct-main 仍通过任务分支 PR 集成。
- 已分开提交前批准的本地零依赖检查与合并前自动 CI，并说明单域、shared、文档分类保持现行规则。
- 通用指南按受影响行为读取和检索，移除复制即抽象、固定误报率、旧 runner 及 Trellis 专属重复段落；业务索引仅调整清单触发条件，保留具体合同语义。
- 检查器仅移除获准的四项自然语言逐字断言，自测增加四项等义改写正例与缺失 GKD 入口负例；尚待本地验证。

## 资料与检查器修改完成

- 已逐份完整复核 20 个允许删除的文件：Plugin SDK backend 6 个、frontend 7 个，create-aio-plugin frontend 7 个，均只有 `To fill` / `To be filled by the team`、提问或示例占位，没有有效项目事实，因此全部删除，无需保留例外。
- 定向检索未发现上述三个目录的外部活动引用；目录索引内部相对引用随获准模板一并删除。历史审计中的其他旧模板路径不在本轮修复范围。
- 旧 `simple-change-record.md` 添加停用标识，根级 plan/progress/review 添加历史状态与现行入口；原正文保留，未复制主工作树版本。
- 未修改业务源码、产品模板、依赖、workflow/classifier、其他治理脚本或全局 Skills；检查器未引入 parser、配置或新的流程状态。

## 本地验证终态

以下命令均直接在执行 worktree 运行，不经 package-manager，未安装依赖或写检查产物。

| 命令 | 实际结果 |
| --- | --- |
| `git status --short` | 成功；暂存前只有本轮获准修改/删除、execution 与 progress。 |
| `git diff --name-status` | 成功；37 个既有文件全部在交接范围内，其中删除 20 个。另新增本执行 progress。 |
| `git diff --stat` | 成功；主要删除占位模板和重复指南。 |
| `git diff --check` | 成功，范围内修改后复跑仍无空白错误。 |
| `node --check scripts/check-cloud-only-verification.mjs` | 成功。 |
| `node --check scripts/check-cloud-only-verification.selftest.mjs` | 成功；自测消融调整后复跑成功。 |
| `node scripts/check-cloud-only-verification.mjs` | 成功：`repository contract passed`；修改后复跑成功。 |
| `node scripts/check-cloud-only-verification.selftest.mjs` | 成功：`all assertions passed`；修改后复跑成功。 |
| `node scripts/check-spec-links.mjs` | 成功，无断链输出；修改后复跑成功。 |

- 只读 Node 内建模块核对 `git diff --unified=0 -- '*.md'` 的新增/修改链接：15 个本地链接、2 个锚点通过。
- 只读 Node 将根级 plan/progress/review 和旧 simple-change-record 新增横幅后的正文，与基线 `git show` 逐字比较：4 份全部保持原样。
- 对活动入口和本次规范定向 `rg` 检索旧固定 runner、完整 base SHA 绑定、禁止选择检查、ANY value、无条件双域要求、Trellis 专属重复段落和固定误报率：无匹配（退出码 1）。
- 对仓库定向检索三个已删除模板目录及本次移除标题的锚点引用：无匹配（排除 main 提供的 execution；退出码 1）。模板删除前检索中命中的产品 package 路径与其他历史模板记录均保留。
- 起始探索曾对不存在的 `.trellis/spec/index.md`、`.trellis/config.yaml` 检索，退出码 2；随后按实际目录重做引用检索，未依赖不存在的入口。
- 已只读核对提交 hook：无自定义 `core.hooksPath`，pre-commit、prepare-commit-msg、commit-msg、post-commit 均不存在，不会由本地提交触发被禁止工具。

## 场景走查与消融

| 场景 | 规则定位与边界 |
| --- | --- |
| 只读审查 | AGENTS 的施工前 PLAN 要求不扩大到只读检索；按 GKD 角色和相关实现事实审查。 |
| 简单 direct-main | GKD 选择路线，AIO 保留任务分支 PR；不要求 delegated execution/progress。 |
| delegated | GKD 默认手动、明确选择才自动；执行交接与进度在声明 worktree，执行完成交回 main。 |
| 普通文档 | 按需读取相关段落，本地批准的零依赖检查；自动 CI 使用既有文档分类，不触发业务场景清单。 |
| 单域行为 | 只读取受影响合同，相关业务回归在 GitHub Actions；单域 PR 按现行分类选对应 job。 |
| shared 脚本 | 保留 shared 路径选择前端和 Rust；本任务云端完整 CI 由 main 后续启动 PR 自动验证。 |
| 范围内修复 | 沿获批 execution 修复并复跑受影响检查；本轮等义夹具调整在此范围内。 |
| 材料性偏差 | 记录 progress 并暂停，交回 main 判断和更新授权/交接。 |
| 已有授权交付 | 运维说明按已有任务授权执行，由 GKD 路由后续验收与收尾，不另建项目确认机制。 |

- 消融发现原等义自测会因文档原句改写而失败，已改用自测自有的固定文案夹具；四项独立等义改写均通过，现行文档可正常改写。
- 保留必要 GKD/execution/progress 入口、禁止本地/旧命令、Actions guard、Tauri hook、自动/手动门禁和候选 PR 边界负例；生产 checker 仅删除四项自然语言逐字断言。
- 未新增抽象、parser、配置、第二套生命周期、重复确认或全量读取要求。业务索引的场景含义保持，只有触发条件调整。
- 本地允许检查完成；云端 CI 尚未运行，由 main 后续负责。独立验收、main review、PR、归档、合并和清理均未执行，本轮结果不代表整个任务交付。

## 最终本地提交

- 已完成本地中文提交：`chore(规则): 对齐 GKD 流程与验证范围`；最终提交以本分支 HEAD 为准，SHA 由执行返回消息报告。
- `git diff --cached --check` 成功，显式暂存 38 个获准文件；未暂存或提交 `.gkd/execution.md`。
- 提交后 `git diff --name-status bc891afbb80b70efc81b628a1c48b1bd0a7051da HEAD` 成功：17 个既有文档/脚本修改、20 个空模板删除、1 个执行 progress 新增，无范围外变更。
- 提交后 `git diff --check bc891afbb80b70efc81b628a1c48b1bd0a7051da HEAD` 成功；`git status --short` 仅显示 `?? .gkd/execution.md`，属于交接声明的已知状态。
- 本节提交事实随同一任务提交补入，不产生额外业务修改。执行完成后停止，交回 main 接续独立验收与后续已授权交付。
