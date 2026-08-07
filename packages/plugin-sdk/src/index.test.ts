import { describe, expect, it, test } from "vitest";
import contract from "../../../docs/plugins/plugin-api-v1-contract.json";
import {
  type ActivationEvent,
  type GatewayHookName,
  type JsonValue,
  type PluginCapability,
  type PluginContributes,
  type PluginExtensionExecutionReport,
  type PluginHookContext,
  type PluginHookResult,
  type PluginApi,
  type PluginManifest,
  type PluginPermission,
  type PluginRuntime,
  type UiContributionSlot,
  permissionRisk,
  validateManifest,
} from "./index";

const openRouterManifest: PluginManifest = {
  id: "acme.openrouter",
  name: "OpenRouter Provider",
  version: "0.1.0",
  apiVersion: "1.0.0",
  main: "dist/extension.js",
  runtime: { kind: "extensionHost", language: "typescript" },
  activationEvents: [],
  contributes: {
    providers: [
      {
        providerType: "openrouter",
        displayName: "OpenRouter",
        targetCliKeys: ["claude", "codex"],
        extensionNamespace: "openrouter",
      },
    ],
    ui: {
      "providers.editor.sections": [
        {
          id: "openrouter-routing",
          title: "OpenRouter 路由",
          order: 100,
          schema: {
            type: "section",
            fields: [
              { type: "text", key: "route", label: "Route" },
              { type: "boolean", key: "fallbackEnabled", label: "启用模型兜底" },
            ],
          },
        },
      ],
    },
    commands: [
      {
        command: "acme.openrouter.refreshModels",
        title: "刷新 OpenRouter 模型",
        category: "Provider",
      },
    ],
  },
  capabilities: ["provider.extensionValues", "commands.execute"],
  hostCompatibility: {
    app: ">=0.62.0 <1.0.0",
    pluginApi: "^1.0.0",
    platforms: ["macos", "windows", "linux"],
  },
};

