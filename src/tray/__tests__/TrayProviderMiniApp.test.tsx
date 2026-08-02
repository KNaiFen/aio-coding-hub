import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  getTrayProviderMiniSnapshot,
  listenTrayProviderMiniSnapshot,
  setTrayProviderMiniWindowHovered,
  type TrayProviderMiniSnapshot,
} from "../../services/trayProviderMini";
import { TrayProviderMiniApp } from "../TrayProviderMiniApp";

vi.mock("../../hooks/useTheme", () => ({
  useTheme: () => ({ theme: "system", resolvedTheme: "light", setTheme: vi.fn() }),
}));

vi.mock("../../services/trayProviderMini", async () => {
  const actual = await vi.importActual<typeof import("../../services/trayProviderMini")>(
    "../../services/trayProviderMini"
  );
  return {
    ...actual,
    getTrayProviderMiniSnapshot: vi.fn(),
    listenTrayProviderMiniSnapshot: vi.fn(),
    setTrayProviderMiniWindowHovered: vi.fn(),
  };
});

const snapshot: TrayProviderMiniSnapshot = {
  generation: 4,
  generatedAtMs: 1_780_000_000_000,
  hours: 6,
  cliKey: "codex",
  selectionSource: "active_request",
  routeName: "日常路由",
  providers: [
    {
      providerId: 1,
      providerName: "大春",
      unavailableReasons: ["spend_limit", "oauth_limit"],
      availability: [
        "healthy",
        "healthy",
        "unhealthy",
        "no_data",
        "healthy",
        "healthy",
        "healthy",
        "healthy",
        "healthy",
        "healthy",
        "healthy",
        "healthy",
      ],
    },
    {
      providerId: 2,
      providerName: "备用供应商",
      unavailableReasons: ["circuit_open"],
      availability: Array.from({ length: 12 }, () => "no_data"),
    },
  ],
  unavailable: false,
};

describe("TrayProviderMiniApp", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(listenTrayProviderMiniSnapshot).mockResolvedValue(vi.fn());
    vi.mocked(getTrayProviderMiniSnapshot).mockResolvedValue(snapshot);
    vi.mocked(setTrayProviderMiniWindowHovered).mockResolvedValue();
  });

  it("renders the frozen CLI route, compact reasons, and twelve stable cells", async () => {
    render(<TrayProviderMiniApp />);

    expect(await screen.findByText("Codex")).toBeInTheDocument();
    expect(screen.getByText("日常路由")).toBeInTheDocument();
    expect(screen.getByText("6h")).toBeInTheDocument();
    expect(screen.getAllByText("限")).toHaveLength(1);
    expect(screen.getByText("熔")).toBeInTheDocument();
    const timelines = screen.getAllByLabelText("供应商可用性");
    expect(within(timelines[0]!).getAllByTitle(/正常|异常|无数据/)).toHaveLength(12);
    expect(within(timelines[1]!).getAllByTitle("无数据")).toHaveLength(12);
  });

  it("reports pointer handoff without adding focusable controls", async () => {
    const { container } = render(<TrayProviderMiniApp />);
    await screen.findByText("大春");
    const panel = container.querySelector("main");
    expect(panel).not.toBeNull();

    fireEvent.pointerEnter(panel!);
    fireEvent.pointerLeave(panel!);

    await waitFor(() => {
      expect(setTrayProviderMiniWindowHovered).toHaveBeenNthCalledWith(1, true);
      expect(setTrayProviderMiniWindowHovered).toHaveBeenNthCalledWith(2, false);
    });
    expect(container.querySelectorAll("button, input, a")).toHaveLength(0);
  });

  it("shows an explicit empty state when no CLI is currently proxied", async () => {
    vi.mocked(getTrayProviderMiniSnapshot).mockResolvedValue({
      ...snapshot,
      cliKey: null,
      selectionSource: null,
      routeName: null,
      providers: [],
    });

    render(<TrayProviderMiniApp />);

    expect(await screen.findByText("暂无已接管 CLI")).toBeInTheDocument();
  });

  it("resets scroll when a new frozen generation arrives", async () => {
    let snapshotHandler: ((next: TrayProviderMiniSnapshot | null) => void) | undefined;
    vi.mocked(listenTrayProviderMiniSnapshot).mockImplementation(async (handler) => {
      snapshotHandler = handler;
      return vi.fn();
    });
    const manyProviders = Array.from({ length: 12 }, (_, index) => ({
      ...snapshot.providers[0]!,
      providerId: index + 1,
      providerName: `供应商 ${index + 1}`,
    }));
    vi.mocked(getTrayProviderMiniSnapshot).mockResolvedValue({
      ...snapshot,
      providers: manyProviders,
    });
    const { container } = render(<TrayProviderMiniApp />);
    await screen.findByText("供应商 12");
    const scroller = container.querySelector("header + div") as HTMLDivElement;
    scroller.scrollTop = 160;

    act(() => {
      snapshotHandler?.({ ...snapshot, generation: snapshot.generation + 1 });
    });

    expect(scroller.scrollTop).toBe(0);
  });
});
