import { CLI_KEYS, isCliKey, type CliKey } from "../../constants/clis";
import { emitListenerSnapshot } from "../../utils/listeners";
import { HOME_OVERVIEW_TABS, type HomeOverviewTabKey } from "./homeOverviewTabOrder";

export const HOME_OVERVIEW_VISIBILITY_STORAGE_KEY = "aio-home-overview-visibility";

export type HomeOverviewVisibility = {
  version: 1;
  hiddenTabs: HomeOverviewTabKey[];
  hiddenCliKeys: CliKey[];
};

type Listener = () => void;

const DEFAULT_HOME_OVERVIEW_VISIBILITY: HomeOverviewVisibility = Object.freeze({
  version: 1,
  hiddenTabs: [],
  hiddenCliKeys: [],
});
const TAB_KEYS = HOME_OVERVIEW_TABS.map((item) => item.key);
const TAB_KEY_SET = new Set<HomeOverviewTabKey>(TAB_KEYS);
const listeners = new Set<Listener>();

let cachedRaw: string | null | undefined;
let cachedSnapshot = DEFAULT_HOME_OVERVIEW_VISIBILITY;

function emit() {
  emitListenerSnapshot(listeners, (listener) => listener());
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value != null && !Array.isArray(value);
}

function normalizeHiddenTabs(value: unknown): HomeOverviewTabKey[] {
  if (!Array.isArray(value)) return [];

  const hidden = new Set<HomeOverviewTabKey>();
  for (const item of value) {
    if (typeof item === "string" && TAB_KEY_SET.has(item as HomeOverviewTabKey)) {
      hidden.add(item as HomeOverviewTabKey);
    }
  }

  return hidden.size >= TAB_KEYS.length ? [] : TAB_KEYS.filter((key) => hidden.has(key));
}

function normalizeHiddenCliKeys(value: unknown): CliKey[] {
  if (!Array.isArray(value)) return [];

  const hidden = new Set<CliKey>();
  for (const item of value) {
    if (isCliKey(item)) hidden.add(item);
  }

  return hidden.size >= CLI_KEYS.length ? [] : CLI_KEYS.filter((key) => hidden.has(key));
}

export function normalizeHomeOverviewVisibility(input: unknown): HomeOverviewVisibility {
  if (
    !isRecord(input) ||
    input.version !== 1 ||
    !Array.isArray(input.hiddenTabs) ||
    !Array.isArray(input.hiddenCliKeys)
  ) {
    return DEFAULT_HOME_OVERVIEW_VISIBILITY;
  }

  return {
    version: 1,
    hiddenTabs: normalizeHiddenTabs(input.hiddenTabs),
    hiddenCliKeys: normalizeHiddenCliKeys(input.hiddenCliKeys),
  };
}

function isLocalStorageEvent(event: StorageEvent) {
  if (typeof window === "undefined" || event.storageArea == null) return true;

  try {
    return event.storageArea === window.localStorage;
  } catch {
    return false;
  }
}

function handleStorageEvent(event: StorageEvent) {
  if (!isLocalStorageEvent(event)) return;
  if (event.key !== HOME_OVERVIEW_VISIBILITY_STORAGE_KEY && event.key !== null) return;

  cachedRaw = undefined;
  emit();
}

export function readHomeOverviewVisibilityFromStorage(): HomeOverviewVisibility {
  if (typeof window === "undefined") return DEFAULT_HOME_OVERVIEW_VISIBILITY;

  let raw: string | null;
  try {
    raw = window.localStorage.getItem(HOME_OVERVIEW_VISIBILITY_STORAGE_KEY);
  } catch {
    raw = null;
  }

  if (raw === cachedRaw) return cachedSnapshot;

  let next = DEFAULT_HOME_OVERVIEW_VISIBILITY;
  if (raw) {
    try {
      next = normalizeHomeOverviewVisibility(JSON.parse(raw));
    } catch {}
  }

  cachedRaw = raw;
  cachedSnapshot = next;
  return cachedSnapshot;
}

export function writeHomeOverviewVisibilityToStorage(value: HomeOverviewVisibility) {
  if (typeof window === "undefined") return;

  const next = normalizeHomeOverviewVisibility(value);
  try {
    const raw = JSON.stringify(next);
    window.localStorage.setItem(HOME_OVERVIEW_VISIBILITY_STORAGE_KEY, raw);
    cachedRaw = raw;
    cachedSnapshot = next;
  } catch {
    cachedRaw = undefined;
    cachedSnapshot = DEFAULT_HOME_OVERVIEW_VISIBILITY;
  }

  emit();
}

export function subscribeHomeOverviewVisibility(listener: Listener) {
  if (listeners.size === 0 && typeof window !== "undefined") {
    window.addEventListener("storage", handleStorageEvent);
  }
  listeners.add(listener);

  return () => {
    listeners.delete(listener);
    if (listeners.size === 0 && typeof window !== "undefined") {
      window.removeEventListener("storage", handleStorageEvent);
    }
  };
}

export function visibleHomeOverviewTabs(value: HomeOverviewVisibility): HomeOverviewTabKey[] {
  const hidden = new Set(value.hiddenTabs);
  return TAB_KEYS.filter((key) => !hidden.has(key));
}

export function visibleHomeOverviewCliKeys(value: HomeOverviewVisibility): CliKey[] {
  const hidden = new Set(value.hiddenCliKeys);
  return CLI_KEYS.filter((key) => !hidden.has(key));
}
