import { act, fireEvent, render, screen } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import type { ReactElement } from "react";
import { MemoryRouter } from "react-router-dom";
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { useRequestLogsPageFeed } from "../../hooks/useRequestLogsPageFeed";
import type { TraceSession } from "../../services/gateway/traceStore";
import { createTestQueryClient } from "../../test/utils/reactQuery";
import { clearTauriRuntime, setTauriRuntime } from "../../test/utils/tauriRuntime";
import { LOGS_PAGE_SIZE_STORAGE_KEY, LogsPage } from "../LogsPage";

const traceStoreState = vi.hoisted(() => ({
  traces: [] as TraceSession[],
}));

const originalLocalStorage = window.localStorage;
const originalLocalStorageDescriptor = Object.getOwnPropertyDescriptor(window, "localStorage");
const localStorageEntries = new Map<string, string>();
const testLocalStorage: Storage = {
  get length() {
    return localStorageEntries.size;
  },
  clear() {
    localStorageEntries.clear();
  },
  getItem(key) {
    return localStorageEntries.get(key) ?? null;
  },
  key(index) {
    return Array.from(localStorageEntries.keys())[index] ?? null;
  },
  removeItem(key) {
    localStorageEntries.delete(key);
  },
  setItem(key, value) {
    localStorageEntries.set(key, String(value));
  },
};

vi.mock("../../components/home/HomeRequestLogsPanel", () => ({
  HomeRequestLogsPanel: ({
    requestLogs,
    activeRequests,
    summaryTextOverride,
    emptyStateTitle,
    traces,
    requestLogOrder,
    onRefreshRequestLogs,
  }: {
    requestLogs: Array<{ id: number }>;
    activeRequests: Array<{ trace_id: string }>;
    summaryTextOverride?: string;
    emptyStateTitle?: string;
    traces: TraceSession[];
    requestLogOrder?: string;
    onRefreshRequestLogs: () => void;
  }) => (
    <div data-testid="home-request-logs-panel">
      <span data-testid="page-log-ids">{requestLogs.map((log) => log.id).join(",")}</span>
      <span data-testid="active-count">{activeRequests.length}</span>
      <span data-testid="summary">{summaryTextOverride ?? ""}</span>
      <span data-testid="empty-title">{emptyStateTitle ?? ""}</span>
      <span data-testid="trace-ids">{traces.map((trace) => trace.trace_id).join(",")}</span>
      <span data-testid="request-log-order">{requestLogOrder ?? ""}</span>
      <button type="button" onClick={onRefreshRequestLogs}>
        刷新当前页
      </button>
    </div>
  ),
}));

vi.mock("../../components/home/RequestLogDetailDialog", () => ({
  RequestLogDetailDialog: () => <div data-testid="request-log-detail-dialog" />,
}));

vi.mock("../../hooks/useRequestLogsPageFeed", () => ({
  useRequestLogsPageFeed: vi.fn(),
}));

vi.mock("../../services/gateway/traceStore", () => ({
  useTraceStore: () => ({ traces: traceStoreState.traces }),
}));

function renderWithProviders(element: ReactElement) {
  const client = createTestQueryClient();
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter>{element}</MemoryRouter>
    </QueryClientProvider>
  );
}

function createTrace(traceId: string, overrides: Partial<TraceSession> = {}): TraceSession {
  return {
    trace_id: traceId,
    cli_key: "claude",
    method: "POST",
    path: "/v1/messages",
    query: null,
    requested_model: "test-model",
    first_seen_ms: 1_000,
    last_seen_ms: 2_000,
    attempts: [],
    ...overrides,
  } as TraceSession;
}

function mockPageFeed(overrides: Record<string, unknown> = {}) {
  const refreshRequestLogs = vi.fn().mockResolvedValue({ data: null });
  vi.mocked(useRequestLogsPageFeed).mockImplementation((options) => {
    const capturedSnapshotId =
      options.snapshotId ?? (options.snapshotRevision === 0 ? "snapshot-1" : null);
    return {
      requestLogs: [],
      snapshotId: capturedSnapshotId,
      totalCount: 120,
      totalPages: 3,
      page: options.page,
      activeRequests: [],
      activeRequestsAvailable: true,
      requestLogsLoading: false,
      requestLogsRefreshing: false,
      requestLogsPageFetching: false,
      requestLogsAvailable: true,
      requestLogsSnapshotExpired: false,
      refreshActiveRequests: vi.fn(),
      refreshRequestLogs,
      ...overrides,
    } as any;
  });
  return refreshRequestLogs;
}

function latestFeedOptions() {
  const calls = vi.mocked(useRequestLogsPageFeed).mock.calls;
  return calls[calls.length - 1]?.[0];
}

