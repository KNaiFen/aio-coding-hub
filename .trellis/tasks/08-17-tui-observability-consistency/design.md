# TUI 观测语义与窄屏布局设计

## Context

The affected data already reaches `ObserverRequest` or `ObserverProviderStatus`. The defects are in `aio-tui` presentation:

- model-line truncation discards the compaction-mode suffix;
- the configured-route validator does not recognize `provider_cross`;
- route summaries use aggregate attempt counts without inspecting skipped hops;
- detail formatting ignores projected route and metric evidence;
- provider availability places a time range and its result on one logical line.

The implementation should therefore remain inside the TUI formatter/layout layer plus its specifications and tests. It must not reparse raw request-log markers or create a second source of routing truth.

The later Codex 372K settings requirement is intentionally excluded. It is consolidated in `08-17-codex-372k-context-window`, which owns its configuration transaction and catalog lifecycle and has no TUI presentation dependency.

## Design Principles

1. Semantic evidence outranks variable identifiers at narrow widths. A bounded state label such as `压缩·远程` is more important than the tail of a long model name.
2. Use projected structured fields. `configured_model_route`, `route`, `session_reuse`, usage fields, and availability buckets are authoritative for this task.
3. Preserve fail-open compatibility. Missing or future optional data must degrade to the existing ordinary presentation rather than hide the request or fail rendering.
4. Share derived semantics. Cards, detail, and status output should call common helpers where they describe the same model or route.
5. Keep vertical expansion in detail views. Detail already scrolls; do not compress semantically separate availability values into one line.

## Model And Compaction Formatting

Introduce or refactor a display-width-aware helper that formats a variable lead and a bounded semantic suffix.

- If the complete line fits, preserve the current text.
- If the suffix fits but the line does not, reserve the suffix width and truncate only the variable model/effort lead.
- If the suffix itself does not fit, use the existing grapheme-safe deterministic truncation.
- For a changed-model route, preserve the source-line trailing arrow and apply suffix preservation to the right-aligned target line.
- Apply the same helper to unrouted and unchanged-model single lines so all three paths have identical compaction priority.

The configured-route validator adds `provider_cross` beside `global` and `provider`. It still validates non-empty source/effective models and required effort evidence. `request_model` remains the shared compact source/effective representation for detail and status projections. Policy labels map `global` to `全局`, `provider` to `供应商覆盖`, and `provider_cross` to `跨供应商`.

## Route Presentation

Derive a bounded presentation struct from `ObserverRequest.route` when route hops are available:

- `skipped_count`: number of skipped candidate hops;
- `sent_attempt_count`: sum of normalized attempts for non-skipped hops;
- `retry_count` and `provider_switch_count`: retain the Observer's authoritative counters, which already exclude skipped hops;
- `has_sent_attempt`: whether at least one non-skipped upstream attempt exists.

The compact label emits every applicable token in stable order: `切N`, `跳N`, `重N`, `请N`. A plain successful single sent attempt remains `直连`. A skipped-only route renders `跳N` and the detail says `未发出上游请求`. When `route` is empty, use the current aggregate-counter fallback for old observers.

Route tone follows the same derived state:

- switch, retry, or skipped evidence: warning;
- at least one sent attempt without those conditions: success;
- no sent attempt: default, except skipped-only remains warning.

Each detail hop receives one outcome label:

- `skipped=true`: `已跳过/未发送`;
- `skipped=false, ok=true`: `成功`;
- `skipped=false, ok=false`: `失败`;
- missing `ok`: `进行中` for an active request or `结果未知` otherwise.

Status and error-code evidence remain additive and bounded on the same hop line or on a continuation line if the existing detail wrapper requires it.

## Metrics And Detail

Represent the compact cache total as optional display data:

- both cache buckets absent: `—`;
- either bucket present: saturating sum of present buckets, preserving current compact-number formatting.

Add two detail fields without changing the card line count:

- `Session复用  是/否`, sourced from `session_reuse`;
- `输出速率  <value>` or `—`, sourced from the existing `output_tokens_per_second` helper and its current validity rules.

Do not change rate calculation, Session identity, or request projection.

## Provider Availability Detail

`provider_availability_detail_lines` expands each bucket from one line to two:

```text
  HH:MM-HH:MM
  <状态>  成N 败N
```

The range and aggregate lines remain unchanged. Bucket order, `.take(12)`, host-local timestamp formatting, and state labels remain unchanged. Increased vertical height is handled by the existing detail scroll.

## Specifications

Update:

- `.trellis/spec/aio-coding-hub/cross-layer/local-observer-tui-contract.md` for all TUI behavior in this task;
- `.trellis/spec/aio-coding-hub/cross-layer/configured-model-routing-contract.md` to make `provider_cross` TUI behavior explicit.

No Observer schema, desktop behavior, or gateway routing contract should change.

## File Ownership And Expected Change Surface

Primary files:

- `src-tauri/crates/aio-tui/src/format.rs`
- `src-tauri/crates/aio-tui/src/ui.rs`
- `.trellis/spec/aio-coding-hub/cross-layer/local-observer-tui-contract.md`
- `.trellis/spec/aio-coding-hub/cross-layer/configured-model-routing-contract.md`

Task records may add `execution.md`, `delivery.md`, findings, and acceptance evidence during later phases. Changes outside this surface require main-session review against the stop conditions.

## Verification Design

Formatter tests cover width and semantic matrices without relying only on substring presence. UI tests inspect `TestBackend` row boundaries so a test cannot pass after Ratatui wraps one logical line into two physical lines.

The minimum matrix includes:

- compaction: local/remote; unrouted/unchanged/changed route; widths 0, 1, 24, 31, 32, and 80;
- configured route: global/provider/provider_cross plus malformed/future source;
- route: skipped-only, skipped then sent, retry only, switch plus skip plus retry, active unknown outcome, terminal success/failure;
- metrics: both cache buckets missing, one present, both present, valid/invalid output rate, Session reused/not reused;
- availability: all four states, representative multi-digit counts, 24- and 32-column buffers, and detail scrolling.

Repository policy forbids ordinary local Cargo/test/build commands. Local evidence comes from `$gkd-local-verify` with the registered full base SHA; Rust and integration coverage runs in GitHub Actions on the final PR head.
