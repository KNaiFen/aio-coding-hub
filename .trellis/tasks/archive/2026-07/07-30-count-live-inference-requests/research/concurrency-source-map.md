# Concurrency source map

## Existing ownership

- `src-tauri/src/gateway/active_requests.rs` registers active requests by
  `trace_id` and removes them when each request completes. The backend snapshot
  is already one entry per live request.
- `src/services/gateway/activeRequests.ts` contains the inference endpoint
  whitelist and currently deduplicates matching entries by CLI + Session ID.
- `src/components/home/HomeRequestLogsPanel.tsx` is the only production caller
  of the deduplicating helper and owns the visible current-concurrency copy.

## Required semantic change

- Preserve `isActiveInferenceRequest` exactly.
- Count filtered snapshot entries directly.
- Do not read Session ID or trace ID in the count helper.

## Test map

- `src/services/gateway/__tests__/activeRequests.test.ts`: endpoint matrix,
  same-Session behavior, 13/11 scenario, auxiliary exclusions.
- `src/components/home/__tests__/HomeRequestLogsPanel.test.tsx`: rendered
  count, rerender after completions, copy, empty/unavailable/hidden states.
- Existing feed and Home overview tests continue to cover snapshot
  availability and prop wiring.
