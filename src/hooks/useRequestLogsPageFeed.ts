import { useCallback, useEffect, useMemo } from "react";
import { gatewayEventNames } from "../constants/gatewayEvents";
import {
  useActiveRequestLogsSnapshotQuery,
  useRequestLogsPageAllQuery,
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
  cursor: string | null;
  limit: number;
  enabled?: boolean;
  liveUpdatesEnabled?: boolean;
  liveUpdateWindowMs?: number;
  refreshOnForeground?: boolean;
  foregroundThrottleMs?: number;
};

function normalizeRefreshWindowMs(value: number | undefined) {
  if (!Number.isFinite(value) || value == null) return 400;
  return Math.max(200, Math.min(2_000, Math.trunc(value)));
}

export function useRequestLogsPageFeed({
  filters,
  cursor,
  limit,
  enabled = true,
  liveUpdatesEnabled = false,
  liveUpdateWindowMs,
  refreshOnForeground = false,
  foregroundThrottleMs = 1000,
}: UseRequestLogsPageFeedOptions) {
  const foregroundActive = useDocumentVisibility();
  const pageQuery = useRequestLogsPageAllQuery(filters, cursor, limit, { enabled });
  const activeRequestsQuery = useActiveRequestLogsSnapshotQuery({ enabled });
  const latestPage = cursor == null;
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
      await refreshRequestLogs();
    },
    onError: (error) => {
      logToConsole("warn", "刷新请求日志当前页失败", { error: String(error) });
      return null;
    },
  });

  const refreshForForeground = useCallback(() => {
    if (!enabled) return;
    if (latestPage) {
      void refreshRequestLogs();
      return;
    }
    void refreshActiveRequests();
  }, [enabled, latestPage, refreshActiveRequests, refreshRequestLogs]);

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
    nextCursor: pageQuery.isPlaceholderData ? null : (pageQuery.data?.nextCursor ?? null),
    activeRequests,
    activeRequestsAvailable,
    requestLogsLoading,
    requestLogsRefreshing,
    requestLogsPageFetching: pageQuery.isFetching,
    requestLogsAvailable,
    refreshActiveRequests,
    refreshRequestLogs,
  };
}
