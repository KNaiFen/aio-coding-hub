# 按最终成功上游尝试计算 Token 速度

## Goal

新增最终成功上游尝试计时并替换单请求与聚合速度口径。

## Requirements

- 单请求速率为 `output_tokens * 1000 / final_upstream_attempt_duration_ms`。
- 分母只覆盖最终成功的真实上游尝试，包含 TTFT，排除路由、失败尝试、重试退避、账号切换和 OAuth 刷新。
- 流式结束点取上游 relay EOF；背压、客户端关闭、读取错误或流内错误污染时保持未知。
- 非流式结束点取上游响应体完整读完；只有最终成功请求提交样本。
- 历史记录不近似回填；旧输出段速率不再作为主速度。
- 单请求、实时事件、日志详情、供应商汇总、排行、趋势、日聚合和 TUI 使用同一新口径。

## Acceptance Criteria

- [x] 新增带版本的最终尝试耗时字段，并贯通日志、ledger、usage view、IPC 与生成绑定。
- [x] SQLite schema 48 升至 49；旧行字段为 `NULL/0`，迁移和 ensure 路径均受测试保护。
- [x] 单事件流式和非流式成功请求可计算速度；失败、中断或污染请求显示未知。
- [x] 重试、退避和路由切换不进入分母，上游 EOF 后的下游读取延迟不进入分母。
- [x] 聚合使用 `SUM(output_tokens) * 1000 / SUM(duration_ms)`，不混入旧口径。
- [x] 不设置人为上限或伪造缺失值。

## Notes

- 参考 sub2api 固定提交 `00b8596176809906993169c283671811ad04f58d` 的 per-attempt duration 语义。
