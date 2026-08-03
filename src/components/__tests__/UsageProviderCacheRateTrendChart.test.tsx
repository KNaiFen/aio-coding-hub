import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { UsageProviderCacheRateTrendRowV1 } from "../../services/usage/usage";

vi.mock("../../hooks/useTheme", () => ({
  useTheme: () => ({ theme: "light", resolvedTheme: "light", setTheme: vi.fn() }),
}));

vi.mock("../charts/lazyRecharts", () => {
  const renderTooltipContent = (content: any, props: any) => {
    if (!content || typeof content.type !== "function") return null;
    const TooltipContent = content.type;
    return <TooltipContent {...content.props} {...props} />;
  };

  const warnPayload = Array.from({ length: 13 }, (_, index) => {
    const key = `warn-${index}`;
    return {
      dataKey: key,
      name: `Warn ${index}`,
      color: "#ef4444",
      value: 0.2 + index / 100,
      payload: {
        [`${key}_meta`]: {
          denomTokens: 1000 + index,
          cacheReadTokens: 200 + index,
          requestsSuccess: 10 + index,
        },
      },
    };
  });

  const okPayload = [
    {
      dataKey: "ok",
      name: "OK",
      color: "#22c55e",
      value: 0.9,
      payload: {
        ok_meta: {
          denomTokens: 5000,
          cacheReadTokens: 4500,
          requestsSuccess: 20,
        },
      },
    },
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
    ReferenceArea: ({ x1, x2 }: any) => (
      <div data-testid="reference-area" data-x1={x1} data-x2={x2} />
    ),
    ReferenceLine: ({ y }: any) => <div data-testid="reference-line" data-y={y} />,
    ResponsiveContainer: ({ children }: any) => <div data-testid="responsive">{children}</div>,
    Tooltip: ({ content }: any) => (
      <div data-testid="tooltip">
        {renderTooltipContent(content, { active: false, payload: null, label: "empty" })}
        {renderTooltipContent(content, { active: true, payload: [], label: "empty-list" })}
        {renderTooltipContent(content, {
          active: true,
          label: "warn",
          payload: [
            { dataKey: "", value: 0.5, payload: {} },
            { dataKey: "missing-meta", value: 0.5, payload: {} },
            { dataKey: "nan", value: Number.NaN, payload: { nan_meta: {} } },
            ...warnPayload,
          ],
        })}
        {renderTooltipContent(content, { active: true, label: "ok", payload: okPayload })}
      </div>
    ),
    XAxis: ({ ticks }: any) => <div data-testid="x-axis" data-ticks={ticks?.join(",") ?? ""} />,
    YAxis: ({ ticks }: any) => <div data-testid="y-axis" data-ticks={ticks?.join(",") ?? ""} />,
  };
});

import { UsageProviderCacheRateTrendChart } from "../UsageProviderCacheRateTrendChart";

const sampleRow: UsageProviderCacheRateTrendRowV1 = {
  day: "2026-02-20",
  hour: null,
  granularity: "day",
  key: "openai",
  name: "OpenAI",
  denom_tokens: 200,
  cache_read_input_tokens: 100,
  requests_success: 10,
};

describe("components/UsageProviderCacheRateTrendChart", () => {
  it("renders without data", () => {
    const { container } = render(<UsageProviderCacheRateTrendChart rows={[]} />);
    expect(container).toBeTruthy();
  });

  it("renders backend-selected weekly buckets", () => {
    const rows: UsageProviderCacheRateTrendRowV1[] = [
      { ...sampleRow, granularity: "week" },
      { ...sampleRow, granularity: "week", day: "2026-02-27", cache_read_input_tokens: 200 },
      { ...sampleRow, granularity: "week", key: "anthropic", name: "Anthropic" },
    ];
    render(<UsageProviderCacheRateTrendChart rows={rows} />);
    expect(screen.getByTestId("line-chart").getAttribute("data-points")).toBe("2");
    expect(screen.getByTestId("x-axis").getAttribute("data-ticks")).toBe("02/20,02/27");
  });

  it("keeps hourly buckets distinct across days", () => {
    const rows: UsageProviderCacheRateTrendRowV1[] = [
      { ...sampleRow, granularity: "hour", hour: 10 },
      { ...sampleRow, granularity: "hour", day: "2026-02-21", hour: 14 },
    ];
    render(<UsageProviderCacheRateTrendChart rows={rows} />);
    expect(screen.getByTestId("x-axis").getAttribute("data-ticks")).toBe("02/20 10:00,02/21 14:00");
  });

  it("hides and restores a provider series from the legend", () => {
    render(<UsageProviderCacheRateTrendChart rows={[sampleRow]} />);
    const legendButton = screen.getByRole("button", { name: "OpenAI" });

    expect(legendButton).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByTestId("line-openai")).toHaveAttribute("data-hidden", "false");
    expect(screen.getByTestId("reference-area")).toBeTruthy();

    fireEvent.click(legendButton);
    expect(legendButton).toHaveAttribute("aria-pressed", "false");
    expect(screen.getByTestId("line-openai")).toHaveAttribute("data-hidden", "true");
    expect(screen.queryByTestId("reference-area")).toBeNull();

    fireEvent.click(legendButton);
    expect(legendButton).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByTestId("line-openai")).toHaveAttribute("data-hidden", "false");
    expect(screen.getByTestId("reference-area")).toBeTruthy();
  });

  it("enables the chart accessibility layer and labels the legend group", () => {
    render(<UsageProviderCacheRateTrendChart rows={[sampleRow]} />);
    expect(screen.getByLabelText("供应商缓存命中率趋势图")).toHaveAttribute(
      "data-accessibility-layer",
      "true"
    );
    expect(screen.getByRole("group", { name: "供应商图例" })).toBeTruthy();
  });

  it("renders month buckets", () => {
    render(
      <UsageProviderCacheRateTrendChart
        rows={[{ ...sampleRow, granularity: "month", day: "2026-02" }]}
      />
    );
    expect(screen.getByTestId("x-axis").getAttribute("data-ticks")).toBe("2026-02");
  });

  it("renders year buckets", () => {
    render(
      <UsageProviderCacheRateTrendChart
        rows={[{ ...sampleRow, granularity: "year", day: "2026" }]}
      />
    );
    expect(screen.getByTestId("x-axis").getAttribute("data-ticks")).toBe("2026");
  });
});
