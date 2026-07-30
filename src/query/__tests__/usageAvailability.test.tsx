import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { usageAvailabilityTimelineV1 } from "../../services/usage/usage";
import { createQueryWrapper, createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
import { usageKeys } from "../keys";
import { useUsageAvailabilityTimelineV1Query } from "../usage";

vi.mock("../../services/usage/usage", async () => {
  const actual = await vi.importActual<typeof import("../../services/usage/usage")>(
    "../../services/usage/usage"
  );
  return {
    ...actual,
    usageAvailabilityTimelineV1: vi.fn(),
  };
});

describe("query/usage availability", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    setTauriRuntime();
  });

  it("normalizes a stable rolling key and forwards the service input", async () => {
    vi.mocked(usageAvailabilityTimelineV1).mockResolvedValue({
      start_ms: 100,
      end_ms: 200,
      bucket_size_ms: 300_000,
      buckets: [],
    });
    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const input = {
      lookbackMs: 86_400_000,
      startMs: null,
      endMs: null,
      cliKey: " claude ",
      providerId: 7,
    } as never;
    const normalized = {
      lookbackMs: 86_400_000,
      startMs: null,
      endMs: null,
      cliKey: "claude" as const,
      providerId: 7,
    };

    const { result } = renderHook(
      () =>
        useUsageAvailabilityTimelineV1Query(input, {
          refetchIntervalMs: 15_000,
        }),
      { wrapper }
    );

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(usageAvailabilityTimelineV1).toHaveBeenCalledWith(normalized);
    expect(client.getQueryState(usageKeys.availabilityTimelineV1(normalized))).toBeTruthy();
  });

  it("does not call the service when disabled", async () => {
    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(
      () =>
        useUsageAvailabilityTimelineV1Query(
          {
            lookbackMs: 86_400_000,
            startMs: null,
            endMs: null,
            cliKey: null,
            providerId: null,
          },
          { enabled: false }
        ),
      { wrapper }
    );
    await Promise.resolve();

    expect(usageAvailabilityTimelineV1).not.toHaveBeenCalled();
  });
});
