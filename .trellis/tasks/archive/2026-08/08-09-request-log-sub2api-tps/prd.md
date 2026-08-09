# 请求日志与聚合 TPS 对齐

## Goal

对可信的最终成功上游 attempt 计算单条 TPS，并将所有聚合表面改为 sub2api 的逐请求算术平均。

## Requirements

- 固定参照 `Wei-Shaw/sub2api@00b8596176809906993169c283671811ad04f58d`。
- 单条分母包含 TTFB 且只覆盖最终成功 attempt；不减 TTFB，不加启发式回退。
- 客户端断开后若后端 drain 取得可信完成，最终 attempt 计时仍有效；未完成或上游错误仍无效。
- Summary、trend、provider/CLI/model/session/folder leaderboard 和 day detail 全部使用 TPS sum/count。
- 旧 timing v0 不回填；实施与测试代码完成后等待父任务统一验证。

## Acceptance Criteria

- [ ] 断开后可信完成的新请求保存 timing v1 并显示速度。
- [ ] `375` 与 `500` TPS 样本聚合为 `437.5`，不是旧加权值 `400`。
- [ ] raw、rollup、文件夹筛选和 preview 口径一致，对外字段名不变。
