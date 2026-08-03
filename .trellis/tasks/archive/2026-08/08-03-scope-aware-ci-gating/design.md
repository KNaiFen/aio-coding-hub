# 按变更范围分级运行 CI - Technical Design

## Policy

Add `.github/ci-scope.json` as the single, reviewable classification policy.
Rules are evaluated with the following precedence:

1. Any path not explicitly allowed as process or checked documentation requires
   complete CI.
2. Checked documentation enables targeted documentation checks.
3. Process documentation enables no additional suite.

The classifier hard-codes `.github/**` and its own implementation/self-test as
complete-CI control-plane paths. The editable policy cannot downgrade itself or
the code that interprets it.

The process-documentation allowlist covers repository process records only:
`AGENTS.md`, `CHANGELOG.md`, `PENDING.md`, `PENDING_COMPLETED.md`, Markdown under
`omx_wiki/`, Markdown/JSON/JSONL task records under `.trellis/tasks/`, and
Markdown workspace journals under `.trellis/workspace/`.

The checked-documentation allowlist covers `README.md`, `README_EN.md`, Markdown
under `docs/`, Markdown under `.trellis/spec/` and `.trellis/agents/`, plus
`.trellis/workflow.md`. The JSON plugin API contract remains complete-CI scope.

## Classifier

Add a dependency-free Node.js module and CLI:

- Parse `git diff --name-status -z` so spaces and rename/copy records are safe.
- For pull requests, resolve `git merge-base <base> <head>` and diff from that
  commit to the head. For pushes, diff `<before>..<head>`.
- Inspect both paths of rename/copy records. Inspect the previous path for a
  deletion.
- Emit `scope`, `full_ci`, and `docs_checks` to `$GITHUB_OUTPUT`.
- Manual dispatch, all-zero/missing SHAs, empty changes, malformed policy, Git
  errors, and unclassified paths resolve to `full_ci=true`.
- Export pure classification helpers so a Node.js self-test can cover boundary
  behavior without a GitHub runner.

`full_ci` and `docs_checks` are independent. A mixed source plus checked-doc
change may set both true, allowing the full suite and targeted document checks
to run together.

## Workflow Graph

Add an always-running `change-scope` job before suite jobs. Keep `pr-title`
independent and cheap.

- `docs-contract` runs when `docs_checks=true` and invokes only Node.js scripts
  that do not require dependency installation.
- `support-contract`, `frontend`, `rust`, and `candidate-plan` require
  `full_ci=true`.
- Release build/assembly jobs keep their existing dependency on
  `candidate-plan`.
- `ci-gate` uses `always()`, depends on every possible path, requires the
  classifier and whichever suites were selected, and verifies unselected suites
  were skipped. This job remains the sole strict required check.

No workflow-level path filtering is introduced.

## Validation

- Run the classifier self-test and all targeted documentation checks locally.
- Exercise the CLI against known documentation-only and product-code history.
- Format/check changed non-Rust files and run `git diff --check` plus Trellis
  validation.
- Let GitHub Actions own Rust, generated bindings, audit, signing, and native
  validation.
- Merge the complete-CI implementation PR, then archive this task in a separate
  process-doc-only PR and verify the lightweight job graph before merging it.
