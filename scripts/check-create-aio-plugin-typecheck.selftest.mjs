import assert from "node:assert/strict";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

const packageJsonPath = fileURLToPath(
  new URL("../packages/create-aio-plugin/package.json", import.meta.url)
);
const packageDir = dirname(packageJsonPath);
const repoRoot = dirname(dirname(packageDir));
const packageRequire = createRequire(packageJsonPath);
const typescriptRoot = dirname(packageRequire.resolve("typescript/package.json"));
const tscPath = join(typescriptRoot, "bin", "tsc");

function runTypecheck(root) {
  return spawnSync(process.execPath, [tscPath, "-p", join(root, "tsconfig.json"), "--noEmit"], {
    encoding: "utf8",
  });
}

const packageJson = JSON.parse(readFileSync(packageJsonPath, "utf8"));
assert.equal(packageJson.scripts?.typecheck, "tsc -p tsconfig.json --noEmit");

const root = mkdtempSync(join(tmpdir(), "aio-create-plugin-typecheck-"));
try {
  const fixturePackageDir = join(root, "packages", "create-aio-plugin");
  const fixtureSdkDir = join(root, "packages", "plugin-sdk", "src");
  mkdirSync(join(fixturePackageDir, "src"), { recursive: true });
  mkdirSync(fixtureSdkDir, { recursive: true });
  symlinkSync(join(repoRoot, "node_modules"), join(root, "node_modules"), "dir");
  symlinkSync(join(packageDir, "node_modules"), join(fixturePackageDir, "node_modules"), "dir");
  writeFileSync(
    join(fixturePackageDir, "tsconfig.json"),
    readFileSync(join(packageDir, "tsconfig.json"))
  );
  writeFileSync(join(fixtureSdkDir, "index.ts"), "export type PluginManifest = { id: string };\n");

  const fixturePath = join(fixturePackageDir, "src", "fixture.ts");
  writeFileSync(
    fixturePath,
    'import type { PluginManifest } from "@aio-coding-hub/plugin-sdk";\n' +
      'export const manifest: PluginManifest = { id: "valid" };\n'
  );
  const valid = runTypecheck(fixturePackageDir);
  assert.equal(valid.status, 0, `${valid.stdout}\n${valid.stderr}`);
  assert.equal(existsSync(join(fixturePackageDir, "src", "fixture.js")), false);

  writeFileSync(fixturePath, "export const value: string = 1;\n");
  const invalid = runTypecheck(fixturePackageDir);
  assert.notEqual(invalid.status, 0);
  assert.match(`${invalid.stdout}\n${invalid.stderr}`, /TS2322/);
} finally {
  rmSync(root, { recursive: true, force: true });
}

console.error("[create-aio-plugin:typecheck:selftest] all assertions passed");
