# 施工入口：熔断恢复探测接入

> 按照本文件和它列出的任务材料施工。完成实现、PR、CI 和 `delivery.md` 后暂停，等待 main 验收。

## 快速定位

- 任务目录：`.trellis/tasks/08-11-availability-circuit-recovery/`
- Worktree：`/Users/knaifen/Documents/Codex/aio-coding-hub/08-11-availability-circuit-recovery`
- 分支：`fix/availability-circuit-recovery`
- 基线：`origin/main` @ `9b05b28d5841584dc6f2a867947afd5d23f76246`
- 规划提交：pending coordinator freeze; do not start until this field is replaced with the recorded SHA.
- 实施授权：已确认，2026-08-11；覆盖 `prd.md` 中 R-01 至 R-08 与 AC-01 至 AC-07。
- PR 目标：`main`；PR：尚未创建。
- 当前唯一写者：user execution session。
- 当前阶段：待开工。
- PENDING 审阅：`AIO-PENDING-029` 延后且禁止触碰；不得读取、执行、修改、移动或删除 `upgrade-tui.command`。

## 开工顺序

1. 阅读当前 worktree 生效的 `AGENTS.md`、本文件、`prd.md`、`design.md`、`implement.md`。
2. 阅读 `.trellis/spec/aio-coding-hub/backend/index.md`、`.trellis/spec/aio-coding-hub/cross-layer/index.md` 和 `.trellis/spec/guides/index.md`。
3. 在规划 SHA 已冻结后运行：
   ```bash
   python3 ./.trellis/scripts/task.py start .trellis/tasks/08-11-availability-circuit-recovery
   ```
4. 核验：当前路径、分支、`git merge-base 9b05b28d5841584dc6f2a867947afd5d23f76246 HEAD`、规划 SHA、无材料性未决问题和唯一写者均正确。

任何一项失败时停止并报告 main；不要从聊天记录补全缺失决定。

## 锁定行为

- Open 冷却时间不因探测成功提前缩短；仅结果完成时已到期的 Open 才可进入 HalfOpen。
- 有效手动和定时 probe 都只作为 HalfOpen 证据写一次；三次连续成功沿用现有规则关闭，HalfOpen 失败立即重新 Open。
- 只有定时 probe 的成功写入后仍是 HalfOpen，才在成功完成后 30 秒追加一次 recovery probe；第三次成功已 Closed 时不得追加。
- 补测必须属于现有 scheduler/generation/limiter/in-flight 体系，禁止裸 `spawn + sleep`。
- Gateway 未运行时不修改离线持久化熔断状态；无 IPC、配置、schema 或前端合同变更。

## 允许范围

- `src-tauri/src/app/provider_availability_probe_runtime.rs`：单次证据接入、recovery target、调度失效和 runtime 测试。
- `src-tauri/src/app/gateway_state.rs`、`src-tauri/src/gateway/runtime.rs`、`src-tauri/src/gateway/proxy/provider_router.rs`：运行中 Gateway 的内部访问、状态记录与既有事件复用。
- 相关 Rust 单元/集成测试与任务交付材料。

不得借机重构一般熔断策略、修改默认阈值/时长、改变定时探测设置语义，或处理任何其他 PENDING 项。

## 实施导航

- 权威技术设计：`design.md`；按其中的“熔断证据写入”和“定时恢复补测”顺序实现。
- 权威步骤与测试矩阵：`implement.md`。
- 每个实际 HTTP flight 只在 `finish_probe` 记一次；scheduled caller 可在结果已写后排 follow-up，但不能重复累计成功。
- recovery target 执行前必须再次确认 active circuit 为 HalfOpen，以消除等待期间其他成功导致 Closed 后的无用请求。

## 验证与交付

- 本地仅允许：`git diff --check origin/main...HEAD`、`node scripts/check-cloud-only-verification.mjs` 及其 self-test；不得运行 Cargo、pnpm、依赖安装、测试、构建、格式化或生成。
- 提前创建 Draft PR；最新 PR head 必须通过相关 GitHub Actions Rust 测试/编译与 `ci-gate`。按路径规则跳过的 job 在 `delivery.md` 说明。
- 完成时按 `docs/operations/templates/delivery.md` 在本任务目录创建并填写 `delivery.md`，绑定完整 base/head SHA、CI URL、AC 结果与风险，然后暂停。不得合并 PR、开启自动合并、归档任务或删除 worktree。

## 停止并询问 main

- 需要让 Gateway 未运行时也写持久化熔断状态。
- 必须修改公开 IPC、设置、schema、前端合同或上述允许范围之外的重要模块。
- 发现“Open 到期后探测才计入”“HalfOpen 失败立即 Open”“30 秒只由定时成功继续”任一锁定决定无法满足。
- CI 失败疑似基础设施或 main 已有问题，且没有可靠的任务内修复。
