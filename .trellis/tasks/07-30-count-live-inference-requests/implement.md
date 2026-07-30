# Implementation Plan: Count live inference requests

## 1. Derived count

- Rename `countActiveInferenceSessions` to
  `countActiveInferenceRequests`.
- Return the number of snapshot entries matching
  `isActiveInferenceRequest`.
- Update the single production import/call site.

## 2. Home copy and accessibility

- Replace the session-deduplication tooltip with:
  `按活跃模型推理请求统计；同一会话与子代理的每个请求均计 1`.
- Preserve the short dynamic `aria-label` and associate the detailed rule with
  the displayed concurrency element.

## 3. Tests

- Add the exact 3 parent + 10 subagent = 13 scenario.
- Remove two subagent entries and assert 11.
- Assert same-Session parallel requests count independently.
- Preserve endpoint-classification coverage and auxiliary-request exclusions.
- Update Home UI tests for 13 → 11, tooltip copy, `0`, `--`, and hidden state.

## 4. Specification

- Add Home realtime concurrency semantics to the request-log/usage-ledger
  cross-layer contract.

## 5. Local verification

- Run targeted Vitest suites.
- Run `pnpm typecheck`.
- Run `pnpm lint`.
- Run `pnpm build`.
- Do not run Cargo, rustfmt, Clippy, Tauri binding generation, or any command
  that compiles Rust locally.

## 6. Release workflow

- Commit the feature.
- Commit the independent `0.60.36 -> 0.60.37` version bump.
- Push the branch and create a PR to `main` on `origin`.
- Require all PR CI checks to pass.
- Merge with a normal merge commit.
- Require exact-merge `main` CI to pass.
- Tag `aio-coding-hub-v0.60.37` and require Release workflow success.
