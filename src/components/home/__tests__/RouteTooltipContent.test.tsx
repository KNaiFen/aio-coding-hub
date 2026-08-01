import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { RouteTooltipContent } from "../RouteTooltipContent";
import type { RequestLogRouteHop } from "../../../services/gateway/requestLogs";
import { createRequestLogRouteHop } from "../../../services/gateway/requestLogFixtures";

function makeHop(overrides: Partial<RequestLogRouteHop> = {}): RequestLogRouteHop {
  return createRequestLogRouteHop({
    provider_name: "TestProvider",
    ...overrides,
  });
}

describe("RouteTooltipContent", () => {
  it("returns null for empty hops", () => {
    const { container } = render(<RouteTooltipContent hops={[]} finalStatus={200} />);
    expect(container.innerHTML).toBe("");
  });

  it("renders a successful hop", () => {
    render(<RouteTooltipContent hops={[makeHop()]} finalStatus={200} />);
    expect(screen.getAllByText("TestProvider").length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText("成功")).toBeInTheDocument();
  });

  it("renders a failed hop with error_code", () => {
    render(
      <RouteTooltipContent
        hops={[makeHop({ ok: false, error_code: "TIMEOUT" })]}
        finalStatus={500}
      />
    );
    expect(screen.getByText("失败")).toBeInTheDocument();
  });

  it("renders a skipped hop", () => {
    render(
      <RouteTooltipContent hops={[makeHop({ ok: false, skipped: true })]} finalStatus={null} />
    );
    expect(screen.getByText("已跳过")).toBeInTheDocument();
    expect(screen.getByText("未发送")).toBeInTheDocument();
  });

  it("renders the full summary", () => {
    render(
      <RouteTooltipContent
        hops={[makeHop()]}
        finalStatus={200}
        summary="跳过 2 个候选，实际请求 1 次"
      />
    );
    expect(screen.getByText("跳过 2 个候选，实际请求 1 次")).toBeInTheDocument();
  });

  it("renders hop with attempts > 1", () => {
    render(<RouteTooltipContent hops={[makeHop({ ok: true, attempts: 3 })]} finalStatus={200} />);
    expect(screen.getByText("成功（重试 2 次）")).toBeInTheDocument();
  });

  it("renders skipped hop with attempts > 1", () => {
    render(
      <RouteTooltipContent
        hops={[makeHop({ ok: false, skipped: true, attempts: 2 })]}
        finalStatus={null}
      />
    );
    expect(screen.getByText("已跳过")).toBeInTheDocument();
    expect(screen.queryByText("已跳过 2 次")).not.toBeInTheDocument();
  });

  it("renders provider name as 未知 for empty name", () => {
    render(<RouteTooltipContent hops={[makeHop({ provider_name: "" })]} finalStatus={200} />);
    // Two instances: one in chain view and one in hop row
    expect(screen.getAllByText("未知").length).toBeGreaterThanOrEqual(1);
  });

  it("deduplicates known skip decisions and gateway reasons", () => {
    const { rerender } = render(
      <RouteTooltipContent
        hops={[
          makeHop({
            ok: false,
            skipped: true,
            error_code: "GW_PROVIDER_RATE_LIMITED",
            decision: "skip",
            reason: "provider skipped by rate limit",
          }),
        ]}
        finalStatus={null}
      />
    );
    expect(screen.getByText("供应商限额")).toBeInTheDocument();
    expect(screen.queryByText("skip")).not.toBeInTheDocument();
    expect(screen.queryByText("provider skipped by rate limit")).not.toBeInTheDocument();

    rerender(
      <RouteTooltipContent
        hops={[
          makeHop({
            ok: false,
            skipped: true,
            error_code: "GW_PROVIDER_CIRCUIT_OPEN",
            decision: "skip",
            reason: "provider skipped by circuit breaker (open)",
          }),
        ]}
        finalStatus={null}
      />
    );
    expect(screen.getByText("供应商熔断")).toBeInTheDocument();
    expect(
      screen.queryByText("provider skipped by circuit breaker (open)")
    ).not.toBeInTheDocument();
  });

  it("preserves unknown future decisions and reasons on a wrapping detail line", () => {
    const { container } = render(
      <RouteTooltipContent
        hops={[
          makeHop({
            ok: false,
            decision: "future_route_action",
            reason: "future reason with a very-long-unbroken-value-that-must-wrap",
          }),
        ]}
        finalStatus={500}
      />
    );
    expect(screen.getByText("future_route_action")).toBeInTheDocument();
    expect(
      screen.getByText("future reason with a very-long-unbroken-value-that-must-wrap")
    ).toHaveClass("break-words");
    expect(container.querySelector(".text-white")).not.toBeInTheDocument();
  });

  it("hides a status-only raw reason while retaining the localized decision", () => {
    render(
      <RouteTooltipContent
        hops={[
          makeHop({
            ok: false,
            status: 500,
            decision: "failover",
            reason: "status=500",
          }),
        ]}
        finalStatus={500}
      />
    );
    expect(screen.getByText("切换供应商")).toBeInTheDocument();
    expect(screen.queryByText("status=500")).not.toBeInTheDocument();
  });
});
