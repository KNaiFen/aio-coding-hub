import { useMemo } from "react";
import { type CliFilterKey } from "../../constants/clis";
import type { UsagePeriod } from "../../services/usage/usage";
import type { CustomDateRangeApplied } from "../../hooks/useCustomDateRange";
import { useUsageAvailabilityTimelineV1Query } from "../../query/usage";
import { useGatewayCircuitByProviderId } from "../../query/gateway";
import {
  buildAvailabilityTimelineFromBuckets,
  type AvailabilityTimelineData,
} from "../../components/usage/usageAvailabilityTimeline";

function availabilityQueryRange(
  period: UsagePeriod,
  customApplied: CustomDateRangeApplied | null
): { lookbackMs: number | null; startMs: number | null; endMs: number | null } {
  switch (period) {
    case "daily":
      return { lookbackMs: 24 * 60 * 60 * 1000, startMs: null, endMs: null };
    case "weekly":
      return { lookbackMs: 7 * 24 * 60 * 60 * 1000, startMs: null, endMs: null };
    case "monthly":
      return { lookbackMs: 30 * 24 * 60 * 60 * 1000, startMs: null, endMs: null };
    case "allTime":
      return { lookbackMs: 90 * 24 * 60 * 60 * 1000, startMs: null, endMs: null };
    case "custom":
      if (customApplied) {
        return {
          lookbackMs: null,
          startMs: customApplied.startTs * 1000,
          endMs: customApplied.endTs * 1000,
        };
      }
      return { lookbackMs: 24 * 60 * 60 * 1000, startMs: null, endMs: null };
  }
}

export function useUsageAvailabilityData({
  enabled,
  cliKey,
  providerId,
  period,
  customApplied,
}: {
  enabled: boolean;
  cliKey: CliFilterKey;
  providerId: number | null;
  period: UsagePeriod;
  customApplied: CustomDateRangeApplied | null;
}) {
  const queryRange = availabilityQueryRange(period, customApplied);
  const timelineQuery = useUsageAvailabilityTimelineV1Query(
    {
      ...queryRange,
      cliKey: cliKey === "all" ? null : cliKey,
      providerId,
    },
    {
      enabled,
      refetchIntervalMs: enabled ? 15000 : false,
    }
  );

  const claudeCircuit = useGatewayCircuitByProviderId("claude");
  const codexCircuit = useGatewayCircuitByProviderId("codex");
  const geminiCircuit = useGatewayCircuitByProviderId("gemini");
  const grokCircuit = useGatewayCircuitByProviderId("grok");

  const mergedCircuitMap = useMemo(() => {
    return {
      ...claudeCircuit.circuitByProviderId,
      ...codexCircuit.circuitByProviderId,
      ...geminiCircuit.circuitByProviderId,
      ...grokCircuit.circuitByProviderId,
    };
  }, [
    claudeCircuit.circuitByProviderId,
    codexCircuit.circuitByProviderId,
    geminiCircuit.circuitByProviderId,
    grokCircuit.circuitByProviderId,
  ]);

  const data: AvailabilityTimelineData | null = useMemo(() => {
    if (!timelineQuery.data) return null;
    return buildAvailabilityTimelineFromBuckets(timelineQuery.data, mergedCircuitMap);
  }, [timelineQuery.data, mergedCircuitMap]);

  return {
    data,
    loading: enabled && timelineQuery.isLoading,
    refreshing: enabled && timelineQuery.isFetching && !timelineQuery.isLoading,
    refetch: () => {
      void timelineQuery.refetch();
      void claudeCircuit.refetch();
      void codexCircuit.refetch();
      void geminiCircuit.refetch();
      void grokCircuit.refetch();
    },
  };
}
