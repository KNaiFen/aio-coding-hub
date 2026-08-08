# AIO Coding Hub Backend Specs

Rules for the root application's Rust backend and local gateway runtime.

## Topics

- [Gateway attempt budget contract](./gateway-attempt-budget-contract.md):
  per-request provider attempts, reserved internal retries, strict model
  discovery, and cross-request circuit-breaker accounting.
- [Codex request content-encoding contract](./codex-request-content-encoding-contract.md):
  bounded decoding at the gateway boundary, supported HTTP encodings, identity
  forwarding, and local failure classification.
- [Codex context compaction observation contract](./codex-context-compaction-observation-contract.md):
  original-request signatures, fail-open classification, bounded markers, and
  strict separation from routing and HTTP request encoding.
- [Codex managed model route contract](../cross-layer/codex-managed-model-route-contract.md):
  readable profile aliases plus legacy UUID lookup, complete picker catalog
  lifecycle, one-provider routing, same-provider retry, and terminal
  wire-vs-observed route evidence.
- [Local observer and TUI contract](../cross-layer/local-observer-tui-contract.md):
  loopback-only authenticated snapshots and non-blocking read-only monitoring
  boundaries.
- [Upstream error response rule contract](./upstream-error-response-rule-contract.md):
  final-error matching, fail-open protocol rewriting, and separation from
  provider routing and health facts.

## Pre-Development Checklist

When changing gateway retry or circuit behavior:

1. Read [Gateway attempt budget contract](./gateway-attempt-budget-contract.md).
2. Identify whether each counter is request-scoped or persisted across requests.
3. Trace the effective provider retry policy, including provider overrides.
4. Keep strict helper routes explicit instead of relying on shared retry math.

When changing managed Codex alias routing or model-route detection:

1. Read [Codex managed model route contract](../cross-layer/codex-managed-model-route-contract.md).
2. Keep the managed provider as the only candidate while preserving common
   gates and same-provider retry.
3. Prove later terminal matched/unobserved evidence cannot leave a stale severe
   mapping from an earlier attempt.

When changing Codex request-body encoding:

1. Read [Codex request content-encoding contract](./codex-request-content-encoding-contract.md).
2. Keep semantic context compaction separate from HTTP request-body encoding.
3. Bound every decoded layer and preserve non-Codex transport behavior.
4. Keep decoding failures before provider selection and circuit accounting.

When changing Codex context-compaction observation:

1. Read [Codex context compaction observation contract](./codex-context-compaction-observation-contract.md).
2. Capture only bounded evidence from the original decoded request before
   plugins.
3. Keep every classifier outcome observational; malformed or future input must
   continue forwarding without a marker.
4. Test local, remote v1, remote v2, unknown, and conflicting metadata.

When changing the local observer runtime:

1. Read [Local observer and TUI contract](../cross-layer/local-observer-tui-contract.md).
2. Keep observer startup and snapshot reads best-effort; never await them on a
   request-forwarding path or hold the gateway database pool for observation.
3. Keep circuit inspection non-mutating and all projections bounded and
   secret-free.
4. Bound trace-ID query inputs without silently truncating the active registry;
   global concurrency remains registry-wide, while an over-limit database
   projection must degrade explicitly instead of changing active semantics.

When changing plugin runtime quarantine:

1. Persist the lifecycle transition before exposing quarantine to callers.
2. Remove the quarantined plugin from the live gateway snapshot, runtime cache,
   and hook circuits before attempting a full database-backed refresh. A failed
   refresh must retain the already-pruned snapshot.
3. Dispose the extension host independently of full snapshot refresh success;
   in-flight calls may finish under their existing deadline, but no later
   snapshot lookup may start the quarantined plugin.

When changing upstream error response rules:

1. Read [Upstream error response rule contract](./upstream-error-response-rule-contract.md).
2. Keep retry, failover, quota, cooldown, and circuit decisions on original
   upstream facts.
3. Apply a rewrite only at a terminal HTTP error response and fail open when
   bounded evidence is unavailable.
4. Keep attempt logs original and the request-level status client-visible.