describe("validateManifest", () => {
  test("keeps plugin API contribution types representable", () => {
    const gatewayHook: GatewayHookName = "gateway.request.afterBodyRead";
    const permission: PluginPermission = "request.body.read";
    const activationEvent: ActivationEvent = "onCommand:acme.openrouter.refreshModels";
    const capability: PluginCapability = "provider.extensionValues";
    const privacyCapability: PluginCapability = "privacy.redact";
    const slot: UiContributionSlot = "providers.editor.sections";

    expect(permissionRisk(permission)).toBe("high");

    const manifest: PluginManifest = {
      id: "acme.openrouter",
      name: "OpenRouter Provider",
      version: "0.1.0",
      apiVersion: "1.0.0",
      main: "dist/extension.js",
      runtime: { kind: "extensionHost", language: "typescript" },
      activationEvents: [
        "onStartup",
        activationEvent,
        "onGatewayHook:gateway.request.afterBodyRead",
      ],
      contributes: {
        providers: [
          {
            providerType: "openrouter",
            displayName: "OpenRouter",
            targetCliKeys: ["claude", "codex"],
            extensionNamespace: "openrouter",
          },
        ],
        commands: [
          {
            command: "acme.openrouter.refreshModels",
            title: "Refresh OpenRouter models",
            category: "Provider",
          },
        ],
        gatewayHooks: [{ name: gatewayHook, priority: 10 }],
        protocolBridges: [
          {
            bridgeType: "acme.openrouter.openai-gemini",
            inboundProtocol: "openai.chat",
            outboundProtocol: "gemini.generateContent",
            supportsStreaming: true,
          },
        ],
        ui: {
          [slot]: [
            {
              id: "openrouter-routing",
              title: "OpenRouter routing",
              schema: {
                type: "section",
                fields: [{ type: "text", key: "route", label: "Route" }],
              },
            },
          ],
          "settings.sections": [
            {
              id: "openrouter-refresh",
              title: "OpenRouter refresh",
              schema: {
                type: "panel",
                fields: [
                  {
                    type: "button",
                    key: "refresh",
                    label: "Refresh",
                    command: "acme.openrouter.refreshModels",
                  },
                ],
              },
            },
          ],
        },
      },
      capabilities: [
        capability,
        privacyCapability,
        "commands.execute",
        "gateway.hooks",
        "protocol.bridge",
      ],
      hostCompatibility: { app: ">=0.62.0 <1.0.0", pluginApi: "^1.0.0" },
    };

    const runtime: PluginRuntime = manifest.runtime;
    const contributes: PluginContributes = manifest.contributes ?? {};
    const replaceRequestResult: PluginHookResult = {
      action: "replace",
      requestBody: '{"messages":[]}',
    };
    const replaceResponseHeadersResult: PluginHookResult = {
      action: "replace",
      headers: { "x-plugin-redacted": "1" },
      responseBody: '{"ok":true}',
    };
    const passResult: PluginHookResult = { action: "pass" };
    const pluginApi: PluginApi = {
      commands: {
        registerCommand: (_command, _handler) => undefined,
      },
      gateway: {
        registerHook: (_name, _handler) => undefined,
      },
      privacy: {
        redactText: (text) => ({ hit: true, count: 1, redacted: text }),
        redactRequestBody: (body) => ({ hit: false, count: 0, redacted: body }),
      },
    };

    expect(runtime.kind).toBe("extensionHost");
    expect(contributes.commands?.[0]?.command).toBe("acme.openrouter.refreshModels");
    expect(contributes.ui?.[slot]?.[0]?.schema.type).toBe("section");
    expect(contributes.providers?.[0]?.extensionNamespace).toBe("openrouter");
    expect(contributes.gatewayHooks?.[0]?.name).toBe(gatewayHook);
    expect(contributes.protocolBridges?.[0]?.bridgeType).toBe("acme.openrouter.openai-gemini");
    expect(validateManifest(manifest)).toEqual({ ok: true });
    expect(replaceRequestResult.action).toBe("replace");
    expect(replaceResponseHeadersResult.headers?.["x-plugin-redacted"]).toBe("1");
    expect(passResult.action).toBe("pass");
    expect(pluginApi.privacy?.redactText("secret").count).toBe(1);
  });

  test("validates extension host provider manifest", () => {
    expect(validateManifest(openRouterManifest)).toEqual({ ok: true });
  });

  test("rejects wasm as unsupported public runtime", () => {
    const manifest = {
      ...openRouterManifest,
      runtime: { kind: "wasm", abiVersion: "1.0.0" },
      hooks: [{ name: "gateway.request.afterBodyRead" }],
      permissions: ["request.body.read"],
    } as unknown as PluginManifest;

    expect(validateManifest(manifest)).toEqual({
      ok: false,
      error: {
        code: "PLUGIN_UNSUPPORTED_RUNTIME",
        message: "community plugins must use extensionHost runtime",
      },
    });
  });

  test("rejects unknown contribution fields", () => {
    const manifest = {
      ...openRouterManifest,
      contributes: {
        legacyRules: [{ rules: ["rules/main.json"] }],
      },
    };

    expect(validateManifest(manifest as PluginManifest)).toEqual({
      ok: false,
      error: {
        code: "PLUGIN_INVALID_CONTRIBUTION",
        message: "unsupported contribution field: legacyRules",
      },
    });
  });

  test("rejects extension host manifest without main", () => {
    const manifest = { ...openRouterManifest, main: undefined };
    expect(validateManifest(manifest as PluginManifest)).toEqual({
      ok: false,
      error: {
        code: "PLUGIN_MISSING_MAIN",
        message: "extensionHost runtime requires main",
      },
    });
  });

  test("rejects unknown UI contribution slot", () => {
    const manifest = {
      ...openRouterManifest,
      contributes: {
        ui: {
          "providers.editor.unknown": [],
        },
      },
    };
    expect(validateManifest(manifest as PluginManifest).ok).toBe(false);
  });

  test("validates protocol bridge manifest", () => {
    const manifest: PluginManifest = {
      id: "acme.bridge",
      name: "Claude OpenAI Gemini Bridge",
      version: "0.1.0",
      apiVersion: "1.0.0",
      main: "dist/extension.js",
      runtime: { kind: "extensionHost", language: "typescript" },
      contributes: {
        protocols: [
          { protocolId: "openai.chat", direction: "both" },
          { protocolId: "gemini.generateContent", direction: "both" },
        ],
        protocolBridges: [
          {
            bridgeType: "acme.bridge.openai-gemini",
            inboundProtocol: "openai.chat",
            outboundProtocol: "gemini.generateContent",
            supportsStreaming: true,
          },
        ],
      },
      capabilities: ["protocol.bridge"],
      hostCompatibility: { app: ">=0.62.0 <1.0.0", pluginApi: "^1.0.0" },
    };

    expect(validateManifest(manifest)).toEqual({ ok: true });
  });

  test("rejects non-namespaced protocol bridge contribution", () => {
    const manifest: PluginManifest = {
      id: "acme.bridge",
      name: "Claude OpenAI Gemini Bridge",
      version: "0.1.0",
      apiVersion: "1.0.0",
      main: "dist/extension.js",
      runtime: { kind: "extensionHost", language: "typescript" },
      contributes: {
        protocolBridges: [
          {
            bridgeType: "openai-gemini",
            inboundProtocol: "openai.chat",
            outboundProtocol: "gemini.generateContent",
          },
        ],
      },
      capabilities: ["protocol.bridge"],
      hostCompatibility: { app: ">=0.62.0 <1.0.0", pluginApi: "^1.0.0" },
    };

    expect(validateManifest(manifest)).toMatchObject({
      ok: false,
      error: { code: "PLUGIN_INVALID_PROTOCOL_BRIDGE_CONTRIBUTION" },
    });
  });

  test("rejects invalid protocol bridge contribution id", () => {
    const manifest: PluginManifest = {
      id: "acme.bridge",
      name: "Claude OpenAI Gemini Bridge",
      version: "0.1.0",
      apiVersion: "1.0.0",
      main: "dist/extension.js",
      runtime: { kind: "extensionHost", language: "typescript" },
      contributes: {
        protocolBridges: [
          {
            bridgeType: "acme.bridge.OpenAI",
            inboundProtocol: "openai.chat",
            outboundProtocol: "gemini.generateContent",
          },
        ],
      },
      capabilities: ["protocol.bridge"],
      hostCompatibility: { app: ">=0.62.0 <1.0.0", pluginApi: "^1.0.0" },
    };

    expect(validateManifest(manifest)).toMatchObject({
      ok: false,
      error: { code: "PLUGIN_INVALID_PROTOCOL_BRIDGE_CONTRIBUTION" },
    });
  });

  test("rejects malformed provider contribution", () => {
    const manifest = {
      ...openRouterManifest,
      contributes: {
        providers: [
          {
            providerType: "",
            displayName: "OpenRouter",
            targetCliKeys: ["claude", "openai"],
            extensionNamespace: "openrouter",
          },
        ],
      },
    };

    expect(validateManifest(manifest as PluginManifest)).toMatchObject({
      ok: false,
      error: { code: "PLUGIN_INVALID_PROVIDER_CONTRIBUTION" },
    });
  });

  test("rejects non-object contributes", () => {
    const manifest = {
      ...openRouterManifest,
      contributes: [],
    };

    expect(validateManifest(manifest as unknown as PluginManifest)).toMatchObject({
      ok: false,
      error: { code: "PLUGIN_INVALID_CONTRIBUTES" },
    });
  });

  test("rejects malformed protocol bridge contribution", () => {
    const manifest = {
      ...openRouterManifest,
      contributes: {
        protocolBridges: [
          {
            bridgeType: "acme.bridge.openai-gemini",
            inboundProtocol: "openai.chat",
            outboundProtocol: "gemini.generateContent",
            supportsStreaming: "yes",
          },
        ],
      },
    };

    expect(validateManifest(manifest as unknown as PluginManifest)).toMatchObject({
      ok: false,
      error: { code: "PLUGIN_INVALID_PROTOCOL_BRIDGE_CONTRIBUTION" },
    });
  });

  test("rejects invalid UI field schema", () => {
    const manifest = {
      ...openRouterManifest,
      contributes: {
        ui: {
          "providers.editor.sections": [
            {
              id: "openrouter-routing",
              schema: {
                type: "section",
                fields: [{ type: "button", key: "refresh", label: "Refresh" }],
              },
            },
          ],
        },
      },
    };

    expect(validateManifest(manifest as PluginManifest)).toMatchObject({
      ok: false,
      error: { code: "PLUGIN_INVALID_UI_CONTRIBUTION" },
    });
  });

  test("requires explicit activation events to match command and gateway hook contributions", () => {
    const explicitManifest: PluginManifest = {
      ...openRouterManifest,
      activationEvents: ["onStartup", "onCommand:acme.openrouter.refreshModels"],
    };
    expect(validateManifest(explicitManifest)).toEqual({ ok: true });

    const missingCommandEvent = {
      ...explicitManifest,
      activationEvents: ["onStartup"],
    };
    expect(validateManifest(missingCommandEvent as PluginManifest)).toMatchObject({
      ok: false,
      error: { code: "PLUGIN_INVALID_ACTIVATION_EVENT" },
    });
  });

  test("rejects deprecated, blank, padded, and unknown activation events", () => {
    for (const event of [
      "onProviderEditor:openrouter",
      "onProtocolBridge:acme.openrouter.openai-gemini",
      "onCommand:",
      "onCommand: acme.openrouter.refreshModels",
      "onGatewayHook:gateway.request.afterBodyRead ",
      "onUnknown:thing",
    ]) {
      const manifest = {
        ...openRouterManifest,
        activationEvents: [event],
      };

      expect(validateManifest(manifest as unknown as PluginManifest)).toMatchObject({
        ok: false,
        error: { code: "PLUGIN_INVALID_ACTIVATION_EVENT" },
      });
    }
  });

  test("keeps missing and empty activation events as legacy on-demand manifests", () => {
    const missing = { ...openRouterManifest, activationEvents: undefined };

    expect(validateManifest(missing as PluginManifest)).toEqual({ ok: true });
    expect(validateManifest(openRouterManifest)).toEqual({ ok: true });
  });

  test("keeps the documented activation event families synchronized", () => {
    expect(contract.activationEvents).toEqual([
      "onStartup",
      "onCommand:<command>",
      "onGatewayHook:<hook>",
    ]);
  });

  test("validates gatewayHooks manifest", () => {
    const manifest: PluginManifest = {
      ...openRouterManifest,
      contributes: {
        gatewayHooks: [
          { name: "gateway.request.afterBodyRead", priority: 10, failurePolicy: "fail-open" },
        ],
      },
      capabilities: ["gateway.hooks"],
    };

    expect(validateManifest(manifest)).toEqual({ ok: true });
  });

  test("validates privacy redaction capability for extension host plugins", () => {
    const manifest: PluginManifest = {
      ...openRouterManifest,
      contributes: {
        gatewayHooks: [
          {
            name: "gateway.request.afterBodyRead",
            priority: 5,
            failurePolicy: "fail-closed",
            timeoutMs: 5000,
          },
          { name: "log.beforePersist", priority: 1, failurePolicy: "fail-closed" },
        ],
      },
      capabilities: ["gateway.hooks", "privacy.redact"],
    };

    expect(validateManifest(manifest)).toEqual({ ok: true });
  });

  test("rejects gatewayHooks with reserved or unknown hook", () => {
    const reservedHookManifest = {
      ...openRouterManifest,
      contributes: {
        gatewayHooks: [{ name: "gateway.request.received" }],
      },
      capabilities: ["gateway.hooks"],
    };
    expect(validateManifest(reservedHookManifest as PluginManifest)).toMatchObject({
      ok: false,
      error: { code: "PLUGIN_RESERVED_HOOK" },
    });

    const unknownHookManifest = {
      ...openRouterManifest,
      contributes: {
        gatewayHooks: [{ name: "gateway.request.missing" }],
      },
      capabilities: ["gateway.hooks"],
    };
    expect(validateManifest(unknownHookManifest as unknown as PluginManifest)).toMatchObject({
      ok: false,
      error: { code: "PLUGIN_UNKNOWN_HOOK" },
    });
  });

  test("rejects gatewayHooks with invalid timeoutMs", () => {
    const manifest = {
      ...openRouterManifest,
      contributes: {
        gatewayHooks: [{ name: "gateway.request.afterBodyRead", timeoutMs: 0 }],
      },
      capabilities: ["gateway.hooks"],
    };

    expect(validateManifest(manifest as unknown as PluginManifest)).toMatchObject({
      ok: false,
      error: { code: "PLUGIN_INVALID_HOOK_TIMEOUT" },
    });
  });

  test("rejects gatewayHooks with invalid failurePolicy", () => {
    for (const failurePolicy of ["fail-close", "unexpected", 42]) {
      const manifest = {
        ...openRouterManifest,
        contributes: {
          gatewayHooks: [{ name: "gateway.request.afterBodyRead", failurePolicy }],
        },
        capabilities: ["gateway.hooks"],
      };

      expect(validateManifest(manifest as unknown as PluginManifest)).toMatchObject({
        ok: false,
        error: { code: "PLUGIN_INVALID_FAILURE_POLICY" },
      });
    }
  });

  test("rejects top-level legacy hooks and permissions", () => {
    expect(
      validateManifest({
        ...openRouterManifest,
        hooks: [{ name: "gateway.request.afterBodyRead" }],
      } as unknown as PluginManifest)
    ).toMatchObject({
      ok: false,
      error: { code: "PLUGIN_INVALID_MANIFEST" },
    });

    expect(
      validateManifest({
        ...openRouterManifest,
        permissions: ["request.body.read"],
      } as unknown as PluginManifest)
    ).toMatchObject({
      ok: false,
      error: { code: "PLUGIN_INVALID_MANIFEST" },
    });
  });

  it("rejects every reserved hook from the contract", () => {
    for (const hook of contract.reservedHooks) {
      const result = validateManifest({
        ...openRouterManifest,
        contributes: { gatewayHooks: [{ name: hook as never }] },
        capabilities: ["gateway.hooks"],
      } as PluginManifest);

      expect(result).toMatchObject({
        ok: false,
        error: { code: "PLUGIN_RESERVED_HOOK" },
      });
    }
  });

  test("rejects contributions without required capabilities", () => {
    const cases: Array<{ manifest: PluginManifest; message: string }> = [
      {
        manifest: {
          ...openRouterManifest,
          contributes: {
            commands: [{ command: "acme.openrouter.refreshModels", title: "Refresh models" }],
          },
          capabilities: [],
        },
        message: "commands contribution requires commands.execute",
      },
      {
        manifest: {
          ...openRouterManifest,
          contributes: {
            providers: [
              {
                providerType: "openrouter",
                displayName: "OpenRouter",
                targetCliKeys: ["claude", "codex"],
                extensionNamespace: "openrouter",
              },
            ],
          },
          capabilities: [],
        },
        message: "provider contribution requires provider.extensionValues",
      },
      {
        manifest: {
          ...openRouterManifest,
          contributes: {
            gatewayHooks: [{ name: "gateway.request.afterBodyRead" }],
          },
          capabilities: [],
        },
        message: "gatewayHooks contribution requires gateway.hooks",
      },
      {
        manifest: {
          ...openRouterManifest,
          contributes: {
            protocolBridges: [
              {
                bridgeType: "acme.openrouter.openai-gemini",
                inboundProtocol: "openai.chat",
                outboundProtocol: "gemini.generateContent",
              },
            ],
          },
          capabilities: [],
        },
        message: "protocolBridges contribution requires protocol.bridge",
      },
      {
        manifest: {
          ...openRouterManifest,
          contributes: {
            ui: {
              "providers.editor.sections": [
                {
                  id: "openrouter-routing",
                  schema: {
                    type: "section",
                    fields: [{ type: "text", key: "route", label: "Route" }],
                  },
                },
              ],
            },
          },
          capabilities: [],
        },
        message: "providers.editor.sections UI contribution requires provider.extensionValues",
      },
      {
        manifest: {
          ...openRouterManifest,
          contributes: {
            ui: {
              "providers.editor.fields": [
                {
                  id: "openrouter-models",
                  schema: {
                    type: "section",
                    fields: [
                      {
                        type: "select",
                        key: "model",
                        label: "Model",
                        options: [{ value: "auto", label: "Auto" }],
                      },
                    ],
                  },
                },
              ],
            },
          },
          capabilities: [],
        },
        message: "providers.editor.fields UI contribution requires provider.extensionValues",
      },
      {
        manifest: {
          ...openRouterManifest,
          contributes: {
            ui: {
              "settings.sections": [
                {
                  id: "openrouter-refresh",
                  schema: {
                    type: "section",
                    fields: [
                      {
                        type: "button",
                        key: "refresh",
                        label: "Refresh",
                        command: "acme.openrouter.refreshModels",
                      },
                    ],
                  },
                },
              ],
            },
          },
          capabilities: [],
        },
        message: "UI command field requires commands.execute",
      },
    ];

    for (const { manifest, message } of cases) {
      expect(validateManifest(manifest)).toEqual({
        ok: false,
        error: {
          code: "PLUGIN_MISSING_CAPABILITY",
          message,
        },
      });
    }
  });

  it("rejects manifests without a supported host compatibility range", () => {
    expect(
      validateManifest({
        ...openRouterManifest,
        hostCompatibility: { app: "", pluginApi: "^1.0.0" },
      })
    ).toMatchObject({
      ok: false,
      error: { code: "PLUGIN_INVALID_HOST_COMPATIBILITY" },
    });

    expect(
      validateManifest({
        ...openRouterManifest,
        hostCompatibility: { app: ">=0.62.0 <1.0.0", pluginApi: "^2.0.0" },
      })
    ).toMatchObject({
      ok: false,
      error: { code: "PLUGIN_UNSUPPORTED_PLUGIN_API" },
    });
  });

  it("rejects future manifest apiVersion majors even when hostCompatibility supports v1", () => {
    const result = validateManifest({
      ...openRouterManifest,
      apiVersion: "2.0.0",
      hostCompatibility: { app: ">=0.62.0 <1.0.0", pluginApi: "^1.0.0" },
    });

    expect(result).toMatchObject({
      ok: false,
      error: { code: "PLUGIN_INCOMPATIBLE_API" },
    });
  });

  it("rejects wasm as an unsupported public runtime", () => {
    const result = validateManifest({
      ...openRouterManifest,
      runtime: { kind: "wasm", abiVersion: "2.0.0" },
    } as unknown as PluginManifest);

    expect(result).toMatchObject({
      ok: false,
      error: { code: "PLUGIN_UNSUPPORTED_RUNTIME" },
    });
  });
});

