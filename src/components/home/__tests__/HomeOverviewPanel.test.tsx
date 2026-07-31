import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { type ComponentProps, type ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { CliKey } from "../../../constants/clis";
import { HomeOverviewPanel } from "../HomeOverviewPanel";
import type { HomeRequestLogsPanelProps } from "../HomeRequestLogsPanel";
import type {
  HomeCliWorkspaceConfig,
  HomeWorkspaceConfigItem,
  HomeWorkspaceConfigItemType,
} from "../homeWorkspaceConfigTypes";

const { homeRequestLogsPanelMock } = vi.hoisted(() => ({
  homeRequestLogsPanelMock: vi.fn((_props: HomeRequestLogsPanelProps) => <div>request-logs</div>),
}));

vi.mock("../HomeUsageSection", () => ({
  HomeUsageSection: ({ usageWindowDays }: { usageWindowDays?: number }) => (
    <div>{`usage-section:${String(usageWindowDays)}`}</div>
  ),
}));

vi.mock("../HomeActiveSessionsCard", () => ({
  HomeActiveSessionsCardContent: ({
    activeSessions,
  }: {
    activeSessions: Array<{ session_id: string }>;
  }) => <div>active-sessions:{activeSessions.length}</div>,
}));

vi.mock("../HomeProviderLimitPanel", () => ({
  HomeProviderLimitPanelContent: ({ rows }: { rows: Array<{ provider_id: number }> }) => (
    <div>provider-limit:{rows.length}</div>
  ),
}));

vi.mock("../HomeOAuthQuotaPanel", () => ({
  HomeOAuthQuotaPanelContent: ({
    rows,
    hasProviders,
    hasRefreshed,
    refreshing,
    onRefresh,
    onRefreshRow,
  }: {
    rows: Array<{ providerId: number }>;
    hasProviders: boolean;
    hasRefreshed: boolean;
    refreshing: boolean;
    onRefresh?: () => void;
    onRefreshRow?: (providerId: number) => void;
  }) => (
    <div>
      <div>{`oauth-quota:${rows.length}:${String(hasProviders)}:${String(hasRefreshed)}:${String(refreshing)}`}</div>
      <button type="button" onClick={() => onRefresh?.()}>
        refresh-oauth-quota
      </button>
      <button type="button" onClick={() => onRefreshRow?.(rows[0]?.providerId ?? 0)}>
        refresh-oauth-quota-row
      </button>
    </div>
  ),
}));

vi.mock("../HomeWorkspaceConfigPanel", () => ({
  HomeWorkspaceConfigPanel: ({
    configs,
    selectedCliKey,
    onSelectCliKey,
    headerAddon,
  }: {
    configs: HomeCliWorkspaceConfig[];
    selectedCliKey: CliKey | null;
    onSelectCliKey: (cliKey: CliKey) => void;
    headerAddon?: ReactNode;
  }) => {
    const selectedConfig =
      configs.find((config) => config.cliKey === selectedCliKey) ?? configs[0] ?? null;

    if (!selectedConfig) return <div>workspace-config:empty</div>;

    return (
      <div>
        <div>
          {configs.map((config) => (
            <button key={config.cliKey} type="button" onClick={() => onSelectCliKey(config.cliKey)}>
              {config.cliLabel}
            </button>
          ))}
        </div>
        <div>{`selected-workspace:${selectedConfig.workspaceName?.trim() || "默认"}`}</div>
        {headerAddon}
        {selectedConfig.items.map((item) => (
          <div key={item.id}>{item.name}</div>
        ))}
      </div>
    );
  },
}));

vi.mock("../HomeRequestLogsPanel", () => ({
  HomeRequestLogsPanel: homeRequestLogsPanelMock,
}));

function makeWorkspaceItem(
  id: number,
  type: HomeWorkspaceConfigItemType,
  label: string,
  name: string,
  enabled = true
): HomeWorkspaceConfigItem {
  const prefix = type === "prompts" ? "prompt" : type === "mcp" ? "mcp" : "skill";
  return {
    id: `${prefix}:${id}`,
    resourceId: id,
    type,
    label,
    name,
    enabled,
  };
}

function makeWorkspaceConfig(
  cliKey: CliKey,
  cliLabel: string,
  workspaceId: number,
  workspaceName: string
): HomeCliWorkspaceConfig {
  return {
    cliKey,
    cliLabel,
    workspaceId,
    workspaceName,
    workspaces: [{ id: workspaceId, name: workspaceName, isActive: true }],
    loading: false,
    items: [makeWorkspaceItem(workspaceId, "prompts", "Prompt", `${cliLabel} Prompt`)],
  };
}

type PanelProps = ComponentProps<typeof HomeOverviewPanel>;
type PanelOverrides = Omit<Partial<PanelProps>, "displayOptions"> & {
  displayOptions?: Partial<PanelProps["displayOptions"]>;
};

function makePanelProps(overrides: PanelOverrides = {}): PanelProps {
  const { displayOptions, ...rest } = overrides;
  return {
    displayOptions: {
      customTooltip: false,
      usage: true,
      workspaceConfigQuickToggle: false,
      ...displayOptions,
    },
    devPreviewEnabled: false,
    cliPriorityOrder: ["claude", "codex", "gemini", "grok"],
    visibleTabKeys: ["workspaceConfig", "circuit", "sessions", "providerLimit", "oauthQuota"],
    visibleCliKeys: ["claude", "codex", "gemini", "grok"],
    usageWindowDays: 15,
    usageHeatmapRows: [],
    usageHeatmapLoading: false,
    sortModes: [],
    sortModesLoading: false,
    sortModesAvailable: true,
    activeModeByCli: { claude: null, codex: null, gemini: null, grok: null },
    activeModeToggling: { claude: false, codex: false, gemini: false, grok: false },
    onSetCliActiveMode: vi.fn(),
    activeSessions: [],
    activeSessionsLoading: false,
    activeSessionsAvailable: true,
    workspaceConfigs: [
      makeWorkspaceConfig("claude", "Claude", 1, "Claude Workspace"),
      makeWorkspaceConfig("codex", "Codex", 2, "Codex Workspace"),
      makeWorkspaceConfig("gemini", "Gemini", 3, "Gemini Workspace"),
    ],
    providerLimitRows: [],
    providerLimitLoading: false,
    providerLimitAvailable: true,
    providerLimitRefreshing: false,
    onRefreshProviderLimit: vi.fn(),
    oauthQuotaRows: [],
    oauthQuotaVisible: false,
    oauthQuotaRefreshing: false,
    oauthQuotaHasRefreshed: false,
    onRefreshOAuthQuota: vi.fn().mockResolvedValue(undefined),
    onRefreshOAuthQuotaRow: vi.fn().mockResolvedValue(undefined),
    openCircuits: [],
    onResetCircuitProvider: vi.fn(),
    resettingCircuitProviderIds: new Set(),
    traces: [],
    requestLogs: [],
    activeRequestsAvailable: true,
    requestLogsLoading: false,
    requestLogsRefreshing: false,
    requestLogsAvailable: true,
    onRefreshRequestLogs: vi.fn(),
    selectedLogId: null,
    onSelectLogId: vi.fn(),
    ...rest,
  };
}

function renderPanel(overrides: PanelOverrides = {}) {
  const props = makePanelProps(overrides);
  return {
    props,
    ...render(<HomeOverviewPanel {...props} />),
  };
}

describe("components/home/HomeOverviewPanel", () => {
  beforeEach(() => {
    window.localStorage.clear();
    homeRequestLogsPanelMock.mockClear();
  });

  it("renders the compact usage and information column beside full-height request logs", () => {
    renderPanel();

    const usage = screen.getByText("usage-section:15");
    const requestLogs = screen.getByText("request-logs");
    expect(usage.parentElement).toHaveClass("xl:col-span-5");
    expect(requestLogs.parentElement).toHaveClass("xl:col-span-7");
    expect(screen.getByRole("tab", { name: "配置信息" })).toBeInTheDocument();
  });

  it("keeps request log controls available in the unified layout", () => {
    renderPanel({ activeRequestsAvailable: false });

    const latestCall =
      homeRequestLogsPanelMock.mock.calls[homeRequestLogsPanelMock.mock.calls.length - 1]?.[0];
    expect(latestCall).toMatchObject({
      activeRequestsAvailable: false,
      showCurrentConcurrency: true,
      displayOptions: {
        compactModeToggle: true,
        refreshButton: true,
      },
    });
    expect(latestCall).not.toHaveProperty("compactModeOverride");
  });

  it("hides the usage card without affecting information tabs or request logs", () => {
    renderPanel({ displayOptions: { usage: false } });

    expect(screen.queryByText(/usage-section:/)).not.toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "配置信息" })).toBeInTheDocument();
    expect(screen.getByText("request-logs")).toBeInTheDocument();
  });

  it("renders only the configured information tabs", () => {
    renderPanel({ visibleTabKeys: ["workspaceConfig", "sessions"] });

    expect(screen.getAllByRole("tab").map((tab) => tab.textContent)).toEqual([
      "配置信息",
      "活跃 Session",
    ]);
    expect(screen.queryByRole("tab", { name: "熔断信息" })).not.toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "供应商限额" })).not.toBeInTheDocument();
  });

  it("falls back to all information tabs when an invalid caller hides every tab", () => {
    renderPanel({ visibleTabKeys: [] });

    expect(screen.getAllByRole("tab")).toHaveLength(5);
  });

  it("filters workspace CLI buttons while preserving CLI priority order", async () => {
    renderPanel({
      cliPriorityOrder: ["gemini", "codex", "claude", "grok"],
      visibleCliKeys: ["codex", "gemini"],
    });

    expect(
      (await screen.findAllByRole("button", { name: /Codex|Gemini/ })).map(
        (button) => button.textContent
      )
    ).toEqual(["Gemini", "Codex"]);
    expect(screen.queryByRole("button", { name: "Claude" })).not.toBeInTheDocument();
    expect(screen.getByText("selected-workspace:Gemini Workspace")).toBeInTheDocument();
  });

  it("moves workspace selection to the first visible CLI after preferences change", async () => {
    const props = makePanelProps({ visibleCliKeys: ["codex", "gemini"] });
    const view = render(<HomeOverviewPanel {...props} />);

    fireEvent.click(await screen.findByRole("button", { name: "Gemini" }));
    expect(screen.getByText("selected-workspace:Gemini Workspace")).toBeInTheDocument();

    view.rerender(<HomeOverviewPanel {...props} visibleCliKeys={["codex"]} />);
    await waitFor(() => {
      expect(screen.getByText("selected-workspace:Codex Workspace")).toBeInTheDocument();
    });
  });

  it("moves tab selection to the first visible tab after preferences change", async () => {
    const props = makePanelProps({ visibleTabKeys: ["workspaceConfig", "sessions"] });
    const view = render(<HomeOverviewPanel {...props} />);

    fireEvent.click(screen.getByRole("tab", { name: "活跃 Session" }));
    expect(await screen.findByText("active-sessions:0")).toBeInTheDocument();

    view.rerender(<HomeOverviewPanel {...props} visibleTabKeys={["workspaceConfig"]} />);
    await waitFor(() => {
      expect(screen.getByRole("tab", { name: "配置信息" })).toHaveAttribute(
        "aria-selected",
        "true"
      );
    });
  });

  it("renders route strategy for the selected visible CLI", async () => {
    const onSetCliActiveMode = vi.fn();
    renderPanel({
      visibleCliKeys: ["codex"],
      sortModes: [{ id: 1, name: "工作策略", created_at: 1, updated_at: 1 }],
      onSetCliActiveMode,
    });

    const strategy = await screen.findByRole("combobox", { name: "Codex 路由策略" });
    fireEvent.change(strategy, { target: { value: "1" } });
    expect(onSetCliActiveMode).toHaveBeenCalledWith("codex", 1);
  });

  it("renders preview circuits without forwarding reset actions", () => {
    const onResetCircuitProvider = vi.fn();
    renderPanel({ devPreviewEnabled: true, onResetCircuitProvider });

    fireEvent.click(screen.getByRole("tab", { name: "熔断信息" }));
    expect(screen.getByText("Claude Main")).toBeInTheDocument();
    const reset = screen.getAllByRole("button", { name: "解除熔断" })[0];
    expect(reset).toBeDisabled();
    fireEvent.click(reset);
    expect(onResetCircuitProvider).not.toHaveBeenCalled();
  });

  it("uses real circuit rows and forwards reset actions", () => {
    const onResetCircuitProvider = vi.fn();
    renderPanel({
      openCircuits: [
        {
          cli_key: "claude",
          provider_id: 7,
          provider_name: "Real Claude Provider",
          displayState: "open",
          open_until: Math.floor(Date.now() / 1000) + 60,
        },
      ],
      onResetCircuitProvider,
    });

    fireEvent.click(screen.getByRole("tab", { name: "熔断信息" }));
    fireEvent.click(screen.getByRole("button", { name: "解除熔断" }));
    expect(onResetCircuitProvider).toHaveBeenCalledWith(7);
  });

  it("auto-switches to visible circuit information when a circuit opens", async () => {
    const props = makePanelProps();
    const view = render(<HomeOverviewPanel {...props} />);
    fireEvent.click(screen.getByRole("tab", { name: "供应商限额" }));

    view.rerender(
      <HomeOverviewPanel
        {...props}
        openCircuits={[
          {
            cli_key: "codex",
            provider_id: 9,
            provider_name: "Codex Circuit",
            displayState: "open",
            open_until: Math.floor(Date.now() / 1000) + 60,
          },
        ]}
      />
    );

    expect(await screen.findByText("Codex Circuit")).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "熔断信息" })).toHaveAttribute("aria-selected", "true");
  });

  it("does not auto-switch to circuit information when that tab is hidden", async () => {
    const props = makePanelProps({ visibleTabKeys: ["workspaceConfig", "providerLimit"] });
    const view = render(<HomeOverviewPanel {...props} />);
    fireEvent.click(screen.getByRole("tab", { name: "供应商限额" }));

    view.rerender(
      <HomeOverviewPanel
        {...props}
        openCircuits={[
          {
            cli_key: "codex",
            provider_id: 9,
            provider_name: "Hidden Circuit",
            displayState: "open",
            open_until: Math.floor(Date.now() / 1000) + 60,
          },
        ]}
      />
    );

    await waitFor(() => {
      expect(screen.getByRole("tab", { name: "供应商限额" })).toHaveAttribute(
        "aria-selected",
        "true"
      );
    });
    expect(screen.queryByText("Hidden Circuit")).not.toBeInTheDocument();
  });

  it("renders preview active sessions and provider limits", async () => {
    renderPanel({ devPreviewEnabled: true });

    fireEvent.click(screen.getByRole("tab", { name: "活跃 Session" }));
    expect(await screen.findByText("active-sessions:3")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("tab", { name: "供应商限额" }));
    expect(await screen.findByText("provider-limit:3")).toBeInTheDocument();
  });

  it("forwards OAuth quota refresh actions for real data", async () => {
    const onRefreshOAuthQuota = vi.fn().mockResolvedValue(undefined);
    const onRefreshOAuthQuotaRow = vi.fn().mockResolvedValue(undefined);
    renderPanel({
      oauthQuotaVisible: true,
      oauthQuotaRows: [{ providerId: 9 } as never],
      oauthQuotaHasRefreshed: true,
      onRefreshOAuthQuota,
      onRefreshOAuthQuotaRow,
    });

    fireEvent.click(screen.getByRole("tab", { name: "OAuth 配额" }));
    expect(await screen.findByText("oauth-quota:1:true:true:false")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "refresh-oauth-quota" }));
    fireEvent.click(screen.getByRole("button", { name: "refresh-oauth-quota-row" }));
    expect(onRefreshOAuthQuota).toHaveBeenCalledTimes(1);
    expect(onRefreshOAuthQuotaRow).toHaveBeenCalledWith(9);
  });

  it("restores persisted tab order before applying visibility", () => {
    window.localStorage.setItem(
      "aio-home-overview-tab-order",
      JSON.stringify(["providerLimit", "sessions", "circuit", "workspaceConfig", "oauthQuota"])
    );

    renderPanel({ visibleTabKeys: ["workspaceConfig", "sessions", "providerLimit"] });

    expect(screen.getAllByRole("tab").map((tab) => tab.textContent)).toEqual([
      "供应商限额",
      "活跃 Session",
      "配置信息",
    ]);
  });
});
