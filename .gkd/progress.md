# 自定义定价执行进度

日期：2026-09-08。Execution r4，基线 `b147ced0f8cab342ee57925a5bd6b08d37e061db`。
唯一 worktree：`/Users/knaifen/Documents/Codex/aio-coding-hub/model-price-rules`。
分支：`feat/model-price-rules`。范围内实施与本地提交已获批准。

## 实施事实

- 独立 `model-prices/price-rules.json` v1 DTO，精确 CLI/完整模型键、原子替换、读写错误传播、重复键与非负有限数值校验。单价最多 9 位小数，倍率最多 6 位小数，范围均为 0 到 1,000,000；保存边界拒绝整体/分项同时存在。
- 新增规则 get/set 和参考价 IPC，注册到共享命令表。参考价返回实际参考模型、五类计费项和标准/优先/长上下文变体；精确失败后才查别名。
- 共享费用函数先解析原参考价格和 Token 口径，再逐项应用固定价或倍率，之后应用原供应商倍率。实际计费身份沿用 effective_cost_basis。原模型与别名目标规则最多选一条。
- 每批读取规则。首次终态后费用和供应商倍率保持，包括 NULL/0；详情清理后仍从账本判断终态。迟到 pending 不降级终态账本。历史补齐继续走 reference-only 入口，无新规则参数。
- 零价详情保留 Some(0)，排行榜覆盖统计包含 0，已有覆盖的总费用可以返回 0。
- 设置新增自定义定价入口，保留定价匹配。规则列表与单个编辑表单支持新增、编辑、启停、删除、保存、取消；整体和五项输入全部常驻且只做互斥校验，不自动清空。草稿独立于查询刷新，读取失败阻止保存，保存失败保留草稿，保存中禁止关闭。
- 增加模型规则、核心计费、参考价、实时落库与账本、历史补齐、零价排行榜、service/query/settings 和编辑器真实工作流回归。新增跨层合同并更新索引。

## 定位修正与判断

- 账本终态错误标记为 `error_present != 0`，详情表为 `error_code IS NOT NULL`；按各自既有字段判断，未新增终态定义。
- `summary.rs` 与 `folders.rs` 的覆盖计数已按非 NULL，保持原实现。同步仅写参考价表与 basellm-cache，未修改同步流程。
- 规则读取失败记录 error，并继续保存请求事实；该批新费用为未知，不静默回退参考价收费。已终态费用仍保留。此行为在合同及回归中明示。
- 消融审查已移除重复 IPC 转换、冗余结果空值分支和无实质价值的同步文件模拟断言。未增加模式字段/切换、通用规则引擎、历史版本表、重算任务或无关重构。

## 实际验证证据

环境：macOS，本 worktree，POSIX shell；仅 Git/只读源文件检索与 Node 内置模块合同。无依赖安装和构建产物。

以下命令均显式以本 worktree 为工作目录执行：

| 命令 | 结果 |
| --- | --- |
| `git status --short` | 退出 0；初始只有 main 交接的未跟踪 execution，后续修改均在批准范围 |
| `git diff --check` | 实施中及最终代码修正后退出 0，无输出 |
| `git diff b147ced0f8cab342ee57925a5bd6b08d37e061db --stat` | 退出 0，核对修改范围；新文件随后纳入提交 |
| `git diff b147ced0f8cab342ee57925a5bd6b08d37e061db -- <本次具体路径>` | 退出 0；分批检查费用计算、落库、零价路径、IPC、query/service 和设置接线 |
| `node scripts/check-cloud-only-verification.mjs` | 最终代码修正后退出 0，输出 `[cloud-only-verification] repository contract passed` |

仅读取必要规则、合同、源文件；定位使用限定目录的 rg 和 FastCtx read。没有执行 package-manager、测试、lint/typecheck、build、Cargo、Tauri、格式化、生成器、服务器或浏览器。未运行任何后台任务。

## 云端待办与风险

- `src/generated/bindings.ts` 未手改，也未运行生成器。前端已按新增 Rust DTO/命令接入，因此此实现提交仍依赖对应 head/attempt 的自动 Rust CI 生成绑定修正产物。当前工作树不能视为前端可编译证据。
- Rust/TypeScript 格式、Clippy、类型检查、业务测试、覆盖率、构建和实际窗口视觉验证均未运行。上述回归只有源码证据，需自动云端 CI 执行；Rust 格式漂移仅接受对应 head/attempt 的受限产物。
- 未执行真实同步网络场景；同步不覆盖规则的结论来自独立路径与写入调用的源代码检查。
- 未作独立验收。批准范围内实施可交回 main；后续生成绑定、CI 和独立技术验收不能由本地静态通过替代。

## 提交与停止边界

本任务按批准许可作一个中文 Conventional Commit：`feat: 支持模型自定义定价与倍率`。
仅暂存本任务代码、测试、合同及本进度，main 所有的 execution 保留未跟踪且不修改。
提交后停止交回 main；不推送、PR、合并、发布、清理或派生代理。

## r4.1 四项审查返工

日期：2026-09-08。唯一 worktree 和分支同上；本轮从初次实现提交
`9a4ec1119074a24584a5f8bb1bb3156aef634e23` 继续，依据 execution r4.1 第 17-26 行。
初始 Git 状态仅有 main 所有的未跟踪 `.gkd/execution.md`，本轮未修改或暂存该文件。

