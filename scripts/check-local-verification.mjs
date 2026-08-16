import { spawnSync } from "node:child_process";
import { lstatSync, readFileSync, realpathSync, statSync } from "node:fs";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const modulePath = fileURLToPath(import.meta.url);
const repoRoot = dirname(dirname(modulePath));
const FULL_SHA = /^[0-9a-f]{40}$/;
const NODE_SOURCE = /\.(?:cjs|js|mjs)$/;
const MAX_UNTRACKED_TEXT_BYTES = 16 * 1024 * 1024;

export class VerificationError extends Error {}
export class UsageError extends Error {}

function run(command, args, { cwd = repoRoot, accepted = [0], label } = {}) {
  const result = spawnSync(command, args, {
    cwd,
    encoding: "utf8",
    shell: false,
    maxBuffer: 16 * 1024 * 1024,
  });
  if (result.error) throw new VerificationError(`${label ?? command}: ${result.error.message}`);
  if (!accepted.includes(result.status)) {
    const detail = (result.stderr || result.stdout || "no diagnostic output").trim();
    throw new VerificationError(`${label ?? command} failed (${result.status}): ${detail}`);
  }
  return result;
}

function git(args, options = {}) {
  return run("git", args, { ...options, label: options.label ?? `git ${args.join(" ")}` });
}

export function parseArguments(argv) {
  if (argv.length !== 2 || argv[0] !== "--base") {
    throw new UsageError("usage: node scripts/check-local-verification.mjs --base <full-lowercase-sha>");
  }
  if (!FULL_SHA.test(argv[1])) {
    throw new UsageError("--base must be a lowercase 40-character SHA");
  }
  return { base: argv[1] };
}

export function parseNulPaths(value) {
  return value.split("\0").filter(Boolean);
}

function assertRepositoryRoot(root) {
  const top = git(["rev-parse", "--show-toplevel"], { cwd: root }).stdout.trim();
  if (realpathSync(top) !== realpathSync(root)) {
    throw new UsageError(`runner must execute in repository ${root}`);
  }
}

function assertSafeFile(root, path) {
  const absolute = resolve(root, path);
  const fromRoot = relative(root, absolute);
  if (!fromRoot || fromRoot.startsWith("..") || isAbsolute(fromRoot)) {
    throw new VerificationError(`changed path escapes repository: ${path}`);
  }
  const metadata = lstatSync(absolute);
  if (metadata.isSymbolicLink() || !metadata.isFile()) {
    throw new VerificationError(`changed Node path must be a regular file: ${path}`);
  }
  const resolved = realpathSync(absolute);
  const resolvedFromRoot = relative(realpathSync(root), resolved);
  if (!resolvedFromRoot || resolvedFromRoot.startsWith("..") || isAbsolute(resolvedFromRoot)) {
    throw new VerificationError(`changed Node path resolves outside repository: ${path}`);
  }
  return absolute;
}

function changedPaths(root, base) {
  const commands = [
    ["diff", "--name-only", "-z", "--diff-filter=ACMR", base, "HEAD", "--"],
    ["diff", "--cached", "--name-only", "-z", "--diff-filter=ACMR", "--"],
    ["diff", "--name-only", "-z", "--diff-filter=ACMR", "--"],
    ["ls-files", "--others", "--exclude-standard", "-z"],
  ];
  return new Set(
    commands.flatMap((args) => parseNulPaths(git(args, { cwd: root }).stdout))
  );
}

export function collectChangedNodeFiles(root, base) {
  return [...changedPaths(root, base)]
    .filter((path) => NODE_SOURCE.test(path))
    .filter((path) => {
      try {
        return lstatSync(resolve(root, path)).isFile() || lstatSync(resolve(root, path)).isSymbolicLink();
      } catch (error) {
        if (error?.code === "ENOENT") return false;
        throw error;
      }
    })
    .sort();
}

