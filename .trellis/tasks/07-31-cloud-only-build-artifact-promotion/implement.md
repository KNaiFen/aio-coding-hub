# Implementation Plan

## 1. Remove local native-build triggers

- [ ] Delete tracked pre-commit/pre-push hooks and their installer.
- [ ] Remove `postinstall`, hook installation, native precommit/prepush aliases and all repository-approved local Rust/Tauri package commands.
- [ ] Simplify remaining local aggregate checks to Node/TypeScript-only work.
- [ ] Clear this clone's `core.hooksPath` only when its exact value is `.githooks`.
- [ ] Add a static regression that rejects automatic hook installation and local native command aliases.

Rollback point: before workflow changes, verify `pnpm install`, `git commit --dry-run`-equivalent inspection and push configuration have no repository hook path. Do not run a native command to prove the negative.

## 2. Create cloud-native drift handling

- [ ] Move frozen pnpm install, Rust formatting, minimal `Cargo.lock` synchronization and generated binding ownership into the Rust CI job.
- [ ] Distinguish normalization command failure from drift; on drift upload a path-limited `cloud-native-fixes.patch` in an independent step, then fail in a later step before remaining Rust gates.
- [ ] Keep Clippy, full locked Rust tests and Cargo audit after a clean canonicalization gate.
- [ ] Remove Tauri system dependencies and Rust-backed binding generation from the frontend job.
- [ ] Add static/self-tests for patch path restrictions, naming and failure behavior.

Rollback point: frontend and Rust CI remain separate jobs; only the ownership of generated native files moves.

## 3. Extend the shared support/release contract

- [ ] Extend `support-matrix.mjs` with four-file synchronized version detection, release-candidate planning, manual cloud matrices, deterministic manifest creation and strict verification.
- [ ] Add self-tests for canonical success plus unknown schema, path traversal, duplicate/missing/extra files, bad repository/SHA/version/tag/run attempt/targets, size and SHA mismatch.
- [ ] Reverse static workflow assertions: CI must build final candidates; Release must not contain native setup/build/signing or fallback paths.
- [ ] Update README matrix generation from local commands to cloud workflow targets.

Rollback point: existing stable asset naming and updater `latest.json` mapping remain unchanged.

## 4. Build release candidates in main CI

- [ ] Add current-main-only typed `workflow_dispatch` recovery inputs; require a full candidate SHA and source-derived tag, keep a trusted control checkout, verify fetched ancestry and an existing successful exact-SHA main CI, then build from a separate source checkout.
- [ ] Let a first-time version-changing main push use the current run's required quality jobs instead of requiring impossible prior exact-SHA success; bind the source-validation run in both modes.
- [ ] Create/use the protected `release-signing` Environment in candidate jobs; do not reference repository-level signing secrets from CI.
- [ ] Make `main` concurrency unique per run while retaining cancellation for superseded PR/dev runs.
- [ ] Add trusted main-only Windows x64/macOS ARM64 build jobs with locked dependencies and current signing behavior.
- [ ] Upload run-attempt-specific one-day platform artifacts.
- [ ] Add an assembly job that depends on all required validation/build jobs, accepts only current-run/current-attempt platform artifacts, verifies inputs, writes the manifest including trusted control/source-validation fields and uploads one immutable 30-day final candidate.
- [ ] Fail closed on partial failed-job reruns that would require cross-attempt artifact mixing; document full rerun or a fresh recovery run.
- [ ] Ensure failed overall CI cannot produce a final candidate even if a platform temporary artifact exists.

Rollback point: candidate jobs are conditionally isolated from ordinary PR and non-version main CI.

## 5. Convert Release to artifact promotion

- [ ] Remove Release build matrix, Rust/pnpm dependency setup, Tauri action and signing secret access.
- [ ] Select a successful exact-SHA main candidate or current-main recovery candidate whose manifest binds the requested source SHA, control SHA and run attempt, then use its exact non-expired artifact ID.
- [ ] Split Release into read-only `resolve-and-verify` and write-capable `publish` jobs; pass only exact identifiers and independently re-download/reverify the same artifact ID on the clean publish runner.
- [ ] Use the draft's stable creation timestamp for updater metadata, upload verified bytes, generate checksums and publish.
- [ ] Validate an existing draft's tag/target/status and workflow ownership marker, re-peel the fetched tag before mutation and publication, clear old managed assets, upload the complete set, then require exact remote names, sizes and SHA-256 digests before publish.
- [ ] Add workflow-contract tests proving all invalid/missing/expired/ambiguous cases fail without fallback build, and draft re-runs cannot publish stale extra assets.

