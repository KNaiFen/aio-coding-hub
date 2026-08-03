import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { UsageProviderMetricTrendRowV1 } from "../../services/usage/usage";
import type { UsageTrendMetric } from "../UsageProviderMetricsTrendChart";

vi.mock("../../hooks/useTheme", () => ({
  useTheme: () => ({ theme: "light", resolvedTheme: "light", setTheme: vi.fn() }),
}));

vi.mock("../charts/lazyRecharts", () => {
  const renderTooltipContent = (content: any, props: any) => {
    if (!content || typeof content.type !== "function") return null;
    const TooltipContent = content.type;
    return <TooltipContent {...content.props} {...props} />;
  };

  const payload = [
    {
      dataKey: "codex:1",
      name: "codex/OpenAI",
      color: "#22c55e",
      value: 1200,
      payload: {
        "codex:1_meta": {
          samples: 7,
          requestsTotal: 24,
        },
      },
    },
    { dataKey: "", value: 1, payload: {} },
    { dataKey: "missing-meta", value: 1, payload: {} },
    { dataKey: "nan", value: Number.NaN, payload: { nan_meta: {} } },
  ];

  return {
    CartesianGrid: () => <div data-testid="grid" />,
    Line: ({ dataKey, hide }: any) => (
      <div data-testid={`line-${dataKey}`} data-hidden={String(Boolean(hide))} />
    ),
    LineChart: ({ children, data, accessibilityLayer, ...props }: any) => (
      <div
        data-testid="line-chart"
        data-points={data?.length ?? 0}
        data-accessibility-layer={String(Boolean(accessibilityLayer))}
        aria-label={props["aria-label"]}
      >
        {children}
      </div>
    ),
    ResponsiveContainer: ({ children }: any) => <div data-testid="responsive">{children}</div>,
    Tooltip: ({ content }: any) => (
      <div data-testid="tooltip">
        {renderTooltipContent(content, { active: false, payload: null, label: "empty" })}
        {renderTooltipContent(content, { active: true, payload: [], label: "empty-list" })}
        {renderTooltipContent(content, { active: true, label: "2026-02-20", payload })}
      </div>
    ),
    XAxis: ({ ticks }: any) => <div data-testid="x-axis" data-ticks={ticks?.join(",") ?? ""} />,
    YAxis: ({ tickFormatter }: any) => (
      <div data-testid="y-axis" data-formatted={tickFormatter ? tickFormatter(1200) : ""} />
    ),
  };
});

import { UsageProviderMetricsTrendChart } from "../UsageProviderMetricsTrendChart";

const sampleRow: UsageProviderMetricTrendRowV1 = {
  day: "2026-02-20",
  hour: null,
  granularity: "day",
  key: "codex:1",
  name: "codex/OpenAI",
  cli_key: "codex",
  provider_id: 1,
  provider_name: "OpenAI",
  requests_total: 12,
  requests_success: 10,
  duration_samples: 10,
  ttfb_samples: 8,
  output_rate_samples: 6,
  avg_duration_ms: 1200,
  avg_ttfb_ms: 300,
  avg_output_tokens_per_second: 42.5,
};

describe("components/UsageProviderMetricsTrendChart", () => {
  const metrics: UsageTrendMetric[] = ["duration", "ttfb", "rate"];

  it("renders no buckets or lines without data", () => {
    render(<UsageProviderMetricsTrendChart rows={[]} metric="duration" />);
    expect(screen.getByTestId("line-chart").getAttribute("data-points")).toBe("0");
    expect(screen.queryByTestId("line-codex:1")).toBeNull();
  });

  it("uses adaptive hourly buckets without collapsing multiple days into one day", () => {
    const rows: UsageProviderMetricTrendRowV1[] = [
      { ...sampleRow, granularity: "hour", hour: 10 },
      { ...sampleRow, granularity: "hour", day: "2026-02-21", hour: 14 },
    ];
    render(<UsageProviderMetricsTrendChart rows={rows} metric="duration" />);

    expect(screen.getByTestId("line-chart").getAttribute("data-points")).toBe("2");
    expect(screen.getByTestId("x-axis").getAttribute("data-ticks")).toBe("02/20 10:00,02/21 14:00");
  });

  it("formats weekly, monthly, and yearly buckets from backend granularity", () => {
    const rows: UsageProviderMetricTrendRowV1[] = [
      { ...sampleRow, granularity: "week", day: "2026-02-16" },
      { ...sampleRow, granularity: "month", day: "2026-03" },
      { ...sampleRow, granularity: "year", day: "2027" },
    ];
    render(<UsageProviderMetricsTrendChart rows={rows} metric="ttfb" />);

    expect(screen.getByTestId("x-axis").getAttribute("data-ticks")).toBe("2026-02-16,2026-03,2027");
  });

  it("shows metric value, successful samples, and total requests in tooltip", () => {
    render(<UsageProviderMetricsTrendChart rows={[sampleRow]} metric="duration" />);

    expect(screen.getAllByText("codex/OpenAI")).toHaveLength(2);
    expect(screen.getByText("成功样本 7 / 请求 24")).toBeTruthy();
    expect(screen.queryByText(/missing-meta|nan/)).toBeNull();
  });

  for (const metric of metrics) {
    it(`renders provider series for metric=${metric}`, () => {
      const rows: UsageProviderMetricTrendRowV1[] = [
        sampleRow,
        {
          ...sampleRow,
          key: "claude:2",
          name: "claude/Anthropic",
          cli_key: "claude",
          provider_id: 2,
          provider_name: "Anthropic",
        },
      ];
      render(<UsageProviderMetricsTrendChart rows={rows} metric={metric} />);

      expect(screen.getByTestId("line-codex:1")).toBeTruthy();
      expect(screen.getByTestId("line-claude:2")).toBeTruthy();
    });
  }

  it("formats rate axis as tokens per second", () => {
    render(<UsageProviderMetricsTrendChart rows={[sampleRow]} metric="rate" />);
    expect(screen.getByTestId("y-axis").getAttribute("data-formatted")).not.toContain("ms");
  });

  it("enables the chart accessibility layer with a metric-specific label", () => {
    render(<UsageProviderMetricsTrendChart rows={[sampleRow]} metric="ttfb" />);
    const chart = screen.getByLabelText("供应商TTFB趋势图");
    expect(chart).toHaveAttribute("data-accessibility-layer", "true");
  });

  it("hides and restores a provider series from the legend", () => {
    render(<UsageProviderMetricsTrendChart rows={[sampleRow]} metric="duration" />);
    const legendButton = screen.getByRole("button", { name: "codex/OpenAI" });

    expect(legendButton).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByTestId("line-codex:1")).toHaveAttribute("data-hidden", "false");

    fireEvent.click(legendButton);
    expect(legendButton).toHaveAttribute("aria-pressed", "false");
    expect(screen.getByTestId("line-codex:1")).toHaveAttribute("data-hidden", "true");

    fireEvent.click(legendButton);
    expect(legendButton).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByTestId("line-codex:1")).toHaveAttribute("data-hidden", "false");
  });
});