export function findWhitespaceErrors(path, content) {
  if (content.includes(0)) return [];
  const text = content.toString("utf8");
  const errors = [];
  text.split("\n").forEach((line, index) => {
    const body = line.endsWith("\r") ? line.slice(0, -1) : line;
    if (/[ \t]+$/.test(body)) errors.push(`${path}:${index + 1}: trailing whitespace`);
  });
  if (/\n[ \t\r]*\n$/.test(text)) errors.push(`${path}: new blank line at EOF`);
  return errors;
}

function assertUntrackedWhitespace(root) {
  const paths = parseNulPaths(
    git(["ls-files", "--others", "--exclude-standard", "-z"], { cwd: root }).stdout
  );
  const failures = [];
  for (const path of paths) {
    const absolute = resolve(root, path);
    const metadata = lstatSync(absolute);
    if (!metadata.isFile() || metadata.isSymbolicLink()) continue;
    if (statSync(absolute).size > MAX_UNTRACKED_TEXT_BYTES) {
      throw new VerificationError(`untracked file exceeds local whitespace scan limit: ${path}`);
    }
    failures.push(...findWhitespaceErrors(path, readFileSync(absolute)));
  }
  if (failures.length > 0) {
    throw new VerificationError(`untracked whitespace check failed:\n${failures.join("\n")}`);
  }
}

function runNodeScript(relativePath) {
  run(process.execPath, [join(repoRoot, relativePath)], { label: relativePath });
}

function runDiffChecks(base) {
  git(["diff", "--check", base, "HEAD", "--"], { label: "committed diff check" });
  git(["diff", "--cached", "--check", "--"], { label: "index diff check" });
  git(["diff", "--check", "--"], { label: "worktree diff check" });
}

export function verify(base) {
  assertRepositoryRoot(repoRoot);
  git(["cat-file", "-e", `${base}^{commit}`], { label: "base commit lookup" });
  const ancestry = git(["merge-base", "--is-ancestor", base, "HEAD"], {
    accepted: [0, 1],
    label: "base ancestry check",
  });
  if (ancestry.status !== 0) throw new UsageError(`base ${base} is not an ancestor of HEAD`);

  const head = git(["rev-parse", "HEAD"]).stdout.trim();
  runNodeScript("scripts/check-local-verification.selftest.mjs");
  runNodeScript("scripts/check-cloud-only-verification.selftest.mjs");
  runNodeScript("scripts/check-cloud-only-verification.mjs");
  runDiffChecks(base);
  assertUntrackedWhitespace(repoRoot);

  const nodeFiles = collectChangedNodeFiles(repoRoot, base);
  for (const path of nodeFiles) {
    run(process.execPath, ["--check", assertSafeFile(repoRoot, path)], {
      label: `node --check ${path}`,
    });
  }

  return {
    outcome: "local_ready",
    base_sha: base,
    head_sha: head,
    checked_node_files: nodeFiles,
    checks: [
      "local-runner-selftest",
      "cloud-only-selftest",
      "cloud-only-contract",
      "committed-diff",
      "index-diff",
      "worktree-diff",
      "untracked-whitespace",
      "changed-node-syntax",
    ],
    cloud_owned: [
      "dependencies",
      "format",
      "lint",
      "typecheck",
      "tests",
      "coverage",
      "build",
      "generators",
      "rust-cargo",
      "tauri",
      "signing-packaging",
      "dev-server-runtime-ui",
    ],
  };
}

if (process.argv[1] && resolve(process.argv[1]) === modulePath) {
  try {
    const { base } = parseArguments(process.argv.slice(2));
    console.log(JSON.stringify(verify(base), null, 2));
  } catch (error) {
    const usage = error instanceof UsageError;
    console.error(
      JSON.stringify({ outcome: usage ? "invalid_input" : "local_failure", reason: String(error.message ?? error) })
    );
    process.exit(usage ? 2 : 1);
  }
}
