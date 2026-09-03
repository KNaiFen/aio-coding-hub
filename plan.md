# Token 速度估算补全

## 目标

降低请求日志中 Token 速度缺失的频率，同时保持现有精确速度和聚合统计口径不变。

## Worktree

- 路径：`/Users/knaifen/Documents/Codex/aio-coding-hub/aio-coding-hub-token-speed-timing`
- 分支：`fix/token-speed-upstream-timing`

## 范围

- 在最终上游 attempt 的同一时间口径下保留候选计时：记录 attempt 开始到上游协议完成或干净 EOF 的观测时长。
- 精确计时不可用但候选边界完整且未发现错误时，持久化独立估算时长；速度使用 `output_tokens / estimated_final_attempt_duration`。
- TUI 与桌面请求日志展示统一优先精确值，再显示带 `≈` 的同口径估算值；聚合统计只使用精确字段。
- 补充最小测试夹具和静态一致性覆盖，确保字段从采集、持久化、观察协议到展示端贯通。

## 非目标

- 不改变 `final_upstream_attempt_duration_ms` 的采集语义。
- 不把失败、中断、无输出 Token 或不完整/受污染的请求标记为估算成功。
- 不使用整请求 `duration_ms`、`ttfb_ms` 或首 token 后生成时长作为估算分母。
- 不回填历史记录，不改变聚合 SQL 的精确字段口径。

## 行为约束

- 估算仅允许成功终态、`output_tokens > 0`、候选最终 attempt 时长 `> 0`。
- 精确计时有效时始终使用精确值，不显示 `≈`。
- 候选时长必须来自最终 attempt 开始到协议完成/干净 EOF；读取错误、流内错误、客户端中断或下游背压污染时不可用。
- 估算可用于 TUI 和桌面请求日志展示；不写入聚合输入、不替换精确字段。

## 可判定验收标准

1. 精确样本仍显示现有 `t/s` 数值，数值不变。
2. 成功但缺少精确最终尝试计时、拥有正输出 Token 和同口径候选时长的请求展示 `≈<value> t/s`，例如 `≈12.4 t/s`。
3. 失败、中断、无输出 Token、候选时长缺失或非正时长请求不显示估算值。
4. 估算时长独立持久化；聚合数据不使用估算值。

## 允许的验证

- 仅运行仓库规定的零依赖检查：`scripts/gkd-verify --base-sha <full-lowercase-sha>`。
- 不运行依赖安装、package manager、测试运行器、lint、类型检查、构建、Cargo/Tauri 或开发服务器。
