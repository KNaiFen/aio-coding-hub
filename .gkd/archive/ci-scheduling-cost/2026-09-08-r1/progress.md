# CI 调度与归档成本优化 Progress

- Execution：r1，2026-09-08；执行角色已完成实现并停止，独立审查和云端验证待 main 后续安排。
- worktree：`<执行 worktree>`；分支：`ci/scheduling-cost`。
- baseline：`692da663ce0eaac2569e2c74eff78ee0c2317118`。
- 实现 HEAD：`094af0190aaa3f13cef68269e75e641060fae923`。
- 本地提交：`ci: 优化文档分析与合同前置调度`，只包含交接六份实现文件；execution/progress 未提交。

## 实施事实

1. `.github/workflows/codeql.yml`：新增只在 push/PR 运行的 change-scope，复用现有 classifier、完整历史 checkout、Node 22、固定 action SHA、10 分钟 timeout 和 job 级 contents read。输出 scope/frontend_ci/rust_ci。analyze 使用交接规定的 needs/if；只有成功且完整的纯文档证明跳过双语言，schedule/manual、分类失败或输出缺失继续分析，整次取消除外。原触发、并发、分析权限、双语言 no-build matrix 与步骤保持。
2. `.github/workflows/ci.yml`：frontend、rust、observer-macos 直接依赖 change-scope 和 contracts，保留 always 与分类结果/域选择，并要求 contracts success。三个重任务之后仍可并行；candidate-plan、候选构建、Rust tests/benchmark 和 ci-gate 原脚本未改。
3. `scripts/check-ci-quality-gates.mjs`：原解析器新增 CodeQL 分类 job 的触发、输入/输出、历史、权限、命令与 analyze 条件检查，检查三重任务 needs/if。复用并导出三个既有解析函数给 selftest；原 gate digest `9f33c0c292156d3886a41a4dc5135d2d2ae637abd6dc951db61465254add4b11` 保留。
4. `scripts/check-ci-quality-gates.selftest.mjs`：更新旧“与 contracts 独立”负例为缺失依赖/成功条件/status 表达式负例；新增 CodeQL 契约负例。通过既有 runClassifier 与真实路径 fixture，把过程/受检文档、单域/共享/未知/混合、空 diff、删除、改名、复制和异常输出接入真实 job 条件；读取原 Bash gate 验证正常 skipped 和所需任务失败、取消、异常 skipped。schedule/manual skipped 分类、失败分类、缺失输出、workflow 取消及忽略分析失败也有用例。
5. `docs/operations/github-actions-governance.md` 与 `.trellis/spec/aio-coding-hub/cross-layer/cloud-only-verification-contract.md`：同步合同前置和 CodeQL 选择行为；治理文档明确审查后一次收尾角色合批已知归档与最终待验证 head，必要提前反馈/修复可继续，后来 CI/merge/清理事实按真实时间记录并引用 PR/Actions，不递归回填自身 SHA/CI。

实现改动合计六文件，424 行新增、39 行删除。没有修改 classifier/policy、其他脚本、业务代码、主 worktree 或 execution；没有需要 main 决定的新边界或材料性缺口。

## 实际验证

环境：本机 FastCtx 非交互 bash，所有 Git 和 Node 命令的 cwd 均显式为上述执行 worktree；使用 PATH 中现有 Node，只检查语法，不加载或执行模块。未安装依赖，未运行包脚本、测试、格式化/生成、构建或打包。

| 命令 | 实际结果 | 证明范围 |
| --- | --- | --- |
| `git status --short --branch`（开始） | 退出 0；分支正确，仅 `.gkd/execution.md` 未跟踪 | 初始实现范围无他人未提交改动 |
| `git rev-parse HEAD`（开始） | 退出 0；与 baseline 一致 | 基线匹配 |
| `node --check scripts/check-ci-quality-gates.mjs` | 退出 0，无输出 | 检查器 JavaScript 语法通过 |
| `node --check scripts/check-ci-quality-gates.selftest.mjs` | 退出 0，无输出 | selftest JavaScript 语法通过 |
| `git diff --check` | 退出 0，无输出 | 最终六份实现差异无空白错误 |
| `git diff` 与 `git diff --cached --stat` | 退出 0；检查六份实现差异，暂存仅六文件 | 范围与不变量差异复核 |
| `git add -- .github/workflows/ci.yml .github/workflows/codeql.yml scripts/check-ci-quality-gates.mjs scripts/check-ci-quality-gates.selftest.mjs docs/operations/github-actions-governance.md .trellis/spec/aio-coding-hub/cross-layer/cloud-only-verification-contract.md` | 退出 0 | 精确暂存实现文件 |
| `git commit -m 'ci: 优化文档分析与合同前置调度'` | 退出 0；提交六文件 | 本地中文提交完成 |
| `git rev-parse HEAD`（提交后） | 退出 0；`094af0190aaa3f13cef68269e75e641060fae923` | 实现 HEAD 已固定 |

