import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
  UsageError,
  collectChangedNodeFiles,
  findWhitespaceErrors,
  parseArguments,
  parseNulPaths,
  shouldRunAdapterSmoke,
  shouldRunHistorySmoke,
} from "./check-local-verification.mjs";

const fullSha = "a".repeat(40);
assert.deepEqual(parseArguments(["--base", fullSha]), { base: fullSha });
for (const argv of [[], ["--base", "abc"], ["--base", fullSha.toUpperCase()], ["--base", fullSha, "--all"], ["--command", "pnpm test"]]) {
  assert.throws(() => parseArguments(argv), UsageError);
}
assert.deepEqual(parseNulPaths("a.mjs\0b.js\0"), ["a.mjs", "b.js"]);
assert.deepEqual(findWhitespaceErrors("clean.mjs", Buffer.from("const ok = true;\n")), []);
assert.deepEqual(findWhitespaceErrors("binary", Buffer.from([0, 32, 10])), []);
assert.match(findWhitespaceErrors("bad.mjs", Buffer.from("const bad = true; \n"))[0], /trailing whitespace/);
assert.equal(shouldRunAdapterSmoke(new Set(["src/main.ts"])), false);
assert.equal(shouldRunAdapterSmoke(new Set([".gkd/review-adapter.json"])), true);
assert.equal(shouldRunAdapterSmoke(new Set(["scripts/check-gkd-adapter.mjs"])), true);
assert.equal(shouldRunAdapterSmoke(new Set(["scripts/gkd-verify"])), true);
assert.equal(shouldRunHistorySmoke(new Set(["src/main.ts"])), false);
assert.equal(shouldRunHistorySmoke(new Set([".gkd/history-adapter.json"])), true);
assert.equal(shouldRunHistorySmoke(new Set(["scripts/check-gkd-history.mjs"])), true);
assert.equal(shouldRunHistorySmoke(new Set(["scripts/check-gkd-adapter.mjs"])), true);

function runGit(cwd, args) {
  const result = spawnSync("git", args, { cwd, encoding: "utf8", shell: false });
  assert.equal(result.status, 0, result.stderr || result.stdout);
  return result.stdout.trim();
}

const root = mkdtempSync(join(tmpdir(), "aio-local-verify-"));
try {
  runGit(root, ["init", "--initial-branch=main"]);
  runGit(root, ["config", "user.name", "Local Verify Selftest"]);
  runGit(root, ["config", "user.email", "local-verify@example.invalid"]);
  writeFileSync(join(root, "base.mjs"), "export const base = true;\n");
  runGit(root, ["add", "base.mjs"]);
  runGit(root, ["commit", "-m", "base"]);
  const base = runGit(root, ["rev-parse", "HEAD"]);

  writeFileSync(join(root, "committed.mjs"), "export const committed = true;\n");
  runGit(root, ["add", "committed.mjs"]);
  runGit(root, ["commit", "-m", "committed"]);
  writeFileSync(join(root, "staged.cjs"), "module.exports = true;\n");
  runGit(root, ["add", "staged.cjs"]);
  writeFileSync(join(root, "base.mjs"), "export const worktree = true;\n");
  writeFileSync(join(root, "untracked.js"), "export const untracked = true;\n");
  writeFileSync(join(root, "ignored.ts"), "export const ignored: boolean = true;\n");

  assert.deepEqual(collectChangedNodeFiles(root, base), [
    "base.mjs",
    "committed.mjs",
    "staged.cjs",
    "untracked.js",
  ]);
  assert.match(readFileSync(join(root, "untracked.js"), "utf8"), /untracked/);
} finally {
  rmSync(root, { recursive: true, force: true });
}

console.log("[local-verification:selftest] all assertions passed");
