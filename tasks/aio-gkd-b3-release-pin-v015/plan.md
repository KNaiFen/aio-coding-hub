# AIO GKD v0.1.5 Bundle Pin Plan

## Goal

Make the repaired GKD `v0.1.5` release the exact, independently verifiable AIO consumer input before any further adoption work.

## User Decisions

- Continue within the authorized GKD/AIO migration boundary after the stable R10 release.
- Use the exact GitHub Release source, bundle digest, and asset SHA-256 as one atomic consumer pin.

## Behavior And Defaults

- The existing Node standard-library adapter keeps an explicit expected pin; no dynamic download, source lookup, or release discovery is added.

## Scope

- Bundle pin facts and their existing validator, selftest, and documentation references.

## Non-Goals

- No change to AIO policy, review adapter, resource facts, CI configuration, product behavior, GKD canonical source, production, tag, Release, or deployment behavior.

## Acceptance Criteria

- All requirements criteria pass with canonical JSON, zero dependency installation, fixed-head CI, and independent acceptance.

## Compatibility

- AIO's policy, review adapter, resource facts, required check names, and ordinary product PR behavior remain unchanged.

## Security And Data

- Persist only public release identifiers and digests; do not add credentials, personal paths, runtime receipts, or external discovery behavior.

## Migration

- This is a one-step consumer pin upgrade. It does not migrate historical Trellis tasks or delete former adapter facts.

## Public Interfaces

- Existing `.gkd/bundle-pin.json` and `scripts/check-gkd-adapter.mjs` interfaces only.

## Execution Route

- Automatic route using the verified published `v0.1.5` runtime and one exact `gkd_executor`.

## External Side Effects

- Candidate branch commit/push, one AIO PR, scope-related CI repair, and fixed-head merge after independent acceptance only.

## Action Mode

- `implement_and_merge_on_acceptance` with `ci_repair`, `commit`, `conditional_merge`, `pr_update`, `push`, and `ready_for_review`.

## Implementation Notes

- Replace only the exact release constants and canonical pin facts; retain existing strict validation and negative coverage without copying GKD logic.
