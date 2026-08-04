// Usage:
// - Entry: Home "代理记录" button -> `/#/logs`.
// - Backend commands: `request_logs_snapshot_page_all`, `request_log_get`,
//   `request_attempt_logs_by_trace_id`.

import { CalendarClock, ChevronLeft, ChevronRight, RefreshCw } from "lucide-react";
import { useCallback, useEffect, useMemo, useReducer } from "react";
import { HomeRequestLogsPanel } from "../components/home/HomeRequestLogsPanel";
import { RequestLogDetailDialog } from "../components/home/RequestLogDetailDialog";
import { cliFilterItemsWith, type CliFilterKey } from "../constants/clis";
import { GatewayErrorCodes } from "../constants/gatewayErrorCodes";
import { useRequestLogsPageFeed } from "../hooks/useRequestLogsPageFeed";
import { useTraceStore, type TraceSession } from "../services/gateway/traceStore";
import type {
  RequestLogErrorScope,
  RequestLogPageFilters,
  RequestLogStatusFilter,
} from "../services/gateway/requestLogs";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { Input } from "../ui/Input";
import { PageHeader } from "../ui/PageHeader";
import { Popover } from "../ui/Popover";
import { Select } from "../ui/Select";
import { Switch } from "../ui/Switch";
import { TabList, type TabListItem } from "../ui/TabList";

export const LOGS_PAGE_SIZE_STORAGE_KEY = "aio-logs-page-size";
export const LOGS_PAGE_SIZE_OPTIONS = [50, 100, 200] as const;
const LOGS_PAGE_DEFAULT_SIZE = 50;
const TEXT_FILTER_DEBOUNCE_MS = 300;
const AUTO_REFRESH_INTERVAL_MS = 2000;
const LOG_CLI_FILTER_ITEMS = cliFilterItemsWith("logs");
const LOG_ERROR_SCOPE_ITEMS: Array<TabListItem<RequestLogErrorScope>> = [
  { key: "all", label: "全部" },
  { key: "all_errors", label: "全部报错" },
  { key: "stream_internal_error", label: "流内错误" },
];
const INTERRUPTED_ERROR_CODES = new Set<string>([
  GatewayErrorCodes.REQUEST_ABORTED,
  GatewayErrorCodes.STREAM_ABORTED,
  GatewayErrorCodes.REQUEST_INTERRUPTED_BY_RESTART,
  GatewayErrorCodes.REQUEST_INTERRUPTED_BY_GATEWAY_STOP,
]);
type LogsPageSize = (typeof LOGS_PAGE_SIZE_OPTIONS)[number];
type StatusPredicate = (status: number | null) => boolean;
type TimeRangeInputs = { from: string; to: string };
type TimeRangePreset = "lastHour" | "today" | "yesterday";

type LogsPageState = {
  cliKey: CliFilterKey;
  statusFilter: string;
  errorCodeFilter: string;
  pathFilter: string;
  appliedStatusFilter: string;
  appliedErrorCodeFilter: string;
  appliedPathFilter: string;
  errorScope: RequestLogErrorScope;
  timeFromDraft: string;
  timeToDraft: string;
  appliedTimeFrom: string;
  appliedTimeTo: string;
  autoRefresh: boolean;
  pageSize: LogsPageSize;
  page: number;
  pageInput: string;
  snapshotId: string | null;
  snapshotRevision: number;
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
  | { type: "setErrorScope"; errorScope: RequestLogErrorScope }
  | { type: "setTimeFromDraft"; value: string }
  | { type: "setTimeToDraft"; value: string }
  | { type: "applyTimeRange"; range: TimeRangeInputs }
  | { type: "setQuickTimeRange"; range: TimeRangeInputs }
  | { type: "clearTimeRange" }
  | { type: "setAutoRefresh"; autoRefresh: boolean }
  | { type: "setPageSize"; pageSize: LogsPageSize }
  | { type: "setPageInput"; pageInput: string }
  | { type: "captureSnapshot"; snapshotId: string }
  | { type: "goToPage"; page: number; snapshotId: string }
  | { type: "refreshSnapshot" }
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
    errorScope: "all",
    timeFromDraft: "",
    timeToDraft: "",
    appliedTimeFrom: "",
    appliedTimeTo: "",
    autoRefresh: true,
    pageSize: readLogsPageSizeFromStorage(),
    page: 1,
    pageInput: "1",
    snapshotId: null,
    snapshotRevision: 0,
    selectedLogId: null,
  };
}

