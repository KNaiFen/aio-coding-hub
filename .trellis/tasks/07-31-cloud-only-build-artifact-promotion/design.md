# Technical Design

## 1. Current-State Boundaries

### 1.1 Local trigger chain

```text
pnpm install
  -> package.json postinstall
  -> scripts/install-git-hooks.mjs
  -> git config core.hooksPath=.githooks

git commit
  -> .githooks/pre-commit
  -> cargo fmt / cargo check / Cargo.lock writeback

git push
  -> .githooks/pre-push
  -> generated bindings / cargo test / cargo clippy
```

This chain is especially unsafe because ordinary package installation changes repository-local Git configuration, and a commit can both compile and mutate tracked files. `check:generated-bindings` is also not a read-only check: it runs a Rust example and Prettier writes the generated TypeScript file.

### 1.2 Current cloud flow

```text
PR/main CI
  -> frontend checks + a separate Rust-backed binding generation
  -> Rust fmt/clippy/test/audit
  -> no retained release artifact

release tag
  -> find successful CI for SHA
  -> install Node/Rust/dependencies again
  -> build and sign Windows x64 + macOS ARM64
  -> upload one-day temporary artifacts
  -> assemble and publish
```

The CI result is reused only as an authorization signal. No compiled release bytes are reused.

## 2. Target Data Flow

```text
local edit
  -> local Node/TypeScript checks only
  -> push branch
  -> cloud PR CI
       -> canonical Rust/lock/bindings check
       -> drift patch artifact on mismatch
       -> Rust test/clippy/audit
       -> frontend checks
  -> merge exact version commit to main
  -> one main CI run
       -> quality jobs -----------------------------+
       -> signed platform builds in parallel -------+--> final candidate manifest/artifact
  -> tag exact main SHA
  -> release validates + downloads by artifact ID
  -> verify manifest and file hashes
  -> publish the same package/signature bytes
```

The final candidate assembly depends on every required validation job and both platform build jobs. Platform builds may run in parallel with validation to avoid serial CI latency, but their one-day temporary artifacts are not publishable. A failed overall CI run cannot produce the 30-day final candidate.

## 3. Local Surface Cleanup

### 3.1 Remove implicit hooks

- Delete `.githooks/pre-commit` and `.githooks/pre-push`.
- Delete `scripts/install-git-hooks.mjs`.
- Remove `hooks:install` and `postinstall` from `package.json`.
- For this clone, run a one-time Git config cleanup only when the stored value exactly matches `.githooks`. Use fixed-value removal so an unrelated user hook path is never erased.
- Add a static contract check that rejects reintroduction of tracked pre-commit/pre-push hooks, hook-installing lifecycle scripts, or `.githooks` as a configured repository convention.

Deleting the tracked hook files makes stale `.githooks` configuration harmless in other clones. The explicit cleanup removes ambiguity in the current clone.

### 3.2 Remove repository-approved native commands

Remove package aliases and helper scripts that invoke Rust/Tauri locally, including:

- `tauri`, `tauri:dev`, `tauri:fmt`, `tauri:check`, `tauri:clippy`, `tauri:test`;
- `tauri:build` and target-specific build aliases;
- `tauri:gen-types`, `check:generated-bindings`, `plugin:perf-smoke`;
- native portions of `check:precommit*`, `check:prepush`, and `check:plugin-hardening`.

Keep explicit frontend commands such as `dev`, `build`, `typecheck`, `lint`, Vitest and package-level TypeScript tests. Workflows invoke `cargo` and `pnpm exec tauri` directly so there is no misleading local package alias.

Direct user-entered system commands cannot be blocked by repository files. The durable boundary is therefore: no automatic trigger, no documented/repository-approved native command, and an explicit agent rule prohibiting local Rust/Tauri execution.

## 4. Cloud Validation Contract

### 4.1 Rust canonicalization owner

Move all native canonicalization into the existing Linux Rust job:

