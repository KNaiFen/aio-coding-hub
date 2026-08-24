import { createHash } from "node:crypto";
import { lstatSync, readFileSync } from "node:fs";
import { dirname, isAbsolute, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const modulePath = fileURLToPath(import.meta.url);
const repoRoot = dirname(dirname(modulePath));
const SHA1 = /^[0-9a-f]{40}$/;
const SHA256 = /^[0-9a-f]{64}$/;
const IDENTIFIER = /^[A-Za-z0-9][A-Za-z0-9_.-]{0,63}$/;
const IDENTITY = /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/;
const BRANCH = /^[A-Za-z0-9][A-Za-z0-9._/-]{0,127}$/;
const POLICY_PATH = /^[A-Za-z0-9._/-]{1,255}$/;

const BUNDLE_VERSION = "0.1.5";
const RELEASE_SOURCE_SHA = "60ac0c49f1054ce2edea49b3ab6758bfbd3432b3";
const EXECUTION_BUNDLE_DIGEST = "d749b753fb11aeab44d41b4e1d8bec44c7fa2d18a4b08148fbc0e0c127e27e6d";
const ASSET_SHA256 = "f259475f4ca6c3425e53d734d03633541d6a1997e41991eb5a6115958d06a298";
const ADAPTER_NAME = "aio-gkd-review";
const REPOSITORY_ID = "aio-coding-hub";
const REPOSITORY_IDENTITY = "KNaiFen/aio-coding-hub";
const POLICY_PATH_VALUE = ".gkd/policy.json";
const ADAPTER_POLICY_PATH = ".gkd/adapter-policy.json";
const HISTORY_ADAPTER_PATH = ".gkd/history-adapter.json";
const RESOURCE_FACTS_PATH = ".gkd/resource-facts.json";
const RUNNER_KIND = "github-hosted-linux";
const RUNNER_SOURCE = "github-actions-workflow";
const ADAPTER_SMOKE_PATHS = [
  "scripts/check-gkd-adapter.mjs",
  "scripts/check-gkd-adapter.selftest.mjs",
  "scripts/check-local-verification.mjs",
  "scripts/check-local-verification.selftest.mjs",
  "scripts/gkd-verify",
];
const CLOUD_OWNED = [
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
];
const RUNNER_CLASSES = ["macos-latest", "ubuntu-22.04", "ubuntu-latest", "windows-latest"];
const CACHE_CLASSES = ["pnpm", "rust"];
const ARTIFACT_CLASSES = [
  { nameTemplate: "cloud-native-fixes-{sha}-{run_attempt}", retentionDays: 7 },
  { nameTemplate: "dev-build-{target_id}-{sha}", retentionDays: 7 },
  { nameTemplate: "release-candidate-{sha}-{run_id}-{run_attempt}", retentionDays: 30 },
  { nameTemplate: "release-platform-{sha}-{run_attempt}-{target_id}", retentionDays: 1 },
  { nameTemplate: "tui-platform-{sha}-{run_attempt}-{target_id}", retentionDays: 1 },
];

export class AdapterError extends Error {}

function fail(code) {
  throw new AdapterError(code);
}

function isRecord(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function assertExactKeys(value, keys, code) {
  if (!isRecord(value) || Object.keys(value).length !== keys.length || !keys.every((key) => key in value)) {
    fail(code);
  }
}

function assertExactValue(value, expected, code) {
  if (JSON.stringify(value) !== JSON.stringify(expected)) fail(code);
}

export function canonicalJson(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (isRecord(value)) {
    return `{${Object.keys(value)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`)
      .join(",")}}`;
  }
  if (value === null || ["boolean", "number", "string"].includes(typeof value)) {
    return JSON.stringify(value);
  }
  fail("CANONICAL_VALUE_INVALID");
}

export function canonicalBytes(value) {
  return Buffer.from(`${canonicalJson(value)}\n`);
}

export function digestObject(value) {
  return createHash("sha256").update(canonicalBytes(value)).digest("hex");
}

function resolveRepositoryFile(root, relativePath) {
  const path = resolve(root, relativePath);
  const fromRoot = relative(root, path);
  if (!fromRoot || fromRoot.startsWith("..") || isAbsolute(fromRoot)) fail("ADAPTER_PATH_INVALID");
  return path;
}

function readJsonFile(root, relativePath, canonical) {
  const path = resolveRepositoryFile(root, relativePath);
  let metadata;
  let raw;
  let value;
  try {
    metadata = lstatSync(path);
    raw = readFileSync(path);
    value = JSON.parse(raw.toString("utf8"));
  } catch {
    fail("ADAPTER_FILE_INVALID");
  }
  if (!metadata.isFile() || metadata.isSymbolicLink() || !isRecord(value)) fail("ADAPTER_FILE_INVALID");
  if (canonical && !raw.equals(canonicalBytes(value))) fail("ADAPTER_JSON_NOT_CANONICAL");
  return value;
}

function validatePin(pin) {
  assertExactKeys(pin, ["assetSha256", "bundleVersion", "executionBundleDigest", "releaseSourceSha"], "PIN_FIELDS_INVALID");
  if (
    pin.bundleVersion !== BUNDLE_VERSION ||
    pin.releaseSourceSha !== RELEASE_SOURCE_SHA ||
    pin.executionBundleDigest !== EXECUTION_BUNDLE_DIGEST ||
    pin.assetSha256 !== ASSET_SHA256 ||
    !SHA1.test(pin.releaseSourceSha) ||
    !SHA256.test(pin.executionBundleDigest) ||
    !SHA256.test(pin.assetSha256)
  ) {
    fail("PIN_INVALID");
  }
}

function validateCapabilities(capabilities) {
  assertExactKeys(capabilities, ["artifacts", "checks", "diff", "pullRequest"], "ADAPTER_CAPABILITIES_INVALID");
  if (Object.values(capabilities).some((value) => value !== true)) fail("ADAPTER_CAPABILITIES_INVALID");
}

function validateRepository(repository) {
  assertExactKeys(
    repository,
    ["capabilities", "defaultBranch", "id", "identity", "policyPath", "provider"],
    "ADAPTER_REPOSITORY_INVALID"
  );
  if (
    repository.id !== REPOSITORY_ID ||
    repository.identity !== REPOSITORY_IDENTITY ||
    repository.provider !== "github" ||
    repository.defaultBranch !== "main" ||
    repository.policyPath !== POLICY_PATH_VALUE ||
    !IDENTIFIER.test(repository.id) ||
    !IDENTITY.test(repository.identity) ||
    !BRANCH.test(repository.defaultBranch) ||
    !POLICY_PATH.test(repository.policyPath)
  ) {
    fail("ADAPTER_REPOSITORY_INVALID");
  }
  validateCapabilities(repository.capabilities);
}

function validateAdapter(adapter) {
  assertExactKeys(adapter, ["adapterDigest", "adapterName", "repositories", "schemaVersion"], "ADAPTER_FIELDS_INVALID");
  if (
    adapter.schemaVersion !== 1 ||
    adapter.adapterName !== ADAPTER_NAME ||
    !IDENTIFIER.test(adapter.adapterName) ||
    !Array.isArray(adapter.repositories) ||
    adapter.repositories.length !== 1 ||
    !SHA256.test(adapter.adapterDigest)
  ) {
    fail("ADAPTER_INVALID");
  }
  validateRepository(adapter.repositories[0]);
  const { adapterDigest, ...withoutDigest } = adapter;
  if (adapterDigest !== digestObject(withoutDigest)) fail("ADAPTER_DIGEST_INVALID");
}

function validatePolicy(policy, repository) {
  if (
    !isRecord(policy) ||
    policy.provider !== repository.provider ||
    policy.repository !== `github.com/${repository.identity}` ||
    policy.baseBranch !== repository.defaultBranch
  ) {
    fail("POLICY_BINDING_INVALID");
  }
}

function validateResourceFactsPolicy(resourcePolicy, policy) {
  assertExactKeys(resourcePolicy, ["baseBranch", "policyDigest", "requiredChecks"], "RESOURCE_POLICY_FIELDS_INVALID");
  if (
    resourcePolicy.policyDigest !== digestObject(policy) ||
    resourcePolicy.baseBranch !== policy.baseBranch ||
    JSON.stringify(resourcePolicy.requiredChecks) !== JSON.stringify(policy.requiredChecks)
  ) {
    fail("RESOURCE_POLICY_BINDING_INVALID");
  }
}

function validateResourceFacts(resourceFacts, policy) {
  assertExactKeys(resourceFacts, ["billing", "policy", "resource", "runner", "schemaVersion"], "RESOURCE_FACTS_FIELDS_INVALID");
  if (resourceFacts.schemaVersion !== 1) fail("RESOURCE_FACTS_INVALID");

  assertExactKeys(resourceFacts.runner, ["kind", "source", "verified"], "RESOURCE_RUNNER_FIELDS_INVALID");
  if (
    resourceFacts.runner.kind !== RUNNER_KIND ||
    resourceFacts.runner.source !== RUNNER_SOURCE ||
    resourceFacts.runner.verified !== true
  ) {
    fail("RESOURCE_RUNNER_INVALID");
  }

  assertExactKeys(resourceFacts.resource, ["capacity", "verified"], "RESOURCE_RESOURCE_FIELDS_INVALID");
  if (resourceFacts.resource.capacity !== "unknown" || resourceFacts.resource.verified !== false) {
    fail("RESOURCE_RESOURCE_INVALID");
  }

  assertExactKeys(resourceFacts.billing, ["cost", "verified"], "RESOURCE_BILLING_FIELDS_INVALID");
  if (resourceFacts.billing.cost !== "unknown" || resourceFacts.billing.verified !== false) {
    fail("RESOURCE_BILLING_INVALID");
  }

  validateResourceFactsPolicy(resourceFacts.policy, policy);
}

function validateAdapterPolicyVerification(verification) {
  assertExactKeys(
    verification,
    ["adapterSmoke", "baseArgument", "baseShaPattern", "cloudOwned", "entrypoint", "zeroArtifact"],
    "ADAPTER_POLICY_VERIFICATION_FIELDS_INVALID"
  );
  assertExactKeys(verification.adapterSmoke, ["pathPrefixes", "paths"], "ADAPTER_POLICY_SMOKE_FIELDS_INVALID");
  assertExactValue(verification.adapterSmoke.pathPrefixes, [".gkd/"], "ADAPTER_POLICY_SMOKE_INVALID");
  assertExactValue(verification.adapterSmoke.paths, ADAPTER_SMOKE_PATHS, "ADAPTER_POLICY_SMOKE_INVALID");
  assertExactValue(verification.cloudOwned, CLOUD_OWNED, "ADAPTER_POLICY_CLOUD_BOUNDARY_INVALID");
  if (
    verification.entrypoint !== "scripts/gkd-verify" ||
    verification.baseArgument !== "--base-sha" ||
    verification.baseShaPattern !== "^[0-9a-f]{40}$" ||
    verification.zeroArtifact !== true
  ) {
    fail("ADAPTER_POLICY_VERIFICATION_INVALID");
  }
}

function validateAdapterPolicyCi(ci) {
  assertExactKeys(ci, ["artifacts", "cacheClasses", "runnerClasses"], "ADAPTER_POLICY_CI_FIELDS_INVALID");
  assertExactValue(ci.runnerClasses, RUNNER_CLASSES, "ADAPTER_POLICY_RUNNERS_INVALID");
  assertExactValue(ci.cacheClasses, CACHE_CLASSES, "ADAPTER_POLICY_CACHES_INVALID");
  if (!Array.isArray(ci.artifacts) || ci.artifacts.length !== ARTIFACT_CLASSES.length) {
    fail("ADAPTER_POLICY_ARTIFACTS_INVALID");
  }
  for (const artifact of ci.artifacts) {
    assertExactKeys(artifact, ["nameTemplate", "retentionDays"], "ADAPTER_POLICY_ARTIFACT_FIELDS_INVALID");
  }
  assertExactValue(ci.artifacts, ARTIFACT_CLASSES, "ADAPTER_POLICY_ARTIFACTS_INVALID");
}

function validateAdapterPolicyRelease(release) {
  assertExactKeys(
    release,
    ["candidate", "checksum", "existingRelease", "requireMainAncestor", "tagTemplate"],
    "ADAPTER_POLICY_RELEASE_FIELDS_INVALID"
  );
  assertExactKeys(
    release.candidate,
    ["artifactNameTemplate", "branch", "conclusion", "events", "repository", "sameSourceSha", "uniqueUnexpired", "workflow"],
    "ADAPTER_POLICY_CANDIDATE_FIELDS_INVALID"
  );
  assertExactValue(
    release.candidate,
    {
      artifactNameTemplate: "release-candidate-{sha}-{run_id}-{run_attempt}",
      branch: "main",
      conclusion: "success",
      events: ["push", "workflow_dispatch"],
      repository: REPOSITORY_IDENTITY,
      sameSourceSha: true,
      uniqueUnexpired: true,
      workflow: "ci.yml",
    },
    "ADAPTER_POLICY_CANDIDATE_INVALID"
  );
  assertExactKeys(release.checksum, ["algorithm", "coversAllOtherAssets", "manifest"], "ADAPTER_POLICY_CHECKSUM_FIELDS_INVALID");
  assertExactValue(
    release.checksum,
    { algorithm: "sha256", coversAllOtherAssets: true, manifest: "SHA256SUMS.txt" },
    "ADAPTER_POLICY_CHECKSUM_INVALID"
  );
  assertExactKeys(
    release.existingRelease,
    ["assetNamesAndChecksumsMustMatch", "overwrite"],
    "ADAPTER_POLICY_IMMUTABILITY_FIELDS_INVALID"
  );
  assertExactValue(
    release.existingRelease,
    { assetNamesAndChecksumsMustMatch: true, overwrite: false },
    "ADAPTER_POLICY_IMMUTABILITY_INVALID"
  );
  if (release.tagTemplate !== "aio-coding-hub-v{semver}") fail("ADAPTER_POLICY_TAG_INVALID");
  if (release.requireMainAncestor !== true) fail("ADAPTER_POLICY_MAIN_ANCESTRY_INVALID");
}

function validateAdapterPolicy(adapterPolicy) {
  assertExactKeys(adapterPolicy, ["ci", "release", "schemaVersion", "verification"], "ADAPTER_POLICY_FIELDS_INVALID");
  if (adapterPolicy.schemaVersion !== 1) fail("ADAPTER_POLICY_INVALID");
  validateAdapterPolicyVerification(adapterPolicy.verification);
  validateAdapterPolicyCi(adapterPolicy.ci);
  validateAdapterPolicyRelease(adapterPolicy.release);
}

export function validateHistoryAdapter(historyAdapter) {
  assertExactKeys(
    historyAdapter,
    ["active", "archive", "manifestName", "schemaVersion"],
    "HISTORY_ADAPTER_FIELDS_INVALID"
  );
  if (historyAdapter.schemaVersion !== 1 || historyAdapter.manifestName !== "task.json") {
    fail("HISTORY_ADAPTER_INVALID");
  }

  assertExactKeys(
    historyAdapter.active,
    ["coordinationVersionsRejected", "location", "requiredCount", "root", "worktreePath"],
    "HISTORY_ADAPTER_ACTIVE_FIELDS_INVALID"
  );
  assertExactValue(
    historyAdapter.active,
    {
      coordinationVersionsRejected: [1],
      location: "tracked-immediate-child",
      requiredCount: 1,
      root: ".trellis/tasks",
      worktreePath: "must-be-null",
    },
    "HISTORY_ADAPTER_ACTIVE_INVALID"
  );

  assertExactKeys(
    historyAdapter.archive,
    ["location", "requiredStatus", "root", "worktreePath"],
    "HISTORY_ADAPTER_ARCHIVE_FIELDS_INVALID"
  );
  assertExactValue(
    historyAdapter.archive,
    {
      location: "tracked-descendants",
      requiredStatus: "completed",
      root: ".trellis/tasks/archive",
      worktreePath: "ignored",
    },
    "HISTORY_ADAPTER_ARCHIVE_INVALID"
  );
}

export function readHistoryAdapter(root = repoRoot) {
  const historyAdapter = readJsonFile(root, HISTORY_ADAPTER_PATH, true);
  validateHistoryAdapter(historyAdapter);
  return historyAdapter;
}

export function verifyAdapter(root = repoRoot) {
  const pin = readJsonFile(root, ".gkd/bundle-pin.json", true);
  const adapter = readJsonFile(root, ".gkd/review-adapter.json", true);
  const policy = readJsonFile(root, adapter.repositories[0].policyPath, false);
  const adapterPolicy = readJsonFile(root, ADAPTER_POLICY_PATH, true);
  const resourceFacts = readJsonFile(root, RESOURCE_FACTS_PATH, true);
  readHistoryAdapter(root);
  validatePin(pin);
  validateAdapter(adapter);
  validatePolicy(policy, adapter.repositories[0]);
  validateAdapterPolicy(adapterPolicy);
  validateResourceFacts(resourceFacts, policy);
  return {
    outcome: "adapter_ready",
    adapterDigest: adapter.adapterDigest,
    bundleVersion: pin.bundleVersion,
  };
}

if (process.argv[1] && resolve(process.argv[1]) === modulePath) {
  try {
    console.log(JSON.stringify(verifyAdapter()));
  } catch (error) {
    console.error(JSON.stringify({ outcome: "adapter_failure", reason: error instanceof AdapterError ? error.message : "ADAPTER_FAILURE" }));
    process.exit(1);
  }
}
