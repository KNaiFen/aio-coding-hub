import { describe, expect, it } from "vitest";
import contract from "../../../docs/plugins/plugin-api-v1-contract.json";
import { pluginEffectiveDataAccess } from "../pluginCapabilities";
import type { PluginManifest } from "../plugins";

function manifest(hooks: string[], capabilities = ["gateway.hooks"]): PluginManifest {
  return {
    id: "community.policy",
    name: "Policy",
    version: "1.0.0",
    apiVersion: "1.0.0",
    runtime: { kind: "extensionHost", language: "typescript" },
    main: "extension.cjs",
    hostCompatibility: { app: "*", pluginApi: "*" },
    capabilities,
    contributes: { gatewayHooks: hooks.map((name) => ({ name })) },
  };
}

describe("public plugin data access display contract", () => {
  it.each(contract.activeHooks)("derives %s from the shared hook permissions", (name) => {
    const matrix = contract.hookMatrix[name as keyof typeof contract.hookMatrix];
    expect(matrix).toBeDefined();
    expect(pluginEffectiveDataAccess(manifest([name]))).toEqual(
      [...new Set([...matrix.readPermissions, ...matrix.writePermissions])].sort()
    );
  });
  it("only exposes metadata and complete response access for beforeCommit", () => {
    expect(pluginEffectiveDataAccess(manifest(["gateway.response.beforeCommit"]))).toEqual([
      "request.meta.read",
      "response.body.read",
      "response.header.read",
    ]);
    expect(pluginEffectiveDataAccess(manifest(["gateway.response.beforeCommit"], []))).toEqual([]);
  });
});
