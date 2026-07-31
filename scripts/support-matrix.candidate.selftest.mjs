import assert from "node:assert/strict";
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  CLOUD_BUILD_TARGETS,
  FORK_RELEASE_TARGET_IDS,
  RELEASE_CANDIDATE_MANIFEST,
  buildManualCloudMatrix,
  buildReleaseMatrix,
  createUpdaterDisabledOverlay,
  createCandidateManifest,
  expectedCandidateFiles,
  planReleaseCandidate,
  readSynchronizedApplicationVersion,
  verifyCandidateManifest,
} from "./support-matrix.mjs";

const VERSION = "1.2.3";
const SOURCE_SHA = "1".repeat(40);
const BEFORE_SHA = "2".repeat(40);
const CONTROL_SHA = "3".repeat(40);

function writeVersionFixture(root, overrides = {}) {
  mkdirSync(join(root, "src-tauri"), { recursive: true });
  const values = {
    package: VERSION,
    cargoToml: VERSION,
    cargoLock: VERSION,
    tauri: VERSION,
    ...overrides,
  };
  writeFileSync(join(root, "package.json"), JSON.stringify({ version: values.package }));
  writeFileSync(
    join(root, "src-tauri/Cargo.toml"),
    `[package]\nname = "aio-coding-hub"\nversion = "${values.cargoToml}"\n\n[dependencies]\n`
  );
  writeFileSync(
    join(root, "src-tauri/Cargo.lock"),
    `version = 4\n\n[[package]]\nname = "aio-coding-hub"\nversion = "${values.cargoLock}"\n`
  );
  writeFileSync(join(root, "src-tauri/tauri.conf.json"), JSON.stringify({ version: values.tauri }));
}

function clone(value) {
  return structuredClone(value);
}

function expectRejected(mutator, fixture) {
  const manifest = clone(fixture.manifest);
  mutator(manifest, fixture.directory);
  assert.throws(() => verifyCandidateManifest(manifest, fixture.context, fixture.directory));
}

