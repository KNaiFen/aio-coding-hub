import { describe, expect, it } from "vitest";
import { isTrayProviderMiniWindow } from "../windowMode";

describe("tray window mode", () => {
  it("selects only the explicit tray provider mini renderer", () => {
    expect(isTrayProviderMiniWindow("?window=tray-provider-mini")).toBe(true);
    expect(isTrayProviderMiniWindow("?window=tray-provider-mini&preview=1")).toBe(true);
    expect(isTrayProviderMiniWindow("?window=main")).toBe(false);
    expect(isTrayProviderMiniWindow("")).toBe(false);
  });
});
