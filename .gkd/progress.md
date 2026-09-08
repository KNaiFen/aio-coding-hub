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
