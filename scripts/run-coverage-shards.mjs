/**
 * Run the unit test suite as sequential vitest shards with blob reports,
 * then merge the reports so the coverage thresholds gate the combined run.
 * Per-shard thresholds are disabled because they only make sense globally.
 */
import { spawnSync } from "node:child_process";
import { rmSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import { createPnpmInvocation } from "./lib/pnpm-cli.mjs";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const SHARD_COUNT = 4;
const NO_THRESHOLDS = [
  "--coverage.thresholds.statements=0",
  "--coverage.thresholds.branches=0",
  "--coverage.thresholds.functions=0",
  "--coverage.thresholds.lines=0",
];

function runPnpm(args) {
  const invocation = createPnpmInvocation(args);
  const result = spawnSync(invocation.command, invocation.args, {
    cwd: repoRoot,
    stdio: "inherit",
    shell: false,
  });
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

function main() {
  rmSync(resolve(repoRoot, ".vitest-reports"), { recursive: true, force: true });

  for (let shard = 1; shard <= SHARD_COUNT; shard += 1) {
    console.log(`[coverage-shards] shard ${shard}/${SHARD_COUNT}`);
    runPnpm([
      "exec",
      "vitest",
      "run",
      "--reporter=blob",
      "--coverage",
      ...NO_THRESHOLDS,
      `--shard=${shard}/${SHARD_COUNT}`,
    ]);
  }

  console.log("[coverage-shards] merging reports and applying coverage thresholds");
  runPnpm(["exec", "vitest", "run", "--merge-reports", "--coverage"]);
}

const invokedPath = process.argv[1] ? pathToFileURL(resolve(process.argv[1])).href : null;
if (invokedPath === import.meta.url) main();
