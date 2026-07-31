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
  "scripts/pnpm-cli.selftest.mjs",
  "scripts/check-spec-links.mjs",
  "scripts/run-checks.mjs",
  "scripts/run-coverage-shards.mjs",
  "scripts/support-matrix.mjs",
  "scripts/support-matrix.homebrew-cask.selftest.mjs",
  "packages/create-aio-plugin/scripts/run-cli.mjs",
]);

const FORBIDDEN_SCRIPT_NAMES = new Set([
  "preinstall",
  "install",
  "postinstall",
  "prepare",
  "prepublish",
  "prepublishOnly",
  "prepack",
  "postpack",
  "hooks:install",
  "plugin:perf-smoke",
  "check:generated-bindings",
  "check:precommit:tauri",
  "tauri",
]);
const FORBIDDEN_MANIFEST_CONFIG_KEYS = new Set(["husky", "simple-git-hooks", "lefthook"]);

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
const AUTOMATION_PATH_PATTERNS = [
  /(?:^|\/)\.vscode\/tasks\.json$/,
  /(?:^|\/)(?:\.fleet|\.idea)\//,
  /(?:^|\/)(?:GNUmakefile|Makefile|makefile|Justfile|justfile|\.justfile|Taskfile\.(?:yml|yaml))$/,
];
const REPOSITORY_HOOK_PATH_PATTERNS = [
  /(?:^|\/)(?:\.githooks|\.git-hooks|\.husky)\//,
  /(?:^|\/)\.pre-commit-config\.ya?ml$/i,
  /(?:^|\/)(?:\.?lefthook|lefthook-local)\.ya?ml$/i,
];
const PNPMFILE_PATH_PATTERN = /(?:^|\/)\.pnpmfile\.(?:cjs|mjs|js)$/i;
const PACKAGE_MANAGER_CONFIG_PATH_PATTERN = /(?:^|\/)(?:\.npmrc|pnpm-workspace\.ya?ml)$/i;
const PNPMFILE_CONFIG_PATTERN = /(?:^|\r?\n)\s*pnpmfile\s*[:=]/i;
const PNPM_BUILD_OVERRIDE_PATTERN =
  /(?:^|\r?\n)\s*(?:allowBuilds|allow-builds|onlyBuiltDependencies|only-built-dependencies|neverBuiltDependencies|never-built-dependencies|ignoredBuiltDependencies|ignored-built-dependencies|dangerouslyAllowAllBuilds|dangerously-allow-all-builds)\s*=/i;
