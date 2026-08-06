/**
 * Single source of truth for GitHub-Actions-only aggregate check stages.
 *
 * Usage:
 *   node scripts/run-checks.mjs <stage>
 *   node scripts/run-checks.mjs --list
 *
 * Adding a check: define its command in CHECKS, then add its id to the
 * stages that should run it. Do not invoke this file locally: every
 * dependency-backed package script is guarded for GitHub Actions.
 */
import { spawnSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const modulePath = fileURLToPath(import.meta.url);
const repoRoot = resolve(dirname(modulePath), "..");

export const CHECKS = {
  "format-check": "pnpm format:check",
  "cloud-only-verification": "node scripts/check-cloud-only-verification.mjs",
  lint: "pnpm lint",
  typecheck: "pnpm typecheck",
  "no-instant-now-sub": "pnpm check:no-instant-now-sub",
  "ci-quality-gates": "pnpm check:ci-quality-gates",
  "spec-links": "pnpm check:spec-links",
  "homebrew-cask": "pnpm check:homebrew-cask",
  "tui-release-contract": "pnpm check:tui-release-contract",
  "gateway-error-codes": "pnpm check:gateway-error-codes",
  "plugin-system-docs": "pnpm check:plugin-system-docs",
  "plugin-api-contract": "pnpm check:plugin-api-contract",
  "plugin-sdk-typecheck": "pnpm plugin-sdk:typecheck",
  "plugin-sdk-test": "pnpm plugin-sdk:test",
  "create-aio-plugin-test": "pnpm create-aio-plugin:test",
  "create-aio-plugin-typecheck": "pnpm create-aio-plugin:typecheck",
  "unit-coverage-shards": "pnpm test:unit:coverage:shards",
};

const FRONTEND_STATIC = ["cloud-only-verification", "lint", "typecheck", "no-instant-now-sub"];
const FULL_CI_STATIC = [
  "cloud-only-verification",
  "lint",
  "typecheck",
  "no-instant-now-sub",
  "ci-quality-gates",
  "homebrew-cask",
  "tui-release-contract",
  "gateway-error-codes",
  "plugin-system-docs",
  "plugin-api-contract",
  "plugin-sdk-typecheck",
  "create-aio-plugin-typecheck",
];

export const STAGES = {
  "frontend-static": FRONTEND_STATIC,
  "frontend-full": [
    "format-check",
    ...FRONTEND_STATIC,
    "spec-links",
    "homebrew-cask",
    "tui-release-contract",
    "gateway-error-codes",
  ],
  "full-ci": [...FULL_CI_STATIC, "unit-coverage-shards", "plugin-sdk-test", "create-aio-plugin-test"],
  "plugin-hardening": [
    "cloud-only-verification",
    "plugin-api-contract",
    "plugin-sdk-test",
    "plugin-sdk-typecheck",
    "create-aio-plugin-typecheck",
  ],
};

function listStages() {
  for (const [stage, ids] of Object.entries(STAGES)) {
    console.log(`${stage}:`);
    for (const id of ids) {
      console.log(`  ${id}: ${CHECKS[id]}`);
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
    const command = CHECKS[id];
    console.log(`[checks] (${index + 1}/${ids.length}) ${id}: ${command}`);
    const result = spawnSync(command, {
      cwd: repoRoot,
      stdio: "inherit",
      shell: true,
    });
    if (result.status !== 0) {
      console.error(`[checks] ${id} failed (exit ${result.status ?? "signal"})`);
      process.exit(result.status ?? 1);
    }
  }
  console.log(`[checks] stage "${arg}" passed (${ids.length} checks)`);
}

if (process.argv[1] && resolve(process.argv[1]) === modulePath) {
  main();
}
