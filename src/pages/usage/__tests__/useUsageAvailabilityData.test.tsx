import { renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useGatewayCircuitByProviderId } from "../../../query/gateway";
import { useUsageAvailabilityTimelineV1Query } from "../../../query/usage";
import { buildAvailabilityTimelineFromBuckets } from "../../../components/usage/usageAvailabilityTimeline";
import type { CustomDateRangeApplied } from "../../../hooks/useCustomDateRange";
import type { UsagePeriod, UsageAvailabilityTimelineV1 } from "../../../services/usage/usage";
import { useUsageAvailabilityData } from "../useUsageAvailabilityData";

vi.mock("../../../query/usage", () => ({
  useUsageAvailabilityTimelineV1Query: vi.fn(),
}));

vi.mock("../../../query/gateway", () => ({
  useGatewayCircuitByProviderId: vi.fn(),
}));

vi.mock("../../../components/usage/usageAvailabilityTimeline", () => ({
  buildAvailabilityTimelineFromBuckets: vi.fn(() => ({ providers: [] })),
}));

function makeTimeline(): UsageAvailabilityTimelineV1 {
  return {
    start_ms: 100_000,
    end_ms: 200_000,
    bucket_size_ms: 300_000,
    buckets: [
      {
        cli_key: "claude",
        provider_id: 9,
        provider_name: "P9",
        bucket_start_ms: 100_000,
        requests_total: 2,
        requests_success: 1,
        total_duration_ms: 400,
      },
    ],
  };
}

function mockCircuit(cli: string, circuitByProviderId: Record<number, unknown> = {}) {
  return {
    circuitByProviderId,
    refetch: vi.fn(),
    cli,
  };
}

type RangeHookProps = {
  period: UsagePeriod;
  customApplied: CustomDateRangeApplied | null;
};

function lastAvailabilityInput() {
  const calls = vi.mocked(useUsageAvailabilityTimelineV1Query).mock.calls;
  return calls[calls.length - 1]?.[0];
}

