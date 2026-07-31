/**
 * Single source of truth for aggregate check stages.
 *
 * Usage:
 *   node scripts/run-checks.mjs <stage>
 *   node scripts/run-checks.mjs --list
 *
 * Adding a check: define its command in CHECKS, then add its id to the
 * stages that should run it. These stages are intentionally local-only and
 * may execute Node, TypeScript, frontend tests, or frontend builds only.
 */
import { spawnSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const pnpmCommand = process.platform === "win32" ? "pnpm.cmd" : "pnpm";

export const CHECKS = {
  "format-check": "format:check",
  lint: "lint",
  typecheck: "typecheck",
  "local-native-boundary": "check:local-native-boundary",
  "no-instant-now-sub": "check:no-instant-now-sub",
  "spec-links": "check:spec-links",
  "support-matrix": "check:support-matrix",
  "homebrew-cask": "check:homebrew-cask",
  "gateway-error-codes": "check:gateway-error-codes",
  "plugin-system-docs": "check:plugin-system-docs",
  "plugin-api-contract": "check:plugin-api-contract",
  "plugin-sdk-typecheck": "plugin-sdk:typecheck",
  "plugin-sdk-test": "plugin-sdk:test",
  "create-aio-plugin-test": "create-aio-plugin:test",
  "unit-coverage-shards": "test:unit:coverage:shards",
};

const PRECOMMIT_LOCAL = ["local-native-boundary", "lint", "typecheck", "no-instant-now-sub"];
const PREPUSH_STATIC = [
  "local-native-boundary",
  "lint",
  "typecheck",
  "spec-links",
  "support-matrix",
  "homebrew-cask",
  "gateway-error-codes",
  "plugin-system-docs",
  "plugin-api-contract",
  "plugin-sdk-typecheck",
];

export const STAGES = {
  precommit: PRECOMMIT_LOCAL,
  "precommit-full": [
    "format-check",
    ...PRECOMMIT_LOCAL,
    "spec-links",
    "support-matrix",
    "homebrew-cask",
    "gateway-error-codes",
  ],
  prepush: [...PREPUSH_STATIC, "unit-coverage-shards", "plugin-sdk-test", "create-aio-plugin-test"],
  "plugin-hardening": [
    "local-native-boundary",
    "plugin-api-contract",
    "plugin-sdk-test",
    "plugin-sdk-typecheck",
  ],
};

function listStages() {
  for (const [stage, ids] of Object.entries(STAGES)) {
    console.log(`${stage}:`);
    for (const id of ids) {
      console.log(`  ${id}: pnpm ${CHECKS[id]}`);
    }
  }
}

function main() {
  const arg = process.argv[2];
  if (arg === "--list") {
    listStages();
    return;
  }

  const ids = STAGES[arg];
  if (!ids) {
    console.error(`[checks] unknown stage: ${arg ?? "<none>"}`);
    console.error(`[checks] available stages: ${Object.keys(STAGES).join(", ")}`);
    process.exit(1);
  }

  for (const [index, id] of ids.entries()) {
    const script = CHECKS[id];
    console.log(`[checks] (${index + 1}/${ids.length}) ${id}: pnpm ${script}`);
    const result = spawnSync(pnpmCommand, [script], {
      cwd: repoRoot,
      stdio: "inherit",
    });
    if (result.status !== 0) {
      console.error(`[checks] ${id} failed (exit ${result.status ?? "signal"})`);
      process.exit(result.status ?? 1);
    }
  }
  console.log(`[checks] stage "${arg}" passed (${ids.length} checks)`);
}

const invokedPath = process.argv[1] ? pathToFileURL(resolve(process.argv[1])).href : null;
if (invokedPath === import.meta.url) main();
