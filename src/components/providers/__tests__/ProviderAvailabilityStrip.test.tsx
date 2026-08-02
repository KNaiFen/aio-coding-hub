import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import type {
  ProviderAvailabilityState,
  ProviderAvailabilityTimeline,
} from "../../../generated/bindings";
import {
  normalizeProviderAvailabilityTimeline,
  ProviderAvailabilityStrip,
} from "../ProviderAvailabilityStrip";

const START_AT_MS = Date.UTC(2026, 7, 2, 8, 0, 0);
const BUCKET_DURATION_MS = 10 * 60_000;

function makeTimeline(
  states: ProviderAvailabilityState[] = ["healthy", "unhealthy", "no_data"]
): ProviderAvailabilityTimeline {
  return {
    provider_id: 7,
    hours: 6,
    bucket_count: 36,
    bucket_minutes: 10,
    success_count: 3,
    failure_count: 2,
    buckets: Array.from({ length: 36 }, (_, index) => {
      const state = states[index] ?? "no_data";
      return {
        start_at_ms: START_AT_MS + index * BUCKET_DURATION_MS,
        end_at_ms: START_AT_MS + (index + 1) * BUCKET_DURATION_MS,
        success_count: state === "healthy" ? 3 : 0,
        failure_count: state === "unhealthy" ? 2 : 0,
        state,
      };
    }),
  };
}

describe("components/providers/ProviderAvailabilityStrip", () => {
  it("renders 36 semantic cells and exposes exact counters in the tooltip", async () => {
    const user = userEvent.setup();
    const timeline = makeTimeline();
    render(<ProviderAvailabilityStrip timeline={timeline} providerName="大春" />);

    expect(screen.getByLabelText("大春 过去 6 小时可用性")).toBeInTheDocument();
    expect(screen.getByText("成功 3 · 失败 2")).toBeInTheDocument();
    const cells = screen.getAllByRole("img");
    expect(cells).toHaveLength(36);
    expect(cells[0]).toHaveClass("bg-success");
    expect(cells[0].className).toContain("dark:ring");
    expect(cells[1]).toHaveClass("bg-danger");
    expect(cells[2]).toHaveClass("bg-surface-muted");

    await user.hover(cells[0]);
    const tooltip = await screen.findByRole("tooltip");
    const formatter = new Intl.DateTimeFormat("zh-CN", {
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    });
    expect(tooltip).toHaveTextContent(
      `${formatter.format(new Date(START_AT_MS))} - ${formatter.format(
        new Date(START_AT_MS + BUCKET_DURATION_MS)
      )}`
    );
    expect(tooltip).toHaveTextContent("高可用 · 成功 3 · 失败 0");
    expect(tooltip).toHaveTextContent("成功率 100.0%");
  });

  it("rejects malformed timelines instead of rendering partial or unsafe data", () => {
    const unsupportedState = makeTimeline();
    unsupportedState.buckets[0].state = "future" as ProviderAvailabilityState;
    expect(normalizeProviderAvailabilityTimeline(unsupportedState)).toBeNull();

    const unsafeTimestamp = makeTimeline();
    unsafeTimestamp.buckets[0].start_at_ms = Number.MAX_SAFE_INTEGER;
    expect(normalizeProviderAvailabilityTimeline(unsafeTimestamp)).toBeNull();

    const missingBuckets = makeTimeline();
    missingBuckets.buckets = null as unknown as ProviderAvailabilityTimeline["buckets"];
    expect(normalizeProviderAvailabilityTimeline(missingBuckets)).toBeNull();

    const discontinuous = makeTimeline();
    discontinuous.buckets[1].start_at_ms += 1;
    expect(normalizeProviderAvailabilityTimeline(discontinuous)).toBeNull();

    const partial = makeTimeline();
    partial.buckets.pop();
    const { container } = render(
      <ProviderAvailabilityStrip timeline={partial} providerName="大春" />
    );
    expect(container).toBeEmptyDOMElement();
  });
});