1. install Node/pnpm and Rust 1.90.0 on the same runner, then run `pnpm install --frozen-lockfile`;
2. run canonical `cargo fmt`, minimal workspace lock synchronization, the Specta export example, and Prettier on the generated binding; any command error fails immediately and is not reported as drift;
3. inspect a path-limited Git diff for Rust source, `Cargo.lock`, and generated bindings and expose `drift=true` only when that diff is non-empty;
4. when `drift=true`, write `git diff --binary` to `cloud-native-fixes.patch`, upload it in an independent always-eligible step with a SHA/run-attempt-specific name, then fail in a separate step before Clippy/tests;
5. if unchanged, continue with Clippy, full locked tests and Cargo audit.

The patch workflow is preferred over an auto-commit bot because it does not grant write credentials to code originating in a PR. Patch contents are bounded to known tracked paths and never include build output or secrets.

The frontend job removes Tauri/Linux system dependency setup and `check:generated-bindings`. This avoids a second cold Rust compilation on another runner.

### 4.2 CI concurrency

Use a unique group for every `main` run, for example a group containing `github.run_id`. PRs group by PR number and `dev` groups by ref with cancellation enabled. This avoids both running-job cancellation and GitHub's single-pending-run replacement behavior for a shared main group.

## 5. Release-Candidate Contract

### 5.1 Candidate planning

Extend `scripts/support-matrix.mjs` as the single owner for:

- synchronized application-version validation across `package.json`, `src-tauri/Cargo.toml`, the root package entry in `src-tauri/Cargo.lock`, and `src-tauri/tauri.conf.json`;
- release-candidate detection;
- official and manual cloud build matrices;
- stable asset preparation;
- candidate manifest creation and verification;
- static CI/Release contract checks.

A main push is a release candidate only when the current synchronized application version differs from `github.event.before`. Candidate planning checks out the event's full 40-hex SHA, explicitly fetches the recorded `before` object and `origin/main`, and fails closed when `before` is all zeroes, unavailable, moved out of the expected history, or not an ancestor. It compares synchronized versions from both immutable commits instead of reading a moving ref.

`workflow_dispatch` on the current `main` gains a required boolean `build_release_candidate` input whose default is `false`, plus `candidate_sha` and expected-tag inputs for cloud recovery. Only the literal boolean expression `inputs.build_release_candidate == true` enables recovery. `candidate_sha` must be a full 40-hex commit, the expected tag must equal the tag derived from all four synchronized source versions, and neither value is executed as a ref. The trusted control checkout remains the dispatch-captured full `github.sha` on current `main`, explicitly fetches `origin/main`, proves that neither has moved and that the candidate is in `main` history, and then uses a separate checkout directory to build the source commit. Arbitrary branch or tag workflow definitions never reach signing steps.

The historical exact-SHA CI prerequisite applies only to recovery: recovery must locate a successful prior main CI for the candidate SHA and bind its run ID/attempt. A first-time version-changing main push satisfies validation through the required quality jobs in that same run; its assembly cannot run until those jobs succeed. The manifest binds the trusted control SHA, candidate source SHA and source-validation run.

### 5.2 Platform builds

For trusted candidates, Windows x64 and macOS ARM64 jobs:

- declare the protected main-only `release-signing` Environment before any signing secret is exposed;
- use the pinned Rust toolchain and current support-matrix target/bundle values;
- install dependencies in GitHub Actions;
- validate updater signing secrets only inside the signing/build steps;
- invoke Tauri with a locked Cargo build;
- generate stable updater names and portable ZIPs exactly once;
- upload per-platform temporary artifacts named with run ID, run attempt and updater platform, retained for one day.

These jobs can run alongside frontend/Rust validation. The candidate assembly job depends on all required CI jobs and therefore cannot run when any validation or platform build fails.

Assembly accepts platform artifacts only from the current `github.run_id` and current `github.run_attempt`; it never combines attempts. A partial "Re-run failed jobs" is expected to fail closed if successful platform artifacts came from an earlier attempt. Operators must use "Re-run all jobs" or start a new current-main recovery run.

### 5.3 Immutable manifest

The assembly job uploads one final artifact named:

```text
release-candidate-<source-sha>-<run-id>-<run-attempt>
```

