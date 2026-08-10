import { appEventNames } from "../../constants/appEvents";
import type { CodexCatalogEventPayload } from "../../generated/bindings";
import { logToConsole } from "../consoleLog";
import { listenDesktopEvent } from "../desktop/event";

const CODEX_CATALOG_EVENT_STATUSES = ["updated", "failed"] as const;

export function parseCodexCatalogEventPayload(value: unknown): CodexCatalogEventPayload | null {
  if (value == null || typeof value !== "object" || Array.isArray(value)) return null;
  const status = (value as Record<string, unknown>).status;
  if (
    typeof status !== "string" ||
    !(CODEX_CATALOG_EVENT_STATUSES as readonly string[]).includes(status)
  ) {
    return null;
  }
  return { status: status as CodexCatalogEventPayload["status"] };
}

export async function listenProviderCodexCatalogEvents(
  onEvent: (payload: CodexCatalogEventPayload) => void
): Promise<() => void> {
  return listenDesktopEvent<unknown>(appEventNames.providerCodexCatalog, (rawPayload) => {
    const payload = parseCodexCatalogEventPayload(rawPayload);
    if (!payload) {
      logToConsole(
        "warn",
        "忽略无效的 Codex 模型目录事件",
        { payload_type: typeof rawPayload },
        appEventNames.providerCodexCatalog
      );
      return;
    }
    onEvent(payload);
  });
}
