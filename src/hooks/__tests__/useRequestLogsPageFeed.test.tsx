import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { gatewayEventNames } from "../../constants/gatewayEvents";
import {
  useActiveRequestLogsSnapshotQuery,
  useRequestLogsPageAllQuery,
} from "../../query/requestLogs";
import { subscribeGatewayEvent } from "../../services/gateway/gatewayEventBus";
import type { RequestLogPageFilters } from "../../services/gateway/requestLogs";
import { useDocumentVisibility } from "../useDocumentVisibility";
import { useRequestLogsPageFeed } from "../useRequestLogsPageFeed";
import { useWindowForeground } from "../useWindowForeground";

vi.mock("../../query/requestLogs", () => ({
  useActiveRequestLogsSnapshotQuery: vi.fn(),
  useRequestLogsPageAllQuery: vi.fn(),
}));

vi.mock("../../services/gateway/gatewayEventBus", () => ({
  subscribeGatewayEvent: vi.fn(),
}));

vi.mock("../useDocumentVisibility", () => ({
  useDocumentVisibility: vi.fn(),
}));

vi.mock("../useWindowForeground", () => ({
  useWindowForeground: vi.fn(),
}));

const FILTERS: RequestLogPageFilters = {
  cliKey: null,
  status: null,
  errorCodeContains: null,
  methodPathContains: null,
};

function mockQueries() {
  const pageRefetch = vi.fn().mockResolvedValue({ data: { items: [], nextCursor: null } });
  const activeRefetch = vi.fn().mockResolvedValue({ data: [] });
  vi.mocked(useRequestLogsPageAllQuery).mockReturnValue({
    data: { items: [], nextCursor: null },
    isLoading: false,
    isFetching: false,
    isPlaceholderData: false,
    refetch: pageRefetch,
  } as any);
  vi.mocked(useActiveRequestLogsSnapshotQuery).mockReturnValue({
    data: [],
    isLoading: false,
    isFetching: false,
    isError: false,
    refetch: activeRefetch,
  } as any);
  return { activeRefetch, pageRefetch };
}

function latestForegroundOptions() {
  const calls = vi.mocked(useWindowForeground).mock.calls;
  return calls[calls.length - 1]?.[0];
}

