# AIO GKD Resource Facts Adapter Delivery

## Result

- Updated the AIO GKD bundle pin to the published v0.1.4 source, execution bundle, and asset digests.
- Added the AIO-only resource facts v1 document. It binds the current policy digest, base branch, and required checks; it records only the public GitHub Actions source for a GitHub-hosted Linux runner.
- Kept resource capacity and billing cost as `unknown` with `verified: false`.
- Extended the zero-dependency adapter validator and selftest to reject non-canonical facts, unknown fields, runner-source impersonation, policy binding drift, and verified unknown resource or billing values.

## Acceptance Evidence

- The bundle pin and resource facts are canonical JSON and the resource facts bind the current policy digest, base branch, and required checks.
- `scripts/gkd-verify` passed its local runner selftest, cloud-only contract checks, committed/index/worktree diff checks, untracked whitespace check, adapter selftest and smoke, and changed Node syntax checks.
- The required GitHub checks `ci-gate` and `pr-title` succeeded for the verified implementation candidate.
- Dependency installation, formatting, linting, type checking, tests, coverage, builds, generators, Rust/Tauri checks, and signing or packaging remain cloud-owned checks.

## Scope And Risk

- `.gkd/policy.json`, `.gkd/review-adapter.json`, workflows, runner configuration, GitHub settings, product code, Trellis history, and production installations were not changed.
- Resource facts are an AIO project adapter, not a GKD-wide schema, a runtime resource scan, or billing evidence. No CPU, memory, disk, price, or billing value is asserted.

## Candidate Output Bundle

- Candidate output bundle SHA-256: `77e6abe7ece07fe8986d54992b9a057bbbf25636d0f8fdd711c4855293c5d3d6`.
