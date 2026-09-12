# CI 调度与归档成本优化 PLAN

- Revision：r1，2026-09-08。
- 状态：r1 已批准，delegated/automatic 执行中。
- 授权依据：用户先要求“按照这个思路写 PLAN，4 不需要”，后明确“automatic 模型执行这个 PLAN”。本次批准覆盖第 6 节实施、本地提交、推送、PR、正常合并、云端验证、归档及本任务默认清理；第 4 项仍排除。
- 目标仓库：`KNaiFen/aio-coding-hub`；目标分支：`main`。
- 路线：`delegated/automatic`，使用已配置的 gkd_execute、gkd_accept、gkd_closeout 角色；执行 worktree 为 `<执行 worktree>`，分支 `ci/scheduling-cost`，baseline 为 `692da663ce0eaac2569e2c74eff78ee0c2317118`。启动前远端 SHA 与规则复核无变化。

## 1. 目标与成功标准

落实调查建议的前 3 项：将交付前已知归档纳入最终待验证 head、让纯文档改动跳过 CodeQL 分析、让前端/Rust/macOS 重任务等待合同成功。收益分别为减少重复验证轮次、减少纯文档分析用量、减少合同失败后的无效执行。

成功标准：

1. 主 CI 既有变更覆盖保持一致；required `ci-gate` 和 `pr-title` 始终按原触发范围报告，相关检查失败或未执行不能被聚合为成功。
2. CodeQL 自动 PR/push 仅在既有分类器成功证明整个比较范围为纯过程文档或纯受检文档时跳过双语言分析。源码、混合、共享、未知、空 diff 和无法分类仍选择两种语言；定时与手动运行继续全量。
3. 选中的 frontend、rust、observer-macos 只在 contracts 成功后启动；合同失败、取消或异常跳过时不启动重任务，正常完成的聚合 gate 必须失败，整次取消不能形成成功门槛。
4. 本任务交付前可知的归档与整理在最终待验证 head 推送前完成。最终 CI、合并和清理事实按真实时点记录，不因回填记录自身的 SHA 或 CI 再制造提交。
5. 实现、合同检查和自然触发的必要云端验证通过，独立审查完成。性能效果样本不足时写明“实现完成，效果待观察”，不追加无限观察门槛。

明确排除原建议第 4 项：不拆分或并行化 Rust 测试与 benchmark，不调整测试线程、构建并发、benchmark 触发和合并门槛。缓存、runner、依赖版本、候选去重、发布流程及通用 GKD skills 也不属于本次范围。

## 2. 基线与证据