describe("PluginHookResult", () => {
  it("represents host mutation fields without legacy contextPatch", () => {
    const result: PluginHookResult = {
      action: "replace",
      requestBody: '{"messages":[]}',
      responseBody: '{"ok":true}',
      headers: { "x-plugin-redacted": "1" },
    };

    expect(result).toEqual({
      action: "replace",
      requestBody: '{"messages":[]}',
      responseBody: '{"ok":true}',
      headers: { "x-plugin-redacted": "1" },
    });
    expect("contextPatch" in result).toBe(false);
  });
});

describe("PluginHookContext", () => {
  it("types provider-neutral normalized request messages", () => {
    const context: PluginHookContext = {
      hook: "gateway.request.afterBodyRead",
      traceId: "trace-sdk",
      config: {},
      context: {
        request: {
          bodyTruncated: false,
          normalizedMessages: [
            {
              role: "user",
              text: "hello from codex",
              source: "openai.responses.input_text",
            },
          ],
          normalizedMessagesTruncated: false,
        },
        response: { bodyTruncated: true },
        stream: { chunkTruncated: true },
        log: { messageTruncated: true },
      },
    };

    expect(context.context.request?.normalizedMessages?.[0]?.text).toBe("hello from codex");
    expect(context.context.request?.bodyTruncated).toBe(false);
    expect(context.context.request?.normalizedMessagesTruncated).toBe(false);
    expect(context.context.response?.bodyTruncated).toBe(true);
    expect(context.context.stream?.chunkTruncated).toBe(true);
    expect(context.context.log?.messageTruncated).toBe(true);
  });
});

