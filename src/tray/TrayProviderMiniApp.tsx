// Usage: Compact, non-interactive provider route status for the macOS tray hover window.

import { Route } from "lucide-react";
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { CLI_REGISTRY } from "../constants/clis";
import { useTheme } from "../hooks/useTheme";
import {
  getTrayProviderMiniSnapshot,
  listenTrayProviderMiniSnapshot,
  setTrayProviderMiniWindowHovered,
  type TrayProviderMiniAvailabilityState,
  type TrayProviderMiniProvider,
  type TrayProviderMiniSnapshot,
  type TrayProviderMiniUnavailableReason,
} from "../services/trayProviderMini";

const availabilityStyle: Record<TrayProviderMiniAvailabilityState, string> = {
  healthy: "bg-emerald-500 dark:bg-emerald-400",
  degraded: "bg-amber-500 dark:bg-amber-400",
  unhealthy: "bg-rose-500 dark:bg-rose-400",
  no_data: "bg-muted-foreground/20 dark:bg-muted-foreground/30",
};

const availabilityLabel: Record<TrayProviderMiniAvailabilityState, string> = {
  healthy: "正常",
  degraded: "降级",
  unhealthy: "异常",
  no_data: "无数据",
};

const EXACT_COUNT_MAX = 99_999;
const TEN_THOUSAND = 10_000;
const HUNDRED_MILLION = 100_000_000;

const reasonPresentation: Record<
  TrayProviderMiniUnavailableReason,
  { marker: string; title: string; className: string }
> = {
  circuit_open: {
    marker: "熔",
    title: "熔断中",
    className: "border-rose-500/30 bg-rose-500/10 text-rose-600 dark:text-rose-300",
  },
  cooldown: {
    marker: "冷",
    title: "冷却中",
    className: "border-amber-500/30 bg-amber-500/10 text-amber-700 dark:text-amber-300",
  },
  spend_limit: {
    marker: "限",
    title: "消费限额已达",
    className: "border-amber-500/30 bg-amber-500/10 text-amber-700 dark:text-amber-300",
  },
  oauth_limit: {
    marker: "限",
    title: "OAuth 配额已达",
    className: "border-amber-500/30 bg-amber-500/10 text-amber-700 dark:text-amber-300",
  },
};

function cliName(cliKey: string | null): string {
  return CLI_REGISTRY.find((cli) => cli.key === cliKey)?.name ?? cliKey ?? "CLI";
}

export function formatTrayProviderMiniCount(count: number): string {
  if (count <= EXACT_COUNT_MAX) return String(count);

  const divisor = count >= HUNDRED_MILLION ? HUNDRED_MILLION : TEN_THOUSAND;
  const unit = divisor === HUNDRED_MILLION ? "亿" : "万";
  const scaled = count / divisor;
  const truncated =
    scaled < 100 ? Math.floor((count * 10) / divisor) / 10 : Math.floor(count / divisor);
  return `${truncated}${unit}`;
}

function ProviderReasonMarkers({ reasons }: { reasons: TrayProviderMiniUnavailableReason[] }) {
  const seen = new Set<string>();
  const markers = reasons.filter((reason) => {
    const marker = reasonPresentation[reason].marker;
    if (seen.has(marker)) return false;
    seen.add(marker);
    return true;
  });

  return (
    <div
      className="flex h-[18px] min-w-[40px] shrink-0 items-center justify-end gap-1"
      aria-label="供应商状态"
    >
      {markers.map((reason) => {
        const presentation = reasonPresentation[reason];
        return (
          <span
            key={presentation.marker}
            className={`inline-flex h-[18px] w-[18px] items-center justify-center rounded border text-[9px] font-semibold ${presentation.className}`}
            title={presentation.title}
          >
            {presentation.marker}
          </span>
        );
      })}
    </div>
  );
}

function AvailabilityCells({ provider }: { provider: TrayProviderMiniProvider }) {
  return (
    <div
      className="grid h-3 grid-cols-[repeat(18,minmax(0,1fr))] items-center gap-0.5"
      aria-label="供应商可用性"
    >
      {provider.availability.map((state, index) => (
        <span
          key={`${provider.providerId}-${index}`}
          className={`h-1.5 min-w-0 rounded-[2px] ${availabilityStyle[state]}`}
          title={availabilityLabel[state]}
        />
      ))}
    </div>
  );
}

