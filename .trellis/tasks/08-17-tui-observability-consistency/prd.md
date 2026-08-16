# 统一 TUI 观测语义与窄屏布局

## Plan Status

- Implementation authorization: confirmed
- Confirmation date and summary: 2026-08-17; the user confirmed there are no further requirements and authorized worktree creation, handoff, execution documentation, and implementation for this task
- Confirmed coverage: the requirements, locked decisions, non-goals, acceptance criteria, and stop conditions recorded below
- Planning revision: scope frozen by the authorization commit; the full planning SHA is recorded by `task.py delegate`
- Execution route: delegated worktree
- Migrated from direct-main record: none; this is a new complex Trellis task

## Material Facts, Assumptions, and Open Questions

| Item | Source | Status / closure condition |
|---|---|---|
| Context-compaction metadata reaches active and terminal Observer requests; the request detail already renders it | `aio-tui/src/format.rs`, Observer snapshot path | Confirmed; no protocol or persistence change is expected |
| The request-card compaction label is appended after the model and then truncated from the right | `aio-tui/src/format.rs` | Confirmed; the formatter must preserve the semantic suffix before truncating variable model text |
| `policy_source=provider_cross` reaches the Observer projection but is rejected by the TUI validator | Observer contract and `aio-tui/src/format.rs` | Confirmed; fix the TUI validator and labels without reparsing raw markers |
| Provider availability bucket time, state, success, and failure are emitted on one logical line | `aio-tui/src/ui.rs` | Confirmed; split every bucket into a time line and a result line |
| Route hops already expose `skipped` and `ok`, while the TUI summary and detail omit those semantics | Observer protocol, desktop presentation, `aio-tui/src/format.rs` | Confirmed; derive bounded TUI presentation from the projected route |
| The added Codex 372K requirement affects settings, configuration transactions, and model-catalog ownership rather than TUI observation | User requirement and code audit, 2026-08-17 | Closed; all Codex transaction and feature work is consolidated in `08-17-codex-372k-context-window` |

No material product question remains open. Do not start implementation without explicit user authorization.

## Goal

Make the standalone TUI faithfully expose model routing, context compaction, route outcomes, request metrics, and provider availability at narrow terminal widths, without changing gateway behavior or inventing data that the Observer did not provide.

## Requirements

### R1. Preserve context-compaction mode in request cards

- Unrouted requests, unchanged-model routes, and changed-model routes must display `压缩·本地`, `压缩·远程`, or the existing bounded unknown fallback when a valid compaction marker is present.
- At a 32-column terminal, where the request list currently receives 31 columns, the complete local/remote mode label takes priority over model and reasoning-effort text.
- When the available width is smaller than the mode label itself, truncation remains deterministic, grapheme-safe, and non-panicking.
- Request detail keeps the full existing context-compaction section.

### R2. Render cross-provider configured-model routes consistently

- Treat `policy_source=provider_cross` as a valid configured route after the Observer has accepted and final-provider-scoped it.
- Request cards use the existing two-line routed-model presentation, and request detail/status projections use `源模型→有效模型` when the model changed.
- The route-rule label for `provider_cross` is `跨供应商`, not `未知`.
- Unknown, malformed, future, or incomplete route values continue to fail open to the ordinary model presentation.

### R3. Preserve skipped and sent route semantics

- A route containing only skipped candidates must not be labeled `直连` and must not receive a success tone.
- Compact summaries expose every applicable bounded signal: provider switches, skipped candidates, retries, and actual sent attempts. They must not hide retry data merely because skipped candidates also exist.
- Detail summaries distinguish “未发出上游请求” from a sent direct request.
- Every route hop in detail explicitly distinguishes skipped/not-sent, success, failure, and pending/unknown outcomes using the projected `skipped` and `ok` fields.
- If route detail is absent, preserve the existing counter-based fallback so older observers remain usable.

### R4. Align request metrics and detail evidence

- When both cache-read and cache-creation counts are absent, the request card displays `C —`, not `C 0`.
- When at least one cache bucket is known, preserve the current compact sum of known buckets.
- Request detail displays whether the Session was reused, without adding another field to the five-line request card.
- Request detail displays output rate using the existing visibility and calculation helper; this task does not change rate calculation semantics.

### R5. Split provider availability bucket time and result

- In provider detail, every availability bucket uses two logical lines: `HH:MM-HH:MM` on the first and state plus `成N 败N` on the second.
- Time and result never share a logical line.
- Host-local time, the 12-bucket cap, state vocabulary, aggregate summary, and vertical scrolling remain unchanged.

### R6. Keep specifications and tests authoritative

