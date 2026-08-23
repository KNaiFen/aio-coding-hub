# AIO GKD Resource Facts Adapter Plan

## Goal

将 AIO consumer 固定到已发布 GKD `v0.1.4`，并提供能被审查、可复核且不虚构 runtime capacity 或 billing 数据的项目级 resource facts。

## User Decisions

- 用户授权继续 B2，并已授权固定 head 验收无阻塞后的 PR 合并。
- 使用已从 GitHub Release 下载并通过 SHA-256 与 bundle verify 校验的 GKD `v0.1.4` asset，不从 source 或生产安装重建。
- AIO 只维护自己的 repository facts；通用 GKD schema 和 lifecycle 继续由 release bundle 维护。

## Behavior And Defaults

- `.gkd/resource-facts.json` 使用无尾随空白的 canonical JSON，`policy.policyDigest` 由当前 `.gkd/policy.json` canonical bytes 的 SHA-256 得出。
- `runner.verified` 仅表示公开 CI workflow 可以证实 GitHub-hosted Linux runner；`resource` 与 `billing` 的未知值始终保持 `verified: false`。
- adapter smoke 不访问网络、用户目录、runtime receipt 或账单 API，也不安装依赖或生成构建产物。

## Scope

- 实现 requirements 所列 bundle pin、resource facts、adapter smoke/selftest 和项目文档的最小变更。

## Non-Goals

- 不变更 AIO CI DAG、workflow、runner、GitHub settings、release 或消费者的生产安装。
- 不新增 GKD 通用 resource schema、scanner、CLI、role 或 Skills。
- 不迁移或删除 Trellis task 历史，也不把临时扫描结果写入版本控制。

## Acceptance Criteria

- 满足 `requirements.md` 的所有 acceptance criteria。

## Compatibility

- `.gkd/policy.json` 和 `.gkd/review-adapter.json` 的当前结构与 digest 不变。
- 普通产品改动不会触发完整 GKD suite；resource facts 只通过现有 adapter 触发面进入 local verification。

## Security And Data

- 仅保存公开 repository identity、check、版本和 digest；不得写入 token、cookie、账单、个人路径或实际 runtime receipt。
- strict smoke 对 symlink、非 canonical JSON、未知字段或越界 binding 快速失败，不实现默认值或静默回退。

## Migration

- 无数据迁移。新增的 resource facts 只面向本项目后续 B3 资源审查，不改变旧 task lifecycle。

## Public Interfaces

- 新增 `.gkd/resource-facts.json` 作为 AIO 专有 canonical project fact。
- 扩展 `node scripts/check-gkd-adapter.mjs` 的 adapter validation 表面，不新增外部服务或 CLI。

## Execution Route

- automatic。trusted main 在同一已验证 bundle 上完成 project verification、完整 six-gate route decision 和 `TrustedMainRuntimeBridge.prepare`；只允许 bridge 绑定的一个 exact `gkd_executor` 且 `fork_turns="none"`。

## External Side Effects

- 允许任务分支 commit/push、创建或更新一个 AIO PR、修复本任务范围内 CI，并在固定-head 独立验收无阻塞后 squash merge。
- 不允许 tag、Release、GitHub settings、runner、Secrets、production installation 或 AIO 产品发布。

## Action Mode

- `implement_and_merge_on_acceptance`；允许 `ci_repair`、`commit`、`conditional_merge`、`pr_update`、`push`、`ready_for_review`。

## Implementation Notes

- 用 Node 标准库扩展既有 canonical JSON 和 SHA-256 检查，不复制 GKD Python task core。
- 保持 changed-path gating 的现状；只新增 resource facts 对现有 adapter 触发面的覆盖。
- 任务完成时记录本地验证和 fixed-head CI evidence，不写入机器本地 `.gkd/runtime-project.json`。
