import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { scanRedactedText, validateCiReleaseAdapter, validateWorkflowSurface } from "./check-gkd-ci-release.mjs";

const root = fileURLToPath(new URL("..", import.meta.url));
const result = validateCiReleaseAdapter(root);
assert.equal(result.outcome, "ci_release_ready");
assert.deepEqual(result.requiredChecks, ["ci-gate", "pr-title"]);
assert.deepEqual(scanRedactedText("fixture.txt", "token=ghp_12345678901234567890"), [{ code: "CREDENTIAL_SHAPED", path: "fixture.txt" }]);
assert.deepEqual(scanRedactedText("fixture.txt", "path=/Users/example/private.key"), [{ code: "MACHINE_LOCAL_PATH", path: "fixture.txt" }]);
assert.deepEqual(scanRedactedText("fixture.txt", "clean=true"), []);
const ciWorkflow = readFileSync(`${root}.github/workflows/ci.yml`, "utf8");
const prTitleWorkflow = readFileSync(`${root}.github/workflows/pr-title.yml`, "utf8");
const releaseWorkflow = readFileSync(`${root}.github/workflows/release.yml`, "utf8");
assert.throws(() => validateWorkflowSurface({ ci: ciWorkflow.replace(/(  ci-gate:[\s\S]*?\n    if: )always\(\)/, "$1success()"), prTitle: prTitleWorkflow, release: releaseWorkflow }), /GATE_NOT_FAIL_CLOSED/);
assert.throws(() => validateWorkflowSurface({ ci: ciWorkflow.replace("release-candidate-${{ github.sha }}-${{ github.run_id }}-${{ github.run_attempt }}", "release-candidate-static"), prTitle: prTitleWorkflow, release: releaseWorkflow }), /ARTIFACT_DECLARATION_MISSING/);
console.log("[gkd-ci-release:selftest] all assertions passed");
