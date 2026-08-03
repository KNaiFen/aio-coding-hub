import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  useUsageLeaderboardV2Query,
  useUsageProviderCacheRateTrendV1Query,
  useUsageProviderMetricTrendV1Query,
  useUsageSummaryV2Query,
} from "../../../query/usage";
import { useUsagePageDataModel } from "../useUsagePageDataModel";

vi.mock("../../../query/usage", () => ({
  useUsageSummaryV2Query: vi.fn(),
  useUsageLeaderboardV2Query: vi.fn(),
  useUsageProviderCacheRateTrendV1Query: vi.fn(),
  useUsageProviderMetricTrendV1Query: vi.fn(),
}));

const summaryRefetch = vi.fn();
const leaderboardRefetch = vi.fn();
const cacheTrendRefetch = vi.fn();
const metricsTrendRefetch = vi.fn();

function queryResult({
  data,
  error = null,
  refetch,
}: {
  data: unknown;
  error?: Error | null;
  refetch: () => unknown;
}) {
  return {
    data,
    error,
    isError: error != null,
    isFetching: false,
    refetch,
  } as any;
}

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(useUsageSummaryV2Query).mockReturnValue(
    queryResult({ data: null, error: new Error("summary failed"), refetch: summaryRefetch })
  );
  vi.mocked(useUsageLeaderboardV2Query).mockReturnValue(
    queryResult({ data: [], refetch: leaderboardRefetch })
  );
  vi.mocked(useUsageProviderCacheRateTrendV1Query).mockReturnValue(
    queryResult({ data: [], refetch: cacheTrendRefetch })
  );
  vi.mocked(useUsageProviderMetricTrendV1Query).mockReturnValue(
    queryResult({ data: [], refetch: metricsTrendRefetch })
  );
});

describe("pages/usage/useUsagePageDataModel", () => {
  it("keeps summary errors visible on the metrics tab and retries the failed data group", () => {
    const { result } = renderHook(() =>
      useUsagePageDataModel({
        tableTab: "metricsTrend",
        scope: "provider",
        period: "weekly",
        cliKey: "all",
        providerId: null,
        customApplied: null,
        bounds: { startTs: 1, endTs: 2 },
      })
    );

    expect(result.current.errorText).toContain("汇总数据：");
    expect(result.current.errorText).toContain("summary failed");
    expect(result.current.panelErrorText).toBeNull();

    act(() => result.current.handleRetry());
    expect(summaryRefetch).toHaveBeenCalledTimes(1);
    expect(leaderboardRefetch).toHaveBeenCalledTimes(1);
    expect(metricsTrendRefetch).not.toHaveBeenCalled();
  });

  it("reports and retries both summary and metrics failures", () => {
    vi.mocked(useUsageProviderMetricTrendV1Query).mockReturnValue(
      queryResult({
        data: [],
        error: new Error("metrics failed"),
        refetch: metricsTrendRefetch,
      })
    );
    const { result } = renderHook(() =>
      useUsagePageDataModel({
        tableTab: "metricsTrend",
        scope: "provider",
        period: "weekly",
        cliKey: "all",
        providerId: null,
        customApplied: null,
        bounds: { startTs: 1, endTs: 2 },
      })
    );

    expect(result.current.errorText).toContain("汇总数据：");
    expect(result.current.errorText).toContain("性能趋势：");
    expect(result.current.panelErrorText).toContain("metrics failed");

    act(() => result.current.handleRetry());
    expect(summaryRefetch).toHaveBeenCalledTimes(1);
    expect(leaderboardRefetch).toHaveBeenCalledTimes(1);
    expect(metricsTrendRefetch).toHaveBeenCalledTimes(1);
  });
});
