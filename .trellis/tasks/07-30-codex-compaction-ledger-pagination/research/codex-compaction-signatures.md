# Codex context compaction request signatures

Local reference: `.local/codex-cli-reference`, inspected at the 2026-07-29
source revision.

Canonical metadata is serialized by
`codex-rs/core/src/responses_metadata.rs` into
`client_metadata["x-codex-turn-metadata"]`; the direct
`x-codex-turn-metadata` header is a compatibility projection.

Compaction metadata contains:

- `request_kind: "compaction"`
- `compaction.trigger`: `manual` or `auto`
- `compaction.reason`: `user_requested`, `context_limit`,
  `model_downshift`, or `comp_hash_changed`
- `compaction.implementation`: `responses`, `responses_compact`, or
  `responses_compaction_v2`
- `compaction.phase`: `standalone_turn`, `pre_turn`, or `mid_turn`
- `compaction.strategy`: `memento` or `prefix_compaction`

Protocol fallbacks:

- Remote v1 posts to `/responses/compact`.
- Remote v2 posts to `/responses` and includes an input item
  `{ "type": "compaction_trigger" }`.
- Local compaction also posts to `/responses`; no reliable prompt heuristic
  distinguishes it, so local detection requires explicit metadata.

The source explicitly describes body metadata as canonical. Therefore AIO
must snapshot original decoded metadata before plugins, use body over header,
and treat every parser/protocol mismatch as an optional observation failure.
