// Usage:
// - Entry: Home "代理记录" button -> `/#/logs`.
// - Backend commands: `request_logs_page_all`, `request_log_get`, `request_attempt_logs_by_trace_id`.

import { ChevronLeft, ChevronRight, ChevronsUp } from "lucide-react";
import { useEffect, useMemo, useReducer } from "react";
import { HomeRequestLogsPanel } from "../components/home/HomeRequestLogsPanel";
import { RequestLogDetailDialog } from "../components/home/RequestLogDetailDialog";
import { cliFilterItemsWith, type CliFilterKey } from "../constants/clis";
import { GatewayErrorCodes } from "../constants/gatewayErrorCodes";
import { useRequestLogsPageFeed } from "../hooks/useRequestLogsPageFeed";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { Input } from "../ui/Input";
import { PageHeader } from "../ui/PageHeader";
import { Select } from "../ui/Select";
import { Switch } from "../ui/Switch";
import { TabList } from "../ui/TabList";
import { useTraceStore } from "../services/gateway/traceStore";
import type {
  RequestLogPageFilters,
  RequestLogStatusFilter,
} from "../services/gateway/requestLogs";

export const LOGS_PAGE_SIZE_STORAGE_KEY = "aio-logs-page-size";
export const LOGS_PAGE_SIZE_OPTIONS = [50, 100, 200] as const;
const LOGS_PAGE_DEFAULT_SIZE = 50;
const TEXT_FILTER_DEBOUNCE_MS = 300;
const AUTO_REFRESH_INTERVAL_MS = 2000;
const LOG_CLI_FILTER_ITEMS = cliFilterItemsWith("logs");

type LogsPageSize = (typeof LOGS_PAGE_SIZE_OPTIONS)[number];
type StatusPredicate = (status: number | null) => boolean;

type LogsPageState = {
  cliKey: CliFilterKey;
  statusFilter: string;
  errorCodeFilter: string;
  pathFilter: string;
  appliedStatusFilter: string;
  appliedErrorCodeFilter: string;
  appliedPathFilter: string;
  autoRefresh: boolean;
  pageSize: LogsPageSize;
  cursorStack: Array<string | null>;
  selectedLogId: number | null;
};

type LogsPageAction =
  | { type: "setCliKey"; cliKey: CliFilterKey }
  | { type: "setStatusFilter"; statusFilter: string }
  | { type: "setErrorCodeFilter"; errorCodeFilter: string }
  | { type: "setPathFilter"; pathFilter: string }
  | {
      type: "applyTextFilters";
      statusFilter: string;
      errorCodeFilter: string;
      pathFilter: string;
    }
  | { type: "setAutoRefresh"; autoRefresh: boolean }
  | { type: "setPageSize"; pageSize: LogsPageSize }
  | { type: "goToNextPage"; cursor: string }
  | { type: "goToPreviousPage" }
  | { type: "goToLatestPage" }
  | { type: "setSelectedLogId"; selectedLogId: number | null }
  | { type: "resetFilters" };

function isLogsPageSize(value: number): value is LogsPageSize {
  return LOGS_PAGE_SIZE_OPTIONS.some((option) => option === value);
}

export function readLogsPageSizeFromStorage(): LogsPageSize {
  if (typeof window === "undefined") return LOGS_PAGE_DEFAULT_SIZE;
  try {
    const value = Number(window.localStorage.getItem(LOGS_PAGE_SIZE_STORAGE_KEY));
    return isLogsPageSize(value) ? value : LOGS_PAGE_DEFAULT_SIZE;
  } catch {
    return LOGS_PAGE_DEFAULT_SIZE;
  }
}

function writeLogsPageSizeToStorage(pageSize: LogsPageSize) {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(LOGS_PAGE_SIZE_STORAGE_KEY, String(pageSize));
  } catch {
    // Page-size persistence is best effort.
  }
}

function createInitialLogsPageState(): LogsPageState {
  return {
    cliKey: "all",
    statusFilter: "",
    errorCodeFilter: "",
    pathFilter: "",
    appliedStatusFilter: "",
    appliedErrorCodeFilter: "",
    appliedPathFilter: "",
    autoRefresh: true,
    pageSize: readLogsPageSizeFromStorage(),
    cursorStack: [null],
    selectedLogId: null,
  };
}

