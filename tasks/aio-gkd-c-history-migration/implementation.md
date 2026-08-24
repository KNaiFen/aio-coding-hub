# AIO GKD Historical Task Adapter Implementation

## Internal Design

- `.gkd/history-adapter.json` is the single declarative source for AIO's tracked history roots and active/archive handling rules.
- `scripts/check-gkd-adapter.mjs` validates that declaration alongside the existing pinned project facts without acquiring history or lifecycle behavior.
- `scripts/check-gkd-history.mjs` owns only the AIO Trellis compatibility read: it obtains tracked paths from Git, classifies active versus archived manifests, validates their minimal historical invariants, and emits a redacted deterministic summary.
- The existing local runner invokes the history selftest and checker only when history adapter/checker surfaces change, preserving the zero-artifact local boundary.

## Execution Details

1. Add the canonical history declaration and strict adapter validation with negative selftests for unknown fields and policy drift.
2. Implement the tracked-only history checker and isolated Git fixture selftest for one/zero/multiple active tasks, legacy active state, stale archived paths, malformed archives, untracked substitution, and repeat-run stability.
3. Wire the two history scripts into the existing versioned local verifier and update its trigger selftests.
4. Update the adapter operations guide and root fact inventory; do not edit Trellis task manifests or old lifecycle implementation.
5. Run the registered AIO local verifier, prove tracked Trellis task paths are unchanged, create/update one PR, and deliver a canonical fixed head with `delivery.md` as the penultimate commit.
6. Stop before acceptance, merge, records-only closeout, cleanup, milestone D, release, deployment, production, or GitHub settings changes.
