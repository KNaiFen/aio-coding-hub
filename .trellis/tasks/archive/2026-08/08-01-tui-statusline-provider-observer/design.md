# Design

## Status-line configuration

`aio-tui` owns a bounded `tui-config-v1.json` beside the observer descriptor, using the same
home/dot-directory overrides. The schema stores an ordered status-item list and a color flag.
Reads accept only known bounded values; unknown/corrupt content falls back to defaults. Writes
use a same-directory temporary file, flush, Unix `0600`, and atomic replacement. Explicit
`--items` input is validated and takes precedence; `NO_COLOR` always disables ANSI styling.

The renderer projects each selected item into text plus a semantic style, wraps by Unicode
display width without losing styles, and uses a dim middle-dot separator. The configurator is
an alternate-screen picker with a best-effort live snapshot; it remains usable while AIO is
offline. `--once` renders the same ordered data as unstyled text.

## Observer contention and log rendering

The observer keeps its one-connection read-only pool and one-query semaphore. A cache miss waits
fairly for the query permit for at most 1.6 seconds, then rechecks the exact cache key before
querying. Timeout returns authenticated structured `OBS_BUSY`/429 and is never cached as a data
snapshot. The TUI uses a 3.5-second request timeout, maps 429 to a Busy reason, and keeps the last
successful snapshot. Genuine database failures still mark only database-backed sections
unavailable.

Request cards become styled spans: state, identity, route, and usage receive semantic accents;
the whole card is no longer green on success. A dim, width-bounded horizontal rule separates
cards. Text labels and separators carry the hierarchy when colors are disabled.

## Provider observation and dashboard

The v1 snapshot query gains optional `include_providers=true`, and the response gains an optional
provider section. The cache key includes this flag. Status and request views omit the projection;
the provider view requests it on entry/refresh. A new TUI against an older observer retries the
legacy snapshot after an unsupported-query response and shows only the provider view as
unsupported. Older clients ignore the additive response field.

The provider collection is capped at 512 and reports truncation. Its fixed DTO includes only:
provider id/name/CLI, active-route rank and enable flags, authentication kind, preferred and
eligibility labels, peeked circuit state/count/threshold/recovery time, configured spend-window
usage, and bounded cached OAuth quota text/reset times. It excludes endpoints, credentials,
tokens, email, notes, tags, extensions, and arbitrary/error JSON. Database reads are parameterized;
circuits use non-mutating peek; OAuth data is local cache only.

Eligible active-route providers appear in actual route order, followed by route-disabled,
out-of-route, or provider-disabled rows in stable provider order. `all` groups Codex, Claude,
Grok, and Gemini. Each provider renders five fixed lines: route/name/preferred; CLI/auth/enabled;
eligibility/circuit; spend limits; cached OAuth quotas. Enter opens bounded read-only detail.
Left/Right switches dashboard views, which retain independent selection and scrolling.

All configuration and observer failures are observational only. They cannot enter gateway
forwarding, provider selection, retries, circuit mutation, or request logging paths.

## Release

The three logical changes remain separate commits. A fourth release commit synchronizes version
0.60.40. Native drift is accepted only from the GitHub Actions patch artifact. A normal merge to
main must have a successful exact-SHA main CI/release candidate before the release tag is pushed.
