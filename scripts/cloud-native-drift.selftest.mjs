import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  classifyCloudNativeDriftPaths,
  classifyWorkingTree,
  isAllowedCloudNativeDriftPath,
  isAllowedCloudNativeUntrackedPath,
} from "./cloud-native-drift.mjs";

for (const path of [
  "src-tauri/Cargo.lock",
  "src-tauri/build.rs",
  "src-tauri/src/main.rs",
  "src-tauri/examples/export-bindings.rs",
  "src/generated/bindings.ts",
]) {
  assert.equal(isAllowedCloudNativeDriftPath(path), true, path);
}

for (const path of [
  "package.json",
  "src-tauri/tauri.conf.json",
  "src-tauri/target/output.rs",
  "src/generated/other.ts",
  "../src-tauri/src/main.rs",
  "src-tauri\\src\\main.rs",
  "/src-tauri/src/main.rs",
  "src-tauri/src/file\ndrift=false.rs",
  "src-tauri/src/file\rdrift=false.rs",
  "src-tauri/src/file\tdrift=false.rs",
]) {
  assert.equal(isAllowedCloudNativeDriftPath(path), false, path);
}

assert.equal(isAllowedCloudNativeUntrackedPath("src-tauri/Cargo.lock"), true);
assert.equal(isAllowedCloudNativeUntrackedPath("src/generated/bindings.ts"), true);
assert.equal(isAllowedCloudNativeUntrackedPath("src-tauri/src/generated.rs"), false);

assert.deepEqual(
  classifyCloudNativeDriftPaths([
    "src-tauri/src/main.rs",
    "src-tauri/Cargo.lock",
    "src-tauri/src/main.rs",
  ]),
  ["src-tauri/Cargo.lock", "src-tauri/src/main.rs"]
);
assert.throws(
  () => classifyCloudNativeDriftPaths(["src-tauri/src/main.rs", "README.md"]),
  /outside the patch boundary/
);

const temporaryRoot = mkdtempSync(join(tmpdir(), "aio-cloud-native-drift-"));
const repositoryRoot = join(temporaryRoot, "repository");
try {
  mkdirSync(repositoryRoot);
  execFileSync("git", ["init", "--quiet"], { cwd: repositoryRoot });
  writeFileSync(join(repositoryRoot, "README.md"), "fixture\n");
  execFileSync("git", ["add", "README.md"], { cwd: repositoryRoot });
  execFileSync(
    "git",
    [
      "-c",
      "user.name=AIO CI",
      "-c",
      "user.email=ci@example.invalid",
      "commit",
      "--quiet",
      "-m",
      "fixture",
    ],
    { cwd: repositoryRoot }
  );

  mkdirSync(join(repositoryRoot, "src-tauri"), { recursive: true });
  writeFileSync(join(repositoryRoot, "src-tauri", "Cargo.lock"), "# regenerated\n");
  const patchPath = join(temporaryRoot, "cloud-native-fixes.patch");
  const githubOutputPath = join(temporaryRoot, "github-output.txt");
  const result = classifyWorkingTree({ repositoryRoot, patchPath, githubOutputPath });
  assert.deepEqual(result, {
    drift: true,
    changedPaths: ["src-tauri/Cargo.lock"],
  });
  assert.match(readFileSync(patchPath, "utf8"), /new file mode/);
  assert.match(readFileSync(patchPath, "utf8"), /\+\+\+ b\/src-tauri\/Cargo\.lock/);
  assert.match(readFileSync(githubOutputPath, "utf8"), /drift=true/);
  assert.equal(
    execFileSync("git", ["ls-files", "--stage", "--", "src-tauri/Cargo.lock"], {
      cwd: repositoryRoot,
      encoding: "utf8",
    }),
    ""
  );

  const maliciousPath = join(
    repositoryRoot,
    "src-tauri",
    "generated\ndrift=false\nchanged_paths=.rs"
  );
  writeFileSync(maliciousPath, "fn generated() {}\n");
  assert.throws(
    () =>
      classifyWorkingTree({
        repositoryRoot,
        patchPath: join(temporaryRoot, "malicious.patch"),
        githubOutputPath: join(temporaryRoot, "malicious-output.txt"),
      }),
    (error) => {
      assert.match(error.message, /unexpected untracked paths/);
      assert.equal(/[\r\n]/.test(error.message), false);
      return true;
    }
  );
  rmSync(maliciousPath);

  writeFileSync(join(repositoryRoot, "package.json"), "{}\n");
  assert.throws(
    () =>
      classifyWorkingTree({
        repositoryRoot,
        patchPath: join(temporaryRoot, "rejected.patch"),
        githubOutputPath: join(temporaryRoot, "rejected-output.txt"),
      }),
    /unexpected untracked paths: \["package\.json"\]/
  );
} finally {
  rmSync(temporaryRoot, { recursive: true, force: true });
}

console.log("cloud-native-drift self-test passed");
