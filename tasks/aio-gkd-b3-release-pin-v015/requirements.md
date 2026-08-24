# AIO GKD v0.1.5 Bundle Pin

## Goal

Upgrade AIO's consumer pin from published GKD `v0.1.4` to published GKD `v0.1.5`, which contains the accepted R9 GitHub acceptance and delivery sequencing repair.

## User Decisions

- The user authorized AIO task PRs, scope-related CI repair, fixed-head acceptance, and merge after the R9 stable release and isolated restage.
- The pin must use only the verified `v0.1.5` GitHub Release asset, never GKD source or production staging.
- The original AIO checkout's untracked `.trellis/tasks/08-17-gkd-workflow-remediation/` is outside the task and must never be deleted, overwritten, or committed.

## Scope

- Update `.gkd/bundle-pin.json` to the exact `v0.1.5` source, execution bundle digest, and asset SHA-256.
- Update the existing adapter validator, adapter selftest fixture, and adapter documentation so their strict expected release facts match the new pin.
- Deliver one fixed-head AIO PR; trusted main performs independent acceptance and merge only after local verification and policy-required CI succeed.

## Non-Goals

- Do not change `.gkd/policy.json`, `.gkd/review-adapter.json`, `.gkd/resource-facts.json`, workflows, runners, GitHub settings, Secrets, production `~/.codex`, release/version behavior, Trellis history, or AIO product code.
- Do not implement the remaining adapter policy, historical-task migration, or CI/release integration phases.

## Acceptance Criteria

- [ ] The pin is canonical JSON and exactly binds `v0.1.5`, source `60ac0c49f1054ce2edea49b3ab6758bfbd3432b3`, bundle `d749b753fb11aeab44d41b4e1d8bec44c7fa2d18a4b08148fbc0e0c127e27e6d`, and asset SHA-256 `f259475f4ca6c3425e53d734d03633541d6a1997e41991eb5a6115958d06a298`.
- [ ] Existing adapter smoke and selftest accept the new exact pin and reject prior or tampered release facts.
- [ ] The versioned AIO local verifier and fixed-head required checks `ci-gate` and `pr-title` pass; the candidate stops before release or production side effects.