- Update the local Observer/TUI contract with narrow-width compaction priority, cross-provider TUI rendering, skipped-route semantics, request-detail evidence, and the two-line availability layout.
- Update the configured-model-routing contract so it no longer states that cross-provider routing leaves the TUI formatter unchanged.
- Add formatter and rendered-buffer regression tests for all acceptance criteria below.

## Non-Goals

- Do not change request classification, context-compaction detection, gateway routing, retry/failover execution, cost attribution, persistence, or Observer authentication.
- Do not add or change Observer protocol fields unless current repository evidence proves a required field is unavailable; such a discovery is a stop condition.
- Do not change the desktop request-log UI merely to implement the TUI fixes.
- Do not add Session reuse to the compact request card.
- Do not change provider cards outside the provider detail availability section.
- Do not create a release, change package versions, or alter release configuration.

## Acceptance Criteria

- [ ] AC1: With local or remote compaction and a long Codex model/effort, a 32-column terminal renders the complete `压缩·本地` or `压缩·远程` label for unrouted, unchanged-route, and changed-route cards without exceeding the available display width.
- [ ] AC2: Widths `0`, `1`, and other widths smaller than the compaction label remain grapheme-safe and non-panicking; every rendered line stays within its width.
- [ ] AC3: Valid active and terminal `provider_cross` requests render source and effective models with `→`; detail identifies the policy as `跨供应商`.
- [ ] AC4: Unknown/future/malformed configured-route values continue to render the original model safely and do not create a target-model line.
- [ ] AC5: Skipped-only, skipped-then-sent, sent-retry, and provider-switch combinations produce compact and detailed summaries containing all applicable switch/skip/retry/request counts.
- [ ] AC6: Skipped-only routes say that no upstream request was sent, every detail hop exposes its outcome, and skipped-only status presentation is not success-toned.
- [ ] AC7: Cache metrics distinguish both-unknown from zero; Session reuse and valid output rate are visible in request detail without changing output-rate calculation rules.
- [ ] AC8: Each provider availability bucket produces exactly two ordered logical lines, with no time text in the result line and no state/count text in the time line.
- [ ] AC9: A 24-column and a 32-column `TestBackend` rendering keeps representative `成N 败N` text together on the result line and supports scrolling through the expanded section.
- [ ] AC10: Existing local-time formatting, routed-model arrow visibility, old-observer fallback, optional-field fail-open behavior, and provider-list card layout remain covered by regression tests.
- [ ] AC11: The applicable specifications describe the shipped behavior and contain no stale statement that cross-provider routing leaves TUI formatting unchanged.
- [ ] AC12: The fixed local verification command passes against the recorded full base SHA, and the final PR head passes the repository-required GitHub checks.

## Stop Conditions

- A future scope change affects the task boundary, data contract, ownership, or suitable execution order; revise planning and reconfirm before starting.
- A required display value is not present in the current bounded Observer projection and would require a protocol or persistence change.
- Implementation would change gateway routing, pricing, retry behavior, security boundaries, migration behavior, or release configuration.
- The active OAuth planning task begins modifying the same contracts or other newly active work creates a semantic conflict.
- Upstream drift invalidates the formatter, protocol, or contract assumptions recorded here.

## Scope and Decision Changes

| Date | Old / new decision | Affected acceptance criteria | Decision owner / resume condition |
|---|---|---|---|
| 2026-08-17 | Initial scope includes the three reported TUI defects plus confirmed adjacent route/metric inconsistencies | AC1-AC12 | User confirmed planning; implementation remains unauthorized |
| 2026-08-17 | The added Codex 372K requirement is a separate subsystem with a configuration-safety prerequisite; keep this TUI task independent | All | Scope split completed; each task still requires separate implementation authorization |
| 2026-08-17 | Freeze this TUI scope and authorize worktree creation, handoff, execution documentation, and implementation | AC1-AC12 | User confirmed no further requirements |
| 2026-08-17 | Consolidate Codex transaction hardening and the 372K switch into one separate Codex task | All | User decision; TUI remains one independent worktree |

## PENDING Review

- `PENDING.md` was reviewed on 2026-08-17 and contains no unresolved entries.

## Notes

- High-confidence compaction regression point: `eeccf64dc2d60698d0df48ff3fcbcd2aafd24688` (2026-08-05); the truncation-prone suffix layout originated in `3efe533d9816fa25876936ca0423b288f3cdae2d` (2026-08-01).
- Cross-provider TUI omission entered with `552d8bf2ee0902aa9fb0ba71886f5abc79031eb1` (2026-08-14).
- The single-line availability bucket originated in `d6975c46c4b016b7a93e4d679eef8b510f2cdb8c` (2026-08-02).
