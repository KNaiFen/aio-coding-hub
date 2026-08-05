# 技术设计

## 计时边界

每次真实上游发送继续创建独立 `AttemptTiming`。新增共享的最终尝试计时状态：流式 relay 在无污染地读到上游 EOF 时写入 `attempt_started.elapsed()`；队列满、接收端关闭、读取错误或流内错误会使其无效。非流式在完整读取上游 body 后捕获同一 elapsed 值，并在后续处理仍成功时提交。

## 数据合同

新增 nullable `final_upstream_attempt_duration_ms` 与整数 `final_upstream_attempt_timing_version`。字段流经 request completion、gateway event、request log、usage ledger 和 usage event view。schema 49 只加列并重建相关 view/rollup；旧行保持未知。

现有 `upstream_stream_duration_ms` 保留原 v1 输出窗口语义，但所有面向用户的速度改用新字段。汇总、排行、趋势和日 rollup 使用合格样本的 token/duration 加权比值。

## 兼容与失败

只有终态成功、未排除统计、输出 token 大于零、计时版本有效且 duration 大于零的请求进入速度。失败、中断、历史数据和被背压污染的请求显示未知。没有数值封顶或近似回填。
