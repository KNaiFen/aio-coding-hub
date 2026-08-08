import { Power, RefreshCw, ShieldAlert } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { retryAppStartupStatusSnapshot } from "../../app/startupStatusStore";
import { logToConsole } from "../../services/consoleLog";
import type { AppStartupStatus } from "../../services/app/startupStatus";
import { appExit } from "../../services/app/dataManagement";
import { Button } from "../../ui/Button";

export function AppMaintenanceScreen({ status }: { status: AppStartupStatus }) {
  const [retrying, setRetrying] = useState(false);
  const [retryError, setRetryError] = useState<string | null>(null);
  const [exiting, setExiting] = useState(false);

  async function retry() {
    if (!status.canRetry || retrying) return;
    setRetrying(true);
    setRetryError(null);
    try {
      await retryAppStartupStatusSnapshot();
    } catch (error) {
      const message = "重试数据清理失败：请查看 Console 日志";
      logToConsole("error", "重试数据清理失败", { error: String(error) });
      setRetryError(message);
      toast.error(message);
    } finally {
      setRetrying(false);
    }
  }

  async function exit() {
    if (exiting) return;
    setExiting(true);
    try {
      await appExit();
    } catch {
      setExiting(false);
    }
  }

  return (
    <main className="flex min-h-screen items-center justify-center bg-background px-6 py-10 text-foreground">
      <section className="w-full max-w-xl border-y border-amber-300 py-8 dark:border-amber-700">
        <div className="flex items-center gap-3 text-amber-700 dark:text-amber-300">
          <ShieldAlert aria-hidden="true" className="size-6 shrink-0" />
          <h1 className="text-lg font-semibold">数据重置尚未完成</h1>
        </div>
        <p className="mt-3 break-words text-sm leading-6 text-secondary-foreground">
          {status.errorMessage ?? "维护操作需要重新执行。"}
        </p>
        {retryError ? (
          <p role="alert" className="mt-2 break-words text-sm text-destructive">
            {retryError}
          </p>
        ) : null}
        <div className="mt-6 flex flex-wrap gap-3">
          <Button onClick={() => void retry()} disabled={!status.canRetry || retrying}>
            <RefreshCw aria-hidden="true" className={retrying ? "size-4 animate-spin" : "size-4"} />
            {retrying ? "重试中" : "重试"}
          </Button>
          <Button variant="secondary" onClick={() => void exit()} disabled={exiting}>
            <Power aria-hidden="true" className="size-4" />
            {exiting ? "正在退出" : "退出"}
          </Button>
        </div>
      </section>
    </main>
  );
}
