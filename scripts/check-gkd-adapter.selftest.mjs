import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { canonicalBytes, digestObject, verifyAdapter } from "./check-gkd-adapter.mjs";

const root = mkdtempSync(join(tmpdir(), "aio-gkd-adapter-"));
mkdirSync(join(root, ".gkd"));

function writeCanonical(relativePath, value) {
  writeFileSync(join(root, relativePath), canonicalBytes(value));
}

function readFixture() {
  const pin = {
    assetSha256: "f259475f4ca6c3425e53d734d03633541d6a1997e41991eb5a6115958d06a298",
    bundleVersion: "0.1.5",
    executionBundleDigest: "d749b753fb11aeab44d41b4e1d8bec44c7fa2d18a4b08148fbc0e0c127e27e6d",
    releaseSourceSha: "60ac0c49f1054ce2edea49b3ab6758bfbd3432b3",
  };
  const withoutDigest = {
    adapterName: "aio-gkd-review",
    repositories: [
      {
        capabilities: { artifacts: true, checks: true, diff: true, pullRequest: true },
        defaultBranch: "main",
        id: "aio-coding-hub",
        identity: "KNaiFen/aio-coding-hub",
        policyPath: ".gkd/policy.json",
        provider: "github",
      },
    ],
    schemaVersion: 1,
  };
  const policy = {
    baseBranch: "main",
    provider: "github",
    repository: "github.com/KNaiFen/aio-coding-hub",
    requiredChecks: ["ci-gate", "pr-title"],
    schemaVersion: 1,
  };
  const adapterPolicy = {
    ci: {
      artifacts: [
        { nameTemplate: "cloud-native-fixes-{sha}-{run_attempt}", retentionDays: 7 },
        { nameTemplate: "dev-build-{target_id}-{sha}", retentionDays: 7 },
        { nameTemplate: "release-candidate-{sha}-{run_id}-{run_attempt}", retentionDays: 30 },
        { nameTemplate: "release-platform-{sha}-{run_attempt}-{target_id}", retentionDays: 1 },
        { nameTemplate: "tui-platform-{sha}-{run_attempt}-{target_id}", retentionDays: 1 },
      ],
      cacheClasses: ["pnpm", "rust"],
      runnerClasses: ["macos-latest", "ubuntu-22.04", "ubuntu-latest", "windows-latest"],
    },
    release: {
      candidate: {
        artifactNameTemplate: "release-candidate-{sha}-{run_id}-{run_attempt}",
        branch: "main",
        conclusion: "success",
        events: ["push", "workflow_dispatch"],
        repository: "KNaiFen/aio-coding-hub",
        sameSourceSha: true,
        uniqueUnexpired: true,
        workflow: "ci.yml",
      },
      checksum: { algorithm: "sha256", coversAllOtherAssets: true, manifest: "SHA256SUMS.txt" },
      existingRelease: { assetNamesAndChecksumsMustMatch: true, overwrite: false },
      requireMainAncestor: true,
      tagTemplate: "aio-coding-hub-v{semver}",
    },
    schemaVersion: 1,
    verification: {
      adapterSmoke: {
        pathPrefixes: [".gkd/"],
        paths: [
          "scripts/check-gkd-adapter.mjs",
          "scripts/check-gkd-adapter.selftest.mjs",
          "scripts/check-local-verification.mjs",
          "scripts/check-local-verification.selftest.mjs",
          "scripts/gkd-verify",
        ],
      },
      baseArgument: "--base-sha",
      baseShaPattern: "^[0-9a-f]{40}$",
      cloudOwned: [
        "dependencies",
        "format",
        "lint",
        "typecheck",
        "tests",
        "coverage",
        "build",
        "generators",
        "rust-cargo",
        "tauri",
        "signing-packaging",
        "dev-server-runtime-ui",
      ],
      entrypoint: "scripts/gkd-verify",
      zeroArtifact: true,
    },
  };
  const historyAdapter = {
    active: {
      coordinationVersionsRejected: [1],
      location: "tracked-immediate-child",
      requiredCount: 1,
      root: ".trellis/tasks",
      worktreePath: "must-be-null",
    },
    archive: {
      location: "tracked-descendants",
      requiredStatus: "completed",
      root: ".trellis/tasks/archive",
      worktreePath: "ignored",
    },
    manifestName: "task.json",
    schemaVersion: 1,
  };
  return {
    pin,
    adapter: { ...withoutDigest, adapterDigest: digestObject(withoutDigest) },
    adapterPolicy,
    historyAdapter,
    policy,
    resourceFacts: {
      billing: { cost: "unknown", verified: false },
      policy: {
        baseBranch: policy.baseBranch,
        policyDigest: digestObject(policy),
        requiredChecks: policy.requiredChecks,
      },
      resource: { capacity: "unknown", verified: false },
      runner: { kind: "github-hosted-linux", source: "github-actions-workflow", verified: true },
      schemaVersion: 1,
    },
  };
}

function writeFixture(fixture) {
  writeCanonical(".gkd/bundle-pin.json", fixture.pin);
  writeCanonical(".gkd/review-adapter.json", fixture.adapter);
  writeCanonical(".gkd/adapter-policy.json", fixture.adapterPolicy);
  writeCanonical(".gkd/history-adapter.json", fixture.historyAdapter);
  writeCanonical(".gkd/policy.json", fixture.policy);
  writeCanonical(".gkd/resource-facts.json", fixture.resourceFacts);
}

function expectFailure(mutate, expected) {
  const fixture = readFixture();
  mutate(fixture);
  writeFixture(fixture);
  assert.throws(() => verifyAdapter(root), expected);
}

