import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { dirname, posix, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const defaultRepoRoot = resolve(scriptDir, "..");

const FORBIDDEN_PATHS = new Set([
  "scripts/install-git-hooks.mjs",
  "scripts/check-generated-bindings.mjs",
  "scripts/tauri-build.mjs",
  "scripts/tauri-dev.mjs",
  "scripts/tauri-test.mjs",
  "scripts/tauri-gen-types.mjs",
]);

const ALLOWED_HELPER_PATHS = new Set([
  "scripts/check-gateway-error-codes.mjs",
  "scripts/check-local-native-boundary.mjs",
  "scripts/check-local-native-boundary.selftest.mjs",
  "scripts/check-no-instant-now-sub.mjs",
  "scripts/check-plugin-api-contract.mjs",
  "scripts/check-plugin-system-completion.mjs",
  "scripts/check-plugin-system-docs.mjs",
  "scripts/check-pnpm-audit.mjs",
  "scripts/check-pnpm-audit.selftest.mjs",
  "scripts/check-spec-links.mjs",
  "scripts/run-checks.mjs",
  "scripts/run-coverage-shards.mjs",
  "scripts/support-matrix.mjs",
  "scripts/support-matrix.homebrew-cask.selftest.mjs",
  "packages/create-aio-plugin/scripts/run-cli.mjs",
]);

const FORBIDDEN_SCRIPT_NAMES = new Set([
  "postinstall",
  "hooks:install",
  "plugin:perf-smoke",
  "check:generated-bindings",
  "check:precommit:tauri",
  "tauri",
]);

const ALLOWED_SCRIPT_EXECUTABLES = new Set([
  "eslint",
  "node",
  "npm",
  "pnpm",
  "prettier",
  "tsc",
  "tsx",
  "vite",
  "vitest",
]);

const NATIVE_COMMAND_PATTERNS = [
  ["Cargo", /(?:^|[\s;&|()"'/\\])cargo(?:\.exe)?(?=$|[\s;&|()"'/\\])/i],
  ["rustc", /(?:^|[\s;&|()"'/\\])rustc(?:\.exe)?(?=$|[\s;&|()"'/\\])/i],
  ["rustfmt", /(?:^|[\s;&|()"'/\\])rustfmt(?:\.exe)?(?=$|[\s;&|()"'/\\])/i],
  ["Clippy", /(?:^|[\s;&|()"'/\\])(?:clippy|clippy-driver)(?:\.exe)?(?=$|[\s;&|()"'/\\])/i],
  ["rustup", /(?:^|[\s;&|()"'/\\])rustup(?:\.exe)?(?=$|[\s;&|()"'/\\])/i],
  ["cross", /(?:^|[\s;&|()"'/\\])cross(?:\.exe)?(?=$|[\s;&|()"'/\\])/i],
  ["wasm-pack", /(?:^|[\s;&|()"'/\\])wasm-pack(?:\.exe)?(?=$|[\s;&|()"'/\\])/i],
  ["Specta generation", /(?:^|[\s;&|()"'/\\])(?:specta|export-bindings)(?=$|[\s;&|()"'/\\])/i],
  ["Tauri CLI", /(?:^|[\s;&|()"'/\\])tauri(?:\.exe)?(?=$|[\s;&|()"'/\\])/i],
  ["Tauri package alias", /\b(?:pnpm|yarn|bun)\s+(?:run\s+)?tauri(?::|\s|$)/i],
  ["Tauri npm alias", /\bnpm\s+run\s+tauri(?::|\s|$)/i],
  [
    "native helper",
    /scripts\/(?:tauri-(?:build|dev|test|gen-types)|check-generated-bindings)\.mjs/i,
  ],
];

const HOOK_CONFIGURATION_PATTERN =
  /(?:install-git-hooks|core\.hooksPath|\.githooks|husky\s+install|simple-git-hooks|lefthook\s+install)/i;
const INLINE_NODE_PATTERN = /(?:^|[\s;&|()])node(?:\.exe)?\s+(?:-e|--eval)(?=$|\s)/i;
const SHELL_HELPER_PATTERN = /(?:^|[\s;&|()])(?:bash|sh|pwsh|powershell)(?:\.exe)?\s+/i;
const AUTOMATION_PATH_PATTERN =
  /^(?:\.vscode\/tasks\.json|\.fleet\/|\.idea\/|Makefile$|Justfile$|justfile$|Taskfile\.(?:yml|yaml)$)/;

function findNativeCommands(command) {
  return NATIVE_COMMAND_PATTERNS.filter(([, pattern]) => pattern.test(command)).map(
    ([label]) => label
  );
}

function runtimeHelperInvocations(command) {
  const invocations = [];
  const helperPattern = /(?:^|[\s;&|()])(?:node|tsx)\s+["']?([^"'\s;&|()]+)["']?/gi;
  for (const match of command.matchAll(helperPattern)) {
    invocations.push(match[1]);
  }
  return invocations;
}

function referencedHelpers(manifestPath, command) {
  const helpers = [];
  for (const candidate of runtimeHelperInvocations(command)) {
    if (!/\.(?:[cm]?js|ts)$/.test(candidate)) continue;
    const normalized = posix.normalize(posix.join(posix.dirname(manifestPath), candidate));
    helpers.push(normalized);
  }
  return helpers;
}

function referencedExecutables(command) {
  const executables = [];
  for (const rawSegment of command.split(/&&|\|\||[;|]/)) {
    const tokens = rawSegment.trim().split(/\s+/).filter(Boolean);
    const executable = tokens.find((token) => !/^[A-Za-z_][A-Za-z0-9_]*=/.test(token));
    if (executable) executables.push(executable.replace(/^['"]|['"]$/g, ""));
  }
  return executables;
}

function hasActiveTrellisHooks(contents) {
  return contents.split(/\r?\n/).some((line) => {
    const withoutComment = line.replace(/\s+#.*$/, "");
    return /^hooks\s*:/.test(withoutComment);
  });
}

export function evaluateLocalNativeBoundary(snapshot) {
  const violations = [...(snapshot.collectionErrors ?? [])];
  const trackedPaths = [...(snapshot.trackedPaths ?? [])];

  for (const path of trackedPaths) {
    if (FORBIDDEN_PATHS.has(path)) violations.push(`${path}: forbidden local native helper`);
    if (path.startsWith(".githooks/") || path.startsWith(".husky/")) {
      violations.push(`${path}: tracked repository hook is forbidden`);
    }
  }

  for (const hooksPath of snapshot.hooksPaths ?? []) {
    violations.push(
      `git config core.hooksPath: repository-local override is forbidden (${JSON.stringify(hooksPath)})`
    );
  }

  for (const manifest of snapshot.manifests ?? []) {
    if (manifest.error) {
      violations.push(`${manifest.path}: ${manifest.error}`);
      continue;
    }
    if (manifest.scripts === undefined) continue;
    if (
      manifest.scripts === null ||
      typeof manifest.scripts !== "object" ||
      Array.isArray(manifest.scripts)
    ) {
      violations.push(`${manifest.path}: scripts must be an object`);
      continue;
    }

    for (const [name, command] of Object.entries(manifest.scripts)) {
      const owner = `${manifest.path} script ${name}`;
      if (FORBIDDEN_SCRIPT_NAMES.has(name) || name.startsWith("tauri:")) {
        violations.push(`${owner}: forbidden local native entry point`);
      }
      if (typeof command !== "string") {
        violations.push(`${owner}: command must be a string`);
        continue;
      }
      for (const nativeKind of findNativeCommands(command)) {
        violations.push(`${owner}: invokes ${nativeKind}`);
      }
      if (HOOK_CONFIGURATION_PATTERN.test(command)) {
        violations.push(`${owner}: configures repository hooks`);
      }
      if (INLINE_NODE_PATTERN.test(command)) {
        violations.push(`${owner}: inline Node execution is not an approved local helper`);
      }
      if (SHELL_HELPER_PATTERN.test(command)) {
        violations.push(`${owner}: shell helper execution is not approved`);
      }
      for (const target of runtimeHelperInvocations(command)) {
        if (!/\.(?:[cm]?js|ts)$/.test(target)) {
          violations.push(`${owner}: Node/tsx must directly invoke an approved helper file`);
        }
      }
      for (const executable of referencedExecutables(command)) {
        if (!ALLOWED_SCRIPT_EXECUTABLES.has(executable)) {
          violations.push(
            `${owner}: executable ${executable} is not in the Node/frontend allowlist`
          );
        }
      }
      for (const helper of referencedHelpers(manifest.path, command)) {
        if (!ALLOWED_HELPER_PATHS.has(helper)) {
          violations.push(`${owner}: helper ${helper} is not in the Node/frontend allowlist`);
        }
      }
    }
  }

  const aggregate = snapshot.files?.["scripts/run-checks.mjs"];
  if (aggregate !== undefined) {
    for (const nativeKind of findNativeCommands(aggregate)) {
      violations.push(`scripts/run-checks.mjs: invokes ${nativeKind}`);
    }
    if (HOOK_CONFIGURATION_PATTERN.test(aggregate)) {
      violations.push("scripts/run-checks.mjs: configures repository hooks");
    }
  }

  const trellisConfig = snapshot.files?.[".trellis/config.yaml"];
  if (trellisConfig !== undefined && hasActiveTrellisHooks(trellisConfig)) {
    violations.push(".trellis/config.yaml: active lifecycle hooks are forbidden");
  }

  for (const path of trackedPaths.filter((candidate) => AUTOMATION_PATH_PATTERN.test(candidate))) {
    const contents = snapshot.files?.[path] ?? "";
    for (const nativeKind of findNativeCommands(contents)) {
      violations.push(`${path}: local automation invokes ${nativeKind}`);
    }
    if (HOOK_CONFIGURATION_PATTERN.test(contents)) {
      violations.push(`${path}: local automation configures repository hooks`);
    }
  }

  return [...new Set(violations)].sort();
}

function readManifest(repoRoot, path) {
  try {
    const value = JSON.parse(readFileSync(resolve(repoRoot, path), "utf8"));
    return { path, scripts: value.scripts };
  } catch (error) {
    return { path, error: `cannot be read as JSON: ${error.message}` };
  }
}

function collectGitPaths(repoRoot, collectionErrors) {
  try {
    const output = execFileSync(
      "git",
      ["ls-files", "--cached", "--others", "--exclude-standard", "-z"],
      { cwd: repoRoot, encoding: "utf8", maxBuffer: 16 * 1024 * 1024 }
    );
    return output
      .split("\0")
      .filter(Boolean)
      .filter((path) => existsSync(resolve(repoRoot, path)));
  } catch (error) {
    collectionErrors.push(`git ls-files: ${error.message}`);
    return [];
  }
}

function collectHooksPaths(repoRoot, collectionErrors) {
  try {
    const output = execFileSync("git", ["config", "--local", "--get-all", "core.hooksPath"], {
      cwd: repoRoot,
      encoding: "utf8",
    });
    return output.split(/\r?\n/).filter(Boolean);
  } catch (error) {
    if (error.status === 1) return [];
    collectionErrors.push(`git config core.hooksPath: ${error.message}`);
    return [];
  }
}

export function collectLocalNativeBoundarySnapshot(repoRoot = defaultRepoRoot) {
  const root = resolve(repoRoot);
  const collectionErrors = [];
  const trackedPaths = collectGitPaths(root, collectionErrors);
  const manifests = trackedPaths
    .filter((path) => path === "package.json" || path.endsWith("/package.json"))
    .map((path) => readManifest(root, path));
  const files = {};

  for (const path of trackedPaths) {
    if (
      path === "scripts/run-checks.mjs" ||
      path === ".trellis/config.yaml" ||
      AUTOMATION_PATH_PATTERN.test(path)
    ) {
      try {
        files[path] = readFileSync(resolve(root, path), "utf8");
      } catch (error) {
        collectionErrors.push(`${path}: cannot be read: ${error.message}`);
      }
    }
  }

  return {
    collectionErrors,
    trackedPaths,
    manifests,
    hooksPaths: collectHooksPaths(root, collectionErrors),
    files,
  };
}

export function checkLocalNativeBoundary(repoRoot = defaultRepoRoot) {
  return evaluateLocalNativeBoundary(collectLocalNativeBoundarySnapshot(repoRoot));
}

function main() {
  const violations = checkLocalNativeBoundary();
  if (violations.length > 0) {
    console.error("Local native-build boundary check failed:");
    for (const violation of violations) console.error(`- ${violation}`);
    process.exit(1);
  }
  console.error(
    "[local-native-boundary] repository-controlled local checks are Node/frontend-only."
  );
}

const invokedPath = process.argv[1] ? pathToFileURL(resolve(process.argv[1])).href : null;
if (invokedPath === import.meta.url) main();
