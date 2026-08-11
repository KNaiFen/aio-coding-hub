# Cloud-Only Verification Contract

## 1. Scope

This repository keeps local worktrees free of dependency and build artifacts.
The contract applies to repository rules, README instructions, root/workspace
package scripts, Tauri build hooks, Trellis workflow/agent guidance, active AIO
specs, `ci.yml`, `pr-title.yml`, `performance.yml`, and `dev-build.yml`.
Historical tasks, archived records, and workspace journals remain historical
evidence and are not rewritten.

## 2. Local Allowlist

Local execution is limited to direct, dependency-free, non-writing checks:

```bash
node scripts/check-cloud-only-verification.selftest.mjs
node scripts/check-cloud-only-verification.mjs
node --check <changed-file.mjs>
git diff --check
```

Do not invoke those checks through a package manager. Repository dependency
installation, development servers, formatting, type checking, linting, tests,
coverage, builds, generators, Cargo, Tauri, signing, and packaging are all
cloud-owned, even if a previous checkout already contains dependencies or
targets. A task may add another direct Node source contract only when it imports
no third-party package, does not spawn a prohibited tool, and does not write.

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

- `docs-contract` and `support-contract` run the cloud-only checker directly;
  the support job runs its self-test first.
- `frontend` installs frozen dependencies, audits them, runs lint, both plugin
  package type checks and tests, GUI E2E, unit coverage, and the Vite build.
  It may be skipped only when the classifier proves no frontend/shared path is
  present.
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
- README or active Trellis guidance recommends a prohibited local command;
- Tauri regains a local dev hook;
- `dev-build.yml` or `performance.yml` gains a non-manual trigger, manual CI can
  run heavy jobs outside `main`, or candidate desktop/TUI jobs stop
  being skipped outside eligible main runs;
- a protected CI command is moved to a comment or non-`run` field;
- frontend/Rust selection stops using the classifier outputs, support no longer
  runs for either selected code domain, or a shared/unknown path becomes cheap;
- the support/docs contract stops invoking the checker;
- frontend install/audit/lint/typecheck/test/build or Rust
  format/bindings/Clippy/tests/audit disappears;
- the automatic `ci-gate` no longer owns the selectable frontend/Rust/support
  results, or manual CI can report the same required check name;
- `pr-title.yml` checks out PR code, misses title edits, or is folded back into
  full CI.

The positive fixture and repository scan must run without dependencies and
without writing any file.
