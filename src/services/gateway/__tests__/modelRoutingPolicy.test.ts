import { describe, expect, it } from "vitest";
import {
  cloneModelRoutingPolicy,
  DEFAULT_MODEL_ROUTING_POLICY,
  emptyModelRoutingRule,
  normalizeModelRoutingPolicy,
  validateModelRoutingPolicy,
} from "../modelRoutingPolicy";

describe("modelRoutingPolicy", () => {
  it("clones policies without sharing mutable rules", () => {
    const cloned = cloneModelRoutingPolicy({
      enabled: true,
      rules: [
        {
          source_model: "fable5",
          source_reasoning_effort: null,
          target_model: "opus4.8",
          reasoning_effort: null,
        },
      ],
    });

    cloned.rules[0].target_model = "terra";

    expect(cloned).not.toEqual(DEFAULT_MODEL_ROUTING_POLICY);
    expect(DEFAULT_MODEL_ROUTING_POLICY).toEqual({ enabled: false, rules: [] });
    expect(emptyModelRoutingRule()).toEqual({
      source_model: "",
      source_reasoning_effort: null,
      target_model: null,
      reasoning_effort: null,
    });
  });

  it("trims exact-match fields and turns blank overrides into null", () => {
    expect(
      normalizeModelRoutingPolicy({
        enabled: true,
        rules: [
          {
            source_model: "  fable5  ",
            source_reasoning_effort: " HIGH ",
            target_model: " opus4.8 ",
            reasoning_effort: " LOW ",
          },
        ],
      })
    ).toEqual({
      enabled: true,
      rules: [
        {
          source_model: "fable5",
          source_reasoning_effort: "high",
          target_model: "opus4.8",
          reasoning_effort: "low",
        },
      ],
    });
  });

  it("accepts model-only and effort-only rules", () => {
    expect(
      validateModelRoutingPolicy({
        enabled: true,
        rules: [
          {
            source_model: "fable5",
            source_reasoning_effort: null,
            target_model: "opus4.8",
            reasoning_effort: null,
          },
          {
            source_model: "gpt-expensive",
            source_reasoning_effort: "high",
            target_model: null,
            reasoning_effort: "low",
          },
        ],
      })
    ).toBeNull();
  });

  it("rejects missing overrides, duplicate exact sources, unsafe text, and byte overflow", () => {
    expect(
      validateModelRoutingPolicy({
        enabled: true,
        rules: [
          {
            source_model: "fable5",
            source_reasoning_effort: null,
            target_model: null,
            reasoning_effort: null,
          },
        ],
      })
    ).toContain("至少填写目标模型或思考强度");

    expect(
      validateModelRoutingPolicy({
        enabled: true,
        rules: [
          {
            source_model: "fable5",
            source_reasoning_effort: "high",
            target_model: "opus",
            reasoning_effort: null,
          },
          {
            source_model: " fable5 ",
            source_reasoning_effort: " HIGH ",
            target_model: "terra",
            reasoning_effort: null,
          },
        ],
      })
    ).toContain("与已有来源模型及思考强度重复");

    expect(
      validateModelRoutingPolicy({
        enabled: true,
        rules: [
          {
            source_model: "fable5",
            source_reasoning_effort: "high",
            target_model: "opus",
            reasoning_effort: null,
          },
          {
            source_model: "fable5",
            source_reasoning_effort: null,
            target_model: "terra",
            reasoning_effort: null,
          },
        ],
      })
    ).toBeNull();

    expect(
      validateModelRoutingPolicy({
        enabled: true,
        rules: [
          {
            source_model: "fable5",
            source_reasoning_effort: "8192",
            target_model: "opus",
            reasoning_effort: null,
          },
        ],
      })
    ).toContain("来源思考强度不是受支持的标准值");

    expect(
      validateModelRoutingPolicy({
        enabled: true,
        rules: [
          {
            source_model: "fable5",
            source_reasoning_effort: null,
            target_model: "opus\n4.8",
            reasoning_effort: null,
          },
        ],
      })
    ).toContain("控制字符");

    expect(
      validateModelRoutingPolicy({
        enabled: true,
        rules: [
          {
            source_model: "模".repeat(86),
            source_reasoning_effort: null,
            target_model: "opus",
            reasoning_effort: null,
          },
        ],
      })
    ).toContain("不能超过 256 字节");
  });
});
