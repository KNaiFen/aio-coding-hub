# AIO GKD Project Adapter Policy

## Goal

Complete milestone B's remaining AIO-specific adapter policy by binding the repository's existing local verification, CI resource/artifact, and release promotion contracts without changing GKD's generic policy schema or copying generic workflow logic.

## User Decisions

- The user authorized the remaining AIO adapter policy task, its PR, scope-related CI repair, fixed-head acceptance, and merge after the verified GKD `v0.1.5` restage.
- GKD `v0.1.5` owns the strict generic `.gkd/policy.json` schema; AIO-specific fields must not be added to that file.
- The original AIO checkout's untracked `.trellis/tasks/08-17-gkd-workflow-remediation/` is outside the task and must never be deleted, overwritten, or committed.

## Scope

- Add one canonical AIO-owned `.gkd/adapter-policy.json` containing only declarative facts already enforced by the repository's versioned local runner and GitHub workflows.
- Bind zero-artifact local verification and its cloud-owned boundary; the existing GitHub-hosted runner/cache and bounded artifact-retention classes; and the existing tag, main-ancestor, same-SHA candidate, checksum, and immutable-release promotion contract.
- Extend the existing adapter validator and selftest to validate the new file strictly, reject drift and unknown fields, and keep the published `v0.1.5` bundle pin immutable.
- Update the adapter operations document and root `AGENTS.md` fact inventory to identify the new AIO-only policy and its boundary.
- Deliver one fixed-head AIO PR; trusted main performs independent acceptance and merge only after the versioned local verifier and policy-required CI succeed.

## Non-Goals

- Do not modify GKD canonical source, `.gkd/policy.json`, `.gkd/review-adapter.json`, `.gkd/resource-facts.json`, `.gkd/bundle-pin.json`, workflows, runner configuration, required checks, GitHub settings, Secrets, package versions, product code, Trellis history, production `~/.codex`, tags, Releases, or deployments.
- Do not migrate historical tasks, change CI implementation, run a release, or add dynamic workflow/API discovery.
- Do not duplicate GKD task, role, claim, acceptance, monitor, review, resource-scanner, or release implementation in AIO.

## Acceptance Criteria

- [ ] `.gkd/adapter-policy.json` is canonical JSON with an exact schema and no machine-local or secret data.
- [ ] The policy exactly binds `scripts/gkd-verify` with a full lowercase base SHA, zero-artifact local work, the current cloud-owned validation categories, GitHub-hosted runner/cache classes, artifact name/retention classes, `aio-coding-hub-v{semver}`, main ancestry, successful same-SHA main CI candidate selection, `SHA256SUMS.txt`, and immutable equivalent existing-release assets.
- [ ] The existing adapter validator rejects unknown fields and drift in verification, runner/cache, artifact, tag, candidate-SHA, checksum, main-ancestry, or immutability facts.
- [ ] Existing generic policy/project staging remains valid and the exact GKD `v0.1.5` bundle pin does not change.
- [ ] `scripts/gkd-verify --base-sha b35e34245a1667e647965be58ba44654ca0ba053`, `git diff --check`, and fixed-head `ci-gate`/`pr-title` pass; the candidate stops before acceptance, merge, cleanup, or release side effects.
