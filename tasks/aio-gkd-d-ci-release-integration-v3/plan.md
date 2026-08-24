# AIO GKD CI and Release Adapter Plan

## Goal

Finish milestone D with a narrow, read-only-compatible adapter that makes AIO's existing cloud CI and release workflow contracts explicit and verifiable through the installed GKD bundle.

## User Decisions

- Continue the approved adoption sequence automatically from the C closeout merge.
- Select speed-first using only verified public repository and GitHub-hosted runner facts; unknown capacity, disk, memory, CPU, and price remain unknown or conservative.
- Keep trusted main responsible for acceptance, merge, records, and cleanup; the executor stops after fixed-head delivery.

## Behavior And Defaults

- Local verification is an Air-safe micro contract: Node-only adapter/selftests, syntax and diff checks, no dependency install or large artifact.
- Actions remain speed-first and parallel: contracts, frontend, Rust, and release candidate matrix are independent where their existing dependencies allow; `ci-gate` aggregates every expected result and fails closed on unexpected skips.
- Artifact and cache declarations are bounded by the existing AIO policy; retention is explicit and no cleanup is treated as a repair for an unknown peak-disk bound.
- Leak scanning is redacted and deterministic. It reports a stable finding code and repository-relative path, never matched values or machine paths.
- Release candidate and finalization records bind one source SHA, one candidate artifact set, and one finalization PR; this task never publishes them.

## Scope

- New `.gkd/ci-release-adapter.json` and `scripts/check-gkd-ci-release.mjs` plus selftest.
- Minimal changes to `.gkd/adapter-policy.json`, `scripts/check-gkd-adapter.mjs`, `scripts/check-local-verification.mjs`, and relevant workflow/documentation files required by the declaration.
- Existing AIO workflow jobs and cloud-only boundary remain the source of truth for dependency, test, build, audit, and packaging execution.

## Non-Goals

- No Trellis migration, task-history rewrite, product change, dependency update, GitHub setting, Secret, paid runner, tag, Release, deployment, or local build.
- No hand-written GKD task state, offer, claim, activation, delivery, review, acceptance, or receipt.

## Acceptance Criteria

- All requirements criteria pass, including strict declaration validation, recommendation/fact binding, parallel/fail-closed workflow checks, redacted leak output, same-SHA release checks, local verification, and required fixed-head CI.

## Compatibility

- Existing `.gkd/policy.json`, bundle pin, adapter facts, history adapter, Trellis task files, product workflows, and required checks remain compatible.
- Existing release workflow remains write-capable only when independently authorized; this task verifies its guard surface without invoking it.

## Security And Data

- Store only repository-relative paths, enum-like workflow facts, digests, and stable policy values. Reject unknown fields, symlinks, credential-shaped strings, machine-local absolute paths, and source-SHA drift.
- Do not print secrets, prompts, transcripts, runner credentials, artifact contents, or absolute machine paths.

## Migration

- No historical task or release migration is performed. The adapter documents current workflow facts and makes future same-SHA finalization explicit.

## Public Interfaces

- New AIO-only `.gkd/ci-release-adapter.json` and `scripts/check-gkd-ci-release.mjs` machine result.
- Existing `scripts/gkd-verify --base-sha <full-sha>` remains the sole local verification entrypoint.

## Execution Route

- Automatic route through the verified published GKD v0.1.5 bundle, one exact direct `gkd_executor`, and canonical offer/claim bridge.

## External Side Effects

- Candidate branch commits, one AIO implementation PR, scope-related CI repair, independent fixed-head merge, one records-only closeout PR, and isolated cleanup. No tag/Release promotion in this task.

## Action Mode

- `implement_and_merge_on_acceptance` with `ci_repair`, `commit`, `conditional_merge`, `pr_update`, `push`, and `ready_for_review`.

## Implementation Notes

- Reuse the existing canonical JSON, adapter policy, resource recommendation, and release-promotion helpers; keep the new checker zero-dependency and repository-relative.
- Parse workflow text structurally enough to prove required job/check names and explicit `needs`/`if: always()` aggregation; do not implement a second YAML engine.
- Use controlled fixture files for leak and same-SHA cases. Keep all output canonical and path-minimized.
- Run only `scripts/gkd-verify --base-sha a133a79c819ff875cfffca40967700679b4fc383`; dependency, frontend, Rust, build, packaging, release, and deployment work remain cloud-owned.
- Treat the existing quality-gate selftest as part of the workflow contract: any intentional dependency change must update its mutation fixture and expected assertion before delivery.
