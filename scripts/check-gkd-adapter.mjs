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
const RESOURCE_FACTS_PATH = ".gkd/resource-facts.json";
const RUNNER_KIND = "github-hosted-linux";
const RUNNER_SOURCE = "github-actions-workflow";

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

export function verifyAdapter(root = repoRoot) {
  const pin = readJsonFile(root, ".gkd/bundle-pin.json", true);
  const adapter = readJsonFile(root, ".gkd/review-adapter.json", true);
  const policy = readJsonFile(root, adapter.repositories[0].policyPath, false);
  const resourceFacts = readJsonFile(root, RESOURCE_FACTS_PATH, true);
  validatePin(pin);
  validateAdapter(adapter);
  validatePolicy(policy, adapter.repositories[0]);
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
