# TUI 观测语义与窄屏布局实施计划

## Preconditions

1. Keep the added Codex requirements in their separate config-hardening and 372K tasks; recheck that no upstream change creates an unexpected contract conflict.
2. Obtain explicit implementation authorization for this TUI task.
3. Commit the complete planning materials and record the full planning commit SHA.
4. Fetch `origin`, confirm a clean synchronized main checkout, create a task worktree from the full `origin/main` SHA, and register it through `task.py delegate`.
5. Add the task-specific `execution.md`, run `task.py start`, commit the coordination transition, verify `task.py doctor`, and generate the handoff.

Do not perform any implementation step before all preconditions pass.

## Work Package 1: Freeze Contracts And Fixtures

- Update the two applicable cross-layer contracts with the approved TUI behavior.
- Extend TUI test fixtures so they can construct route hops, `provider_cross`, both compaction modes, optional cache data, Session reuse, valid output rate, and availability buckets.
- Add failing formatter-level expectations for AC1-AC8 before changing presentation helpers.

Exit condition: the desired semantics are explicit in contracts and represented by precise fixtures and assertions.

## Work Package 2: Model Route And Compaction Formatting

- Add `provider_cross` to configured-route validation and map its detail label to `跨供应商`.
- Refactor model-line formatting to reserve the compaction suffix before truncating variable text.
- Apply the helper to ordinary, unchanged-route, and changed-route target lines while preserving source arrows and right alignment.
- Keep unknown/future/malformed route fallback unchanged.
- Complete the width matrix and active/terminal route tests for AC1-AC4.

Exit condition: compaction mode and cross-provider model evidence are visible and width-safe across every model-card branch.

## Work Package 3: Route Summary And Hop Outcomes

- Add one shared derived-route presentation helper based on bounded projected hops, with aggregate-counter fallback when hops are absent.
- Use it for compact card wording, detail wording, and route tone.
- Render every applicable switch/skip/retry/request count in stable order.
- Add explicit skipped/success/failure/pending-or-unknown labels to detail hops.
- Cover skipped-only and mixed route matrices for AC5-AC6.

Exit condition: no skipped-only route is labeled or colored as a successful direct request, and card/detail semantics agree.

## Work Package 4: Metrics And Request Detail

- Preserve unknown cache totals instead of coercing both missing buckets to zero.
- Add Session reuse and output-rate fields to request detail only.
- Reuse the existing output-rate helper and do not alter its calculation or validity gates.
- Add regression tests for AC7 and unchanged five-line ordinary request cards.

Exit condition: missing metrics remain unknown and projected detail evidence is no longer dropped.

## Work Package 5: Provider Availability Layout

- Split each availability bucket into a time line and a result line.
- Preserve local-time conversion, state text, aggregate summary, ordering, bucket cap, and scrolling.
- Add direct line-boundary tests and 24-/32-column rendered-buffer tests for AC8-AC10.

Exit condition: time never shares a logical line with state/count results, and representative narrow renders do not wrap the result unexpectedly.

## Work Package 6: Scope Review And Delivery

- Review the final diff against every requirement, non-goal, stop condition, and applicable specification.
- Run `$gkd-local-verify` with the registered full base SHA; do not run ad hoc dependency, Cargo, frontend, or build commands locally.
- Write `delivery.md` with AC-by-AC evidence and any justified deviation.
- Commit implementation and delivery evidence, run `task.py deliver`, commit the delivered state, push the task branch, and open or update the PR.
- Wait for required checks on the exact final head through `$gkd-ci-monitor`, then pause for `$gkd-accept`/main fixed-head acceptance.

Exit condition: the worktree is clean, delivery evidence and required CI bind to the same full PR head SHA, and the execution writer has stopped writing.

## Dependency And Merge Notes

- The existing `08-03-upstream-claude-oauth` task has no current file or semantic overlap. Recheck before worktree creation because both tasks are still planning.
- `08-17-codex-config-transaction-hardening` and `08-17-codex-372k-context-window` are intentionally separate; they may be scheduled independently from this TUI task after separate authorization.
- All primary changes converge on `aio-tui/src/format.rs` and `ui.rs`; use one execution writer rather than parallel implementation worktrees.
- Any future requirement that touches a separate subsystem should be split if combining it would broaden ownership, validation, or rollback beyond this TUI task.