function withFreshSnapshot(
  state: LogsPageState,
  updates: Partial<LogsPageState> = {}
): LogsPageState {
  return {
    ...state,
    ...updates,
    page: 1,
    pageInput: "1",
    snapshotId: null,
    snapshotRevision: state.snapshotRevision + 1,
  };
}

function logsPageReducer(state: LogsPageState, action: LogsPageAction): LogsPageState {
  switch (action.type) {
    case "setCliKey":
      return withFreshSnapshot(state, { cliKey: action.cliKey });
    case "setStatusFilter":
      return { ...state, statusFilter: action.statusFilter };
    case "setErrorCodeFilter":
      return { ...state, errorCodeFilter: action.errorCodeFilter };
    case "setPathFilter":
      return { ...state, pathFilter: action.pathFilter };
    case "applyTextFilters":
      return withFreshSnapshot(state, {
        appliedStatusFilter: action.statusFilter,
        appliedErrorCodeFilter: action.errorCodeFilter,
        appliedPathFilter: action.pathFilter,
      });
    case "setErrorScope":
      return withFreshSnapshot(state, { errorScope: action.errorScope });
    case "setTimeFromDraft":
      return { ...state, timeFromDraft: action.value };
    case "setTimeToDraft":
      return { ...state, timeToDraft: action.value };
    case "applyTimeRange":
      return withFreshSnapshot(state, {
        timeFromDraft: action.range.from,
        timeToDraft: action.range.to,
        appliedTimeFrom: action.range.from,
        appliedTimeTo: action.range.to,
      });
    case "setQuickTimeRange":
      return withFreshSnapshot(state, {
        timeFromDraft: action.range.from,
        timeToDraft: action.range.to,
        appliedTimeFrom: action.range.from,
        appliedTimeTo: action.range.to,
      });
    case "clearTimeRange":
      return withFreshSnapshot(state, {
        timeFromDraft: "",
        timeToDraft: "",
        appliedTimeFrom: "",
        appliedTimeTo: "",
      });
    case "setAutoRefresh":
      return { ...state, autoRefresh: action.autoRefresh };
    case "setPageSize":
      return withFreshSnapshot(state, { pageSize: action.pageSize });
    case "setPageInput":
      return { ...state, pageInput: action.pageInput };
    case "captureSnapshot":
      return state.snapshotId == null ? { ...state, snapshotId: action.snapshotId } : state;
    case "goToPage":
      return {
        ...state,
        page: action.page,
        pageInput: String(action.page),
        snapshotId: action.snapshotId,
      };
    case "refreshSnapshot":
      return withFreshSnapshot(state);
    case "setSelectedLogId":
      return { ...state, selectedLogId: action.selectedLogId };
    case "resetFilters":
      return withFreshSnapshot(state, {
        cliKey: "all",
        statusFilter: "",
        errorCodeFilter: "",
        pathFilter: "",
        appliedStatusFilter: "",
        appliedErrorCodeFilter: "",
        appliedPathFilter: "",
        errorScope: "all",
        timeFromDraft: "",
        timeToDraft: "",
        appliedTimeFrom: "",
        appliedTimeTo: "",
      });
  }
}

