import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const checkerPath = fileURLToPath(new URL("./check-no-instant-now-sub.mjs", import.meta.url));

function runFixture(source, { createTarget = true } = {}) {
  const root = mkdtempSync(join(tmpdir(), "aio-instant-check-"));
  try {
    if (createTarget) {
      const sourceDir = join(root, "src-tauri", "src");
      mkdirSync(sourceDir, { recursive: true });
      writeFileSync(join(sourceDir, "fixture.rs"), source);
    }
    return spawnSync(process.execPath, [checkerPath], {
      cwd: root,
      encoding: "utf8",
    });
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

for (const source of [
  "let started_at = std::time::Instant::now() - std::time::Duration::from_secs(5);\n",
  "let started_at = std::time::Instant::now()\n  - std::time::Duration::from_secs(5);\n",
  "let started_at = std::time::Instant::now() /* explain */ - std::time::Duration::from_secs(5);\n",
  "let started_at = std::time::Instant::now() // explain\n  - std::time::Duration::from_secs(5);\n",
  "let started_at = std::time::Instant::now() /* outer /* inner */ outer */ - std::time::Duration::from_secs(5);\n",
]) {
  const forbidden = runFixture(source);
  assert.equal(forbidden.status, 1, source);
  assert.match(forbidden.stderr, /Forbidden pattern detected/);
  assert.match(forbidden.stderr, /fixture\.rs:1/);
}

for (const source of [
  "let started_at = std::time::Instant::now().checked_sub(duration);\n",
  "let elapsed = std::time::Instant::now().saturating_duration_since(started_at);\n",
]) {
  const allowed = runFixture(source);
  assert.equal(allowed.status, 0, allowed.stderr);
}

const missingTarget = runFixture("", { createTarget: false });
assert.equal(missingTarget.status, 2);
assert.match(missingTarget.stderr, /Expected directory not found/);

console.error("[no-instant-now-sub:selftest] all assertions passed");
