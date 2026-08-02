// Usage: Secret-free IPC and event contract for the macOS tray provider mini window.

import { commands } from "../generated/bindings";
import { listenDesktopEvent } from "./desktop/event";
import { invokeGeneratedIpc } from "./generatedIpc";

export const TRAY_PROVIDER_MINI_SNAPSHOT_EVENT = "tray-provider-mini:snapshot";
export const TRAY_PROVIDER_MINI_BUCKET_COUNT = 12;

export type TrayProviderMiniAvailabilityState = "healthy" | "unhealthy" | "no_data";
export type TrayProviderMiniUnavailableReason =
  | "circuit_open"
  | "cooldown"
  | "spend_limit"
  | "oauth_limit";
export type TrayProviderMiniSelectionSource = "active_request" | "recent_request" | "enabled_cli";

export type TrayProviderMiniProvider = {
  providerId: number;
  providerName: string;
  unavailableReasons: TrayProviderMiniUnavailableReason[];
  availability: TrayProviderMiniAvailabilityState[];
};

export type TrayProviderMiniSnapshot = {
  generation: number;
  generatedAtMs: number;
  hours: 3 | 6 | 12;
  cliKey: string | null;
  selectionSource: TrayProviderMiniSelectionSource | null;
  routeName: string | null;
  providers: TrayProviderMiniProvider[];
  unavailable: boolean;
};

const AVAILABILITY_STATES = new Set<TrayProviderMiniAvailabilityState>([
  "healthy",
  "unhealthy",
  "no_data",
]);
const UNAVAILABLE_REASONS = new Set<TrayProviderMiniUnavailableReason>([
  "circuit_open",
  "cooldown",
  "spend_limit",
  "oauth_limit",
]);
const SELECTION_SOURCES = new Set<TrayProviderMiniSelectionSource>([
  "active_request",
  "recent_request",
  "enabled_cli",
]);
const CLI_KEYS = new Set(["claude", "codex", "gemini", "grok"]);

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function finiteInteger(value: unknown, minimum = 0): number | null {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= minimum
    ? value
    : null;
}

function boundedString(value: unknown, maxLength: number): string | null {
  if (typeof value !== "string") return null;
  const normalized = value.trim();
  if (normalized.length === 0 || normalized.length > maxLength) return null;
  return normalized;
}

function normalizeProvider(value: unknown): TrayProviderMiniProvider | null {
  if (!isRecord(value)) return null;
  const providerId = finiteInteger(value.providerId, 1);
  const providerName = boundedString(value.providerName, 128);
  if (providerId == null || providerName == null) return null;

  const unavailableReasons = Array.isArray(value.unavailableReasons)
    ? Array.from(
        new Set(
          value.unavailableReasons.filter(
            (reason): reason is TrayProviderMiniUnavailableReason =>
              typeof reason === "string" &&
              UNAVAILABLE_REASONS.has(reason as TrayProviderMiniUnavailableReason)
          )
        )
      )
    : [];
  const availability = Array.isArray(value.availability)
    ? value.availability.filter(
        (state): state is TrayProviderMiniAvailabilityState =>
          typeof state === "string" &&
          AVAILABILITY_STATES.has(state as TrayProviderMiniAvailabilityState)
      )
    : [];

  return {
    providerId,
    providerName,
    unavailableReasons,
    availability:
      availability.length === TRAY_PROVIDER_MINI_BUCKET_COUNT
        ? availability
        : Array.from({ length: TRAY_PROVIDER_MINI_BUCKET_COUNT }, () => "no_data" as const),
  };
}

export function normalizeTrayProviderMiniSnapshot(value: unknown): TrayProviderMiniSnapshot | null {
  if (!isRecord(value)) return null;
  const generation = finiteInteger(value.generation, 1);
  const generatedAtMs = finiteInteger(value.generatedAtMs);
  const hours = value.hours === 3 || value.hours === 6 || value.hours === 12 ? value.hours : null;
  if (generation == null || generatedAtMs == null || hours == null) return null;

  const cliKey = value.cliKey == null ? null : boundedString(value.cliKey, 16);
  if (cliKey != null && !CLI_KEYS.has(cliKey)) return null;
  const routeName = value.routeName == null ? null : boundedString(value.routeName, 32);
  const selectionSource =
    value.selectionSource == null
      ? null
      : typeof value.selectionSource === "string" &&
          SELECTION_SOURCES.has(value.selectionSource as TrayProviderMiniSelectionSource)
        ? (value.selectionSource as TrayProviderMiniSelectionSource)
        : null;
  if (value.selectionSource != null && selectionSource == null) return null;
  if (!Array.isArray(value.providers) || value.providers.length > 512) return null;
  const providers = value.providers.map(normalizeProvider);
  if (providers.some((provider) => provider == null)) return null;

  return {
    generation,
    generatedAtMs,
    hours,
    cliKey,
    selectionSource,
    routeName,
    providers: providers as TrayProviderMiniProvider[],
    unavailable: value.unavailable === true,
  };
}

export async function getTrayProviderMiniSnapshot(): Promise<TrayProviderMiniSnapshot | null> {
  const value = await invokeGeneratedIpc<unknown, null>({
    title: "读取托盘供应商状态失败",
    cmd: "tray_provider_mini_snapshot_get",
    invoke: () => commands.trayProviderMiniSnapshotGet(),
    nullResultBehavior: "return_fallback",
    fallback: null,
  });
  return normalizeTrayProviderMiniSnapshot(value);
}

export async function setTrayProviderMiniWindowHovered(hovered: boolean): Promise<void> {
  await invokeGeneratedIpc<boolean>({
    title: "同步托盘面板状态失败",
    cmd: "tray_provider_mini_window_hover_set",
    args: { hovered },
    invoke: () => commands.trayProviderMiniWindowHoverSet(hovered),
  });
}

export function listenTrayProviderMiniSnapshot(
  handler: (snapshot: TrayProviderMiniSnapshot | null) => void
): Promise<() => void> {
  return listenDesktopEvent<unknown>(TRAY_PROVIDER_MINI_SNAPSHOT_EVENT, (payload) => {
    handler(normalizeTrayProviderMiniSnapshot(payload));
  });
}
