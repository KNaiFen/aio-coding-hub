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
    assetSha256: "713fc828d234bc7ddd298cb68f5abfe1ede29f7891c283924cf3c3b98b2c0330",
    bundleVersion: "0.1.4",
    executionBundleDigest: "cdaa791ace82a5e7c407b29a93a4211b852d7f364900bbcd8a549dbe918bf2a7",
    releaseSourceSha: "be1e515a64c4095676922c484555fb2a048da681",
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
  return {
    pin,
    adapter: { ...withoutDigest, adapterDigest: digestObject(withoutDigest) },
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
    bundleVersion: "0.1.4",
  });

  writeFileSync(join(root, ".gkd/bundle-pin.json"), "{ }\n");
  assert.throws(() => verifyAdapter(root), /ADAPTER_JSON_NOT_CANONICAL/);

  writeFixture(readFixture());
  writeFileSync(join(root, ".gkd/resource-facts.json"), "{ }\n");
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