describe("PluginApi", () => {
  it("represents every active Extension Host API namespace", () => {
    const runtimeReport: PluginExtensionExecutionReport = {
      id: 1,
      pluginId: "acme.openrouter",
      contributionType: "command",
      contributionId: "acme.openrouter.refreshModels",
      commandOrHook: "acme.openrouter.refreshModels",
      traceId: "trace-sdk",
      status: "completed",
      startedAtMs: 100,
      durationMs: 5,
      failureKind: null,
      errorCode: null,
      inputBudget: { bytes: 10 },
      outputBudget: { bytes: 20 },
      mutationSummary: {},
      replayable: false,
      createdAt: 1,
    };
    const storage = new Map<string, JsonValue>();
    const api: PluginApi = {
      commands: {
        registerCommand: (_command, _handler) => undefined,
      },
      gateway: {
        registerHook: (_name, _handler) => undefined,
      },
      privacy: {
        redactText: (text) => ({ hit: true, count: 1, redacted: text.replace("secret", "[密钥]") }),
        redactRequestBody: (body) => ({ hit: false, count: 0, redacted: body }),
      },
      storage: {
        get: (key) => storage.get(key) ?? null,
        set: (key, value) => {
          storage.set(key, value);
        },
      },
      diagnostics: {
        getRuntimeReports: (limit) => (limit === 1 ? [runtimeReport] : []),
      },
    };

    api.storage?.set("lastReport", { id: runtimeReport.id });
    expect(api.commands).toBeDefined();
    expect(api.gateway).toBeDefined();
    expect(api.privacy?.redactText("secret").redacted).toBe("[密钥]");
    expect(api.storage?.get("lastReport")).toEqual({ id: 1 });
    expect(api.diagnostics?.getRuntimeReports(1)[0]?.pluginId).toBe("acme.openrouter");
  });
});

describe("permissionRisk", () => {
  it("keeps permissionRisk defined for every v1 permission", () => {
    for (const permission of [...contract.activePermissions, ...contract.reservedPermissions]) {
      expect(permissionRisk(permission as never)).toMatch(/^(low|medium|high|critical)$/);
    }
  });

  it("matches the host permission risk table", () => {
    expect(permissionRisk("response.header.read")).toBe("low");
    expect(permissionRisk("response.header.write")).toBe("medium");
    expect(permissionRisk("file.read")).toBe("high");
    expect(permissionRisk("file.write")).toBe("high");
    expect(permissionRisk("secret.read")).toBe("critical");
  });
});
