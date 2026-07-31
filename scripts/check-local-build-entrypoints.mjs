import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const packageJson = JSON.parse(readFileSync(join(repoRoot, "package.json"), "utf8"));
const failures = [];

const forbiddenScripts = new Set([
  "postinstall",
  "hooks:install",
  "plugin:perf-smoke",
  "check:generated-bindings",
  "check:precommit:tauri",
]);
const nativeCommand = /(?:^|[\s;&|])(?:cargo|rustc|rustfmt|clippy|tauri)(?=$|[\s;&|:])/i;

for (const [name, command] of Object.entries(packageJson.scripts ?? {})) {
  if (forbiddenScripts.has(name) || name === "tauri" || name.startsWith("tauri:")) {
    failures.push(`package.json script ${name} is a local native entry point`);
  }
  if (typeof command === "string" && nativeCommand.test(command)) {
    failures.push(`package.json script ${name} invokes a native tool`);
  }
}

for (const path of [
  ".githooks/pre-commit",
  ".githooks/pre-push",
  "scripts/install-git-hooks.mjs",
  "scripts/tauri-build.mjs",
  "scripts/tauri-dev.mjs",
  "scripts/tauri-gen-types.mjs",
  "scripts/tauri-test.mjs",
]) {
  if (existsSync(join(repoRoot, path))) failures.push(`${path} must remain removed`);
}

if (failures.length > 0) {
  console.error("Local native-build entry point check failed:");
  for (const failure of failures) console.error(`- ${failure}`);
  process.exit(1);
}

console.error("[local-build-entrypoints] no repository-managed local native build entry points");
