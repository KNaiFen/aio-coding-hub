# 差异审查报告

## 执行摘要

| 严重级别 | 数量 |
| --- | ---: |
| Critical | 0 |
| High | 0 |
| Medium | 0 |
| Low | 0 |

- 总体风险：Medium。风险来自网关路由资格和错误语义变化，不是已发现缺陷。
- 当前建议：Conditional。允许提交并进入 PR；只有 GitHub Actions 原生检查通过，且合并前最新 `origin/main` 的代码与业务影响审查通过，才允许合并。
- 分析文件：27/27。
- 高风险生产路径：限额计算、候选过滤、Session 绑定、failover 收口，共 100% 人工审查。
- 已发现安全回归：0。

## 变更内容

基线为建分支时的 `origin/main`：`ca15f02b`。当前差异包含 3 个已提交变更和待提交的路由实现，共 27 个文件，约 `+1016/-159` 行。

| 范围 | 风险 | 影响 |
| --- | --- | --- |
| Tray 成功/失败计数布局 | Low | 固定宽度重新分配，加入明确分隔线，不改变数据来源 |
| 限额判断抽取与前置过滤 | High | 已知 OAuth/消费限额供应商不再进入 Session 偏好和尝试序列 |
| 发送前限额复检 | High | 覆盖选择后限额变化的竞态，不生成伪 attempt |
| 全限额错误收口 | High | 返回 `GW_NO_ENABLED_PROVIDER`、空 attempts、无 `Retry-After` |
| 路由测试与跨层契约 | Medium | 锁定 forced、managed、Session、混合 gate 和真实 429 边界 |
| 既有前端测试格式化 | Low | 仅 Prettier 机械换行，无行为变化 |

## 关键发现

未发现 Critical、High、Medium 或 Low 级别的待修复问题。

已在审查中修正一项性能回归：普通 API Key 且未配置消费限额的供应商会直接跳过限额数据库过滤，不会为每次请求新增无意义的数据库连接。

## 测试覆盖

- OAuth 已耗尽：高优先级供应商零调用、零 attempt；低优先级供应商作为第一个真实请求并保留 Session 复用标志。
- 消费总限额已耗尽：高优先级供应商零调用；后续供应商 `provider_index=1`。
- 全部已限额：HTTP 503 `GW_NO_ENABLED_PROVIDER`、空响应/持久化 attempts、无 `Retry-After`，并清理不再合格的 Session 绑定。
- 限额与熔断混合：仅熔断供应商保留 skipped 审计，最终仍是 `GW_ALL_PROVIDERS_UNAVAILABLE`。
- forced provider 与 managed model：已限额时不越权回退到其他路由成员。
- 真实上游 429：仍是实际失败 attempt，并按既有策略切换供应商。
- Tray：极值计数、固定轨道和分隔结构由组件测试及跨层常量测试覆盖；404x234 明暗主题视觉验证无溢出。

本地允许范围内的验证结果：307 个测试文件、2693 个测试通过；TypeScript typecheck、ESLint、Vite build、Prettier check、spec link check、gateway error-code sync 和 `git diff --check` 通过。

仓库规则禁止本地运行 Rust 编译、rustfmt、Clippy 和 Rust 测试，因此原生路由测试和编译正确性必须由 GitHub Actions 验证，未通过前不得合并。

## 爆炸半径

`ProviderResolutionMiddleware::run` 是所有网关请求的候选解析入口，属于高影响路径。过滤只在候选包含 OAuth 或任一消费限额配置时启用，并使用一次 blocking-pool 任务和一个数据库连接；数据库或 blocking-pool 故障保持原有 fail-open 行为。

`evaluate_provider_limits` 复用原有 OAuth snapshot、5h、daily、weekly、monthly 和 total 计算，未放宽或删除计算条件。发送前复检继续覆盖选择与实际发送之间的状态变化。

`failover_loop` 的新错误收口仅在 attempts 为空、Ready 供应商数为 0、至少发生一次限额排除、且没有熔断或 cooldown 排除时触发，不会吞掉真实上游失败或其他 gate 的审计记录。

## 历史上下文

被删除的 `provider skipped by rate limit` attempt 最初随 failover 模块拆分保留，后续错误码同步提交只强化了跨层枚举一致性；它不是安全修复或权限校验。稳定的 `GW_PROVIDER_RATE_LIMITED`、`REASON_RATE_LIMITED` 和旧 attempt 解析仍保留，避免破坏历史日志兼容。

未发现已修复漏洞代码被重新引入，也未发现鉴权、密钥输出、请求体日志或外部输入校验被削弱。

## 对抗性检查

- 限额在路由选择后、发送前变化：发送 gate 再次判断，不调用上游，也不伪造重试。
- 限额数据库不可用：候选 fail-open，避免基础设施故障静默关闭整个路由。
- 强制命中已限额供应商：返回无启用供应商，不绕过 forced 约束。
- 托管模型绑定供应商已限额：不跨供应商发送相同远端模型。
- 限额与熔断同时存在：限额候选不可见，熔断候选仍可审计。
- 上游实时返回 429：不被误判为预先已知限额，保留真实 attempt 与 failover。

## 建议与合并门槛

- [x] 完成本地前端验证和视觉验证。
- [x] 完成当前分支五轴差异审查。
- [ ] GitHub Actions 的 Rust tests、rustfmt、Clippy、生成绑定和原生构建全部通过。
- [ ] 合并前重新 fetch `origin/main`，逐提交审查其代码与业务影响。
- [ ] 若主线有漂移，普通 merge 最新 `origin/main`，解决文本和语义冲突后重跑验证与 CI。
- [ ] 无法同时保留主线和本分支业务语义时停止合并，交由用户处理。

## 分析方法

- 策略：Deep。生产代码变更少于 20 个文件，逐个 diff 区域审查，并用 CodeGraph 跟踪候选解析、Session 绑定、限额 gate、错误收口与真实 429 路径。
- 技术：基线/当前版本对比、历史 blame/log、调用链与爆炸半径分析、测试矩阵映射、对抗性边界推演、前端全量验证和实际像素检查。
- 限制：依仓库政策未在本地编译或执行 Rust；此项由 CI 补足。
- 置信度：当前分支业务语义为 High；原生编译为 Pending CI；最终主线兼容性为 Pending pre-merge review。

## 主线差异审查

### 第一次同步

- 建分支基线：`ca15f02bd6b409df08f2892b5e5082bcc42aa3b3`。
- 审查时 `origin/main`：`523256fc4108f03731bedb3962ff1d88acab01f4`。
- 新增历史：`6dc8a96d`、`4b7d4c35` 和 PR #34 merge commit `523256fc`。
- 变更范围：6 个 `.trellis/tasks/08-03-codex-stream-internal-error-retry` 文件，内容为完成证据、状态更新和 active 到 archive 的重命名。
- 产品代码、GitHub Actions、依赖清单、共享 `.trellis/spec` 和本任务目录变化：0 个文件。
- 与本分支直接重叠文件：0 个。
- 业务影响：无新的运行时行为。归档文档描述的是已存在于基线 `ca15f02b` 的流内错误重试功能，不修改其代码或契约。
- 冲突预演：`git merge-tree` 未发现文本冲突；不存在需要组合的业务语义。
- 处理结果：通过普通 merge 合入，merge commit `b026cc6b9d71738572024dae8075571d97373d02`，未做冲突补丁。

结论：第一次主线同步可接受。PR 最终合并前仍须重新 fetch；若 `origin/main` 再次移动，必须追加审查记录并重新验证，不能仅依据 CI 结果合并。
