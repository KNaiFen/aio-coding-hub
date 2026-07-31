# Cloud Build And Release Artifact Contract

## Scenario: Change Native Validation, Desktop Builds, Or Release Promotion

### 1. Scope / Trigger

Use this contract when changing repository hooks, package scripts, Rust or
generated-binding validation, desktop build targets, candidate artifacts,
signing, release selection, or release asset upload.

The ownership flow is:

```text
local source edit
  -> Node/TypeScript/frontend checks only
  -> pull request or main CI
       -> native canonicalization + drift patch
       -> Rust validation + audit
       -> trusted signed candidate builds when eligible
       -> immutable final candidate + manifest
  -> release tag
       -> select exact successful candidate artifact ID
       -> verify the manifest and every byte twice
       -> publish without compiling
```

The principal owners are `.github/workflows/ci.yml` for canonical validation
and release candidates, `.github/workflows/dev-build.yml` for unsigned manual
development artifacts, `.github/workflows/release.yml` for promotion, and
`scripts/support-matrix.mjs` for the shared target/manifest/static workflow
contract.

### 2. Local Execution Boundary

- The repository must not track pre-commit, pre-push, Husky, or equivalent
  hooks, install a hook path from an install lifecycle script, or depend on a
  repository hook to make commits safe.
- Root and workspace package scripts may run only Node.js, TypeScript,
  frontend tests, frontend builds, and static contract readers locally. They
  must not expose direct or aggregate aliases for Cargo, rustfmt, Clippy, Rust
  tests, Specta generation, or the Tauri CLI.
- `pnpm install`, ordinary commit/push operations, and Trellis lifecycle work
  must not compile application-native/Rust/Tauri code or write generated
  Rust-owned files. The one canonical root pnpm policy may enable only the
  pinned frontend `esbuild` dependency install step; package-level or `.npmrc`
  overrides fail closed.
- `pnpm dev` is the local frontend-only Vite server. Native integration and
  desktop packages come from cloud artifacts; interactive native hot reload
  has no repository-supported local replacement.
- `scripts/check-local-native-boundary.mjs` fails closed on tracked hooks and
  equivalent hook-manager configuration, hook installation, native package
  aliases, native aggregate stages, active Trellis lifecycle hooks, executable
  pnpmfiles, and repository-controlled editor/Make/Just/Task automation files.
  Allowlisted JavaScript helpers use explicit process contracts; namespace,
  alias, reflective, shell-enabled, or otherwise unauditable dispatch fails.
  CI and package aggregates scan the real repository before running boundary
  self-tests. The checker only parses files and Git metadata; it never executes
  an audited helper.
- A system-installed native tool remains outside repository control. The
  enforceable boundary is the absence of repository-managed triggers,
  aliases, and instructions.

### 3. Cloud Canonicalization And Drift Patches

- The Linux Rust CI job is the sole owner of Rust formatting, minimal lockfile
  synchronization, Specta export, and formatting of generated TypeScript
  bindings. The frontend job does not repeat this native compilation.
- A canonicalization command error is a tool failure, not drift. CI fails
  immediately and must not upload a partial repair artifact.
- After all canonicalization commands succeed, CI inspects only the approved
  Rust source, `Cargo.lock`, and generated-binding paths. A non-empty bounded
  binary diff becomes `cloud-native-fixes.patch`.
- Patch upload and final failure are separate steps: upload remains eligible
  when drift exists, then a later step fails the job before Clippy/tests. The
  patch contains no build output, workspace-external file, or secret.
- Developers apply the emitted patch and push again. They do not regenerate
  or normalize the native files locally.
- Clippy, locked Rust tests, and Cargo audit run only after the canonical drift
  gate is clean.

### 4. Trusted Release Candidates

- A normal candidate is an exact full SHA from a version-changing `main`
  push. A recovery candidate is explicit data supplied to the current trusted
  `main` workflow and must pass full-SHA, expected-tag, ancestry, current-main,
  and prior exact-SHA successful-CI checks.
- Candidate platform builds are Windows x64 and macOS ARM64 only. They may run
  in parallel with validation, but final assembly depends on every required
  validation and platform job succeeding in the same workflow attempt.
- Signing jobs declare the protected, main-only `release-signing` Environment.
  Pull requests, feature branches, tags, and the manual development workflow
  never receive updater signing secrets.
- Temporary platform artifacts are attempt-specific and retained for one day.
  Assembly accepts only artifacts from the current run ID and attempt; partial
  job reruns fail closed instead of mixing attempts.
