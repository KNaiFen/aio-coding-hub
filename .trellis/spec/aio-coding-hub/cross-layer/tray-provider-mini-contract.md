# Tray Provider Mini Geometry Contract

## Scope

The macOS Tray provider mini window has one geometry contract spanning the
React WebView and `src-tauri/src/app/resident.rs`. It changes layout density
only; the snapshot DTO, provider filtering, ordering, availability aggregation,
hover lifecycle, frozen-generation behavior, title, width, and empty state are
outside this contract and must remain unchanged.

## Geometry

- Each provider row in `TrayProviderMiniApp` uses Tailwind `h-6` (24 logical
  pixels).
- Its internal scrolling container uses `max-h-[240px]`, so it displays at
  most ten provider rows before scrolling.
- `TRAY_PROVIDER_MINI_ROW_HEIGHT` is `24.0` and
  `TRAY_PROVIDER_MINI_MAX_VISIBLE_ROWS` remains `10`.
- Native logical height remains `42 + content + 2`: empty content is `68`, and
  non-empty content is `min(provider_count, 10) * 24`.
- Native placement converts the complete logical height through the existing
  monitor scale factor. At 2x scale, a ten-row window has a 568 physical-pixel
  height.

## Behavioral Guarantees

- More than ten providers remain rendered in the WebView and are reachable by
  internal scrolling; they are not truncated from the snapshot or DOM.
- A new snapshot generation resets the internal scroll position to the top.
- The header and empty-state dimensions remain 42 and 68 logical pixels.

## Verification

- Frontend tests assert `h-6`, `max-h-[240px]`, DOM reachability beyond ten
  rows, and generation-driven scroll reset.
- Rust unit tests assert logical heights for 0, 1, 5, 10, and 20 providers
  (112, 68, 164, 284, and 284) plus 2x placement height. Native tests run in
  GitHub Actions; local frontend-only validation must not invoke Cargo tooling.
