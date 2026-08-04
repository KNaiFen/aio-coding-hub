import { useCallback, useEffect, useMemo } from "react";
import { gatewayEventNames } from "../constants/gatewayEvents";
import {
  useActiveRequestLogsSnapshotQuery,
  isRequestLogSnapshotExpiredError,
  useRequestLogsSnapshotPageAllQuery,
} from "../query/requestLogs";
import { logToConsole } from "../services/consoleLog";
import { subscribeGatewayEvent } from "../services/gateway/gatewayEventBus";
import { normalizeGatewayRequestSignalEvent } from "../services/gateway/gatewayEvents";
import { isRequestSignalComplete } from "../services/gateway/requestLogState";
import type { RequestLogPageFilters } from "../services/gateway/requestLogs";
import { useCoalescedAsyncRefresh } from "./useCoalescedAsyncRefresh";
import { useDocumentVisibility } from "./useDocumentVisibility";
import { useWindowForeground } from "./useWindowForeground";

const ACTIVE_REQUEST_SIGNAL_REFRESH_WINDOW_MS = 200;

type UseRequestLogsPageFeedOptions = {
  filters: RequestLogPageFilters;
  snapshotId: string | null;
  page: number;
  snapshotRevision: number;
  limit: number;
  enabled?: boolean;
  liveUpdatesEnabled?: boolean;
  liveUpdateWindowMs?: number;
  refreshOnForeground?: boolean;
  foregroundThrottleMs?: number;
  onRefreshSnapshot: () => void;
};

function normalizeRefreshWindowMs(value: number | undefined) {
  if (!Number.isFinite(value) || value == null) return 400;
  return Math.max(200, Math.min(2_000, Math.trunc(value)));
}

export function useRequestLogsPageFeed({
  filters,
  snapshotId,
  page,
  snapshotRevision,
  limit,
  enabled = true,
  liveUpdatesEnabled = false,
  liveUpdateWindowMs,
  refreshOnForeground = false,
  foregroundThrottleMs = 1000,
  onRefreshSnapshot,
}: UseRequestLogsPageFeedOptions) {
  const foregroundActive = useDocumentVisibility();
  const pageQuery = useRequestLogsSnapshotPageAllQuery(
    filters,
    snapshotId,
    page,
    limit,
    snapshotRevision,
    { enabled }
  );
  const activeRequestsQuery = useActiveRequestLogsSnapshotQuery({ enabled });
  const latestPage = page === 1;
  const signalSubscriptionEnabled = enabled && liveUpdatesEnabled;
  const liveRefreshEnabled = signalSubscriptionEnabled && foregroundActive;
  const pageLiveRefreshEnabled = liveRefreshEnabled && latestPage;
  const refreshWindowMs = normalizeRefreshWindowMs(liveUpdateWindowMs);

  const refreshActiveRequests = useCallback(
    () => activeRequestsQuery.refetch(),
    [activeRequestsQuery]
  );
  const refreshRequestLogs = useCallback(() => {
    return Promise.all([pageQuery.refetch(), activeRequestsQuery.refetch()]).then(
      ([pageResult]) => pageResult
    );
  }, [activeRequestsQuery, pageQuery]);

  const { schedule: scheduleActiveRequestsRefresh } = useCoalescedAsyncRefresh<
    "start" | "complete",
    unknown
  >({
    enabled: liveRefreshEnabled,
    delayMs: ACTIVE_REQUEST_SIGNAL_REFRESH_WINDOW_MS,
    task: async () => {
      await refreshActiveRequests();
    },
    onError: (error) => {
      logToConsole("warn", "刷新进行中请求快照失败", { error: String(error) });
      return null;
    },
  });
  const { schedule: schedulePageRefresh } = useCoalescedAsyncRefresh<void, unknown>({
    enabled: pageLiveRefreshEnabled,
    delayMs: refreshWindowMs,
    task: async () => {
      onRefreshSnapshot();
    },
    onError: (error) => {
      logToConsole("warn", "刷新请求日志当前页失败", { error: String(error) });
      return null;
    },
  });

  const refreshForForeground = useCallback(() => {
    if (!enabled) return;
    if (latestPage) {
      onRefreshSnapshot();
    }
    void refreshActiveRequests();
  }, [enabled, latestPage, onRefreshSnapshot, refreshActiveRequests]);

  useWindowForeground({
    enabled: enabled && refreshOnForeground,
    throttleMs: foregroundThrottleMs,
    onForeground: refreshForForeground,
  });

  useEffect(() => {
    if (!signalSubscriptionEnabled) return;

    let cancelled = false;
    const requestSignalSub = subscribeGatewayEvent(gatewayEventNames.requestSignal, (payload) => {
      const requestSignal = normalizeGatewayRequestSignalEvent(payload);
      if (cancelled || !requestSignal) return;

      scheduleActiveRequestsRefresh(requestSignal.phase);
      if (isRequestSignalComplete(requestSignal)) {
        schedulePageRefresh();
      }
    });

    void requestSignalSub.ready.catch((error) => {
      if (cancelled) return;
      requestSignalSub.unsubscribe();
      logToConsole("warn", "请求记录实时监听初始化失败", {
        stage: "useRequestLogsPageFeed",
        error: String(error),
      });
    });

    return () => {
      cancelled = true;
      requestSignalSub.unsubscribe();
    };
  }, [scheduleActiveRequestsRefresh, schedulePageRefresh, signalSubscriptionEnabled]);

  const pageTransitionLoading = pageQuery.isLoading || pageQuery.isPlaceholderData;
  const requestLogs = useMemo(
    () => (pageQuery.isPlaceholderData ? [] : (pageQuery.data?.items ?? [])),
    [pageQuery.data, pageQuery.isPlaceholderData]
  );
  const activeRequests = useMemo(() => activeRequestsQuery.data ?? [], [activeRequestsQuery.data]);
  const activeRequestsAvailable: boolean | null = !enabled
    ? null
    : activeRequestsQuery.isLoading
      ? null
      : activeRequestsQuery.isError
        ? false
        : activeRequestsQuery.data != null;
  const requestLogsLoading = pageTransitionLoading;
  const requestLogsRefreshing =
    (pageQuery.isFetching && !pageTransitionLoading) ||
    (activeRequestsQuery.isFetching && !activeRequestsQuery.isLoading);
  const requestLogsAvailable: boolean | null = pageTransitionLoading
    ? null
    : pageQuery.data != null;

  return {
    requestLogs,
    snapshotId: pageQuery.isPlaceholderData ? null : (pageQuery.data?.snapshotId ?? null),
    totalCount: pageQuery.isPlaceholderData ? null : (pageQuery.data?.totalCount ?? null),
    totalPages: pageQuery.isPlaceholderData ? null : (pageQuery.data?.totalPages ?? null),
    page: pageQuery.isPlaceholderData ? null : (pageQuery.data?.page ?? null),
    activeRequests,
    activeRequestsAvailable,
    requestLogsLoading,
    requestLogsRefreshing,
    requestLogsPageFetching: pageQuery.isFetching,
    requestLogsAvailable,
    requestLogsSnapshotExpired: isRequestLogSnapshotExpiredError(pageQuery.error),
    refreshActiveRequests,
    refreshRequestLogs,
  };
}
