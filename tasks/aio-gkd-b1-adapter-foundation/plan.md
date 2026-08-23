# AIO GKD Bundle And Review Adapter Foundation Plan

## Goal

将首次 consumer 的发布 bundle pin 和 review adapter 变成可审查、可复核的 AIO 项目事实，同时保持普通 AIO PR 不承担 GKD 完整测试成本。

## User Decisions

- 用户已批准继续 AIO adoption，并授权任务在固定 head 验收后合并。
- 使用已验收发布的 GKD `v0.1.3`，不从 canonical source 或生产目录临时重建。
- AIO 项目值保留在 adapter；GKD 通用生命周期、角色和固定-head机制继续由 bundle 提供。

## Behavior And Defaults

- `bundle-pin.json` 与 `review-adapter.json` 使用无尾随空白的 canonical JSON；review adapter digest 排除自身字段后按 GKD canonical JSON 算法计算。
- adapter smoke 默认不运行。只要变化集包含 `.gkd/`、`scripts/check-gkd-adapter.mjs`、其 selftest 或 local verification runner，才运行它。
- smoke 只读取仓库内的 adapter/policy 文件，不调用网络、不读取用户目录、不安装依赖、不创建构建产物。

## Scope

- 实现 requirements 中列出的 bundle pin、review adapter、adapter smoke/selftest、最小 local-runner 触发和项目文档。

## Non-Goals

- 不在本任务中变更 legacy task 工具、工作流、CI DAG、发布流程、项目资源策略或历史 archive。
- 不新增 GKD 通用 schema、CLI、角色或安装行为。

## Acceptance Criteria

- 满足 `requirements.md` 的全部 acceptance criteria。

## Compatibility

- `policy.json` 的现有 schema/version/digest 保持不变。
- 新的 bundle pin 与 review adapter 是新增的 AIO project adapter，不改变公共产品 API 或现有 task records。

## Security And Data

- adapter 仅保存公开仓库 identity、检查名、版本和 SHA digest；不得写入 token、cookie、secret、用户目录、机器路径或运行时 receipt。
- smoke 对不规则文件、symlink、非 canonical JSON 或越界路径快速失败，不回显敏感内容。

## Migration

- 无数据迁移。旧 Trellis lifecycle 保持原样，直到后续真实 canary 的新路径证据完整。

## Public Interfaces

- 新增 `node scripts/check-gkd-adapter.mjs` 作为 AIO adapter 的零依赖 smoke 入口。
- 新增 `.gkd/bundle-pin.json` 与 `.gkd/review-adapter.json`；两者的结构与更新边界由 AIO 文档说明。

## Execution Route

- 自动路线。trusted main 必须使用同一已验证 bundle 的 project verification、完整 six-gate route decision 和 `TrustedMainRuntimeBridge.prepare`；只允许 bridge 返回的一个 direct `gkd_executor`，`fork_turns="none"`。

## External Side Effects

- 允许任务分支 commit/push、创建或更新一个 AIO PR、修复本任务范围内的 CI 与固定-head验收后的 squash merge。
- 不允许 tag、Release、GitHub settings、runner、Secrets、production installation 或 AIO 产品发布。

## Action Mode

- `implement_and_merge_on_acceptance`；允许 action 按字典序为 `ci_repair`、`commit`、`conditional_merge`、`pr_update`、`push`、`ready_for_review`。

## Implementation Notes

- 用 Node 标准库实现 canonical JSON、SHA-256、严格字段检查和跨文件 binding，不引入第三方包或复制 GKD task state。
- local runner 使用已有的 changed-path 集合判断是否调用 smoke，并在结构化结果中仅回显是否执行及非敏感 digest。
- 文档只引用 bundle Skill/项目 policy 的职责边界；不将 `.gkd/runtime-project.json` 进入 Git。
