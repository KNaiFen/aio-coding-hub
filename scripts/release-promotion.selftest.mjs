import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import {
  assessExistingRelease,
  selectReleaseCandidate,
  validateReleaseAssets,
} from "./release-promotion.mjs";

const releaseWorkflowPath = fileURLToPath(
  new URL("../.github/workflows/release.yml", import.meta.url)
);
const releaseWorkflow = readFileSync(releaseWorkflowPath, "utf8");
const guardStageIndex = releaseWorkflow.indexOf("Stage release promotion guard");
const sourceCheckoutIndex = releaseWorkflow.indexOf('git checkout --detach "$source_sha"');
const guardImport = "pathToFileURL(`${process.env.RUNNER_TEMP}/release-promotion.mjs`).href";
const checksumA = "a".repeat(64);
const checksumB = "b".repeat(64);
const checksumC = "c".repeat(64);
const assetNames = ["aio-coding-hub-macos-arm.zip", "latest.json", "SHA256SUMS.txt"];
const candidateManifest = [
  `${checksumA}  aio-coding-hub-macos-arm.zip`,
  `${checksumB}  latest.json`,
].join("\n");

const sourceSha = "d".repeat(40);
const releaseCandidateName = `release-candidate-${sourceSha}-7-1`;
const candidateRun = {
  artifacts: [{ expired: false, id: 42, name: releaseCandidateName }],
  id: 7,
  runAttempt: 1,
};
assert.deepEqual(selectReleaseCandidate([candidateRun], sourceSha), {
  artifactId: 42,
  artifactName: releaseCandidateName,
  runId: 7,
});
assert.throws(
  () => selectReleaseCandidate([], sourceSha),
  /exactly one unexpired release candidate, found 0/
);
assert.throws(
  () =>
    selectReleaseCandidate(
      [
        candidateRun,
        {
          artifacts: [{ expired: false, id: 43, name: `release-candidate-${sourceSha}-8-1` }],
          id: 8,
          runAttempt: 1,
        },
      ],
      sourceSha
    ),
  /exactly one unexpired release candidate, found 2/
);
assert.throws(
  () => selectReleaseCandidate([{ ...candidateRun, artifacts: [] }], sourceSha),
  /exactly one unexpired release candidate, found 0/
);

assert.deepEqual(
  validateReleaseAssets({
    assetNames,
    manifestText: candidateManifest,
    label: "Candidate release",
  }).names,
  new Set(assetNames)
);
assert.throws(
  () =>
    validateReleaseAssets({
      assetNames: [...assetNames, "unlisted.txt"],
      manifestText: candidateManifest,
      label: "Candidate release",
    }),
  /checksum entries do not match its asset names/
);
assert.throws(
  () =>
    validateReleaseAssets({
      assetNames,
      manifestText: `${checksumA}  aio-coding-hub-macos-arm.zip`,
      label: "Candidate release",
    }),
  /checksum entries do not match its asset names/
);
assert.throws(
  () =>
    validateReleaseAssets({
      assetNames,
      manifestText: "not-a-checksum",
      label: "Candidate release",
    }),
  /invalid or duplicate checksum entry/
);

assert.deepEqual(
  assessExistingRelease({
    candidateAssetNames: assetNames,
    candidateManifestText: candidateManifest,
    existingAssetNames: [...assetNames].reverse(),
    existingManifestText: [
      `${checksumB}  latest.json`,
      `${checksumA}  aio-coding-hub-macos-arm.zip`,
    ].join("\n"),
  }),
  { equivalent: true }
);
assert.deepEqual(
  assessExistingRelease({
    candidateAssetNames: assetNames,
    candidateManifestText: candidateManifest,
    existingAssetNames: ["aio-coding-hub-macos-arm.zip", "SHA256SUMS.txt"],
    existingManifestText: `${checksumA}  aio-coding-hub-macos-arm.zip`,
  }),
  { equivalent: false, reason: "asset names differ" }
);
assert.throws(
  () =>
    assessExistingRelease({
      candidateAssetNames: assetNames,
      candidateManifestText: candidateManifest,
      existingAssetNames: [...assetNames, "unlisted.txt"],
      existingManifestText: candidateManifest,
    }),
  /Existing release checksum entries do not match its asset names/
);
assert.deepEqual(
  assessExistingRelease({
    candidateAssetNames: assetNames,
    candidateManifestText: candidateManifest,
    existingAssetNames: assetNames,
    existingManifestText: [
      `${checksumC}  aio-coding-hub-macos-arm.zip`,
      `${checksumB}  latest.json`,
    ].join("\n"),
  }),
  { equivalent: false, reason: "asset checksums differ" }
);

assert.ok(
  releaseWorkflow.includes("group: release-${{ inputs.tag || github.ref_name }}"),
  "release workflow must serialize push and dispatch by the resolved release tag"
);
assert.ok(
  releaseWorkflow.includes("cancel-in-progress: false"),
  "release workflow must queue, not cancel, same-tag publication"
);
assert.ok(
  guardStageIndex !== -1 && guardStageIndex < sourceCheckoutIndex,
  "release workflow must stage its promotion guard before checking out the release source"
);
assert.equal(
  releaseWorkflow.split(guardImport).length - 1,
  2,
  "both promotion checks must import the staged guard instead of the release source checkout"
);
assert.ok(
  releaseWorkflow.includes("selectReleaseCandidate(candidateRuns, process.env.SOURCE_SHA)"),
  "release workflow must reject ambiguous candidates through the shared guard"
);
assert.ok(
  releaseWorkflow.includes("validateReleaseAssets({"),
  "release workflow must verify every candidate asset before first publication"
);
assert.ok(
  releaseWorkflow.includes("assessExistingRelease({"),
  "release workflow must compare an existing release before publication"
);
assert.ok(
  releaseWorkflow.includes("overwrite_files: false"),
  "release workflow must never overwrite existing release assets"
);
assert.ok(
  releaseWorkflow.includes("steps.release-preflight.outputs.should_publish == 'true'"),
  "release workflow must skip an identical existing release"
);

console.log("Release promotion self-test passed.");