When changing recovery-journal-backed filesystem projection:

1. Reuse a caller-held SQLite connection for journal lease, checkpoint, and
   replay-context mutations; do not borrow a second pool connection while the
   first is held.
2. Keep a fresh pool borrow only at orchestration boundaries where no active
   connection is held.
3. When a `local://` Skill source must restore a missing SSOT, first copy it
   into a journal-owned `desired` artifact, then bind that artifact hash to
   `installed_content_hash` in the same authoritative SQLite commit. Replay
   only from the validated artifact; an existing unsafe SSOT entry or a hash
   mismatch must fail closed.
4. Skill uninstall may omit a `previous` artifact only when the SSOT directory
   entry is genuinely absent. Symlinks, junctions, files, and other unsafe
   entries remain errors; the committed row deletion then drives managed-link
   cleanup through the ordinary authoritative projection.
5. A workspace-local recovery stash is empty only when its file is absent.
   Unreadable, oversized, non-file, invalid UTF-8, malformed JSON/TOML, and
   invalid-schema stashes are `RECOVERY_ARTIFACT_INVALID`; validate the target
   stash before any managed target write and keep the journal unresolved on
   failure.

When changing Provider Sync managed-backup pruning:

1. Classify, isolate, enumerate, validate, and delete from already-opened
   trusted root/child handles with no-follow semantics; a path, directory name,
   timestamp, or file presence is never ownership proof.
2. Share one bounded prune budget across root enumeration, manifest parsing,
   snapshot validation, content hashing, and deletion revalidation. Exhaustion,
   identity/content change, unsupported platform behavior, or malformed input
   must fail closed and preserve the candidate or isolated data.
3. Keep per-tree limits separate from the aggregate operation budget. Before
   the first irreversible isolation, reserve the full worst-case work for all
   remaining whole-tree validations and per-file deletion hashes; budget
   exhaustion must leave the candidate at its original path.
4. Bind ordinary files with a bounded streaming content digest in addition to
   identity/metadata, and keep detailed warnings bounded with a suppressed-count
   summary.
5. Treat double tombstones, handle-relative operations, content digests, Windows
   ChangeTime, and final revalidation as race-window reduction only. POSIX and
   Windows do not provide a portable "delete exactly this previously verified
   identity" guarantee against a malicious concurrent writer with the same UID
   or equivalent permissions. Do not document or test these measures as fully
   eliminating that final syscall boundary.

## Quality Check

- Unit-test the attempt-budget calculation at its boundary values.
- GitHub Actions must run route-level tests that exercise real provider retries
  and failover.
- Verify circuit failure counts across multiple requests.
- GitHub Actions must run the full Rust suite after changing shared
  failover-loop inputs.
- Route-test managed and ordinary Codex requests together after changing
  provider selection, final wire-model tracking, or response observation.
- Verify supported Codex encodings arrive upstream as identity JSON, while
  invalid or oversized encoded bodies make zero upstream attempts.
- Prove context-compaction classification cannot change request bytes, headers,
  timeouts, provider selection, retries, circuit state, or response status.
- Prove error response rules cannot change provider selection, retry counts,
  circuit state, or attempt evidence, including a failure followed by success.
- Exercise recovery projection in the one-connection test pool so journal
  bookkeeping cannot silently depend on spare pool capacity.
- Exercise invalid workspace stash preflight before managed MCP writes, then
  repair the stash and prove the same unresolved journal replays to completion.
- Exercise command, startup, and gateway quarantine while a full plugin refresh
  fails; the target plugin must remain absent and disposed while unrelated
  plugin snapshot/circuit state survives.
- Exercise missing-SSOT recovery from a local Skill source, persisted-hash
  mismatch rejection, and uninstall cleanup of broken managed links in GitHub
  Actions.
- Exercise Provider Sync root/child replacement, symlink/reparse rejection,
  equal-length in-place rewrites, shared-budget exhaustion, warning caps, and
  the Unix/Windows final-delete boundary in GitHub Actions. Observable changes
  must preserve data and report a bounded warning.
