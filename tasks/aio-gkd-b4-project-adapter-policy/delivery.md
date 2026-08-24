# AIO GKD Project Adapter Policy Delivery

## Result

- Added canonical AIO-only `.gkd/adapter-policy.json` facts for the repository's existing local verification, GitHub Actions resource/artifact, and release promotion contracts.
- Extended the zero-dependency adapter validator and selftest to reject unknown fields and drift at each material policy boundary.
- Kept the generic GKD policy and published `v0.1.5` bundle pin unchanged.

## Implementation

- Verification policy binds `scripts/gkd-verify --base-sha <full-lowercase-sha>`, zero-artifact local work, adapter-smoke paths, and the current cloud-owned categories.
- CI policy binds the current GitHub-hosted runner labels, pnpm/Rust cache classes, and drift, development, platform, TUI, and final candidate artifact names with their bounded retention days.
- Release policy binds `aio-coding-hub-v{semver}`, main ancestry, successful same-SHA main CI, the unique unexpired candidate, `SHA256SUMS.txt`, and equivalent-assets-only handling for an existing Release.
- `AGENTS.md` and `docs/operations/gkd-adapter.md` record the AIO-only policy boundary without extending the generic GKD schema or adding workflow/API discovery.

## Verification

- Implementation head: `9025bc381ff9dcec1cb98142cb77666f34969403`.
- Manual-recovery verification head: `f61fca35aa125836559ce23a5421b325bb4e86ca`.
- `scripts/gkd-verify --base-sha b35e34245a1667e647965be58ba44654ca0ba053` passed again at the manual-recovery verification head, covering the local runner selftest, cloud-only contract checks, adapter selftest/smoke, committed/index/worktree diff checks, untracked whitespace check, and changed Node syntax checks.
- Required GitHub checks `ci-gate` and `pr-title` remain pending for the final fixed delivery head.
- Dependency installation, formatting, linting, type checking, tests, coverage, builds, generators, Rust/Tauri checks, and signing or packaging remain cloud-owned.

## Manual Recovery

- The prior fixed-head acceptance attempt at `6a066eb74518052be3d9543958fca9f7a638bf5a` was rejected solely because the CI monitor received an unsupported absolute policy path and returned `POLICY_PATH_UNSUPPORTED`; no candidate-code finding was recorded.
- Review of the requirements, rejected attempt, current diff, and existing implementation found no new defect, so the implementation remains unchanged.

## Scope And Risk

- `.gkd/policy.json`, `.gkd/review-adapter.json`, `.gkd/resource-facts.json`, `.gkd/bundle-pin.json`, workflows, runner configuration, GitHub settings, product code, Trellis history, tags, Releases, deployments, and production installations were not changed.
- The new policy contains only repository-public declarative facts; no credentials, runtime receipts, machine-local paths, or dynamic discovery were added.
- This candidate stops before acceptance, merge, cleanup, release, or deployment side effects.

## Candidate Output Bundle

- Deterministic Git source archive of manual-recovery implementation head `f61fca35aa125836559ce23a5421b325bb4e86ca` SHA-256: `c545c0bb3e6bae304dfd8b33adea8cbc09dda9647fb4a8f955ad103bee5c9014`.
