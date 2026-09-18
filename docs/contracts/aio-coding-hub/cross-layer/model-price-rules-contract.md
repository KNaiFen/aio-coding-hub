# Model Price Rules Contract

## Storage And Boundaries

`model-prices/price-rules.json` is an independent, atomically replaced v1 rule
set. Price synchronization only owns reference prices and never replaces rules.
The get/set IPC commands read and write the entire validated set. Each exact
CLI and complete model name has at most one rule, including disabled rules.
The reference IPC returns five resolved price items and their available standard,
priority, above-200k, and combined priority/long-context variants, plus the actual
reference model. Missing items remain unpriced.

Prices use USD per million tokens, at most nine decimal places. Multipliers use
at most six decimal places. Both accept finite values from zero through one
million; invalid or unrepresentable values are rejected, never rounded at the
input boundary. Each item has optional price and multiplier fields; the rule
has an optional whole-model multiplier. No mode field exists. Whole and item
multipliers are mutually exclusive, including explicit zero and one.

## Cost Resolution

The existing effective CLI/model is authoritative after routing and protocol
bridging. Exact reference lookup precedes aliases. Prefer its enabled exact
rule; a target rule is eligible only when reference lookup actually found that
alias target. One computation applies at most one rule.

Resolve existing reference prices, priority, long-context tiers, cache fallback,
and CLI token conventions first. A custom price replaces only that item's price
and premiums. An item multiplier applies to that item's complete original cost.
Whole-model multipliers apply to all five items; unspecified multipliers are one.
Reference cache fallback is independent of an input override. A model without a
reference needs both input and output prices; cache prices may be overridden or
derived from that input using the existing fallback. Sum the five items and apply
the existing effective provider multiplier once, preserving fixed-point rounding.
No-rule calculations retain their previous results. Known zero costs remain zero
through detail and leaderboard views; absent usage and unpriced costs remain null.

## Lifetime And History

Every writer batch reads current rules without a cross-request rule cache.
The first terminal persistence selects the saved rules, including requests already
in flight when rules change. A terminal trace is identified by status/error,
not by a non-null cost; subsequent writes preserve its cost and provider multiplier,
including null and zero. A surviving ledger row preserves this boundary after
detail retention. Pending writes cannot downgrade a terminal ledger row.

Rules never update history or invalidate historical query caches. Historical
missing-cost repair uses the original reference-only calculator and recorded
provider multiplier. Request details and the usage ledger share their existing
write transaction. A rule-read failure is logged; request facts are retained and
new costs stay unknown rather than silently using reference-only charges.

## Editor And Verification

One editor form exposes whole multiplier and the five price/multiplier pairs.
Conflicts block save without clearing or hiding inputs. The rule list supports
add, edit, enable/disable, delete, save and cancel. Editing owns a draft independent
of query refreshes. Read errors block saves; save errors keep the draft. Reopening
loads saved rules. Price variants remain labeled in reference/effective previews;
the frontend never calculates persisted request fees.

Focused regressions cover exact/alias selection, multiplier conflicts, fixed-price
and tier behavior, token conventions, zero/unknown prices, route/bridge identities,
terminal preservation, retained ledger traces, reference-only history, and editor
failure/cancel/save flows. Business tests, generated bindings, formatting, lint,
type checks and builds run only in automatic GitHub Actions under the
[cloud-only verification contract](./cloud-only-verification-contract.md).
