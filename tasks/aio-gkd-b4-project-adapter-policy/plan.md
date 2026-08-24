# AIO GKD Project Adapter Policy Plan

## Goal

Finish the policy portion of milestone B with a small AIO-owned declaration that binds existing repository behavior and remains separate from GKD's generic CI policy.

## User Decisions

- Continue the approved AIO adoption sequence after the verified GKD `v0.1.5` pin and B3 closeout.
- Keep generic GKD schemas unchanged and persist only AIO-specific public repository facts.
- Use one automatic `gkd_executor`; trusted main owns independent acceptance, exact-head merge, records, and cleanup.

## Behavior And Defaults

- Keep `.gkd/policy.json` byte-compatible with the strict GKD v1 schema used by project staging and the fixed-head monitor.
- Add `.gkd/adapter-policy.json` as the only new project fact. It is read solely by the existing AIO adapter validator and contains no executable behavior beyond the declared local entrypoint identity.
- Validate exact fields and constants in the existing zero-dependency Node adapter; do not parse YAML, invoke GitHub, install dependencies, or discover releases at runtime.

## Scope

- `verification`: versioned local entrypoint, full lowercase SHA-1 base binding, zero-artifact mode, adapter-smoke trigger, and the existing cloud-owned categories.
- `ci`: current GitHub-hosted job classes, pnpm/Rust cache classes, and existing drift/platform/TUI/final candidate artifact patterns with their bounded retention days.
- `release`: current semantic tag template, main-ancestor requirement, successful main-CI and same-SHA candidate requirements, checksum manifest, and equivalent-assets-only behavior for an existing release.
- Existing validator/selftest, adapter documentation, and root `AGENTS.md` fact inventory required to bind and explain this file.

## Non-Goals

- No GKD source, generic policy, workflow, runner, required-check, settings, Secrets, product, Trellis history, release, deployment, production, or historical migration changes.
- No dynamic workflow parsing, GitHub/API lookup, release discovery, or generic lifecycle implementation in AIO.

## Acceptance Criteria

- All requirements criteria pass with canonical JSON, strict negative coverage, unchanged generic project staging, zero dependency installation, fixed-head CI, and independent acceptance.

## Compatibility

- Generic GKD project staging, routing, CI monitoring, review adapter, resource facts, bundle pin, required checks, and ordinary product PR behavior remain unchanged.

## Security And Data

- Store only repository-public declarative facts. Reject absolute paths, runtime state, credentials, URLs with tokens, unknown fields, and noncanonical JSON.

## Migration

- This is an additive project-policy binding. It does not migrate task state, rewrite history, or delete any existing adapter fact.

## Public Interfaces

- New AIO-only `.gkd/adapter-policy.json`; existing `scripts/check-gkd-adapter.mjs` validation output and `scripts/gkd-verify` entrypoint remain compatible.

## Execution Route

- Automatic route using the verified published GKD `v0.1.5` runtime and one exact `gkd_executor`.

## External Side Effects

- Candidate branch commit/push, one AIO PR, scope-related CI repair, and fixed-head merge after independent acceptance only.

## Action Mode

- `implement_and_merge_on_acceptance` with `ci_repair`, `commit`, `conditional_merge`, `pr_update`, `push`, and `ready_for_review`.

## Implementation Notes

- Use only `scripts/gkd-verify --base-sha b35e34245a1667e647965be58ba44654ca0ba053` locally. Dependency, frontend, Rust, test, build, generator, signing, packaging, and release execution remain cloud-owned or out of scope.
