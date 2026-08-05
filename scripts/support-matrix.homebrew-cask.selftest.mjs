import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const command = ["scripts/support-matrix.mjs", "homebrew-cask"];
const repository = "KNaiFen/aio-coding-hub";
const armSha256 = "6b126f39ec625e97d182301fafcbfff81ce6f332e297880aef2b0eab0a3c0c4a";
const caskAssetName = "aio-coding-hub-macos-arm.zip";

function runSupportMatrix(args) {
  return spawnSync("node", [...command, ...args], {
    cwd: process.cwd(),
    encoding: "utf8",
  });
}

function assertIncludes(value, expected) {
  if (!value.includes(expected)) {
    throw new Error(`Expected output to include:\n${expected}\n\nActual output:\n${value}`);
  }
}

function assertEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`${label}\nExpected: ${expected}\nActual: ${actual}`);
  }
}

function assertNotIncludes(value, unexpected) {
  if (value.includes(unexpected)) {
    throw new Error(`Expected output not to include:\n${unexpected}\n\nActual output:\n${value}`);
  }
}

function testPrintsCaskForCurrentRelease() {
  const result = runSupportMatrix([
    "--tag",
    "aio-coding-hub-v0.60.4",
    "--repo",
    repository,
    "--macos-arm-sha256",
    armSha256,
  ]);

  assertEqual(result.status, 0, "homebrew-cask command should succeed");
  assertIncludes(result.stdout, 'cask "aio-coding-hub" do');
  assertIncludes(result.stdout, 'version "0.60.4"');
  assertIncludes(result.stdout, `sha256 "${armSha256}"`);
  assertIncludes(
    result.stdout,
    `url "https://github.com/${repository}/releases/download/aio-coding-hub-v#{version}/${caskAssetName}"`
  );
  assertNotIncludes(result.stdout, "intel");
  assertNotIncludes(result.stdout, "#{arch}");
  assertIncludes(result.stdout, 'app "AIO Coding Hub.app"');
  assertIncludes(result.stdout, "auto_updates true");
  assertIncludes(result.stdout, "depends_on :macos");
  assertIncludes(result.stdout, "depends_on arch: :arm64");
}

function testWritesCaskToOutputPath() {
  const root = mkdtempSync(join(tmpdir(), "aio-homebrew-cask-"));
  const outputPath = join(root, "Casks/aio-coding-hub.rb");

  try {
    const result = runSupportMatrix([
      "--tag",
      "aio-coding-hub-v0.60.4",
      "--repo",
      repository,
      "--macos-arm-sha256",
      armSha256,
      "--output",
      outputPath,
    ]);

    assertEqual(result.status, 0, "homebrew-cask command should write an output file");
    assertIncludes(readFileSync(outputPath, "utf8"), 'cask "aio-coding-hub" do');
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

function testRequiresMacosArmHash() {
  const result = runSupportMatrix(["--tag", "aio-coding-hub-v0.60.4", "--repo", repository]);

  assertEqual(result.status, 1, "homebrew-cask command should fail without the ARM hash");
  assertIncludes(result.stderr, "Missing required argument: --macos-arm-sha256");
}

function testRejectsLegacyIntelHash() {
  const result = runSupportMatrix([
    "--tag",
    "aio-coding-hub-v0.60.4",
    "--repo",
    repository,
    "--macos-arm-sha256",
    armSha256,
    "--macos-intel-sha256",
    "18f376bc6266e8cef4fb3978240ba0247c56b703370f6a95269443c2adbbbcc6",
  ]);

  assertEqual(result.status, 1, "homebrew-cask command should reject the legacy Intel hash");
  assertIncludes(result.stderr, "formal Release excludes macOS Intel desktop assets");
}

function testReleaseContractsContainCaskAsset() {
  for (const path of [".github/workflows/ci.yml", ".github/workflows/release.yml"]) {
    assertIncludes(readFileSync(path, "utf8"), caskAssetName);
  }
}

for (const testCase of [
  testPrintsCaskForCurrentRelease,
  testWritesCaskToOutputPath,
  testRequiresMacosArmHash,
  testRejectsLegacyIntelHash,
  testReleaseContractsContainCaskAsset,
]) {
  testCase();
}

console.log("[support-matrix] Homebrew Cask self-test passed.");
