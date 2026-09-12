> 归档快照：2026-09-08 12:29 +08:00。来源：主工作树，作者 main。保留原记录日期、观察时点及历史结论；本机绝对路径与运行时角色标识已脱敏。下文的活动路径是原始来源说明，五类完整记录均已保存于本目录。

# 模型自定义定价与倍率审查

## 当前结论

- 日期：2026-09-08；PLAN r5，业务 execution r4.1；最终实现 CI 观察于 12:25 +08:00。
- 当前 head：`173ce95b66ff674d67f5e802e4c466125215aa85`，PR #193；基线 `b147ced0f8cab342ee57925a5bd6b08d37e061db`。
- 返工 writer `返工施工角色` 已提交并停止；独立 `返工独立审查角色` 已只读复查并停止，四项 findings 全部解决，未发现新的可操作问题。
- main 决定：实现、独立审查及必要云端验证通过，允许进入一次 gkd_closeout 连续归档、最终提交检查、合并及发布。所有 writer/accept/独立 CI 角色已停止；尚未宣称合并或发布完成。
- 修复证据：`cost.rs:454` 将普通输入最低设为 0；`request_logs.rs:961` 查到别名目标参考价才改名；`ModelPriceRulesDialog.tsx:97` 以候选规则集解析实际接管规则；`query/modelPrices.ts:90` 在别名保存成功后失效参考查询。相应回归源码覆盖原失败场景和受影响组合。
- 已有静态检查：两轮 writer 的 `git diff --check` 与 `node scripts/check-cloud-only-verification.mjs` 均退出 0；本次独立复查未重跑测试，main 不重复全量技术审查。
- 最终云端证据：CI run `34184637399` attempt 1 成功，head 同上；contracts、frontend、rust、observer-macos、ci-gate 全成功，独立监控确认 required+expected 的 ci-gate/pr-title 全成功且无缺失。frontend 313 测试文件通过及构建通过；Rust 主库 2969 passed / 0 failed / 4 ignored，workspace/集成测试、Clippy、绑定一致性、audit 与必要 benchmark 通过。未运行本地或桌面窗口手工视觉验证，不把源码审查当视觉证据。
- 权限边界：用户于 2026-09-08 明确授权推送、PR、合并和发版；允许 `feat/model-price-rules` 普通 PR、CI 必要修正、通过后 squash 合并及发布 `0.60.60`。本地主线分叉保留，不直接推送远端 main。
- 当前保留主工作树 PLAN/plan-changes/review 与执行 worktree execution/progress，交给收尾角色保存脱敏归档；本任务五类独占活动记录在必要交付完成且归档可访问后可精确移出。保留所有任务分支/worktree及其他任务旧归档；本地主线分叉内容已比较，禁止重置或合入 squash 前分支。
- r5 发布准备由 main 直接处理并审查：七个版本/说明路径仅更新为 `0.60.60` 及本功能 CHANGELOG；差异检查与零依赖合同退出 0。无业务逻辑或依赖变化，不重复 delegated 全量审查。
- 首轮 CI run `34182820598` attempt 1：head `3c114408c1f6033a833e69a7588236dd0aab1f1a`，frontend 测试通过，构建缺绑定；Rust 因生成漂移停止。main 已完整审查并接收对应 artifact，仅含九个 Rust 文件格式及新增命令/DTO 绑定，通过允许的两项静态检查；无需重复业务审查，新 head CI 待定。
- 第二轮 CI run `34183291396` attempt 1：head `185177ef3fc33100567f071bad32f03469751857` 的 frontend 全通过，Rust Clippy/生成一致性通过；两个新回归因单连接测试池占用超时失败。main 阅读失败测试完整函数及相邻模式，直接修正七行连接生命周期并逐入口复查；生产代码/断言不变，两项静态检查通过。此局部测试修正按 direct-main 处理，无需重复全量业务验收，继续新 head 云端验证。

## 初次审查（已由当前结论替代）

- 日期：2026-09-08；PLAN r4，首次 execution r4。
- 基线：`b147ced0f8cab342ee57925a5bd6b08d37e061db`。
- 审查 head：`9a4ec1119074a24584a5f8bb1bb3156aef634e23`。
- writer `初次施工角色` 已完成提交并停止；独立 `初次独立审查角色` 已只读审查并停止。
- main 决定：返工。独立审查的四项 findings 均在已批准行为和范围内，main 已按出处抽查关键代码，无需修改 PLAN 或重新取得实施许可。
- 现有 `git diff --check` 和零依赖 Node 合同通过仅为静态证据。生成绑定、前后端 CI、格式、类型、测试、构建和视觉验证均待完成。

## 原 Findings（全部已解决）

1. P1，`src-tauri/src/domain/cost.rs:557`：缓存量大于总输入时 `billable_input_tokens` 为负，固定输入价分支将其当抵扣。扣除缓存后的普通输入必须最低为 0。覆盖 Codex/Grok/Gemini 的固定价回归；示例 Codex 输入 100、cache read 80、write 50，普通输入价每 Token 0.004、缓存价 0.001/0.006，费用应为 0.38 USD，当前为 0.26 USD。
2. P2，`src-tauri/src/infra/request_logs.rs:953`：别名目标无参考价格时仍改写计费模型名，导致自定义模型错误套用目标的 `1m` 溢价。只有目标参考价存在才采用目标名称；两端均无参考价时保留实际计费模型。示例 `custom` 输入 2/输出 10 USD 每百万，映射到无价 `missing-1m`，300k cache write 5m 应 0.75 USD 而非 1 USD。
3. P2，`src/components/settings/ModelPriceRulesDialog.tsx:190`：停用原模型规则后预览只显示参考价，漏掉后端会接管的别名目标启用规则。按当前候选规则集选择实际生效规则；补充参考输入 2、目标倍率 3，停用原规则后预览为 6 的交互回归。
4. P2，`src/query/modelPrices.ts:64`、`:82`：修改定价别名后仅失效 aliases 缓存，参考价仍可能在五分钟内显示旧目标。保存别名后失效相关参考价查询，并验证参考模型/费用更新且不覆盖用户草稿。

## 返工记录

- execution 更新为 r4.1，PLAN 仍为 r4；从上述实现 head 继续修复，使用新的单轮 `gkd_execute`，不复用已结束代理。
- 本轮只处理四项 findings 和必要回归，遵守原零依赖检查与本地提交许可，不推送或运行本地业务测试/生成器。
- 修复提交后仅复查受影响部分及既有 findings，不重复全量审查。
- 返工提交为 `d101deba2e649b815dd990466848fb5614debe97`，针对性复查已完成，结果见当前结论。
