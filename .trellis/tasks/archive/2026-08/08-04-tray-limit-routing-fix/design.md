# 技术设计

## Tray 几何

- 原生窗口宽度维持 404px，行内固定轨道改为 `96px / 170px / 96px`。
- 总计区域使用两个 44px 子网格，中间放置 1px、`aria-hidden` 的浅色竖线；固定轨道剩余空间形成对称留白。
- 每个子网格继续使用 `12px / 32px` 标签和值轨道，确保 99,999 精确值和 `42.9亿` 等缩写不改变。

## 限额判定边界

- 从现有网关限额 gate 中抽出单一判定器，输入数据库连接、供应商与请求时间，输出 `Allow` 或 `Limited`；OAuth 与配置消费限额共用该结果。
- 判定查询保持当前 fail-open 语义。预过滤复用单个连接，仅对 OAuth 或配置了消费限额的候选执行数据库检查，避免引入展示读模型或 `usage_events` 热路径。
- 现有发送前 gate 调用同一判定器作为竞态保护，不维护第二套限额算法。

## 路由流程

1. 按现有默认/自定义/托管路线取得全局开启候选。
2. 应用 forced-provider 收窄，禁止越界回退。
3. 在 bounded blocking pool 中执行限额资格过滤，保持剩余候选顺序。
4. 仅使用过滤后候选解析 Session 绑定；缺失的旧绑定按现有机制清理。
5. 若候选为空，进入 `NoEnabledProvider` 早期收口，并在内部诊断中标记 `all_candidates_limit_excluded`。
6. 对剩余候选执行 circuit/cooldown、限额竞态复查、凭据与发送准备。

限额拒绝不写 `FailoverAttempt`。发送准备只有在所有 gate 通过并取得 Ready slot 后才递增 `providers_tried`，因此首个真实请求自然获得 provider index 1；attempt index 继续从真实 attempts 数组派生。

## 终态与兼容

- 预过滤全限额直接复用 `EarlyErrorKind::NoEnabledProvider`：503、`GW_NO_ENABLED_PROVIDER`、空 attempts、无 `Retry-After`。
- 若发送前竞态复查导致零 Ready provider，且没有真实 attempt、circuit/cooldown skip，只存在限额排除，则使用同一 NoEnabledProvider 契约。
- 若仍有 circuit/cooldown skip，继续使用 `GW_ALL_PROVIDERS_UNAVAILABLE`；若已有真实发送失败，继续使用现有 all-failed 收口。
- 首次真实上游 429 的 attempt 与 failover 不变。限额快照保存后，后续请求才由资格过滤消除无效首跳。
- 不变更数据库、IPC、生成绑定或前端实时重试展示逻辑。

## 可观测性与性能

- 限额资格过滤只写有界结构化 tracing 与现有 no-enabled 特殊诊断，不把供应商名称、凭据或额度详情放入客户端响应。
- `attempts_json` 只反映真实发送和仍需审计的非限额 gate skip；限额资格排除不再伪装成一次尝试。
- 路由候选上限仍为既有 512，预过滤复用一个 SQLite 连接；无 OAuth、无消费限额的供应商直接 `Allow`。

## 合并策略

- 分支基于创建时最新 `origin/main`。PR CI 通过后重新 fetch；若云端 main 前进，将 main 合入功能分支，逐项保留非冲突主线改动并重新验证。
- 只在最新主线兼容性与 CI 均确认后合并。若冲突涉及无法同时保留的产品语义，保留 PR 交由用户决策。