const root = mkdtempSync(join(tmpdir(), "support-matrix-selftest-"));
try {
  const versionRoot = join(root, "version");
  writeVersionFixture(versionRoot);
  assert.equal(readSynchronizedApplicationVersion(versionRoot).version, VERSION);
  for (const key of ["package", "cargoToml", "cargoLock", "tauri"]) {
    const mismatchRoot = join(root, `version-mismatch-${key}`);
    writeVersionFixture(mismatchRoot, { [key]: "1.2.4" });
    assert.throws(() => readSynchronizedApplicationVersion(mismatchRoot), /version drifted/);
  }
  const duplicateLockRoot = join(root, "version-duplicate-lock-entry");
  writeVersionFixture(duplicateLockRoot);
  writeFileSync(
    join(duplicateLockRoot, "src-tauri/Cargo.lock"),
    `version = 4\n\n[[package]]\nname = "aio-coding-hub"\nversion = "${VERSION}"\n\n[[package]]\nname = "aio-coding-hub"\nversion = "${VERSION}"\n`
  );
  assert.throws(
    () => readSynchronizedApplicationVersion(duplicateLockRoot),
    /exactly one aio-coding-hub/
  );

  assert.deepEqual(
    buildReleaseMatrix().map((item) => item.target_id),
    FORK_RELEASE_TARGET_IDS
  );
  assert.equal(buildManualCloudMatrix().length, 6);
  assert.deepEqual(createUpdaterDisabledOverlay(), {
    bundle: { createUpdaterArtifacts: false },
  });
  assert.equal(new Set(CLOUD_BUILD_TARGETS.map((item) => item.id)).size, 6);
  const universal = CLOUD_BUILD_TARGETS.find((item) => item.id === "macos-universal");
  assert.equal(universal.tauriTarget, "universal-apple-darwin");
  assert.deepEqual(universal.rustupTargets, ["aarch64-apple-darwin", "x86_64-apple-darwin"]);

  const versions = new Map([
    [SOURCE_SHA, { version: "1.2.4", tag: "aio-coding-hub-v1.2.4" }],
    [BEFORE_SHA, { version: VERSION, tag: `aio-coding-hub-v${VERSION}` }],
  ]);
  const loadVersionAtRevision = (sha) => {
    const value = versions.get(sha);
    if (!value) throw new Error(`missing ${sha}`);
    return value;
  };
  const pushPlan = planReleaseCandidate({
    eventName: "push",
    eventRef: "refs/heads/main",
    eventSha: SOURCE_SHA,
    beforeSha: BEFORE_SHA,
    repository: "KNaiFen/aio-coding-hub",
    recoveryRequested: false,
    loadVersionAtRevision,
  });
  assert.equal(pushPlan.mode, "main-push");
  assert.equal(pushPlan.sourceSha, SOURCE_SHA);
  versions.set(BEFORE_SHA, versions.get(SOURCE_SHA));
  assert.equal(
    planReleaseCandidate({
      eventName: "push",
      eventRef: "refs/heads/main",
      eventSha: SOURCE_SHA,
      beforeSha: BEFORE_SHA,
      repository: "KNaiFen/aio-coding-hub",
      recoveryRequested: false,
      loadVersionAtRevision,
    }).enabled,
    false
  );
  assert.throws(
    () =>
      planReleaseCandidate({
        eventName: "push",
        eventRef: "refs/heads/main",
        eventSha: SOURCE_SHA,
        beforeSha: "0".repeat(40),
        repository: "KNaiFen/aio-coding-hub",
        recoveryRequested: false,
        loadVersionAtRevision,
      }),
    /zero before SHA/
  );
  const recovery = planReleaseCandidate({
    eventName: "workflow_dispatch",
    eventRef: "refs/heads/main",
    eventSha: CONTROL_SHA,
    beforeSha: "",
    repository: "KNaiFen/aio-coding-hub",
    recoveryRequested: true,
    candidateSha: SOURCE_SHA,
    expectedTag: "aio-coding-hub-v1.2.4",
    loadVersionAtRevision,
  });
  assert.equal(recovery.mode, "recovery");
  assert.equal(recovery.trustedControlSha, CONTROL_SHA);
  assert.throws(
    () =>
      planReleaseCandidate({
        eventName: "workflow_dispatch",
        eventRef: "refs/heads/main",
        eventSha: CONTROL_SHA,
        repository: "KNaiFen/aio-coding-hub",
        recoveryRequested: true,
        candidateSha: SOURCE_SHA,
        expectedTag: "aio-coding-hub-v9.9.9",
        loadVersionAtRevision,
      }),
    /mismatch/
  );

  const candidateDirectory = join(root, "candidate");
  mkdirSync(candidateDirectory);
  for (const [index, spec] of expectedCandidateFiles().entries()) {
    writeFileSync(join(candidateDirectory, spec.name), `asset-${index}-${spec.name}`);
  }
  const context = {
    repository: "KNaiFen/aio-coding-hub",
    sourceSha: SOURCE_SHA,
    trustedControlSha: CONTROL_SHA,
    sourceValidationRunId: 123,
    sourceValidationRunAttempt: 2,
    version: VERSION,
    tag: `aio-coding-hub-v${VERSION}`,
    workflowRunId: 456,
    workflowRunAttempt: 3,
    targetIds: FORK_RELEASE_TARGET_IDS,
  };
  const manifest = createCandidateManifest(context, candidateDirectory);
  assert.equal(
    JSON.stringify(createCandidateManifest(context, candidateDirectory)),
    JSON.stringify(manifest)
  );
  writeFileSync(
    join(candidateDirectory, RELEASE_CANDIDATE_MANIFEST),
    `${JSON.stringify(manifest, null, 2)}\n`
  );
  assert.equal(
    verifyCandidateManifest(manifest, context, candidateDirectory).sourceSha,
    SOURCE_SHA
  );

  const fixture = { manifest, context, directory: candidateDirectory };
  expectRejected((value) => {
    value.schemaVersion = 2;
  }, fixture);
  expectRejected((value) => {
    value.unknownField = true;
  }, fixture);
  expectRejected((value) => {
    value.repository = "other/repo";
  }, fixture);
  expectRejected((value) => {
    value.sourceSha = "a".repeat(39);
  }, fixture);
  expectRejected((value) => {
    value.trustedControlSha = "A".repeat(40);
  }, fixture);
  expectRejected((value) => {
    value.sourceValidationRunId = 0;
  }, fixture);
  expectRejected((value) => {
    value.sourceValidationRunAttempt = 0;
  }, fixture);
  expectRejected((value) => {
    value.version = "01.2.3";
  }, fixture);
  expectRejected((value) => {
    value.tag = "aio-coding-hub-v9.9.9";
  }, fixture);
  expectRejected((value) => {
    value.workflowRunAttempt = 0;
  }, fixture);
  expectRejected((value) => {
    value.workflowRunId = 0;
  }, fixture);
  expectRejected((value) => {
    value.targetIds.reverse();
  }, fixture);
  expectRejected((value) => {
    value.targetIds = ["macos-arm64"];
  }, fixture);
  expectRejected((value) => {
    value.files[0].name = "../escape";
  }, fixture);
  expectRejected((value) => {
    value.files[0].name = "windows\\escape";
  }, fixture);
  expectRejected((value) => {
    value.files[1].name = value.files[0].name;
  }, fixture);
  expectRejected((value) => {
    value.files.pop();
  }, fixture);
  expectRejected((value) => {
    value.files[0].size += 1;
  }, fixture);
  expectRejected((value) => {
    value.files[0].targetId = "windows-arm64";
  }, fixture);
  expectRejected((value) => {
    value.files[0].sha256 = "f".repeat(64);
  }, fixture);

  const extraDirectory = join(root, "candidate-extra");
  cpSync(candidateDirectory, extraDirectory, { recursive: true });
  writeFileSync(join(extraDirectory, "unexpected.bin"), "extra");
  assert.throws(
    () => verifyCandidateManifest(manifest, context, extraDirectory),
    /directory entries drifted/
  );
  const missingDirectory = join(root, "candidate-missing");
  cpSync(candidateDirectory, missingDirectory, { recursive: true });
  rmSync(join(missingDirectory, manifest.files[0].name));
  assert.throws(
    () => verifyCandidateManifest(manifest, context, missingDirectory),
    /directory entries drifted/
  );

  const persisted = JSON.parse(
    readFileSync(join(candidateDirectory, RELEASE_CANDIDATE_MANIFEST), "utf8")
  );
  assert.deepEqual(persisted, manifest);
} finally {
  rmSync(root, { recursive: true, force: true });
}

console.log("support-matrix candidate self-test passed");
