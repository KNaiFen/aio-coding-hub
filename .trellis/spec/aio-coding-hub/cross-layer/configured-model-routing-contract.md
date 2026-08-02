# Configured Model Routing Contract

## Scope And Ownership

Configured model routing rewrites a model inference request for a selected
provider. It is distinct from managed Codex aliases and protocol bridges. One
global policy is stored in application settings; each provider may inherit it,
replace it with a complete policy, or explicitly disable it.

A policy contains ordered exact-match rules with `source_model`, optional
`target_model`, and optional free-text `reasoning_effort`. Values are trimmed,
matching is case-sensitive, duplicate sources and rules with neither output are
invalid, and there are no wildcard or default rules. Runtime reads of damaged
or future configuration fail open as no configured route.

## Request Pipeline

- Match exactly once against the original client-requested model. Plugin,
  bridge, failover, or prior-attempt mutations do not change that match key.
- Apply protocol bridges and built-in mappings first. Apply the configured
  target model and effort last for each provider attempt, after
  `RequestBeforeSend`, so the configured route owns the final wire values.
- Include normal inference requests and Codex Responses compact. Exclude model
  lists, search, token counting, probes, discovery, non-POST traffic, and
  managed `aio/` aliases.
- Claude writes `output_config.effort`; Responses writes `reasoning.effort`;
  Chat Completions writes `reasoning_effort`. Gemini numeric text writes an
  integer `thinkingBudget`; other text writes `thinkingLevel`; either removes
  the sibling field.
- A rewrite is single-pass. A target model never feeds another configured rule.

The route module may return no marker for any parse, shape, or rewrite problem.
It must not reject the request, log its body, alter provider selection, add a
retry, or affect circuit health.

## Persistence And Portability

Provider policy uses nullable whole-policy override semantics: absent or null
inherits global, a non-null enabled policy replaces global, and a non-null
disabled policy suppresses global. IPC uses an explicit specified flag so an
omitted patch cannot erase an override. Duplication, provider sharing, and full
configuration export/import preserve the policy without exposing credentials.

Settings and provider writes validate strictly. Schema migration and runtime
decoding sanitize defensively so startup and forwarding remain available even
when persisted JSON is malformed.

## Cost And Observation

Only an applied marker scoped to the final provider is authoritative. It stores
bounded source/effective model, optional effort, policy source, final provider,
and final CLI/model cost basis without request content.

The effective target model drives request cost, usage-ledger cost, cost
backfill, and provider spend-limit accounting. If that target has no price,
cost is unknown and must not fall back to the source model.

Desktop and TUI views retain the original model and add a route indication.
They parse markers independently and fail open: invalid or future metadata only
hides the route label and can never break a list, snapshot, or request.

## Verification

Cover exact/case-sensitive matching, model-only and effort-only rules,
inherit/replace/disable behavior, all four protocols, Gemini numeric/text
effort, compact requests, managed/auxiliary exclusions, plugin ordering,
provider failover, target pricing, missing price, marker provider scope,
malformed persistence, and malformed observation metadata.