function logsPageReducer(state: LogsPageState, action: LogsPageAction): LogsPageState {
  switch (action.type) {
    case "setCliKey":
      return { ...state, cliKey: action.cliKey, cursorStack: [null] };
    case "setStatusFilter":
      return { ...state, statusFilter: action.statusFilter };
    case "setErrorCodeFilter":
      return { ...state, errorCodeFilter: action.errorCodeFilter };
    case "setPathFilter":
      return { ...state, pathFilter: action.pathFilter };
    case "applyTextFilters":
      return {
        ...state,
        appliedStatusFilter: action.statusFilter,
        appliedErrorCodeFilter: action.errorCodeFilter,
        appliedPathFilter: action.pathFilter,
        cursorStack: [null],
      };
    case "setAutoRefresh":
      return { ...state, autoRefresh: action.autoRefresh };
    case "setPageSize":
      return { ...state, pageSize: action.pageSize, cursorStack: [null] };
    case "goToNextPage":
      return { ...state, cursorStack: [...state.cursorStack, action.cursor] };
    case "goToPreviousPage":
      return {
        ...state,
        cursorStack:
          state.cursorStack.length > 1 ? state.cursorStack.slice(0, -1) : state.cursorStack,
      };
    case "goToLatestPage":
      return { ...state, cursorStack: [null] };
    case "setSelectedLogId":
      return { ...state, selectedLogId: action.selectedLogId };
    case "resetFilters":
      return {
        ...state,
        cliKey: "all",
        statusFilter: "",
        errorCodeFilter: "",
        pathFilter: "",
        appliedStatusFilter: "",
        appliedErrorCodeFilter: "",
        appliedPathFilter: "",
        cursorStack: [null],
      };
  }
}

function parseStatusFilter(query: string): RequestLogStatusFilter | null {
  const raw = query.trim();
  if (!raw) return null;

  const exact = raw.match(/^(\d{3})$/);
  if (exact) {
    return { op: "eq", value: Number(exact[1]) };
  }

  const not = raw.match(/^!\s*(\d{3})$/);
  if (not) {
    return { op: "neq", value: Number(not[1]) };
  }

  const gte = raw.match(/^>=\s*(\d{3})$/);
  if (gte) {
    return { op: "gte", value: Number(gte[1]) };
  }

  const lte = raw.match(/^<=\s*(\d{3})$/);
  if (lte) {
    return { op: "lte", value: Number(lte[1]) };
  }

  return null;
}

function buildStatusPredicate(filter: RequestLogStatusFilter | null): StatusPredicate | null {
  if (!filter) return null;
  switch (filter.op) {
    case "eq":
      return (status) => status === filter.value;
    case "neq":
      return (status) => status == null || status !== filter.value;
    case "gte":
      return (status) => status != null && status >= filter.value;
    case "lte":
      return (status) => status != null && status <= filter.value;
  }
}

