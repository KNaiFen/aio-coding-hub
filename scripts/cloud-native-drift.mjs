import { execFileSync } from "node:child_process";
import { writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const ALLOWED_EXACT_PATHS = new Set(["src-tauri/Cargo.lock", "src/generated/bindings.ts"]);

export function isAllowedCloudNativeDriftPath(path) {
  if (
    typeof path !== "string" ||
    path.length === 0 ||
    /[\u0000-\u001f\u007f]/.test(path) ||
    path.startsWith("/") ||
    path.includes("\\") ||
    path.split("/").includes("..") ||
    path.startsWith("src-tauri/target/")
  ) {
    return false;
  }
  return ALLOWED_EXACT_PATHS.has(path) || /^src-tauri\/(?:[^/]+\/)*[^/]+\.rs$/.test(path);
}

export function isAllowedCloudNativeUntrackedPath(path) {
  return ALLOWED_EXACT_PATHS.has(path);
}

export function classifyCloudNativeDriftPaths(paths) {
  const normalized = [...new Set(paths)].sort();
  const rejected = normalized.filter((path) => !isAllowedCloudNativeDriftPath(path));
  if (rejected.length > 0) {
    throw new Error(
      `Native canonicalization changed paths outside the patch boundary: ${JSON.stringify(rejected)}`
    );
  }
  return normalized;
}

function readChangedPaths(repositoryRoot) {
  const output = execFileSync(
    "git",
    ["diff", "--name-only", "--diff-filter=ACDMRTUXB", "-z", "--no-ext-diff"],
    { cwd: repositoryRoot, encoding: "utf8", maxBuffer: 16 * 1024 * 1024 }
  );
  return output.split("\0").filter(Boolean);
}

function readUntrackedPaths(repositoryRoot) {
  const output = execFileSync("git", ["ls-files", "--others", "--exclude-standard", "-z"], {
    cwd: repositoryRoot,
    encoding: "utf8",
    maxBuffer: 16 * 1024 * 1024,
  });
  return output.split("\0").filter(Boolean);
}

function appendGithubOutput(path, values) {
  const body = Object.entries(values)
    .map(([key, value]) => {
      if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(key) || /[\r\n]/.test(value)) {
        throw new Error("Unsafe GitHub output value.");
      }
      return `${key}=${value}`;
    })
    .join("\n");
  writeFileSync(path, `${body}\n`, { encoding: "utf8", flag: "a" });
}

function parseArgs(rawArgs) {
  const args = new Map();
  for (let index = 0; index < rawArgs.length; index += 2) {
    const key = rawArgs[index];
    const value = rawArgs[index + 1];
    if (!key?.startsWith("--") || value == null || value.startsWith("--")) {
      throw new Error(`Invalid argument pair: ${key ?? "<missing>"}`);
    }
    const name = key.slice(2);
    if (args.has(name)) throw new Error(`Duplicate argument: ${key}`);
    args.set(name, value);
  }
  return args;
}

export function classifyWorkingTree({ repositoryRoot, patchPath, githubOutputPath }) {
  const untrackedPaths = readUntrackedPaths(repositoryRoot);
  const rejectedUntrackedPaths = untrackedPaths.filter(
    (path) => !isAllowedCloudNativeUntrackedPath(path)
  );
  if (rejectedUntrackedPaths.length > 0) {
    throw new Error(
      `Native canonicalization created unexpected untracked paths: ${JSON.stringify(rejectedUntrackedPaths)}`
    );
  }
  const changedPaths = classifyCloudNativeDriftPaths([
    ...readChangedPaths(repositoryRoot),
    ...untrackedPaths,
  ]);
  if (changedPaths.length === 0) {
    appendGithubOutput(githubOutputPath, { drift: "false", changed_paths: "[]" });
    return Object.freeze({ drift: false, changedPaths });
  }

  if (untrackedPaths.length > 0) {
    execFileSync("git", ["add", "--intent-to-add", "--", ...untrackedPaths], {
      cwd: repositoryRoot,
      stdio: "ignore",
    });
  }
  let patch;
  try {
    patch = execFileSync("git", ["diff", "--binary", "--no-ext-diff"], {
      cwd: repositoryRoot,
      maxBuffer: 64 * 1024 * 1024,
    });
  } finally {
    if (untrackedPaths.length > 0) {
      execFileSync("git", ["reset", "--quiet", "--", ...untrackedPaths], {
        cwd: repositoryRoot,
        stdio: "ignore",
      });
    }
  }
  if (patch.length === 0) {
    throw new Error("Native drift was detected but the bounded patch is empty.");
  }
  writeFileSync(patchPath, patch, { flag: "wx" });
  appendGithubOutput(githubOutputPath, {
    drift: "true",
    changed_paths: JSON.stringify(changedPaths),
  });
  return Object.freeze({ drift: true, changedPaths });
}

export function main(argv = process.argv.slice(2)) {
  const [command, ...rest] = argv;
  if (command !== "classify") {
    throw new Error(
      "Usage: node scripts/cloud-native-drift.mjs classify --repo <path> --patch <path> --github-output <path>"
    );
  }
  const args = parseArgs(rest);
  const allowed = new Set(["repo", "patch", "github-output"]);
  for (const key of args.keys()) {
    if (!allowed.has(key)) throw new Error(`Unknown argument: --${key}`);
  }
  for (const key of allowed) {
    if (!args.get(key)) throw new Error(`Missing required argument: --${key}`);
  }
  classifyWorkingTree({
    repositoryRoot: resolve(args.get("repo")),
    patchPath: resolve(args.get("patch")),
    githubOutputPath: resolve(args.get("github-output")),
  });
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  main();
}
