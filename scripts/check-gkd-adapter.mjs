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

const BUNDLE_VERSION = "0.1.3";
const RELEASE_SOURCE_SHA = "2a63cd8ff2fcb7f0cb155dcc32578cda4b3381af";
const EXECUTION_BUNDLE_DIGEST = "cc465d26f08edb2a133775e4d6a58aa517eab1bde0ec2e1ec72f6d9f2c8883bd";
const ASSET_SHA256 = "9d9e6ea0fff64e0894af08a547b6798f1f6634e0e4cf4e174cd8dfc5c0179954";
const ADAPTER_NAME = "aio-gkd-review";
const REPOSITORY_ID = "aio-coding-hub";
const REPOSITORY_IDENTITY = "KNaiFen/aio-coding-hub";
const POLICY_PATH_VALUE = ".gkd/policy.json";

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

export function verifyAdapter(root = repoRoot) {
  const pin = readJsonFile(root, ".gkd/bundle-pin.json", true);
  const adapter = readJsonFile(root, ".gkd/review-adapter.json", true);
  validatePin(pin);
  validateAdapter(adapter);
  validatePolicy(readJsonFile(root, adapter.repositories[0].policyPath, false), adapter.repositories[0]);
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
