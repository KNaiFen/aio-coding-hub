# CI 调度与归档成本优化 Execution

- Revision：r2；来自已批准 PLAN r1，2026-09-08。r1 已实现，r2 仅处理独立审查 F1。
- worktree：`<执行 worktree>`。
- 分支：`ci/scheduling-cost`；baseline：`692da663ce0eaac2569e2c74eff78ee0c2317118`。
- 用户授权：“automatic 模型执行这个 PLAN”；本执行角色只实施、写 progress、本地中文提交，然后停止。推送/PR/合并/归档/清理由 main 后续交接收尾角色完成。

## 本轮修复限定

- 起始 HEAD：`094af0190aaa3f13cef68269e75e641060fae923`；原 writer 已停止。
- 独立验收发现 F1：`scripts/check-ci-quality-gates.mjs:586` 的 CodeQL analyze checkout 只被 action 列表约束，添加 `if: false` 或 `continue-on-error: true` 不能被拒绝；PLAN 要求 checkout/init/analyze 全部必须执行且不忽略失败。
- 本轮只在原 checker 中对 analyze 首个 checkout 复用 `stepRunsUnconditionally`，并在原 selftest 加“checkout if false”和“checkout continue-on-error true”两个负例。既有 helper 若覆盖两者可直接使用；不得重构解析器或新增抽象。当前 workflow 本身正确，不修改。
- 只允许本轮编辑/提交 `scripts/check-ci-quality-gates.mjs` 和 `scripts/check-ci-quality-gates.selftest.mjs`，追加 progress 的修复、检查和新 HEAD。下方 r1 内容作为完整目标约束保留，已完成的其余实现不要重做。
- 本地仍只 `node --check` 两文件和 `git diff --check`，不执行测试本体。中文提交后停止，返回 HEAD 和复查线索；不推送/PR/归档。

## 目标

实施三项：在治理文档落实归档与最终验证合批；CodeQL 自动事件通过现有分类器证明纯文档时跳过双语言分析；frontend/rust/observer-macos 等 contracts 成功后再启动。

原建议第 4 项不做：不拆分 Rust 测试与 benchmark，不改变测试线程、构建并发、benchmark 触发或门槛。不改缓存、runner、依赖版本、发布/候选行为、业务代码、平台设置或用户级 skills。

## 所有权与规则

- 你是当前执行 worktree 的唯一 writer，但并非唯一工作区使用者。不要回退或清理他人改动，不修改主工作树。
- 只读本 execution 和适用规则，无需重读主 PLAN/历史调查。先完整读根 AGENTS、两份下列现行文档和即将修改的代码；存在 `.codegraph` 时先用其定位，否则不创建索引。
- 不得派生子代理。方案、最终验证和交付决定由 main 负责。
- 面向用户中文。使用 apply_patch 语义编辑，机械批量修改可 FastCtx replace。最小实现，完成后消融审查，删除本次不必要抽象。
- 不本地安装依赖、测试、覆盖率、编译、格式化/生成、打包或跑包脚本，不绕过 Actions guard。允许 Git 读/本地提交、`git diff --check`、`node --check` 无依赖语法检查；测试全部留 GitHub Actions。

## 允许修改的实现文件

1. `.github/workflows/ci.yml`
2. `.github/workflows/codeql.yml`
3. `scripts/check-ci-quality-gates.mjs`
4. `scripts/check-ci-quality-gates.selftest.mjs`
5. `docs/operations/github-actions-governance.md`
6. `.trellis/spec/aio-coding-hub/cross-layer/cloud-only-verification-contract.md`
7. `.gkd/progress.md`（执行记录；本轮可不提交，留收尾归档）。

execution 为 main 所有，不修改也不提交。`.github/ci-scope.json`、`scripts/ci-change-scope.mjs` 和 classifier selftest 只复用，不计划改动。需要其他文件联动时记录出处与必要性并返回 main，不自行扩大范围。

## 精确改法

### CodeQL

- 在 codeql.yml 新增轻量 `change-scope` job，仅 `push`/`pull_request` 自动事件运行；完整 checkout 历史、Node 22、contents read job 权限、合理短 timeout（沿用主 CI 分类 10 分钟）、action 复用仓库现有固定 SHA。
- 与主 CI 相同 env/input：event、PR base SHA、PR head 或 github.sha、push before SHA，执行既有 `node scripts/ci-change-scope.mjs --event ... --base ... --head ... --before ...`；输出 scope/frontend_ci/rust_ci。
- analyze 增加 `needs: change-scope`，job if 固定为下式（可换行，语义不变）：