try {
  writeFixture(readFixture());
  assert.deepEqual(verifyAdapter(root), {
    outcome: "adapter_ready",
    adapterDigest: "eac007446f5ce616aad866185b66da59a1fc5c74b32de21c0dffe117ed0443b6",
    bundleVersion: "0.1.5",
  });

  writeFileSync(join(root, ".gkd/bundle-pin.json"), "{ }\n");
  assert.throws(() => verifyAdapter(root), /ADAPTER_JSON_NOT_CANONICAL/);

  writeFixture(readFixture());
  writeFileSync(join(root, ".gkd/adapter-policy.json"), "{ }\n");
  assert.throws(() => verifyAdapter(root), /ADAPTER_JSON_NOT_CANONICAL/);

  writeFixture(readFixture());
  writeFileSync(join(root, ".gkd/resource-facts.json"), "{ }\n");
  assert.throws(() => verifyAdapter(root), /ADAPTER_JSON_NOT_CANONICAL/);

  writeFixture(readFixture());
  writeFileSync(join(root, ".gkd/history-adapter.json"), "{ }\n");
  assert.throws(() => verifyAdapter(root), /ADAPTER_JSON_NOT_CANONICAL/);

  expectFailure(
    (fixture) => {
      fixture.pin.unexpected = true;
    },
    /PIN_FIELDS_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.adapter.adapterDigest = "0".repeat(64);
    },
    /ADAPTER_DIGEST_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.policy.repository = "github.com/KNaiFen/other";
    },
    /POLICY_BINDING_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.adapter.repositories[0].identity = "KNaiFen/other";
      const { adapterDigest, ...withoutDigest } = fixture.adapter;
      fixture.adapter.adapterDigest = digestObject(withoutDigest);
    },
    /ADAPTER_REPOSITORY_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.pin.bundleVersion = "0.1.3";
    },
    /PIN_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.adapterPolicy.unexpected = true;
    },
    /ADAPTER_POLICY_FIELDS_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.historyAdapter.unexpected = true;
    },
    /HISTORY_ADAPTER_FIELDS_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.historyAdapter.active.requiredCount = 2;
    },
    /HISTORY_ADAPTER_ACTIVE_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.historyAdapter.active.unexpected = true;
    },
    /HISTORY_ADAPTER_ACTIVE_FIELDS_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.historyAdapter.active.coordinationVersionsRejected = [];
    },
    /HISTORY_ADAPTER_ACTIVE_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.historyAdapter.archive.worktreePath = "required";
    },
    /HISTORY_ADAPTER_ARCHIVE_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.historyAdapter.archive.unexpected = true;
    },
    /HISTORY_ADAPTER_ARCHIVE_FIELDS_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.adapterPolicy.verification.entrypoint = "scripts/other";
    },
    /ADAPTER_POLICY_VERIFICATION_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.adapterPolicy.verification.baseShaPattern = "^[0-9a-f]+$";
    },
    /ADAPTER_POLICY_VERIFICATION_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.adapterPolicy.verification.zeroArtifact = false;
    },
    /ADAPTER_POLICY_VERIFICATION_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.adapterPolicy.verification.adapterSmoke.paths.pop();
    },
    /ADAPTER_POLICY_SMOKE_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.adapterPolicy.verification.cloudOwned.pop();
    },
    /ADAPTER_POLICY_CLOUD_BOUNDARY_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.adapterPolicy.ci.runnerClasses[0] = "self-hosted";
    },
    /ADAPTER_POLICY_RUNNERS_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.adapterPolicy.ci.cacheClasses = ["pnpm"];
    },
    /ADAPTER_POLICY_CACHES_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.adapterPolicy.ci.artifacts[0].nameTemplate = "cloud-fixes-{sha}";
    },
    /ADAPTER_POLICY_ARTIFACTS_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.adapterPolicy.ci.artifacts[0].retentionDays = 30;
    },
    /ADAPTER_POLICY_ARTIFACTS_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.adapterPolicy.ci.artifacts[0].unexpected = true;
    },
    /ADAPTER_POLICY_ARTIFACT_FIELDS_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.adapterPolicy.release.tagTemplate = "v{semver}";
    },
    /ADAPTER_POLICY_TAG_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.adapterPolicy.release.candidate.sameSourceSha = false;
    },
    /ADAPTER_POLICY_CANDIDATE_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.adapterPolicy.release.candidate.conclusion = "failure";
    },
    /ADAPTER_POLICY_CANDIDATE_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.adapterPolicy.release.candidate.uniqueUnexpired = false;
    },
    /ADAPTER_POLICY_CANDIDATE_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.adapterPolicy.release.checksum.manifest = "checksums.txt";
    },
    /ADAPTER_POLICY_CHECKSUM_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.adapterPolicy.release.requireMainAncestor = false;
    },
    /ADAPTER_POLICY_MAIN_ANCESTRY_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.adapterPolicy.release.existingRelease.overwrite = true;
    },
    /ADAPTER_POLICY_IMMUTABILITY_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.resourceFacts.unexpected = true;
    },
    /RESOURCE_FACTS_FIELDS_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.resourceFacts.runner.source = "github-hosted-linux";
    },
    /RESOURCE_RUNNER_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.policy.requiredChecks = ["ci-gate"];
    },
    /RESOURCE_POLICY_BINDING_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.resourceFacts.policy.requiredChecks = ["ci-gate"];
    },
    /RESOURCE_POLICY_BINDING_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.resourceFacts.resource.verified = true;
    },
    /RESOURCE_RESOURCE_INVALID/
  );
  expectFailure(
    (fixture) => {
      fixture.resourceFacts.billing.verified = true;
    },
    /RESOURCE_BILLING_INVALID/
  );
} finally {
  rmSync(root, { recursive: true, force: true });
}

console.log("[gkd-adapter:selftest] all assertions passed");
