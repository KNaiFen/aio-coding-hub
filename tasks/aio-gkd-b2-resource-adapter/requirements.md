# AIO GKD Resource Facts Adapter

## Goal

将 AIO 的已发布 GKD bundle pin 升级到 `v0.1.4`，并新增一个只陈述公开、可验证项目事实的 resource adapter，供后续资源审查使用。

## User Decisions

- 用户已授权继续冻结的 AIO adoption 计划，并允许固定 head 独立验收无阻塞后合并。
- 只使用已发布 GKD `v0.1.4` Release asset；其 release source 为 `be1e515a64c4095676922c484555fb2a048da681`，execution bundle digest 为 `cdaa791ace82a5e7c407b29a93a4211b852d7f364900bbcd8a549dbe918bf2a7`，asset SHA-256 为 `713fc828d234bc7ddd298cb68f5abfe1ede29f7891c283924cf3c3b98b2c0330`。
- resource adapter 只能保存 AIO 可从版本控制 policy 或公开 workflow 确认的事实；未知资源容量和账单数据必须明确标为未验证。

## Scope

- 更新 `.gkd/bundle-pin.json` 到上述已发布 `v0.1.4` 事实。
- 新增 canonical `.gkd/resource-facts.json`，绑定已有 `.gkd/policy.json` 的 digest、base branch 和 required checks，并声明 GitHub-hosted Linux runner 的已验证来源。
- 扩展既有零依赖 `scripts/check-gkd-adapter.mjs` 及 selftest，严格校验 resource facts 的 canonical JSON、字段、policy binding 和未知资源数据的边界。
- 在 `AGENTS.md` 与 `docs/operations/gkd-adapter.md` 说明 AIO project facts 的职责边界。

## Non-Goals

- 不修改 `.gkd/policy.json`、`.gkd/review-adapter.json`、GitHub workflow、runner 配置、required checks、Release、tag、Secrets 或 GitHub settings。
- 不在 AIO 复制或修改 GKD 的通用 schema、task/route/claim/accept 生命周期、Skills 或 runtime staging。
- 不扫描、推断或持久化 runner CPU、内存、磁盘、价格或账单；不修改产品代码、`.trellis` 历史或生产 `~/.codex`。

## Acceptance Criteria

- [ ] `bundle-pin.json` 为 canonical JSON，精确绑定已发布 GKD `v0.1.4` 的 source SHA、execution bundle digest 与 asset SHA-256。
- [ ] `resource-facts.json` 为 canonical JSON，只含 schema v1 的公开项目事实，严格绑定当前 policy digest、base branch 和 required checks，且未知容量/账单字段保持未验证。
- [ ] adapter smoke 拒绝非 canonical JSON、未知字段、host source 冒充 runner、policy digest/check 漂移以及将未知资源数据标记为已验证的输入。
- [ ] selftest 覆盖正例和上述负例；现有 local verification runner 仍只在 adapter 相关路径变化时运行 smoke。
- [ ] 文档准确说明 resource facts 是 AIO 专有 adapter，不是 GKD 通用协议，也不能充当实时资源扫描或账单事实。
- [ ] 任务分支的 `git diff --check` 与 `node scripts/gkd-verify --base <base-sha>` 等仓库批准的 local verification 通过，固定 PR head 的 `ci-gate` 与 `pr-title` 成功。
