// Usage:
// - 链路 tooltip 的富文本内容组件，展示请求路径概览 + 每个 provider 的尝试详情。
// - 由 `buildRequestRouteMeta` 在 requestLogPresentation.ts 中调用。
// - skipped provider 也是 route hop，并在详情中明确标记为未发出上游请求。

import type { RequestLogRouteHop } from "../../services/gateway/requestLogs";
import { cn } from "../../utils/cn";
import { getErrorCodeLabel } from "./requestLogErrorLabels";

type RouteTooltipContentProps = {
  hops: RequestLogRouteHop[];
  finalStatus: number | null;
  summary?: string;
};

function resolveProviderName(raw: string | undefined | null): string {
  const trimmed = raw?.trim();
  return !trimmed || trimmed === "Unknown" ? "未知" : trimmed;
}

function normalizeAttempts(value: number | null | undefined): number {
  return Number.isSafeInteger(value) && value != null && value > 0 && value <= 9_999 ? value : 1;
}

function resolveDecisionLabel(
  decision: string | null | undefined,
  input: { skipped: boolean; ok: boolean; retryCount: number }
): string | null {
  const normalized = decision?.trim().toLowerCase();
  if (!normalized) return null;
  if ((normalized === "skip" && input.skipped) || (normalized === "success" && input.ok)) {
    return null;
  }
  if (input.retryCount > 0 && (normalized === "retry" || normalized === "retry_same_provider")) {
    return null;
  }
  const knownLabels: Record<string, string> = {
    abort: "停止重试",
    failover: "切换供应商",
    switch: "切换供应商",
    switch_provider: "切换供应商",
  };
  return knownLabels[normalized] ?? decision?.trim() ?? null;
}

function resolveVisibleReason(hop: RequestLogRouteHop, status: number | null): string | null {
  const reason = hop.reason?.trim();
  if (!reason) return null;
  const normalized = reason.toLowerCase();
  if (status != null && normalized === `status=${status}`) return null;
  if (
    hop.error_code === "GW_PROVIDER_RATE_LIMITED" &&
    normalized === "provider skipped by rate limit"
  ) {
    return null;
  }
  if (
    hop.error_code === "GW_PROVIDER_CIRCUIT_OPEN" &&
    (normalized === "provider skipped by circuit breaker" ||
      normalized.startsWith("provider skipped by circuit breaker ("))
  ) {
    return null;
  }
  return reason;
}

export function RouteTooltipContent({ hops, finalStatus, summary }: RouteTooltipContentProps) {
  if (hops.length === 0) return null;

  return (
    <div className="flex min-w-0 flex-col gap-2 py-0.5">
      {summary ? (
        <div className="rounded-md bg-secondary/80 px-2 py-1.5 text-[11px] leading-relaxed text-foreground">
          {summary}
        </div>
      ) : null}

      <div className="flex items-start gap-1 text-[11px] font-medium text-foreground">
        <span className="shrink-0 pt-px text-muted-foreground">链路</span>
        <span className="flex items-center gap-1 flex-wrap">
          {hops.map((hop, idx) => {
            const name = resolveProviderName(hop.provider_name);
            return (
              <span
                key={`${hop.provider_id}-${hop.status ?? "pending"}-${hop.error_code ?? "ok"}-${name}-${idx}`}
                className="flex items-center gap-1"
              >
                {idx > 0 && <span className="text-muted-foreground">→</span>}
                <span className="text-foreground">{name}</span>
              </span>
            );
          })}
        </span>
      </div>

      <div className="border-t border-border/80" />

      <div className="flex flex-col gap-1.5">
        {hops.map((hop, idx) => (
          <RouteHopRow
            key={`${hop.provider_id}-${hop.status ?? "pending"}-${hop.error_code ?? "ok"}-${idx}`}
            hop={hop}
            isLast={idx === hops.length - 1}
            finalStatus={finalStatus}
            index={idx}
            totalHops={hops.length}
          />
        ))}
      </div>
    </div>
  );
}

// ── 单个 hop 行 ──────────────────────────────────────────────

