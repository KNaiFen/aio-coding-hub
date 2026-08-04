import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  getTrayProviderMiniSnapshot,
  listenTrayProviderMiniSnapshot,
  setTrayProviderMiniWindowHovered,
  type TrayProviderMiniSnapshot,
} from "../../services/trayProviderMini";
import { formatTrayProviderMiniCount, TrayProviderMiniApp } from "../TrayProviderMiniApp";

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
      successCount: 23,
      failureCount: 1,
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
      successCount: 0,
      failureCount: 4,
      availability: Array.from({ length: 18 }, () => "no_data"),
    },
  ],
  unavailable: false,
};

describe("formatTrayProviderMiniCount", () => {
  it.each([
    [0, "0"],
    [9, "9"],
    [1_034, "1034"],
    [99_999, "99999"],
    [100_000, "10万"],
    [123_000, "12.3万"],
    [123_456, "12.3万"],
    [999_999, "99.9万"],
    [1_000_000, "100万"],
    [99_999_999, "9999万"],
    [100_000_000, "1亿"],
    [4_294_967_295, "42.9亿"],
  ])("formats %i as %s without overstating the exact count", (count, expected) => {
    expect(formatTrayProviderMiniCount(count)).toBe(expected);
  });
});

describe("TrayProviderMiniApp", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(listenTrayProviderMiniSnapshot).mockResolvedValue(vi.fn());
    vi.mocked(getTrayProviderMiniSnapshot).mockResolvedValue(snapshot);
    vi.mocked(setTrayProviderMiniWindowHovered).mockResolvedValue();
  });

  it("renders the frozen CLI route, totals, and eighteen stable cells", async () => {
    render(<TrayProviderMiniApp />);

    expect(await screen.findByText("Codex")).toBeInTheDocument();
    expect(screen.getByText("日常路由")).toBeInTheDocument();
    expect(screen.getByText("6h")).toBeInTheDocument();
    expect(screen.getAllByText("限")).toHaveLength(1);
    expect(screen.getByText("熔")).toBeInTheDocument();
    expect(screen.getByLabelText("总计 成功 23，失败 1")).toBeInTheDocument();
    expect(screen.getByLabelText("总计 成功 0，失败 4")).toBeInTheDocument();
    expect(screen.queryByText("成")).not.toBeInTheDocument();
    expect(screen.queryByText("败")).not.toBeInTheDocument();
    const timelines = screen.getAllByLabelText("供应商可用性");
    expect(within(timelines[0]!).getAllByTitle(/正常|异常|无数据/)).toHaveLength(18);
    expect(within(timelines[1]!).getAllByTitle("无数据")).toHaveLength(18);
  });

  it("keeps provider, availability, and exact total columns fixed", async () => {
    const longChineseName = "这是一个很长很长的中文供应商名称";
    const longEnglishName = "provider-with-a-very-long-english-display-name";
    vi.mocked(getTrayProviderMiniSnapshot).mockResolvedValue({
      ...snapshot,
      providers: [
        {
          ...snapshot.providers[0]!,
          providerName: longChineseName,
          unavailableReasons: ["circuit_open", "cooldown", "spend_limit", "oauth_limit"],
          successCount: 0,
          failureCount: 9,
        },
        {
          ...snapshot.providers[1]!,
          providerName: longEnglishName,
          unavailableReasons: [],
          successCount: 1_034,
          failureCount: 99_999,
        },
        {
          ...snapshot.providers[0]!,
          providerId: 3,
          providerName: "超大计数",
          unavailableReasons: [],
          successCount: 123_456,
          failureCount: 4_294_967_295,
        },
      ],
    });

    const { container } = render(<TrayProviderMiniApp />);
    expect(await screen.findByText("12.3万")).toBeInTheDocument();
    expect(screen.getByText("42.9亿")).toBeInTheDocument();

    const rows = container.querySelectorAll("header + div > .divide-y > div");
    expect(rows).toHaveLength(3);
    rows.forEach((row) => {
      expect(row).toHaveClass("grid-cols-[92px_198px_72px]");
    });

    expect(screen.getByTitle(longChineseName)).toHaveClass("truncate");
    expect(screen.getByTitle(longEnglishName)).toHaveClass("truncate");
    expect(screen.getByText("熔")).toBeInTheDocument();
    expect(screen.getByText("冷")).toBeInTheDocument();
    expect(screen.getAllByText("限")).toHaveLength(1);
    screen
      .getAllByLabelText("供应商状态")
      .forEach((markers) => expect(markers).toHaveClass("min-w-[40px]", "shrink-0"));

    const totals = [
      screen.getByLabelText("总计 成功 0，失败 9"),
      screen.getByLabelText("总计 成功 1034，失败 99999"),
      screen.getByLabelText("总计 成功 123456，失败 4294967295"),
    ];
    totals.forEach((total) => {
      expect(total).toHaveClass("grid-cols-[32px_32px]", "gap-2");
      expect(total).toHaveAttribute("role", "group");
      expect(total.children).toHaveLength(2);
      expect(within(total).queryByText("成")).not.toBeInTheDocument();
      expect(within(total).queryByText("败")).not.toBeInTheDocument();
      total.querySelectorAll("span[title]").forEach((value) => {
        expect(value).toHaveClass("font-mono", "text-[9px]", "whitespace-nowrap", "text-right");
      });
    });
    expect(screen.getByTitle("123456")).toHaveTextContent("12.3万");
    expect(screen.getByTitle("4294967295")).toHaveTextContent("42.9亿");
    expect(container.querySelector("main")).toHaveClass("tray-provider-mini-surface");
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
    const rows = container.querySelectorAll("header + div > .divide-y > div");

    expect(scroller).toHaveClass("max-h-[240px]");
    expect(rows).toHaveLength(12);
    rows.forEach((row) => expect(row).toHaveClass("h-6"));
    expect(screen.getByText("供应商 12")).toBeInTheDocument();
    scroller.scrollTop = 160;

    act(() => {
      snapshotHandler?.({
        ...snapshot,
        generation: snapshot.generation + 1,
        providers: manyProviders,
      });
    });

    expect(scroller.scrollTop).toBe(0);
    expect(screen.getByText("供应商 12")).toBeInTheDocument();
  });
});