- 2026-09-08 18:07-18:18（北京时间）只读采样：最近连续 40 次主 CI，详细读取 10 次 job 数据；另核查近期 CodeQL 和 PR #190/#192/#193/#194。
- 远端 main 为 `692da663ce0eaac2569e2c74eff78ee0c2317118`，写计划前再次通过 branches/main API 确认。调查时本地 HEAD 为 `060cb7ef`，相关 workflows、scripts 和两份现行 CI 文档相对该远端提交无差异。本地主干有独有历史及其他任务未提交归档，必须保留。
- [main ruleset](https://github.com/KNaiFen/aio-coding-hub/rules/20130768) 生效，required 为 `ci-gate`、`pr-title`，绑定 GitHub Actions，严格要求更新到基线；无生效 merge queue 的可见证据。CodeQL 非 required。
- 分类来源：`.github/ci-scope.json:3`、`scripts/ci-change-scope.mjs:309`。PR 比较 merge-base 到 PR head，push 比较 before 到 head，删除按旧路径、改名/复制按两端覆盖并集，分类异常默认全量。
- 现行 DAG：`.github/workflows/ci.yml:82` 的 contracts 与 frontend/rust/observer-macos 并行，后 3 个 job 当前只依赖 change-scope。`ci-gate` 在 `:708` 使用 always 并严格检查选中成功、未选中 skipped。
- [合同失败 run 34197833833](https://github.com/KNaiFen/aio-coding-hub/actions/runs/34197833833)：创建后 26 秒合同已失败，gate 在约 20 分 14 秒后失败；重任务仍执行。合同样本自身约 8-12 秒。
- [轻量 PR CI 34203730893](https://github.com/KNaiFen/aio-coding-hub/actions/runs/34203730893) 墙钟 27 秒，配套 [CodeQL 34203730875](https://github.com/KNaiFen/aio-coding-hub/actions/runs/34203730875) 双语言 job 合计 14 分 01 秒；对应 push CI 17 秒，CodeQL job 合计 14 分 19 秒。CodeQL 不阻塞该 PR 合并，收益主要是累计用量。
- [PR #193](https://github.com/KNaiFen/aio-coding-hub/pull/193) 在实现 head `173ce95b` 的 CI 成功后追加纯归档 `27ce7c27`，新增 [run 34187295190](https://github.com/KNaiFen/aio-coding-hub/actions/runs/34187295190)：主 CI 墙钟 33 分 50 秒，相关 workflows job 合计 54 分 28 秒。这是同一 PR 的新增验证 head；完整 PR 仍含源码，不能改成只比较最后提交来跳过检查。
- #190 归档已在 PR 创建前合批，第二轮源于真实修复；#192 合并后补记未另开 PR；#194 后续 head 属于平台合并状态恢复。四例未证明普遍存在额外归档 PR。
- 样本首个 job 等待通常 2-3 秒；已查 Rust/pnpm 缓存命中，cargo-audit 安装步骤仅 3 秒，暂不支持优先优化缓存。
- 命令摘要：Git status/diff/history；`gh api repos/KNaiFen/aio-coding-hub/{branches/main,rules/branches/main,rulesets/20130768}`；Actions runs/jobs 与少量日志；PR commits/files/checks。以上均为只读。

平台语义已按 [GitHub required checks 文档](https://docs.github.com/en/pull-requests/how-tos/merge-and-close-pull-requests/troubleshooting-required-status-checks) 和 [workflow needs 文档](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#jobsjob_idneeds) 核对：跳过整个 required workflow 可能留下 pending；依赖失败导致的 skipped 不能代替聚合失败判定。本次不在 required workflow 上增加路径过滤。

以上执行时长不等于计费分钟，未乘 runner 平台倍率；重叠运行不累加为用户等待。不由少量样本推算统一提速比例或 flake 率。

## 3. 实现设计

### 3.1 P1：归档在最终验证前合批

在 `docs/operations/github-actions-governance.md` 的提交与发版段落记录本项目的执行顺序：main 根据实现差异和已有证据审查，注明最终云端验证待定；收尾角色在同一轮内准备归档、推送最终 head、等待验证并完成获准交付。

- PLAN、execution、progress、review、实现说明、已有检查结果和活动记录迁移等交付前已知内容，随原任务 PR 的最终待验证提交推送。代码与归档可为独立本地提交，但合批推送，避免仅为归档制造一个已成功之后的新 head。
- 需要提前推送获得云端生成修正或真实反馈时仍可进行；本次优化不要求一次提交、一次 CI 或一律一个 PR。后续真实修复仍按新 head 验证。
- 归档保留待定事实，例如“最终 head CI 待验证”，不得提前写成成功。合并后才产生的 merge SHA、main CI 和清理结果保存为 PR/Actions 可追溯事实及主工作树归档中的带时间观察，不为回填归档自身结果递归提交。
- 主代理不提前执行收尾归档；沿用 gkd-closeout 的单角色连续收尾方式。本项目文档只落实顺序，不修改用户级 GKD skill 或其他项目规则。

风险与观察：最后阶段若仍有真实代码修复，额外验证是必要成本。验收比较新增 head 的原因和归档是否已合批；发现归档缺失则补齐真实材料并验证该新 head，不复用旧成功冒充最终检查。

### 3.2 P1：CodeQL 复用分类器跳过纯文档分析

仅在 `.github/workflows/codeql.yml` 增加一个自动事件使用的轻量 `change-scope` job，复用 `node scripts/ci-change-scope.mjs` 及 `.github/ci-scope.json`，不新增分类脚本、白名单或公共 workflow 抽象。

1. `change-scope` 仅在 `push`、`pull_request` 运行，使用完整 Git 历史、Node 22 和与主 CI 相同的 event/base/head/before 输入。输出 `scope`、`frontend_ci`、`rust_ci`，只需要 contents read 权限。
2. `analyze` 依赖 `change-scope`。只有分类 job 成功、scope 为 `process-docs` 或 `checked-docs`、两个源码输出均为字符串 `false` 时，才证明可跳过分析。
3. job 条件固定为 `${{ !cancelled() && !(needs.change-scope.result == 'success' && (needs.change-scope.outputs.scope == 'process-docs' || needs.change-scope.outputs.scope == 'checked-docs') && needs.change-scope.outputs.frontend_ci == 'false' && needs.change-scope.outputs.rust_ci == 'false') }}`，解除 skipped 依赖传播并尊重整次取消。定时/手动事件的分类 job 按条件跳过，分析仍执行。分类 job 失败或输出缺失且整次 workflow 未取消时也选择双语言分析，同时保留原分类失败结果，不伪装成功。
4. 保持 JS/TS 与 Rust 两个固定 matrix 项、`build-mode: none`、`fail-fast: false`、45 分钟 timeout、已固定的 action SHA、分析输入、现有触发/并发和分析权限。源码 PR 即使只改一端也保留两种语言，不引入语言裁剪。
5. `assertCodeqlContract` 更新为接受并验证新增 job、输入、needs 与纯文档条件，仍校验每个选中分支的 checkout/init/analyze 必须执行、不能忽略失败。复用现有检查器的解析能力，不重写通用 YAML 解析器。

收益与风险：以一个廉价分类 job 替代纯文档的两种语言分析；源码事件增加一次轻量分类等待。主要风险是错误证明纯文档，故必须验证完整 PR diff、删除改名与异常场景。若出现源码漏选、定时/手动漏跑或分析输入变化，回退 CodeQL 调度及对应合同到原无条件双语言结构。

### 3.3 P1：合同成功后启动重任务

在 `.github/workflows/ci.yml` 中将 frontend、rust、observer-macos 的 `needs` 改为 `[change-scope, contracts]`，各自 `if` 保留现有 `always()`、分类成功和对应源码选择，并增加 `needs.contracts.result == 'success'`。

- contracts 的选择条件、内部检查和 job 本身保持不变；合同成功后，三个选中重任务仍相互并行。
- 保留 manual-dispatch-guard 的自动 skipped 语义，避免其沿 needs 链传播导致自动 CI 漏跑。
- `ci-gate` 的 required 名称、always、needs、结果绑定和 fail-closed 聚合脚本保持不变。合同失败导致选中重任务 skipped 时，gate 仍检查合同与选中任务必须成功，从而失败。
- candidate-plan 仍可与合同并行；候选构建已有 contracts/frontend/rust 成功前置条件，保持原链路、矩阵和发布职责。
- 更新质量合同的 `CI_JOB_CONDITIONS`，并明确检查三个 job 的 needs。更新云端合同和治理文档中“frontend 与 contracts 并行”等现行描述。

收益与风险：合同失败后重任务不分配 runner，gate 可更快得出失败。成功路径会增加合同和调度等待，预期约为短合同耗时加新增调度开销，不能只用 8-12 秒视作保证。通过自然运行观察；若成功路径等待增加抵消实际收益，回退三项依赖和相应合同，保留既有严格聚合判定。

目标 DAG：

```text
change-scope -> contracts -> frontend / rust / observer-macos（按原域选择，并行）
change-scope -> candidate-plan
contracts + frontend + rust + candidate-plan -> 原候选矩阵 -> assemble
全部原依赖结果 -> ci-gate（always，严格判定）
CodeQL 自动事件 change-scope -> analyze ×2（只有纯文档证明可跳过）
CodeQL schedule/manual -> change-scope skipped -> analyze ×2
```

## 4. 文件范围与目标覆盖

实现允许修改：

| 文件 | 改动职责 |
| --- | --- |
| `.github/workflows/codeql.yml` | 复用分类器的轻量 job 与 analyze 条件 |
| `.github/workflows/ci.yml` | frontend/rust/observer-macos 的 needs 与合同成功条件 |
| `scripts/check-ci-quality-gates.mjs` | `assertCodeqlContract`、`CI_JOB_CONDITIONS` 和直接依赖合同 |
| `scripts/check-ci-quality-gates.selftest.mjs` | 对应调度和失败传播的必要正反例 |
| `docs/operations/github-actions-governance.md` | CodeQL 分类、合同前置、归档与最终验证顺序 |
| `.trellis/spec/aio-coding-hub/cross-layer/cloud-only-verification-contract.md` | 更新重任务依赖事实与校验职责 |
| 本任务 `.gkd/` 活动及归档 Markdown | 授权、交接、审查和收尾记录 |

`.github/ci-scope.json`、`scripts/ci-change-scope.mjs` 及其 selftest 作为复用输入和已有验证，不计划修改。其余 checker 保持现状；若现有合同确有必要联动而超出上述范围，由 main 记录原因并决定 PLAN 范围更新。

| 比较范围/事件 | 主 CI 目标 | CodeQL 目标 |
| --- | --- | --- |
| 纯过程文档 | 分类和 gate；contracts 与重任务 skipped | 自动分类成功后 analyze skipped |
| README、docs/spec Markdown 等受检文档 | 分类、contracts、gate；重任务 skipped | 自动分类成功后 analyze skipped |
| 纯前端 PR | 合同成功后 frontend | 两种语言 |
| 纯 Rust PR | 合同成功后 rust 与 observer-macos | 两种语言 |
| 共享/锁文件/构建配置/生成器/workflow/未知/混合 PR | 合同成功后两端与 observer-macos | 两种语言 |
| 含源码的 main/dev push | 既有全量行为，合同成功后重任务 | 两种语言 |
| 删除或跨域改名/复制 | 复用原路径及两端并集，保持必要覆盖 | 涉及任意源码/未知路径即两种语言 |
| 空 diff、分类异常 | 既有全量或明确失败，不产生假成功 | 两种语言；分类 job 真失败仍保留失败 |
| CodeQL 定时/手动 | 不改变主 CI 的独立手动边界 | 分类 job skipped，双语言执行 |

## 5. 必要验证

### 本地边界

拟计划阶段仅检查计划差异与引用，未运行实现验证。执行阶段仅允许 `git diff --check`、Git/gh 只读检查和必要的无依赖语法检查；不安装依赖、不运行包脚本、完整测试、编译、格式/绑定生成、打包或长时性能检查，不绕过 Actions guard。

### GitHub Actions

复用自然触发的原任务 PR CI，不为同一 head 再 dispatch 手动 CI。必要检查由既有 contracts 执行：

- `node scripts/ci-change-scope.selftest.mjs`
- `node scripts/check-ci-quality-gates.selftest.mjs` 与 `node scripts/check-ci-quality-gates.mjs`
- `node scripts/check-cloud-only-verification.selftest.mjs` 与 `node scripts/check-cloud-only-verification.mjs`
- 既有 action pin、文档链接及其他合同检查；源码任务继续执行原 frontend、rust、observer-macos。

在现有质量 selftest 内添加与本次行为直接相关的证明，不为每个文档文件新增测试：

1. CodeQL 正确复用相同分类命令、Git 历史和事件输入；纯过程/受检文档可跳过，两域源码/混合/共享/未知必须运行。通过既有 classifier fixtures 验证删除、跨域改名和完整 PR diff；新增调度断言验证这些输出如何决定 job。
2. schedule/manual 的分类 skipped 不阻止分析；分类失败、取消及输出缺失不成为纯文档证明；init/analyze 不得静默跳过或忽略失败。
3. 三个主 CI 重任务确实依赖 contracts 并检查成功；合同成功但对应域未选中仍跳过，合同失败/取消/skipped 不启动重任务。
4. 聚合脚本在所需合同或任务失败/取消/异常 skipped 时不能成功；纯文档正常 skipped 可成功。复用实际聚合脚本和现有 fixtures，不另造平行 gate 实现，也不改 approved script digest 来放宽判定。

云端通过条件：最终 PR head 的 `ci-gate`、`pr-title` 成功；本任务改动 workflows/scripts，必须实际执行全量主 CI 和 CodeQL 两语言，不能用 skipped 代替。记录 PR head、checkout 实际测试 merge SHA、run/attempt、job 结果和未覆盖范围。纯文档及失败传播由必要 fixtures 先证明，后续自然运行再观察效果，不专门造红 PR、空提交或额外演练 workflow。

## 6. 执行、交付与归档

本节已由用户“automatic 模型执行这个 PLAN”批准；最初“写 PLAN”阶段没有开展以下动作。

1. 批准后 main 重新核对远端 main 和规则；从当时确认的远端 main 创建 sibling worktree 和独立任务分支，记录完整 baseline。当前本地主干独有提交和其他任务脏文件不带入执行分支，也不重置或清理。
2. main 生成自包含 execution 后，以 `gkd_execute`、`fork_turns=none` 交接单 writer 实施、必要验证记录和中文本地提交。范围或授权未变化的真实失败修复继续同任务处理。
3. writer 停止后，由 `gkd_accept` 独立审查实现及已有证据；main 作最终审查决定。可以先完成代码审查并注明最终 CI 待定，再交接收尾，避免等待成功后才准备归档。
4. main 一次性交接 `gkd_closeout`：在执行 worktree 准备 `.gkd/archive/ci-scheduling-cost/2026-09-08-r1/` 的本任务必要记录、迁出已跟踪活动正文，然后合批推送最终待验证 head。不要将本地主干计划提交的父历史合入执行分支；归档以本任务文件和事实为来源。
5. 已批准的交付范围包括：中文本地提交、推送本任务分支、创建或更新一个面向 main 的 PR、等待最终 head 的 required 与本任务 expected 检查、通过后绑定完整 head 正常合并、等待实际 main merge SHA 的自动主 CI 与 CodeQL。不得绕过保护或以其他 SHA 结果替代。
6. 本次无需版本变更、tag、Release、签名候选或本地安装；不主动 dispatch、重跑或取消 CI，不修改 required checks、ruleset、merge queue、runner、仓库权限和平台设置。新增 CodeQL 分类 job 只使用上文约定的 contents read；分析权限保持原样。失败需真实修复时提交新 head 让自动 CI 验证。
7. 最终验证、merge SHA 和清理结果在主工作树同任务归档中按时点补记并引用 PR/Actions；不为回填这些结果递归提交。默认清理只覆盖本任务临时 worktree、本地/远端任务分支及已独立保存的本任务活动记录；仅在 writer 停止、必要交付完成、成果实际合并、无新增待保留内容后清理。
8. 终点是实现、必要云端验证、独立审查、合并和本任务归档/清理均有事实。外部交付受阻则保留成果与未完成项，不以启动、交接或旧 head 成功宣称完成。

## 7. 消融审查

- 只保留三个有证据的改善项，已按用户要求移除 Rust 测试/benchmark 并行化。
- 复用一个分类器和现有质量 checker/selftest，不新增路径策略、语言矩阵、通用 workflow 或 CI 取消机器人。
- 保留实际源码比较基准和必要失败判定，不引入跨 head 继承绿色结果的捷径。
- 归档规则只写入现有项目治理文档并用于本任务收尾，不修改通用 GKD，不要求所有任务固定一个 PR。
- 不追加缓存、runner、发布或业务代码重构；不把观察到的单次耗时写成承诺的提速比例。
