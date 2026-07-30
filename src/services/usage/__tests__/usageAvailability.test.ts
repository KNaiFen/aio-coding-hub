import { describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { normalizeUsageAvailabilityInput, usageAvailabilityTimelineV1 } from "../usage";

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      usageAvailabilityTimelineV1: vi.fn(),
    },
  };
});

describe("services/usage/usage availability", () => {
  it("normalizes filters, invokes the generated command, and narrows cli keys", async () => {
    vi.mocked(commands.usageAvailabilityTimelineV1).mockResolvedValue({
      status: "ok",
      data: {
        start_ms: 100,
        end_ms: 200,
        bucket_size_ms: 300_000,
        buckets: [
          {
            cli_key: "claude",
            provider_id: 7,
            provider_name: "P7",
            bucket_start_ms: 100,
            requests_total: 2,
            requests_success: 1,
            total_duration_ms: 450,
          },
        ],
      },
    });

    const result = await usageAvailabilityTimelineV1({
      lookbackMs: null,
      startMs: 100,
      endMs: 200,
      cliKey: " claude " as never,
      providerId: 7,
    });

    expect(commands.usageAvailabilityTimelineV1).toHaveBeenCalledWith({
      lookbackMs: null,
      startMs: 100,
      endMs: 200,
      cliKey: "claude",
      providerId: 7,
    });
    expect(result.buckets[0]?.cli_key).toBe("claude");
  });

  it("requires exactly one valid rolling or custom range", () => {
    expect(() =>
      normalizeUsageAvailabilityInput({
        lookbackMs: 86_400_000,
        startMs: 1,
        endMs: 2,
        cliKey: null,
        providerId: null,
      })
    ).toThrow("SEC_INVALID_INPUT");
    expect(() =>
      normalizeUsageAvailabilityInput({
        lookbackMs: null,
        startMs: 2,
        endMs: 1,
        cliKey: null,
        providerId: null,
      })
    ).toThrow("SEC_INVALID_INPUT");
    expect(() =>
      normalizeUsageAvailabilityInput({
        lookbackMs: 86_400_000,
        startMs: null,
        endMs: null,
        cliKey: null,
        providerId: 0,
      })
    ).toThrow("SEC_INVALID_INPUT");
  });
});
