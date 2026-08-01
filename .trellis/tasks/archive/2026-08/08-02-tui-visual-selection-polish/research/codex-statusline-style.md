# Codex CLI Statusline Research

Source: `.local/codex-cli-reference/codex-rs/tui` at the locally pinned reference.

- `ui_consts.rs` defines `LIVE_PREFIX_COLS` and `FOOTER_INDENT_COLS` as two
  terminal columns. Footer status content starts after that gutter.
- `bottom_pane/status_line_style.rs` groups fields into semantic accents. Theme
  syntax scopes are preferred; fallback colors are ordinary cyan, green and
  magenta.
- Separators use `" · "` with the `DIM` modifier.
- Bright ANSI colors are normalized to ordinary variants. RGB theme colors are
  softened to 85 percent saturation without dimming the field text.
- Status fields are not bold. Disabling theme colors dims the whole line.

AIO has no Codex syntax-theme resolver, so this task adopts the stable fallback
palette and keeps red/yellow only for AIO-specific error and warning semantics.
