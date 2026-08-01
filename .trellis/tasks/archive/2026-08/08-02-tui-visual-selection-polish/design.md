# Technical Design

## Branch Isolation

Work happens in `/private/tmp/aio-tui-visual-selection-polish` on
`codex/tui-visual-selection-polish`, based on `origin/main`. The primary worktree
stays on `main` for parallel work.

## Selection Lifecycle

`LogsState` stores request and provider selections as `Option<usize>` with
independent expiry deadlines. Navigation selects an item and arms a five-second
deadline. Snapshot application preserves a selected request by opaque key and a
provider by id only while that selection exists; it never arms or extends a
deadline.

The event loop clears expired selections while a list is visible or hidden. A
cleared selection renders with `ListState::select(None)`, resetting the viewport
to the newest rows. Enter suspends expiry for the active detail target; Esc
returns to the list and starts a fresh five-second deadline.

## Styled Provider Cards

Provider cards become bounded styled segments rather than one style per line.
Four base lines remain: identity, CLI/auth/toggles, eligibility/circuit, and
spend. A fifth OAuth line is emitted only when bounded cached quota fields contain
displayable text. Styled truncation walks Unicode graphemes and preserves span
styles.

Semantic values use normal ANSI colors: ready/closed/on are green;
limits/cooldown/half-open are yellow; open circuit and hard-off values are red;
not-in-route, stopped, missing and unknown values are dark gray. Field labels use
cyan, names use magenta, and separators are dim.

## Statusline

Interactive status rendering uses an inner rectangle offset by two terminal
columns, so every wrapped line shares Codex's footer gutter. Plain `--once`
output bypasses this renderer and remains unchanged.

The fixed palette mirrors Codex's fallback behavior: normal cyan, green and
magenta accents with dim separators. Bright ANSI colors and bold modifiers are
not used. Existing red/yellow error semantics remain because AIO reports gateway
and upstream health states that should stay actionable.

## Compatibility

No public schema or protocol changes are required. `NO_COLOR` continues to
remove semantic foreground colors, while active selection uses reverse video as
the non-color fallback. Unknown future provider states degrade to dim text.