- The final candidate is immutable, retained for 30 days, and named with its
  source SHA, workflow run ID, and run attempt. It is the only artifact class
  eligible for formal Release promotion.
- `main` runs use unique concurrency ownership so a later push cannot cancel a
  still-valid earlier candidate run. Superseded pull request and `dev` runs may
  still be canceled.

### 5. Candidate Manifest

The candidate includes one deterministic schema-versioned manifest binding:

- canonical repository identity;
- source SHA and trusted control SHA;
- source-validation run ID and attempt;
- synchronized application version and derived tag;
- candidate workflow run ID and attempt;
- the exact sorted target ID set; and
- every file's safe basename, target ID, byte size, and SHA-256 digest.

The verifier rejects unknown schemas, non-canonical SHAs/version/tags, path
separators, unsafe or duplicate names, missing/extra targets or files,
cross-attempt provenance, and size/digest mismatches. Unknown fields do not
weaken a known schema; a future format receives a new explicit version.

### 6. Manual Cloud Development Builds

- `workflow_dispatch` exposes the six established targets: Windows x64/ARM64,
  macOS x64/ARM64/universal, and Linux x64.
- Checkout uses the dispatch-captured full SHA. The selected ref is display
  metadata only, and concurrency includes the target ID.
- Every build passes an ephemeral Tauri configuration overlay that disables
  updater artifact creation. The tracked production configuration is not
  changed.
- Development artifacts are clearly labeled, unsigned, retained for seven
  days, and ineligible for candidate assembly or Release.

### 7. Release Promotion

- The tag workflow contains no Rust toolchain setup, dependency installation,
  native build matrix, Tauri invocation, signing secret, or fallback compile.
- Tag resolution recursively peels the fetched tag to one immutable commit and
  proves `main` ancestry. Candidate selection uses the exact successful
  workflow run and immutable artifact ID, not a moving name alone.
- A read-only resolution job downloads and completely verifies the candidate.
  A separate publish job independently downloads and verifies the same
  artifact ID before receiving content-write authority.
- Candidate absence, expiry, ambiguity, or any manifest/file mismatch fails
  before draft creation. Recovery means producing another cloud candidate,
  never compiling in Release.
- Reusing a workflow-owned draft requires exact draft/tag/commit/ownership
  validation. Existing assets are cleared, the complete new set is uploaded,
  and remote names, counts, sizes, and SHA-256 digests must match exactly
  before publication. Missing remote digest data requires download-and-hash or
  failure.
- Stable release metadata derives from a fixed draft creation timestamp so a
  rerun does not create byte drift in updater metadata or checksums.

### 8. Validation Matrix

| Condition | Required result |
| --- | --- |
| Package install, commit, push, or Trellis archive | No native process and no generated-file write |
| Tracked hook, native package alias, or active local automation reappears | Local boundary check fails |
| Canonicalization command errors | CI fails; upload no repair patch |
| Canonicalization succeeds and approved files drift | Upload bounded patch, then fail before later native gates |
| Ordinary PR or non-version `main` run | Run validation; expose no signing environment |
| Eligible exact `main` candidate | Build both signed targets and assemble only after all required jobs pass |
| Partial rerun lacks current-attempt platform bytes | Fail assembly; never mix attempts |
| Manual development target | Produce unsigned seven-day artifact with updater output disabled |
| Candidate is missing, expired, duplicate, or context-mismatched | Release fails before draft mutation |
| Candidate file size or digest mismatches | Both read and publish jobs fail closed |
| Managed draft contains stale/extra asset | Clear it and require an exact final inventory before publish |

### 9. Tests Required

- Keep the local-boundary pure-function self-test exhaustive for root/workspace
  scripts, dependency-build policy drift, indirect/reflective process aliases,
  hooks and equivalent hook-manager configuration, Trellis hooks, nested
  editor/Make/Just/Task automation, and allowlisted Node/frontend helpers.
- Keep support-matrix self-tests for all six manual targets, the two formal
  targets, synchronized versions, manifest success, and every schema/path/
  provenance/size/digest negative.
- Keep static workflow tests proving CI owns canonicalization/candidates and
  Release contains promotion only.
- Local verification is Node/frontend-only: boundary self-test, spec links,
  typecheck, lint, focused frontend tests, the frontend build, and diff checks.
- Native verification, signing, cross-platform packaging, and promotion-byte
  equality require GitHub Actions evidence. Record run IDs, attempts, artifact
  IDs/expiry, and hashes rather than claiming them from a local machine.
