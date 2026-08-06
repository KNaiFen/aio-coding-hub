import { useAsyncListener } from "../hooks/useAsyncListener";
import { listenAppHeartbeat } from "../services/app/appHeartbeat";
import { useAppBackgroundTasks } from "./useAppBackgroundTasks";
import { useAppEventListeners } from "./useAppEventListeners";
import { useAppRuntimeSync } from "./useAppRuntimeSync";
import { useAppStartupTasks } from "./useAppStartupTasks";
import {
  listenAndSyncAppStartupStatusSnapshot,
  useAppStartupStatus,
  useAppStartupStatusSynchronized,
} from "./startupStatusStore";

export function useAppBootstrap() {
  useAsyncListener(listenAppHeartbeat, "listenAppHeartbeat", "应用心跳监听初始化失败");
  useAsyncListener(
    listenAndSyncAppStartupStatusSnapshot,
    "listenAndSyncAppStartupStatusSnapshot",
    "应用启动状态监听初始化失败"
  );

  return {
    status: useAppStartupStatus(),
    synchronized: useAppStartupStatusSynchronized(),
  };
}

export function AppRuntimeServices() {
  useAppRuntimeSync();
  useAppEventListeners();
  useAppStartupTasks();
  useAppBackgroundTasks();
  return null;
}
