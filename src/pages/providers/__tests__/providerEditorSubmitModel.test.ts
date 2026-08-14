import { describe, expect, it } from "vitest";
import { DEFAULT_UPSTREAM_RETRY_POLICY } from "../../../services/gateway/upstreamRetryPolicy";
import { DEFAULT_MODEL_ROUTING_POLICY } from "../../../services/gateway/modelRoutingPolicy";
import { DEFAULT_FORM_VALUES, deriveCodexBridgeTarget } from "../providerEditorUtils";
import { buildProviderEditorUpsertInput } from "../providerEditorSubmitModel";
import type { ProviderEditorPayloadContext } from "../providerEditorActionContext";

function makeContext(
  overrides: Partial<ProviderEditorPayloadContext> = {}
): ProviderEditorPayloadContext {
  return {
    mode: "create",
    cliKey: "claude",
    editingProviderId: null,
    authMode: "api_key",
    codexBridgeTarget: "openai_chat",
    baseUrlMode: "order",
    baseUrlRows: [{ id: "1", url: "https://example.com/v1", ping: { status: "idle" } }],
    tags: [],
    claudeModels: {},
    modelMapping: { default_model: null, exact: {} },
    testModel: "",
    availabilityProbeEnabled: false,
    availabilityProbeIntervalMinutes: "10",
    streamIdleTimeoutSeconds: "",
    upstreamRetryPolicyOverrideEnabled: false,
    upstreamRetryPolicyDraft: DEFAULT_UPSTREAM_RETRY_POLICY,
    modelRoutingPolicyOverrideEnabled: false,
    modelRoutingPolicyDraft: DEFAULT_MODEL_ROUTING_POLICY,
    apiKeyConfigured: false,
    isCodexGatewaySource: false,
    sourceProviderId: null,
    selectedCx2ccSourceProvider: null,
    formValues: {
      ...DEFAULT_FORM_VALUES,
      name: "Provider A",
      api_key: "sk-test",
    },
    ...overrides,
  };
}

