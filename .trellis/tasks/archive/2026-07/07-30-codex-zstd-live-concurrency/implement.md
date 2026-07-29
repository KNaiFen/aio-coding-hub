# Implementation Plan

- [x] 将官方 Codex CLI 克隆保留到 `.local/codex-cli-reference/`，记录对齐的客户端源码版本。
- [x] 增加有界 Zstd 编解码并扩展 `GatewayRequestBody`，补充单元测试和端到端代理测试。
- [x] 验证 Zstd 请求恢复模型、请求级思考强度、Session 补全和未修改原始透传。
- [x] 基于 active request snapshot 实现推理路径过滤、Session 去重和可用性传播。
- [x] 将首页并发改接实时快照，保留 TTL Session 查询与独立日志页行为。
- [x] 把请求记录相关文案改为“未缓存输入”“缓存写入”，覆盖零值和缺失值。
- [x] 更新前后端针对性测试与快照，不修改数据库结构。
- [x] 将 package、Cargo、Tauri 和 Cargo.lock 根包版本同步为 `0.60.34`。
- [x] 检查 diff、生成绑定合同、格式和测试；本机只做无构建环境的静态检查。
- [x] 分为修复提交和版本提交推送 `origin/main`，等待精确 SHA 的 GitHub CI 通过。
- [x] 推送 `aio-coding-hub-v0.60.34`，等待 Release 公开并核对目标平台、更新清单和 SHA-256。
- [x] 归档 Trellis 任务，保留 `.local/codex-cli-reference/`，确认无本地构建产物。
