# AIO GKD CI 与 Release Adapter Delivery

## Result

- 实现头：`57d083fa0e3a5cca94c9093f76d9314d43b03fa2`。
- 新增 AIO 专有 `.gkd/ci-release-adapter.json` 与零依赖 checker/selftest，绑定已取证的 speed-first recommendation、unknown capacity/billing、Air-safe 本地 micro boundary、并行 cloud job groups、required checks、redacted leak scan、bounded artifacts/cache 和 same-source-SHA candidate/finalization 规则。
- `ci.yml` contracts job 接入 checker；frontend/rust 保留原 scope gate 并解除不必要的 contracts 串行依赖，candidate/assemble 仍由 contracts、frontend、rust 和 candidate-plan 的明确结果 fail-closed 聚合。
- 未创建 tag、Release、deployment、artifact 发布、GitHub settings/Secrets/paid runner 变更，也未修改 Trellis history 或普通产品代码。

## Acceptance Criteria

| Criterion | Result | Evidence |
|---|---|---|
| Canonical CI/release declaration and live recommendation binding | Pass | `node scripts/check-gkd-ci-release.selftest.mjs` and `node scripts/check-gkd-ci-release.mjs` return `ci_release_ready`; recommendation digest `8ec8e8b4bb28490ea4fa238828c5d012b9ec7f48a951b51189d084a9a0a2104e`. |
| Required checks, independent jobs, Air-safe commands, bounded artifacts/cache and fail-closed gate | Pass | Checker proves `ci-gate`/`pr-title`, cloud job declarations, `ci-gate` `always()` aggregation, source-SHA artifact template, and retention bounds; adapter/local/cloud selftests pass. |
| Redacted leak scan | Pass | Selftest rejects credential-shaped and machine-local fixture values while retaining only stable code and relative path. |
| Same-SHA release candidate and two-PR finalization contract | Pass | Checker proves `run.head_sha === SOURCE_SHA`, unique `selectReleaseCandidate`, `SHA256SUMS.txt`, immutable asset guard and declaration `candidatePr`/`finalizationPr`/`sameSourceSha`; no GitHub write is performed. |
| Local verification | Blocked by pre-existing lifecycle document whitespace | `scripts/gkd-verify --base-sha a133a79c819ff875cfffca40967700679b4fc383` reaches the registered verifier but fails its committed diff check on the already-fixed task documents' trailing blank lines: `requirements.md:39`, `plan.md:73`, `implementation.md:17`. These files are lifecycle-digest-bound and were not modified by this executor. |

## Verification

| Type | Command/check | Result |
|---|---|---|
| Adapter | `node scripts/check-gkd-adapter.selftest.mjs && node scripts/check-gkd-adapter.mjs` | Pass; adapter digest `eac007446f5ce616aad866185b66da59a1fc5c74b32de21c0dffe117ed0443b6`. |
| CI/release adapter | `node scripts/check-gkd-ci-release.selftest.mjs && node scripts/check-gkd-ci-release.mjs` | Pass; leak findings empty. |
| Local/cloud contracts | `node scripts/check-local-verification.selftest.mjs`, cloud-only selftest and contract | Pass. |
| Syntax/diff | Changed Node `--check`; `git diff --check` for implementation surface | Pass. |
| Trellis boundary | Base-to-head `.trellis/tasks` path diff | No implementation changes. |
| Required GitHub checks | `ci-gate`, `pr-title` | Pending fixed-head CI monitor. |

## Candidate Output Bundle

- Deterministic Git source archive of implementation head `57d083fa0e3a5cca94c9093f76d9314d43b03fa2` SHA-256: `b2a82d0a67d30f5c9d4cf9aa7ff7826edecdcb4a2b4af8cda2ed3744927b311d`.

## Scope And Risk

- Dependency installation, frontend/Rust/Tauri tests, formatting, lint, typecheck, build, signing, packaging, release publication and deployment remain cloud-owned or explicitly out of scope.
- Remaining blocker is limited to the lifecycle task documents created before implementation; changing them would invalidate the registered task document digests and requires trusted rework, not executor-side editing.
