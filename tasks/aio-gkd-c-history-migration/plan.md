# AIO GKD Historical Task Adapter Plan

## Goal

Finish milestone C with a narrow, read-only compatibility adapter for AIO's tracked Trellis history while using canonical GKD for all new task coordination.

## User Decisions

- Continue the approved adoption sequence automatically from the accepted B4 main head.
- Keep historical manifests immutable and classify machine-local worktree paths as ignored archive evidence, never live state.
- Keep the original checkout's untracked remediation plan outside Git and outside the history scan.
- Use one exact automatic `gkd_executor`; trusted main owns independent acceptance, merge, records, and cleanup.

## Behavior And Defaults

- Discover manifests from the Git index, not from an unrestricted filesystem walk.
- Treat immediate tracked children of `.trellis/tasks` as active and tracked descendants of `.trellis/tasks/archive` as historical.
- Require exactly one active Trellis manifest on the current project line. Active `worktree_path` must be null and `coordination.version=1` is rejected as a legacy execution route.
- Require archived manifests to have `status=completed`; ignore the value and existence of their historical `worktree_path` without emitting it.
- Keep the checker deterministic, read-only, zero-dependency, and free of network or runtime attachment writes.

## Scope

- New canonical `.gkd/history-adapter.json` with exact AIO history facts.
- New `scripts/check-gkd-history.mjs` and `scripts/check-gkd-history.selftest.mjs`.
- Existing adapter validator/selftest and local verifier/selftest changes required to bind and execute the history check.
- Minimal updates to `docs/operations/gkd-adapter.md` and the root `AGENTS.md` fact inventory.
- Canonical task planning, automatic routing, fixed-head delivery, independent acceptance, and records-only closeout.

## Non-Goals

- No Trellis manifest rewrites, old-worktree deletion, legacy lifecycle refactor, GKD core fork, workflow changes, product changes, release, deployment, or production installation.
- No use of a Trellis manifest as a canonical GKD migration document and no handwritten GKD attachment, task, offer, claim, delivery, review, acceptance, or receipt state.
- No milestone D optimization/integration or milestone E deletion.

## Acceptance Criteria

- All requirements criteria pass, including strict project-fact validation, tracked-only discovery, fail-closed active locator fixtures, archived stale-path compatibility, repeat-run stability, unchanged Trellis task bytes, local verification, and required fixed-head CI.

## Compatibility

- Existing `.gkd/policy.json`, bundle pin, review/resource adapters, B4 adapter policy, GitHub workflows, required checks, product paths, and canonical GKD task behavior remain unchanged.
- Existing completed Trellis history remains readable even when its recorded worktree no longer exists.

## Security And Data

- Store only repository-relative roots and enum-like policy values. Reject unknown fields, symlinks, malformed JSON, untracked manifest substitution, path escape, active machine paths, and ambiguous active selection.
- Do not print historical worktree paths, runtime capabilities, credentials, prompts, transcripts, or machine-local attachment contents.

## Migration

- The fixed base contains zero tracked active Trellis v1 tasks with an absolute worktree path, so no active manifest rewrite is eligible.
- Existing archived Trellis tasks stay legacy-read-only; their stale worktree values are ignored rather than resolved or rewritten.
- Canonical GKD v1 migration remains owned by the installed bundle and is not generalized to the Trellis schema.

## Public Interfaces

- New AIO-only `.gkd/history-adapter.json` and `scripts/check-gkd-history.mjs` machine result.
- Existing `scripts/gkd-verify --base-sha <full-sha>` remains the sole local verification entrypoint and gains the history selftest/check for relevant changes.

## Execution Route

- Automatic route through the verified published GKD `v0.1.5` bundle, one exact direct `gkd_executor`, and the canonical offer/claim bridge.

## External Side Effects

- Candidate branch commits, one AIO implementation PR, scope-related CI repair, independent fixed-head merge, one records-only closeout PR, and isolated cleanup.

## Action Mode

- `implement_and_merge_on_acceptance` with `ci_repair`, `commit`, `conditional_merge`, `pr_update`, `push`, and `ready_for_review`.

## Implementation Notes

- Reuse the existing canonical JSON and strict exact-key helpers in `scripts/check-gkd-adapter.mjs`; keep history enumeration in its own module.
- Use `git ls-files -z -- .trellis/tasks` and filter repository-relative `task.json` paths. Tests must initialize isolated Git fixtures so untracked files can prove they are ignored.
- Compare `git diff --name-only` or blob digests for `.trellis/tasks/**` against the base to prove no tracked history was changed.
- Run only `scripts/gkd-verify --base-sha 3f856c88749f4875889164fa72caeebc22143d98` locally; dependency, frontend, Rust, build, packaging, release, and deployment work remain cloud-owned or out of scope.