const PNPM_WORKSPACE_PATH = "pnpm-workspace.yaml";
const EXPECTED_ALLOW_BUILDS = new Map([
  ["es5-ext", false],
  ["esbuild", true],
  ["msw", false],
]);
const EXPECTED_ONLY_BUILT_DEPENDENCIES = new Set(["esbuild"]);
const JAVASCRIPT_SOURCE_PATTERN = /\.(?:cjs|mjs|js|cts|mts|ts|tsx)$/;
const PROCESS_MODULE_PATTERN =
  /(?:node:)?child_process|child_["'`+\s]*process|process\.getBuiltinModule|Bun\.spawn|Deno\.Command/;
const PROCESS_CALL_PATTERN =
  /(?:^|[^\w.$])(spawnSync|spawn|execFileSync|execFile|execSync|exec|fork)\s*\(\s*([^,\r\n)]+)/gm;
const INDIRECT_PROCESS_CALL_PATTERN =
  /Reflect\.apply\s*\(\s*(?:(?:[A-Za-z_$][\w$]*)\.)?(?:spawnSync|spawn|execFileSync|execFile|execSync|exec|fork)|(?:spawnSync|spawn|execFileSync|execFile|execSync|exec|fork)\s*\.\s*(?:call|apply)\s*\(|\[\s*["'`](?:spawnSync|spawn|execFileSync|execFile|execSync|exec|fork)["'`]\s*\]\s*\(/m;
const PROCESS_EXECUTION_CONTRACTS = new Map([
  ["scripts/check-local-native-boundary.mjs", new Set(["git"])],
  ["scripts/check-plugin-api-contract.selftest.mjs", new Set(["process.execPath"])],
  ["scripts/check-plugin-system-docs.mjs", new Set(["git"])],
  ["scripts/check-pnpm-audit.mjs", new Set(["listCommand.command"])],
  ["scripts/cloud-native-drift.mjs", new Set(["git"])],
  ["scripts/cloud-native-drift.selftest.mjs", new Set(["git"])],
  ["scripts/run-checks.mjs", new Set(["invocation.command"])],
  ["scripts/run-coverage-shards.mjs", new Set(["invocation.command"])],
  ["scripts/support-matrix.homebrew-cask.selftest.mjs", new Set(["process.execPath"])],
  ["scripts/support-matrix.mjs", new Set(["git"])],
]);
const FRONTEND_COMMAND_PATTERNS = [
  /^vite(?: build| preview)?$/,
  /^tsc(?: -p tsconfig\.json(?: --noEmit)?)?$/,
  /^prettier --(?:write|check) \.$/,
  /^vitest watch$/,
  /^vitest run(?: --coverage| --shard=[1-9]\d*\/[1-9]\d*| src\/e2e)?$/,
  /^eslint src\/(?: --fix)?$/,
];
const SAFE_SCRIPT_TOKEN_PATTERN = /^[A-Za-z0-9@_./:=,+-]+$/;
const SHELL_META_PATTERN = /["'`$\\\r\n<>|;&]/;

function findNativeCommands(command) {
  return NATIVE_COMMAND_PATTERNS.filter(([, pattern]) => pattern.test(command)).map(
    ([label]) => label
  );
}

function isAutomationPath(path) {
  return AUTOMATION_PATH_PATTERNS.some((pattern) => pattern.test(path));
}

function isRepositoryHookPath(path) {
  return REPOSITORY_HOOK_PATH_PATTERNS.some((pattern) => pattern.test(path));
}

function normalizeProcessExpression(expression) {
  return expression.replace(/[\s"'`+]/g, "");
}

function processLaunches(contents) {
  return [...contents.matchAll(PROCESS_CALL_PATTERN)].map((match) => ({
    api: match[1],
    command: normalizeProcessExpression(match[2]),
  }));
}

function validateExecutableSource(path, contents, violations) {
  const launches = processLaunches(contents);
  const usesProcessCapability =
    PROCESS_MODULE_PATTERN.test(contents) ||
    launches.length > 0 ||
    INDIRECT_PROCESS_CALL_PATTERN.test(contents);
  if (!usesProcessCapability) return;

  const contract = PROCESS_EXECUTION_CONTRACTS.get(path);
  if (!contract) {
    violations.push(`${path}: process execution is not approved for a local source file`);
    return;
  }
  if (INDIRECT_PROCESS_CALL_PATTERN.test(contents)) {
    violations.push(`${path}: indirect process dispatch is forbidden`);
  }
  if (launches.length === 0) {
    violations.push(`${path}: process execution cannot be statically audited`);
    return;
  }
  for (const launch of launches) {
    if (!contract.has(launch.command)) {
      violations.push(
        `${path}: ${launch.api} command ${JSON.stringify(launch.command)} is outside its process contract`
      );
    }
  }
  if (/shell\s*:\s*true/.test(contents)) {
    violations.push(`${path}: shell-enabled process execution is forbidden`);
  }
}

function readTopLevelYamlSection(contents, sectionName) {
  const lines = contents.split(/\r?\n/);
  const starts = lines
    .map((line, index) =>
      /^([A-Za-z][A-Za-z0-9]*):\s*(?:#.*)?$/.exec(line)?.[1] === sectionName ? index : -1
    )
    .filter((index) => index >= 0);
  if (starts.length !== 1) return null;

  const section = [];
  for (const line of lines.slice(starts[0] + 1)) {
    if (/^[^\s#]/.test(line)) break;
    const withoutComment = line.replace(/\s+#.*$/, "");
    if (withoutComment.trim() !== "") section.push(withoutComment);
  }
  return section;
}

function validatePnpmWorkspaceBuildPolicy(contents, violations) {
  const allowBuildLines = readTopLevelYamlSection(contents, "allowBuilds");
  const onlyBuiltLines = readTopLevelYamlSection(contents, "onlyBuiltDependencies");
  if (allowBuildLines === null || onlyBuiltLines === null) {
    violations.push(`${PNPM_WORKSPACE_PATH}: exact dependency build policy is required`);
    return;
  }

  const allowBuilds = new Map();
  for (const line of allowBuildLines) {
    const match = /^  ([A-Za-z0-9@/_.+-]+):\s*(true|false)\s*$/.exec(line);
    if (!match || allowBuilds.has(match[1])) {
      violations.push(`${PNPM_WORKSPACE_PATH}: allowBuilds must use unique literal booleans`);
      return;
    }
    allowBuilds.set(match[1], match[2] === "true");
  }

  const onlyBuiltDependencies = new Set();
  for (const line of onlyBuiltLines) {
    const match = /^  - ([A-Za-z0-9@/_.+-]+)\s*$/.exec(line);
    if (!match || onlyBuiltDependencies.has(match[1])) {
      violations.push(
        `${PNPM_WORKSPACE_PATH}: onlyBuiltDependencies must be a unique literal list`
      );
      return;
    }
    onlyBuiltDependencies.add(match[1]);
  }

  const allowBuildsMatch =
    allowBuilds.size === EXPECTED_ALLOW_BUILDS.size &&
    [...EXPECTED_ALLOW_BUILDS].every(([name, allowed]) => allowBuilds.get(name) === allowed);
  const onlyBuiltMatch =
    onlyBuiltDependencies.size === EXPECTED_ONLY_BUILT_DEPENDENCIES.size &&
    [...EXPECTED_ONLY_BUILT_DEPENDENCIES].every((name) => onlyBuiltDependencies.has(name));
  if (!allowBuildsMatch || !onlyBuiltMatch) {
    violations.push(`${PNPM_WORKSPACE_PATH}: dependency build allowlist may only enable esbuild`);
  }
}

function scriptSegments(command) {
  const withoutAnd = command.replaceAll("&&", "");
  if (SHELL_META_PATTERN.test(withoutAnd)) return null;
  const segments = command.split(/\s*&&\s*/);
  if (segments.some((segment) => segment.trim() === "")) return null;
  return segments.map((segment) => segment.trim());
}

function validatePnpmSegment(tokens, manifest, packagesByName) {
  if (tokens.length === 2) {
    return Object.hasOwn(manifest.scripts, tokens[1]);
  }
  if (tokens.length === 4 && tokens[1] === "--filter") {
    const target = packagesByName.get(tokens[2]);
    return target != null && Object.hasOwn(target.scripts ?? {}, tokens[3]);
  }
  return false;
}

function validateScriptCommand(manifest, command, packagesByName) {
  const segments = scriptSegments(command);
  if (!segments) return false;

  for (const segment of segments) {
    const tokens = segment.split(/\s+/);
    if (tokens.some((token) => !SAFE_SCRIPT_TOKEN_PATTERN.test(token))) return false;
    if (FRONTEND_COMMAND_PATTERNS.some((pattern) => pattern.test(segment))) continue;

    if (tokens[0] === "node" || tokens[0] === "tsx") {
      if (tokens.length < 2 || !/\.(?:[cm]?js|ts)$/.test(tokens[1])) return false;
      const helper = posix.normalize(posix.join(posix.dirname(manifest.path), tokens[1]));
      if (!ALLOWED_HELPER_PATHS.has(helper)) return false;
      continue;
    }

    if (tokens[0] === "pnpm" && validatePnpmSegment(tokens, manifest, packagesByName)) {
      continue;
    }
    return false;
  }
  return true;
}

function hasActiveTrellisHooks(contents) {
  return contents.split(/\r?\n/).some((line) => {
    const withoutComment = line.replace(/\s+#.*$/, "");
    return /^\s*hooks\s*:/.test(withoutComment);
  });
}

export function evaluateLocalNativeBoundary(snapshot) {
  const violations = [...(snapshot.collectionErrors ?? [])];
  const trackedPaths = [...(snapshot.trackedPaths ?? [])];

  for (const path of trackedPaths) {
    if (FORBIDDEN_PATHS.has(path)) violations.push(`${path}: forbidden local native helper`);
    if (PNPMFILE_PATH_PATTERN.test(path)) {
      violations.push(`${path}: executable pnpm install hook is forbidden`);
    }
    if (isRepositoryHookPath(path)) {
      violations.push(`${path}: tracked repository hook is forbidden`);
    }
  }

  for (const hooksPath of snapshot.hooksPaths ?? []) {
    violations.push(
      `git config core.hooksPath: repository-local override is forbidden (${JSON.stringify(hooksPath)})`
    );
  }

  const packagesByName = new Map(
    (snapshot.manifests ?? [])
      .filter((manifest) => typeof manifest.name === "string" && manifest.name !== "")
      .map((manifest) => [manifest.name, manifest])
  );

  for (const manifest of snapshot.manifests ?? []) {
    if (manifest.error) {
      violations.push(`${manifest.path}: ${manifest.error}`);
      continue;
    }
    if (manifest.pnpm !== undefined) {
      violations.push(`${manifest.path}: package-level pnpm lifecycle policy is forbidden`);
    }
    for (const key of manifest.hookConfigKeys ?? []) {
      violations.push(`${manifest.path}: ${key} hook configuration is forbidden`);
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
      if (!validateScriptCommand(manifest, command, packagesByName)) {
        violations.push(`${owner}: command is outside the exact Node/frontend grammar`);
      }
    }
  }

  for (const [path, contents] of Object.entries(snapshot.files ?? {})) {
    if (JAVASCRIPT_SOURCE_PATTERN.test(path)) {
      validateExecutableSource(path, contents, violations);
    }
  }

  const trellisConfig = snapshot.files?.[".trellis/config.yaml"];
  if (trellisConfig !== undefined && hasActiveTrellisHooks(trellisConfig)) {
    violations.push(".trellis/config.yaml: active lifecycle hooks are forbidden");
  }

  for (const path of trackedPaths.filter((candidate) =>
    PACKAGE_MANAGER_CONFIG_PATH_PATTERN.test(candidate)
  )) {
    const contents = snapshot.files?.[path] ?? "";
    if (PNPMFILE_CONFIG_PATTERN.test(contents)) {
      violations.push(`${path}: custom pnpmfile install hook is forbidden`);
    }
    if (/(?:^|\/)\.npmrc$/i.test(path) && PNPM_BUILD_OVERRIDE_PATTERN.test(contents)) {
      violations.push(`${path}: pnpm dependency build policy override is forbidden`);
    }
  }

  const pnpmWorkspacePaths = trackedPaths.filter((path) =>
    /(?:^|\/)pnpm-workspace\.ya?ml$/i.test(path)
  );
  if (pnpmWorkspacePaths.length !== 1 || pnpmWorkspacePaths[0] !== PNPM_WORKSPACE_PATH) {
    violations.push(`${PNPM_WORKSPACE_PATH}: one canonical root workspace policy is required`);
  } else {
    validatePnpmWorkspaceBuildPolicy(snapshot.files?.[PNPM_WORKSPACE_PATH] ?? "", violations);
  }

  for (const path of trackedPaths.filter((candidate) => isAutomationPath(candidate))) {
    violations.push(`${path}: repository-controlled local automation file is forbidden`);
  }

  return [...new Set(violations)].sort();
}

function readManifest(repoRoot, path) {
  try {
    const value = JSON.parse(readFileSync(resolve(repoRoot, path), "utf8"));
    return {
      path,
      name: value.name,
      scripts: value.scripts,
      pnpm: value.pnpm,
      hookConfigKeys: [...FORBIDDEN_MANIFEST_CONFIG_KEYS].filter((key) =>
        Object.hasOwn(value, key)
      ),
    };
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
      path === ".trellis/config.yaml" ||
      isAutomationPath(path) ||
      PACKAGE_MANAGER_CONFIG_PATH_PATTERN.test(path) ||
      JAVASCRIPT_SOURCE_PATTERN.test(path)
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
