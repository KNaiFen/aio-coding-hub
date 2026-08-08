import { useAsyncListener } from "../hooks/useAsyncListener";
import { listenGatewayEvents } from "../services/gateway/gatewayEvents";
import { listenNoticeEvents } from "../services/notification/noticeEvents";
import { listenTaskCompleteNotifyEvents } from "../services/notification/taskCompleteNotifyEvents";

export function useAppEventListeners() {
  useAsyncListener(listenGatewayEvents, "listenGatewayEvents", "网关事件监听初始化失败");
  useAsyncListener(listenNoticeEvents, "listenNoticeEvents", "通知事件监听初始化失败");
  useAsyncListener(
    listenTaskCompleteNotifyEvents,
    "listenTaskCompleteNotifyEvents",
    "任务结束提醒监听初始化失败"
  );
}