```text
${{ !cancelled() && !(needs.change-scope.result == 'success' && (needs.change-scope.outputs.scope == 'process-docs' || needs.change-scope.outputs.scope == 'checked-docs') && needs.change-scope.outputs.frontend_ci == 'false' && needs.change-scope.outputs.rust_ci == 'false') }}
```

- schedule/manual 的分类 skipped 仍跑双语言；分类失败/输出缺失且整次 workflow 未取消仍选择双语言，保留分类失败结果。不能用输出为空证明纯文档。
- 保持双语言固定 matrix、no-build、fail-fast false、45 分钟、checkout/init/analyze 的固定 SHA 与输入、现有触发/并发/分析权限。不增加 step 条件跳过，不忽略失败，不加 workflow paths 过滤，不裁剪单语言。
- 在原 `assertCodeqlContract` 验证新增分类 job 的触发、输入、历史、命令、输出、只读权限、analyze needs/if；其余契约仍维持。复用现有解析器，不新增路径分类器/白名单/公共 workflow 或通用 YAML 层。

### 主 CI

- frontend、rust、observer-macos 的 needs 改 `[change-scope, contracts]`；各 if 保留 always、change-scope 成功与原域选择，并加 contracts 成功。
- 三个重任务在合同成功后仍并行。contracts 自身选择与 steps 保持；candidate-plan 原并行关系、候选依赖与矩阵保持。
- manual-dispatch-guard 自动 skipped 沿 needs 链不应阻止正常自动检查，保留必要状态表达式。
- ci-gate 名称、always、needs、env 和原聚合脚本/digest 保持。选中任务因合同失败而 skipped 必须导致 gate 失败，不能放宽聚合。
- 更新质量 checker 的 CI_JOB_CONDITIONS 并检查三 job needs。同步云端合同和治理文档中“与合同并行”的旧描述。

### 归档文档

在现有治理文档的提交/发版段落写项目顺序：实现可先凭差异和已有证据审查，最终 CI 待定；一次收尾角色准备交付前已知归档、推送最终 head、等必要 CI、获准合并清理。

- 已知 PLAN/execution/progress/review/说明/活动记录迁移随最终待验证 head 合批推送。可多个本地提交，不在成功后仅为归档制造新 head。
- 必要提前云端反馈和真实修复不禁止，不要求一次 CI 或一律单 PR。
- 后来 CI/merge SHA/清理事实按真实时间保存到主工作树归档，引用 PR/Actions；不提前写成功，不为回填自身 SHA/CI 递归提交。
- 主代理不提前归档，不修改通用 GKD skill。

## 必要云端验证设计

在既有 `check-ci-quality-gates.selftest.mjs` 添加与本次实际调度相关的正反例，复用真实 workflow/gate 和 classifier fixtures，不另造平行实现：

- 纯过程/受检文档跳过 CodeQL；单域源码、共享、未知、混合、空 diff、异常必须分析；删除/跨域改名/复制依赖既有完整路径并集，仍校验输出到 job 的映射。
- schedule/manual 的 skipped 分类仍分析；分类失败/取消/缺失输出不是纯文档证明；保持不忽略分析失败。
- 选中域 + contracts success 才启动重 job；needs 和 if 的约束缺失应被 selftest 拒绝。
- 原 gate 对所需 failure/cancelled/异常 skipped 不能成功，正常文档 skipped 成功；复用实际 Bash 聚合脚本和已有 fixtures，不改 digest 放宽。

执行角色本地只运行 `git diff --check` 和对变更的两份 mjs 执行 `node --check`，命令退出 0。禁止在本地运行 selftest/checker 本体、依赖或完整测试。云端由收尾推送自然触发现有 contracts（classifier selftest、quality checker/selftest、cloud-only checker/selftest、pin/doc 合同）、frontend、rust、observer-macos 和双语言 CodeQL；本任务为控制平面变更，必须全量，最终 head required ci-gate/pr-title 都成功。不得额外 dispatch/重跑/取消。

## 输出与停止

写 progress：实现判断、实际文件、消融结论、本地检查命令与结果、云端未执行事实、剩余风险。只 stage 六份实现文件，中文本地提交（建议 `ci: 优化文档分析与合同前置调度`），不提交 execution/progress、不推送。

返回完整 HEAD、差异摘要、检查结果、未覆盖或阻塞点、progress 路径，并停止。main 将启动独立技术验收，再一次交接收尾归档、最终推送和云端验证。不要等待并不存在的 CI，不先创建 PR 或归档。
