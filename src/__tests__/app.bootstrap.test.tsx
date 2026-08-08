import { fireEvent, render, screen } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createTestQueryClient } from "../test/utils/reactQuery";
import { createTestAppSettings } from "../test/fixtures/settings";

vi.mock("../services/app/appHeartbeat", () => ({
  listenAppHeartbeat: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock("../services/gateway/gatewayEvents", () => ({
  listenGatewayEvents: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock("../services/notification/noticeEvents", () => ({
  listenNoticeEvents: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock("../services/notification/taskCompleteNotifyEvents", () => ({
  listenTaskCompleteNotifyEvents: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock("../services/app/startup", () => ({
  appStartupStatusGet: vi.fn(),
  appStartupRetry: vi.fn(),
  listenAppStartupStatusEvents: vi.fn(),
  startupSyncDefaultPromptsFromFilesOncePerSession: vi.fn().mockResolvedValue(undefined),
  startupSyncModelPricesOnce: vi.fn().mockResolvedValue(undefined),
}));
vi.mock("../app/AppRoutes", () => ({
  AppRoutes: () => <div data-testid="app-routes" />,
}));
vi.mock("../services/backgroundTasks", () => ({
  registerBackgroundTask: vi.fn(() => vi.fn()),
  startBackgroundTaskScheduler: vi.fn(),
  setBackgroundTaskSchedulerForeground: vi.fn(),
  emitBackgroundTaskVisibilityTrigger: vi.fn(),
}));
vi.mock("../services/cli/cliProxy", () => ({
  cliProxyStatusAll: vi.fn().mockResolvedValue([]),
}));
vi.mock("../hooks/useUpdateMeta", async () => {
  const actual =
    await vi.importActual<typeof import("../hooks/useUpdateMeta")>("../hooks/useUpdateMeta");
  return {
    ...actual,
    updateCheckNow: vi.fn().mockResolvedValue(null),
  };
});
vi.mock("../services/settings/settings", async () => {
  const actual = await vi.importActual<typeof import("../services/settings/settings")>(
    "../services/settings/settings"
  );
  return {
    ...actual,
    settingsGet: vi.fn(),
  };
});
vi.mock("../app/settingsRuntimeController", () => ({
  applySettingsRuntimeSnapshot: vi.fn(),
  resetSettingsRuntimeController: vi.fn(),
}));
vi.mock("../services/consoleLog", () => ({ logToConsole: vi.fn() }));
vi.mock("../app/startupStatusStore", () => ({
  listenAndSyncAppStartupStatusSnapshot: vi.fn().mockResolvedValue(() => {}),
  retryAppStartupStatusSnapshot: vi.fn(),
  useAppStartupStatus: vi.fn(() => ({
    running: false,
    maintenanceMode: false,
    currentStage: "ready",
    failedStage: null,
    errorMessage: null,
    canRetry: false,
  })),
  useAppStartupStatusSynchronized: vi.fn(() => true),
}));

import { listenAppHeartbeat } from "../services/app/appHeartbeat";
import {
  listenAndSyncAppStartupStatusSnapshot,
  retryAppStartupStatusSnapshot,
  useAppStartupStatus,
  useAppStartupStatusSynchronized,
} from "../app/startupStatusStore";
import {
  registerBackgroundTask,
  setBackgroundTaskSchedulerForeground,
  startBackgroundTaskScheduler,
} from "../services/backgroundTasks";
import { listenGatewayEvents } from "../services/gateway/gatewayEvents";
import { listenNoticeEvents } from "../services/notification/noticeEvents";
import { settingsGet } from "../services/settings/settings";
import {
  startupSyncDefaultPromptsFromFilesOncePerSession,
  startupSyncModelPricesOnce,
} from "../services/app/startup";
import { listenTaskCompleteNotifyEvents } from "../services/notification/taskCompleteNotifyEvents";
import { updateCheckNow } from "../hooks/useUpdateMeta";
import { cliProxyStatusAll } from "../services/cli/cliProxy";
import {
  applySettingsRuntimeSnapshot,
  resetSettingsRuntimeController,
} from "../app/settingsRuntimeController";
import { logToConsole } from "../services/consoleLog";

const READY_STATUS = {
  running: false,
  maintenanceMode: false,
  currentStage: "ready" as const,
  failedStage: null,
  errorMessage: null,
  canRetry: false,
};

const MAINTENANCE_STATUS = {
  running: false,
  maintenanceMode: true,
  currentStage: "failed" as const,
  failedStage: "resetting_data" as const,
  errorMessage: "数据重置未完成",
  canRetry: true,
};

async function renderApp() {
  const { default: App } = await import("../App");
  const client = createTestQueryClient();
  const tree = () => (
    <QueryClientProvider client={client}>
      <App />
    </QueryClientProvider>
  );
  const rendered = render(tree());
  return {
    ...rendered,
    rerenderApp: () => rendered.rerender(tree()),
  };
}

describe("App bootstrap", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(listenAppHeartbeat).mockResolvedValue(() => {});
    vi.mocked(listenGatewayEvents).mockResolvedValue(() => {});
    vi.mocked(listenNoticeEvents).mockResolvedValue(() => {});
    vi.mocked(listenTaskCompleteNotifyEvents).mockResolvedValue(() => {});
    vi.mocked(listenAndSyncAppStartupStatusSnapshot).mockResolvedValue(() => {});
    vi.mocked(retryAppStartupStatusSnapshot).mockResolvedValue(undefined);
    vi.mocked(useAppStartupStatus).mockReturnValue(READY_STATUS);
    vi.mocked(useAppStartupStatusSynchronized).mockReturnValue(true);
    vi.mocked(registerBackgroundTask).mockImplementation(() => vi.fn());
    vi.mocked(startupSyncModelPricesOnce).mockResolvedValue(undefined);
    vi.mocked(startupSyncDefaultPromptsFromFilesOncePerSession).mockResolvedValue(undefined);
    vi.mocked(resetSettingsRuntimeController).mockImplementation(() => {});
    vi.mocked(settingsGet).mockResolvedValue(
      createTestAppSettings({
        enable_cache_anomaly_monitor: true,
        enable_task_complete_notify: false,
      })
    );
  });

  it("wires listeners, startup tasks, and settings-driven toggles", async () => {
    await renderApp();

    await vi.waitFor(() => {
      expect(listenAppHeartbeat).toHaveBeenCalledTimes(1);
      expect(listenAndSyncAppStartupStatusSnapshot).toHaveBeenCalledTimes(1);
      expect(listenGatewayEvents).toHaveBeenCalledTimes(1);
      expect(listenNoticeEvents).toHaveBeenCalledTimes(1);
      expect(listenTaskCompleteNotifyEvents).toHaveBeenCalledTimes(1);
      expect(startupSyncModelPricesOnce).toHaveBeenCalledTimes(1);
      expect(startupSyncDefaultPromptsFromFilesOncePerSession).toHaveBeenCalledTimes(1);
      expect(applySettingsRuntimeSnapshot).toHaveBeenCalledWith(
        expect.objectContaining({
          enable_cache_anomaly_monitor: true,
          enable_task_complete_notify: false,
        })
      );
      expect(registerBackgroundTask).toHaveBeenCalledTimes(2);
      expect(startBackgroundTaskScheduler).toHaveBeenCalledTimes(1);
      expect(setBackgroundTaskSchedulerForeground).toHaveBeenCalledWith(true);
      expect(updateCheckNow).not.toHaveBeenCalled();
      expect(cliProxyStatusAll).not.toHaveBeenCalled();
    });
  });

  it("keeps normal routes and background hooks unmounted during reset maintenance", async () => {
    vi.mocked(useAppStartupStatus).mockReturnValue(MAINTENANCE_STATUS);

    await renderApp();

    expect(await screen.findByText("数据维护尚未完成")).toBeInTheDocument();
    expect(screen.queryByTestId("app-routes")).not.toBeInTheDocument();
    expect(listenAppHeartbeat).toHaveBeenCalledTimes(1);
    expect(listenAndSyncAppStartupStatusSnapshot).toHaveBeenCalledTimes(1);
    expect(listenGatewayEvents).not.toHaveBeenCalled();
    expect(listenNoticeEvents).not.toHaveBeenCalled();
    expect(listenTaskCompleteNotifyEvents).not.toHaveBeenCalled();
    expect(startupSyncModelPricesOnce).not.toHaveBeenCalled();
    expect(startupSyncDefaultPromptsFromFilesOncePerSession).not.toHaveBeenCalled();
    expect(registerBackgroundTask).not.toHaveBeenCalled();
    expect(startBackgroundTaskScheduler).not.toHaveBeenCalled();
  });

  it("surfaces maintenance retry failures without an unhandled rejection", async () => {
    vi.mocked(useAppStartupStatus).mockReturnValue(MAINTENANCE_STATUS);
    vi.mocked(retryAppStartupStatusSnapshot).mockRejectedValueOnce(new Error("retry boom"));

    await renderApp();
    fireEvent.click(await screen.findByRole("button", { name: "重试" }));

    expect(await screen.findByText("重试维护操作失败：请查看 Console 日志")).toBeInTheDocument();
    expect(logToConsole).toHaveBeenCalledWith("error", "重试维护操作失败", {
      error: "Error: retry boom",
    });
    expect(screen.getByRole("button", { name: "重试" })).toBeEnabled();
  });

  it("unmounts runtime owners when maintenance begins and mounts them once after recovery", async () => {
    const unlistenGateway = vi.fn();
    const unlistenNotice = vi.fn();
    const unlistenTaskComplete = vi.fn();
    const unregisterCliProxy = vi.fn();
    const unregisterUpdate = vi.fn();
    vi.mocked(listenGatewayEvents).mockResolvedValue(unlistenGateway);
    vi.mocked(listenNoticeEvents).mockResolvedValue(unlistenNotice);
    vi.mocked(listenTaskCompleteNotifyEvents).mockResolvedValue(unlistenTaskComplete);
    vi.mocked(registerBackgroundTask)
      .mockReturnValueOnce(unregisterCliProxy)
      .mockReturnValueOnce(unregisterUpdate)
      .mockImplementation(() => vi.fn());

    const app = await renderApp();
    await vi.waitFor(() => expect(listenGatewayEvents).toHaveBeenCalledTimes(1));
    await vi.waitFor(() => expect(registerBackgroundTask).toHaveBeenCalledTimes(2));

    vi.mocked(useAppStartupStatus).mockReturnValue(MAINTENANCE_STATUS);
    app.rerenderApp();

    expect(await screen.findByText("数据维护尚未完成")).toBeInTheDocument();
    await vi.waitFor(() => {
      expect(unlistenGateway).toHaveBeenCalledTimes(1);
      expect(unlistenNotice).toHaveBeenCalledTimes(1);
      expect(unlistenTaskComplete).toHaveBeenCalledTimes(1);
      expect(unregisterCliProxy).toHaveBeenCalledTimes(1);
      expect(unregisterUpdate).toHaveBeenCalledTimes(1);
    });

    vi.mocked(useAppStartupStatus).mockReturnValue(READY_STATUS);
    app.rerenderApp();

    await vi.waitFor(() => expect(listenGatewayEvents).toHaveBeenCalledTimes(2));
    expect(listenNoticeEvents).toHaveBeenCalledTimes(2);
    expect(listenTaskCompleteNotifyEvents).toHaveBeenCalledTimes(2);
    expect(registerBackgroundTask).toHaveBeenCalledTimes(4);
    expect(listenAndSyncAppStartupStatusSnapshot).toHaveBeenCalledTimes(1);
  });
});
