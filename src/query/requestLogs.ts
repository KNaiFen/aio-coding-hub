import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  REQUEST_ATTEMPT_LOGS_DEFAULT_LIMIT,
  REQUEST_LOGS_DEFAULT_LIMIT,
  REQUEST_LOGS_PAGE_DEFAULT_LIMIT,
  requestAttemptLogsByTraceId,
  requestLogGet,
  requestLogsListAfterIdAll,
  requestLogsListAll,
  requestLogsPageAll,
  requestLogsSnapshotPageAll,
  normalizeRequestAttemptLogsLimit,
  normalizeRequestLogTraceIdOrNull,
  normalizeRequestLogsPageLimit,
  normalizeRequestLogsLimit,
  type RequestLogPage,
  type RequestLogPageFilters,
  type RequestLogSnapshotPage,
  type RequestLogSummary,
} from "../services/gateway/requestLogs";
import { activeRequestLogsSnapshot, type ActiveRequest } from "../services/gateway/activeRequests";
import {
  isPersistedRequestLogIncomplete,
  requestLogCreatedAtMs,
} from "../services/gateway/requestLogState";
import { logToConsole } from "../services/consoleLog";
import { requestLogsKeys } from "./keys";

type RequestLogsIncrementalRefreshResult = {
  mode: "full" | "incremental";
  items: RequestLogSummary[];
};

export const REQUEST_LOG_DETAIL_STALE_TIME_MS = 0;
export const REQUEST_LOG_DETAIL_GC_TIME_MS = 60 * 1000;

function isRequestLogsQueryEnabled(enabled: boolean | undefined) {
  return enabled ?? true;
}

export function isRequestLogSnapshotExpiredError(error: unknown) {
  return String(error).includes("REQUEST_LOG_SNAPSHOT_EXPIRED:");
}

function sortRequestLogsDesc(a: RequestLogSummary, b: RequestLogSummary) {
  const aTsMs = requestLogCreatedAtMs(a);
  const bTsMs = requestLogCreatedAtMs(b);
  if (aTsMs !== bTsMs) return bTsMs - aTsMs;
  return b.id - a.id;
}

function computeRequestLogsCursorId(rows: RequestLogSummary[]) {
  let maxId = 0;
  for (const row of rows) {
    if (Number.isFinite(row.id) && row.id > maxId) maxId = row.id;
  }
  return maxId;
}

function shouldUseFullRefresh(prev: RequestLogSummary[] | null | undefined) {
  if (!prev?.length) return true;
  return prev.some(isPersistedRequestLogIncomplete);
}

function mergeRequestLogs(prev: RequestLogSummary[], incoming: RequestLogSummary[], limit: number) {
  const byId = new Map<number, RequestLogSummary>();
  for (const row of incoming) byId.set(row.id, row);
  for (const row of prev) {
    if (!byId.has(row.id)) byId.set(row.id, row);
  }
  const merged = Array.from(byId.values());
  merged.sort(sortRequestLogsDesc);
  return merged.slice(0, limit);
}

function capRequestLogs(rows: RequestLogSummary[], limit: number) {
  return rows.slice().sort(sortRequestLogsDesc).slice(0, limit);
}

export function useRequestLogsListAllQuery(
  limit?: number | null,
  options?: { enabled?: boolean; refetchIntervalMs?: number | false }
) {
  const enabled = isRequestLogsQueryEnabled(options?.enabled);
  const normalizedLimit = normalizeRequestLogsLimit(limit) ?? REQUEST_LOGS_DEFAULT_LIMIT;

  return useQuery<RequestLogSummary[]>({
    queryKey: requestLogsKeys.listAll(normalizedLimit),
    queryFn: async () => {
      const rows = await requestLogsListAll(normalizedLimit);
      return capRequestLogs(rows, normalizedLimit);
    },
    enabled,
    placeholderData: keepPreviousData,
    refetchInterval: options?.refetchIntervalMs ?? false,
  });
}

export function useRequestLogsPageAllQuery(
  filters: RequestLogPageFilters,
  cursor: string | null,
  limit?: number | null,
  options?: { enabled?: boolean }
) {
  const enabled = isRequestLogsQueryEnabled(options?.enabled);
  const normalizedLimit = normalizeRequestLogsPageLimit(limit) ?? REQUEST_LOGS_PAGE_DEFAULT_LIMIT;

  return useQuery<RequestLogPage>({
    queryKey: requestLogsKeys.pageAll(filters, cursor, normalizedLimit),
    queryFn: () => requestLogsPageAll(filters, cursor, normalizedLimit),
    enabled,
    placeholderData: keepPreviousData,
    // The latest page is a moving head. Keep it stale so returning from a
    // stable history cursor fetches completions that arrived while inactive.
    ...(cursor == null ? { staleTime: 0 } : {}),
  });
}

