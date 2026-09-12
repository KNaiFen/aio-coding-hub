# Cloud-Only Verification Contract

## 1. Scope

This repository prohibits local checks that generate large amounts of file
artifacts or sustain high CPU usage. The contract applies to README instructions,
root/workspace package scripts, Tauri build hooks, active AIO specs, `ci.yml`,
`codeql.yml`, `pr-title.yml`, `performance.yml`, and `dev-build.yml`.

## 2. Local Resource Boundary

Dependency installation, full tests and coverage, compilation, packaging, and
long-running performance checks use GitHub Actions. Necessary lightweight,
short-lived checks follow the approved `$gkd-main` plan. Project constraints
are recorded in [AGENTS.md](../../../../AGENTS.md).

## 3. Package And Tauri Boundaries

- Root and workspace package scripts are GitHub-Actions-only. Every retained
  script starts with the repository Actions environment guard; direct local use
  fails before a dependency-backed command starts.
- Root/workspace `preinstall` guards reject normal local package installation.
  Package-manager flags that suppress lifecycle scripts are still prohibited by
  repository policy and are not a supported bypass.
- The root `dev`, `preview`, precommit, and prepush entry points are absent.
- Tauri has no repository-managed `beforeDevCommand`. Its cloud build hook may
  call the guarded frontend build script because GitHub Actions owns that run.
- CI invokes the zero-dependency contract with `node`, not through `pnpm`.

## 4. Required Cloud Gates

`ci.yml` keeps `workflow_dispatch` and fail-closed automatic `ci-gate`
semantics. Manual runs are main-only and report `manual-ci-gate`, so they cannot
replace the protected branch check. Routine PR validation uses the automatic
workflow rather than a second manual run. Pull requests select frontend, Rust,
or both from the changed paths. Proven documentation-only PRs and `dev`/`main`
pushes skip both domains; pushes containing code or unknown paths and main
manual runs select both domains.

- `contracts` is the only dependency-free static contract job. It runs the
  cloud-only checker for checked documentation or either selected source
  domain, and runs the cloud-only self-test when a source domain is selected.
- `frontend` installs frozen dependencies, audits them, runs lint, both plugin
  package type checks and tests, root unit coverage, and the Vite build. The
  root coverage run discovers `src/e2e`; there is no separate E2E command.
  When selected, it starts only after `contracts` succeeds; the gate requires
  both to succeed. An unselected frontend domain is skipped.
- `rust` installs the pinned toolchain, runs Rust formatting and lock/binding
  canonicalization, fails with a bounded drift artifact when files change,
  then runs Clippy, Rust tests, and dependency audit. A frontend-only PR or a
  documentation-only PR/push leaves it unselected; shared/unknown paths and
  protected branch pushes containing code select it. Selected Rust and
  `observer-macos` jobs also wait for `contracts` success, then run alongside
  the selected frontend job. A selected job skipped because contracts failed
  still fails the aggregate gate. `candidate-plan` remains parallel to
  contracts, and candidate dependencies are unchanged.
- Candidate desktop/TUI jobs remain limited to eligible main commits or an
  explicit manual candidate request. They are skipped for PR branches and are
  not required for every PR.
- `pr-title.yml` checks pull request titles without checkout and reruns on title
  edits without starting full CI.
- Relevant automatic Rust paths retain the release benchmark. Manual CI omits
  it; `performance.yml` provides an explicit main-only benchmark without
  signing or release permissions.
- `dev-build.yml` has only the `workflow_dispatch` trigger and produces the
  selected unsigned integration artifact in GitHub Actions.
- `codeql.yml` reuses the same classifier for automatic PR/push events. Both
  no-build languages are skipped only after successful classification reports
  a documentation scope and explicit false frontend/Rust outputs. Source,
  shared, unknown, mixed, empty, or failed classification still selects both
  languages. Schedule/manual events skip classification and analyze both;
  failed or missing classifier outputs cannot prove documentation-only changes.
  A failed classifier job retains its failure while analysis proceeds unless
  the workflow was cancelled.

## 5. Drift Handling

GitHub Actions owns native/generated canonicalization and emits a bounded
patch when it detects drift. Corrections use the artifact from the affected SHA
and run attempt. Local regeneration and artifacts from a different source are
not supported.

## 6. Checker Coverage

`scripts/check-cloud-only-verification.selftest.mjs` covers failures when:

- a root/workspace script lacks the Actions guard or a local dev/precommit
  entry reappears;
- README presents an unavailable local package/native command, or an active
  AIO spec adds a bare package/native command or local quality-gate instruction;
- Tauri regains a local dev hook;
- `dev-build.yml` gains a non-manual trigger, manual CI can
  run heavy jobs outside `main`, or candidate desktop/TUI jobs stop
  being skipped outside eligible main runs;
- `contracts` stops invoking the production checker unconditionally;
- manual CI can report the same required check name.

`scripts/check-ci-quality-gates.selftest.mjs` owns the CI job and command
regressions, including failures when:

- a protected CI command is moved to a comment or non-`run` field;
- frontend/Rust selection stops using the classifier outputs, or `contracts` no
  longer runs for checked docs or either selected code domain;
- frontend/Rust/observer-macos loses the contracts dependency, success check,
  or status expression needed to handle the skipped automatic manual guard;
- `contracts` stops invoking the production checker, or source-only self-tests
  become eligible on process-documentation-only changes;
- frontend install/audit/lint/typecheck/test/build or Rust
  format/bindings/Clippy/tests/audit disappears;
- the automatic `ci-gate` no longer owns the selectable contracts/frontend/Rust
  results, or manual CI can report the same required check name;
- the PR title validation command or performance benchmark disappears;
- CodeQL loses its classifier inputs, outputs, full checkout history, read-only
  classification permissions, or proof required to skip both languages.

The quality-gate self-test maps existing classifier fixtures into the actual
workflow conditions and executes the unchanged Bash gate with selected,
unselected, failed, cancelled, and unexpectedly skipped job results.

`scripts/ci-change-scope.selftest.mjs` owns changed-path classification,
including full CI for shared/unknown paths and the documentation-only tiers.

The cloud-only checker retains README and active-spec checks for unavailable
local commands. It does not read AGENTS or parse GKD workflow prose. A focused
positive fixture accepts a concise GKD reference alongside the cloud contract.
The fixtures and repository scan use only built-in Node modules and write no
files.
