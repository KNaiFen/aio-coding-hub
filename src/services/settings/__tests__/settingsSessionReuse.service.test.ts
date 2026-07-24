import { describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import { settingsSessionReuseSet } from "../settingsSessionReuse";

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      settingsSessionReuseSet: vi.fn(),
    },
  };
});

vi.mock("../../consoleLog", async () => {
  const actual = await vi.importActual<typeof import("../../consoleLog")>("../../consoleLog");
  return {
    ...actual,
    logToConsole: vi.fn(),
  };
});

describe("services/settings/settingsSessionReuse", () => {
  it("rethrows invoke errors and logs", async () => {
    vi.mocked(commands.settingsSessionReuseSet).mockRejectedValueOnce(new Error("reuse boom"));

    await expect(settingsSessionReuseSet(false)).rejects.toThrow("reuse boom");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "保存会话复用设置失败",
      expect.objectContaining({
        cmd: "settings_session_reuse_set",
        error: expect.stringContaining("reuse boom"),
      })
    );
  });

  it("maps generated args and treats null as runtime error", async () => {
    vi.mocked(commands.settingsSessionReuseSet).mockResolvedValueOnce(null as any);
    await expect(settingsSessionReuseSet(false)).rejects.toThrow(
      "IPC_NULL_RESULT: settings_session_reuse_set"
    );

    vi.mocked(commands.settingsSessionReuseSet).mockResolvedValueOnce({
      status: "ok",
      data: { schema_version: 1 } as any,
    });
    await settingsSessionReuseSet(false);

    expect(commands.settingsSessionReuseSet).toHaveBeenCalledWith({
      enableSessionReuse: false,
    });
  });

  it("rejects malformed boolean input before generated commands", async () => {
    await expect(settingsSessionReuseSet("no" as any)).rejects.toThrow(
      "enableSessionReuse must be a boolean"
    );

    expect(commands.settingsSessionReuseSet).not.toHaveBeenCalled();
  });
});
