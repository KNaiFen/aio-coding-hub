# 实施计划

## 任务准备

- [x] 对比 sub2api PR #2481 与 AIO fake 200、SSE prefix、usage tracker 和 failover loop。
- [x] 锁定 Codex Responses-only、500ms 默认保护窗、1 MiB 放行上限、正反关键词、未知透传、结构化脱敏日志和全量 400 迁移。
- [x] 读取 gateway attempt budget、failover route、upstream error response、settings ownership、provider share 与跨层复用合同。
- [x] 配置 Trellis implement/check 上下文并校验任务工件，随后激活任务。

## 后端实现

1. [x] 扩展重试策略/AppSettings 类型、默认值、反序列化、边界校验、普通设置补丁、供应商覆盖及分享/备份往返。
2. [x] 增加幂等 400 容量规则迁移，覆盖全局与供应商覆盖，并测试等价、禁用、满规则列表和重复加载。
3. [x] 建立共享 Codex SSE 终止事件解析、结构化错误提取、关键词分类、消息限长与凭据脱敏 helper。
4. [x] 将 Codex prefix 检查改为保护窗状态机，接入 0..5000ms、真实输出判定、成功提前提交、1 MiB 放行和 idle/read error 既有路径。
5. [x] 增加 `RetryPolicyMatch::StreamInternalError`，复用预算、退避、熔断和供应商切换；记录每次提交前拦截的结构化 attempt 证据。
6. [x] 扩展流结束 tracker / completion，使提交后的 `response.failed` / `response.incomplete` 透传但准确更新最后一次 attempt 和 `GW_FAKE_200` 日志。
7. [x] 扩展 request-end 投影与边界序列化，确保成功重试和最终失败都保存证据，原始 SSE/普通输出/合成凭据不落盘。

## 前端实现

8. [x] 更新设置类型、默认克隆与校验，在“传输错误”下增加“流内部错误”开关及正反关键词编辑；全局页增加保护窗输入，供应商页保持整套覆盖语义。
9. [x] 扩展共享 attempts/error-details 解析器，在供应商链路和最终错误卡展示事件/type/code/message/命中词/处置，增加带 tooltip 的复制按钮。
10. [x] 更新前端 fixture 和测试，覆盖默认值、列表限制、继承/覆盖、未知旧日志、重试成功证据、最终失败详情和复制脱敏消息。

## 验证与交付

11. [x] 按项目规则仅在本地运行定向 Vitest、源码范围 Vitest、TypeScript、ESLint、Prettier、Vite build、Trellis validate 和 `git diff --check`。
12. [x] 审查 diff、残留调试输出、TODO、凭据模式与无关改动；不在本机运行 Cargo、rustfmt、Clippy、Rust tests 或绑定生成。
13. [x] 按逻辑切片提交，使用 `origin` 创建目标为 `main` 的功能 PR；触发 `dev-build` / PR Actions 获取 Rust、格式、Clippy、测试、生成绑定和桌面集成结果。
14. [x] 只应用 CI 报告的有界 Rust 格式/生成绑定漂移，重新验证并更新 PR；不合并、不发布。

## 回滚点

- 设置 schema/default/迁移与前端编辑器作为一个跨层提交同进同退。
- SSE 保护窗与 retry match 作为一个行为提交；若 CI 暴露协议回归，先关闭默认开关或回退该提交，不修改其他协议。
- 日志 evidence 是可选字段，可独立回退展示，但不得留下后端持久化原始 SSE 的临时实现。
- `AIO-PENDING-015` 保持原任务状态，不在本任务中修改或归档。
