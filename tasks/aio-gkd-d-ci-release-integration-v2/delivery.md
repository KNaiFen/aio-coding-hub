# AIO GKD CI 与 Release Adapter Delivery

## Result

- 实现头：`fba3171e214633fe5caa02450bcef047cf0a9a8c`。
- 新增 AIO 专有 `.gkd/ci-release-adapter.json` 与零依赖 checker/selftest，绑定 speed-first recommendation、unknown capacity/billing、Air-safe 本地 micro boundary、并行 cloud job groups、required checks、redacted leak scan、bounded artifacts/cache 和 same-source-SHA candidate/finalization 规则。
- `ci.yml` contracts job 接入 checker；frontend/rust 保留原 scope gate 并解除不必要的 contracts 串行依赖，candidate/assemble 仍由明确结果 fail-closed 聚合。
- 未创建 tag、Release、deployment、artifact 发布、GitHub settings/Secrets/paid runner 变更，也未修改 Trellis history 或普通产品代码。

## Acceptance Criteria

| Criterion | Result | Evidence |
|---|---|---|
| Canonical CI/release declaration and live recommendation binding | Pass | `node scripts/check-gkd-ci-release.selftest.mjs` and `node scripts/check-gkd-ci-release.mjs` return `ci_release_ready`; recommendation digest `8ec8e8b4bb28490ea4fa238828c5d012b9ec7f48a951b51189d084a9a0a2104e`. |
| Required checks, independent jobs, Air-safe commands, bounded artifacts/cache and fail-closed gate | Pass | Checker proves `ci-gate`/`pr-title`, cloud job declarations, `ci-gate` `always()` aggregation, source-SHA artifact template, and retention bounds; adapter/local/cloud selftests pass. |
| Redacted leak scan | Pass | Selftest rejects credential-shaped and machine-local fixture values while retaining only stable code and relative path. |
| Same-SHA release candidate and two-PR finalization contract | Pass | Checker proves `run.head_sha === SOURCE_SHA`, unique `selectReleaseCandidate`, `SHA256SUMS.txt`, immutable asset guard and declaration `candidatePr`/`finalizationPr`/`sameSourceSha`; no GitHub write is performed. |
| Local verification | Pass | `scripts/gkd-verify --base-sha a133a79c819ff875cfffca40967700679b4fc383` returns `local_ready` at implementation head. |

## Verification

| Type | Command/check | Result |
|---|---|---|
| Adapter | `node scripts/check-gkd-adapter.selftest.mjs && node scripts/check-gkd-adapter.mjs` | Pass; adapter digest `eac007446f5ce616aad866185b66da59a1fc5c74b32de21c0dffe117ed0443b6`. |
| CI/release adapter | `node scripts/check-gkd-ci-release.selftest.mjs && node scripts/check-gkd-ci-release.mjs` | Pass; adapter digest `acffae998164a02d4b949f9f856ea9646eaee2b64e69a262e84977188a1ec444`; leak findings empty. |
| Local/cloud contracts | Registered verifier | Pass; cloud-owned dependencies, frontend, Rust, build, packaging, signing and deployment remain unrun. |
| Syntax/diff | Changed Node `--check`; `git diff --check` | Pass. |
| Trellis boundary | Base-to-head `.trellis/tasks` path diff | No implementation changes. |
| Required GitHub checks | `ci-gate`, `pr-title` | Pending fixed-head CI monitor. |

## Candidate Output Bundle

- Deterministic Git source archive of implementation head `fba3171e214633fe5caa02450bcef047cf0a9a8c` SHA-256: `001bf33bc8f9667fca0dc4a6f8804f27c5460500d1c41ee92b6dcbc51d348a3b`.

## Scope And Risk

- Dependency installation, frontend/Rust/Tauri tests, formatting, lint, typecheck, build, signing, packaging, release publication and deployment remain cloud-owned or explicitly out of scope.
- Trusted main must perform fixed-head CI monitoring, acceptance, merge, records-only closeout and cleanup; this executor stops after delivery.
