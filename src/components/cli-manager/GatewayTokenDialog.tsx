import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { AlertTriangle, Check, Copy } from "lucide-react";
import { copyText } from "../../services/clipboard";
import { logToConsole } from "../../services/consoleLog";
import {
  gatewayBearerTokenAcknowledge,
  gatewayBearerTokenReveal,
  gatewayBearerTokenRotate,
  type GatewayBearerTokenReveal,
} from "../../services/gateway/gateway";
import { Button } from "../../ui/Button";
import { Dialog } from "../../ui/Dialog";

export function useGatewayTokenController(available: boolean) {
  const [tokenDialog, setTokenDialog] = useState<GatewayBearerTokenReveal | null>(null);
  const [tokenDialogOpen, setTokenDialogOpen] = useState(false);
  const [revealPending, setRevealPending] = useState(false);
  const [actionPending, setActionPending] = useState(false);
  const revealInFlightRef = useRef<Promise<void> | null>(null);
  const actionInFlightRef = useRef(false);

  const revealPendingGatewayToken = useCallback(() => {
    if (!available) return Promise.resolve();
    const existing = revealInFlightRef.current;
    if (existing) return existing;

    setRevealPending(true);
    let request: Promise<void>;
    request = gatewayBearerTokenReveal()
      .then((reveal) => {
        if (!reveal) return;
        setTokenDialog(reveal);
        setTokenDialogOpen(true);
      })
      .catch(() => {
        logToConsole("error", "读取网关访问令牌失败");
        toast("读取网关访问令牌失败：请稍后重试");
      })
      .finally(() => {
        if (revealInFlightRef.current === request) {
          revealInFlightRef.current = null;
        }
        setRevealPending(false);
      });
    revealInFlightRef.current = request;
    return request;
  }, [available]);

  useEffect(() => {
    void revealPendingGatewayToken();
  }, [revealPendingGatewayToken]);

  const rotateGatewayToken = useCallback(async () => {
    const revealInFlight = revealInFlightRef.current;
    if (revealInFlight) await revealInFlight;
    if (actionInFlightRef.current) return;
    actionInFlightRef.current = true;
    setActionPending(true);
    try {
      const reveal = await gatewayBearerTokenRotate();
      setTokenDialog(reveal);
      setTokenDialogOpen(true);
    } catch {
      logToConsole("error", "轮换网关访问令牌失败");
      toast("轮换网关访问令牌失败：请稍后重试");
    } finally {
      actionInFlightRef.current = false;
      setActionPending(false);
    }
  }, []);

  const copyGatewayToken = useCallback(async () => {
    if (!tokenDialog) return;
    try {
      await copyText(tokenDialog.token);
      toast("访问令牌已复制");
    } catch {
      toast("复制失败：当前环境不支持剪贴板");
    }
  }, [tokenDialog]);

  const acknowledgeGatewayToken = useCallback(async () => {
    if (actionInFlightRef.current) return;
    actionInFlightRef.current = true;
    setActionPending(true);
    try {
      await gatewayBearerTokenAcknowledge();
      setTokenDialog(null);
      setTokenDialogOpen(false);
      toast("网关访问令牌已确认");
    } catch {
      logToConsole("error", "确认网关访问令牌失败");
      toast("确认网关访问令牌失败：请稍后重试");
    } finally {
      actionInFlightRef.current = false;
      setActionPending(false);
    }
  }, []);

  const closeGatewayTokenDialog = useCallback((open: boolean) => {
    setTokenDialogOpen(open);
    if (!open) setTokenDialog(null);
  }, []);

  return {
    tokenDialog,
    tokenDialogOpen,
    tokenActionPending: revealPending || actionPending,
    revealPendingGatewayToken,
    rotateGatewayToken,
    copyGatewayToken,
    acknowledgeGatewayToken,
    closeGatewayTokenDialog,
  };
}

export type GatewayTokenController = ReturnType<typeof useGatewayTokenController>;

export function GatewayTokenDialog({ controller }: { controller: GatewayTokenController }) {
  return (
    <Dialog
      open={controller.tokenDialogOpen && controller.tokenDialog != null}
      title="网关访问令牌"
      description="该令牌仅在此处显示一次。关闭而不确认后，需要轮换令牌才能再次获得访问凭据。"
      onOpenChange={controller.closeGatewayTokenDialog}
      className="max-w-xl"
    >
      <div className="space-y-4">
        <code className="block break-all border border-border bg-secondary px-3 py-2 font-mono text-xs text-foreground">
          {controller.tokenDialog?.token}
        </code>
        {controller.tokenDialog?.wsl_sync_error ? (
          <div className="flex items-start gap-2 border border-amber-200 bg-amber-50 p-3 text-xs text-amber-900 dark:border-amber-800 dark:bg-amber-950/40 dark:text-amber-200">
            <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" />
            <span>{controller.tokenDialog.wsl_sync_error}</span>
          </div>
        ) : null}
        <div className="flex flex-wrap justify-end gap-2">
          <Button type="button" variant="secondary" onClick={controller.copyGatewayToken}>
            <Copy className="h-4 w-4" />
            复制
          </Button>
          <Button
            type="button"
            onClick={controller.acknowledgeGatewayToken}
            disabled={controller.tokenActionPending}
          >
            <Check className="h-4 w-4" />
            已保存
          </Button>
        </div>
      </div>
    </Dialog>
  );
}
