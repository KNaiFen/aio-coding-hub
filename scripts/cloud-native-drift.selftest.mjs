import assert from "node:assert/strict";
import {
  classifyCloudNativeDriftPaths,
  isAllowedCloudNativeDriftPath,
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
]) {
  assert.equal(isAllowedCloudNativeDriftPath(path), false, path);
}

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

console.log("cloud-native-drift self-test passed");
