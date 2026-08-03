import { Suspense, useMemo, useState, type ReactNode } from "react";
import {
  CartesianGrid,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "./charts/lazyRecharts";
import type { UsageProviderMetricTrendRowV1 } from "../services/usage/usage";
import { useTheme } from "../hooks/useTheme";
import { cn } from "../utils/cn";
import { formatDurationMs, formatInteger, formatTokensPerSecond } from "../utils/formatters";
import {
  pickPaletteColor,
  getAxisStyle,
  getGridLineStyle,
  getTooltipStyle,
  getAxisLineStroke,
  CHART_ANIMATION,
} from "./charts/chartTheme";
import {
  buildProviderTrendTicks,
  providerTrendBucketKey,
  providerTrendBucketLabel,
  providerTrendLabelContext,
} from "./charts/providerTrendAxis";
import { ProviderTrendLegend } from "./charts/ProviderTrendLegend";

export type UsageTrendMetric = "duration" | "ttfb" | "rate";

type MetricConfig = {
  pickValue: (row: UsageProviderMetricTrendRowV1) => number | null;
  pickSamples: (row: UsageProviderMetricTrendRowV1) => number;
  format: (value: number) => string;
  label: string;
};

const METRIC_CONFIG: Record<UsageTrendMetric, MetricConfig> = {
  duration: {
    pickValue: (row) => (row.avg_duration_ms == null ? null : Number(row.avg_duration_ms)),
    pickSamples: (row) => Number(row.duration_samples) || 0,
    format: formatDurationMs,
    label: "平均耗时",
  },
  ttfb: {
    pickValue: (row) => (row.avg_ttfb_ms == null ? null : Number(row.avg_ttfb_ms)),
    pickSamples: (row) => Number(row.ttfb_samples) || 0,
    format: formatDurationMs,
    label: "TTFB",
  },
  rate: {
    pickValue: (row) =>
      row.avg_output_tokens_per_second == null ? null : Number(row.avg_output_tokens_per_second),
    pickSamples: (row) => Number(row.output_rate_samples) || 0,
    format: formatTokensPerSecond,
    label: "输出速率",
  },
};

type PointMeta = {
  samples: number;
  requestsTotal: number;
};

type ChartDataPoint = {
  label: string;
  [provider: string]: string | number | PointMeta | undefined;
};

type ChartTooltipPayloadEntry = {
  dataKey?: string | number;
  payload?: unknown;
  value?: unknown;
  name?: unknown;
  color?: string;
};

type ChartTooltipProps = {
  active?: boolean;
  payload?: ChartTooltipPayloadEntry[];
  label?: ReactNode;
};

type TooltipItem = PointMeta & {
  name: string;
  color: string;
  value: number;
};

function MetricsTooltip({
  active,
  payload,
  label,
  isDark,
  tooltipStyle,
  format,
}: ChartTooltipProps & {
  isDark: boolean;
  tooltipStyle: ReturnType<typeof getTooltipStyle>;
  format: (value: number) => string;
}) {
  if (!active || !payload?.length) return null;

  const items = payload
    .map((entry): TooltipItem | null => {
      const providerKey = String(entry.dataKey ?? "");
      const value = Number(entry.value);
      if (!providerKey || !Number.isFinite(value)) return null;
      const meta = (entry.payload as ChartDataPoint | undefined)?.[`${providerKey}_meta`] as
        | PointMeta
        | undefined;
      if (!meta) return null;
      return {
        name: String(entry.name ?? providerKey),
        color: entry.color ?? "",
        value,
        ...meta,
      };
    })
    .filter((item): item is TooltipItem => item != null)
    .sort((a, b) => b.samples - a.samples);

  if (!items.length) return null;

  return (
    <div
      style={{
        backgroundColor: tooltipStyle.backgroundColor,
        border: tooltipStyle.border,
        borderRadius: tooltipStyle.borderRadius,
        boxShadow: tooltipStyle.boxShadow,
        padding: tooltipStyle.padding,
        color: tooltipStyle.color,
        minWidth: 220,
      }}
    >
      <div style={{ marginBottom: 8, fontWeight: 600 }}>{label}</div>
      {items.map((item) => (
        <div key={item.name} style={{ marginTop: 6 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <span
              aria-hidden="true"
              style={{
                width: 8,
                height: 8,
                borderRadius: 999,
                backgroundColor: item.color,
                flex: "0 0 auto",
              }}
            />
            <span
              style={{
                flex: 1,
                minWidth: 0,
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
            >
              {item.name}
            </span>
            <span
              style={{
                color: isDark ? "#e2e8f0" : "#0f172a",
                fontVariantNumeric: "tabular-nums",
              }}
            >
              {format(item.value)}
            </span>
          </div>
          <div
            style={{
              margin: "2px 0 0 16px",
              color: isDark ? "#94a3b8" : "#64748b",
              fontSize: 12,
            }}
          >
            成功样本 {formatInteger(item.samples)} / 请求 {formatInteger(item.requestsTotal)}
          </div>
        </div>
      ))}
    </div>
  );
}

function niceCeil(value: number): number {
  if (!Number.isFinite(value) || value <= 0) return 1;
  const exponent = Math.floor(Math.log10(value));
  const base = Math.pow(10, exponent);
  const fraction = value / base;
  const niceFraction = fraction <= 1 ? 1 : fraction <= 2 ? 2 : fraction <= 5 ? 5 : 10;
  return niceFraction * base;
}

type ProviderSeries = {
  key: string;
  name: string;
  color: string;
  totalRequests: number;
};

export function UsageProviderMetricsTrendChart({
  rows,
  metric,
  className,
}: {
  rows: UsageProviderMetricTrendRowV1[];
  metric: UsageTrendMetric;
  className?: string;
}) {
  const { resolvedTheme } = useTheme();
  const [hiddenProviders, setHiddenProviders] = useState<ReadonlySet<string>>(() => new Set());
  const isDark = resolvedTheme === "dark";
  const axisStyle = useMemo(() => getAxisStyle(isDark), [isDark]);
  const gridLineStyle = useMemo(() => getGridLineStyle(isDark), [isDark]);
  const tooltipStyle = useMemo(() => getTooltipStyle(isDark), [isDark]);
  const axisLineStroke = getAxisLineStroke(isDark);
  const config = METRIC_CONFIG[metric];

  const { chartData, providers, xAxisTicks, yMax } = useMemo(() => {
    const labelContext = providerTrendLabelContext(rows);
    const buckets = new Map<string, UsageProviderMetricTrendRowV1>();
    const byProvider = new Map<
      string,
      {
        name: string;
        totalRequests: number;
        points: Map<string, UsageProviderMetricTrendRowV1>;
      }
    >();

    for (const row of rows) {
      if (!row.key) continue;
      const xKey = providerTrendBucketKey(row);
      if (!xKey) continue;
      buckets.set(xKey, row);
      const provider = byProvider.get(row.key) ?? {
        name: row.name || row.key,
        totalRequests: 0,
        points: new Map(),
      };
      provider.name = row.name || provider.name;
      provider.totalRequests += Math.max(0, Number(row.requests_success) || 0);
      provider.points.set(xKey, row);
      byProvider.set(row.key, provider);
    }

    const bucketEntries = Array.from(buckets.entries()).sort(([left], [right]) =>
      left.localeCompare(right)
    );
    const providers: ProviderSeries[] = Array.from(byProvider.entries())
      .map(([key, provider]) => ({ key, ...provider }))
      .sort(
        (left, right) =>
          right.totalRequests - left.totalRequests || left.key.localeCompare(right.key)
      )
      .map((provider, index) => ({
        key: provider.key,
        name: provider.name,
        color: pickPaletteColor(index),
        totalRequests: provider.totalRequests,
      }));

    let globalMax = 0;
    const labels: string[] = [];
    const chartData = bucketEntries.map(([xKey, bucketRow]) => {
      const label = providerTrendBucketLabel(bucketRow, labelContext);
      labels.push(label);
      const point: ChartDataPoint = { label };
      for (const provider of providers) {
        const row = byProvider.get(provider.key)?.points.get(xKey);
        if (!row) continue;
        const value = config.pickValue(row);
        if (value == null || !Number.isFinite(value)) continue;
        globalMax = Math.max(globalMax, value);
        point[provider.key] = value;
        point[`${provider.key}_meta`] = {
          samples: Math.max(0, config.pickSamples(row)),
          requestsTotal: Math.max(0, Number(row.requests_total) || 0),
        };
      }
      return point;
    });

    return {
      chartData,
      providers,
      xAxisTicks: buildProviderTrendTicks(labels),
      yMax: niceCeil(globalMax),
    };
  }, [config, rows]);

  const toggleProvider = (providerKey: string) => {
    setHiddenProviders((current) => {
      const next = new Set(current);
      if (next.has(providerKey)) next.delete(providerKey);
      else next.add(providerKey);
      return next;
    });
  };

  return (
    <div className={cn("flex h-full min-h-0 w-full flex-col", className)}>
      <ProviderTrendLegend
        providers={providers}
        hiddenProviders={hiddenProviders}
        onToggle={toggleProvider}
      />
      <div className="min-h-0 flex-1">
        <Suspense fallback={<div className="h-full w-full" />}>
          <ResponsiveContainer width="100%" height="100%">
            <LineChart
              data={chartData}
              margin={{ left: 0, right: 16, top: 12, bottom: 0 }}
              accessibilityLayer
              aria-label={"供应商" + config.label + "趋势图"}
            >
              <CartesianGrid
                vertical={false}
                stroke={gridLineStyle.stroke}
                strokeDasharray={gridLineStyle.strokeDasharray}
              />
              <XAxis
                dataKey="label"
                axisLine={{ stroke: axisLineStroke }}
                tickLine={false}
                tick={{ ...axisStyle }}
                ticks={xAxisTicks}
                interval="preserveStartEnd"
              />
              <YAxis
                domain={[0, yMax]}
                axisLine={false}
                tickLine={false}
                tick={{ ...axisStyle }}
                tickFormatter={config.format}
                width={58}
              />
              <Tooltip
                content={
                  <MetricsTooltip
                    isDark={isDark}
                    tooltipStyle={tooltipStyle}
                    format={config.format}
                  />
                }
              />
              {providers.map((provider) => (
                <Line
                  key={provider.key}
                  type="monotone"
                  dataKey={provider.key}
                  name={provider.name}
                  stroke={provider.color}
                  strokeWidth={2}
                  dot={false}
                  hide={hiddenProviders.has(provider.key)}
                  animationDuration={CHART_ANIMATION.animationDuration}
                />
              ))}
            </LineChart>
          </ResponsiveContainer>
        </Suspense>
      </div>
    </div>
  );
}