### 实施与回归源码证据

1. `src-tauri/src/domain/cost.rs` 在各 CLI 扣除缓存后的普通输入 Token 统一应用 `.max(0)`，固定输入价不能产生负费用；原参考价路径的正数门槛保持。`fixed_input_price_clamps_oversubscribed_cache_buckets` 覆盖 Codex/Grok 输入 100、read 80、write 50 时为 0.38 USD，Gemini 输入 100、read 130、write 50 时为 0.43 USD，以及 Claude 累加仍为 0.78 USD；同时覆盖总写入与 5m 明细口径。
2. `src-tauri/src/infra/request_logs.rs` 仅在别名目标 `price_json.is_some()` 时切换 `priced_model`。`model_rules_only_use_alias_context_premium_after_reference_lookup_succeeds` 使用隔离临时应用目录中的 `custom -> missing-1m` 别名，断言原模型自定义输入 2/输出 10 USD 每百万、300k cache write 5m 在无参考价时为 0.75 USD；插入目标参考价后仍按目标 1m 名称计为 1 USD；停用原规则后目标倍率 3 接管为 3 USD。每个场景同时断言详情与账本。
3. `ModelPriceRulesDialog.tsx` 从当前候选规则集选取启用的原模型规则，缺失时只按参考查询实际返回的参考模型寻找启用目标规则；固定价、倍率和缓存继承预览共用这一条选择结果。交互回归点击停用当前规则，断言目标输入参考 2、倍率 3 显示 6，并保留 priority/长上下文变体；覆盖无参考价、精确参考价、目标在未保存候选集内停用，以及保存时原规则字段保留。
4. `src/query/keys.ts` 增加 `references()` 前缀；别名保存成功仅新增该参考查询前缀的失效，原别名缓存更新/失效保持。查询回归先读取 `model-a -> target-a` 的五分钟新鲜缓存，再保存 `target-b` 别名并重新挂载同一模型查询，断言重新读取目标名称和价格，且失效仅涉及 references/aliases。已有草稿交互回归增加参考刷新阶段，断言模型名和自定义输入价 7 保留、参考目标更新后生效价仍为 7。

### 本轮检查与判断

环境：macOS、指定 worktree、POSIX shell。仅 Git、只读文件检索和 Node 内置模块静态合同；未启动后台任务。

| 实际命令 | 实际结果 |
| --- | --- |
| `git status --short` / `git status --short --branch` | 退出 0；确认 `feat/model-price-rules`，返工仅 8 个批准代码/测试路径，execution 保持未跟踪 |
| `git diff 9a4ec1119074a24584a5f8bb1bb3156aef634e23 -- <上述 8 个路径>` | 分为 Rust 和前端两组执行，均退出 0；人工核对本轮完整差异 |
| `git diff b147ced0f8cab342ee57925a5bd6b08d37e061db -- src/query/keys.ts src/query/modelPrices.ts` | 退出 0；核对原基线至当前参考查询/别名保存变化 |
| `git diff b147ced0f8cab342ee57925a5bd6b08d37e061db --stat` | 退出 0；代码修正后累计 30 个文件，均在原批准范围，未新增依赖或生成产物 |
| `git diff --check` | 代码与回归修正后退出 0，无输出 |
| `node scripts/check-cloud-only-verification.mjs` | 代码与回归修正后退出 0；`[cloud-only-verification] repository contract passed` |

消融判断：仅保留一个 Token 下界、别名名称赋值时机修正、组件内两次精确查找和参考查询前缀失效。没有新增规则引擎、模式/面板、跨请求缓存、辅助持久化或无关重构。未降低或删除原有断言。

### 未覆盖与提交

- 上述业务回归均只新增/修改源码，未在本地运行；Rust/前端测试、格式、类型、Clippy、构建、窗口交互均待普通 PR 自动 CI。没有执行安装、包管理器、服务器、格式化或生成器。
- `src/generated/bindings.ts` 延续前轮待云端状态；本轮未触及 IPC/DTO，也未本地生成或声称可编译。无可复用的对应 head 云端业务验证证据。
- 本轮无材料性范围偏差或未处理 finding；本地静态通过不等于独立验收。获准本地提交说明为 `fix: 修正自定义定价与预览计算`，仅暂存本轮 8 个代码/测试文件及 progress。提交后交回 main 并停止，不推送或开展后续生命周期操作。

## r5 发布准备（main）

- 用户于 2026-09-08 授权推送、PR、合并、发版；两位实施 writer 和两位 accept 已停止，main 已通过 r4.1 独立代码审查。
- main 按 PLAN r5 将五份包/应用 manifest 及 Cargo.lock 中三个本地包统一为 `0.60.60`，新增 CHANGELOG 的本次定价能力说明。未修改依赖或发布工作流。
- main 检查本轮完整 diff，只有七个版本/说明路径；`git diff --check` 和 `node scripts/check-cloud-only-verification.mjs` 均退出 0。消融审查无多余改动。
- 业务测试、生成绑定和构建仍待普通 PR 自动 CI；本轮没有执行本地测试或生成器。
