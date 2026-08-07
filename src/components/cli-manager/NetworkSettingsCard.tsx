import { useCallback, useEffect, useMemo, useReducer, useState } from "react";
import { toast } from "sonner";
import type { AppSettings, GatewayListenMode } from "../../services/settings/settings";
import { logToConsole } from "../../services/consoleLog";
import { copyText } from "../../services/clipboard";
import {
  gatewayBearerTokenAcknowledge,
  gatewayBearerTokenReveal,
  gatewayBearerTokenRotate,
  type GatewayBearerTokenReveal,
} from "../../services/gateway/gateway";
import {
  formatHostPort,
  parseCustomListenAddress,
  validateGatewayCustomListenAddress,
} from "../../services/settings/settingsValidation";
import { useGatewayMeta } from "../../hooks/useGatewayMeta";
import { useWslHostAddressQuery } from "../../query/wsl";
import { Card } from "../../ui/Card";
import { Button } from "../../ui/Button";
import { Dialog } from "../../ui/Dialog";
import { Input } from "../../ui/Input";
import { Select } from "../../ui/Select";
import { SettingsRow } from "../../ui/SettingsRow";
import { cn } from "../../utils/cn";
import { AlertTriangle, Check, Copy, Network, RefreshCw } from "lucide-react";

export type NetworkSettingsCardProps = {
  available: boolean;
  saving: boolean;
  settings: AppSettings;
  onPersistSettings: (
    patch: Partial<AppSettings> & { upstream_proxy_password?: never }
  ) => Promise<AppSettings | null>;
};

type NetworkDraftState = {
  sourceKey: string;
  listenMode: GatewayListenMode;
  customAddress: string;
};

type NetworkDraftAction =
  | { type: "resetFromSettings"; state: NetworkDraftState }
  | { type: "setListenMode"; listenMode: GatewayListenMode }
  | { type: "setCustomAddress"; customAddress: string };

type GatewayTokenDialogState = GatewayBearerTokenReveal;

function createNetworkDraftState(settings: AppSettings): NetworkDraftState {
  return {
    sourceKey: `${settings.gateway_listen_mode}:${settings.gateway_custom_listen_address}`,
    listenMode: settings.gateway_listen_mode,
    customAddress: settings.gateway_custom_listen_address,
  };
}

function networkDraftReducer(
  state: NetworkDraftState,
  action: NetworkDraftAction
): NetworkDraftState {
  if (action.type === "resetFromSettings") {
    return action.state;
  }
  if (action.type === "setListenMode") {
    return { ...state, listenMode: action.listenMode };
  }
  return { ...state, customAddress: action.customAddress };
}

