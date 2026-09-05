# CI Change-Scope Contract

### 1. Scope / Trigger

Apply this contract whenever CI job selection, the required `ci-gate`, the
change-scope classifier, or a path listed in `.github/ci-scope.json` changes.
The policy is an execution boundary: repository layout and file extensions do
not implicitly grant a cheaper CI route.

### 2. Execution Tiers

| Tier                  | Required jobs                                                 | Intended content                                                             |
| --------------------- | ------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| Process documentation | `change-scope`, independent `pr-title`, `ci-gate`             | Task records, pending lists, agent/process notes                             |
| Checked documentation | Process jobs plus `contracts`                                 | READMEs, product Markdown, Trellis specs/workflow                            |
| Frontend CI           | `contracts`, `frontend`, `ci-gate`                            | Frontend source, assets, and frontend-only workspace packages                |
| Rust CI               | `contracts`, `rust`, `ci-gate`                                | Rust source, Cargo manifests, and Cargo lockfiles                            |
| Complete CI           | Contracts, frontend, Rust, and eligible candidate jobs        | Shared/generated files, CI control plane, unknown or mixed frontend/Rust changes |

The required workflow must always start. Workflow-level `paths-ignore` is
forbidden because a skipped required workflow can leave branch protection
waiting for a check that never reports.

### 3. Classification Contract

- Only exact paths and prefix-plus-extension rules in `.github/ci-scope.json`
  may select a documentation, frontend, or Rust-only tier.
- `.gkd/` Markdown and the existing root `plan.md`, `progress.md`, and
  `review.md` are process records with no build or release consumers. Other
  extensions under `.gkd/` remain unknown and require complete CI. `AGENTS.md`
  is checked documentation so handoff changes run the cloud-only contract.
- `.github/**` and the classifier/self-test scripts are immutable control-plane
  exceptions: code hard-codes them to complete CI, so the policy cannot grant
  itself or its interpreter a cheaper route.
- `.github/**`, root dependency files, CI/tooling scripts, generated frontend bindings, and
  `docs/plugins/plugin-api-v1-contract.json` require complete CI. Unknown paths
  also require complete CI.
- A path matching conflicting documentation or source tiers requires complete
  CI; policy ambiguity must never reduce validation. An explicit shared rule
  takes precedence over a broader frontend prefix.
- Checked documentation, frontend, Rust, and shared selection are independent
  flags. Mixed documentation plus one source domain runs the targeted docs and
  source jobs; frontend plus Rust or any shared path runs complete CI.
- Pull requests use their base/head merge base and may use domain selection.
  Pushes use the event's `before` and head objects. Proven documentation-only
  pushes keep their documentation tier; any code or unknown path forces complete
  CI for `dev`/`main` integration. Main-only manual and unsupported events also
  require complete CI; manual CI omits the release benchmark because it has a
  dedicated workflow.
- Parse NUL-delimited `git diff --name-status` records. Renames and copies
  classify both old and new paths; deletions classify the old path.
- Invalid/all-zero SHAs, missing history, Git errors, malformed policy,
  malformed diff output, and empty diffs fail closed to complete CI.

### 4. Workflow Contract

- `change-scope` is the only owner of `scope`, `full_ci`, `frontend_ci`,
  `rust_ci`, `shared_ci`, and `docs_checks` outputs and must run before
  selectable suites.
- `contracts` is the only dependency-free static contract job. It runs when
  checked documentation or either source domain is selected. Step conditions
  keep docs-only checks on `docs_checks`, source self-tests on frontend/Rust
  selection, and plugin docs/API checks on checked docs or frontend selection.
  Frontend, Rust, and contracts run in parallel after successful classification;
  `ci-gate` requires all selected jobs to succeed. Candidate planning requires
  `full_ci=true` and retains the main/version-change and signed-build requirements.
- `ci-gate` uses `always()`, depends on every selectable job, and validates both
  selected successes and unselected `skipped` results. A classifier failure,
  missing output, unexpected skip, cancellation, or selected-suite failure must
  fail the gate.
- The automatic gate remains named exactly `ci-gate` so the protected branch's
  strict required-check rule stays stable. Manual runs use `manual-ci-gate` and
  cannot satisfy that rule. PR title validation is a separate required check.

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

- Exact paths, prefix extension filters, frontend-only, Rust-only, shared,
  unknown files, and mixed tiers.
- Rename/copy in both directions across tier boundaries, plus deletions.
- Pull-request merge-base and push-before range construction.
- Empty output, invalid SHA, malformed name-status records, and policy errors.
- A complete-CI implementation PR followed by real frontend-only, Rust-only,
  and process-doc-only PRs that prove each protected-branch route.
