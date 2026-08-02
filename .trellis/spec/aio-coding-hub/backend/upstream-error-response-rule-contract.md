# Upstream Error Response Rule Contract

Configurable response rules may change the final HTTP error returned to a CLI,
but they are not part of provider routing or health evaluation.

## Source Facts

- Match only an upstream HTTP 4xx/5xx response.
- Match against the original upstream status, bounded response body, request CLI,
  and actual provider ID.
- HTTP 200 in-band errors and transport failures are outside this feature.
- Retry, failover, quota handling, cooldown, and circuit accounting always use
  original upstream facts and finish before a rewrite is returned.
- A failed attempt's candidate is discarded by every later success or different
  failure. Only the final upstream attempt can supply a terminal rewrite.

## Matching

- Enabled rules are ordered by ascending priority and stable list position.
- Status entries are ORed; keyword entries are case-insensitive literal ORs.
- `any` matches either configured group; `all` requires every configured group.
- Empty CLI and provider scopes mean all. Non-empty scopes must both match.
- If a higher-priority rule cannot be evaluated safely because its body is
  unavailable or incomplete, stop and return the original AIO behavior. Do not
  skip to a lower-priority rule.

## Response Construction

- Status and message behavior are independent `passthrough` or `override`
  choices. All client statuses remain within 400-599.
- Message passthrough means extracting a bounded upstream message and wrapping
  it in a protocol-compatible JSON envelope. It never means forwarding unknown
  upstream bytes as the new envelope.
- Claude, Codex/Grok, and Gemini use their respective JSON error envelopes.
- A rewritten response drops stale entity and hop-by-hop headers, preserves a
  valid `Retry-After`, and emits the AIO `x-trace-id`.
- Any extraction, match, configuration, or response-build failure returns the
  pre-existing AIO response without changing forwarding behavior.

## Persistence And Audit

- Rules belong to the ordinary AppSettings field-owned transaction. Settings
  writes reject invalid rules; settings reads drop invalid entries individually.
- The main request status is the client-visible rewritten status. Attempt rows
  retain original upstream statuses and decisions.
- A successful rewrite appends one bounded `upstream_error_response_rule`
  special setting containing only rule/provider identity, before/after status,
  and behavior modes.
- Never persist or log response bodies, matching keywords, extracted messages,
  or configured custom messages as rule audit evidence.

## Verification

- Cover priority, Any/All, scopes, every behavior combination, body unavailable,
  malformed/future config, all-failed failover, and direct abort paths.
- Prove intermediate failure followed by success cannot leave a marker or
  rewritten status.
- Prove malformed `special_settings_json` hides the badge without breaking Home
  or Logs rendering.
