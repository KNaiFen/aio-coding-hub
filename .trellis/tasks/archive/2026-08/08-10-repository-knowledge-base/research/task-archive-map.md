# Trellis 活跃任务归档映射

## 归档范围

- 早期遗留 5 项：onboarding、Codex 模型发现、session reuse、cloud-only 制品晋升、首页 tooltip 密度。
- 上游选择性集成 6 项：父任务与 5 个已进入 PR #30/#98 的子任务；Claude OAuth 候选保留。
- 请求可观测/供应商状态 6 项：父任务与 5 个子任务，主体已由 PR #52 合并，后续由 PR #73 / `0.60.49` 收口。

合计 17 个目录。

## 主要交付证据

- Codex model discovery：`33b21e56` 已是 `main` 祖先。
- Session reuse：`fc41acf0`、`24d20ce0` 已是 `main` 祖先。
- Cloud-only 制品：PR #6-#9；后继零产物合同 PR #86 已归档。
- Home tooltip：PR #12，merge `7a5a150b`。
- 上游选择性集成：PR #30 merge `003b7b17`；Provider 趋势等最终进入 PR #98 merge `66b97166`。
- 请求可观测树：PR #52 merge `eeccf64d`，后续正确性收口 PR #73 merge `2a79978c`。

## 保留任务

`08-03-upstream-claude-oauth` 的验收要求真实隔离账号完成登录、exchange、refresh 与 401 refresh。该验证未发生，分支也未进入 `main`；它保留为独立 planning 任务，base branch 改回 `main`。
