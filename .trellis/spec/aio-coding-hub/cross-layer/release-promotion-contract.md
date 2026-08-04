# Release Promotion Contract

### 1. Scope

Apply this contract when release tag validation, source resolution, candidate
selection, or publication changes. The release workflow promotes artifacts that
were already built and validated by `main` CI; it must not rebuild them.

### 2. Remote Tag Identity

- Accept only the canonical `aio-coding-hub-vMAJOR.MINOR.PATCH` tag format.
- Resolve the exact `refs/tags/<tag>` value from `origin`, not from checkout
  state. Fetch it without a destination ref so the result is written only to
  `FETCH_HEAD` and no local tag is created, replaced, or force-updated.
- Peel `FETCH_HEAD^{commit}` immediately after the tag fetch. Any later fetch
  overwrites `FETCH_HEAD`, so resolving after the `main` fetch is invalid.
- Reject a missing tag, a tag that does not peel to a commit, or a source commit
  that is not an ancestor of `origin/main`.
- Checkout the resolved commit in detached mode and validate that its manifests
  match the requested release version.

### 3. Candidate Promotion

- Select only a successful `push` or `workflow_dispatch` CI run from this
  repository's `main` branch whose `head_sha` exactly equals the resolved release
  source.
- Across all eligible CI runs, require exactly one unexpired artifact named
  `release-candidate-<sha>-<run-id>-<attempt>`; multiple candidates are
  ambiguous and must fail closed.
- Verify every expected desktop, TUI, signature, checksum, and updater file
  before publication. Require the candidate checksum manifest to cover every
  uploaded asset before either a first publication or a retry. Publication
  reuses those bytes without rebuilding.
- Before publishing a tag that already has a Release, require the candidate's
  complete asset-name set and `SHA256SUMS.txt` mapping to match exactly. A
  match is an idempotent no-op; any missing, extra, or different asset fails
  closed. Never overwrite existing Release assets.

### 4. Trigger Parity

- Annotated-tag pushes and manual `workflow_dispatch(tag)` use the same remote
  tag and source validation path.
- Annotated-tag pushes and manual `workflow_dispatch(tag)` serialize against
  the same final tag; different tags may publish independently.
- A tag-trigger checkout may already contain a same-name lightweight local tag.
  That state must not alter the resolved remote commit or cause ref clobbering.
- Validation failures stop before candidate download and publication; the manual
  entry point is recovery orchestration, not a weaker validation path.

### 5. Required Tests

- Reproduce an annotated remote tag plus same-name lightweight local tag and
  prove the legacy destination refspec fails while `FETCH_HEAD` resolution
  succeeds without changing the local tag.
- Cover manual resolution with no local tag and prove no tag ref is created.
- Reject invalid and missing tags plus a validly named tag outside `main` history.
- Keep a static workflow contract that locks tag fetch, immediate peel, then
  `main` fetch ordering.
