import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { dayKeyFromLocalDate } from "../../../utils/dateKeys";
import { HomeUsageSection } from "../HomeUsageSection";

const tokensChartSpy = vi.fn();

vi.mock("../../UsageTokensChart", () => ({
  UsageTokensChart: (props: unknown) => {
    tokensChartSpy(props);
    return <div>tokens-chart</div>;
  },
}));

describe("components/home/HomeUsageSection", () => {
  beforeEach(() => {
    tokensChartSpy.mockClear();
  });

  it("shows today's token total in the compact usage card header", () => {
    const today = dayKeyFromLocalDate(new Date());

    render(
      <HomeUsageSection
        usageHeatmapRows={[
          {
            day: today,
            hour: 9,
            requests_total: 1,
            requests_with_usage: 1,
            requests_success: 1,
            requests_failed: 0,
            total_tokens: 600_000,
          },
          {
            day: today,
            hour: 14,
            requests_total: 1,
            requests_with_usage: 1,
            requests_success: 1,
            requests_failed: 0,
            total_tokens: 900_000,
          },
          {
            day: "2000-01-01",
            hour: 8,
            requests_total: 1,
            requests_with_usage: 1,
            requests_success: 1,
            requests_failed: 0,
            total_tokens: 5_000_000,
          },
        ]}
        usageHeatmapLoading={false}
      />
    );

    expect(screen.getByText("今日用量")).toBeInTheDocument();
    expect(screen.getByText("1.5M")).toBeInTheDocument();
    expect(screen.getByText("tokens-chart")).toBeInTheDocument();
  });

  it("renders preview usage data when dev preview is enabled and rows are empty", () => {
    render(
      <HomeUsageSection
        devPreviewEnabled={true}
        usageHeatmapRows={[]}
        usageHeatmapLoading={false}
      />
    );

    expect(screen.getByText("tokens-chart")).toBeInTheDocument();
    expect(screen.getByText("今日用量")).toBeInTheDocument();
    expect(screen.getByText(/\d+(\.\d)?M/)).toBeInTheDocument();
  });

  it("passes the configured usage window days to the chart", () => {
    render(
      <HomeUsageSection usageWindowDays={30} usageHeatmapRows={[]} usageHeatmapLoading={false} />
    );

    const lastCall = tokensChartSpy.mock.calls[tokensChartSpy.mock.calls.length - 1];
    expect(lastCall?.[0]).toEqual(expect.objectContaining({ days: 30 }));
  });
});