export function LogsPage() {
  const { traces } = useTraceStore();
  const showCustomTooltip = true;

  const [state, dispatch] = useReducer(logsPageReducer, undefined, createInitialLogsPageState);
  const {
    cliKey,
    statusFilter,
    errorCodeFilter,
    pathFilter,
    appliedStatusFilter,
    appliedErrorCodeFilter,
    appliedPathFilter,
    autoRefresh,
    pageSize,
    cursorStack,
    selectedLogId,
  } = state;
  const currentCursor = cursorStack[cursorStack.length - 1] ?? null;
  const currentPageNumber = cursorStack.length;
  const hasPreviousPage = cursorStack.length > 1;
  const setSelectedLogId = (selectedLogId: number | null) =>
    dispatch({ type: "setSelectedLogId", selectedLogId });
  const parsedDraftStatusFilter = useMemo(() => parseStatusFilter(statusFilter), [statusFilter]);
  const statusFilterValid = statusFilter.trim().length === 0 || parsedDraftStatusFilter != null;
  const appliedStatus = useMemo(
    () => parseStatusFilter(appliedStatusFilter),
    [appliedStatusFilter]
  );
  const pageFilters = useMemo<RequestLogPageFilters>(
    () => ({
      cliKey: cliKey === "all" ? null : cliKey,
      status: appliedStatus,
      errorCodeContains: appliedErrorCodeFilter.trim() || null,
      methodPathContains: appliedPathFilter.trim() || null,
    }),
    [appliedErrorCodeFilter, appliedPathFilter, appliedStatus, cliKey]
  );
  const {
    requestLogs,
    nextCursor,
    activeRequests,
    activeRequestsAvailable,
    requestLogsLoading,
    requestLogsRefreshing,
    requestLogsPageFetching,
    requestLogsAvailable,
    refreshRequestLogs,
  } = useRequestLogsPageFeed({
    filters: pageFilters,
    cursor: currentCursor,
    limit: pageSize,
    liveUpdatesEnabled: autoRefresh,
    liveUpdateWindowMs: AUTO_REFRESH_INTERVAL_MS,
    refreshOnForeground: autoRefresh,
  });

  useEffect(() => {
    writeLogsPageSizeToStorage(pageSize);
  }, [pageSize]);

  useEffect(() => {
    const nextAppliedStatusFilter = statusFilterValid ? statusFilter : "";
    if (
      nextAppliedStatusFilter === appliedStatusFilter &&
      errorCodeFilter === appliedErrorCodeFilter &&
      pathFilter === appliedPathFilter
    ) {
      return;
    }

    const timer = window.setTimeout(() => {
      dispatch({
        type: "applyTextFilters",
        statusFilter: nextAppliedStatusFilter,
        errorCodeFilter,
        pathFilter,
      });
    }, TEXT_FILTER_DEBOUNCE_MS);
    return () => window.clearTimeout(timer);
  }, [
    appliedErrorCodeFilter,
    appliedPathFilter,
    appliedStatusFilter,
    errorCodeFilter,
    pathFilter,
    statusFilter,
    statusFilterValid,
  ]);

  const statusPredicate = useMemo(() => buildStatusPredicate(appliedStatus), [appliedStatus]);
  const activeFilterCount = [
    cliKey !== "all",
    statusFilter.trim().length > 0,
    errorCodeFilter.trim().length > 0,
    pathFilter.trim().length > 0,
  ].filter(Boolean).length;

  const filteredActiveRequests = useMemo(() => {
    const errorNeedle = appliedErrorCodeFilter.trim().toLowerCase();
    const pathNeedle = appliedPathFilter.trim().toLowerCase();

    return activeRequests.filter((request) => {
      if (cliKey !== "all" && request.cli_key !== cliKey) return false;
      if (statusPredicate && !statusPredicate(null)) return false;
      if (errorNeedle) return false;
      if (pathNeedle) {
        const haystack = `${request.method} ${request.path}`.toLowerCase();
        if (!haystack.includes(pathNeedle)) return false;
      }
      return true;
    });
  }, [activeRequests, appliedErrorCodeFilter, appliedPathFilter, cliKey, statusPredicate]);
  const filteredTraces = useMemo(() => {
    const errorNeedle = appliedErrorCodeFilter.trim().toLowerCase();
    const pathNeedle = appliedPathFilter.trim().toLowerCase();

    return traces.filter((trace) => {
      if (cliKey !== "all" && trace.cli_key !== cliKey) return false;
      if (statusPredicate) {
        const status = trace.summary?.status ?? null;
        if (!statusPredicate(status)) return false;
      }
      if (errorNeedle) {
        const raw = (trace.summary?.error_code ?? "").toLowerCase();
        if (!raw.includes(errorNeedle)) return false;
      }
      if (pathNeedle) {
        const haystack = `${trace.method} ${trace.path}`.toLowerCase();
        if (!haystack.includes(pathNeedle)) return false;
      }
      return true;
    });
  }, [appliedErrorCodeFilter, appliedPathFilter, cliKey, statusPredicate, traces]);
  const logsSummaryText =
    requestLogsAvailable === false
      ? undefined
      : requestLogs.length === 0 && requestLogsLoading
        ? "加载中…"
        : requestLogsRefreshing
          ? `更新中… · 第 ${currentPageNumber} 页 · 本页 ${requestLogs.length} 条`
          : `第 ${currentPageNumber} 页 · 本页 ${requestLogs.length} 条`;

  function resetFilters() {
    dispatch({ type: "resetFilters" });
  }

  return (
    <div className="flex h-full flex-col gap-6 overflow-hidden">
      <PageHeader title="代理记录" />

      <Card padding="sm" className="overflow-visible flex flex-col gap-4">
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div className="text-sm font-semibold text-foreground">筛选条件</div>

          <div className="flex flex-wrap items-center gap-3">
            <div className="flex items-center gap-2 text-xs text-muted-foreground">
              <span>自动刷新</span>
              <Switch
                checked={autoRefresh}
                onCheckedChange={(autoRefresh) => dispatch({ type: "setAutoRefresh", autoRefresh })}
                size="sm"
                disabled={requestLogsAvailable === false}
              />
            </div>
            <Button
              variant="secondary"
              size="sm"
              onClick={resetFilters}
              disabled={activeFilterCount === 0}
            >
              清空筛选
            </Button>
          </div>
        </div>

        <div className="grid items-start gap-4 md:grid-cols-2 xl:grid-cols-[1.35fr_1fr_1fr_1fr]">
          <div className="space-y-2">
            <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              CLI
            </div>
            <TabList
              ariaLabel="CLI 过滤"
              items={LOG_CLI_FILTER_ITEMS}
              value={cliKey}
              onChange={(cliKey) => dispatch({ type: "setCliKey", cliKey })}
              size="sm"
              className="w-full"
              buttonClassName="shrink-0 px-3 py-1.5 whitespace-nowrap"
            />
          </div>

          <div className="space-y-2">
            <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              Status
            </div>
            <Input
              value={statusFilter}
              onChange={(e) => dispatch({ type: "setStatusFilter", statusFilter: e.target.value })}
              placeholder="例：499 / 524 / !200 / >=400"
              mono
              disabled={requestLogsAvailable === false}
            />
            <div className="text-[11px] leading-4 text-muted-foreground">
              支持 `499`、`!200`、`&gt;=400`、`&lt;=399`
            </div>
            {!statusFilterValid ? (
              <div className="text-[11px] leading-4 text-rose-600 dark:text-rose-400">
                表达式不合法：支持 499 / !200 / &gt;=400 / &lt;=399
              </div>
            ) : null}
          </div>

          <div className="space-y-2">
            <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              error_code
            </div>
            <Input
              value={errorCodeFilter}
              onChange={(e) =>
                dispatch({ type: "setErrorCodeFilter", errorCodeFilter: e.target.value })
              }
              placeholder={`例：${GatewayErrorCodes.UPSTREAM_TIMEOUT}`}
              mono
              disabled={requestLogsAvailable === false}
            />
            <div className="text-[11px] leading-4 text-muted-foreground">
              支持按错误码关键字模糊匹配
            </div>
          </div>

          <div className="space-y-2">
            <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              Path
            </div>
            <Input
              value={pathFilter}
              onChange={(e) => dispatch({ type: "setPathFilter", pathFilter: e.target.value })}
              placeholder="例：/v1/messages"
              mono
              disabled={requestLogsAvailable === false}
            />
            <div className="text-[11px] leading-4 text-muted-foreground">
              按请求路径或方法路径组合模糊匹配
            </div>
          </div>
        </div>

        <div className="flex flex-wrap items-center justify-between gap-3 border-t border-line pt-3">
          <label className="flex items-center gap-2 text-xs text-muted-foreground">
            <span>每页</span>
            <Select
              aria-label="每页条数"
              value={String(pageSize)}
              onChange={(event) => {
                const nextPageSize = Number(event.target.value);
                if (isLogsPageSize(nextPageSize)) {
                  dispatch({ type: "setPageSize", pageSize: nextPageSize });
                }
              }}
              className="h-8 w-24 rounded-md px-2"
              mono
              disabled={requestLogsAvailable === false}
            >
              {LOGS_PAGE_SIZE_OPTIONS.map((option) => (
                <option key={option} value={option}>
                  {option}
                </option>
              ))}
            </Select>
            <span>条</span>
          </label>

          <div className="flex flex-wrap items-center gap-2">
            <span
              className="min-w-14 text-center text-xs tabular-nums text-muted-foreground"
              aria-live="polite"
            >
              第 {currentPageNumber} 页
            </span>
            <Button
              variant="secondary"
              size="icon"
              aria-label="上一页"
              title="上一页"
              onClick={() => dispatch({ type: "goToPreviousPage" })}
              disabled={
                !hasPreviousPage || requestLogsPageFetching || requestLogsAvailable === false
              }
            >
              <ChevronLeft className="h-4 w-4" />
            </Button>
            <Button
              variant="secondary"
              size="icon"
              aria-label="下一页"
              title="下一页"
              onClick={() => {
                if (nextCursor) dispatch({ type: "goToNextPage", cursor: nextCursor });
              }}
              disabled={
                nextCursor == null || requestLogsPageFetching || requestLogsAvailable === false
              }
            >
              <ChevronRight className="h-4 w-4" />
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => dispatch({ type: "goToLatestPage" })}
              disabled={
                !hasPreviousPage || requestLogsPageFetching || requestLogsAvailable === false
              }
              title="回到最新一页"
            >
              <ChevronsUp className="h-4 w-4" />
              回到最新
            </Button>
          </div>
        </div>
      </Card>

      <HomeRequestLogsPanel
        displayOptions={{
          customTooltip: showCustomTooltip,
          openLogsPageButton: false,
          compactModeToggle: false,
        }}
        title="代理记录列表"
        summaryTextOverride={logsSummaryText}
        compactModeOverride={false}
        emptyStateTitle={activeFilterCount > 0 ? "没有符合筛选条件的代理记录" : "当前没有代理记录"}
        traces={filteredTraces}
        activeRequests={filteredActiveRequests}
        activeRequestsAvailable={activeRequestsAvailable}
        requestLogs={requestLogs}
        requestLogsLoading={requestLogsLoading}
        requestLogsRefreshing={requestLogsRefreshing}
        requestLogsAvailable={requestLogsAvailable}
        requestLogOrder="source"
        onRefreshRequestLogs={() => void refreshRequestLogs()}
        selectedLogId={selectedLogId}
        onSelectLogId={setSelectedLogId}
      />

      <RequestLogDetailDialog selectedLogId={selectedLogId} onSelectLogId={setSelectedLogId} />
    </div>
  );
}