function ProviderTotals({ provider }: { provider: TrayProviderMiniProvider }) {
  const successText = formatTrayProviderMiniCount(provider.successCount);
  const failureText = formatTrayProviderMiniCount(provider.failureCount);

  return (
    <div
      className="grid grid-cols-[32px_32px] items-center gap-2 font-medium tabular-nums"
      role="group"
      aria-label={`总计 成功 ${provider.successCount}，失败 ${provider.failureCount}`}
    >
      <span
        className="whitespace-nowrap text-right font-mono text-[9px] text-emerald-600 dark:text-emerald-300"
        title={String(provider.successCount)}
        aria-hidden="true"
      >
        {successText}
      </span>
      <span
        className="whitespace-nowrap text-right font-mono text-[9px] text-rose-600 dark:text-rose-300"
        title={String(provider.failureCount)}
        aria-hidden="true"
      >
        {failureText}
      </span>
    </div>
  );
}

function ProviderRows({ providers }: { providers: TrayProviderMiniProvider[] }) {
  return (
    <div className="divide-y divide-border/60">
      {providers.map((provider) => (
        <div
          key={provider.providerId}
          className="grid h-6 grid-cols-[92px_198px_72px] items-center gap-2 px-3"
        >
          <div className="flex min-w-0 items-center gap-1.5">
            <span
              className="min-w-0 flex-1 truncate text-xs font-medium text-foreground"
              title={provider.providerName}
            >
              {provider.providerName}
            </span>
            <ProviderReasonMarkers reasons={provider.unavailableReasons} />
          </div>
          <AvailabilityCells provider={provider} />
          <ProviderTotals provider={provider} />
        </div>
      ))}
    </div>
  );
}

function EmptyState({
  snapshot,
  failed,
}: {
  snapshot: TrayProviderMiniSnapshot | null;
  failed: boolean;
}) {
  let text = "正在读取供应商状态";
  if (failed || snapshot?.unavailable) text = "供应商状态暂不可用";
  else if (snapshot && !snapshot.cliKey) text = "暂无已接管 CLI";
  else if (snapshot) text = "当前路由没有已开启供应商";

  return (
    <div className="flex h-[68px] items-center justify-center px-4 text-xs text-muted-foreground">
      {text}
    </div>
  );
}

export function TrayProviderMiniApp() {
  useTheme();
  const [snapshot, setSnapshot] = useState<TrayProviderMiniSnapshot | null>(null);
  const [failed, setFailed] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    void (async () => {
      try {
        const cleanup = await listenTrayProviderMiniSnapshot((nextSnapshot) => {
          if (disposed) return;
          setSnapshot(nextSnapshot);
          setFailed(false);
        });
        if (disposed) {
          cleanup();
          return;
        }
        unlisten = cleanup;
        const initialSnapshot = await getTrayProviderMiniSnapshot();
        if (!disposed) setSnapshot(initialSnapshot);
      } catch {
        if (!disposed) setFailed(true);
      }
    })();

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  useLayoutEffect(() => {
    if (scrollRef.current) scrollRef.current.scrollTop = 0;
  }, [snapshot?.generation]);

  const reportHover = (hovered: boolean) => {
    void setTrayProviderMiniWindowHovered(hovered).catch(() => undefined);
  };
  const providers = snapshot?.providers ?? [];

  return (
    <main
      className="tray-provider-mini-surface isolate h-screen w-screen overflow-hidden rounded-[14px] text-foreground"
      onPointerEnter={() => reportHover(true)}
      onPointerLeave={() => reportHover(false)}
    >
      <header className="flex h-[42px] items-center justify-between gap-3 border-b border-border/60 px-3.5">
        <div className="flex min-w-0 items-center gap-2">
          <span className="shrink-0 text-[13px] font-semibold">
            {cliName(snapshot?.cliKey ?? null)}
          </span>
          <span className="text-muted-foreground/50">/</span>
          <Route className="h-3 w-3 shrink-0 text-muted-foreground" aria-hidden="true" />
          <span
            className="truncate text-xs text-muted-foreground"
            title={snapshot?.routeName ?? ""}
          >
            {snapshot?.routeName ?? "默认"}
          </span>
        </div>
        <span className="shrink-0 text-[10px] tabular-nums text-muted-foreground">
          {snapshot?.hours ?? 6}h
        </span>
      </header>
      <div ref={scrollRef} className="max-h-[240px] overflow-y-auto overscroll-contain">
        {providers.length > 0 ? (
          <ProviderRows providers={providers} />
        ) : (
          <EmptyState snapshot={snapshot} failed={failed} />
        )}
      </div>
    </main>
  );
}
