// Usage:
// - Render in `HomeOverviewPanel` as the compact usage chart above the information panel.

import { useMemo } from "react";
import type { UsageHourlyRow } from "../../services/usage/usage";
import { Card } from "../../ui/Card";
import { formatTokensMillions } from "../../utils/chartHelpers";
import { buildRecentDayKeys, dayKeyFromLocalDate } from "../../utils/dateKeys";
import { UsageTokensChart } from "../UsageTokensChart";

export type HomeUsageSectionProps = {
  devPreviewEnabled?: boolean;
  usageWindowDays?: number;
  usageHeatmapRows: UsageHourlyRow[];
  usageHeatmapLoading: boolean;
};

function buildPreviewUsageRows(days = 15): UsageHourlyRow[] {
  const dayKeys = buildRecentDayKeys(days);
  const dayWave = [0.82, 1.18, 0.94, 1.36, 0.88, 1.24, 0.98] as const;

  const noise = (seed: number) => {
    const value = Math.sin(seed * 12.9898 + 78.233) * 43758.5453;
    return value - Math.floor(value);
  };

  const gaussian = (hour: number, center: number, spread: number) => {
    const delta = (hour - center) / spread;
    return Math.exp(-(delta * delta));
  };

  return dayKeys.flatMap((day, index) => {
    const dayFactor = dayWave[index % dayWave.length];
    const dayDrift = 0.9 + noise(index + 101) * 0.28;

    return Array.from({ length: 24 }, (_, hour) => {
      const hourProfile =
        gaussian(hour, 2.5, 2.4) * 0.38 +
        gaussian(hour, 9.5, 3.4) * 0.92 +
        gaussian(hour, 14.5, 3.8) * 1.08 +
        gaussian(hour, 20, 3.1) * 0.84;
      const hourNoise = 0.74 + noise(index * 37 + hour * 19 + 7) * 0.62;
      const scatterNoise = noise(index * 83 + hour * 29 + 17);
      const gapNoise = noise(index * 41 + hour * 13 + 3);
      const activity = hourProfile * dayFactor * dayDrift * hourNoise;

      const gapThreshold = 0.2 + noise(index * 17 + 5) * 0.16;
      const isQuietHour =
        activity < 0.17 ||
        (gapNoise < gapThreshold && scatterNoise < 0.72) ||
        (hour >= 0 && hour <= 4 && scatterNoise < 0.58);

      const leveledActivity = Math.max(0, activity * (0.9 + scatterNoise * 0.45));
      const requestsBase = isQuietHour ? 0 : Math.max(1, Math.round(4 + leveledActivity * 7));
      const totalTokens = isQuietHour ? 0 : Math.round(85_000 + leveledActivity * 205_000);
      const failed = hour >= 19 && requestsBase > 0 && (index + hour) % 11 === 0 ? 1 : 0;

      return {
        day,
        hour,
        requests_total: requestsBase,
        requests_with_usage: requestsBase > 0 ? requestsBase : 0,
        requests_success: requestsBase - failed,
        requests_failed: failed,
        total_tokens: totalTokens,
      };
    });
  });
}

export function HomeUsageSection({
  devPreviewEnabled = false,
  usageWindowDays = 15,
  usageHeatmapRows,
  usageHeatmapLoading,
}: HomeUsageSectionProps) {
  const displayedUsageHeatmapRows = useMemo(
    () =>
      devPreviewEnabled && usageHeatmapRows.length === 0 && !usageHeatmapLoading
        ? buildPreviewUsageRows(usageWindowDays)
        : usageHeatmapRows,
    [devPreviewEnabled, usageHeatmapLoading, usageHeatmapRows, usageWindowDays]
  );
  const todayTokens = useMemo(() => {
    const todayKey = dayKeyFromLocalDate(new Date());
    return displayedUsageHeatmapRows.reduce((sum, row) => {
      if (row.day !== todayKey) return sum;
      return sum + (Number(row.total_tokens) || 0);
    }, 0);
  }, [displayedUsageHeatmapRows]);

  return (
    <Card className="flex h-48 min-h-48 flex-col" padding="sm">
      <div className="mb-2 flex items-start justify-between gap-3">
        <div className="text-sm font-semibold text-foreground">用量统计</div>
        <div className="shrink-0 text-right text-sm text-muted-foreground">
          <span className="mr-1.5 text-[11px] font-medium text-muted-foreground">今日用量</span>
          <span className="font-semibold text-secondary-foreground dark:text-foreground">
            {formatTokensMillions(todayTokens)}
          </span>
        </div>
      </div>
      {usageHeatmapLoading && displayedUsageHeatmapRows.length === 0 ? (
        <div className="flex flex-1 items-center text-sm text-muted-foreground">加载中…</div>
      ) : (
        <div className="min-h-0 flex-1">
          <UsageTokensChart
            rows={displayedUsageHeatmapRows}
            days={usageWindowDays}
            className="h-full"
          />
        </div>
      )}
    </Card>
  );
}
