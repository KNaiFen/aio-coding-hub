# Tray Provider Mini Geometry Contract

## Scope

The macOS Tray provider mini window has one geometry contract spanning the
React WebView and `src-tauri/src/app/resident.rs`. It owns the native width,
frontend columns, and row density together; the snapshot DTO, provider
filtering, ordering, availability aggregation, hover lifecycle,
frozen-generation behavior, title, and empty state remain unchanged.

## Geometry

- The native logical width is `404` pixels. The complete horizontal contract is
  `1 border + 12 padding + 92 provider + 8 gap + 198 availability + 8 gap + 72 totals + 12 padding + 1 border`.
- Provider rows use fixed `92px / 198px / 72px` tracks. Provider names truncate
  within the first track while preserving the exact name in `title`; reason
  markers reserve at least 40 pixels for two markers, expand for a third, and
  do not shrink.
- The availability track contains eighteen equal `minmax(0, 1fr)` cells
  separated by seventeen 2-pixel gaps.
- Totals contain only two fixed 32-pixel values separated by an 8-pixel gap.
  Success remains green and failure remains red. Values use 9px monospace
  tabular figures, are right-aligned, and never wrap; the exact accessible
  label retains the success/failure meaning without visible label characters.
- Each provider row in `TrayProviderMiniApp` uses Tailwind `h-6` (24 logical
  pixels).
- Its internal scrolling container uses `max-h-[240px]`, so it displays at
  most ten provider rows before scrolling.
- `TRAY_PROVIDER_MINI_ROW_HEIGHT` is `24.0` and
  `TRAY_PROVIDER_MINI_MAX_VISIBLE_ROWS` remains `10`.
- Native logical height remains `42 + content + 2`: empty content is `68`, and
  non-empty content is `min(provider_count, 10) * 24`.
- Native placement converts the complete logical height through the existing
  monitor scale factor. At 2x scale, a ten-row window is 808 by 568 physical
  pixels.

## Count Presentation

- Counts through `99,999` render exactly. Larger values use `万` below one
  hundred million and `亿` at or above that threshold.
- Compact values truncate downward. Scaled values below 100 keep at most one
  decimal; larger scaled values render as truncated integers. A trailing `.0`
  is omitted.
- Compact text is presentation-only. Each value's `title` and the totals'
  accessible label preserve the exact source count.

## Behavioral Guarantees

- More than ten providers remain rendered in the WebView and are reachable by
  internal scrolling; they are not truncated from the snapshot or DOM.
- A new snapshot generation resets the internal scroll position to the top.
- The header and empty-state dimensions remain 42 and 68 logical pixels.

## Verification

- Frontend tests assert the fixed provider, availability, and totals tracks;
  compact-count boundaries; exact titles and accessible labels; `h-6`;
  `max-h-[240px]`; DOM reachability beyond ten rows; and generation-driven
  scroll reset.
- Rust unit tests assert logical heights for 0, 1, 5, 10, and 20 providers
  (112, 68, 164, 284, and 284), 404-pixel logical width, and 808-by-568 2x
  placement. Native and frontend tests run in GitHub Actions; local checks use
  only the cloud-only allowlist.
