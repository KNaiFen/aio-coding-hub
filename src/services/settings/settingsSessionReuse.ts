import { commands } from "../../generated/bindings";
import type { AppSettings } from "./settings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";
import { normalizeBooleanSetting } from "./settingsPrimitiveValidation";

export async function settingsSessionReuseSet(enable: boolean) {
  const normalizedEnable = normalizeBooleanSetting(enable, "enableSessionReuse");
  const update = {
    enableSessionReuse: normalizedEnable,
  };

  return invokeGeneratedIpc<AppSettings>({
    title: "保存会话复用设置失败",
    cmd: "settings_session_reuse_set",
    args: { update },
    invoke: () =>
      commands.settingsSessionReuseSet(update) as Promise<GeneratedCommandResult<AppSettings>>,
  });
}
