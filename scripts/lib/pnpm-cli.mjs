import { isAbsolute as isPosixAbsolute, basename as posixBasename } from "node:path/posix";
import { isAbsolute as isWindowsAbsolute, basename as windowsBasename } from "node:path/win32";

const CHECK_SCRIPTS = new Set([
  "format:check",
  "lint",
  "typecheck",
  "check:local-native-boundary",
  "check:no-instant-now-sub",
  "check:spec-links",
  "check:support-matrix",
  "check:homebrew-cask",
  "check:gateway-error-codes",
  "check:plugin-system-docs",
  "check:plugin-api-contract",
  "plugin-sdk:typecheck",
  "plugin-sdk:test",
  "create-aio-plugin:test",
  "test:unit:coverage:shards",
]);
const COVERAGE_THRESHOLDS_DISABLED = [
  "--coverage.thresholds.statements=0",
  "--coverage.thresholds.branches=0",
  "--coverage.thresholds.functions=0",
  "--coverage.thresholds.lines=0",
];
const COVERAGE_SHARD_COUNT = 4;

function resolvePnpmInvocation(
  args,
  {
    platform = process.platform,
    execPath = process.execPath,
    npmExecPath = process.env.npm_execpath,
  } = {}
) {
  if (!Array.isArray(args) || args.some((arg) => typeof arg !== "string" || arg.includes("\0"))) {
    throw new Error("pnpm arguments must be NUL-free strings.");
  }

  const windows = platform === "win32";
  const isAbsolute = windows ? isWindowsAbsolute : isPosixAbsolute;
  const basename = windows ? windowsBasename : posixBasename;
  if (typeof execPath !== "string" || !isAbsolute(execPath)) {
    throw new Error("Node executable path must be absolute.");
  }
  if (typeof npmExecPath !== "string" || !isAbsolute(npmExecPath)) {
    throw new Error("Run this check through pnpm so its absolute CLI path is available.");
  }

  const cliBasename = basename(npmExecPath);
  if (/^pnpm\.(?:c?js|mjs)$/i.test(cliBasename)) {
    return { command: execPath, args: [npmExecPath, ...args] };
  }
  if ((!windows && cliBasename === "pnpm") || (windows && /^pnpm\.exe$/i.test(cliBasename))) {
    return { command: npmExecPath, args };
  }
  throw new Error("pnpm CLI must be an absolute JavaScript entry or standalone executable.");
}

export function createCheckScriptInvocation(script, options = {}) {
  if (!CHECK_SCRIPTS.has(script)) throw new Error(`Unsupported local check script: ${script}`);
  return resolvePnpmInvocation(["run", script], options);
}

export function createAuditListInvocation(options = {}) {
  return resolvePnpmInvocation(
    ["list", "--recursive", "--prod", "--depth", "Infinity", "--json"],
    options
  );
}

export function createCoverageShardInvocation(shard, options = {}) {
  if (!Number.isInteger(shard) || shard < 1 || shard > COVERAGE_SHARD_COUNT) {
    throw new Error("Coverage shard is outside the fixed four-shard plan.");
  }
  return resolvePnpmInvocation(
    [
      "exec",
      "vitest",
      "run",
      "--reporter=blob",
      "--coverage",
      ...COVERAGE_THRESHOLDS_DISABLED,
      `--shard=${shard}/${COVERAGE_SHARD_COUNT}`,
    ],
    options
  );
}

export function createCoverageMergeInvocation(options = {}) {
  return resolvePnpmInvocation(["exec", "vitest", "run", "--merge-reports", "--coverage"], options);
}