Rollback point: a tag run may be rerun after a cloud candidate recovery, but must never compile.

## 6. Replace local builds with manual cloud builds

- [ ] Refactor `dev-build.yml` to a target selector covering the six existing support-matrix targets.
- [ ] Pin checkout to dispatch-captured `github.sha`, include target ID in concurrency, distinguish universal Tauri/rustup targets, and set explicit `tauriScript` plus locked Cargo arguments.
- [ ] Generate and pass an ephemeral cloud config overlay that disables updater artifacts without changing the tracked production config.
- [ ] Keep artifacts unsigned, development-labelled, seven-day retained and ineligible for Release.
- [ ] Document Vite as the only local development server and the cloud workflow as the native integration/build path.

## 7. Remove stale automation and update durable guidance

- [ ] Remove release-please Cargo.lock workflow/config/manifest/check script and every reference.
- [ ] Update `AGENTS.md` with a hard rule that agents do not run local Rust/Tauri or generated-binding commands.
- [ ] Update Chinese/English README and live Trellis specs.
- [ ] Add `cloud-build-release-artifact-contract.md` and link it from the cross-layer index.
- [ ] Add a supersession note to unfinished task plans that still prescribe future local native commands; preserve archived and completed historical evidence.

## 8. Local verification: Node/metadata only

- [ ] `node scripts/support-matrix.mjs check`
- [ ] release-candidate manifest self-tests
- [ ] hook/local-native-trigger static self-tests
- [ ] `pnpm check:spec-links`
- [ ] `pnpm typecheck`
- [ ] `pnpm lint`
- [ ] focused Vitest tests, then `pnpm test:unit`
- [ ] `pnpm build` (frontend only)
- [ ] `git diff --check`
- [ ] `python ./.trellis/scripts/task.py validate 07-31-cloud-only-build-artifact-promotion`

Explicit prohibition: do not run Cargo, rustfmt, Clippy, Rust tests, Specta generation, `pnpm exec tauri`, Tauri dev/build, or any package script that invokes them locally.

## 9. Cloud verification and review gates

- [x] Resolve the recorded `main`/release-tag Ruleset and `release-signing` Environment decisions before task activation; user approved both.
- [ ] Configure release-tag governance as separate creation and immutable update/deletion Rulesets; never place all three rules behind one maintainer bypass.
- [ ] If authorized, create the Environment/deployment policy and use a pinned reviewed sealed-box bridge to migrate both existing secrets without plaintext disclosure; retain both repository secrets unless both Environment writes and the cloud probe succeed.
- [ ] Disable Environment administrator bypass in GitHub's web settings and re-read `can_admins_bypass=false` through the API before uploading either secret.
- [ ] Run a cloud sign/verify probe through the Environment, delete and re-list repository-level secret names, then remove the migration helper/workflow.
- [ ] Push an implementation branch to `origin` and run PR CI.
- [ ] If native canonicalization drifts, download and apply only the emitted patch artifact; do not regenerate locally.
- [ ] Require frontend, Rust, support-contract, desktop-contract and workflow self-tests to pass.
- [ ] On a trusted `main` validation run, verify candidate manifest SHA/version/run attempt and 30-day retention.
- [ ] Verify a Release dry-path or authorized test tag performs no build and publishes byte-identical candidate files.
- [ ] Run an independent code/security review before merge; do not alter branch rules, version, tag or Release without separate authorization.

## 10. Completion evidence

- [ ] Record exact local Node-only commands and results.
- [ ] Record PR/main workflow run IDs, job conclusions and artifact IDs/expiry.
- [ ] Record binary SHA equality between candidate and promoted Release when a release is authorized.
- [ ] Update the implementation status and cross-layer spec, then commit through the normal Trellis finish flow.
