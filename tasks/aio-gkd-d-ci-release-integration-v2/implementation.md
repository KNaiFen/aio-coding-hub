# AIO GKD CI and Release Adapter Implementation

## Internal Design

- `.gkd/ci-release-adapter.json` is the single AIO declaration for the live speed-first recommendation, local/cloud boundaries, redaction policy, artifact/cache bounds, required checks, and same-SHA finalization.
- `scripts/check-gkd-ci-release.mjs` validates the declaration, checks the workflow guard surface and release-promotion source binding, and emits only deterministic redacted facts.
- The selftest uses isolated fixture text and mutation cases to prove unknown-field rejection, recommendation drift, leak redaction, gate skip rejection, artifact retention bounds, and source-SHA mismatch rejection.
- Existing `check-gkd-adapter` and `check-local-verification` remain orchestration entrypoints; no AIO copy of GKD lifecycle or monitor code is introduced.

## Execution Details

1. Add the canonical declaration and strict checker/selftest with negative mutations first.
2. Wire the checker into the adapter and local verification smoke triggers, then add the minimal `ci.yml` contracts invocation and documentation facts.
3. Validate that current cloud jobs remain independent, the aggregate gate is fail-closed, artifacts/caches are bounded, and release promotion requires the same source SHA without performing writes.
4. Run the registered AIO local verifier, prove `.trellis/tasks/**` is unchanged, create/update one PR, and deliver the final fixed head with `delivery.md` as the penultimate commit.
5. Stop before acceptance, merge, records-only closeout, cleanup, tag, Release, deployment, or GitHub settings changes.