It is retained for 30 days and uploaded without overwrite. Its manifest is deterministic JSON with this logical schema:

```json
{
  "schemaVersion": 1,
  "repository": "KNaiFen/aio-coding-hub",
  "sourceSha": "<40-hex SHA>",
  "trustedControlSha": "<40-hex SHA>",
  "sourceValidationRunId": 123,
  "sourceValidationRunAttempt": 1,
  "version": "0.x.y",
  "tag": "aio-coding-hub-v0.x.y",
  "workflowRunId": 123,
  "workflowRunAttempt": 1,
  "targetIds": ["windows-x64", "macos-arm64"],
  "files": [
    {
      "name": "<stable filename>",
      "targetId": "windows-x64",
      "size": 123,
      "sha256": "<64-hex digest>"
    }
  ]
}
```

Files and targets are sorted. The verifier rejects unknown schema versions, path separators, duplicate/missing/unexpected names, unsafe files, non-canonical SHA/version/tag, wrong repository/run context, size mismatch or digest mismatch.

GitHub Actions v4 artifacts are immutable and can be downloaded across workflow runs with a token plus run ID. Release selection further uses the exact artifact ID, not only a name pattern. See GitHub's artifact contract: <https://docs.github.com/en/actions/tutorials/store-and-share-data>.

## 6. Release Promotion Contract

The tag workflow has no build job. Annotated and lightweight tags are recursively peeled to one commit after explicitly fetching the tag object and `origin/main`; missing objects, a moved tag, a non-commit result or a non-main ancestor fails closed. Its sequence is split across two jobs with no shared filesystem:

1. `resolve-and-verify` has only `actions: read` and `contents: read`; it resolves/peels the tag, proves main ancestry, enumerates candidates for the exact source SHA, selects either a successful exact-SHA main-push candidate or a successful current-main recovery candidate, downloads the exact artifact ID, and verifies the manifest/files including `trustedControlSha` and source-validation run;
2. `resolve-and-verify` outputs the exact candidate run ID, run attempt and artifact ID, but passes no downloaded bytes to another job;
3. `publish` has `actions: read` and `contents: write`; it re-downloads that same artifact ID into a clean runner and repeats the complete manifest, file-size and SHA-256 verification before any release mutation;
4. `publish` only then creates or reuses a draft whose tag and target commit exactly match and whose body contains the workflow's fixed ownership marker; it refuses an already published, mismatched or unmarked existing draft;
5. immediately before the first draft mutation and again before final publication, `publish` re-fetches and recursively peels the tag and verifies it still targets the immutable event commit;
6. derive a stable publication timestamp from the draft release's fixed creation time and generate `latest.json` plus `SHA256SUMS.txt`;
7. enumerate and delete all existing assets owned by that marked draft, then upload the complete current set;
8. re-enumerate remote assets and require the exact expected names, count, byte sizes and `sha256:` digests with no extras before publishing; when the API omits a digest, download and hash each remote asset or fail closed.

Candidate absence, expiry, ambiguity or any mismatch fails before draft creation. There is no fallback compilation. Recovery is an explicit workflow dispatch from current `main` with the exact source SHA/tag as data, never execution of a workflow definition from that tag, followed by rerunning the tag workflow.

Permissions are declared per job, not only as prose or workflow-wide defaults. `resolve-and-verify` cannot mutate repository contents; `publish` independently re-verifies the immutable artifact before receiving `contents: write`. The release workflow never receives updater signing secrets.

## 7. Manual Cloud Build Contract

Refactor `dev-build.yml` into a target-selectable cloud build:

- `workflow_dispatch` choice covers Windows x64/ARM64, macOS x64/ARM64/universal and Linux x64;
- checkout always uses the dispatch-captured full `github.sha`; the selected ref is display metadata only, and the concurrency key includes the selected target ID so different targets never cancel one another;
- build metadata comes from the same support matrix;
- the six-target matrix distinguishes Tauri targets from rustup targets: macOS universal passes `universal-apple-darwin` to Tauri but installs both `aarch64-apple-darwin` and `x86_64-apple-darwin`; after local Tauri aliases are removed, Tauri Action uses `tauriScript: "pnpm exec tauri"` and explicit locked Cargo arguments;
- the workflow creates an ephemeral Tauri config overlay with `bundle.createUpdaterArtifacts=false`, then passes it explicitly to `pnpm exec tauri build`; the tracked production config stays unchanged;
- artifacts are unsigned, clearly named as development builds and retained for seven days;
- no development artifact is accepted by Release.

