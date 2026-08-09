import { beforeEach, describe, expect, it, vi } from "vitest";
import { commands } from "../../generated/bindings";
import { listenDesktopEvent } from "../desktop/event";
import {
  getTrayProviderMiniSnapshot,
  listenTrayProviderMiniSnapshot,
  normalizeTrayProviderMiniSnapshot,
  setTrayProviderMiniWindowHovered,
  TRAY_PROVIDER_MINI_SNAPSHOT_EVENT,
  type TrayProviderMiniSnapshot,
} from "../trayProviderMini";

vi.mock("../desktop/event", () => ({
  listenDesktopEvent: vi.fn(),
}));

vi.mock("../../generated/bindings", () => ({
  commands: {
    trayProviderMiniSnapshotGet: vi.fn(),
    trayProviderMiniWindowHoverSet: vi.fn(),
  },
}));

const snapshot: TrayProviderMiniSnapshot = {
  generation: 7,
  generatedAtMs: 1_780_000_000_000,
  hours: 6,
  cliKey: "codex",
  selectionSource: "active_request",
  routeName: "主路由",
  providers: [
    {
      providerId: 3,
      providerName: "大春",
      unavailableReasons: ["cooldown"],
      successCount: 24,
      failureCount: 2,
      availability: Array.from({ length: 18 }, () => "healthy"),
    },
  ],
  unavailable: false,
};

describe("trayProviderMini service", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("accepts the bounded snapshot contract and preserves eighteen cells", () => {
    const withDegraded = {
      ...snapshot,
      providers: [
        {
          ...snapshot.providers[0],
          availability: [
            "degraded",
            ...snapshot.providers[0]!.availability.slice(1),
          ],
        },
      ],
    } satisfies TrayProviderMiniSnapshot;
    expect(normalizeTrayProviderMiniSnapshot(withDegraded)).toEqual(withDegraded);
  });

  it("fails closed on malformed providers and repairs malformed bucket counts", () => {
    expect(
      normalizeTrayProviderMiniSnapshot({
        ...snapshot,
        providers: [{ ...snapshot.providers[0], providerId: 0 }],
      })
    ).toBeNull();

    const normalized = normalizeTrayProviderMiniSnapshot({
      ...snapshot,
      providers: [{ ...snapshot.providers[0], availability: ["healthy"] }],
    });
    expect(normalized?.providers[0]?.availability).toEqual(
      Array.from({ length: 18 }, () => "no_data")
    );

    expect(
      normalizeTrayProviderMiniSnapshot({
        ...snapshot,
        providers: [{ ...snapshot.providers[0], failureCount: -1 }],
      })
    ).toBeNull();
  });

  it("uses generated IPC for snapshot reads and hover reports", async () => {
    vi.mocked(commands.trayProviderMiniSnapshotGet).mockResolvedValueOnce(snapshot);
    vi.mocked(commands.trayProviderMiniWindowHoverSet).mockResolvedValueOnce(true);

    await expect(getTrayProviderMiniSnapshot()).resolves.toEqual(snapshot);
    await expect(setTrayProviderMiniWindowHovered(true)).resolves.toBeUndefined();
    expect(commands.trayProviderMiniSnapshotGet).toHaveBeenCalledOnce();
    expect(commands.trayProviderMiniWindowHoverSet).toHaveBeenCalledWith(true);
  });

  it("normalizes backend snapshot events", async () => {
    let eventHandler: ((payload: unknown) => void) | undefined;
    const cleanup = vi.fn();
    vi.mocked(listenDesktopEvent).mockImplementation(async (_event, handler) => {
      eventHandler = handler;
      return cleanup;
    });
    const listener = vi.fn();

    const unlisten = await listenTrayProviderMiniSnapshot(listener);
    eventHandler?.(snapshot);

    expect(listenDesktopEvent).toHaveBeenCalledWith(
      TRAY_PROVIDER_MINI_SNAPSHOT_EVENT,
      expect.any(Function)
    );
    expect(listener).toHaveBeenCalledWith(snapshot);
    unlisten();
    expect(cleanup).toHaveBeenCalledOnce();
  });
});