type RouteHopRowProps = {
  hop: RequestLogRouteHop;
  index: number;
  isLast: boolean;
  finalStatus: number | null;
  totalHops: number;
};

function RouteHopRow({ hop, index, isLast, finalStatus, totalHops }: RouteHopRowProps) {
  const providerName = resolveProviderName(hop.provider_name);
  const status = hop.status ?? (isLast ? finalStatus : null) ?? null;
  const attemptCount = normalizeAttempts(hop.attempts);
  const retryCount = Math.max(attemptCount - 1, 0);
  const errorCode = hop.error_code ?? null;
  const errorLabel = errorCode ? getErrorCodeLabel(errorCode) : null;
  const skipped = hop.skipped === true;
  const decisionLabel = resolveDecisionLabel(hop.decision, {
    skipped,
    ok: hop.ok,
    retryCount,
  });
  const visibleReason = resolveVisibleReason(hop, status);

  const statusLabel = skipped
    ? "已跳过"
    : hop.ok
      ? attemptCount > 1
        ? `成功（重试 ${retryCount} 次）`
        : "成功"
      : attemptCount > 1
        ? `失败（重试 ${retryCount} 次）`
        : "失败";

  const statusTone = skipped
    ? "bg-muted text-muted-foreground ring-1 ring-inset ring-border/70"
    : hop.ok
      ? "bg-emerald-50 text-emerald-700 ring-1 ring-inset ring-emerald-500/15 dark:bg-emerald-500/15 dark:text-emerald-300 dark:ring-emerald-400/20"
      : "bg-rose-50 text-rose-700 ring-1 ring-inset ring-rose-500/15 dark:bg-rose-500/15 dark:text-rose-300 dark:ring-rose-400/20";

  const dotTone = skipped
    ? "bg-muted text-muted-foreground ring-1 ring-border"
    : hop.ok
      ? "bg-emerald-50 text-emerald-700 ring-1 ring-emerald-500/20 dark:bg-emerald-500/15 dark:text-emerald-300 dark:ring-emerald-400/25"
      : "bg-rose-50 text-rose-700 ring-1 ring-rose-500/20 dark:bg-rose-500/15 dark:text-rose-300 dark:ring-rose-400/25";

  return (
    <div className="flex items-start gap-2">
      <div className="flex flex-col items-center shrink-0 pt-0.5">
        <span
          className={cn(
            "inline-flex items-center justify-center h-4 w-4 rounded-full text-[9px] font-bold",
            dotTone
          )}
        >
          {index + 1}
        </span>
        {!isLast && totalHops > 1 && <div className="w-px h-3 bg-secondary mt-0.5" />}
      </div>

      <div className="flex flex-col gap-0.5 min-w-0 flex-1">
        <div className="flex min-w-0 items-center gap-1.5">
          <span className="truncate text-[11px] font-medium text-foreground" title={providerName}>
            {providerName}
          </span>
          <span
            className={cn(
              "inline-flex items-center rounded px-1 py-px text-[10px] font-medium shrink-0",
              statusTone
            )}
          >
            {statusLabel}
          </span>
        </div>

        {(status != null || errorLabel || decisionLabel || visibleReason || skipped) && (
          <div className="min-w-0 space-y-0.5 text-[10px]">
            <div className="flex flex-wrap items-center gap-x-1.5 gap-y-0.5">
              {status != null && !skipped && (
                <span
                  className={cn(
                    "font-mono tabular-nums",
                    hop.ok
                      ? "text-emerald-700 dark:text-emerald-300"
                      : "text-rose-700 dark:text-rose-300"
                  )}
                >
                  {status}
                </span>
              )}
              {errorLabel && (
                <span className="text-amber-700 dark:text-amber-300">{errorLabel}</span>
              )}
              {decisionLabel && (
                <span className="max-w-full break-all text-muted-foreground">{decisionLabel}</span>
              )}
              {skipped && <span className="text-muted-foreground">未发送</span>}
            </div>
            {visibleReason && (
              <div className="break-words leading-relaxed text-muted-foreground">
                {visibleReason}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
