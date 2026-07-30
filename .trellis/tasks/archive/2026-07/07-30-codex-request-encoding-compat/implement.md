# Implementation Plan

- [x] 在请求边界实现 Content-Encoding 解析、8 层限制和逐层有界解码。
- [x] 增加 Brotli 直接依赖以及 Deflate、Brotli 的有界解码助手。
- [x] 仅为三个 Codex POST JSON 路径执行规范化，并删除失效实体头。
- [x] 将编码失败接入 early error，新增 `GW_INVALID_REQUEST_CONTENT_ENCODING` 前后端合同。
- [x] 更新受影响的模型、思考强度、Session、隐私过滤和插件路由测试，使其断言明文上游不变量。
- [x] 增加全部编码、别名、重复头、反序堆叠、identity、大小上限、层数上限和损坏数据的单元测试。
- [x] 增加三个 Codex 端点、路径变体、零上游尝试、400/413 日志一致性和非 Codex 原行为的回归测试。
- [x] 新增后端规范并更新规范索引，明确远程上下文压缩与 HTTP 请求压缩的区别。
- [x] 执行错误码合同、TypeScript、Lint、格式化、Rust 定向测试、完整测试和 Clippy。
- [x] 审查最终 diff，确认无正文泄露、无无界解压、无供应商/配置/数据库变更，并保持用户现有未跟踪文件不动。

## Verification Status

- 本地完整 pre-push 流程 15 项全部通过：错误码与支持矩阵合同、TypeScript、Lint、格式化、四个 Vitest 覆盖率分片、全局覆盖率阈值、插件 SDK、脚手架、生成绑定、Rust 单元与集成测试，以及 `cargo clippy --all-targets --locked -- -D warnings`。
- 主分支 CI `30534949485` 在精确提交 `e1fda1fb8033d09b674d87fb37ae5b18c5960e35` 上成功，包含 Rust 测试、Clippy、Cargo Audit、前端检查、构建和平台合同验证。
- Release 任务流 `30537725564` 成功；`aio-coding-hub-v0.60.35` 已正式发布，目标为同一不可变提交，macOS ARM64、Windows x64、签名、`latest.json` 与 `SHA256SUMS.txt` 共 8 个资产均已核验。
- 为使 macOS 本地 pre-push 与 CI 一致，补充了 Rust 路径测试的跨平台可移植性修复；未改变网关运行时行为。
