import { describe, expect, it } from "vitest";
import type { ActiveRequest } from "../activeRequests";
import { countActiveInferenceSessions, isActiveInferenceRequest } from "../activeRequests";

function activeRequest(overrides: Partial<ActiveRequest> = {}): ActiveRequest {
  return {
    trace_id: "trace-1",
    cli_key: "codex",
    method: "POST",
    path: "/v1/responses",
    query: null,
    session_id: "session-1",
    requested_model: "gpt-5",
    created_at_ms: 1_000,
    last_activity_ms: 2_000,
    current_attempt: null,
    ...overrides,
  };
}

describe("services/gateway/activeRequests", () => {
  it("recognizes live inference and compaction endpoints only", () => {
    expect(isActiveInferenceRequest(activeRequest())).toBe(true);
    expect(
      isActiveInferenceRequest(
        activeRequest({ path: "/v1/responses/compact/", session_id: "compact" })
      )
    ).toBe(true);
    expect(
      isActiveInferenceRequest(
        activeRequest({
          cli_key: "claude",
          path: "/v1/messages",
          session_id: "claude-session",
        })
      )
    ).toBe(true);
    expect(
      isActiveInferenceRequest(
        activeRequest({
          cli_key: "grok",
          path: "/v1/chat/completions",
          session_id: "grok-session",
        })
      )
    ).toBe(true);
    expect(
      isActiveInferenceRequest(
        activeRequest({
          cli_key: "gemini",
          path: "/v1beta/models/gemini-2.5-pro:streamGenerateContent",
          session_id: "gemini-session",
        })
      )
    ).toBe(true);

    expect(isActiveInferenceRequest(activeRequest({ method: "GET", path: "/v1/models" }))).toBe(
      false
    );
    expect(isActiveInferenceRequest(activeRequest({ path: "/v1/alpha/search" }))).toBe(false);
    expect(
      isActiveInferenceRequest(
        activeRequest({ cli_key: "claude", path: "/v1/messages/count_tokens" })
      )
    ).toBe(false);
  });

  it("counts unique live sessions and treats subagent sessions separately", () => {
    expect(
      countActiveInferenceSessions([
        activeRequest({ trace_id: "main-1", session_id: "main" }),
        activeRequest({ trace_id: "main-2", session_id: " main " }),
        activeRequest({ trace_id: "subagent", session_id: "subagent-1" }),
        activeRequest({
          trace_id: "other-cli",
          cli_key: "claude",
          path: "/v1/messages",
          session_id: "main",
        }),
        activeRequest({ trace_id: "search", path: "/v1/alpha/search", session_id: "search" }),
      ])
    ).toBe(3);
  });

  it("falls back to trace identity when the session id is missing", () => {
    expect(
      countActiveInferenceSessions([
        activeRequest({ trace_id: "trace-a", session_id: null }),
        activeRequest({ trace_id: "trace-a", session_id: " " }),
        activeRequest({ trace_id: "trace-b", session_id: null }),
        activeRequest({ trace_id: "", session_id: null }),
      ])
    ).toBe(2);
  });
});
