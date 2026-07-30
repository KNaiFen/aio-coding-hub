import { describe, expect, it } from "vitest";
import type { ActiveRequest } from "../activeRequests";
import { countActiveInferenceRequests, isActiveInferenceRequest } from "../activeRequests";

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
    special_settings_json: overrides.special_settings_json ?? null,
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

  it("counts each parent and subagent inference request", () => {
    const parents = Array.from({ length: 3 }, (_, index) =>
      activeRequest({
        trace_id: `parent-${index + 1}`,
        session_id: `parent-session-${index + 1}`,
      })
    );
    const subagents = Array.from({ length: 10 }, (_, index) =>
      activeRequest({
        trace_id: `subagent-${index + 1}`,
        session_id: `subagent-session-${index + 1}`,
      })
    );

    expect(countActiveInferenceRequests([...parents, ...subagents])).toBe(13);
    expect(countActiveInferenceRequests([...parents, ...subagents.slice(0, -2)])).toBe(11);
  });

  it("counts parallel inference requests in the same session separately", () => {
    expect(
      countActiveInferenceRequests([
        activeRequest({ trace_id: "parallel-1", session_id: "shared-session" }),
        activeRequest({ trace_id: "parallel-2", session_id: "shared-session" }),
      ])
    ).toBe(2);
  });

  it("excludes auxiliary and non-POST requests from the active inference count", () => {
    expect(
      countActiveInferenceRequests([
        activeRequest({ trace_id: "inference" }),
        activeRequest({ trace_id: "models", method: "GET", path: "/v1/models" }),
        activeRequest({ trace_id: "search", path: "/v1/alpha/search" }),
        activeRequest({
          trace_id: "token-count",
          cli_key: "claude",
          path: "/v1/messages/count_tokens",
        }),
      ])
    ).toBe(1);
  });
});
