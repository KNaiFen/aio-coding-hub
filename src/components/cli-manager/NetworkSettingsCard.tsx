import { useEffect, useMemo, useReducer, useRef, useState } from "react";
import { toast } from "sonner";
import type { AppSettings, GatewayListenMode } from "../../services/settings/settings";
import { logToConsole } from "../../services/consoleLog";
import {
  formatHostPort,
  parseCustomListenAddress,
  validateGatewayCustomListenAddress,
} from "../../services/settings/settingsValidation";
import { useGatewayMeta } from "../../hooks/useGatewayMeta";
import { useWslHostAddressQuery } from "../../query/wsl";
import { Card } from "../../ui/Card";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";
import { Select } from "../../ui/Select";
import { SettingsRow } from "../../ui/SettingsRow";
import { cn } from "../../utils/cn";
import { AlertTriangle, LoaderCircle, Network, RefreshCw } from "lucide-react";

export type NetworkSettingsCardProps = {
  available: boolean;
  saving: boolean;
  settings: AppSettings;
  onPersistSettings: (
    patch: Partial<AppSettings> & { upstream_proxy_password?: never }
  ) => Promise<AppSettings | null>;
  onGatewayListenSaved: () => Promise<void>;
  onRotateGatewayToken: () => Promise<void>;
  gatewayTokenActionPending: boolean;
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
  onGatewayListenSaved,
  onRotateGatewayToken,
  gatewayTokenActionPending,
}: NetworkSettingsCardProps) {
  const gatewayMeta = useGatewayMeta();
  const gateway = gatewayMeta.gateway;

  const nextDraftState = useMemo(
    () => createNetworkDraftState(settings),
    [settings]
  );
  const [draftState, dispatchDraft] = useReducer(networkDraftReducer, nextDraftState);
  const [applyingNetworkSettings, setApplyingNetworkSettings] = useState(false);
  const settingsRef = useRef(settings);
  const lastSettingsSourceKeyRef = useRef(nextDraftState.sourceKey);
  settingsRef.current = settings;

  useEffect(() => {
    const sourceChanged = lastSettingsSourceKeyRef.current !== nextDraftState.sourceKey;
    lastSettingsSourceKeyRef.current = nextDraftState.sourceKey;
    if (sourceChanged && !applyingNetworkSettings) {
      dispatchDraft({ type: "resetFromSettings", state: nextDraftState });
    }
  }, [applyingNetworkSettings, nextDraftState]);

  const { listenMode, customAddress } = draftState;
  const networkSettingsPending = saving || applyingNetworkSettings;
  const wslHostQuery = useWslHostAddressQuery({
    enabled: available && listenMode === "wsl_auto",
  });
  const wslHost = wslHostQuery.data ?? null;
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

  async function commitListenMode(next: GatewayListenMode) {
    if (!available || applyingNetworkSettings) return;
    setListenMode(next);
    setApplyingNetworkSettings(true);

    try {
      const updated = await onPersistSettings({ gateway_listen_mode: next });
      if (!updated) {
        dispatchDraft({
          type: "resetFromSettings",
          state: createNetworkDraftState(settingsRef.current),
        });
        return;
      }

      dispatchDraft({ type: "resetFromSettings", state: createNetworkDraftState(updated) });
      if (updated.gateway_listen_mode !== "localhost") {
        await onGatewayListenSaved();
      }

      logToConsole("info", "更新监听模式", { next, running: gateway?.running ?? false });
      toast("监听模式已保存");
    } catch (err) {
      logToConsole("error", "更新监听模式失败", { error: String(err), next });
      toast("更新监听模式失败：请稍后重试");
      dispatchDraft({
        type: "resetFromSettings",
        state: createNetworkDraftState(settingsRef.current),
      });
    } finally {
      setApplyingNetworkSettings(false);
    }
  }

  async function commitCustomAddress() {
    if (!available || applyingNetworkSettings) return;
    const trimmed = customAddress.trim();
    const err = validateGatewayCustomListenAddress(trimmed);
    if (err) {
      toast(err);
      setCustomAddress(settingsRef.current.gateway_custom_listen_address);
      return;
    }
    setApplyingNetworkSettings(true);

    try {
      const updated = await onPersistSettings({ gateway_custom_listen_address: trimmed });
      if (!updated) {
        dispatchDraft({
          type: "resetFromSettings",
          state: createNetworkDraftState(settingsRef.current),
        });
        return;
      }

      dispatchDraft({ type: "resetFromSettings", state: createNetworkDraftState(updated) });
      if (updated.gateway_listen_mode !== "localhost") {
        await onGatewayListenSaved();
      }

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
      dispatchDraft({
        type: "resetFromSettings",
        state: createNetworkDraftState(settingsRef.current),
      });
    } finally {
      setApplyingNetworkSettings(false);
    }
  }

  return (
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
              <div className="flex min-h-10 flex-wrap items-center justify-end gap-2">
                <Select
                  value={listenMode}
                  onChange={(e) =>
                    void commitListenMode(e.currentTarget.value as GatewayListenMode)
                  }
                  disabled={networkSettingsPending}
                  className="w-56"
                >
                  <option value="localhost">仅本地 (127.0.0.1)</option>
                  <option value="wsl_auto">WSL 自动检测</option>
                  <option value="lan">局域网 (0.0.0.0)</option>
                  <option value="custom">自定义地址</option>
                </Select>
                {applyingNetworkSettings ? (
                  <span
                    role="status"
                    className="inline-flex items-center gap-1 text-xs text-muted-foreground"
                  >
                    <LoaderCircle className="h-3.5 w-3.5 animate-spin" />
                    正在应用
                  </span>
                ) : null}
              </div>
            </SettingsRow>

            {listenMode === "custom" ? (
              <SettingsRow label="自定义地址">
                <Input
                  value={customAddress}
                  placeholder="0.0.0.0 或 0.0.0.0:37123"
                  onChange={(e) => setCustomAddress(e.currentTarget.value)}
                  onBlur={() => void commitCustomAddress()}
                  disabled={networkSettingsPending}
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
                    disabled={networkSettingsPending || gatewayTokenActionPending}
                    onClick={() => void onRotateGatewayToken()}
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
  );
}
