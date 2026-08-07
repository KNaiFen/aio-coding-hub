import { describe, expect, it } from "vitest";
import { mergeDiscoveredModelIds, validateProviderModelPolicy } from "../providerModelPolicy";

describe("providerModelPolicy", () => {
  it("merges discovered models into the range without creating mappings", () => {
    const policy = {
      version: 1 as const,
      mode: "all" as const,
      modelPatterns: [],
      mappings: [{ source: "gpt-*", target: "upstream-*" }],
    };

    expect(mergeDiscoveredModelIds(policy, ["gpt-5.4", "claude-3", " claude-3 "])).toEqual({
      capacityExceeded: false,
      addedCount: 1,
      policy: {
        ...policy,
        modelPatterns: ["claude-3"],
      },
    });
  });

  it("does not change an excluded draft during discovery", () => {
    const policy = {
      version: 1 as const,
      mode: "excluded" as const,
      modelPatterns: ["legacy-model"],
      mappings: [],
    };
    expect(mergeDiscoveredModelIds(policy, ["new-model"])).toEqual({
      capacityExceeded: false,
      addedCount: 0,
      policy,
    });
  });

  it("keeps the draft unchanged when discovery would exceed capacity", () => {
    const policy = {
      version: 1 as const,
      mode: "selected" as const,
      modelPatterns: Array.from({ length: 500 }, (_, index) => `model-${index}`),
      mappings: [],
    };

    expect(mergeDiscoveredModelIds(policy, ["new-model"])).toEqual({
      capacityExceeded: true,
      addedCount: 0,
      policy,
    });
  });

  it("requires mapping targets and accepts a mapping as selected-model support", () => {
    expect(
      validateProviderModelPolicy({
        version: 1,
        mode: "selected",
        modelPatterns: [],
        mappings: [{ source: "gpt-5.6-luna", target: "deepseek-v4-flash" }],
      })
    ).toBeNull();
    expect(
      validateProviderModelPolicy({
        version: 1,
        mode: "all",
        modelPatterns: [],
        mappings: [{ source: "gpt-5.6-luna", target: "" }],
      })
    ).toBe("上游模型不能为空");
  });
});
