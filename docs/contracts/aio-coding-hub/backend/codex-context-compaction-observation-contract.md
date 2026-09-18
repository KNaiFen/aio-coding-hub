# Codex Context Compaction Observation Contract

## Scope

This contract classifies Codex POST requests whose terminal path is
`responses` or `responses/compact`. It adds diagnostics only. It does not
enable compaction and does not change the request.

Semantic context compaction and HTTP `Content-Encoding` compression are
independent features. Zstd, gzip, and other body encodings never identify a
context-compaction operation.

## Original Request Boundary

Classification runs after the existing bounded Codex request decoder and
before `RequestAfterBodyRead`. It observes the original decoded JSON and the
original semantic headers. Plugin mutations cannot create, remove, or replace
the observation.

Evidence priority is:

1. `client_metadata["x-codex-turn-metadata"]` JSON string in the body.
2. Direct `x-codex-turn-metadata` compatibility header.
3. A terminal `responses/compact` path for remote v1.
4. A top-level `input` item with `type == "compaction_trigger"` for remote v2.

A valid canonical body metadata value is authoritative, including a valid
non-compaction value. Header and protocol fallbacks are used only when a
higher-priority source is absent or unusable. Local compaction is never guessed
from prompt text.

## Marker

The bounded `special_settings_json` entry has type
`codex_context_compaction`. It contains fixed allowlisted values:

- `mode`: `local`, `remote`, or `unknown`
- `implementation`: `responses`, `responses_compact`,
  `responses_compaction_v2`, or `unknown`
- known or `unknown` values for `trigger`, `reason`, `phase`, and `strategy`

Unknown source strings and raw metadata are never persisted.

## Fail-Open Invariant

The classifier is a total, non-panicking optional observation. Invalid UTF-8,
JSON, object shape, enum value, duplicate header, excessive metadata, or future
protocol structure yields a smaller unknown marker or no marker.

Classification must not:

- short-circuit or return a gateway error
- modify body bytes, headers, path, model, authentication, or response
- set the Claude compact-request timeout flag
- change provider selection, retries, provider-health neutrality, or circuit
  accounting
- log request content, metadata, credentials, or parser details

The frontend parser follows the same rule: invalid persisted settings hide the
badge without throwing or preventing list rendering.

## Tests

Cover canonical and compatibility metadata, local/remote implementations,
protocol fallbacks, body/header conflicts, malformed and oversized inputs,
unknown future values, plugin mutation, compressed request normalization, live
snapshot propagation, and byte-identical upstream forwarding.
