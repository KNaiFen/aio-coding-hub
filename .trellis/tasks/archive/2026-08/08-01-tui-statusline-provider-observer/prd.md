# TUI configurable statusline and provider observer

## Goal

Add configurable semantic-color statusline, stabilize concurrent observer clients, improve log readability, and add a five-line provider status view before v0.60.40 release.

## Requirements

- Preserve the existing first three commits on `codex/home-route-tooltip-density` and add
  three independently reviewable TUI/observer commits before the v0.60.40 version commit.
- Let users select and order status-line fields interactively, persist the selection, and
  override it for a single `status` invocation. Semantic colors must remain optional and
  `status --once` must stay plain-text and pipe-friendly.
- Running status and dashboard TUI processes together must not alternate between valid and
  unavailable database projections merely because the observer's read-only query lane is busy.
- Improve request-list scanning with semantic field colors and a visible separator while
  preserving usable no-color output and narrow/CJK terminal behavior.
- Make the default interactive TUI switch between request history and provider status with
  Left/Right. Provider cards use five stable lines, active-route candidates come first, and
  the remainder are visibly marked as disabled or outside the active route.
- Provider observation is local, read-only, bounded, authenticated, and secret-free. It must
  not refresh remote quotas, mutate routes/circuits, or affect gateway forwarding.
- Keep backward compatibility with the current observer v1 endpoint and older TUI/desktop
  combinations wherever possible; unsupported provider observation degrades only that view.
- Do not run Cargo, rustfmt, Clippy, Rust tests, Specta generation, Tauri commands, or native
  packaging locally. GitHub Actions owns all native validation and release artifacts.

## Acceptance Criteria

- [ ] `aio-tui statusline` can toggle/reorder fields, preview the result, toggle colors,
      restore defaults, save atomically, and cancel without writing.
- [ ] `aio-tui status --items ...` overrides selection for one run; malformed persisted
      configuration falls back to defaults and never crashes the TUI.
- [ ] Two concurrent TUI processes retain useful snapshots; bounded observer contention yields
      `OBS_BUSY` and stale-data presentation instead of cached unavailable sections.
- [ ] Request cards color semantic fragments instead of the whole card and have width-bounded
      separators in color and no-color modes.
- [ ] The default dashboard opens on requests and Left/Right switches to a five-line provider
      list with independent selection/scroll state and read-only details.
- [ ] Provider ordering, preferred eligibility, spend/OAuth limits, circuits, truncation, and
      `all` scope are covered without exposing credentials, URLs, email, notes, or raw JSON.
- [ ] Provider projection/config/parser failures are fail-open for the TUI and never change
      request forwarding, retry behavior, routing, circuit accounting, or application shutdown.
- [ ] Local Node/TypeScript checks pass; PR and main GitHub Actions pass before the exact main
      commit is tagged and released as `aio-coding-hub-v0.60.40`.

## Notes

- The untracked `.trellis/workspace/KNaiFen/` directory is user-owned and excluded.
- Repository operations target `origin` / `KNaiFen/aio-coding-hub`; `upstream` is out of scope.
