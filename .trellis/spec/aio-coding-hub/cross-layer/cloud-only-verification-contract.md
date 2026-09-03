# Cloud-Only Verification Contract

## 1. Scope

This repository keeps local worktrees free of dependency and build artifacts.
The contract applies to repository rules, README instructions, root/workspace
package scripts, Tauri build hooks, active AIO specs, `ci.yml`, `pr-title.yml`,
`performance.yml`, and `dev-build.yml`.

## 2. Local Allowlist

Local work follows the declared worktree plan and the `AGENTS.md` boundary. Do
not install dependencies, start development servers, or run package/native
quality gates locally. GitHub Actions owns dependency installation, formatting,
type checking, linting, tests, coverage, builds, generators, Cargo, Tauri,
signing, and packaging. A task may add another direct Node source contract only
when it imports no third-party package, does not spawn a prohibited tool, and
does not write.

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
or both from the changed paths; `dev`/`main` pushes and main manual runs always
select both domains.

- `contracts` is the only dependency-free static contract job. It runs the
  cloud-only checker for checked documentation or either selected source
  domain, and runs the cloud-only and local-runner self-tests when a source
  domain is selected.
- `frontend` installs frozen dependencies, audits them, runs lint, both plugin
  package type checks and tests, root unit coverage, and the Vite build. The
  root coverage run discovers `src/e2e`; there is no separate E2E command.
  It may be skipped only when the classifier proves no frontend/shared path is
  present, and it requires the selected `contracts` job to succeed.
- `rust` installs the pinned toolchain, runs Rust formatting and lock/binding
  canonicalization, fails with a bounded drift artifact when files change,
  then runs Clippy, Rust tests, and dependency audit. It may be skipped only for
  a frontend-only or documentation-only PR; shared/unknown paths and protected
  branch pushes select it.
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

## 5. Drift Handling

Never reproduce native/generated drift locally. Inspect the Actions artifact,
verify its file and semantic scope, apply only that bounded patch, rerun the
allowed dependency-free contracts, and submit the candidate to CI again. Do not
reuse artifacts from another SHA or run attempt.

## 6. Tests Required

The checker self-test must fail when:

- a root/workspace script lacks the Actions guard or a local dev/precommit
  entry reappears;
- README or AGENTS recommends a prohibited local command or bypasses the
  `$gkd-main` worktree handoff;
- Tauri regains a local dev hook;
- `dev-build.yml` or `performance.yml` gains a non-manual trigger, manual CI can
  run heavy jobs outside `main`, or candidate desktop/TUI jobs stop
  being skipped outside eligible main runs;
- a protected CI command is moved to a comment or non-`run` field;
- frontend/Rust selection stops using the classifier outputs, `contracts` no
  longer runs for checked docs or either selected code domain, or a
  shared/unknown path becomes cheap;
- `contracts` stops invoking the production checker, or source-only self-tests
  become eligible on process-documentation-only changes;
- frontend install/audit/lint/typecheck/test/build or Rust
  format/bindings/Clippy/tests/audit disappears;
- the automatic `ci-gate` no longer owns the selectable contracts/frontend/Rust
  results, or manual CI can report the same required check name;
- `pr-title.yml` checks out PR code, misses title edits, or is folded back into
  full CI.

The positive fixture and repository scan must run without dependencies and
without writing any file.
