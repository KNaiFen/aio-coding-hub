# 按变更范围分级运行 CI

## Goal

区分流程文档、受契约检查的文档与代码/配置变更，仅对后者运行完整 CI，同时保留严格 required ci-gate。

## Requirements

- Keep the required `ci-gate` job present on every `dev`/`main` pull request and
  push. Do not use workflow-level `paths-ignore`, because a skipped required
  workflow can leave the check pending.
- Classify every changed path into one of three execution tiers:
  - process documentation: run change classification, PR-title validation when
    applicable, and `ci-gate` only;
  - checked documentation: additionally run the repository's targeted Node.js
    documentation and contract checks;
  - code/configuration/unknown changes: run the existing complete CI graph.
- Use an explicit allowlist for both documentation tiers. Markdown extensions
  and a top-level `docs/` directory alone are not sufficient proof that a path
  is documentation-only.
- Treat `.github/**`, executable Trellis tooling, lockfiles, source code,
  build/release configuration, machine-readable plugin contracts, mixed
  changes, and unknown paths as complete-CI changes.
- Classify both source and destination paths for renames/copies, and classify
  deleted paths using their previous path.
- Compare pull requests against the merge base, pushes against the event's
  `before` SHA, and manual dispatches as complete CI.
- Fail closed to complete CI when the event range, policy, diff, or
  classification cannot be resolved safely.
- Preserve all current release-candidate behavior for complete-CI changes, but
  skip candidate planning and builds on documentation-only pushes.

## Acceptance Criteria

- [ ] A process-documentation-only PR reports a successful required `ci-gate`
      while frontend, Rust, support-contract, documentation-contract, and
      release jobs are skipped.
- [x] A checked-documentation-only PR runs the targeted Node.js documentation
      checks and reports a successful required `ci-gate`, without running the
      frontend or Rust suites.
- [x] Any code, configuration, workflow, classifier-policy, machine-readable
      contract, mixed, unknown, or unsafe diff runs the existing complete CI.
- [x] Rename, copy, deletion, empty-diff, invalid-SHA, and classification-error
      cases are covered by deterministic Node.js self-tests.
- [x] The implementation PR runs and passes the complete CI on GitHub Actions.
- [ ] A separate Trellis archive PR containing only process documentation
      proves the lightweight route in the actual protected-branch workflow.
- [x] The boundary and fail-closed rules are recorded in the cross-layer Trellis
      specification.

## Notes

- This change does not move existing documentation or workflow files. The
  policy owns the boundary centrally and can evolve without coupling repository
  layout to CI cost.
- No product version bump or release is required.

## Verification Evidence

- Implementation commit: `80c4cbd56f6b1f0171bcbe72658115273c68c35a`.
- Implementation PR: <https://github.com/KNaiFen/aio-coding-hub/pull/32>.
- Pull-request complete CI: <https://github.com/KNaiFen/aio-coding-hub/actions/runs/30825901633>.
- Merged `main` commit: `131a4b92bab18bec7908523d74e65c77d22a066e`.
- Merged-commit complete CI: <https://github.com/KNaiFen/aio-coding-hub/actions/runs/30827695672>.
- The classifier self-test covers each documentation tier and fail-closed edge
  case; the implementation PR also exercised `docs-contract` together with the
  complete suites.
- The separate process-documentation archive PR remains the live proof for the
  minimal `change-scope` + `pr-title` + `ci-gate` path.