describe("pages/LogsPage", () => {
  beforeAll(() => {
    Object.defineProperty(window, "localStorage", {
      configurable: true,
      value: testLocalStorage,
    });
  });

  afterAll(() => {
    if (originalLocalStorageDescriptor) {
      Object.defineProperty(window, "localStorage", originalLocalStorageDescriptor);
      return;
    }
    Object.defineProperty(window, "localStorage", {
      configurable: true,
      value: originalLocalStorage,
    });
  });

  beforeEach(() => {
    vi.clearAllMocks();
    vi.useRealTimers();
    window.localStorage.clear();
    setTauriRuntime();
    traceStoreState.traces = [];
    mockPageFeed();
  });

  afterEach(() => {
    vi.useRealTimers();
    window.localStorage.clear();
    clearTauriRuntime();
    traceStoreState.traces = [];
  });

  it("disables request-log filters when request logs are unavailable", () => {
    mockPageFeed({ requestLogsAvailable: false });
    renderWithProviders(<LogsPage />);

    expect(screen.getByRole("switch")).toBeDisabled();
    expect(screen.getByPlaceholderText("例：499 / 524 / !200 / >=400")).toBeDisabled();
    expect(screen.getByPlaceholderText("例：GW_UPSTREAM_TIMEOUT")).toBeDisabled();
    expect(screen.getByPlaceholderText("例：/v1/messages")).toBeDisabled();
    expect(screen.getByRole("tab", { name: "Codex" })).toBeDisabled();
    expect(screen.getByRole("tab", { name: "全部报错" })).toBeDisabled();
    expect(screen.getByRole("button", { name: /全部时间/ })).toBeDisabled();
  });

  it("shows status validation immediately and does not apply an invalid expression", () => {
    renderWithProviders(<LogsPage />);

    fireEvent.change(screen.getByPlaceholderText("例：499 / 524 / !200 / >=400"), {
      target: { value: "nope" },
    });

    expect(screen.getByText(/表达式不合法/)).toBeInTheDocument();
    expect(latestFeedOptions()?.filters.status).toBeNull();
  });

  it("debounces text filters and rebuilds the stable snapshot", () => {
    vi.useFakeTimers();
    renderWithProviders(<LogsPage />);
    const initialRevision = latestFeedOptions()?.snapshotRevision;

    fireEvent.change(screen.getByPlaceholderText("例：/v1/messages"), {
      target: { value: "messages" },
    });
    act(() => vi.advanceTimersByTime(299));
    expect(latestFeedOptions()?.filters.methodPathContains).toBeNull();

    act(() => vi.advanceTimersByTime(1));
    expect(latestFeedOptions()?.filters.methodPathContains).toBe("messages");
    expect(latestFeedOptions()?.snapshotId).toBeNull();
    expect(latestFeedOptions()?.page).toBe(1);
    expect(latestFeedOptions()?.snapshotRevision).toBe((initialRevision ?? 0) + 1);
  });

  it("maps status expressions to the server DTO after the debounce", () => {
    vi.useFakeTimers();
    renderWithProviders(<LogsPage />);

    fireEvent.change(screen.getByPlaceholderText("例：499 / 524 / !200 / >=400"), {
      target: { value: "!200" },
    });
    act(() => vi.advanceTimersByTime(300));

    expect(latestFeedOptions()?.filters.status).toEqual({ op: "neq", value: 200 });
  });

  it("offers direct error scopes without status-code input", () => {
    traceStoreState.traces = [
      createTrace("trace-interrupted", {
        summary: { status: 499, error_code: "GW_STREAM_ABORTED" } as any,
      }),
      createTrace("trace-upstream-error", {
        summary: { status: 503, error_code: "GW_UPSTREAM_TIMEOUT" } as any,
      }),
    ];
    renderWithProviders(<LogsPage />);

    fireEvent.click(screen.getByRole("tab", { name: "全部报错" }));
    expect(latestFeedOptions()?.filters.errorScope).toBe("all_errors");
    expect(latestFeedOptions()?.snapshotId).toBeNull();
    expect(screen.getByTestId("trace-ids")).toHaveTextContent("trace-upstream-error");
    expect(screen.getByTestId("trace-ids")).not.toHaveTextContent("trace-interrupted");

    fireEvent.click(screen.getByRole("tab", { name: "流内错误" }));
    expect(latestFeedOptions()?.filters.errorScope).toBe("stream_internal_error");
  });

  it("applies quick and custom minute-level time ranges", () => {
    vi.useFakeTimers();
    const now = new Date(2026, 7, 1, 10, 45, 37, 500);
    vi.setSystemTime(now);
    renderWithProviders(<LogsPage />);

    fireEvent.click(screen.getByRole("button", { name: /全部时间/ }));
    fireEvent.click(screen.getByRole("button", { name: "1小时内" }));
    const expectedEnd = new Date(2026, 7, 1, 10, 46).getTime();
    expect(latestFeedOptions()?.filters.createdAtMsFrom).toBe(expectedEnd - 60 * 60 * 1000);
    expect(latestFeedOptions()?.filters.createdAtMsTo).toBe(expectedEnd);
    expect(latestFeedOptions()?.filters.createdAtMsTo).toBeGreaterThan(now.getTime());

    vi.setSystemTime(new Date(2026, 7, 1, 23, 59, 37, 500));
    fireEvent.click(screen.getByRole("button", { name: /1小时内/ }));
    fireEvent.click(screen.getByRole("button", { name: "今天" }));
    expect(latestFeedOptions()?.filters.createdAtMsFrom).toBe(new Date(2026, 7, 1, 0, 0).getTime());
    expect(latestFeedOptions()?.filters.createdAtMsTo).toBe(new Date(2026, 7, 2, 0, 0).getTime());

    fireEvent.change(screen.getByLabelText("开始时间"), { target: { value: "2026-08-01T09:15" } });
    fireEvent.change(screen.getByLabelText("结束时间"), { target: { value: "2026-08-01T10:45" } });
    fireEvent.click(screen.getByRole("button", { name: "应用" }));
    expect(latestFeedOptions()?.filters.createdAtMsTo).toBeGreaterThan(
      latestFeedOptions()?.filters.createdAtMsFrom ?? 0
    );

    fireEvent.change(screen.getByLabelText("开始时间"), { target: { value: "2026-08-01T11:00" } });
    fireEvent.change(screen.getByLabelText("结束时间"), { target: { value: "2026-08-01T10:45" } });
    const error = screen.getByRole("alert");
    expect(error).toHaveTextContent("结束时间必须晚于开始时间");
    expect(screen.getByLabelText("开始时间")).toHaveAttribute("aria-invalid", "true");
    expect(screen.getByLabelText("结束时间")).toHaveAttribute(
      "aria-describedby",
      "logs-time-range-error"
    );
  });

  it("keeps null-status active requests for neq while exact status filters hide them", () => {
    vi.useFakeTimers();
    traceStoreState.traces = [
      createTrace("trace-live"),
      createTrace("trace-ok", { summary: { status: 200 } as any }),
      createTrace("trace-error", { summary: { status: 524 } as any }),
    ];
    mockPageFeed({
      activeRequests: [
        { trace_id: "active-live", cli_key: "claude", method: "POST", path: "/v1/messages" },
      ],
    });
    renderWithProviders(<LogsPage />);

    fireEvent.change(screen.getByPlaceholderText("例：499 / 524 / !200 / >=400"), {
      target: { value: "!200" },
    });
    act(() => vi.advanceTimersByTime(300));
    expect(screen.getByTestId("active-count")).toHaveTextContent("1");
    expect(screen.getByTestId("trace-ids")).toHaveTextContent("trace-live,trace-error");

    fireEvent.change(screen.getByPlaceholderText("例：499 / 524 / !200 / >=400"), {
      target: { value: "200" },
    });
    act(() => vi.advanceTimersByTime(300));
    expect(screen.getByTestId("active-count")).toHaveTextContent("0");
    expect(screen.getByTestId("trace-ids")).toHaveTextContent("trace-ok");
  });

  it("navigates by any page number within the captured snapshot and displays totals", () => {
    vi.mocked(useRequestLogsPageFeed).mockImplementation((options) => {
      const rows =
        options.page === 1 ? [{ id: 9 }, { id: 7 }] : options.page === 3 ? [{ id: 1 }] : [];
      return {
        requestLogs: rows,
        snapshotId: options.snapshotId ?? "snapshot-1",
        totalCount: 5,
        totalPages: 3,
        page: options.page,
        activeRequests: [],
        activeRequestsAvailable: true,
        requestLogsLoading: false,
        requestLogsRefreshing: false,
        requestLogsPageFetching: false,
        requestLogsAvailable: true,
        requestLogsSnapshotExpired: false,
        refreshActiveRequests: vi.fn(),
        refreshRequestLogs: vi.fn(),
      } as any;
    });
    renderWithProviders(<LogsPage />);

    expect(screen.getByTestId("page-log-ids")).toHaveTextContent("9,7");
    expect(screen.getByTestId("summary")).toHaveTextContent("第 1 / 3 页 · 共 5 条 · 本页 2 条");
    expect(screen.getByTestId("request-log-order")).toHaveTextContent("source");

    fireEvent.change(screen.getByRole("spinbutton", { name: "跳转页码" }), {
      target: { value: "3" },
    });
    fireEvent.click(screen.getByRole("button", { name: "跳转" }));
    expect(latestFeedOptions()?.page).toBe(3);
    expect(latestFeedOptions()?.snapshotId).toBe("snapshot-1");
    expect(screen.getByTestId("page-log-ids")).toHaveTextContent("1");
    expect(screen.getByRole("button", { name: "下一页" })).toBeDisabled();

    fireEvent.click(screen.getByRole("button", { name: "上一页" }));
    expect(latestFeedOptions()?.page).toBe(2);
  });

  it("resets the snapshot when CLI, page size, or manual refresh changes", () => {
    window.localStorage.setItem(LOGS_PAGE_SIZE_STORAGE_KEY, "100");
    renderWithProviders(<LogsPage />);
    expect(screen.getByRole("combobox", { name: "每页条数" })).toHaveValue("100");
    const initialRevision = latestFeedOptions()?.snapshotRevision ?? 0;

    fireEvent.click(screen.getByRole("tab", { name: "Codex" }));
    expect(latestFeedOptions()?.filters.cliKey).toBe("codex");
    expect(latestFeedOptions()?.snapshotId).toBeNull();
    expect(latestFeedOptions()?.snapshotRevision).toBe(initialRevision + 1);

    fireEvent.change(screen.getByRole("combobox", { name: "每页条数" }), {
      target: { value: "200" },
    });
    expect(latestFeedOptions()?.limit).toBe(200);
    expect(latestFeedOptions()?.snapshotRevision).toBe(initialRevision + 2);
    expect(window.localStorage.getItem(LOGS_PAGE_SIZE_STORAGE_KEY)).toBe("200");

    fireEvent.click(screen.getByRole("button", { name: "刷新并重建分页" }));
    expect(latestFeedOptions()?.snapshotRevision).toBe(initialRevision + 3);
    fireEvent.click(screen.getByRole("button", { name: "刷新当前页" }));
    expect(latestFeedOptions()?.snapshotRevision).toBe(initialRevision + 4);
    act(() => latestFeedOptions()?.onRefreshSnapshot());
    expect(latestFeedOptions()?.snapshotRevision).toBe(initialRevision + 5);
  });

  it("falls back to 50 when the stored page size is unsupported", () => {
    window.localStorage.setItem(LOGS_PAGE_SIZE_STORAGE_KEY, "75");
    renderWithProviders(<LogsPage />);

    expect(screen.getByRole("combobox", { name: "每页条数" })).toHaveValue("50");
    expect(latestFeedOptions()?.limit).toBe(50);
  });

  it("keeps active cards separate from the persisted page-size count", () => {
    mockPageFeed({
      requestLogs: Array.from({ length: 50 }, (_, index) => ({ id: 100 - index })),
      totalCount: 50,
      activeRequests: [{ trace_id: "active-1" }, { trace_id: "active-2" }],
    });
    renderWithProviders(<LogsPage />);

    expect(screen.getByTestId("page-log-ids").textContent?.split(",")).toHaveLength(50);
    expect(screen.getByTestId("active-count")).toHaveTextContent("2");
    expect(screen.getByTestId("summary")).toHaveTextContent("本页 50 条");
  });

  it("filters live traces with path, time, and stream-internal-error semantics", () => {
    vi.useFakeTimers();
    traceStoreState.traces = [
      createTrace("trace-stream", {
        first_seen_ms: new Date("2026-08-01T09:30").getTime(),
        summary: {
          status: 200,
          attempts: [{ stream_internal_error: { event_type: "error" } }],
        } as any,
      }),
      createTrace("trace-health", {
        method: "GET",
        path: "/health",
        first_seen_ms: new Date("2026-08-01T09:45").getTime(),
      }),
    ];
    renderWithProviders(<LogsPage />);
    fireEvent.click(screen.getByRole("tab", { name: "流内错误" }));
    expect(screen.getByTestId("trace-ids")).toHaveTextContent("trace-stream");

    fireEvent.click(screen.getByRole("button", { name: /全部时间/ }));
    fireEvent.change(screen.getByLabelText("开始时间"), { target: { value: "2026-08-01T09:00" } });
    fireEvent.change(screen.getByLabelText("结束时间"), { target: { value: "2026-08-01T10:00" } });
    fireEvent.click(screen.getByRole("button", { name: "应用" }));
    expect(screen.getByTestId("trace-ids")).toHaveTextContent("trace-stream");
  });
});