describe("pages/providers/providerEditorSubmitModel", () => {
  it("requires an api key when editing an api-key provider without a saved secret", () => {
    const result = buildProviderEditorUpsertInput(
      makeContext({
        mode: "edit",
        editingProviderId: 8,
        apiKeyConfigured: false,
        formValues: {
          ...DEFAULT_FORM_VALUES,
          name: "Provider A",
          api_key: "",
        },
      })
    );

    expect(result).toEqual({
      ok: false,
      error: {
        kind: "message",
        message: "请输入 API Key",
      },
    });
  });

  it("clears base urls and api key for oauth providers", () => {
    const result = buildProviderEditorUpsertInput(
      makeContext({
        authMode: "oauth",
        formValues: {
          ...DEFAULT_FORM_VALUES,
          name: "OAuth Provider",
          api_key: "",
          auth_mode: "oauth",
        },
      })
    );

    expect(result.ok).toBe(true);
    if (!result.ok) return;

    expect(result.value.payload.baseUrls).toEqual([]);
    expect(result.value.payload.apiKey).toBeNull();
    expect(result.value.payload.authMode).toBe("oauth");
  });

  it("forces cx2cc gateway sources to use zero cost and no source provider id", () => {
    const result = buildProviderEditorUpsertInput(
      makeContext({
        authMode: "cx2cc",
        isCodexGatewaySource: true,
        formValues: {
          ...DEFAULT_FORM_VALUES,
          name: "CX2CC Provider",
          api_key: "",
        },
      })
    );

    expect(result.ok).toBe(true);
    if (!result.ok) return;

    expect(result.value.payload.costMultiplier).toBe(0);
    expect(result.value.payload.bridgeType).toBe("cx2cc");
    expect(result.value.payload.sourceProviderId).toBeNull();
    expect(result.value.payload.authMode).toBe("api_key");
  });

  it("passes codex availability test model through the payload", () => {
    const result = buildProviderEditorUpsertInput(
      makeContext({
        cliKey: "codex",
        testModel: "gpt-5.4",
      })
    );

    expect(result.ok).toBe(true);
    if (!result.ok) return;

    expect(result.value.payload.availabilityTestModel).toBe("gpt-5.4");
  });

  it.each([
    ["1", 1],
    ["1440", 1440],
  ])("passes availability probe settings at the %s minute boundary", (raw, expected) => {
    const result = buildProviderEditorUpsertInput(
      makeContext({
        availabilityProbeEnabled: true,
        availabilityProbeIntervalMinutes: raw,
      })
    );

    expect(result.ok).toBe(true);
    if (!result.ok) return;

    expect(result.value.payload.availabilityProbeEnabled).toBe(true);
    expect(result.value.payload.availabilityProbeIntervalMinutes).toBe(expected);
  });

  it.each(["", "0", "1.5", "1441"]) (
    "rejects an invalid availability probe interval of %s",
    (availabilityProbeIntervalMinutes) => {
      const result = buildProviderEditorUpsertInput(
        makeContext({
          availabilityProbeEnabled: true,
          availabilityProbeIntervalMinutes,
        })
      );

      expect(result).toEqual({
        ok: false,
        error: {
          kind: "message",
          message: "定时可用性测试间隔必须为 1-1440 分钟",
        },
      });
    }
  );

  it("uses the default interval when scheduled probing is disabled", () => {
    const result = buildProviderEditorUpsertInput(
      makeContext({
        availabilityProbeEnabled: false,
        availabilityProbeIntervalMinutes: "invalid-hidden-value",
      })
    );

    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.payload.availabilityProbeEnabled).toBe(false);
    expect(result.value.payload.availabilityProbeIntervalMinutes).toBe(10);
  });

  it("normalizes a provider model-routing override and clears it when inheritance is selected", () => {
    const overridden = buildProviderEditorUpsertInput(
      makeContext({
        modelRoutingPolicyOverrideEnabled: true,
        modelRoutingPolicyDraft: {
          enabled: true,
          rules: [
            {
              source_model: " fable5 ",
              source_reasoning_effort: null,
              target_model: " opus4.8 ",
              reasoning_effort: " low ",
            },
          ],
        },
      })
    );
    expect(overridden.ok).toBe(true);
    if (!overridden.ok) return;
    expect(overridden.value.payload.modelRoutingPolicyOverride).toEqual({
      enabled: true,
      rules: [
        {
          source_model: "fable5",
          source_reasoning_effort: null,
          target_model: "opus4.8",
          reasoning_effort: "low",
        },
      ],
    });

    const inherited = buildProviderEditorUpsertInput(makeContext());
    expect(inherited.ok).toBe(true);
    if (!inherited.ok) return;
    expect(inherited.value.payload.modelRoutingPolicyOverride).toBeNull();
  });

  it("rejects an enabled provider model-routing override with an incomplete rule", () => {
    const result = buildProviderEditorUpsertInput(
      makeContext({
        modelRoutingPolicyOverrideEnabled: true,
        modelRoutingPolicyDraft: {
          enabled: true,
          rules: [
            {
              source_model: "fable5",
              source_reasoning_effort: null,
              target_model: null,
              reasoning_effort: null,
            },
          ],
        },
      })
    );

    expect(result).toEqual({
      ok: false,
      error: {
        kind: "message",
        message: "第 1 条模型路由至少填写目标模型或思考强度",
      },
    });
  });

  it("passes explicit NewAPI account credential preserve and clear semantics", () => {
    const preserve = buildProviderEditorUpsertInput(
      makeContext({
        accountUsageCredentials: {
          newApiUserId: "42",
          newApiAccessToken: null,
          clearNewApiAccessToken: false,
        },
      })
    );
    expect(preserve.ok).toBe(true);
    if (!preserve.ok) return;
    expect(preserve.value.payload.accountUsageCredentials).toEqual({
      newApiUserId: "42",
      newApiAccessToken: null,
      clearNewApiAccessToken: false,
    });

    const clear = buildProviderEditorUpsertInput(
      makeContext({
        accountUsageCredentials: {
          newApiUserId: null,
          newApiAccessToken: null,
          clearNewApiAccessToken: true,
        },
      })
    );
    expect(clear.ok).toBe(true);
    if (!clear.ok) return;
    expect(clear.value.payload.accountUsageCredentials).toEqual({
      newApiUserId: null,
      newApiAccessToken: null,
      clearNewApiAccessToken: true,
    });
  });

  it("builds codex chat-completions bridge payload", () => {
    const result = buildProviderEditorUpsertInput(
      makeContext({
        cliKey: "codex",
        authMode: "cx2cc",
        codexBridgeTarget: "openai_chat",
        sourceProviderId: 7,
        formValues: {
          ...DEFAULT_FORM_VALUES,
          name: "Codex Chat Bridge",
          api_key: "",
        },
      })
    );

    expect(result.ok).toBe(true);
    if (!result.ok) return;

    expect(result.value.payload.bridgeType).toBe("codex_to_openai_chat");
    expect(result.value.payload.sourceProviderId).toBe(7);
    expect(result.value.payload.baseUrls).toEqual([]);
    expect(result.value.payload.apiKey).toBeNull();
  });

  it("builds codex bridge model mapping payload", () => {
    const result = buildProviderEditorUpsertInput(
      makeContext({
        cliKey: "codex",
        authMode: "cx2cc",
        sourceProviderId: 7,
        modelMapping: {
          default_model: " deepseek-reasoner ",
          exact: {
            " gpt-5.5 ": " deepseek-chat ",
            "": "",
          },
        },
        formValues: {
          ...DEFAULT_FORM_VALUES,
          name: "Codex DeepSeek Bridge",
          api_key: "",
        },
      })
    );

    expect(result.ok).toBe(true);
    if (!result.ok) return;

    expect(result.value.payload.modelMapping).toEqual({
      default_model: "deepseek-reasoner",
      exact: {
        "gpt-5.5": "deepseek-chat",
      },
    });
  });

  it("builds codex responses bridge payload", () => {
    const result = buildProviderEditorUpsertInput(
      makeContext({
        cliKey: "codex",
        authMode: "cx2cc",
        codexBridgeTarget: "openai_responses",
        sourceProviderId: 9,
        formValues: {
          ...DEFAULT_FORM_VALUES,
          name: "Codex Responses Bridge",
          api_key: "",
        },
      })
    );

    expect(result.ok).toBe(true);
    if (!result.ok) return;

    expect(result.value.payload.bridgeType).toBe("codex_to_openai_responses");
    expect(result.value.payload.sourceProviderId).toBe(9);
  });

  it("maps legacy codex anthropic messages bridge edits to responses target", () => {
    expect(deriveCodexBridgeTarget({ bridge_type: "codex_to_anthropic_messages" })).toBe(
      "openai_responses"
    );
  });

  it("requires source provider for codex bridge payloads", () => {
    const result = buildProviderEditorUpsertInput(
      makeContext({
        cliKey: "codex",
        authMode: "cx2cc",
        formValues: {
          ...DEFAULT_FORM_VALUES,
          name: "Codex Bridge",
          api_key: "",
        },
      })
    );

    expect(result).toEqual({
      ok: false,
      error: {
        kind: "message",
        message: "请选择上游来源",
      },
    });
  });
});