The overlay is required because the production `tauri.conf.json` enables updater artifacts and would otherwise require signing keys even for a manual cloud development bundle. This replaces the former local source-build table. Native live reload has no cloud equivalent; `pnpm dev` remains the local frontend-only workflow.

## 8. Orphaned Release Automation

Remove the unused release-please surface:

- `.github/workflows/release-pr-sync-cargo-lock.yml`;
- `release-please-config.json` and `.release-please-manifest.json`;
- `scripts/check-release-pr-changelog.mjs` and package/check-stage references;
- support-matrix assertions for `RELEASE_PLEASE_TOKEN`.

The repository has no release-please workflow and no `RELEASE_PLEASE_TOKEN`; leaving this path creates a latent failed workflow rather than a recovery mechanism.

## 9. Security and Failure Model

- Create a `release-signing` GitHub Environment with a custom deployment policy limited to `main`; signing jobs must name this Environment. Tags never deploy to it.
- The Environment must also report `can_admins_bypass: false`. GitHub's public Environment REST update does not expose that toggle, so it is a required one-time web-setting gate before any Environment secret is written or any repository secret is deleted.
- Protect `main` with required PR/CI and no force-push/deletion. Protect `aio-coding-hub-v*` with two separate tag Rulesets because bypass applies to an entire Ruleset: the creation Ruleset blocks creation except for the explicitly authorized maintainer bypass, while the update/deletion Ruleset has no bypass at all. Combining those rules would incorrectly let a creation-authorized maintainer rewrite or delete an existing release tag. Environment tag-name matching alone is not an ancestry or authorization boundary.
- Migrate the two existing repository secrets without recovering plaintext: fetch the Environment public key, run one audited main-only migration step that seals each existing secret to that key, download only ciphertext/key-id metadata, and use the already authenticated local `gh` token to call the Environment secret API.
- The migration job must use a pinned, reviewed LibSodium-compatible sealed-box implementation, mask inputs, write only fixed-schema ciphertext files, use one-day retention, never print secret-derived decoder errors, and be removed after a gated cloud signature verification succeeds.
- Migration is atomic with respect to the legacy scope: both ciphertexts must be created and both Environment secret writes must succeed before the probe begins. Any seal, upload or probe failure leaves both repository-level secrets intact. Delete both repository-level copies only after both Environment secret names exist and the cloud-only sign/verify probe succeeds; then re-list repository secrets to prove the old scope is empty and remove the temporary workflow/helper.
- Signing jobs run only from the canonical repository's trusted current `main`; recovery treats an older source SHA/tag strictly as checked data. Never use `pull_request_target`, a tag workflow definition for signing, or a privilege-escalating `workflow_run` bridge.
- Build jobs get `contents: read`. Only Release publication gets `contents: write`.
- Artifact uploads are restricted to known output directories; workspace-wide globs are forbidden.
- The current repository has no branch/tag ruleset or Environment. Main/tag Ruleset creation and encrypted signing-secret migration are explicit user decisions before activation; implementation must not silently leave repository-level signing secrets exposed while claiming the new model is secure.
- A failed native classifier, artifact lookup or manifest parse fails closed for release but cannot affect AIO runtime behavior.

## 10. Rollback

- Workflow changes are independently revertible because they do not migrate application data.
- If a candidate expires, rerun the explicit cloud candidate build for the exact current main release commit; do not restore release-time compilation.
- If a platform build is temporarily unavailable, Release remains blocked rather than publishing a partial or rebuilt set.
- Removing local hooks needs no runtime rollback. A developer may configure personal hooks outside repository control, but the project does not reinstall or depend on them.