function parseStatusFilter(query: string): RequestLogStatusFilter | null {
  const raw = query.trim();
  if (!raw) return null;

  const exact = raw.match(/^(\d{3})$/);
  if (exact) return { op: "eq", value: Number(exact[1]) };

  const not = raw.match(/^!\s*(\d{3})$/);
  if (not) return { op: "neq", value: Number(not[1]) };

  const gte = raw.match(/^>=\s*(\d{3})$/);
  if (gte) return { op: "gte", value: Number(gte[1]) };

  const lte = raw.match(/^<=\s*(\d{3})$/);
  if (lte) return { op: "lte", value: Number(lte[1]) };

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

function toDatetimeLocal(ms: number) {
  const date = new Date(ms);
  if (Number.isNaN(date.getTime())) return "";
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(
    date.getHours()
  )}:${pad(date.getMinutes())}`;
}

function parseDatetimeLocal(value: string): number | null {
  if (!value) return null;
  const timestamp = new Date(value).getTime();
  return Number.isFinite(timestamp) ? timestamp : null;
}

function isTimeRangeValid(range: TimeRangeInputs) {
  const from = parseDatetimeLocal(range.from);
  const to = parseDatetimeLocal(range.to);
  if (range.from && from == null) return false;
  if (range.to && to == null) return false;
  return from == null || to == null || from < to;
}

function quickTimeRange(preset: TimeRangePreset): TimeRangeInputs {
  const now = new Date();
  const currentMinuteEnd = new Date(now);
  currentMinuteEnd.setSeconds(0, 0);
  currentMinuteEnd.setMinutes(currentMinuteEnd.getMinutes() + 1);
  if (preset === "lastHour") {
    return {
      from: toDatetimeLocal(currentMinuteEnd.getTime() - 60 * 60 * 1000),
      to: toDatetimeLocal(currentMinuteEnd.getTime()),
    };
  }

  const todayStart = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  if (preset === "today") {
    return {
      from: toDatetimeLocal(todayStart.getTime()),
      to: toDatetimeLocal(currentMinuteEnd.getTime()),
    };
  }

  const yesterdayStart = new Date(todayStart);
  yesterdayStart.setDate(yesterdayStart.getDate() - 1);
  return {
    from: toDatetimeLocal(yesterdayStart.getTime()),
    to: toDatetimeLocal(todayStart.getTime()),
  };
}

function isWithinTimeRange(value: number, from: number | null, to: number | null) {
  if (!Number.isFinite(value)) return false;
  return (from == null || value >= from) && (to == null || value < to);
}

function hasStreamInternalError(
  attempts: ReadonlyArray<{ stream_internal_error?: unknown | null }> | null | undefined
) {
  return attempts?.some((attempt) => attempt.stream_internal_error != null) ?? false;
}

function matchesErrorScope(
  scope: RequestLogErrorScope,
  status: number | null,
  errorCode: string | null,
  streamInternalError: boolean
) {
  if (scope === "all") return true;
  if (scope === "stream_internal_error") return streamInternalError;
  const normalizedErrorCode = errorCode?.trim() ?? "";
  const interrupted =
    (status == null && !normalizedErrorCode) ||
    status === 499 ||
    INTERRUPTED_ERROR_CODES.has(normalizedErrorCode);
  if (interrupted) return false;
  const nonSuccessStatus = status != null && (status < 200 || status >= 300);
  return nonSuccessStatus || Boolean(normalizedErrorCode) || streamInternalError;
}

function parsePageInput(value: string, totalPages: number) {
  const page = Number(value);
  return Number.isSafeInteger(page) && page >= 1 && page <= totalPages ? page : null;
}

function pageSuggestions(page: number, totalPages: number) {
  if (totalPages <= 200) {
    return Array.from({ length: totalPages }, (_, index) => index + 1);
  }
  return Array.from(new Set([1, 2, page - 1, page, page + 1, totalPages]))
    .filter((value) => value >= 1 && value <= totalPages)
    .sort((left, right) => left - right);
}

function traceMatchesFilters(
  trace: TraceSession,
  cliKey: CliFilterKey,
  statusPredicate: StatusPredicate | null,
  errorNeedle: string,
  pathNeedle: string,
  errorScope: RequestLogErrorScope,
  timeFrom: number | null,
  timeTo: number | null
) {
  if (cliKey !== "all" && trace.cli_key !== cliKey) return false;
  if (!isWithinTimeRange(trace.first_seen_ms, timeFrom, timeTo)) return false;

  const status = trace.summary?.status ?? null;
  const errorCode = trace.summary?.error_code ?? null;
  if (statusPredicate && !statusPredicate(status)) return false;
  if (errorNeedle && !(errorCode ?? "").toLowerCase().includes(errorNeedle)) return false;
  if (pathNeedle && !`${trace.method} ${trace.path}`.toLowerCase().includes(pathNeedle))
    return false;

  const streamInternalError = hasStreamInternalError(trace.summary?.attempts);
  return matchesErrorScope(errorScope, status, errorCode, streamInternalError);
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
    errorScope,
    timeFromDraft,
    timeToDraft,
    appliedTimeFrom,
    appliedTimeTo,
    autoRefresh,
    pageSize,
    page,
    pageInput,
    snapshotId,
    snapshotRevision,
    selectedLogId,
  } = state;
  const setSelectedLogId = (nextSelectedLogId: number | null) =>
    dispatch({ type: "setSelectedLogId", selectedLogId: nextSelectedLogId });
  const parsedDraftStatusFilter = useMemo(() => parseStatusFilter(statusFilter), [statusFilter]);
  const statusFilterValid = statusFilter.trim().length === 0 || parsedDraftStatusFilter != null;
  const appliedStatus = useMemo(
    () => parseStatusFilter(appliedStatusFilter),
    [appliedStatusFilter]
  );
  const timeDraft = useMemo(
    () => ({ from: timeFromDraft, to: timeToDraft }),
    [timeFromDraft, timeToDraft]
  );
  const appliedTimeFromMs = useMemo(() => parseDatetimeLocal(appliedTimeFrom), [appliedTimeFrom]);
  const appliedTimeToMs = useMemo(() => parseDatetimeLocal(appliedTimeTo), [appliedTimeTo]);
  const timeRangeValid = isTimeRangeValid(timeDraft);
  const pageFilters = useMemo<RequestLogPageFilters>(
    () => ({
      cliKey: cliKey === "all" ? null : cliKey,
      status: appliedStatus,
      errorCodeContains: appliedErrorCodeFilter.trim() || null,
      methodPathContains: appliedPathFilter.trim() || null,
      errorScope,
      createdAtMsFrom: appliedTimeFromMs,
      createdAtMsTo: appliedTimeToMs,
    }),
    [
      appliedErrorCodeFilter,
      appliedPathFilter,
      appliedStatus,
      appliedTimeFromMs,
      appliedTimeToMs,
      cliKey,
      errorScope,
    ]
  );
  const refreshSnapshot = useCallback(() => {
    dispatch({ type: "refreshSnapshot" });
  }, []);
  const {
    requestLogs,
    snapshotId: loadedSnapshotId,
    totalCount,
    totalPages: loadedTotalPages,
    page: loadedPage,
    activeRequests,
    activeRequestsAvailable,
    requestLogsLoading,
    requestLogsRefreshing,
    requestLogsPageFetching,
    requestLogsAvailable,
    requestLogsSnapshotExpired,
  } = useRequestLogsPageFeed({
    filters: pageFilters,
    snapshotId,
    page,
    snapshotRevision,
    limit: pageSize,
    liveUpdatesEnabled: autoRefresh,
    liveUpdateWindowMs: AUTO_REFRESH_INTERVAL_MS,
    refreshOnForeground: autoRefresh,
    onRefreshSnapshot: refreshSnapshot,
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

  useEffect(() => {
    if (snapshotId == null && loadedSnapshotId) {
      dispatch({ type: "captureSnapshot", snapshotId: loadedSnapshotId });
    }
  }, [loadedSnapshotId, snapshotId]);

  useEffect(() => {
    if (snapshotId != null && requestLogsSnapshotExpired) {
      dispatch({ type: "refreshSnapshot" });
    }
  }, [requestLogsSnapshotExpired, snapshotId]);

  const statusPredicate = useMemo(() => buildStatusPredicate(appliedStatus), [appliedStatus]);
  const hasAppliedTimeRange = appliedTimeFromMs != null || appliedTimeToMs != null;
  const activeFilterCount = [
    cliKey !== "all",
    appliedStatusFilter.trim().length > 0,
    appliedErrorCodeFilter.trim().length > 0,
    appliedPathFilter.trim().length > 0,
    errorScope !== "all",
    hasAppliedTimeRange,
  ].filter(Boolean).length;
  const filteredActiveRequests = useMemo(() => {
    const errorNeedle = appliedErrorCodeFilter.trim().toLowerCase();
    const pathNeedle = appliedPathFilter.trim().toLowerCase();
    if (errorScope !== "all" || hasAppliedTimeRange) return [];

    return activeRequests.filter((request) => {
      if (cliKey !== "all" && request.cli_key !== cliKey) return false;
      if (statusPredicate && !statusPredicate(null)) return false;
      if (errorNeedle) return false;
      return !pathNeedle || `${request.method} ${request.path}`.toLowerCase().includes(pathNeedle);
    });
  }, [
    activeRequests,
    appliedErrorCodeFilter,
    appliedPathFilter,
    cliKey,
    errorScope,
    hasAppliedTimeRange,
    statusPredicate,
  ]);
  const filteredTraces = useMemo(() => {
    const errorNeedle = appliedErrorCodeFilter.trim().toLowerCase();
    const pathNeedle = appliedPathFilter.trim().toLowerCase();
    return traces.filter((trace) =>
      traceMatchesFilters(
        trace,
        cliKey,
        statusPredicate,
        errorNeedle,
        pathNeedle,
        errorScope,
        appliedTimeFromMs,
        appliedTimeToMs
      )
    );
  }, [
    appliedErrorCodeFilter,
    appliedPathFilter,
    appliedTimeFromMs,
    appliedTimeToMs,
    cliKey,
    errorScope,
    statusPredicate,
    traces,
  ]);

  const totalPageCount = Math.max(1, loadedTotalPages ?? 1);
  const displayedPage = loadedPage ?? page;
  const totalLogCount = totalCount ?? 0;
  const pageInputTarget = parsePageInput(pageInput, totalPageCount);
  const availableSnapshotId = loadedSnapshotId ?? snapshotId;
  const pageOptions = useMemo(
    () => pageSuggestions(displayedPage, totalPageCount),
    [displayedPage, totalPageCount]
  );
  const logsSummaryText =
    requestLogsAvailable === false
      ? undefined
      : requestLogs.length === 0 && requestLogsLoading
        ? "加载中…"
        : requestLogsRefreshing
          ? `更新中… · 第 ${displayedPage} / ${totalPageCount} 页 · 共 ${totalLogCount} 条`
          : `第 ${displayedPage} / ${totalPageCount} 页 · 共 ${totalLogCount} 条 · 本页 ${requestLogs.length} 条`;

  function goToPage(target: number | null) {
    if (target == null || !availableSnapshotId) return;
    dispatch({ type: "goToPage", page: target, snapshotId: availableSnapshotId });
  }

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
                onCheckedChange={(nextAutoRefresh) =>
                  dispatch({ type: "setAutoRefresh", autoRefresh: nextAutoRefresh })
                }
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

        <div className="grid items-start gap-4 md:grid-cols-2 xl:grid-cols-[1.35fr_1fr_1fr]">
          <div className="space-y-2">
            <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              CLI
            </div>
            <TabList
              ariaLabel="CLI 过滤"
              items={LOG_CLI_FILTER_ITEMS.map((item) => ({
                ...item,
                disabled: requestLogsAvailable === false,
              }))}
              value={cliKey}
              onChange={(nextCliKey) => dispatch({ type: "setCliKey", cliKey: nextCliKey })}
              size="sm"
              className="w-full"
              buttonClassName="shrink-0 px-3 py-1.5 whitespace-nowrap"
            />
          </div>

          <div className="space-y-2">
            <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              错误范围
            </div>
            <TabList
              ariaLabel="错误范围过滤"
              items={LOG_ERROR_SCOPE_ITEMS.map((item) => ({
                ...item,
                disabled: requestLogsAvailable === false,
              }))}
              value={errorScope}
              onChange={(nextErrorScope) =>
                dispatch({ type: "setErrorScope", errorScope: nextErrorScope })
              }
              size="sm"
              className="w-full"
              buttonClassName="shrink-0 px-3 py-1.5 whitespace-nowrap"
            />
          </div>

          <div className="space-y-2">
            <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              时间
            </div>
            <Popover
              disabled={requestLogsAvailable === false}
              className="h-9 w-full items-center justify-between rounded-md border border-input bg-background px-3 text-sm font-medium text-foreground shadow-sm transition-colors hover:bg-state-hover focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/30"
              trigger={
                <>
                  <span className="inline-flex items-center gap-2">
                    <CalendarClock className="h-4 w-4 text-muted-foreground" />
                    <span>{hasAppliedTimeRange ? "已设置范围" : "全部时间"}</span>
                  </span>
                  <span className="text-xs text-muted-foreground">分钟</span>
                </>
              }
              align="start"
              contentClassName="w-[min(22rem,calc(100vw-2rem))] p-4"
            >
              <div className="flex flex-col gap-4">
                <div className="grid grid-cols-3 gap-2">
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() =>
                      dispatch({ type: "setQuickTimeRange", range: quickTimeRange("lastHour") })
                    }
                  >
                    1小时内
                  </Button>
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() =>
                      dispatch({ type: "setQuickTimeRange", range: quickTimeRange("today") })
                    }
                  >
                    今天
                  </Button>
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() =>
                      dispatch({ type: "setQuickTimeRange", range: quickTimeRange("yesterday") })
                    }
                  >
                    昨天
                  </Button>
                </div>

                <label className="space-y-1 text-xs font-medium text-muted-foreground">
                  <span>开始</span>
                  <Input
                    aria-label="开始时间"
                    aria-invalid={!timeRangeValid}
                    aria-describedby={!timeRangeValid ? "logs-time-range-error" : undefined}
                    type="datetime-local"
                    step={60}
                    value={timeFromDraft}
                    onChange={(event) =>
                      dispatch({ type: "setTimeFromDraft", value: event.target.value })
                    }
                  />
                </label>
                <label className="space-y-1 text-xs font-medium text-muted-foreground">
                  <span>结束</span>
                  <Input
                    aria-label="结束时间"
                    aria-invalid={!timeRangeValid}
                    aria-describedby={!timeRangeValid ? "logs-time-range-error" : undefined}
                    type="datetime-local"
                    step={60}
                    value={timeToDraft}
                    onChange={(event) =>
                      dispatch({ type: "setTimeToDraft", value: event.target.value })
                    }
                  />
                </label>
                {!timeRangeValid ? (
                  <div
                    id="logs-time-range-error"
                    role="alert"
                    className="text-xs text-rose-600 dark:text-rose-400"
                  >
                    结束时间必须晚于开始时间
                  </div>
                ) : null}
                <div className="flex items-center justify-between gap-2">
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => dispatch({ type: "clearTimeRange" })}
                    disabled={!timeFromDraft && !timeToDraft && !hasAppliedTimeRange}
                  >
                    清除
                  </Button>
                  <Button
                    size="sm"
                    onClick={() => dispatch({ type: "applyTimeRange", range: timeDraft })}
                    disabled={!timeRangeValid}
                  >
                    应用
                  </Button>
                </div>
              </div>
            </Popover>
          </div>
        </div>

        <div className="grid items-start gap-4 md:grid-cols-2 xl:grid-cols-3">
          <div className="space-y-2">
            <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              Status
            </div>
            <Input
              value={statusFilter}
              onChange={(event) =>
                dispatch({ type: "setStatusFilter", statusFilter: event.target.value })
              }
              placeholder="例：499 / 524 / !200 / >=400"
              mono
              disabled={requestLogsAvailable === false}
            />
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
              onChange={(event) =>
                dispatch({ type: "setErrorCodeFilter", errorCodeFilter: event.target.value })
              }
              placeholder={`例：${GatewayErrorCodes.UPSTREAM_TIMEOUT}`}
              mono
              disabled={requestLogsAvailable === false}
            />
          </div>

          <div className="space-y-2">
            <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              Path
            </div>
            <Input
              value={pathFilter}
              onChange={(event) =>
                dispatch({ type: "setPathFilter", pathFilter: event.target.value })
              }
              placeholder="例：/v1/messages"
              mono
              disabled={requestLogsAvailable === false}
            />
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
            <Button
              variant="secondary"
              size="icon"
              aria-label="刷新并重建分页"
              title="刷新并重建分页"
              onClick={() => dispatch({ type: "refreshSnapshot" })}
              disabled={requestLogsPageFetching || requestLogsAvailable === false}
            >
              <RefreshCw className="h-4 w-4" />
            </Button>
            <Button
              variant="secondary"
              size="icon"
              aria-label="上一页"
              title="上一页"
              onClick={() => goToPage(displayedPage - 1)}
              disabled={
                displayedPage <= 1 || requestLogsPageFetching || requestLogsAvailable === false
              }
            >
              <ChevronLeft className="h-4 w-4" />
            </Button>
            <label className="flex items-center gap-1 text-xs tabular-nums text-muted-foreground">
              <span>第</span>
              <Input
                aria-label="跳转页码"
                type="number"
                min={1}
                max={totalPageCount}
                list="request-log-page-options"
                value={pageInput}
                onChange={(event) =>
                  dispatch({ type: "setPageInput", pageInput: event.target.value })
                }
                onKeyDown={(event) => {
                  if (event.key === "Enter") goToPage(pageInputTarget);
                }}
                className="h-8 w-20 text-center"
                disabled={requestLogsPageFetching || requestLogsAvailable === false}
              />
              <datalist id="request-log-page-options">
                {pageOptions.map((option) => (
                  <option key={option} value={option} />
                ))}
              </datalist>
              <span>/ {totalPageCount} 页</span>
            </label>
            <Button
              variant="secondary"
              size="sm"
              onClick={() => goToPage(pageInputTarget)}
              disabled={
                pageInputTarget == null || requestLogsPageFetching || requestLogsAvailable === false
              }
            >
              跳转
            </Button>
            <Button
              variant="secondary"
              size="icon"
              aria-label="下一页"
              title="下一页"
              onClick={() => goToPage(displayedPage + 1)}
              disabled={
                displayedPage >= totalPageCount ||
                requestLogsPageFetching ||
                requestLogsAvailable === false
              }
            >
              <ChevronRight className="h-4 w-4" />
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
        onRefreshRequestLogs={() => dispatch({ type: "refreshSnapshot" })}
        selectedLogId={selectedLogId}
        onSelectLogId={setSelectedLogId}
      />

      <RequestLogDetailDialog selectedLogId={selectedLogId} onSelectLogId={setSelectedLogId} />
    </div>
  );
}
