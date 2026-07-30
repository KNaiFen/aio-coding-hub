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
  useTraceStore: () => ({
    traces: traceStoreState.traces,
  }),
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
  vi.mocked(useRequestLogsPageFeed).mockReturnValue({
    requestLogs: [],
    nextCursor: null,
    activeRequests: [],
    activeRequestsAvailable: true,
    requestLogsLoading: false,
    requestLogsRefreshing: false,
    requestLogsPageFetching: false,
    requestLogsAvailable: true,
    refreshActiveRequests: vi.fn(),
    refreshRequestLogs,
    ...overrides,
  } as any);
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
    Reflect.deleteProperty(window, "localStorage");
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

  it("disables filters when request logs are unavailable", () => {
    mockPageFeed({ requestLogsAvailable: false });
    renderWithProviders(<LogsPage />);

    expect(screen.getByRole("switch")).toBeDisabled();
    expect(screen.getByPlaceholderText("例：499 / 524 / !200 / >=400")).toBeDisabled();
    expect(screen.getByPlaceholderText("例：GW_UPSTREAM_TIMEOUT")).toBeDisabled();
    expect(screen.getByPlaceholderText("例：/v1/messages")).toBeDisabled();
  });

  it("shows status validation immediately and does not apply an invalid expression", () => {
    renderWithProviders(<LogsPage />);

    fireEvent.change(screen.getByPlaceholderText("例：499 / 524 / !200 / >=400"), {
      target: { value: "nope" },
    });

    expect(screen.getByText(/表达式不合法/)).toBeInTheDocument();
    expect(latestFeedOptions()?.filters.status).toBeNull();
  });

  it("debounces text filters and resets a history cursor when they apply", () => {
    vi.useFakeTimers();
    vi.mocked(useRequestLogsPageFeed).mockImplementation(({ cursor }) => {
      return {
        requestLogs: [],
        nextCursor: cursor == null ? "opaque-next" : null,
        activeRequests: [],
        activeRequestsAvailable: true,
        requestLogsLoading: false,
        requestLogsRefreshing: false,
        requestLogsPageFetching: false,
        requestLogsAvailable: true,
        refreshActiveRequests: vi.fn(),
        refreshRequestLogs: vi.fn(),
      } as any;
    });
    renderWithProviders(<LogsPage />);

    fireEvent.click(screen.getByRole("button", { name: "下一页" }));
    expect(latestFeedOptions()?.cursor).toBe("opaque-next");

    fireEvent.change(screen.getByPlaceholderText("例：/v1/messages"), {
      target: { value: "messages" },
    });
    act(() => vi.advanceTimersByTime(299));
    expect(latestFeedOptions()?.cursor).toBe("opaque-next");
    expect(latestFeedOptions()?.filters.methodPathContains).toBeNull();

    act(() => vi.advanceTimersByTime(1));
    expect(latestFeedOptions()?.cursor).toBeNull();
    expect(latestFeedOptions()?.filters.methodPathContains).toBe("messages");
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

  it("keeps null-status active requests for neq while exact status filters hide them", () => {
    vi.useFakeTimers();
    traceStoreState.traces = [
      createTrace("trace-live"),
      createTrace("trace-ok", { summary: { status: 200 } as any }),
      createTrace("trace-error", { summary: { status: 524 } as any }),
    ];
    mockPageFeed({
      activeRequests: [
        {
          trace_id: "active-live",
          cli_key: "claude",
          method: "POST",
          path: "/v1/messages",
        },
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

  it("resets the cursor immediately when the CLI filter changes", () => {
    vi.mocked(useRequestLogsPageFeed).mockImplementation(({ cursor }) => {
      return {
        requestLogs: [],
        nextCursor: cursor == null ? "opaque-next" : null,
        activeRequests: [],
        activeRequestsAvailable: true,
        requestLogsLoading: false,
        requestLogsRefreshing: false,
        requestLogsPageFetching: false,
        requestLogsAvailable: true,
        refreshActiveRequests: vi.fn(),
        refreshRequestLogs: vi.fn(),
      } as any;
    });
    renderWithProviders(<LogsPage />);

    fireEvent.click(screen.getByRole("button", { name: "下一页" }));
    expect(latestFeedOptions()?.cursor).toBe("opaque-next");
    fireEvent.click(screen.getByRole("tab", { name: "Codex" }));

    expect(latestFeedOptions()?.cursor).toBeNull();
    expect(latestFeedOptions()?.filters.cliKey).toBe("codex");
  });

  it("navigates with the opaque cursor stack and preserves server row order", () => {
    vi.mocked(useRequestLogsPageFeed).mockImplementation(({ cursor }) => {
      const latest = cursor == null;
      return {
        requestLogs: latest ? [{ id: 9 }, { id: 7 }] : [{ id: 5 }],
        nextCursor: latest ? "opaque-next" : null,
        activeRequests: [],
        activeRequestsAvailable: true,
        requestLogsLoading: false,
        requestLogsRefreshing: false,
        requestLogsPageFetching: false,
        requestLogsAvailable: true,
        refreshActiveRequests: vi.fn(),
        refreshRequestLogs: vi.fn(),
      } as any;
    });
    renderWithProviders(<LogsPage />);

    expect(screen.getByTestId("page-log-ids")).toHaveTextContent("9,7");
    expect(screen.getByTestId("summary")).toHaveTextContent("第 1 页 · 本页 2 条");
    expect(screen.getByTestId("request-log-order")).toHaveTextContent("source");

    fireEvent.click(screen.getByRole("button", { name: "下一页" }));
    expect(screen.getByTestId("page-log-ids")).toHaveTextContent("5");
    expect(screen.getByTestId("summary")).toHaveTextContent("第 2 页 · 本页 1 条");
    expect(screen.getByRole("button", { name: "下一页" })).toBeDisabled();

    fireEvent.click(screen.getByRole("button", { name: "上一页" }));
    expect(latestFeedOptions()?.cursor).toBeNull();
    expect(screen.getByTestId("page-log-ids")).toHaveTextContent("9,7");
  });

  it("returns directly to the latest page from history", () => {
    vi.mocked(useRequestLogsPageFeed).mockImplementation(({ cursor }) => {
      return {
        requestLogs: [],
        nextCursor: cursor == null ? "opaque-next" : null,
        activeRequests: [],
        activeRequestsAvailable: true,
        requestLogsLoading: false,
        requestLogsRefreshing: false,
        requestLogsPageFetching: false,
        requestLogsAvailable: true,
        refreshActiveRequests: vi.fn(),
        refreshRequestLogs: vi.fn(),
      } as any;
    });
    renderWithProviders(<LogsPage />);

    fireEvent.click(screen.getByRole("button", { name: "下一页" }));
    fireEvent.click(screen.getByRole("button", { name: "回到最新" }));

    expect(latestFeedOptions()?.cursor).toBeNull();
    expect(screen.getByText("第 1 页", { selector: "span" })).toBeInTheDocument();
  });

  it("loads and persists the selected page size while resetting history", () => {
    window.localStorage.setItem(LOGS_PAGE_SIZE_STORAGE_KEY, "100");
    vi.mocked(useRequestLogsPageFeed).mockImplementation(({ cursor }) => {
      return {
        requestLogs: [],
        nextCursor: cursor == null ? "opaque-next" : null,
        activeRequests: [],
        activeRequestsAvailable: true,
        requestLogsLoading: false,
        requestLogsRefreshing: false,
        requestLogsPageFetching: false,
        requestLogsAvailable: true,
        refreshActiveRequests: vi.fn(),
        refreshRequestLogs: vi.fn(),
      } as any;
    });
    renderWithProviders(<LogsPage />);

    expect(screen.getByRole("combobox", { name: "每页条数" })).toHaveValue("100");
    expect(latestFeedOptions()?.limit).toBe(100);
    fireEvent.click(screen.getByRole("button", { name: "下一页" }));

    fireEvent.change(screen.getByRole("combobox", { name: "每页条数" }), {
      target: { value: "200" },
    });
    expect(latestFeedOptions()?.limit).toBe(200);
    expect(latestFeedOptions()?.cursor).toBeNull();
    expect(window.localStorage.getItem(LOGS_PAGE_SIZE_STORAGE_KEY)).toBe("200");
  });

  it("falls back to 50 when the stored page size is unsupported", () => {
    window.localStorage.setItem(LOGS_PAGE_SIZE_STORAGE_KEY, "75");
    renderWithProviders(<LogsPage />);

    expect(screen.getByRole("combobox", { name: "每页条数" })).toHaveValue("50");
    expect(latestFeedOptions()?.limit).toBe(50);
  });

  it("manually refreshes the currently selected page", () => {
    const latestRefresh = vi.fn();
    const historyRefresh = vi.fn();
    vi.mocked(useRequestLogsPageFeed).mockImplementation(({ cursor }) => {
      return {
        requestLogs: [],
        nextCursor: cursor == null ? "opaque-next" : null,
        activeRequests: [],
        activeRequestsAvailable: true,
        requestLogsLoading: false,
        requestLogsRefreshing: false,
        requestLogsPageFetching: false,
        requestLogsAvailable: true,
        refreshActiveRequests: vi.fn(),
        refreshRequestLogs: cursor == null ? latestRefresh : historyRefresh,
      } as any;
    });
    renderWithProviders(<LogsPage />);

    fireEvent.click(screen.getByRole("button", { name: "下一页" }));
    fireEvent.click(screen.getByRole("button", { name: "刷新当前页" }));

    expect(historyRefresh).toHaveBeenCalledTimes(1);
    expect(latestRefresh).not.toHaveBeenCalled();
  });

  it("keeps active cards separate from the persisted page-size count", () => {
    mockPageFeed({
      requestLogs: Array.from({ length: 50 }, (_, index) => ({ id: 100 - index })),
      activeRequests: [{ trace_id: "active-1" }, { trace_id: "active-2" }],
    });
    renderWithProviders(<LogsPage />);

    expect(screen.getByTestId("page-log-ids").textContent?.split(",")).toHaveLength(50);
    expect(screen.getByTestId("active-count")).toHaveTextContent("2");
    expect(screen.getByTestId("summary")).toHaveTextContent("本页 50 条");
  });

  it("filters live traces with the applied server filter semantics", () => {
    vi.useFakeTimers();
    traceStoreState.traces = [
      createTrace("trace-messages"),
      createTrace("trace-health", { method: "GET", path: "/health" }),
    ];
    renderWithProviders(<LogsPage />);
    expect(screen.getByTestId("trace-ids")).toHaveTextContent("trace-messages,trace-health");

    fireEvent.change(screen.getByPlaceholderText("例：/v1/messages"), {
      target: { value: "messages" },
    });
    act(() => vi.advanceTimersByTime(300));

    expect(screen.getByTestId("trace-ids")).toHaveTextContent("trace-messages");
  });
});
