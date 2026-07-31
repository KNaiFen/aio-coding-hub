import { afterEach, describe, expect, it, vi } from "vitest";
import {
  HOME_OVERVIEW_VISIBILITY_STORAGE_KEY,
  normalizeHomeOverviewVisibility,
  readHomeOverviewVisibilityFromStorage,
  subscribeHomeOverviewVisibility,
  visibleHomeOverviewCliKeys,
  visibleHomeOverviewTabs,
  writeHomeOverviewVisibilityToStorage,
} from "../homeOverviewVisibility";

afterEach(() => {
  vi.restoreAllMocks();
  window.localStorage.clear();
});

describe("services/home/homeOverviewVisibility", () => {
  it("normalizes invalid, duplicated, and unknown visibility keys", () => {
    expect(normalizeHomeOverviewVisibility("bad")).toEqual({
      version: 1,
      hiddenTabs: [],
      hiddenCliKeys: [],
    });

    const visibility = normalizeHomeOverviewVisibility({
      version: 1,
      hiddenTabs: ["sessions", "sessions", "unknown"],
      hiddenCliKeys: ["codex", "codex", "unknown"],
    });

    expect(visibility).toEqual({
      version: 1,
      hiddenTabs: ["sessions"],
      hiddenCliKeys: ["codex"],
    });
    expect(visibleHomeOverviewTabs(visibility)).not.toContain("sessions");
    expect(visibleHomeOverviewCliKeys(visibility)).not.toContain("codex");
  });

  it("keeps each group fail-open when storage hides every available item", () => {
    const visibility = normalizeHomeOverviewVisibility({
      version: 1,
      hiddenTabs: ["workspaceConfig", "circuit", "sessions", "providerLimit", "oauthQuota"],
      hiddenCliKeys: ["claude", "codex", "gemini", "grok"],
    });

    expect(visibility.hiddenTabs).toEqual([]);
    expect(visibility.hiddenCliKeys).toEqual([]);
    expect(visibleHomeOverviewTabs(visibility)).toHaveLength(5);
    expect(visibleHomeOverviewCliKeys(visibility)).toHaveLength(4);
  });

  it("reads malformed storage fail-open and writes normalized preferences", () => {
    window.localStorage.setItem(HOME_OVERVIEW_VISIBILITY_STORAGE_KEY, "{bad json");
    expect(readHomeOverviewVisibilityFromStorage()).toEqual({
      version: 1,
      hiddenTabs: [],
      hiddenCliKeys: [],
    });

    writeHomeOverviewVisibilityToStorage({
      version: 1,
      hiddenTabs: ["sessions"],
      hiddenCliKeys: ["codex"],
    });

    expect(window.localStorage.getItem(HOME_OVERVIEW_VISIBILITY_STORAGE_KEY)).toBe(
      JSON.stringify({
        version: 1,
        hiddenTabs: ["sessions"],
        hiddenCliKeys: ["codex"],
      })
    );
  });

  it("notifies subscribers for local writes and matching storage events", () => {
    const listener = vi.fn();
    const unsubscribe = subscribeHomeOverviewVisibility(listener);

    writeHomeOverviewVisibilityToStorage({
      version: 1,
      hiddenTabs: ["sessions"],
      hiddenCliKeys: [],
    });
    expect(listener).toHaveBeenCalledTimes(1);

    window.dispatchEvent(
      new StorageEvent("storage", {
        key: HOME_OVERVIEW_VISIBILITY_STORAGE_KEY,
        storageArea: window.localStorage,
      })
    );
    expect(listener).toHaveBeenCalledTimes(2);

    unsubscribe();
  });
});
