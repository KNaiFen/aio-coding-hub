import type { PluginDetail, PluginInstallPreview } from "../../services/plugins";

export function responseCommitPlugin(pluginId = "community.tail-policy"): PluginDetail {
  return {
    summary: {
      id: 99,
      plugin_id: pluginId,
      name: "Community Response Policy",
      current_version: "0.1.0",
      status: "disabled",
      runtime: "extensionHost",
      permission_risk: "high",
      update_available: false,
      last_error: null,
      created_at: 1,
      updated_at: 1,
    },
    manifest: {
      id: pluginId,
      name: "Community Response Policy",
      version: "0.1.0",
      apiVersion: "1.0.0",
      runtime: { kind: "extensionHost", language: "typescript" },
      main: "extension.cjs",
      hostCompatibility: { app: "*", pluginApi: "*" },
      capabilities: ["gateway.hooks", "gateway.provider.switch"],
      contributes: {
        gatewayHooks: [
          {
            name: "gateway.response.beforeCommit",
            priority: 100,
            failurePolicy: "fail-closed",
            match: {
              cliKeys: ["codex"],
              methods: ["POST"],
              paths: ["/responses", "/v1/responses"],
            },
          },
        ],
      },
    },
    install_source: "local",
    installed_dir: null,
    config: {},
    granted_permissions: [],
    pending_permissions: [],
    audit_logs: [],
    runtime_failures: [],
    rollback_versions: [],
  };
}

export function responseCommitPreview(detail = responseCommitPlugin()): PluginInstallPreview {
  return {
    pluginId: detail.summary.plugin_id,
    name: detail.summary.name,
    version: "0.1.0",
    source: "local",
    description: null,
    author: null,
    homepage: null,
    repository: null,
    license: null,
    category: null,
    runtime: {
      kind: "extensionHost",
      label: "Extension Host",
      supported: true,
      blockingReasons: [],
    },
    hooks: [
      {
        name: "gateway.response.beforeCommit",
        priority: 100,
        failurePolicy: "fail-closed",
        timeoutMs: 5000,
        match: { cliKeys: ["codex"], methods: ["POST"], paths: ["/responses", "/v1/responses"] },
      },
    ],
    permissions: ["request.meta.read", "response.body.read", "response.header.read"].map(
      (permission) => ({ permission, risk: "high", granted: false, pending: true })
    ),
    contributionImpact: {
      providers: [],
      protocols: [],
      protocolBridges: [],
      uiSlots: [],
      commands: [],
      gateway: [],
      capabilities: detail.manifest.capabilities ?? [],
    },
    compatibility: {
      compatible: true,
      hostVersion: "0.60.18",
      appRange: "*",
      pluginApiRange: "*",
      platforms: [],
      blockingReasons: [],
    },
    trust: {
      checksum: "test",
      expectedChecksum: null,
      checksumVerified: false,
      signatureVerified: false,
      unsigned: true,
      developerMode: false,
    },
    existingStatus: null,
    existingVersion: null,
    blockingReasons: [],
    warnings: [],
  };
}
