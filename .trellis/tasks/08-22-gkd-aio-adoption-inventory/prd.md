# AIO GKD 接入遗留盘点与迁移映射

## Plan Status

- Implementation authorization: confirmed
- Confirmation date and summary: 2026-08-22，用户在当前对话明确授权继续完成后续 GKD、生产迁移和 AIO 接入工作；本任务只交付 AIO 迁移盘点材料。
- Confirmed coverage: 逐文件归属、后续接入顺序、项目适配层边界和可判定验收标准。
- Planning revision: recorded by the planning commit for this task.
- Execution route: main-session Trellis；本任务只新增任务材料，不委派代码施工。
- Migrated from direct-main record: none.

## Material Facts, Assumptions, and Open Questions

| Item | Source | Status / closure condition |
|---|---|---|
| AIO integration base | `origin/main` at `4d6ee488c6e12ecc5735c865d5d786beb5b8e4f0` | confirmed; this task branch is derived from that exact commit |
| GKD distribution input | Released `v0.1.1` asset `gkd-0.1.1-final-ded7a72.tar.gz`, asset SHA-256 `502875847b1c6c1aa0843f9fe0f1d37810db457cd5e3be288183d1a7ff8c531e` | confirmed; later adapter work must pin this released artifact or a separately verified successor |
| GKD output bundle | P1 accepted output digest `68188dcaeb98d93902b435c98784e242090ed18828e9d96a8dee735244f7d1ef` | confirmed; no AIO source may claim a different bundle as installed evidence |
| AIO task mechanism | `.trellis/scripts/task.py`, `common/task_coordination.py`, and `common/task_acceptance.py` still own repository-local coordination and fixed-head acceptance | confirmed; replacement is deferred to milestones B and C |
| Project-specific policies | AIO owns GitHub identity, `ci-gate`, `pr-title`, local Air-safe checks, Rust/Tauri paths and release rules | confirmed; these must not move into GKD generic policy |
| Local verifier interface | Installed generic `gkd-local-verify` names GKD's own `scripts/gkd-verify`; AIO's current `AGENTS.md` names `node scripts/check-local-verification.mjs --base <SHA>` | confirmed gap; B must add adapter dispatch before AIO can claim the generic skill as its verifier |
| Installed runtime compatibility | Accepted bundle `gkd-role` has `#!/usr/bin/env python3` and imports `tomllib`; this machine resolves `python3` to 3.9.6, which raises `ModuleNotFoundError` | blocked outside this task; a minimal GKD runtime compatibility repair must be accepted before B can run project verification or route decisions |
| User-owned planning material | Main checkout has untracked `.trellis/tasks/08-17-gkd-workflow-remediation/` | confirmed; this task neither edits nor stages it |

There are no material open questions for this documentation-only milestone. The GKD runtime compatibility defect is a recorded external blocker, not a reason to alter AIO source or machine state. Later implementation must stop if the pinned bundle, AIO repository identity, required-check policy, or supported GKD runtime differs from these recorded facts.

## Goal

固化 AIO 从仓库内遗留 GKD 工作流迁移到已发布 GKD bundle 的逐文件归属和实施顺序，使后续适配器与状态迁移不会混入 AIO 专有 CI、发布和产品约束。

## Requirements

- 给出所有已发现的 GKD 角色、任务协调、验收、本地验证、CI、发布和历史任务入口的迁移归属。
- 明确每项是由 bundle 替代、保留为 AIO policy/adapter、在获得新路径证据后删除，或只作为历史只读证据保留。
- 把后续工作固定为 bundle pin/adapters、状态迁移、CI/release 接入、canary/删除五个顺序里程碑，且不在本任务实施后续代码变更。
- 保持 GKD canonical source、生产 `~/.codex`、AIO 产品代码、GitHub runner/Secrets 和 tag/Release 不变。

## Acceptance Criteria

- [ ] AC-01：`inventory.md` 覆盖角色/Skills、任务 CLI/schema、验收、本地验证、CI/release、历史任务和项目规则，并逐项给出归属与后续里程碑。
- [ ] AC-02：映射明确保留 AIO 的 GitHub identity、`ci-gate`、`pr-title`、Rust/Tauri、Air-safe local 与发布策略，未将它们归入通用 GKD。
- [ ] AC-03：`design.md` 和 `implement.md` 记录连续实施顺序、停止条件及每个后续里程碑的完成门。
- [ ] AC-04：本任务提交只包含自己的 `.trellis/tasks/08-22-gkd-aio-adoption-inventory/` 材料；未修改运行代码、CI、发布、生产安装或用户未跟踪目录。
- [ ] AC-05：任务材料在固定 base 上通过仓库允许的本地验证合同，并可由固定 PR head 独立验收。

## Scope and Decision Changes

| Date | Old / new decision | Affected acceptance criteria | Decision owner / resume condition |
|---|---|---|---|
| 2026-08-22 | 用户授权从仅记录候选迁移方案变为继续执行 AIO 接入；首项保持只读盘点和规划材料 | AC-01 to AC-05 | user authorization recorded above |

## PENDING Review

- `PENDING.md` 已于 2026-08-22 审阅，没有未解决条目。
