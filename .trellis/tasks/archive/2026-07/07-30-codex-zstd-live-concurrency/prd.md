# 修复 Codex zstd 审计与实时并发

## Goal

恢复 Codex Desktop 启用远程压缩后的请求审计信息，并把首页“当前并发”改为此刻正在执行的会话数。

## Requirements

- 支持检查 `Content-Encoding: zstd` 的 Codex 请求体，恢复请求阶段的模型和思考强度识别，同时保持远程压缩可用。
- 解压和重压缩必须有请求体上限保护；未修改的请求继续使用原始压缩字节，解压失败时不中断现有透传。
- 首页并发只统计正在执行的推理或压缩请求，按 `cli_key + session_id` 去重；没有 Session ID 时按 trace 回退。
- 并发随现有请求开始/结束事件刷新，失败或初始加载显示 `--`，网关正常且无请求时显示 `0`。
- 保留五分钟 TTL 的活跃 Session 明细和计数语义，不改变会话复用、路由与切换警告。
- 请求记录把现有 `effective_input_tokens` 明确标为“未缓存输入”，把缓存创建字段标为“缓存写入”；显式零与字段缺失必须区分。
- 不推算或覆盖上游 `cache_write_tokens`，不迁移数据库，不回填历史未知记录。
- 版本升级为 `0.60.34`，只在 GitHub Actions 中执行完整测试与应用构建。
- 官方 Codex CLI 源码长期保留在被 Git 忽略的 `.local/codex-cli-reference/`，不得进入 AIO 提交。

## Acceptance Criteria

- [ ] `zstd` Codex `/v1/responses` 请求能记录请求模型、`codex_reasoning_effort` 及其请求来源。
- [ ] 未变更的 `zstd` 请求保持原始字节和编码头；变更后的请求能重新压缩并被上游正确解码。
- [ ] 非法、超限及复合编码继续安全降级，不产生解压炸弹或破坏原请求。
- [ ] 同一 Session 的多个同时请求只计为 1，不同主会话或子代理会话分别计数，结束后及时归零。
- [ ] 搜索、模型发现等非推理请求不计入并发，独立请求日志页不显示首页组件。
- [ ] 请求列表、实时卡片和详情页同时区分“未缓存输入”“缓存写入”“缓存读取”；缺失显示 `—`，显式 0 显示 `0`。
- [ ] GitHub CI 全部通过，`aio-coding-hub-v0.60.34` Release 公开并包含 macOS ARM64 APP ZIP、Windows x64 便携 ZIP、更新包和 `latest.json`。
- [ ] Trellis 任务完成后归档；`.trellis/workspace/KNaiFen/` 保持未跟踪且不提交。

## Notes

- 旧日志没有保存可用于重建请求思考强度的解压 JSON，不做不可靠回填。
- 本机不恢复 Rust 或 Node 构建环境，不创建 `target/`、`node_modules/`、`dist/`。
