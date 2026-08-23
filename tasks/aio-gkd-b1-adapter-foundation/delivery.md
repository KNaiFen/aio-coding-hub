# AIO GKD Bundle And Review Adapter Foundation Delivery

## Result

- Status: awaiting fixed-head CI and independent acceptance.
- Added the versioned GKD bundle pin and review adapter facts for AIO, with a conditional zero-dependency adapter verification path.

## Implementation

- `.gkd/bundle-pin.json` fixes the approved GKD `v0.1.3` release source, asset checksum, and execution bundle digest.
- `.gkd/review-adapter.json` is a canonical review adapter v1 record bound to `.gkd/policy.json`.
- `scripts/check-gkd-adapter.mjs` rejects malformed adapter facts, identity drift, and invalid bindings; its selftest covers one valid fixture and the required negative cases.
- `scripts/check-local-verification.mjs` runs the adapter selftest and smoke only when the adapter or local-verification surface changes. `scripts/gkd-verify` is the versioned entry point for that local contract.
- `AGENTS.md` and `docs/operations/gkd-adapter.md` distinguish versioned project facts from project-local runtime staging.

## Acceptance Criteria

| Criterion | Result | Evidence |
|---|---|---|
| Canonical bundle pin binds the approved release facts | Pass | Adapter smoke validates the exact pin fields and checksums. |
| Review adapter v1 digest and policy binding are correct | Pass | Adapter smoke and the published review adapter validator agree on the adapter digest. |
| Smoke rejects malformed and drifting adapter facts | Pass | `scripts/check-gkd-adapter.selftest.mjs` covers canonicality, unknown fields, digest, policy identity, review identity, and pin failures. |
| Local runner only invokes adapter verification for relevant paths | Pass | Runner selftest covers relevant and ordinary-product path decisions; the local contract records whether the smoke ran. |
| Documentation states project and staging boundaries | Pass | `AGENTS.md` and `docs/operations/gkd-adapter.md`. |
| Required local contract passes | Pass | Versioned local verification completed with adapter selftest, smoke, diff checks, and changed Node syntax checks. |

## Verification

| Type | Check | Result | Notes |
|---|---|---|---|
| Local | `scripts/gkd-verify --base-sha <registered-base-sha>` | Pass | Includes runner selftest, cloud-only contract checks, adapter selftest/smoke, diff checks, and changed Node syntax checks. |
| GitHub | `ci-gate` and `pr-title` | Pending | Must complete at the fixed delivery head. |
| Independent review | GKD acceptance | Pending | Not performed by the executor. |

## Impact And Risk

- API, product behavior, migration, release configuration, GitHub settings, and legacy Trellis lifecycle are unchanged.
- No secrets, credentials, runtime receipts, or machine-local staging data were added to the repository.
- No plan deviation or unresolved implementation item remains.
- Review focus: strict canonical JSON and the conditional local-runner trigger in `scripts/check-gkd-adapter.mjs` and `scripts/check-local-verification.mjs`.
