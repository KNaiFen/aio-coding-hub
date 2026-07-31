# CI and Release Artifact Audit

Date: 2026-07-31

Scope: `origin` repository workflows, recent Actions evidence, artifact retention and release-promotion feasibility. No workflow or release was mutated.

## Current workflow facts

- `ci.yml` runs on PR and `dev`/`main` push. It performs frontend build, Rust Clippy/tests/audit and generated bindings, but uploads no artifact.
- Generated bindings run in the frontend job and compile Rust for about four minutes; the Rust job separately compiles the crate again.
- `main` shares one concurrency group with `cancel-in-progress: true`, so a later main push can cancel an exact release candidate.
- `release.yml` validates that the tag SHA has a successful main CI, then separately installs Node/Rust/dependencies and builds/signs Windows x64 and macOS ARM64.
- Release platform artifacts are temporary and retained for one day.
- `support-matrix.mjs` currently asserts that Release owns the build and that temporary release artifacts use one-day retention.
- `release-pr-sync-cargo-lock.yml` expects `RELEASE_PLEASE_TOKEN`, but the repository exposes only the two Tauri signing secrets and has no release-please workflow.
- The tracked Tauri config enables updater artifact creation. Unsigned manual cloud bundles therefore need an explicit ephemeral config overlay; deleting the existing local wrapper without this replacement would make them fail on missing signing keys.
- Release currently overwrites same-named draft assets but does not prove that no old extra assets remain. A re-run can therefore publish a mixed asset set unless the verified draft is cleared and inventoried.

## Measured duplicate work

For source SHA `ebecb287535092d308aec3b887c09d45e8e95fc2`:

- Main CI run `30617682335`: 2026-07-31 08:49:46Z to 09:08:11Z, success, zero artifacts.
- Generated IPC binding step: roughly 4 minutes 10 seconds on the frontend runner.
- Rust job: roughly 18 minutes, including about 16 minutes of Rust tests.
- Release run `30619327949`: 09:16:29Z to 09:44:21Z, success.
- Windows Tauri build step: roughly 23 minutes 47 seconds.
- macOS Tauri build step: roughly 19 minutes 23 seconds.
- Release uploaded two temporary artifacts, about 33.5 MB and 32.2 MB, expiring after one day.

The Linux Rust test binary cannot be reused as a Windows/macOS desktop package because the targets differ. The removable duplication is release-time target compilation: build each official target once on the exact main SHA, retain it, and promote it later.

## Repository settings

- Actions are enabled; default workflow permissions are read-only.
- Artifact/log default retention is one day; maximum allowed retention is 90 days, so a per-artifact 30-day candidate is valid.
- The repository currently has no branch or tag ruleset. This is a signing-supply-chain risk and requires an explicit user decision before implementation changes repository governance.
- The public repository currently has no GitHub Environment. Both Tauri signing values are repository-level secrets, so a same-repository branch can potentially modify and manually dispatch a workflow that references them; a YAML `if` gate alone is not a secret boundary.

## Promotion feasibility

GitHub documents that artifacts can be shared between jobs and workflow runs. Cross-run `actions/download-artifact@v4` requires a GitHub token, repository and run ID; v4 artifacts are immutable. The pinned action also accepts exact `artifact-ids`.

GitHub's Environment secret API separately exposes an Environment public key and accepts only a LibSodium-encrypted value plus key ID. That permits a one-time runner to transform the existing repository secret directly into target-key ciphertext without returning plaintext to the user or local machine. The local authenticated API call handles only ciphertext; the temporary bridge is removed after cloud verification.

Primary references:

- <https://docs.github.com/en/actions/tutorials/store-and-share-data>
- <https://docs.github.com/en/actions/concepts/workflows-and-actions/workflow-artifacts>
- <https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/control-workflow-concurrency>
- <https://raw.githubusercontent.com/actions/download-artifact/d3f86a106a0bac45b974a628896c90dbdf5c8093/action.yml>
- <https://docs.github.com/en/rest/actions/secrets?apiVersion=2022-11-28>
- <https://docs.github.com/en/code-security/reference/secret-security/secret-types>

## Chosen architecture

- Build signed platform candidates in trusted main CI, in parallel with validation.
- Keep the signing Environment main-only. Recovery executes the current main workflow and treats an older candidate SHA/tag only as ancestry- and CI-verified input data.
- Keep platform outputs temporary; assemble the final 30-day candidate only after all required validation and build jobs succeed.
- Bind the final manifest to SHA, version, run ID and run attempt.
- Locate a successful exact-SHA main CI from Release, choose its exact artifact ID, verify all bytes, then create/publish the draft.
- Fail closed when the candidate is absent/expired/ambiguous; recovery is an explicit cloud candidate run, never a Release fallback build.
- After Release upload, compare each remote asset's `sha256:` digest with the local manifest; download-and-hash or fail when the API cannot provide one.
