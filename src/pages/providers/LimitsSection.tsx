import {
  ChevronDown,
  Clock,
  DollarSign,
  CalendarDays,
  CalendarRange,
  Gauge,
  RotateCcw,
  RefreshCw,
} from "lucide-react";
import { useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { ConfirmDialog } from "../../ui/ConfirmDialog";
import { Tooltip } from "../../ui/Tooltip";
import { Button } from "../../ui/Button";
import { formatUnknownError } from "../../utils/errors";
import { formatUsdRaw } from "../../utils/formatters";
import {
  ProviderLimitRefreshError,
  useProviderLimitResetMutation,
  useProviderLimitUsageV1Query,
} from "../../query/providerLimitUsage";
import type { ProviderLimitPeriod } from "../../services/providers/providerLimitUsage";
import { providerLimitUsageKeys } from "../../query/keys";
import { Input } from "../../ui/Input";
import { LimitCard } from "./LimitCard";
import { RadioButtonGroup } from "./RadioButtonGroup";
import type { DailyResetMode } from "./providerEditorUtils";
import type { UseProviderEditorFormReturn } from "./useProviderEditorForm";

export function LimitsSection(props: { form: UseProviderEditorFormReturn }) {
  const {
    register,
    setValue,
    saving: formSaving,
    dailyResetMode,
    limit5hUsd,
    limitDailyUsd,
    limitWeeklyUsd,
    limitMonthlyUsd,
    limitTotalUsd,
  } = props.form;
  const {
    editingProviderId,
    editProviderName,
    cliKey,
    open,
    limitResetPending,
    setLimitResetPending,
  } = props.form;
  const saving = formSaving || limitResetPending;
  const query = useProviderLimitUsageV1Query(cliKey, { enabled: open && editingProviderId != null });
  const resetMutation = useProviderLimitResetMutation();
  const queryClient = useQueryClient();
  const [period, setPeriod] = useState<ProviderLimitPeriod | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [committed, setCommitted] = useState(false);
  const inFlight = useRef(false);
  const usage = query.data?.find((row) => row.provider_id === editingProviderId);
  const labels: Record<ProviderLimitPeriod, string> = {
    "5h": "5 小时",
    daily: "每日",
    weekly: "每周",
    monthly: "每月",
  };

  async function confirmReset() {
    if (period == null || editingProviderId == null || inFlight.current) return;
    inFlight.current = true;
    setLimitResetPending(true);
    setError(null);
    try {
      if (committed) {
        await queryClient.invalidateQueries(
          { queryKey: providerLimitUsageKeys.all },
          { throwOnError: true }
        );
      } else {
        await resetMutation.mutateAsync({ providerId: editingProviderId, period });
      }
      toast(`${labels[period]}周期已重设`);
      setPeriod(null);
      setCommitted(false);
    } catch (failure) {
      const refreshFailed = committed || failure instanceof ProviderLimitRefreshError;
      setCommitted(refreshFailed);
      setError(
        `${refreshFailed ? "周期已重设，用量刷新失败" : "重设失败"}：${formatUnknownError(failure)}`
      );
    } finally {
      inFlight.current = false;
      setLimitResetPending(false);
    }
  }

  function action(selected: ProviderLimitPeriod) {
    if (editingProviderId == null) return undefined;
    return (
      <Tooltip content={`重设${labels[selected]}周期`}>
        <button
          type="button"
          aria-label={`重设${labels[selected]}周期`}
          disabled={saving}
          className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md text-muted-foreground hover:bg-secondary hover:text-foreground disabled:opacity-50"
          onClick={() => {
            setPeriod(selected);
            setError(null);
            setCommitted(false);
          }}
        >
          <RefreshCw className="h-3.5 w-3.5" />
        </button>
      </Tooltip>
    );
  }

  function currentWindow(selected: ProviderLimitPeriod) {
    if (!usage || query.isError) return undefined;
    const start = usage[`window_${selected}_start_ts`];
    const end = usage[`window_${selected}_end_ts`];
    const format = (ts: number) =>
      new Date(ts * 1000).toLocaleString(undefined, {
        month: "numeric",
        day: "numeric",
        hour: "2-digit",
        minute: "2-digit",
        hour12: false,
      });
    return (
      <div className="mt-0.5 space-y-0.5 text-xs text-muted-foreground">
        <div className="break-words">{format(start)} → {format(end)}</div>
        <div>本期用量 {formatUsdRaw(usage[`usage_${selected}_usd`])} USD</div>
      </div>
    );
  }

  return (
    <details className="group rounded-xl border border-border bg-gradient-to-br from-secondary/80 to-white shadow-sm open:ring-2 open:ring-accent/10 transition-all dark:border-border dark:from-secondary/80 dark:to-secondary">
      <summary className="flex cursor-pointer items-center justify-between px-5 py-4 select-none">
        <div className="flex items-center gap-3">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-gradient-to-br from-amber-400 to-orange-500 shadow-sm">
            <DollarSign className="h-4 w-4 text-white" />
          </div>
          <div>
            <span className="text-sm font-semibold text-secondary-foreground group-open:text-accent dark:text-secondary-foreground">
              限流配置
            </span>
            <p className="text-xs text-muted-foreground">配置不同时间窗口的消费限制以控制成本</p>
          </div>
        </div>
        <ChevronDown className="h-4 w-4 text-muted-foreground transition-transform group-open:rotate-180" />
      </summary>

      <div className="space-y-6 border-t border-border px-5 py-5 dark:border-border">
        {editingProviderId != null && query.isError ? (
          <div role="alert" className="text-sm text-red-600">
            用量读取失败：{formatUnknownError(query.error)}
            <Button
              variant="secondary"
              disabled={saving || query.isFetching}
              onClick={() => void query.refetch()}
            >
              重试读取
            </Button>
          </div>
        ) : null}
        <div>
          <h4 className="mb-3 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
            时间维度限制
          </h4>
          <div className="grid gap-4 sm:grid-cols-2">
            <LimitCard
              icon={<Clock className="h-5 w-5 text-blue-600" />}
              iconBgClass="bg-blue-50 dark:bg-blue-900/30"
              label="5 小时消费上限"
              action={action("5h")}
              currentWindow={currentWindow("5h")}
              hint="留空表示不限制"
              value={limit5hUsd}
              onChange={(value) => setValue("limit_5h_usd", value, { shouldDirty: true })}
              placeholder="例如: 10"
              disabled={saving}
            />
            <LimitCard
              icon={<DollarSign className="h-5 w-5 text-emerald-600" />}
              iconBgClass="bg-emerald-50 dark:bg-emerald-900/30"
              label="每日消费上限"
              action={action("daily")}
              currentWindow={currentWindow("daily")}
              hint="留空表示不限制"
              value={limitDailyUsd}
              onChange={(value) => setValue("limit_daily_usd", value, { shouldDirty: true })}
              placeholder="例如: 100"
              disabled={saving}
            />
            <LimitCard
              icon={<CalendarDays className="h-5 w-5 text-violet-600" />}
              iconBgClass="bg-violet-50 dark:bg-violet-900/30"
              label="周消费上限"
              action={action("weekly")}
              currentWindow={currentWindow("weekly")}
              hint="自然周：周一 00:00:00"
              value={limitWeeklyUsd}
              onChange={(value) => setValue("limit_weekly_usd", value, { shouldDirty: true })}
              placeholder="例如: 500"
              disabled={saving}
            />
            <LimitCard
              icon={<CalendarRange className="h-5 w-5 text-orange-600" />}
              iconBgClass="bg-orange-50 dark:bg-orange-900/30"
              label="月消费上限"
              action={action("monthly")}
              currentWindow={currentWindow("monthly")}
              hint="自然月：每月 1 号 00:00:00"
              value={limitMonthlyUsd}
              onChange={(value) => setValue("limit_monthly_usd", value, { shouldDirty: true })}
              placeholder="例如: 2000"
              disabled={saving}
            />
          </div>
        </div>

        <div>
          <h4 className="mb-3 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
            每日重置设置
          </h4>
          <div className="rounded-xl border border-border bg-white p-4 shadow-sm dark:border-border dark:bg-secondary">
            <div className="flex items-start gap-3">
              <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-sky-50 dark:bg-sky-900/30">
                <RotateCcw className="h-5 w-5 text-sky-600" />
              </div>
              <div className="min-w-0 flex-1 space-y-4">
                <div className="grid gap-4 sm:grid-cols-2">
                  <div>
                    <div className="text-sm font-medium text-secondary-foreground">
                      每日重置模式
                    </div>
                    <p className="mb-2 text-xs text-muted-foreground">rolling 为过去 24 小时窗口</p>
                    <RadioButtonGroup<DailyResetMode>
                      items={[
                        { value: "fixed", label: "固定时间" },
                        { value: "rolling", label: "滚动窗口 (24h)" },
                      ]}
                      ariaLabel="每日重置模式"
                      value={dailyResetMode}
                      onChange={(value) =>
                        setValue("daily_reset_mode", value, { shouldDirty: true })
                      }
                      disabled={saving}
                    />
                  </div>
                  <div>
                    <label
                      htmlFor="provider-daily-reset-time"
                      className="text-sm font-medium text-secondary-foreground"
                    >
                      每日重置时间
                    </label>
                    <p className="mb-2 text-xs text-muted-foreground">
                      {dailyResetMode === "fixed"
                        ? "默认 00:00:00（本机时区）"
                        : "rolling 模式下忽略"}
                    </p>
                    <Input
                      id="provider-daily-reset-time"
                      type="time"
                      step="1"
                      disabled={saving || dailyResetMode !== "fixed"}
                      {...register("daily_reset_time")}
                    />
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div>
          <h4 className="mb-3 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
            其他限制
          </h4>
          <div className="grid gap-4 sm:grid-cols-2">
            <LimitCard
              icon={<Gauge className="h-5 w-5 text-rose-600" />}
              iconBgClass="bg-rose-50 dark:bg-rose-900/30"
              label="总消费上限"
              hint="达到后需手动调整/清除"
              value={limitTotalUsd}
              onChange={(value) => setValue("limit_total_usd", value, { shouldDirty: true })}
              placeholder="例如: 1000"
              disabled={saving}
            />
          </div>
        </div>
      </div>
      <ConfirmDialog
        open={open && period != null}
        title={`重设${period ? labels[period] : ""}周期`}
        description={`供应商「${editProviderName}」：当前周期用量清零，周期从现在重新开始。历史请求和费用保留。`}
        onClose={() => {
          if (!inFlight.current) {
            setPeriod(null);
            setError(null);
          }
        }}
        onConfirm={() => void confirmReset()}
        confirmLabel={committed ? "重试刷新用量" : "确认重设"}
        confirmingLabel={committed ? "刷新中…" : "重设中…"}
        confirming={limitResetPending}
      >
        {error ? <p role="alert" className="text-sm text-red-600">{error}</p> : null}
      </ConfirmDialog>
    </details>
  );
}
