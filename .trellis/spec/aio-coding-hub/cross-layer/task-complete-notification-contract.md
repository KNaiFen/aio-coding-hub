# Task-Complete Notification Contract

## Semantics

Task-complete notifications are a renderer-side convenience layered over
gateway events. They do not define request completion or concurrency. Codex
uses a 120-second quiet period; Claude, Gemini, and Grok use 30 seconds.

The notification session groups overlapping request IDs for one CLI. Start and
completion events update that in-flight set. A notification becomes eligible
only after the set is empty for the complete quiet period.

## Timer And Snapshot Ownership

- Every session owns a monotonically increasing generation. Request starts,
  completions, disablement, cleanup, and rescheduling invalidate older timer or
  asynchronous snapshot work.
- When the timer expires, query the backend active-request snapshot and suppress
  notification if the same CLI still has any active model inference request.
- After the snapshot resolves, re-read the session and require the same
  generation, pending state, and empty in-flight set before notifying.
- Backend snapshot failure skips that notification attempt and records only a
  bounded diagnostic. It must not guess that the CLI is idle.

CLI keys are compared canonically. Auxiliary active requests do not prolong the
quiet period because the backend snapshot uses the model-inference classifier.

## Failure Isolation

Notification permission, timer, event-order, snapshot, and renderer lifecycle
failures must not change forwarding, retries, provider selection, circuit
health, request logs, or active-request registration. Stale callbacks send
nothing. Disabling notifications clears local state only.

## Verification

Use fake timers and mocked snapshots to cover each CLI delay, overlapping
requests, new work during the quiet period, active same-CLI suppression,
different-CLI activity, snapshot failure, disable/cleanup, duplicate or late
events, and a snapshot result that becomes stale while awaiting completion.
