# Count live inference requests

## Goal

Make the Home current-concurrency value represent the number of active model
inference requests, not the number of distinct sessions.

## Requirements

- Count every active request that already matches the existing model-inference
  endpoint classifier as one concurrent request.
- Parallel requests in the same Session must each count independently.
- Parent-agent and subagent requests must each count independently.
- Preserve the existing inference-only scope for Claude Messages, Codex
  Responses/Compact, Grok Chat/Responses, and Gemini GenerateContent.
- Auxiliary requests such as model lists, search, token counting, probes, and
  non-POST requests must remain excluded.
- Keep the unavailable snapshot display as `--` and the available empty
  snapshot display as `0`.
- Update the Home tooltip/accessibility description to state the per-request
  semantics.
- Do not change the backend active-request registry, IPC contracts, gateway
  forwarding, request logging, retries, or provider health behavior.

## Acceptance Criteria

- [ ] Three active parent requests plus ten active subagent requests display
      `13`.
- [ ] When two subagent requests finish, the same snapshot displays `11`.
- [ ] Two inference requests with the same Session ID count as `2`.
- [ ] Auxiliary requests do not increase the concurrency count.
- [ ] Snapshot failure/unavailability still displays `--`.
- [ ] The tooltip explains that each active model inference request counts as
      one, including same-session and subagent requests.
- [ ] Targeted tests, TypeScript typecheck, lint, and frontend build pass.
- [ ] PR CI and exact-merge `main` CI pass before tagging v0.60.37.

## Constraints

- Version `0.60.37` is a patch release after published `0.60.36`.
- Local Rust compilation and Rust tooling must not be run; Rust acceptance is
  provided exclusively by GitHub CI.
