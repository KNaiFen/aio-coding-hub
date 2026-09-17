import { commands, type ObserverLanStatus } from "../generated/bindings";
import { invokeGeneratedIpc } from "./generatedIpc";

export type { ObserverLanStatus };
export const observerLanStatus = () => invokeGeneratedIpc<ObserverLanStatus>({
  title: "读取局域网观察状态失败", cmd: "observer_lan_status", invoke: () => commands.observerLanStatus(),
});
export const observerLanConfigure = (enabled: boolean, port: number) => invokeGeneratedIpc<ObserverLanStatus>({
  title: "保存局域网观察设置失败", cmd: "observer_lan_configure", invoke: () => commands.observerLanConfigure(enabled, port),
});
export const observerLanToken = (rotate: boolean) => invokeGeneratedIpc<string>({
  title: "读取观察令牌失败", cmd: rotate ? "observer_lan_token_rotate" : "observer_lan_token_reveal",
  invoke: () => rotate ? commands.observerLanTokenRotate() : commands.observerLanTokenReveal(),
});
