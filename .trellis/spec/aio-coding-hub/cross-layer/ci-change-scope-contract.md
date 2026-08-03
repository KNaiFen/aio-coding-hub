# CI Change-Scope Contract

### 1. Scope / Trigger

Apply this contract whenever CI job selection, the required `ci-gate`, the
change-scope classifier, or a path listed in `.github/ci-scope.json` changes.
The policy is an execution boundary: repository layout and file extensions do
not implicitly grant a cheaper CI route.

### 2. Execution Tiers

| Tier                  | Required jobs                                                 | Intended content                                                         |
| --------------------- | ------------------------------------------------------------- | ------------------------------------------------------------------------ |
| Process documentation | `change-scope`, PR title when applicable, `ci-gate`           | Task records, pending lists, agent/process notes                         |
| Checked documentation | Process jobs plus `docs-contract`                             | READMEs, product Markdown, Trellis specs/workflow                        |
| Complete CI           | Existing support, frontend, Rust, and eligible candidate jobs | Code, configuration, contracts, tooling, locks, unknown or mixed changes |

The required workflow must always start. Workflow-level `paths-ignore` is
forbidden because a skipped required workflow can leave branch protection
waiting for a check that never reports.

### 3. Classification Contract

- Only exact paths and prefix-plus-extension rules in `.github/ci-scope.json`
  may select either documentation tier.
- `.github/**` and the classifier/self-test scripts are immutable control-plane
  exceptions: code hard-codes them to complete CI, so the policy cannot grant
  itself or its interpreter a cheaper route.
- `.github/**`, `.trellis/scripts/**`, `.trellis/config.yaml`, source and build
  files, lockfiles, and `docs/plugins/plugin-api-v1-contract.json` require
  complete CI. Unknown paths also require complete CI.
- A path matching both documentation tiers requires complete CI; policy
  ambiguity must never reduce validation.
- Checked documentation and complete CI are independent flags. A mixed change
  can run both the targeted documentation job and the complete suites.
- Pull requests use their base/head merge base. Pushes use the event's
  `before` and head objects. Manual and unsupported events require complete CI.
- Parse NUL-delimited `git diff --name-status` records. Renames and copies
  classify both old and new paths; deletions classify the old path.
- Invalid/all-zero SHAs, missing history, Git errors, malformed policy,
  malformed diff output, and empty diffs fail closed to complete CI.

### 4. Workflow Contract

- `change-scope` is the only owner of `scope`, `full_ci`, and `docs_checks`
  outputs and must run before selectable suites.
- `docs-contract` uses dependency-free Node.js checks only. It must validate
  plugin documentation, the plugin API contract, Trellis spec links, and the
  standalone TUI README/release contract.
- Support, frontend, Rust, and candidate planning run only when
  `full_ci=true`. Candidate assembly retains all existing version-change and
  signed-build requirements.
- `ci-gate` uses `always()`, depends on every selectable job, and validates both
  selected successes and unselected `skipped` results. A classifier failure,
  missing output, unexpected skip, cancellation, or selected-suite failure must
  fail the gate.
- `ci-gate` remains named exactly `ci-gate` so the protected branch's strict
  required-check rule stays stable.

### 5. Change Rules

- Add a path to a documentation tier only after tracing every runtime, build,
  test, packaging, generation, and release consumer.
- Put machine-readable contracts in complete CI unless the targeted job proves
  all consumers are covered.
- Changes to the policy, classifier, self-test, or workflow themselves require
  complete CI and must preserve the hard-coded control-plane exception.
- Moving files into a documentation-looking directory is not a substitute for
  classifying their behavior.

### 6. Tests Required

- Exact paths, prefix extension filters, unknown files, and mixed tiers.
- Rename/copy in both directions across tier boundaries, plus deletions.
- Pull-request merge-base and push-before range construction.
- Empty output, invalid SHA, malformed name-status records, and policy errors.
- A complete-CI implementation PR followed by a real process-doc-only PR that
  proves the lightweight protected-branch route.
