import { describe, expect, it } from "vitest";
import { mergeDiscoveredModelIds } from "../providerModelPolicy";

describe("mergeDiscoveredModelIds", () => {
  it("adds uncovered models in both modes without changing existing targets", () => {
    const result = mergeDiscoveredModelIds(
      {
        version: 1,
        mode: "all",
        rules: [{ source: "gpt-*", target: "upstream-*" }],
      },
      ["gpt-5.4", "claude-3", " claude-3 "]
    );

    expect(result).toEqual({
      capacityExceeded: false,
      addedCount: 1,
      policy: {
        version: 1,
        mode: "all",
        rules: [
          { source: "gpt-*", target: "upstream-*" },
          { source: "claude-3", target: null },
        ],
      },
    });
  });

  it("keeps the draft unchanged when discovery would exceed capacity", () => {
    const rules = Array.from({ length: 500 }, (_, index) => ({
      source: `model-${index}`,
      target: null,
    }));
    const policy = { version: 1 as const, mode: "selected" as const, rules };

    expect(mergeDiscoveredModelIds(policy, ["new-model"])).toEqual({
      capacityExceeded: true,
      addedCount: 0,
      policy,
    });
  });
});