## 消融判断与剩余风险

- 完成范围内消融检查：保留现有分类策略和解析函数，不新增路径分类器、白名单、公共 workflow 或通用 YAML 层。仅两个重复调用的测试辅助函数用于计算真实条件与执行真实 gate；没有为单次操作新增生产抽象。
- 两份旧“独立于 contracts”负例随获批行为更新为缺失 contracts 依赖/条件的负例；未放宽质量门、删除受保护命令或修改聚合 digest。
- 本地未执行 checker/selftest 本体或任何 Actions；新增行为断言仍需云端运行。JavaScript 条件适配验证显式布尔表达式，不能独立证明 GitHub 的 needs/skipped 传播；保留指定 status 表达式并由实际自动 workflow 验证。
- 既有 runClassifier 对输入/Git 异常返回 full 结果；分类 job 自身失败与该 full 回退分别覆盖，未改变错误策略。
- 后续必须在最终交付 head 自然触发现有 contracts、frontend、rust、observer-macos 和双语言 CodeQL，并取得 required ci-gate/pr-title 成功。未额外 dispatch、重跑、取消或等待不存在的 CI。
- 未推送、创建 PR、合并、发布、归档或清理。未启动其他代理；main 可据此记录引用进行独立审查，再交接收尾。

## Execution r2：独立审查 F1 修复

- 日期：2026-09-08；worktree：`<执行 worktree>`；分支：`ci/scheduling-cost`。
- 起始 HEAD：`094af0190aaa3f13cef68269e75e641060fae923`；本轮 HEAD：`82bc305834c601a3c2acb45cc61850d19cb02f34`。
- 本地提交：`ci: 补齐 CodeQL checkout 执行约束`；仅两份获批 mjs，14 行新增、1 行删除。execution/progress 未提交。
- `scripts/check-ci-quality-gates.mjs:590`：在 analyze 的既有动作序列检查后复用 `stepRunsUnconditionally(codeqlSteps[0])`，拒绝 checkout 上的条件与忽略失败设置。前置动作序列检查通过时首步必然存在，无需新增空值兜底。
- `scripts/check-ci-quality-gates.selftest.mjs:554`：既有负例表新增 checkout `if: false` 与 `continue-on-error: true` 两项。匹配的 `with` 后直接为 `persist-credentials` 的片段只存在于 analyze checkout；分类 checkout 先声明 `fetch-depth`，不会被误改。复用既有实际替换断言和检查器失败断言。
- 消融判断：只增加一次现有 helper 调用和两个负例，没有新增 helper、解析器重构、workflow 或其他目标改动；未删除断言或放宽原检查。

环境：本机 FastCtx 非交互 bash；以下全部命令 cwd 显式为上述 worktree，Node 来自既有 PATH。仅 `--check` 解析语法，没有执行 checker/selftest 本体、安装依赖、包脚本、编译、格式化/生成或其他测试。

| 命令 | 实际结果 | 判断 |
| --- | --- | --- |
| `git status --short --branch`（开始） | 退出 0；分支正确，仅 execution/progress 未跟踪 | 无他人未提交实现差异 |
| `git rev-parse HEAD`（开始） | 退出 0；与 r2 起始 HEAD 一致 | 基线符合交接 |
| `node --check scripts/check-ci-quality-gates.mjs` | 退出 0，无输出 | checker 语法通过 |
| `node --check scripts/check-ci-quality-gates.selftest.mjs` | 退出 0，无输出 | selftest 语法通过 |
| `git diff --check` | 退出 0，无输出 | 两文件实现差异无空白错误 |
| `git diff -- scripts/check-ci-quality-gates.mjs scripts/check-ci-quality-gates.selftest.mjs` | 退出 0；逐项读取最终差异 | 范围与消融检查完成 |
| `git add -- scripts/check-ci-quality-gates.mjs scripts/check-ci-quality-gates.selftest.mjs` | 退出 0 | 精确暂存获批两文件 |
| `git diff --cached --stat` | 退出 0；仅两 mjs，14 行新增、1 行删除 | 提交范围正确 |
| `git commit -m 'ci: 补齐 CodeQL checkout 执行约束'` | 退出 0 | 本地中文提交完成 |
| `git rev-parse HEAD`（提交后） | 退出 0；`82bc305834c601a3c2acb45cc61850d19cb02f34` | 本轮 HEAD 已固定 |

剩余验证：本地语法与差异检查不证明新增负例运行结果，checker/selftest 本体仍需后续 GitHub Actions 执行。没有新增边界问题或阻塞；没有运行或等待云端 CI、推送、创建 PR、合并、发布、归档、清理或启动子代理。执行角色已停止，可按上述 HEAD 和两处改动交回 main 复查。
