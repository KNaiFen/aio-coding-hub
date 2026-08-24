# AIO GKD CI and Release Adapter Delivery

## Result

- Status: awaiting fixed-head CI and independent acceptance.
- PR: #183.
- Added the canonical AIO CI/release adapter declaration and read-only checker for the installed GKD v0.1.5 contracts.
- Decoupled frontend and Rust cloud jobs from the contracts job while retaining the fail-closed aggregate gate.

## Implementation

- .gkd/ci-release-adapter.json binds the live speed-first recommendation, unknown capacity and billing facts, Air-safe local commands, independent cloud groups, bounded artifact/cache retention, required checks, redacted leak scanning, and same-source-SHA candidate/finalization rules.
- scripts/check-gkd-ci-release.mjs and its selftest reject unknown fields, recommendation drift, workflow guard drift, credential-shaped or machine-local values, unbounded artifacts, and release source-SHA or checksum guard drift.
- Existing adapter and local verification entrypoints run the new checker only for relevant adapter changes; no GKD lifecycle or release engine was copied into AIO.
- scripts/check-ci-quality-gates.selftest.mjs and the quality-gate contract now assert the intended independent frontend/Rust conditions.

## Verification

| Type | Check | Result | Notes |
|---|---|---|---|
| Local | scripts/gkd-verify --base-sha a133a79c819ff875cfffca40967700679b4fc383 | Pass | Returned local_ready at implementation head 9617730df1e7ceaa0001f5b3ffb55e67a68f1654; adapter, CI/release, history, cloud-only, syntax, whitespace, and diff checks passed. |
| Local | git diff --check | Pass | No whitespace errors. |
| GitHub | ci-gate and pr-title | Pending | Must complete at the fixed delivery head. |
| Independent review | GKD acceptance | Pending | Not performed by the executor. |

## Scope And Risk

- Only workflow, .gkd, scripts, and operations documentation surfaces changed.
- No product behavior, dependencies, Trellis history, GitHub settings, Secrets, paid runners, tags, Releases, deployments, production installation, or AIO ordinary product files changed.
- Dependency installation, frontend/Rust checks, builds, packaging, signing, and release publication remain cloud-owned or separately authorized.
- The release adapter performs no GitHub write and does not publish a tag or Release.

## Candidate Output Bundle

- Implementation head: 9617730df1e7ceaa0001f5b3ffb55e67a68f1654.
- Candidate output bundle SHA-256: 0f83430951e1b04b852fb8b53474d726b644ca56dc479eba3458f2bec7606d81.
