import type {
  ProviderAvailabilityBucket,
  ProviderAvailabilityState,
  ProviderAvailabilityTimeline,
} from "../../generated/bindings";
import { Tooltip } from "../../ui/Tooltip";
import { cn } from "../../utils/cn";

const DESKTOP_BUCKET_COUNT = 36;
const AVAILABILITY_HOURS = new Set([3, 6, 12]);
type ProviderAvailabilityRenderState = ProviderAvailabilityState | "degraded";
const AVAILABILITY_STATES = new Set<ProviderAvailabilityRenderState>([
  "healthy",
  "degraded",
  "unhealthy",
  "no_data",
]);
const TIME_FORMATTER = new Intl.DateTimeFormat("zh-CN", {
  month: "2-digit",
  day: "2-digit",
  hour: "2-digit",
  minute: "2-digit",
  hour12: false,
});

function validCount(value: number) {
  return Number.isSafeInteger(value) && value >= 0;
}

function validTimestamp(value: number) {
  return Number.isSafeInteger(value) && Math.abs(value) <= 8_640_000_000_000_000;
}

function validBucket(bucket: ProviderAvailabilityBucket) {
  if (!bucket || typeof bucket !== "object") return false;
  return (
    validTimestamp(bucket.start_at_ms) &&
    validTimestamp(bucket.end_at_ms) &&
    bucket.end_at_ms > bucket.start_at_ms &&
    validCount(bucket.success_count) &&
    validCount(bucket.failure_count) &&
    AVAILABILITY_STATES.has(bucket.state as ProviderAvailabilityRenderState)
  );
}

export function normalizeProviderAvailabilityTimeline(
  timeline: ProviderAvailabilityTimeline | null | undefined
) {
  if (!timeline) return null;
  if (!Number.isSafeInteger(timeline.provider_id) || timeline.provider_id <= 0) return null;
  if (!AVAILABILITY_HOURS.has(timeline.hours)) return null;
  if (timeline.bucket_count !== DESKTOP_BUCKET_COUNT) return null;
  if (timeline.bucket_minutes !== (timeline.hours * 60) / DESKTOP_BUCKET_COUNT) return null;
  if (!validCount(timeline.success_count) || !validCount(timeline.failure_count)) return null;
  if (!Array.isArray(timeline.buckets)) return null;
  if (timeline.buckets.length !== DESKTOP_BUCKET_COUNT) return null;
  if (!timeline.buckets.every(validBucket)) return null;
  if (
    timeline.buckets.some(
      (bucket, index) => index > 0 && timeline.buckets[index - 1].end_at_ms !== bucket.start_at_ms
    )
  ) {
    return null;
  }
  return timeline;
}

function stateLabel(state: ProviderAvailabilityRenderState) {
  switch (state) {
    case "healthy":
      return "高可用";
    case "degraded":
      return "可用性降级";
    case "unhealthy":
      return "低可用";
    default:
      return "无数据";
  }
}

function stateClassName(state: ProviderAvailabilityRenderState) {
  switch (state) {
    case "healthy":
      return "bg-success ring-1 ring-emerald-700/15 dark:ring-emerald-200/15";
    case "degraded":
      return "bg-warning ring-1 ring-amber-700/15 dark:ring-amber-200/15";
    case "unhealthy":
      return "bg-danger ring-1 ring-rose-800/15 dark:ring-rose-100/15";
    default:
      return "bg-surface-muted ring-1 ring-inset ring-line-subtle";
  }
}

function bucketTooltip(bucket: ProviderAvailabilityBucket) {
  const total = bucket.success_count + bucket.failure_count;
  const successRate = total > 0 ? `${((bucket.success_count / total) * 100).toFixed(1)}%` : "—";
  return (
    <div className="space-y-1 text-xs">
      <div className="font-medium text-popover-foreground">
        {TIME_FORMATTER.format(new Date(bucket.start_at_ms))} -{" "}
        {TIME_FORMATTER.format(new Date(bucket.end_at_ms))}
      </div>
      <div className="text-muted-foreground">
        {stateLabel(bucket.state)} · 成功 {bucket.success_count} · 失败 {bucket.failure_count}
      </div>
      <div className="text-muted-foreground">成功率 {successRate}</div>
    </div>
  );
}

export function ProviderAvailabilityStrip({
  timeline,
  providerName,
  className,
}: {
  timeline: ProviderAvailabilityTimeline | null | undefined;
  providerName: string;
  className?: string;
}) {
  const normalized = normalizeProviderAvailabilityTimeline(timeline);
  if (!normalized) return null;

  return (
    <section
      aria-label={`${providerName} 过去 ${normalized.hours} 小时可用性`}
      className={cn("w-full border-t border-line-subtle pt-2 sm:basis-full", className)}
    >
      <div className="mb-1.5 flex items-center justify-between gap-3 text-[11px] text-muted-foreground">
        <span className="font-mono">{normalized.hours}h</span>
        <span className="font-mono tabular-nums">
          成功 {normalized.success_count} · 失败 {normalized.failure_count}
        </span>
      </div>
      <div
        className="grid w-full grid-cols-[repeat(36,minmax(3px,1fr))] gap-[2px]"
        data-testid="provider-availability-cells"
      >
        {normalized.buckets.map((bucket) => {
          const label = `${TIME_FORMATTER.format(new Date(bucket.start_at_ms))} 至 ${TIME_FORMATTER.format(
            new Date(bucket.end_at_ms)
          )}，${stateLabel(bucket.state)}，成功 ${bucket.success_count}，失败 ${bucket.failure_count}`;
          return (
            <Tooltip
              key={`${bucket.start_at_ms}-${bucket.end_at_ms}`}
              content={bucketTooltip(bucket)}
              surface="panel"
              contentClassName="w-56"
              collisionPadding={12}
            >
              <span
                role="img"
                tabIndex={0}
                aria-label={label}
                data-state={bucket.state}
                className={cn(
                  "block h-2.5 min-w-0 rounded-[2px] outline-none transition-[filter,box-shadow] hover:brightness-95 focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1",
                  stateClassName(bucket.state)
                )}
              />
            </Tooltip>
          );
        })}
      </div>
    </section>
  );
}
