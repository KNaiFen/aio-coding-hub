// Display permissions use the same public contract checked against the Rust owner.
import contract from "../../docs/plugins/plugin-api-v1-contract.json";
import type { PluginManifest } from "./plugins";

export function pluginEffectiveDataAccess(manifest: PluginManifest): string[] {
  if (
    manifest.runtime.kind !== "extensionHost" ||
    !manifest.capabilities?.includes("gateway.hooks")
  ) {
    return [];
  }
  const permissions = new Set<string>();
  for (const hook of manifest.contributes?.gatewayHooks ?? []) {
    const matrix = contract.hookMatrix[hook.name as keyof typeof contract.hookMatrix];
    if (!matrix) continue;
    for (const permission of [...matrix.readPermissions, ...matrix.writePermissions]) {
      permissions.add(permission);
    }
  }
  return [...permissions].sort();
}
