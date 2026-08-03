import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { UsageProviderMetricTrendRowV1 } from "../../../services/usage/usage";

vi.mock("../../../components/UsageProviderMetricsTrendChart", () => ({
  UsageProviderMetricsTrendChart: ({ metric, rows }: { metric: string; rows: unknown[] }) => (
    <div data-testid="metrics-chart" data-metric={metric} data-rows={rows.length} />
  ),
}));

import { MetricsTrendBody } from "../UsageDataPanelBodies";

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
  duration_samples: 10,
  ttfb_samples: 8,
  output_rate_samples: 7,
  avg_duration_ms: 1200,
  avg_ttfb_ms: 300,
  avg_output_tokens_per_second: 42.5,
  requests_success: 10,
};

function renderBody(overrides: Partial<Parameters<typeof MetricsTrendBody>[0]> = {}) {
  return render(
    <MetricsTrendBody
      metricsTrendLoading={false}
      metricsTrendRows={[]}
      errorText={null}
      customPending={false}
      period="weekly"
      customApplied={null}
      {...overrides}
    />
  );
}

describe("pages/usage/MetricsTrendBody", () => {
  it("shows skeleton while loading with no data yet", () => {
    const { container } = renderBody({ metricsTrendLoading: true });
    expect(container.querySelector(".animate-pulse")).not.toBeNull();
    expect(screen.queryByTestId("metrics-chart")).toBeNull();
    // metric toggle stays visible in every state
    expect(screen.getByRole("tab", { name: "耗时" })).toBeTruthy();
  });

  it("shows retry hint when empty due to error", () => {
    renderBody({ errorText: "boom" });
    expect(screen.getByText(/加载失败/)).toBeTruthy();
    expect(screen.queryByTestId("metrics-chart")).toBeNull();
  });

  it("shows apply hint when custom range is pending", () => {
    renderBody({ customPending: true });
    expect(screen.getByText(/自定义范围/)).toBeTruthy();
  });

  it("shows empty placeholder when no data", () => {
    renderBody();
    expect(screen.getByText("暂无可展示的指标数据。")).toBeTruthy();
  });

  it("keeps chart visible while refetching with existing rows", () => {
    const { container } = renderBody({
      metricsTrendLoading: true,
      metricsTrendRows: [sampleRow],
    });
    expect(screen.getByTestId("metrics-chart")).toBeTruthy();
    expect(container.querySelector(".animate-pulse")).toBeNull();
  });

  it("switches metric via tabs", () => {
    renderBody({ metricsTrendRows: [sampleRow] });

    const chart = screen.getByTestId("metrics-chart");
    expect(chart.getAttribute("data-metric")).toBe("duration");
    expect(chart.getAttribute("data-rows")).toBe("1");
    fireEvent.click(screen.getByRole("tab", { name: "TTFB" }));
    expect(screen.getByTestId("metrics-chart").getAttribute("data-metric")).toBe("ttfb");

    fireEvent.click(screen.getByRole("tab", { name: "输出速率" }));
    expect(screen.getByTestId("metrics-chart").getAttribute("data-metric")).toBe("rate");
  });

  it("shows a metric-level empty state when the selected metric has no samples", () => {
    renderBody({ metricsTrendRows: [{ ...sampleRow, avg_ttfb_ms: null, ttfb_samples: 0 }] });

    fireEvent.click(screen.getByRole("tab", { name: "TTFB" }));
    expect(screen.getByText("当前指标暂无有效样本。")).toBeTruthy();
    expect(screen.queryByTestId("metrics-chart")).toBeNull();
  });
});
