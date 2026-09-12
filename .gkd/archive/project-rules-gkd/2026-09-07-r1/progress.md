> 归档观察：2026-09-07 02:54:17 +08:00。来源：执行工作树 `.gkd/progress.md`；记录作者：执行角色。正文保留原作者的状态和观察时点，本机路径与账号已脱敏；后续收尾事实见同目录 `summary.md` 及收尾返回。

# 实施记录：项目规则以 GKD 为准

- 执行交接：execution r1；来源为本 worktree 的 `.gkd/execution.md`。
- 日期：2026-09-07。
- worktree：`<执行工作树>`。
- 分支：`chore/gkd-project-rules`。
- 基线与完成实施时实际 HEAD 均为 `a35f2aa4ea469f6e4066582b2e969f1ec44fca2e`。
- 初始状态：仅交接文件 `.gkd/execution.md` 未跟踪，没有已有实施差异。
- 状态：实现和交接规定的轻量验证已完成，停止并交回 main 进行独立验收。本阶段未暂存、未提交。

## 改动范围

- `AGENTS.md` 保留用户级 `$gkd-main` 引用、AIO 技术与仓库身份、默认 `origin`、GitHub 仓库参数、知识库和模块合同入口，以及资源与工具环境约束；删除固定路线、角色、交接、提交、分支、合并和归档规则。
- `README.md`、`README_EN.md`、`docs/README.md` 删除重复流程与提交前要求，保留产品、安装、支持矩阵、知识库分类与文档状态、Actions 和发布事实。
- `docs/operations/github-actions-governance.md` 仅整理“提交与发版”，保留原标题锚点、required checks 和精确候选晋升规则；明确 Conventional Commit 格式是既有 PR 标题校验事实，不再规定 Git 提交格式。
- `.trellis/spec/aio-coding-hub/cross-layer/cloud-only-verification-contract.md` 删除生命周期和固定文件清单，保留资源约束、包 guard、Tauri hook、CI job、候选制品和生成漂移合同；按实际所有者说明 cloud-only、CI quality gates、change-scope selftest 的覆盖。
- backend 与 cross-layer 索引删除通用提交前规则，保留按受影响行为适用的模块边界和回归场景。Codex config 与 config migration 合同只把本地验证入口改为云端合同引用，未改业务语义或生成绑定职责。
- 两份插件文档仅整理仓库本地资源/工具环境入口，保留外部插件作者工具用法、SDK/API 和 CI 说明。
- 删除获批的四份 `.trellis/spec/guides/` 通用指南与 `docs/operations/templates/simple-change-record.md`。共 19 个已有文件发生变化：14 修改、5 删除，另新增本 progress。
- `scripts/check-cloud-only-verification.mjs` 移除 AGENTS fixture 读取/字段、GKD 文件名要求、旧生命周期词和代理流程字串断言；无剩余消费者的 `requireAbsent` 一并删除。
- 对应 selftest 删除流程断言及旧措辞变体，保留一个精简 GKD 引用与云端合同兼容的正例；真实包/Tauri/Actions、README 本地安装与 active-spec 命令负例全部保留。

## 判断与消融检查

- 未发现 `.codegraph/`，因此按普通定向读取和检索定位，没有创建索引。
- 删除目标的现行 Markdown 超链接均位于被删除的指南内部；历史正文、归档和 JSONL 的路径引用保留。两类链接检查均通过，无需修改范围外文件或建立兼容入口。
- 现行入口定向搜索 `direct-main`、`delegated`、默认手动、提交前、四个 GKD 文件名、固定零产物措辞，仅剩治理文档中明确归属 PR 标题的 Conventional Commit 事实。
- 未新增流程模板、文案解析器、白名单、资源监控、数值阈值或其他抽象。模块内 SQLite/request/transaction commit 语义未改。
- 只读 `git diff --name-only -- .github package.json pnpm-lock.yaml packages src src-tauri scripts/require-github-actions.mjs scripts/check-ci-quality-gates.mjs scripts/check-ci-quality-gates.selftest.mjs scripts/check-spec-links.mjs scripts/check-markdown-links.mjs` 无输出：业务、`.github/**`、包清单/guard、Tauri 配置及其他检查器保持原样。
- 没有材料性方案偏移；范围内修正了新增相对路径的多余斜杠和 selftest 覆盖描述，再执行最终检查。

## 验证环境与结果

全部命令显式在上述 worktree 执行。环境为 Darwin arm64，系统已有 Node `v26.5.1`、Git `2.50.1 (Apple Git-155)`。已检查脚本 imports、入口和 I/O：使用内置 Node 模块，只读取仓库文件；Markdown 检查器另启动只读 `git ls-files`。没有依赖安装、生成器、构建工具、网络请求或文件写入。

| 命令 | 实际结果 | 判断 |
| --- | --- | --- |
| `git diff --check` | exit 0，无输出 | 无新增空白错误 |
| `node scripts/check-cloud-only-verification.mjs` | exit 0，`[cloud-only-verification] repository contract passed` | 文档与工具/云端合同兼容 |
| `node scripts/check-cloud-only-verification.selftest.mjs` | exit 0，`[cloud-only-verification:selftest] all assertions passed` | 精简流程正例与保留负例通过 |
| `node scripts/check-ci-quality-gates.mjs` | exit 0，`[ci-quality-gates] repository contract passed` | 现有 CI 门禁结构与命令合同通过 |
| `node scripts/check-spec-links.mjs` | exit 0，无输出 | 规范链接检查通过 |
| `node scripts/check-markdown-links.mjs` | exit 0，无输出 | Markdown 路径与标题锚点检查通过 |

各检查在最终实现完成后运行一次，全部成功，未重复全量测试。随后仅补写本 progress，不再修改实现。

## 未覆盖与剩余事项

- 未运行 package manager、开发服务器、lint/typecheck、完整测试/覆盖率、Cargo、Tauri、构建、签名、打包或长时性能检查；没有产生依赖或构建产物。
- 未推送、创建 PR 或运行 GitHub CI；不宣称远端 CI 成功。既有分类器会因脚本变化在未来 PR 选择前端和 Rust 两端 CI，该行为未改。
- 无已知实施阻塞或范围外修复需求。独立验收和后续提交由 main/收尾角色处理。
- 本阶段无提交许可，HEAD 保持基线；execution 未修改，未写 PLAN/review，未改历史正文或清理活动记录。
