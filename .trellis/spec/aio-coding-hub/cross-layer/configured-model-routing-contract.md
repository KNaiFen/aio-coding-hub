# Configured Model Routing Contract

## Scope And Ownership

Configured model routing rewrites model inference requests after provider
selection. It is distinct from managed Codex aliases and protocol bridges.

- The global policy and each provider's ordinary whole-policy override route
  only within the currently selected provider. They never name another
  provider.
- A cross-provider policy exists only on an enabled source-provider member of
  a named sort mode. Its owner is `(mode UUID, CLI, source provider UUID)`.
  Default mode and the global policy cannot carry cross-provider targets.
- Disabling a provider's ordinary override preserves its member cross policy
  but suppresses both policies at runtime until the override is enabled again.
- Cross targets are stable provider UUIDs and must resolve to another enabled
  member of the same CLI and captured named-mode snapshot. Missing targets stay
  visible as invalid configuration and never silently become the source.

Rules match `source_model` exactly and case-sensitively. Optional
`source_reasoning_effort` is an exact discriminator; absence is a wildcard only
for effort, never for model. The only accepted source and target effort values
are `none`, `minimal`, `low`, `medium`, `high`, `xhigh`, `max`, and `ultra`.
Runtime reads sanitize malformed or future data and fail open as no route.

## Matching And Request Pipeline

For a candidate provider, resolution order is fixed:

1. cross-provider exact source effort;
2. cross-provider source-effort wildcard;
3. ordinary exact source effort;
4. ordinary source-effort wildcard.

The match key is the original client-requested model and protocol-standard
source effort. Old ordinary model-only rules therefore continue to match.
Claude `thinking.budget_tokens` and Gemini `thinkingBudget` are never converted
to effort and never participate in matching.

Source effort is read from exactly five protocol entries: Codex Responses,
Claude Messages, Gemini generate/streamGenerate, Grok Chat Completions, and Grok
Responses. Model lists, token counts, probes, discovery, non-inference traffic,
non-POST traffic, and managed `aio/` aliases are excluded.

Configured model and effort rewrites remain the last owner of wire values for
an attempt. Claude writes `output_config.effort`; Responses writes
`reasoning.effort`; Chat Completions writes `reasoning_effort`; Gemini writes
`generationConfig.thinkingConfig.thinkingLevel` and removes
`thinkingBudget`. Numeric budget text is invalid, not translated.

A rewrite is single-pass. An ordinary target model does not feed another rule.
When A selects temporary target B, B uses A's target model/effort, does not
resolve B's ordinary or cross policy, and cannot chain to C.

## Persistence And Portability

Ordinary provider policy retains nullable whole-policy override semantics:
absent/null inherits global, enabled replaces global, and disabled suppresses
global. Explicit specified flags prevent omitted patches from erasing it.

Named sort modes have stable UUID identities. Member cross policy is persisted
beside sort-mode membership and saved with ordinary provider policy through the
combined owner-scoped transaction and revision checks. Renaming a mode preserves
its UUID; deleting a mode/provider cascades its member policy. Full bundle v5
preserves mode/provider UUIDs and cross policy. Single-provider share and local
provider duplication preserve only ordinary provider policy and never copy
mode membership or cross policy.

Schema migration, bundle import, and runtime decoding sanitize defensively.
Malformed rules and non-standard efforts are discarded without blocking
startup, forwarding, or the rest of a configuration import.

## Execution, Cost, And Observation

Cross routing inserts at most one temporary B before baseline A. B reuses the
normal provider preparation, eligibility gates, credentials, bridge, retry,
circuit, and Ready-provider budget. B never creates, updates, or clears a
session binding. If existing failover policy permits continuation after B
fails, the original client model and unmodified A-to-C baseline resume.

The final provider-scoped `configured_model_route` marker is authoritative for
effective model, price, usage ledger, and spend-limit accounting. Cross success
uses B and B's effective model; fallback success uses the actual A/C provider
and its ordinary route. Missing price remains unknown. The explanatory
`cross_provider_model_route` marker is bounded, single-hop, contains no body,
URL, headers, or credentials, and never changes cost or
`provider_switch_count`.

Desktop and Observer parsers accept `provider_cross` only through the existing
bounded, final-provider-scoped configured-route projection. Desktop may show a
compact `A / source -> B / target` audit label only when the request succeeded
and the final provider equals the marker target. Failed, skipped, malformed,
oversized, mismatched-provider, and future markers stay non-authoritative and
fail open. TUI formatting and its TTFB / switch / retry wording are unchanged.

## Verification

Cover exact-before-wildcard priority, case-sensitive model matching, all eight
efforts, legacy model-only rules, five protocol inputs, Gemini level-only
output, budget exclusions, managed/auxiliary exclusions, one-hop B execution,
non-stream and SSE session guards, B failure baseline restoration, final
provider pricing, bounded markers, v1-v4/v5 import, invalid references, share
stripping, duplicate isolation, and desktop/Observer fail-open projection.
