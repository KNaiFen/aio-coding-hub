import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { validateFloatVersion } from "./float-release.mjs";
import { workflowJobProperty, workflowStepRun, workflowSteps } from "./check-ci-quality-gates.mjs";

const fixture = {
  cargo: '[package]\nname = "aio-float"\nversion = "0.60.63"\n',
  config: { version: "0.60.63" },
  lock: '[[package]]\nname = "aio-coding-hub"\nversion = "0.60.62"\n\n[[package]]\nname = "aio-float"\nversion = "0.60.63"\n',
};
assert.equal(validateFloatVersion(fixture, "aio-float-v0.60.63"), "0.60.63");
assert.throws(() => validateFloatVersion(fixture, "aio-coding-hub-v0.60.63"));
assert.throws(() => validateFloatVersion(fixture, "aio-float-v0.60.62"));
assert.throws(() => validateFloatVersion({ ...fixture, cargo: fixture.cargo.replace("0.60.63", "0.60.62") }));
assert.throws(() => validateFloatVersion({ ...fixture, lock: fixture.lock.replace("0.60.63", "0.60.62") }));
const workflow = readFileSync(new URL("../.github/workflows/float-release.yml", import.meta.url), "utf8");
assert(workflow.includes("tags: ['aio-float-v*']"));
assert(workflow.includes('make_latest: false'));
assert(!workflow.includes('latest.json'));
assert(workflow.includes('uses: ./.github/workflows/float-build.yml'));
const build = readFileSync(new URL("../.github/workflows/float-build.yml", import.meta.url), "utf8");
assert(!/^  (push|pull_request):/m.test(build), "Float builds must use the central classifier without duplicate automatic runs");
assert(/^  workflow_call:/m.test(build));
assert(/^  workflow_dispatch:/m.test(build));
assert.equal(workflowJobProperty(workflow, "build", "uses"), "./.github/workflows/float-build.yml");
assert.equal(workflowJobProperty(workflow, "publish", "needs"), "build");
const steps = workflowSteps(build, "build");
const check = steps.find((step) => step.properties.get("name") === "Check and test Float and shared TUI");
const commands = workflowStepRun(check).value;
for (const command of ["fmt", "clippy", "test"]) {
  assert(commands.includes(`cargo ${command} --manifest-path src-tauri/Cargo.toml -p aio-float -p aio-tui`));
}
const toolchain = steps.find((step) => step.properties.get("uses")?.startsWith("dtolnay/rust-toolchain@"));
assert(toolchain.mappings.get("with").get("components").includes("rustfmt"));
const buildCommands = steps.map((step) => workflowStepRun(step)?.value ?? "").join("\n");
assert(!/pnpm |--workspace|export-bindings|--example|build-release-candidate/.test(buildCommands), "Float builds must not install or compile the desktop application");
const nativeBuild = steps.find((step) => step.properties.get("name") === "Build independent application");
assert.equal(nativeBuild.properties.get("working-directory"), "src-tauri/crates/aio-float");
console.log("Float release isolation checks passed.");