describe("pages/usage/useUsageAvailabilityData", () => {
  beforeEach(() => {
    vi.clearAllMocks();

    vi.mocked(useUsageAvailabilityTimelineV1Query).mockReturnValue({
      data: undefined,
      isLoading: false,
      isFetching: false,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useGatewayCircuitByProviderId).mockImplementation(
      (cli: any) => mockCircuit(cli) as any
    );
  });

  it("passes disabled state and a stable rolling range to the availability query", () => {
    vi.mocked(useUsageAvailabilityTimelineV1Query).mockReturnValue({
      data: undefined,
      isLoading: true,
      isFetching: true,
      refetch: vi.fn(),
    } as any);

    const { result } = renderHook(() =>
      useUsageAvailabilityData({
        enabled: false,
        cliKey: "all",
        providerId: null,
        period: "daily",
        customApplied: null,
      })
    );

    expect(useUsageAvailabilityTimelineV1Query).toHaveBeenCalledWith(
      {
        lookbackMs: 24 * 60 * 60 * 1000,
        startMs: null,
        endMs: null,
        cliKey: null,
        providerId: null,
      },
      {
        enabled: false,
        refetchIntervalMs: false,
      }
    );
    expect(result.current.data).toBeNull();
    expect(result.current.loading).toBe(false);
    expect(result.current.refreshing).toBe(false);
    expect(buildAvailabilityTimelineFromBuckets).not.toHaveBeenCalled();
  });

  it("pushes cli/provider filters down and merges circuit state into server buckets", () => {
    const timeline = makeTimeline();
    vi.mocked(useUsageAvailabilityTimelineV1Query).mockReturnValue({
      data: timeline,
      isLoading: false,
      isFetching: true,
      refetch: vi.fn(),
    } as any);
    const claudeCircuit = mockCircuit("claude", { 9: { provider_id: 9, state: "OPEN" } });
    const codexCircuit = mockCircuit("codex", { 10: { provider_id: 10, state: "CLOSED" } });
    const geminiCircuit = mockCircuit("gemini", { 11: { provider_id: 11, state: "HALF_OPEN" } });
    vi.mocked(useGatewayCircuitByProviderId)
      .mockReturnValueOnce(claudeCircuit as any)
      .mockReturnValueOnce(codexCircuit as any)
      .mockReturnValueOnce(geminiCircuit as any);

    const { result } = renderHook(() =>
      useUsageAvailabilityData({
        enabled: true,
        cliKey: "claude",
        providerId: 9,
        period: "daily",
        customApplied: null,
      })
    );

    expect(lastAvailabilityInput()).toEqual({
      lookbackMs: 24 * 60 * 60 * 1000,
      startMs: null,
      endMs: null,
      cliKey: "claude",
      providerId: 9,
    });
    expect(buildAvailabilityTimelineFromBuckets).toHaveBeenCalledWith(timeline, {
      9: { provider_id: 9, state: "OPEN" },
      10: { provider_id: 10, state: "CLOSED" },
      11: { provider_id: 11, state: "HALF_OPEN" },
    });
    expect(result.current.loading).toBe(false);
    expect(result.current.refreshing).toBe(true);
  });

  it("builds weekly, monthly, all-time, and custom query ranges", () => {
    const initialRangeProps: RangeHookProps = { period: "weekly", customApplied: null };
    const { rerender } = renderHook(
      ({ period, customApplied }: RangeHookProps) =>
        useUsageAvailabilityData({
          enabled: true,
          cliKey: "all",
          providerId: null,
          period,
          customApplied,
        }),
      { initialProps: initialRangeProps }
    );

    expect(lastAvailabilityInput()).toMatchObject({ lookbackMs: 7 * 24 * 60 * 60 * 1000 });

    rerender({ period: "monthly", customApplied: null });
    expect(lastAvailabilityInput()).toMatchObject({ lookbackMs: 30 * 24 * 60 * 60 * 1000 });

    rerender({ period: "allTime", customApplied: null });
    expect(lastAvailabilityInput()).toMatchObject({ lookbackMs: 90 * 24 * 60 * 60 * 1000 });

    rerender({
      period: "custom",
      customApplied: {
        startTs: 100,
        endTs: 200,
        startDate: "2026-01-01",
        endDate: "2026-01-02",
      },
    });
    expect(lastAvailabilityInput()).toMatchObject({
      lookbackMs: null,
      startMs: 100_000,
      endMs: 200_000,
    });

    rerender({ period: "custom", customApplied: null });
    expect(lastAvailabilityInput()).toMatchObject({ lookbackMs: 24 * 60 * 60 * 1000 });
  });

  it("refetches availability and all four cli circuit maps", () => {
    const availabilityRefetch = vi.fn();
    const claudeCircuit = mockCircuit("claude");
    const codexCircuit = mockCircuit("codex");
    const geminiCircuit = mockCircuit("gemini");
    const grokCircuit = mockCircuit("grok");
    vi.mocked(useUsageAvailabilityTimelineV1Query).mockReturnValue({
      data: makeTimeline(),
      isLoading: true,
      isFetching: true,
      refetch: availabilityRefetch,
    } as any);
    vi.mocked(useGatewayCircuitByProviderId)
      .mockReturnValueOnce(claudeCircuit as any)
      .mockReturnValueOnce(codexCircuit as any)
      .mockReturnValueOnce(geminiCircuit as any)
      .mockReturnValueOnce(grokCircuit as any);

    const { result } = renderHook(() =>
      useUsageAvailabilityData({
        enabled: true,
        cliKey: "all",
        providerId: null,
        period: "daily",
        customApplied: null,
      })
    );

    expect(result.current.loading).toBe(true);
    expect(result.current.refreshing).toBe(false);

    result.current.refetch();

    expect(availabilityRefetch).toHaveBeenCalledTimes(1);
    expect(claudeCircuit.refetch).toHaveBeenCalledTimes(1);
    expect(codexCircuit.refetch).toHaveBeenCalledTimes(1);
    expect(geminiCircuit.refetch).toHaveBeenCalledTimes(1);
    expect(grokCircuit.refetch).toHaveBeenCalledTimes(1);
  });
});
