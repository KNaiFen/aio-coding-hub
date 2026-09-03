# Token 速度估算补全

## 目标

降低请求详情中 Token 速度显示为未知的频率，同时保持现有精确速度和聚合统计口径不变。

## Worktree

- 路径：`/Users/knaifen/Documents/Codex/aio-coding-hub/aio-coding-hub-token-speed-estimate`
- 分支：`feat/token-speed-estimate`

## 范围

- 为 TUI 复用统一的速度判定：优先显示精确最终上游尝试速度。
- 当请求成功、输出 Token 和总耗时有效，但精确最终尝试计时不可用时，在 TUI 请求详情显示总耗时估算值，格式为 `≈12.4 t/s`。
- 估算值不写入日志字段，不进入 summary、排行、趋势或日聚合。
- 补充最小测试覆盖精确值优先、估算值和不可估算场景。

## 非目标

- 不改变 `final_upstream_attempt_duration_ms` 的采集语义。
- 不把失败、中断、重试、多次尝试或无输出 Token 的请求标记为估算成功。
- 不修改数据库 schema、聚合 SQL、桌面端展示或历史数据回填。

## 行为约束

- 估算仅允许成功终态且 `output_tokens > 0`、`duration_ms > 0`。
- 精确计时有效时始终使用精确值，不显示 `≈`。
- 估算只用于 TUI 请求详情；请求卡片和聚合仍保持现有行为。

## 可判定验收标准

1. 精确样本仍显示现有 `t/s` 数值，数值不变。
2. 成功但缺少精确最终尝试计时、拥有正输出 Token 和正总耗时的请求详情显示 `≈<value> t/s`，例如 `≈12.4 t/s`。
3. 失败、中断、无输出 Token、无总耗时或非正耗时请求详情不显示估算值。
4. 聚合数据和持久化字段不包含估算值。

## 允许的验证

- 仅运行仓库规定的零依赖检查：`scripts/gkd-verify --base-sha <full-lowercase-sha>`。
- 不运行依赖安装、package manager、测试运行器、lint、类型检查、构建、Cargo/Tauri 或开发服务器。
