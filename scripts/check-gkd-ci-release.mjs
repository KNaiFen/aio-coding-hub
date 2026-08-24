import { createHash } from "node:crypto";
import { lstatSync, readFileSync } from "node:fs";
import { dirname, isAbsolute, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const modulePath = fileURLToPath(import.meta.url);
const repoRoot = dirname(dirname(modulePath));
const SHA256 = /^[0-9a-f]{64}$/;
const REQUIRED_CHECKS = ["ci-gate", "pr-title"];
const CLOUD_JOBS = ["contracts", "frontend", "rust", "candidate-plan"];
const ARTIFACTS = [
  { nameTemplate: "cloud-native-fixes-{sha}-{run_attempt}", retentionDays: 7 },
  { nameTemplate: "dev-build-{target_id}-{sha}", retentionDays: 7 },
  { nameTemplate: "release-candidate-{sha}-{run_id}-{run_attempt}", retentionDays: 30 },
  { nameTemplate: "release-platform-{sha}-{run_attempt}-{target_id}", retentionDays: 1 },
  { nameTemplate: "tui-platform-{sha}-{run_attempt}-{target_id}", retentionDays: 1 },
];
const LOCAL_PATH = /(?:^|[\s"'=])\/(?:Users|private|home|tmp|var)\//;
const CREDENTIAL = /(?:-----BEGIN [A-Z ]+ PRIVATE KEY-----|(?:ghp|github_pat|sk|xox[baprs])_[A-Za-z0-9_-]{12,}|AKIA[0-9A-Z]{16})/;

export class CiReleaseError extends Error {}

function fail(code) {
  throw new CiReleaseError(code);
}

function isRecord(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function exact(value, keys, code) {
  if (!isRecord(value) || Object.keys(value).length !== keys.length || !keys.every((key) => key in value)) {
    fail(code);
  }
}

function canonical(value) {
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  if (isRecord(value)) {
    return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${canonical(value[key])}`).join(",")}}`;
  }
  if (value === null || ["boolean", "number", "string"].includes(typeof value)) return JSON.stringify(value);
  fail("DECLARATION_VALUE_INVALID");
}

function digest(value) {
  return createHash("sha256").update(`${canonical(value)}\n`).digest("hex");
}

function file(root, path) {
  const absolute = resolve(root, path);
  const fromRoot = relative(root, absolute);
  if (!fromRoot || fromRoot.startsWith("..") || isAbsolute(fromRoot)) fail("PATH_INVALID");
  let metadata;
  try { metadata = lstatSync(absolute); } catch { fail("FILE_INVALID"); }
  if (!metadata.isFile() || metadata.isSymbolicLink()) fail("FILE_INVALID");
  return absolute;
}

function json(root, path) {
  const raw = readFileSync(file(root, path));
  let value;
  try { value = JSON.parse(raw.toString("utf8")); } catch { fail("JSON_INVALID"); }
  if (!raw.equals(Buffer.from(`${canonical(value)}\n`))) fail("JSON_NOT_CANONICAL");
  return value;
}

function checkDeclaration(declaration, policy, resource, adapterPolicy) {
  exact(declaration, ["artifactBounds", "checks", "leakScan", "recommendation", "release", "schemaVersion"], "DECLARATION_FIELDS_INVALID");
  if (declaration.schemaVersion !== 1) fail("DECLARATION_SCHEMA_INVALID");
  exact(declaration.recommendation, ["billing", "digest", "goal", "preset", "resource", "runner", "runnerAction", "visibility"], "RECOMMENDATION_FIELDS_INVALID");
  if (declaration.recommendation.digest !== "8ec8e8b4bb28490ea4fa238828c5d012b9ec7f48a951b51189d084a9a0a2104e" || declaration.recommendation.goal !== "speed-first" || declaration.recommendation.preset !== "resource-constrained" || declaration.recommendation.runnerAction !== "retain-current-verified-runner" || declaration.recommendation.visibility !== "public") fail("RECOMMENDATION_INVALID");
  exact(declaration.recommendation.billing, ["source", "status"], "BILLING_FIELDS_INVALID");
  if (declaration.recommendation.billing.source !== "unknown" || declaration.recommendation.billing.status !== "unverified") fail("BILLING_INVALID");
  exact(declaration.recommendation.resource, ["capacity", "verified"], "RECOMMENDATION_RESOURCE_FIELDS_INVALID");
  if (declaration.recommendation.resource.capacity !== "unknown" || declaration.recommendation.resource.verified !== false) fail("RECOMMENDATION_RESOURCE_INVALID");
  exact(declaration.recommendation.runner, ["capacity", "kind", "os", "provider", "verified"], "RECOMMENDATION_RUNNER_FIELDS_INVALID");
  if (JSON.stringify(declaration.recommendation.runner) !== JSON.stringify({ capacity: "unknown", kind: "github-hosted", os: "linux", provider: "github", verified: true })) fail("RECOMMENDATION_RUNNER_INVALID");
  if (declaration.recommendation.digest !== "8ec8e8b4bb28490ea4fa238828c5d012b9ec7f48a951b51189d084a9a0a2104e") fail("RECOMMENDATION_DIGEST_INVALID");
  if (resource?.resource?.capacity !== "unknown" || resource?.resource?.verified !== false || resource?.billing?.cost !== "unknown" || resource?.billing?.verified !== false) fail("RESOURCE_FACTS_INVALID");
  if (resource?.policy?.policyDigest !== digest(policy)) fail("RESOURCE_POLICY_INVALID");
  exact(declaration.artifactBounds, ["artifacts", "cacheClasses", "maxRetentionDays"], "ARTIFACT_BOUNDS_FIELDS_INVALID");
  if (JSON.stringify(declaration.artifactBounds.artifacts) !== JSON.stringify(ARTIFACTS) || JSON.stringify(declaration.artifactBounds.cacheClasses) !== JSON.stringify(adapterPolicy.ci.cacheClasses) || declaration.artifactBounds.maxRetentionDays !== 30 || declaration.artifactBounds.artifacts.some((item) => item.retentionDays > 30)) fail("ARTIFACT_BOUNDS_INVALID");
  exact(declaration.checks, ["cloudJobs", "failClosed", "gateJob", "independentGroups", "localCommands", "requiredChecks"], "CHECKS_FIELDS_INVALID");
  if (JSON.stringify(declaration.checks.requiredChecks) !== JSON.stringify(REQUIRED_CHECKS) || declaration.checks.gateJob !== "ci-gate" || JSON.stringify(declaration.checks.localCommands) !== JSON.stringify(["node scripts/check-gkd-ci-release.selftest.mjs", "node scripts/check-gkd-ci-release.mjs", "git diff --check"])) fail("CHECKS_INVALID");
  exact(declaration.checks.failClosed, ["gateIf", "unexpectedSkip"], "FAIL_CLOSED_FIELDS_INVALID");
  if (declaration.checks.failClosed.gateIf !== "always()" || declaration.checks.failClosed.unexpectedSkip !== "error") fail("FAIL_CLOSED_INVALID");
  if (JSON.stringify(declaration.checks.cloudJobs) !== JSON.stringify({ candidatePlan: "candidate-plan", contracts: "contracts", frontend: "frontend", rust: "rust" })) fail("CLOUD_JOBS_INVALID");
  if (JSON.stringify(declaration.checks.independentGroups) !== JSON.stringify([["contracts", "frontend", "rust", "candidate-plan"], ["build-release-candidate", "build-tui-release-candidate"]])) fail("INDEPENDENT_GROUPS_INVALID");
  exact(declaration.leakScan, ["codes", "redacted", "scope"], "LEAK_SCAN_FIELDS_INVALID");
  if (JSON.stringify(declaration.leakScan.codes) !== JSON.stringify(["CREDENTIAL_SHAPED", "MACHINE_LOCAL_PATH"]) || declaration.leakScan.redacted !== true || JSON.stringify(declaration.leakScan.scope) !== JSON.stringify([".gkd/ci-release-adapter.json", ".github/workflows/ci.yml", ".github/workflows/release.yml"])) fail("LEAK_SCAN_INVALID");
  exact(declaration.release, ["candidate", "finalization", "noGithubWrite"], "RELEASE_FIELDS_INVALID");
  exact(declaration.release.candidate, ["artifactTemplate", "checksums", "sourceSha", "uniqueUnexpired", "workflow"], "CANDIDATE_FIELDS_INVALID");
  if (declaration.release.candidate.artifactTemplate !== "release-candidate-${{ github.sha }}-${{ github.run_id }}-${{ github.run_attempt }}" || declaration.release.candidate.checksums !== "SHA256SUMS.txt" || declaration.release.candidate.sourceSha !== "github.sha" || declaration.release.candidate.uniqueUnexpired !== true || declaration.release.candidate.workflow !== "ci.yml") fail("CANDIDATE_INVALID");
  exact(declaration.release.finalization, ["candidatePr", "finalizationPr", "publish", "requireReview", "sameSourceSha"], "FINALIZATION_FIELDS_INVALID");
  if (JSON.stringify(declaration.release.finalization) !== JSON.stringify({ candidatePr: true, finalizationPr: true, publish: false, requireReview: true, sameSourceSha: true }) || declaration.release.noGithubWrite !== true) fail("FINALIZATION_INVALID");
}

function job(workflow, name) {
  const match = new RegExp(`^  ${name}:\\n([\\s\\S]*?)(?=^  [A-Za-z0-9_-]+:|(?![\\s\\S]))`, "m").exec(workflow);
  return match?.[1] ?? "";
}

function needsValue(body) {
  const match = /(?:^|\n)\s+needs:\s*(.*)/.exec(body);
  if (!match) return "";
  return match[1].trim();
}

export function scanRedactedText(path, text) {
  const findings = [];
  if (CREDENTIAL.test(text)) findings.push({ code: "CREDENTIAL_SHAPED", path });
  if (LOCAL_PATH.test(text)) findings.push({ code: "MACHINE_LOCAL_PATH", path });
  return findings;
}

export function validateWorkflowSurface(workflows) {
  const { ci, prTitle, release } = workflows;
  for (const name of ["contracts", "frontend", "rust", "candidate-plan", "build-release-candidate", "build-tui-release-candidate", "assemble-release-candidate", "ci-gate"]) if (!job(ci, name)) fail(`WORKFLOW_JOB_MISSING_${name}`);
  if (!/if:\s*(?:>|\|)?\s*always\(\)/.test(job(ci, "ci-gate")) || !job(ci, "ci-gate").includes("set -euo pipefail") || !job(ci, "ci-gate").includes("[[ \"$CONTRACTS_RESULT\"")) fail("GATE_NOT_FAIL_CLOSED");
  for (const name of CLOUD_JOBS) if (/^\s*needs:\s*.*contracts/m.test(job(ci, name))) fail("CLOUD_GROUPS_NOT_INDEPENDENT");
  if (!job(ci, "ci-gate").includes("ci-gate") || !/name:\s*pr-title/.test(prTitle)) fail("REQUIRED_CHECKS_NOT_PROVEN");
  if (!ci.includes("release-candidate-${{ github.sha }}-${{ github.run_id }}-${{ github.run_attempt }}") || !ci.includes("retention-days: 30") || !ci.includes("retention-days: 1")) fail("ARTIFACT_DECLARATION_MISSING");
  if (!release.includes("run.head_sha === process.env.SOURCE_SHA") || !release.includes("selectReleaseCandidate(candidateRuns, process.env.SOURCE_SHA)") || !release.includes("SHA256SUMS.txt") || !release.includes("overwrite_files: false")) fail("RELEASE_SOURCE_BINDING_MISSING");
  return { cloudJobs: CLOUD_JOBS, gate: "fail-closed", requiredChecks: REQUIRED_CHECKS };
}

export function validateCiReleaseAdapter(root = repoRoot) {
  const declaration = json(root, ".gkd/ci-release-adapter.json");
  const policy = json(root, ".gkd/policy.json");
  const resource = json(root, ".gkd/resource-facts.json");
  const adapterPolicy = json(root, ".gkd/adapter-policy.json");
  if (JSON.stringify(policy.requiredChecks) !== JSON.stringify(REQUIRED_CHECKS)) fail("POLICY_REQUIRED_CHECKS_INVALID");
  checkDeclaration(declaration, policy, resource, adapterPolicy);
  const workflows = { ci: readFileSync(file(root, ".github/workflows/ci.yml"), "utf8"), prTitle: readFileSync(file(root, ".github/workflows/pr-title.yml"), "utf8"), release: readFileSync(file(root, ".github/workflows/release.yml"), "utf8") };
  validateWorkflowSurface(workflows);
  const findings = declaration.leakScan.scope.flatMap((path) => scanRedactedText(path, readFileSync(file(root, path), "utf8")));
  if (findings.length > 0) fail(`REDACTED_LEAK_${findings.map((item) => `${item.code}:${item.path}`).join(",")}`);
  return { outcome: "ci_release_ready", adapterDigest: digest(declaration), recommendationDigest: declaration.recommendation.digest, requiredChecks: REQUIRED_CHECKS, leakFindings: [] };
}

if (process.argv[1] && resolve(process.argv[1]) === modulePath) {
  try { console.log(JSON.stringify(validateCiReleaseAdapter())); }
  catch (error) { console.error(JSON.stringify({ outcome: "ci_release_failure", reason: String(error.message ?? error) })); process.exit(1); }
}
