import { describe, expect, it } from "vitest";
import { parseAttemptsJson } from "../attemptsJson";

describe("services/gateway/attemptsJson", () => {
  it("parses a valid attempts array", () => {
    const attempts = parseAttemptsJson(
      JSON.stringify([
        {
          provider_id: 1,
          provider_name: "Provider A",
          base_url: "https://example.com",
          requested_upstream_model: "grok-4.5",
          outcome: "success",
          status: 200,
          timeout_secs: 30,
        },
      ])
    );

    expect(attempts).toHaveLength(1);
    expect(attempts?.[0]).toMatchObject({
      provider_id: 1,
      provider_name: "Provider A",
      requested_upstream_model: "grok-4.5",
      outcome: "success",
      timeout_secs: 30,
    });
  });

  it("parses circuit attribution fields when present and leaves them absent otherwise", () => {
    const attempts = parseAttemptsJson(
      JSON.stringify([
        {
          provider_id: 1,
          provider_name: "Provider A",
          base_url: "https://example.com",
          outcome: "skipped",
          status: null,
          circuit_recover_at_unix: 1750001800,
          circuit_trigger_error_code: "GW_UPSTREAM_TIMEOUT",
        },
        {
          provider_id: 2,
          provider_name: "Provider B",
          base_url: "https://example.com",
          outcome: "success",
          status: 200,
        },
      ])
    );

    expect(attempts?.[0]?.circuit_recover_at_unix).toBe(1750001800);
    expect(attempts?.[0]?.circuit_trigger_error_code).toBe("GW_UPSTREAM_TIMEOUT");
    // Success attempts omit the keys entirely; consumers see undefined -> null.
    expect(attempts?.[1]?.circuit_recover_at_unix).toBeUndefined();
    expect(attempts?.[1]?.circuit_trigger_error_code).toBeUndefined();
  });

  it("parses bounded stream-internal-error evidence and rejects malformed evidence", () => {
    const attempts = parseAttemptsJson(
      JSON.stringify([
        {
          provider_id: 1,
          provider_name: "Provider A",
          base_url: "https://example.com",
          outcome: "stream_internal_error",
          status: 200,
          stream_internal_error: {
            event_type: "response.failed",
            error_type: "server_error",
            error_code: "model_at_capacity",
            message: "Selected model is at capacity",
            classification: "retryable",
            matched_keyword: "selected model is at capacity",
            disposition: "retry_same_provider",
            truncated: false,
          },
        },
        {
          provider_id: 2,
          provider_name: "Provider B",
          base_url: "https://example.com",
          outcome: "failure",
          status: 200,
          stream_internal_error: { event_type: "response.failed" },
        },
      ])
    );

    expect(attempts?.[0]?.stream_internal_error).toEqual(
      expect.objectContaining({
        event_type: "response.failed",
        message: "Selected model is at capacity",
        classification: "retryable",
      })
    );
    expect(attempts?.[1]?.stream_internal_error).toBeNull();
  });

  it("returns null for invalid JSON", () => {
    expect(parseAttemptsJson("not json")).toBeNull();
  });

  it("returns null for non-array JSON", () => {
    expect(parseAttemptsJson('{"provider_id":1}')).toBeNull();
    expect(parseAttemptsJson('"plain"')).toBeNull();
  });

  it.each([
    ["null entry", [null]],
    ["non-object entry", ["attempt"]],
    ["missing required fields", [{}]],
    [
      "invalid required field type",
      [
        {
          provider_id: "1",
          provider_name: "Provider A",
          base_url: "https://example.com",
          outcome: "success",
          status: 200,
        },
      ],
    ],
    [
      "invalid optional field type",
      [
        {
          provider_id: 1,
          provider_name: "Provider A",
          base_url: "https://example.com",
          outcome: "success",
          status: 200,
          attempt_duration_ms: "20",
        },
      ],
    ],
    [
      "mixed valid and invalid entries",
      [
        {
          provider_id: 1,
          provider_name: "Provider A",
          base_url: "https://example.com",
          outcome: "success",
          status: 200,
        },
        null,
      ],
    ],
  ])("returns null for a %s", (_caseName, entries) => {
    expect(parseAttemptsJson(JSON.stringify(entries))).toBeNull();
  });

  it("returns null for null, undefined, and empty input", () => {
    expect(parseAttemptsJson(null)).toBeNull();
    expect(parseAttemptsJson(undefined)).toBeNull();
    expect(parseAttemptsJson("")).toBeNull();
  });
});
