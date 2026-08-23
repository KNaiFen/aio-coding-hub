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
    assetSha256: "9d9e6ea0fff64e0894af08a547b6798f1f6634e0e4cf4e174cd8dfc5c0179954",
    bundleVersion: "0.1.3",
    executionBundleDigest: "cc465d26f08edb2a133775e4d6a58aa517eab1bde0ec2e1ec72f6d9f2c8883bd",
    releaseSourceSha: "2a63cd8ff2fcb7f0cb155dcc32578cda4b3381af",
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
  return {
    pin,
    adapter: { ...withoutDigest, adapterDigest: digestObject(withoutDigest) },
    policy: {
      baseBranch: "main",
      provider: "github",
      repository: "github.com/KNaiFen/aio-coding-hub",
      requiredChecks: ["ci-gate", "pr-title"],
      schemaVersion: 1,
    },
  };
}

function writeFixture(fixture) {
  writeCanonical(".gkd/bundle-pin.json", fixture.pin);
  writeCanonical(".gkd/review-adapter.json", fixture.adapter);
  writeCanonical(".gkd/policy.json", fixture.policy);
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
    bundleVersion: "0.1.3",
  });

  writeFileSync(join(root, ".gkd/bundle-pin.json"), "{ }\n");
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
      fixture.pin.bundleVersion = "0.1.4";
    },
    /PIN_INVALID/
  );
} finally {
  rmSync(root, { recursive: true, force: true });
}

console.log("[gkd-adapter:selftest] all assertions passed");
