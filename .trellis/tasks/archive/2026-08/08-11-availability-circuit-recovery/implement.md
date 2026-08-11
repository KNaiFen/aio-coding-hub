# 实施计划：熔断恢复探测接入

## 0. 开工与边界核验

1. 阅读本任务的 `prd.md`、`design.md`、`execution.md`，以及 backend / cross-layer Trellis 规范。
2. 确认 worktree、分支、完整 base SHA 和规划提交均与 `execution.md` 一致；运行 `task.py start` 后才开始代码修改。
3. 只处理本任务允许的 Rust runtime、Gateway 和测试文件；不读取或处理 `upgrade-tui.command`。

## 1. 接入运行中 Gateway 的熔断证据

1. 在 Gateway/app 边界实现 crate-private probe-evidence 入口，安全访问正在运行的 `GatewayRuntime` 所持有的唯一 `Arc<CircuitBreaker>`。
2. 复用现有状态机和 transition emitter：结果完成后先执行 `should_allow`，仅在 HalfOpen 时写成功/失败；保持 Closed 和未到期 Open 不变。
3. 使用可用性 probe 已有的 Provider 元信息、trace ID 和查询到的 CLI key 发送既有 `gateway:circuit` 事件；Gateway 不运行或 Provider 已不存在时安全无操作。
4. 为该入口添加状态机级测试，覆盖过期/未过期 Open、三次成功关闭、HalfOpen 失败重开、Closed 不清失败滑窗和 Gateway 未运行。

## 2. 在共享 probe flight 中写入一次证据

1. 在 `finish_probe` 的 generation 校验成功路径中，在唤醒 waiter 前调用新入口。
2. 保持当前配置 mutation 与结果写入的顺序；过期 flight、内部错误和重复 waiter 均不得写证据。
3. 增加 probe runtime 回归测试，验证并发手动/定时调用共享一个 HTTP probe 时只累计一次熔断结果。

## 3. 实现 30 秒 recovery target

1. 扩展调度运行时内部状态、target 和 `ProbeSource`，支持 recovery source 的独立 due/deadline/trace ID。
2. 在定时 probe 成功返回后检查已写入的熔断 snapshot；仅仍为 HalfOpen 时登记 `completion + 30s` target。
3. 在 scheduler 中统一发射 target，继续使用现有 limiter、generation、expiry 和 in-flight 合并；执行前确认仍是 HalfOpen。
4. 在所有 schedule invalidation 路径清除 recovery target，并确保每 Provider/generation 最多一个 target。
5. 增加确定性调度测试：不早触发、到期一次、成功链持续到 Closed、Closed 后无下一次、配置/删除/休眠取消、合并与并发限制保持有效、主 probe 与 recovery probe 均能保留独立观测。

## 4. 全链路验证与文档

1. 确认没有 IPC、设置、schema、生成绑定或前端改动；必要时补充仅解释“为何只能在 HalfOpen 采纳探测”的短注释。
2. 运行仓库允许的无副作用检查：`git diff --check origin/main...HEAD` 和 `node scripts/check-cloud-only-verification.mjs`；不得运行 Cargo、pnpm、构建、测试或格式化。
3. 尽早创建 Draft PR，推送后等待最新 head 的 GitHub Actions。修复任务相关失败，直至相关 Rust 测试/编译和 `ci-gate` 绿色。
4. 复制并填充 `delivery.md`，写入完整 PR head SHA、base SHA、CI 链接、实际实现、偏移和风险，然后暂停等待 main 验收。
