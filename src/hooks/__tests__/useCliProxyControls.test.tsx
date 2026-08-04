import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { envConflictsCheck } from "../../services/cli/envConflicts";
import { logToConsole } from "../../services/consoleLog";
import { createDeferred } from "../../test/utils/deferred";
import { useCliProxy } from "../useCliProxy";
import { useCliProxyControls } from "../useCliProxyControls";

vi.mock("../../services/consoleLog", () => ({ logToConsole: vi.fn() }));

vi.mock("../../services/cli/envConflicts", async () => {
  const actual = await vi.importActual<typeof import("../../services/cli/envConflicts")>(
    "../../services/cli/envConflicts"
  );
  return { ...actual, envConflictsCheck: vi.fn() };
});

vi.mock("../useCliProxy", async () => {
  const actual = await vi.importActual<typeof import("../useCliProxy")>("../useCliProxy");
  return { ...actual, useCliProxy: vi.fn() };
});

function makeCliProxyState() {
  return {
    loading: false,
    available: true,
    enabled: { claude: false, codex: false, gemini: false },
    appliedToCurrentGateway: { claude: null, codex: null, gemini: null },
    toggling: { claude: false, codex: false, gemini: false },
    setCliProxyEnabled: vi.fn(),
  };
}

describe("hooks/useCliProxyControls", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("immediately disables the target cli without running env conflict checks", () => {
    const cliProxyState = makeCliProxyState();
    vi.mocked(useCliProxy).mockReturnValue(cliProxyState as any);

    const { result } = renderHook(() => useCliProxyControls());

    act(() => {
      result.current.requestCliProxyEnabledSwitch("codex", false);
    });

    expect(cliProxyState.setCliProxyEnabled).toHaveBeenCalledWith("codex", false);
    expect(envConflictsCheck).not.toHaveBeenCalled();
  });

  it("marks the current cli as checking and ignores duplicate enable requests", async () => {
    const cliProxyState = makeCliProxyState();
    vi.mocked(useCliProxy).mockReturnValue(cliProxyState as any);

    let resolveCheck: (value: []) => void = () => {};
    vi.mocked(envConflictsCheck).mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          resolveCheck = resolve as (value: []) => void;
        })
    );

    const { result } = renderHook(() => useCliProxyControls());

    act(() => {
      result.current.requestCliProxyEnabledSwitch("codex", true);
    });

    expect(result.current.cliProxyToggling.codex).toBe(true);

    act(() => {
      result.current.requestCliProxyEnabledSwitch("codex", true);
    });

    expect(envConflictsCheck).toHaveBeenCalledTimes(1);

    await act(async () => {
      resolveCheck([]);
      await Promise.resolve();
    });

    await waitFor(() =>
      expect(cliProxyState.setCliProxyEnabled).toHaveBeenCalledWith("codex", true)
    );
    expect(result.current.cliProxyToggling.codex).toBe(false);
  });

  it("serializes enable checks across cli keys in the same render", async () => {
    const cliProxyState = makeCliProxyState();
    const firstCheck = createDeferred<[]>();
    vi.mocked(useCliProxy).mockReturnValue(cliProxyState as any);
    vi.mocked(envConflictsCheck).mockReturnValueOnce(firstCheck.promise).mockResolvedValueOnce([]);

    const { result } = renderHook(() => useCliProxyControls());

    act(() => {
      result.current.requestCliProxyEnabledSwitch("codex", true);
      result.current.requestCliProxyEnabledSwitch("gemini", true);
    });

    expect(envConflictsCheck).toHaveBeenCalledTimes(1);
    expect(envConflictsCheck).toHaveBeenCalledWith("codex");
    expect(result.current.cliProxyEnableBusy).toBe(true);

    await act(async () => {
      firstCheck.resolve([]);
      await firstCheck.promise;
    });

    await waitFor(() => expect(result.current.cliProxyEnableBusy).toBe(false));

    act(() => {
      result.current.requestCliProxyEnabledSwitch("gemini", true);
    });

    await waitFor(() => expect(envConflictsCheck).toHaveBeenCalledTimes(2));
    expect(envConflictsCheck).toHaveBeenLastCalledWith("gemini");
  });

  it("holds the enable lock while a conflict prompt is open and releases it on cancel", async () => {
    const cliProxyState = makeCliProxyState();
    vi.mocked(useCliProxy).mockReturnValue(cliProxyState as any);
    vi.mocked(envConflictsCheck)
      .mockResolvedValueOnce([
        { var_name: "OPENAI_API_KEY", source_type: "system", source_path: "Process Environment" },
      ])
      .mockResolvedValueOnce([]);

    const { result } = renderHook(() => useCliProxyControls());

    act(() => {
      result.current.requestCliProxyEnabledSwitch("codex", true);
    });

    await waitFor(() => expect(result.current.pendingCliProxyEnablePrompt?.cliKey).toBe("codex"));
    expect(result.current.cliProxyEnableBusy).toBe(true);

    act(() => {
      result.current.requestCliProxyEnabledSwitch("gemini", true);
    });
    expect(envConflictsCheck).toHaveBeenCalledTimes(1);
    expect(result.current.pendingCliProxyEnablePrompt?.cliKey).toBe("codex");

    act(() => {
      result.current.cancelPendingCliProxyEnable();
    });
    expect(result.current.pendingCliProxyEnablePrompt).toBeNull();
    expect(result.current.cliProxyEnableBusy).toBe(false);

    act(() => {
      result.current.requestCliProxyEnabledSwitch("gemini", true);
    });

    await waitFor(() => expect(envConflictsCheck).toHaveBeenCalledTimes(2));
    expect(envConflictsCheck).toHaveBeenLastCalledWith("gemini");
  });

  it("prompts for confirmation when env conflicts are found before enabling", async () => {
    const cliProxyState = makeCliProxyState();
    vi.mocked(useCliProxy).mockReturnValue(cliProxyState as any);
    vi.mocked(envConflictsCheck)
      .mockResolvedValueOnce([
        { var_name: "OPENAI_API_KEY", source_type: "system", source_path: "Process Environment" },
      ])
      .mockResolvedValueOnce([]);

    const { result } = renderHook(() => useCliProxyControls());

    act(() => {
      result.current.requestCliProxyEnabledSwitch("codex", true);
    });

    await waitFor(() =>
      expect(result.current.pendingCliProxyEnablePrompt).toEqual({
        cliKey: "codex",
        conflicts: [
          {
            var_name: "OPENAI_API_KEY",
            source_type: "system",
            source_path: "Process Environment",
          },
        ],
      })
    );
    expect(cliProxyState.setCliProxyEnabled).not.toHaveBeenCalled();
    expect(result.current.cliProxyEnableBusy).toBe(true);

    act(() => {
      result.current.confirmPendingCliProxyEnable();
    });

    expect(cliProxyState.setCliProxyEnabled).toHaveBeenCalledWith("codex", true);
    expect(result.current.pendingCliProxyEnablePrompt).toBeNull();
    expect(result.current.cliProxyEnableBusy).toBe(false);

    act(() => {
      result.current.requestCliProxyEnabledSwitch("gemini", true);
    });
    expect(envConflictsCheck).toHaveBeenCalledTimes(2);
    expect(envConflictsCheck).toHaveBeenLastCalledWith("gemini");
    await waitFor(() =>
      expect(cliProxyState.setCliProxyEnabled).toHaveBeenCalledWith("gemini", true)
    );
  });

  it("logs and still enables the cli when env conflict checks fail", async () => {
    const cliProxyState = makeCliProxyState();
    vi.mocked(useCliProxy).mockReturnValue(cliProxyState as any);
    vi.mocked(envConflictsCheck).mockRejectedValueOnce(new Error("env check boom"));

    const { result } = renderHook(() => useCliProxyControls());

    act(() => {
      result.current.requestCliProxyEnabledSwitch("codex", true);
    });

    await waitFor(() =>
      expect(cliProxyState.setCliProxyEnabled).toHaveBeenCalledWith("codex", true)
    );
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "检查环境变量冲突失败，仍尝试开启 CLI 代理",
      expect.objectContaining({ cli: "codex", error: "Error: env check boom" })
    );
    expect(result.current.cliProxyEnableBusy).toBe(false);
  });
});