export function useRequestLogsSnapshotPageAllQuery(
  filters: RequestLogPageFilters,
  snapshotId: string | null,
  page: number,
  limit: number | null | undefined,
  revision: number,
  options?: { enabled?: boolean }
) {
  const enabled = isRequestLogsQueryEnabled(options?.enabled);
  const normalizedLimit = normalizeRequestLogsPageLimit(limit) ?? REQUEST_LOGS_PAGE_DEFAULT_LIMIT;

  return useQuery<RequestLogSnapshotPage>({
    queryKey: requestLogsKeys.snapshotPageAll(filters, snapshotId, page, normalizedLimit, revision),
    queryFn: () => requestLogsSnapshotPageAll(filters, snapshotId, page, normalizedLimit),
    enabled,
    placeholderData: keepPreviousData,
    retry: false,
  });
}

export function useActiveRequestLogsSnapshotQuery(options?: { enabled?: boolean }) {
  const enabled = isRequestLogsQueryEnabled(options?.enabled);

  return useQuery<ActiveRequest[]>({
    queryKey: requestLogsKeys.activeSnapshot(),
    queryFn: async () => {
      try {
        return await activeRequestLogsSnapshot();
      } catch (error) {
        logToConsole("warn", "读取进行中请求快照失败", { error: String(error) });
        throw error;
      }
    },
    enabled,
    placeholderData: keepPreviousData,
  });
}

export function useRequestLogsIncrementalRefreshMutation(limit?: number | null) {
  const queryClient = useQueryClient();
  const normalizedLimit = normalizeRequestLogsLimit(limit) ?? REQUEST_LOGS_DEFAULT_LIMIT;

  return useMutation<RequestLogsIncrementalRefreshResult>({
    mutationFn: async () => {
      const prev = queryClient.getQueryData<RequestLogSummary[] | null>(
        requestLogsKeys.listAll(normalizedLimit)
      );
      const cursorId = prev?.length ? computeRequestLogsCursorId(prev) : 0;
      const useFullRefresh = shouldUseFullRefresh(prev);

      if (useFullRefresh) {
        const items = await requestLogsListAll(normalizedLimit);
        return { mode: "full" as const, items: capRequestLogs(items, normalizedLimit) };
      }

      const items = await requestLogsListAfterIdAll(cursorId, normalizedLimit);
      return { mode: "incremental" as const, items: capRequestLogs(items, normalizedLimit) };
    },
    onSuccess: (result) => {
      if (!result) return;

      if (result.mode === "full") {
        queryClient.setQueryData(requestLogsKeys.listAll(normalizedLimit), result.items);
        return;
      }

      if (result.items.length === 0) return;

      queryClient.setQueryData<RequestLogSummary[]>(
        requestLogsKeys.listAll(normalizedLimit),
        (cur) => mergeRequestLogs(cur ?? [], result.items, normalizedLimit)
      );
    },
  });
}

export function useRequestLogDetailQuery(logId: number | null) {
  return useQuery({
    queryKey: requestLogsKeys.detail(logId),
    queryFn: () => {
      if (logId == null) return null;
      return requestLogGet(logId);
    },
    enabled: logId != null,
    placeholderData: keepPreviousData,
    staleTime: REQUEST_LOG_DETAIL_STALE_TIME_MS,
    gcTime: REQUEST_LOG_DETAIL_GC_TIME_MS,
  });
}

export function useRequestAttemptLogsByTraceIdQuery(traceId: string | null, limit?: number | null) {
  const normalizedTraceId = normalizeRequestLogTraceIdOrNull(traceId);
  const normalizedLimit =
    normalizeRequestAttemptLogsLimit(limit) ?? REQUEST_ATTEMPT_LOGS_DEFAULT_LIMIT;

  return useQuery({
    queryKey: requestLogsKeys.attemptsByTrace(normalizedTraceId, normalizedLimit),
    queryFn: () => {
      if (!normalizedTraceId) return null;
      return requestAttemptLogsByTraceId(normalizedTraceId, normalizedLimit);
    },
    enabled: Boolean(normalizedTraceId),
    placeholderData: keepPreviousData,
    staleTime: REQUEST_LOG_DETAIL_STALE_TIME_MS,
    gcTime: REQUEST_LOG_DETAIL_GC_TIME_MS,
  });
}
