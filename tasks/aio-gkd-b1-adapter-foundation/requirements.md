# AIO GKD Bundle And Review Adapter Foundation

## Goal

将 AIO 作为首个标准 GKD consumer 的已发布 `v0.1.3` bundle pin 与 review adapter 固化为受版本控制的项目事实，并提供一个只在这些 adapter 表面变更时运行的零依赖兼容 smoke。

## User Decisions

- 用户已授权继续冻结的 AIO adoption 计划，并允许在固定 head 验收无阻塞后自动合并。
- 仅使用 GKD `v0.1.3` 已发布 asset，execution bundle digest 为 `cc465d26f08edb2a133775e4d6a58aa517eab1bde0ec2e1ec72f6d9f2c8883bd`。
- AIO 的 GitHub identity、`main`、`ci-gate`、`pr-title` 只由已合并的 `.gkd/policy.json` 定义；通用 GKD 机制不得复制到 AIO。

## Scope

- 新增 canonical `.gkd/bundle-pin.json`，记录版本、release source SHA、execution bundle digest 与发布 asset SHA-256。
- 新增符合 GKD review adapter v1 的 canonical `.gkd/review-adapter.json`，并将其 identity、default branch、policy path 与现有 policy 绑定。
- 新增 AIO 专有、零依赖的 adapter smoke 及其 selftest；仅在 `.gkd/`、smoke 脚本或 local verification runner 变化时由现有 local runner 调用。
- 增加短 adapter 文档，并在 `AGENTS.md` 补充项目 policy 与 machine-local staging 的定位，不复制 GKD 生命周期教程。

## Non-Goals

- 不迁移或删除 `.trellis` 的旧 task lifecycle、历史记录、agent prompt 或测试。
- 不修改 AIO 产品代码、GitHub workflow、required checks、runner、release、tag、Release、Secrets 或 GitHub settings。
- 不在 AIO 实现 GKD task/route/claim/accept/finalization 通用逻辑，也不修改生产 `~/.codex`。
- 不定义尚未被已发布 bundle 支持的 resource、release 或 CI policy schema；这些工作留给后续里程碑和实时 `gkd-optimize-ci` 审查。

## Acceptance Criteria

- [ ] `bundle-pin.json` 为 canonical JSON，精确绑定 GKD `v0.1.3` 的 source SHA、bundle digest 与 asset SHA-256，且不含个人绝对路径。
- [ ] `review-adapter.json` 满足已发布 review adapter v1 结构，adapter digest 正确，并与 `.gkd/policy.json` 的 repository、base branch 和 policy path 一致。
- [ ] adapter smoke 能拒绝非 canonical JSON、未知字段、错误 digest、policy/review identity 漂移与无效 pin 字段；其 selftest 覆盖至少一个正例和上述负例。
- [ ] 现有 local verification runner 仅在 adapter 相关路径改变时执行 smoke；普通产品变更不执行完整 GKD suite、依赖安装、构建、Rust 或前端测试。
- [ ] 文档和 `AGENTS.md` 准确说明 project policy、bundle pin、review adapter 与 project-local runtime staging 的边界。
- [ ] 任务分支的 `git diff --check` 与 `node scripts/check-local-verification.mjs --base <base-sha>` 通过；固定 PR head 的 `ci-gate`、`pr-title` 成功且独立验收无阻塞 finding。
