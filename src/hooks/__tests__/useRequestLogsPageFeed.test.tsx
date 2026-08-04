import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { gatewayEventNames } from "../../constants/gatewayEvents";
import {
  isRequestLogSnapshotExpiredError,
  useActiveRequestLogsSnapshotQuery,
  useRequestLogsSnapshotPageAllQuery,
} from "../../query/requestLogs";
import { subscribeGatewayEvent } from "../../services/gateway/gatewayEventBus";
import type { RequestLogPageFilters } from "../../services/gateway/requestLogs";
import { useDocumentVisibility } from "../useDocumentVisibility";
import { useRequestLogsPageFeed } from "../useRequestLogsPageFeed";
import { useWindowForeground } from "../useWindowForeground";

vi.mock("../../query/requestLogs", () => ({
  isRequestLogSnapshotExpiredError: vi.fn(() => false),
  useActiveRequestLogsSnapshotQuery: vi.fn(),
  useRequestLogsSnapshotPageAllQuery: vi.fn(),
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
  errorScope: "all",
  createdAtMsFrom: null,
  createdAtMsTo: null,
};

function snapshotPage(items: unknown[] = []) {
  return {
    items,
    snapshotId: "snapshot-1",
    totalCount: items.length,
    totalPages: 1,
    page: 1,
    pageSize: 50,
    expiresAtMs: 1,
  };
}

function mockQueries() {
  const pageRefetch = vi.fn().mockResolvedValue({ data: snapshotPage() });
  const activeRefetch = vi.fn().mockResolvedValue({ data: [] });
  vi.mocked(useRequestLogsSnapshotPageAllQuery).mockReturnValue({
    data: snapshotPage(),
    isLoading: false,
    isFetching: false,
    isPlaceholderData: false,
    error: null,
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
    vi.mocked(isRequestLogSnapshotExpiredError).mockReturnValue(false);
    vi.mocked(useDocumentVisibility).mockReturnValue(true);
    vi.mocked(subscribeGatewayEvent).mockReturnValue({
      ready: Promise.resolve(),
      unsubscribe: vi.fn(),
    });
  });

  it("manually refreshes the persisted snapshot page and active snapshot together", async () => {
    const { activeRefetch, pageRefetch } = mockQueries();
    const { result } = renderHook(() =>
      useRequestLogsPageFeed({
        filters: FILTERS,
        snapshotId: "snapshot-history",
        page: 2,
        snapshotRevision: 0,
        limit: 100,
        onRefreshSnapshot: vi.fn(),
      })
    );

    await act(async () => {
      await result.current.refreshRequestLogs();
    });

    expect(pageRefetch).toHaveBeenCalledTimes(1);
    expect(activeRefetch).toHaveBeenCalledTimes(1);
    expect(useRequestLogsSnapshotPageAllQuery).toHaveBeenCalledWith(
      FILTERS,
      "snapshot-history",
      2,
      100,
      0,
      { enabled: true }
    );
  });

  it("hides placeholder rows and snapshot metadata while a replacement page is loading", () => {
    const { activeRefetch, pageRefetch } = mockQueries();
    const oldItem = { id: 1 };
    vi.mocked(useRequestLogsSnapshotPageAllQuery).mockReturnValue({
      data: snapshotPage([oldItem]),
      isLoading: false,
      isFetching: false,
      isPlaceholderData: false,
      error: null,
      refetch: pageRefetch,
    } as any);
    const { result, rerender } = renderHook(
      ({ page }) =>
        useRequestLogsPageFeed({
          filters: FILTERS,
          snapshotId: "snapshot-1",
          page,
          snapshotRevision: 0,
          limit: 50,
          onRefreshSnapshot: vi.fn(),
        }),
      { initialProps: { page: 1 } }
    );

    expect(result.current.requestLogs).toEqual([oldItem]);
    expect(result.current.snapshotId).toBe("snapshot-1");

    vi.mocked(useRequestLogsSnapshotPageAllQuery).mockReturnValue({
      data: snapshotPage([oldItem]),
      isLoading: false,
      isFetching: true,
      isPlaceholderData: true,
      error: null,
      refetch: pageRefetch,
    } as any);
    rerender({ page: 2 });

    expect(result.current.requestLogs).toEqual([]);
    expect(result.current.snapshotId).toBeNull();
    expect(result.current.totalPages).toBeNull();
    expect(result.current.requestLogsLoading).toBe(true);
    expect(result.current.requestLogsRefreshing).toBe(false);
    expect(result.current.requestLogsAvailable).toBeNull();
    expect(activeRefetch).not.toHaveBeenCalled();
  });

  it("refreshes the first snapshot page on completion signals", async () => {
    vi.useFakeTimers();
    const { activeRefetch, pageRefetch } = mockQueries();
    const refreshSnapshot = vi.fn();
    let eventHandler: ((payload: unknown) => void) | null = null;
    vi.mocked(subscribeGatewayEvent).mockImplementation((event, handler) => {
      expect(event).toBe(gatewayEventNames.requestSignal);
      eventHandler = handler;
      return { ready: Promise.resolve(), unsubscribe: vi.fn() };
    });

    renderHook(() =>
      useRequestLogsPageFeed({
        filters: FILTERS,
        snapshotId: "snapshot-1",
        page: 1,
        snapshotRevision: 0,
        limit: 50,
        liveUpdatesEnabled: true,
        liveUpdateWindowMs: 300,
        onRefreshSnapshot: refreshSnapshot,
      })
    );

    act(() => {
      eventHandler?.({ trace_id: "trace-start", cli_key: "codex", phase: "start", ts: 1 });
    });
    await act(async () => {
      await vi.runOnlyPendingTimersAsync();
    });
    expect(pageRefetch).not.toHaveBeenCalled();
    expect(activeRefetch).toHaveBeenCalledTimes(1);

    act(() => {
      eventHandler?.({ trace_id: "trace-complete", cli_key: "codex", phase: "complete", ts: 2 });
    });
    await act(async () => {
      await vi.runOnlyPendingTimersAsync();
    });
    expect(pageRefetch).not.toHaveBeenCalled();
    expect(refreshSnapshot).toHaveBeenCalledTimes(1);
    expect(activeRefetch).toHaveBeenCalledTimes(2);
  });

  it("keeps active snapshots live on history pages without refreshing persisted rows", async () => {
    vi.useFakeTimers();
    const { activeRefetch, pageRefetch } = mockQueries();
    const refreshSnapshot = vi.fn();
    let eventHandler: ((payload: unknown) => void) | null = null;
    vi.mocked(subscribeGatewayEvent).mockImplementation((_event, handler) => {
      eventHandler = handler;
      return { ready: Promise.resolve(), unsubscribe: vi.fn() };
    });

    renderHook(() =>
      useRequestLogsPageFeed({
        filters: FILTERS,
        snapshotId: "snapshot-history",
        page: 2,
        snapshotRevision: 0,
        limit: 50,
        liveUpdatesEnabled: true,
        liveUpdateWindowMs: 300,
        onRefreshSnapshot: refreshSnapshot,
      })
    );

    act(() => {
      eventHandler?.({ trace_id: "trace-complete", cli_key: "codex", phase: "complete", ts: 1 });
    });
    await act(async () => {
      await vi.runOnlyPendingTimersAsync();
    });

    expect(activeRefetch).toHaveBeenCalledTimes(1);
    expect(pageRefetch).not.toHaveBeenCalled();
    expect(refreshSnapshot).not.toHaveBeenCalled();
  });

  it("keeps foreground refresh independent from live signal updates", async () => {
    const latest = mockQueries();
    const latestRefreshSnapshot = vi.fn();
    const latestRender = renderHook(() =>
      useRequestLogsPageFeed({
        filters: FILTERS,
        snapshotId: "snapshot-1",
        page: 1,
        snapshotRevision: 0,
        limit: 50,
        refreshOnForeground: true,
        onRefreshSnapshot: latestRefreshSnapshot,
      })
    );
    const latestForeground = latestForegroundOptions();

    await act(async () => {
      latestForeground?.onForeground();
      await Promise.resolve();
    });
    expect(latest.pageRefetch).not.toHaveBeenCalled();
    expect(latest.activeRefetch).toHaveBeenCalledTimes(1);
    expect(latestRefreshSnapshot).toHaveBeenCalledTimes(1);
    latestRender.unmount();

    const history = mockQueries();
    const historyRefreshSnapshot = vi.fn();
    renderHook(() =>
      useRequestLogsPageFeed({
        filters: FILTERS,
        snapshotId: "snapshot-history",
        page: 2,
        snapshotRevision: 0,
        limit: 50,
        refreshOnForeground: true,
        onRefreshSnapshot: historyRefreshSnapshot,
      })
    );
    const historyForeground = latestForegroundOptions();

    await act(async () => {
      historyForeground?.onForeground();
      await Promise.resolve();
    });
    expect(history.pageRefetch).not.toHaveBeenCalled();
    expect(history.activeRefetch).toHaveBeenCalledTimes(1);
    expect(historyRefreshSnapshot).not.toHaveBeenCalled();
  });

  it("surfaces snapshot expiration for the page to rebuild", () => {
    mockQueries();
    vi.mocked(isRequestLogSnapshotExpiredError).mockReturnValue(true);

    const { result } = renderHook(() =>
      useRequestLogsPageFeed({
        filters: FILTERS,
        snapshotId: "snapshot-1",
        page: 1,
        snapshotRevision: 0,
        limit: 50,
        onRefreshSnapshot: vi.fn(),
      })
    );

    expect(result.current.requestLogsSnapshotExpired).toBe(true);
  });
});