describe("hooks/useRequestLogsPageFeed", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useRealTimers();
    vi.mocked(useDocumentVisibility).mockReturnValue(true);
    vi.mocked(subscribeGatewayEvent).mockReturnValue({
      ready: Promise.resolve(),
      unsubscribe: vi.fn(),
    });
  });

  it("manually refreshes the current persisted page and active snapshot together", async () => {
    const { activeRefetch, pageRefetch } = mockQueries();
    const { result } = renderHook(() =>
      useRequestLogsPageFeed({
        filters: FILTERS,
        cursor: "opaque-history",
        limit: 100,
      })
    );

    await act(async () => {
      await result.current.refreshRequestLogs();
    });

    expect(pageRefetch).toHaveBeenCalledTimes(1);
    expect(activeRefetch).toHaveBeenCalledTimes(1);
    expect(useRequestLogsPageAllQuery).toHaveBeenCalledWith(FILTERS, "opaque-history", 100, {
      enabled: true,
    });
  });

  it("hides placeholder rows and cursors while a replacement page is loading", () => {
    const { activeRefetch, pageRefetch } = mockQueries();
    const oldItem = { id: 1 };
    vi.mocked(useRequestLogsPageAllQuery).mockReturnValue({
      data: { items: [oldItem], nextCursor: "old-next" },
      isLoading: false,
      isFetching: false,
      isPlaceholderData: false,
      refetch: pageRefetch,
    } as any);
    const { result, rerender } = renderHook(
      ({ cursor }) =>
        useRequestLogsPageFeed({
          filters: FILTERS,
          cursor,
          limit: 50,
        }),
      { initialProps: { cursor: null as string | null } }
    );

    expect(result.current.requestLogs).toEqual([oldItem]);
    expect(result.current.nextCursor).toBe("old-next");

    vi.mocked(useRequestLogsPageAllQuery).mockReturnValue({
      data: { items: [oldItem], nextCursor: "old-next" },
      isLoading: false,
      isFetching: true,
      isPlaceholderData: true,
      refetch: pageRefetch,
    } as any);
    rerender({ cursor: "opaque-history" });

    expect(result.current.requestLogs).toEqual([]);
    expect(result.current.nextCursor).toBeNull();
    expect(result.current.requestLogsLoading).toBe(true);
    expect(result.current.requestLogsRefreshing).toBe(false);
    expect(result.current.requestLogsAvailable).toBeNull();
    expect(activeRefetch).not.toHaveBeenCalled();
  });

  it("refreshes the latest page on completion signals", async () => {
    vi.useFakeTimers();
    const { activeRefetch, pageRefetch } = mockQueries();
    let eventHandler: ((payload: unknown) => void) | null = null;
    vi.mocked(subscribeGatewayEvent).mockImplementation((event, handler) => {
      expect(event).toBe(gatewayEventNames.requestSignal);
      eventHandler = handler;
      return { ready: Promise.resolve(), unsubscribe: vi.fn() };
    });

    renderHook(() =>
      useRequestLogsPageFeed({
        filters: FILTERS,
        cursor: null,
        limit: 50,
        liveUpdatesEnabled: true,
        liveUpdateWindowMs: 300,
      })
    );

    act(() => {
      eventHandler?.({
        trace_id: "trace-start",
        cli_key: "codex",
        phase: "start",
        ts: 1,
      });
    });
    await act(async () => {
      await vi.runOnlyPendingTimersAsync();
    });
    expect(pageRefetch).not.toHaveBeenCalled();
    expect(activeRefetch).toHaveBeenCalledTimes(1);

    act(() => {
      eventHandler?.({
        trace_id: "trace-complete",
        cli_key: "codex",
        phase: "complete",
        ts: 2,
      });
    });
    expect(pageRefetch).not.toHaveBeenCalled();

    await act(async () => {
      await vi.runOnlyPendingTimersAsync();
    });
    expect(pageRefetch).toHaveBeenCalledTimes(1);
    expect(activeRefetch).toHaveBeenCalledTimes(3);
  });

  it("keeps active snapshots live on history pages without refreshing persisted rows", async () => {
    vi.useFakeTimers();
    const { activeRefetch, pageRefetch } = mockQueries();
    let eventHandler: ((payload: unknown) => void) | null = null;
    vi.mocked(subscribeGatewayEvent).mockImplementation((_event, handler) => {
      eventHandler = handler;
      return { ready: Promise.resolve(), unsubscribe: vi.fn() };
    });

    renderHook(() =>
      useRequestLogsPageFeed({
        filters: FILTERS,
        cursor: "opaque-history",
        limit: 50,
        liveUpdatesEnabled: true,
        liveUpdateWindowMs: 300,
      })
    );

    act(() => {
      eventHandler?.({
        trace_id: "trace-complete",
        cli_key: "codex",
        phase: "complete",
        ts: 1,
      });
    });
    await act(async () => {
      await vi.runOnlyPendingTimersAsync();
    });

    expect(activeRefetch).toHaveBeenCalledTimes(1);
    expect(pageRefetch).not.toHaveBeenCalled();
  });

  it("keeps foreground refresh independent from live signal updates", async () => {
    const latest = mockQueries();
    const latestRender = renderHook(() =>
      useRequestLogsPageFeed({
        filters: FILTERS,
        cursor: null,
        limit: 50,
        refreshOnForeground: true,
      })
    );
    const latestForeground = latestForegroundOptions();

    await act(async () => {
      latestForeground?.onForeground();
      await Promise.resolve();
    });
    expect(latest.pageRefetch).toHaveBeenCalledTimes(1);
    expect(latest.activeRefetch).toHaveBeenCalledTimes(1);
    latestRender.unmount();

    const history = mockQueries();
    renderHook(() =>
      useRequestLogsPageFeed({
        filters: FILTERS,
        cursor: "opaque-history",
        limit: 50,
        refreshOnForeground: true,
      })
    );
    const historyForeground = latestForegroundOptions();

    await act(async () => {
      historyForeground?.onForeground();
      await Promise.resolve();
    });
    expect(history.pageRefetch).not.toHaveBeenCalled();
    expect(history.activeRefetch).toHaveBeenCalledTimes(1);
  });
});
