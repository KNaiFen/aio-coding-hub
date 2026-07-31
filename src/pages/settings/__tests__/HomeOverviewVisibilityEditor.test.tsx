import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { HOME_OVERVIEW_VISIBILITY_STORAGE_KEY } from "../../../services/home/homeOverviewVisibility";
import { HomeOverviewVisibilityEditor } from "../HomeOverviewVisibilityEditor";

afterEach(() => {
  window.localStorage.clear();
});

describe("pages/settings/HomeOverviewVisibilityEditor", () => {
  it("stores hidden information tabs locally", () => {
    render(<HomeOverviewVisibilityEditor kind="tabs" />);

    fireEvent.click(screen.getByRole("switch", { name: "首页信息面板：熔断信息" }));

    expect(window.localStorage.getItem(HOME_OVERVIEW_VISIBILITY_STORAGE_KEY)).toBe(
      JSON.stringify({
        version: 1,
        hiddenTabs: ["circuit"],
        hiddenCliKeys: [],
      })
    );
  });

  it("stores hidden CLI buttons locally and protects the final visible option", () => {
    render(<HomeOverviewVisibilityEditor kind="clis" />);

    for (const name of ["Claude", "Codex", "Gemini"]) {
      fireEvent.click(screen.getByRole("switch", { name: `配置信息中显示的 CLI：${name}` }));
    }

    const grok = screen.getByRole("switch", { name: "配置信息中显示的 CLI：Grok" });
    expect(grok).toBeDisabled();
    expect(window.localStorage.getItem(HOME_OVERVIEW_VISIBILITY_STORAGE_KEY)).toBe(
      JSON.stringify({
        version: 1,
        hiddenTabs: [],
        hiddenCliKeys: ["claude", "codex", "gemini"],
      })
    );
  });
});
