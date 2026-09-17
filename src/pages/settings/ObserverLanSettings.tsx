import { useEffect, useState } from "react";
import { Copy, Eye, EyeOff, RefreshCw } from "lucide-react";
import { observerLanConfigure, observerLanStatus, observerLanToken, type ObserverLanStatus } from "../../services/observer";
import { writeDesktopClipboardText } from "../../services/desktop/clipboard";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";
import { Switch } from "../../ui/Switch";
import { SettingsRow } from "../../ui/SettingsRow";
import { formatUnknownError } from "../../utils/errors";

export function ObserverLanSettings() {
  const [status, setStatus] = useState<ObserverLanStatus | null>(null);
  const [port, setPort] = useState("13799");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [token, setToken] = useState<string | null>(null);
  useEffect(() => {
    let active = true;
    const refresh = () => observerLanStatus().then((next) => {
      if (active && next) { setStatus(next); setPort(String(next.port)); }
    }).catch((err) => { if (active) setError(formatUnknownError(err)); });
    void refresh();
    return () => { active = false; };
  }, []);
  async function action(run: () => Promise<void>) {
    setBusy(true); setError(null);
    try { await run(); } catch (err) { setError(formatUnknownError(err)); } finally { setBusy(false); }
  }
  async function save(enabled: boolean) {
    const value = Number(port);
    if (!Number.isInteger(value) || value < 1024 || value > 65535) throw new Error("端口号必须为 1024-65535");
    const next = await observerLanConfigure(enabled, value);
    if (next) { setStatus(next); setPort(String(next.port)); }
  }
  return <section className="border-t border-line-subtle pt-5" aria-label="局域网观察">
    <div className="mb-3 flex items-center justify-between">
      <h2 className="text-sm font-semibold">局域网观察</h2>
      <span className="text-xs text-muted-foreground">{status?.running ? "运行中" : "未运行"}</span>
    </div>
    <SettingsRow label="允许局域网观察"><Switch checked={status?.enabled ?? false} disabled={busy || !status} onCheckedChange={(enabled) => { void action(() => save(enabled)); }} /></SettingsRow>
    <SettingsRow label="观察端口"><div className="flex items-center gap-2">
      <Input aria-label="观察端口" type="number" min={1024} max={65535} value={port} disabled={busy} onChange={(event) => setPort(event.target.value)} className="w-28" />
      <Button disabled={busy || !status} onClick={() => { void action(() => save(status?.enabled ?? false)); }}>保存</Button>
    </div></SettingsRow>
    <SettingsRow label="连接地址"><div className="break-all text-sm">{status?.addresses.map((ip) => `${ip}:${status.port}`).join(" / ") || "暂无局域网地址"}</div></SettingsRow>
    <SettingsRow label="访问令牌"><div className="flex min-w-0 items-center gap-2">
      <Input aria-label="访问令牌" readOnly value={token ?? ""} type={token ? "text" : "password"} className="min-w-0 font-mono" />
      <Button title={token ? "隐藏令牌" : "查看令牌"} aria-label={token ? "隐藏令牌" : "查看令牌"} disabled={busy || !status} onClick={() => { void action(async () => setToken(token ? null : await observerLanToken(false))); }}>{token ? <EyeOff size={16} /> : <Eye size={16} />}</Button>
      <Button title="复制令牌" aria-label="复制令牌" disabled={busy || !status} onClick={() => { void action(async () => { const value = token ?? await observerLanToken(false); if (value) await writeDesktopClipboardText(value); }); }}><Copy size={16} /></Button>
      <Button title="重置令牌" aria-label="重置令牌" disabled={busy || !status} onClick={() => { void action(async () => setToken(await observerLanToken(true))); }}><RefreshCw size={16} /></Button>
    </div></SettingsRow>
    {error || status?.error ? <p role="alert" className="mt-2 text-sm text-red-600">{error ?? status?.error}</p> : null}
  </section>;
}