export function NetworkSettingsCard({
  available,
  saving,
  settings,
  onPersistSettings,
}: NetworkSettingsCardProps) {
  const gatewayMeta = useGatewayMeta();
  const gateway = gatewayMeta.gateway;

  const nextDraftState = createNetworkDraftState(settings);
  const [draftState, dispatchDraft] = useReducer(networkDraftReducer, nextDraftState);
  const effectiveDraftState =
    draftState.sourceKey === nextDraftState.sourceKey ? draftState : nextDraftState;
  if (draftState.sourceKey !== nextDraftState.sourceKey) {
    dispatchDraft({ type: "resetFromSettings", state: nextDraftState });
  }
  const { listenMode, customAddress } = effectiveDraftState;
  const wslHostQuery = useWslHostAddressQuery({
    enabled: available && listenMode === "wsl_auto",
  });
  const wslHost = wslHostQuery.data ?? null;
  const [gatewayTokenDialog, setGatewayTokenDialog] = useState<GatewayTokenDialogState | null>(
    null
  );
  const [gatewayTokenDialogOpen, setGatewayTokenDialogOpen] = useState(false);
  const [gatewayTokenActionPending, setGatewayTokenActionPending] = useState(false);

  function setListenMode(listenMode: GatewayListenMode) {
    dispatchDraft({ type: "setListenMode", listenMode });
  }

  function setCustomAddress(customAddress: string) {
    dispatchDraft({ type: "setCustomAddress", customAddress });
  }

  const currentListenAddress = useMemo(() => {
    if (gateway?.running && gateway.listen_addr) return gateway.listen_addr;

    const port = settings.preferred_port;
    if (listenMode === "localhost") return `127.0.0.1:${port}`;
    if (listenMode === "lan") return `0.0.0.0:${port}`;
    if (listenMode === "wsl_auto") return `${wslHost ?? "127.0.0.1"}:${port}`;
    const parsed = parseCustomListenAddress(customAddress);
    if (!parsed) return "（自定义地址格式无效）";
    return formatHostPort(parsed.host, parsed.port ?? port);
  }, [
    gateway?.listen_addr,
    gateway?.running,
    listenMode,
    customAddress,
    settings.preferred_port,
    wslHost,
  ]);

  const requiresGatewayToken = useMemo(() => {
    if (listenMode === "localhost") return false;
    if (listenMode !== "custom") return true;
    const host = parseCustomListenAddress(customAddress)?.host.trim().toLowerCase();
    if (!host) return false;
    return !(
      host === "localhost" ||
      host === "::1" ||
      host === "[::1]" ||
      host === "127.0.0.1" ||
      host.startsWith("127.")
    );
  }, [customAddress, listenMode]);

  const revealPendingGatewayToken = useCallback(async () => {
    if (!available) return;
    try {
      const reveal = await gatewayBearerTokenReveal();
      if (reveal) {
        setGatewayTokenDialog(reveal);
        setGatewayTokenDialogOpen(true);
      }
    } catch {
      logToConsole("error", "读取网关访问令牌失败");
      toast("读取网关访问令牌失败：请稍后重试");
    }
  }, [available]);

  useEffect(() => {
    void revealPendingGatewayToken();
  }, [revealPendingGatewayToken]);

  function closeGatewayTokenDialog(open: boolean) {
    setGatewayTokenDialogOpen(open);
    if (!open) setGatewayTokenDialog(null);
  }

  async function rotateGatewayToken() {
    if (gatewayTokenActionPending) return;
    setGatewayTokenActionPending(true);
    try {
      const reveal = await gatewayBearerTokenRotate();
      setGatewayTokenDialog(reveal);
      setGatewayTokenDialogOpen(true);
    } catch {
      logToConsole("error", "轮换网关访问令牌失败");
      toast("轮换网关访问令牌失败：请稍后重试");
    } finally {
      setGatewayTokenActionPending(false);
    }
  }

  async function copyGatewayToken() {
    if (!gatewayTokenDialog) return;
    try {
      await copyText(gatewayTokenDialog.token);
      toast("访问令牌已复制");
    } catch {
      toast("复制失败：当前环境不支持剪贴板");
    }
  }

  async function acknowledgeGatewayToken() {
    if (gatewayTokenActionPending) return;
    setGatewayTokenActionPending(true);
    try {
      await gatewayBearerTokenAcknowledge();
      setGatewayTokenDialog(null);
      setGatewayTokenDialogOpen(false);
      toast("网关访问令牌已确认");
    } catch {
      logToConsole("error", "确认网关访问令牌失败");
      toast("确认网关访问令牌失败：请稍后重试");
    } finally {
      setGatewayTokenActionPending(false);
    }
  }

  async function commitListenMode(next: GatewayListenMode) {
    if (!available) return;
    setListenMode(next);

    try {
      const updated = await onPersistSettings({ gateway_listen_mode: next });
      if (!updated) {
        return;
      }

      await revealPendingGatewayToken();

      logToConsole("info", "更新监听模式", { next, running: gateway?.running ?? false });
      toast("监听模式已保存");
    } catch (err) {
      logToConsole("error", "更新监听模式失败", { error: String(err), next });
      toast("更新监听模式失败：请稍后重试");
      setListenMode(settings.gateway_listen_mode);
    }
  }

  async function commitCustomAddress() {
    if (!available) return;
    const trimmed = customAddress.trim();
    const err = validateGatewayCustomListenAddress(trimmed);
    if (err) {
      toast(err);
      setCustomAddress(settings.gateway_custom_listen_address);
      return;
    }

    try {
      const updated = await onPersistSettings({ gateway_custom_listen_address: trimmed });
      if (!updated) {
        setCustomAddress(settings.gateway_custom_listen_address);
        return;
      }

      await revealPendingGatewayToken();

      logToConsole("info", "更新自定义监听地址", {
        address: trimmed,
        running: gateway?.running ?? false,
      });
      toast("自定义监听地址已保存");
    } catch (err) {
      logToConsole("error", "更新自定义监听地址失败", {
        error: String(err),
        address: trimmed,
      });
      toast("更新自定义监听地址失败：请稍后重试");
      setCustomAddress(settings.gateway_custom_listen_address);
    }
  }

  return (
    <>
      <Card className="md:col-span-2 relative overflow-hidden">
      <div className="absolute top-0 right-0 p-4 opacity-5">
        <Network className="h-32 w-32" />
      </div>

      <div className="relative z-10">
        <div className="mb-4 border-b border-border pb-4">
          <h2 className="flex items-center gap-2 text-sm font-semibold text-foreground">
            <Network className="h-5 w-5 text-blue-500" />
            网络设置
          </h2>
        </div>

        {!available ? (
          <div className="text-sm font-medium text-secondary-foreground dark:text-foreground bg-secondary p-4 rounded-lg">
            数据不可用
          </div>
        ) : (
          <div className="space-y-1">
            <SettingsRow label="监听模式">
              <Select
                value={listenMode}
                onChange={(e) => void commitListenMode(e.currentTarget.value as GatewayListenMode)}
                disabled={saving}
                className="w-56"
              >
                <option value="localhost">仅本地 (127.0.0.1)</option>
                <option value="wsl_auto">WSL 自动检测</option>
                <option value="lan">局域网 (0.0.0.0)</option>
                <option value="custom">自定义地址</option>
              </Select>
            </SettingsRow>

            {listenMode === "custom" ? (
              <SettingsRow label="自定义地址">
                <Input
                  value={customAddress}
                  placeholder="0.0.0.0 或 0.0.0.0:37123"
                  onChange={(e) => setCustomAddress(e.currentTarget.value)}
                  onBlur={() => void commitCustomAddress()}
                  disabled={saving}
                  className="font-mono"
                />
              </SettingsRow>
            ) : null}

            <SettingsRow label="当前监听地址">
              <div
                className={cn(
                  "font-mono text-xs text-secondary-foreground bg-secondary px-3 py-2 rounded border border-border break-all",
                  !gateway?.running ? "opacity-80" : null
                )}
              >
                {currentListenAddress}
              </div>
            </SettingsRow>

            {requiresGatewayToken ? (
              <SettingsRow label="远程访问令牌">
                <div className="flex items-center gap-2">
                  <span className="text-xs text-secondary-foreground">
                    非本地访问必须携带 Bearer Token
                  </span>
                  <Button
                    type="button"
                    variant="secondary"
                    size="sm"
                    className="h-8 w-8 p-0"
                    disabled={saving || gatewayTokenActionPending}
                    onClick={() => void rotateGatewayToken()}
                    aria-label="轮换访问令牌"
                    title="轮换访问令牌"
                  >
                    <RefreshCw className="h-4 w-4" />
                  </Button>
                </div>
              </SettingsRow>
            ) : null}

            {listenMode === "lan" ? (
              <div className="mt-3 rounded-lg bg-amber-50 dark:bg-amber-900/30 p-3 text-sm text-amber-800 dark:text-amber-400 border border-amber-100 dark:border-amber-800 flex items-start gap-2">
                <AlertTriangle className="h-4 w-4 mt-0.5 shrink-0" />
                <div>
                  <div className="font-medium">安全提示</div>
                  <div className="text-xs mt-0.5 text-amber-700 dark:text-amber-400">
                    局域网模式会将网关暴露在本机网络接口上；所有远程请求均须提供应用生成的 Bearer Token。
                  </div>
                </div>
              </div>
            ) : null}
          </div>
        )}
      </div>
      </Card>

      <Dialog
        open={gatewayTokenDialogOpen && gatewayTokenDialog != null}
        title="网关访问令牌"
        description="该令牌仅在此处显示一次。关闭而不确认后，需要轮换令牌才能再次获得访问凭据。"
        onOpenChange={closeGatewayTokenDialog}
        className="max-w-xl"
      >
        <div className="space-y-4">
          <code className="block break-all border border-border bg-secondary px-3 py-2 font-mono text-xs text-foreground">
            {gatewayTokenDialog?.token}
          </code>
          {gatewayTokenDialog?.wsl_sync_error ? (
            <div className="flex items-start gap-2 border border-amber-200 bg-amber-50 p-3 text-xs text-amber-900 dark:border-amber-800 dark:bg-amber-950/40 dark:text-amber-200">
              <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" />
              <span>{gatewayTokenDialog.wsl_sync_error}</span>
            </div>
          ) : null}
          <div className="flex flex-wrap justify-end gap-2">
            <Button type="button" variant="secondary" onClick={() => void copyGatewayToken()}>
              <Copy className="h-4 w-4" />
              复制
            </Button>
            <Button
              type="button"
              onClick={() => void acknowledgeGatewayToken()}
              disabled={gatewayTokenActionPending}
            >
              <Check className="h-4 w-4" />
              已保存
            </Button>
          </div>
        </div>
      </Dialog>
    </>
  );
}
