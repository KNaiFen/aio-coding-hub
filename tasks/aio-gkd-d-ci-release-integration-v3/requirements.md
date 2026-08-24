# AIO GKD CI and Release Adapter

## Goal

Complete adoption milestone D by binding AIO's CI and release workflows to the installed GKD v0.1.5 contracts without copying GKD implementation into the consumer repository.

## User Decisions

- Continue automatically from the accepted C closeout using the freshly installed and verified GKD v0.1.5 bundle.
- Use the live `gkd-optimize-ci` recommendation from explicit public-repository, GitHub-hosted Linux, unknown-capacity, and unverified-billing facts; never invent prices or runner capacity.
- Keep GitHub settings, Secrets, paid runners, ordinary product behavior, production `~/.codex`, and the original AIO checkout outside scope.
- CI and release adapter changes are allowed only within this task's workflow, `.gkd`, scripts, and operations documentation surface.
- Implement the planned task and finalization same-SHA contracts, but do not create an AIO tag or Release unless a later explicit authorization covers that external promotion.

## Scope

- Add one canonical `.gkd/ci-release-adapter.json` declaring the speed-first plan, Air-safe local micro boundary, parallel cloud job groups, required checks, redacted leak scan, bounded artifacts/cache, and same-SHA candidate/finalization rules.
- Add a zero-dependency checker and selftest that validate the declaration, current workflows, required gate fail-closed behavior, redacted output, and release candidate source-SHA binding without dispatching CI or publishing.
- Wire the checker into the existing adapter/local verification contract and the `ci.yml` contracts job; preserve the existing cloud-owned dependency, frontend, Rust, audit, generator, and packaging jobs.
- Make only the minimum workflow/documentation edits needed to express independent speed-first job execution, bounded artifact retention, and two-PR same-SHA release promotion guards.
- Deliver one fixed-head AIO PR through the installed GKD automatic route; independent acceptance and merge occur only after local verification and policy-required CI succeed.

## Non-Goals

- Do not modify GitHub settings, branch protection, Secrets, environments, runner classes, billing, or paid infrastructure.
- Do not create tags, Releases, deployments, release assets, or large local build/cache artifacts.
- Do not install dependencies or run frontend, Rust, Tauri, signing, packaging, or product tests locally; those remain cloud-owned.
- Do not modify `.trellis/tasks/**`, the protected original untracked adoption-plan directory, or historical Git commits.
- Do not add a second CI monitor, release engine, task lifecycle, review engine, or generic GKD implementation to AIO.

## Acceptance Criteria

- [ ] `.gkd/ci-release-adapter.json` is canonical JSON with exact fields and no unknown keys; its recommendation digest matches the live speed-first recommendation and records unknown billing/capacity as unknown.
- [ ] The checker proves the existing `ci-gate` and `pr-title` required checks, independent cloud job groups, Air-safe local micro commands, bounded artifact/cache declarations, and fail-closed gate conditions.
- [ ] The redacted leak check rejects credential-shaped or machine-local values while its terminal output contains only stable codes and relative paths.
- [ ] The release contract proves candidate artifacts are unique, checksummed, source-SHA bound, and consumed by a two-PR finalization path; it performs no GitHub write.
- [ ] Repeated checker/selftest runs are byte-stable and write nothing; Trellis history remains unchanged.
- [ ] `scripts/gkd-verify --base-sha a133a79c819ff875cfffca40967700679b4fc383`, `git diff --check`, and fixed-head `ci-gate`/`pr-title` pass before merge.
- [ ] If speed-first decouples `frontend` or `rust` from `contracts`, update the existing `scripts/check-ci-quality-gates.selftest.mjs` fixtures and assertions in the same change; the contracts selftest must pass against the final workflow.
