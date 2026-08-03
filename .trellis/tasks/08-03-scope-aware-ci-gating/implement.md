# 按变更范围分级运行 CI - Implementation Plan

## Phase 1: Classification Contract

- [x] Add the centralized allowlist policy.
- [x] Implement path/diff classification with fail-closed behavior.
- [x] Add deterministic self-tests for tiers and unsafe edge cases.
- [x] Record the durable cross-layer CI contract.

## Phase 2: Workflow Integration

- [x] Add the always-running change-scope job and outputs.
- [x] Add the targeted documentation-contract job.
- [x] Gate existing full suites and release planning on `full_ci`.
- [x] Update `ci-gate` to validate selected and skipped paths explicitly.

## Phase 3: Verification And Delivery

- [x] Run local Node.js, documentation, format, diff, and Trellis checks.
- [x] Review the complete diff for fail-open paths and required-check deadlocks.
- [x] Push the implementation PR and wait for complete GitHub Actions success.
- [x] Merge the implementation PR.
- [ ] Archive the Trellis task in a separate documentation-only PR.
- [ ] Verify only lightweight required checks run, then merge the archive PR.
