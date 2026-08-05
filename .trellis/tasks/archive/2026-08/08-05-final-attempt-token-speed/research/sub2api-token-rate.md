# sub2api Token 速率参考

固定参考提交：`00b8596176809906993169c283671811ad04f58d`。

- `ops_repo_openai_token_stats.go` 使用 `output_tokens * 1000 / duration_ms`，并对请求速率求平均。
- `openai_gateway_forward.go` 在每次 `Forward` 内独立创建 start time。
- handler 的同账号重试、账号切换和退避发生在多次 `Forward` 之间；只记录最终成功结果，因此失败尝试和路由时间不进入成功样本。
- AIO 采用相同的最终成功 attempt 边界，但供应商聚合保留已有加权 token/duration 口径，避免短请求被过度放大。

主源码：<https://github.com/Wei-Shaw/sub2api/blob/00b8596176809906993169c283671811ad04f58d/backend/internal/repository/ops_repo_openai_token_stats.go#L34-L58>
