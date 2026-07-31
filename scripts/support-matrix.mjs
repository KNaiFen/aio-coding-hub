import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import {
  copyFileSync,
  existsSync,
  lstatSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const logger = {
  info(message, ...args) {
    console.error(message, ...args);
  },
  error(message, ...args) {
    console.error(message, ...args);
  },
};

const scriptPath = fileURLToPath(import.meta.url);
const scriptDir = dirname(scriptPath);
const repoRoot = dirname(scriptDir);

export const CANONICAL_REPOSITORY = "KNaiFen/aio-coding-hub";
export const RELEASE_CANDIDATE_MANIFEST = "release-candidate-manifest.json";
export const RELEASE_CANDIDATE_SCHEMA_VERSION = 1;
export const FORK_RELEASE_TARGET_IDS = Object.freeze(["windows-x64", "macos-arm64"]);

export const CLOUD_BUILD_TARGETS = Object.freeze([
  Object.freeze({
    id: "windows-x64",
    osFamily: "windows",
    runner: "windows-latest",
    tauriTarget: "x86_64-pc-windows-msvc",
    rustupTargets: Object.freeze(["x86_64-pc-windows-msvc"]),
    devBundles: "msi",
    releaseCandidate: true,
    releaseBundles: "msi",
    updaterPlatform: "windows-x86_64",
    stableLabel: "win64",
    stableAssetKind: "msi",
    latestAssetName: "aio-coding-hub-win64.msi",
    latestSignatureName: "aio-coding-hub-win64.msi.sig",
    portableAssetName: "aio-coding-hub-win64-portable.zip",
    buildLabel: { zh: "Windows x64", en: "Windows x64" },
    releaseDownloadPackages: { zh: "`.msi` / `-portable.zip`", en: "`.msi` / `-portable.zip`" },
    cloudBuildNote: {
      zh: "`main` CI 生成签名候选；手动工作流生成无签名开发制品",
      en: "Signed candidate from `main` CI; unsigned development artifact from the manual workflow",
    },
  }),
  Object.freeze({
    id: "macos-x64",
    osFamily: "macos",
    runner: "macos-latest",
    tauriTarget: "x86_64-apple-darwin",
    rustupTargets: Object.freeze(["x86_64-apple-darwin"]),
    devBundles: "dmg",
    releaseCandidate: false,
    releaseBundles: "app",
    updaterPlatform: "darwin-x86_64",
    stableLabel: "macos-intel",
    stableAssetKind: "tarball",
    latestAssetName: "aio-coding-hub-macos-intel.tar.gz",
    latestSignatureName: "aio-coding-hub-macos-intel.tar.gz.sig",
    portableAssetName: "aio-coding-hub-macos-intel.zip",
    buildLabel: { zh: "macOS Intel", en: "macOS Intel" },
    releaseDownloadPackages: { zh: "`.zip`", en: "`.zip`" },
    cloudBuildNote: {
      zh: "手动工作流生成无签名开发制品；不进入 Release / updater 矩阵",
      en: "Unsigned development artifact from the manual workflow; excluded from Release/updater",
    },
  }),
  Object.freeze({
    id: "macos-arm64",
    osFamily: "macos",
    runner: "macos-latest",
    tauriTarget: "aarch64-apple-darwin",
    rustupTargets: Object.freeze(["aarch64-apple-darwin"]),
    devBundles: "dmg",
    releaseCandidate: true,
    releaseBundles: "app",
    updaterPlatform: "darwin-aarch64",
    stableLabel: "macos-arm",
    stableAssetKind: "tarball",
    latestAssetName: "aio-coding-hub-macos-arm.tar.gz",
    latestSignatureName: "aio-coding-hub-macos-arm.tar.gz.sig",
    portableAssetName: "aio-coding-hub-macos-arm.zip",
    buildLabel: { zh: "macOS Apple Silicon", en: "macOS Apple Silicon" },
    releaseDownloadPackages: { zh: "`.zip`", en: "`.zip`" },
    cloudBuildNote: {
      zh: "`main` CI 生成签名候选；手动工作流生成无签名开发制品",
      en: "Signed candidate from `main` CI; unsigned development artifact from the manual workflow",
    },
  }),
  Object.freeze({
    id: "linux-x64",
    osFamily: "linux",
    runner: "ubuntu-22.04",
    tauriTarget: "x86_64-unknown-linux-gnu",
    rustupTargets: Object.freeze(["x86_64-unknown-linux-gnu"]),
    devBundles: "deb,appimage",
    releaseCandidate: false,
    releaseBundles: "deb,appimage",
    updaterPlatform: "linux-x86_64",
    stableLabel: "linux-amd64",
    stableAssetKind: "appimage",
    latestAssetName: "aio-coding-hub-linux-amd64.AppImage",
    latestSignatureName: "aio-coding-hub-linux-amd64.AppImage.sig",
    portableAssetName: null,
    buildLabel: { zh: "Linux x64", en: "Linux x64" },
    releaseDownloadPackages: {
      zh: "`.deb` / `.AppImage`",
      en: "`.deb` / `.AppImage`",
    },
    cloudBuildNote: {
      zh: "手动工作流生成无签名开发制品；不进入 Release / updater 矩阵",
      en: "Unsigned development artifact from the manual workflow; excluded from Release/updater",
    },
  }),
  Object.freeze({
    id: "macos-universal",
    osFamily: "macos",
    runner: "macos-latest",
    tauriTarget: "universal-apple-darwin",
    rustupTargets: Object.freeze(["aarch64-apple-darwin", "x86_64-apple-darwin"]),
    devBundles: "dmg",
    releaseCandidate: false,
    releaseBundles: null,
    updaterPlatform: null,
    stableLabel: "macos-universal",
    stableAssetKind: null,
    latestAssetName: null,
    latestSignatureName: null,
    portableAssetName: null,
    buildLabel: { zh: "macOS Universal", en: "macOS Universal" },
    releaseDownloadPackages: null,
    cloudBuildNote: {
      zh: "手动工作流生成无签名开发制品；不进入 Release / updater 矩阵",
      en: "Unsigned development artifact from the manual workflow; excluded from Release/updater",
    },
  }),
  Object.freeze({
    id: "windows-arm64",
    osFamily: "windows",
    runner: "windows-latest",
    tauriTarget: "aarch64-pc-windows-msvc",
    rustupTargets: Object.freeze(["aarch64-pc-windows-msvc"]),
    devBundles: "msi",
    releaseCandidate: false,
    releaseBundles: null,
    updaterPlatform: null,
    stableLabel: "win-arm64",
    stableAssetKind: null,
    latestAssetName: null,
    latestSignatureName: null,
    portableAssetName: null,
    buildLabel: { zh: "Windows ARM64", en: "Windows ARM64" },
    releaseDownloadPackages: null,
    cloudBuildNote: {
      zh: "手动工作流生成无签名开发制品；不进入 Release / updater 矩阵",
      en: "Unsigned development artifact from the manual workflow; excluded from Release/updater",
    },
  }),
]);

const README_MARKERS = Object.freeze({
  releaseDownload: {
    start: "<!-- SUPPORT_MATRIX_RELEASE_DOWNLOAD:START -->",
    end: "<!-- SUPPORT_MATRIX_RELEASE_DOWNLOAD:END -->",
  },
  sourceBuild: {
    start: "<!-- SUPPORT_MATRIX_SOURCE_BUILD:START -->",
    end: "<!-- SUPPORT_MATRIX_SOURCE_BUILD:END -->",
  },
});

const README_LOCALES = Object.freeze([
  { fileName: "README.md", locale: "zh" },
  { fileName: "README_EN.md", locale: "en" },
]);

const WORKFLOW_DIRECTORY = join(repoRoot, ".github/workflows");
const WORKFLOW_PATHS = Object.freeze({
  ci: join(repoRoot, ".github/workflows/ci.yml"),
  devBuild: join(repoRoot, ".github/workflows/dev-build.yml"),
  release: join(repoRoot, ".github/workflows/release.yml"),
  releasePrSyncCargoLock: join(repoRoot, ".github/workflows/release-pr-sync-cargo-lock.yml"),
});

const VERSION_FILES = Object.freeze([
  "package.json",
  "src-tauri/Cargo.toml",
  "src-tauri/Cargo.lock",
  "src-tauri/tauri.conf.json",
]);

const EXPECTED_DESKTOP_OS_FAMILIES = Object.freeze(["linux", "macos", "windows"]);
const SEMVER_PATTERN =
  /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/;
const SHA_PATTERN = /^[0-9a-f]{40}$/;
const SHA256_PATTERN = /^[0-9a-f]{64}$/;

const HOMEBREW_CASK = Object.freeze({
  token: "aio-coding-hub",
  appName: "AIO Coding Hub.app",
  name: "AIO Coding Hub",
  desc: "Local AI CLI unified gateway",
  homepage: "https://github.com/KNaiFen/aio-coding-hub",
  bundleIdentifier: "io.aio.codinghub",
});

function parseArgs(rawArgs) {
  const args = new Map();

  for (let index = 0; index < rawArgs.length; index += 1) {
    const token = rawArgs[index];
    if (!token.startsWith("--")) {
      throw new Error(`Unexpected argument: ${token}`);
    }

    const key = token.slice(2);
    if (args.has(key)) {
      throw new Error(`Duplicate argument: ${token}`);
    }

    const value = rawArgs[index + 1];
    if (value == null || value.startsWith("--")) {
      throw new Error(`Missing value for argument: ${token}`);
    }

    args.set(key, value);
    index += 1;
  }

  return args;
}

function assertOnlyArgs(args, allowed) {
  const allowedSet = new Set(allowed);
  for (const key of args.keys()) {
    if (!allowedSet.has(key)) {
      throw new Error(`Unknown argument: --${key}`);
    }
  }
}

function requireArg(args, key) {
  const value = args.get(key);
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`Missing required argument: --${key}`);
  }
  return value;
}

function parseBoolean(value, label) {
  if (value === "true") return true;
  if (value === "false") return false;
  throw new Error(`${label} must be the literal true or false.`);
}

function parsePositiveInteger(value, label) {
  if (!/^[1-9]\d*$/.test(String(value))) {
    throw new Error(`${label} must be a positive integer: ${value}`);
  }
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed)) {
    throw new Error(`${label} exceeds the safe integer range: ${value}`);
  }
  return parsed;
}

export function assertCanonicalSha(value, label = "SHA") {
  if (typeof value !== "string" || !SHA_PATTERN.test(value)) {
    throw new Error(`${label} must be a lowercase full 40-hex commit SHA: ${value}`);
  }
  return value;
}

export function assertCanonicalVersion(value, label = "version") {
  if (typeof value !== "string" || !SEMVER_PATTERN.test(value)) {
    throw new Error(`${label} is not canonical SemVer: ${value}`);
  }
  return value;
}

export function deriveReleaseTag(version) {
  return `aio-coding-hub-v${assertCanonicalVersion(version)}`;
}

export function assertCanonicalTag(tag, expectedVersion = null) {
  if (typeof tag !== "string" || !tag.startsWith("aio-coding-hub-v")) {
    throw new Error(`Invalid release tag: ${tag}. Expected aio-coding-hub-v<semver>.`);
  }
  const version = tag.slice("aio-coding-hub-v".length);
  assertCanonicalVersion(version, "release tag version");
  if (expectedVersion != null && version !== expectedVersion) {
    throw new Error(`Release tag/version mismatch: tag=${version}, version=${expectedVersion}`);
  }
  return tag;
}

function assertCanonicalRepository(repository) {
  if (typeof repository !== "string" || !/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(repository)) {
    throw new Error(`Invalid repository slug: ${repository}`);
  }
  return repository;
}

function parseCargoPackageVersion(cargoToml) {
  const packageSection = /^\[package\]\s*$([\s\S]*?)(?=^\[|(?![\s\S]))/m.exec(cargoToml)?.[1];
  const version = packageSection ? /^version\s*=\s*"([^"]+)"\s*$/m.exec(packageSection)?.[1] : null;
  if (!version) {
    throw new Error("src-tauri/Cargo.toml is missing [package].version.");
  }
  return version;
}

function parseCargoLockRootVersion(cargoLock) {
  const blocks = cargoLock.split(/(?=^\[\[package\]\]\s*$)/m);
  const matches = blocks.filter(
    (block) =>
      /^\[\[package\]\]\s*$/m.test(block) && /^name\s*=\s*"aio-coding-hub"\s*$/m.test(block)
  );
  if (matches.length !== 1) {
    throw new Error(
      `src-tauri/Cargo.lock must contain exactly one aio-coding-hub package entry; found ${matches.length}.`
    );
  }
  const version = /^version\s*=\s*"([^"]+)"\s*$/m.exec(matches[0])?.[1];
  if (!version) {
    throw new Error("src-tauri/Cargo.lock root package is missing version.");
  }
  return version;
}

function readVersionFromContents(contents) {
  const packageVersion = JSON.parse(contents["package.json"]).version;
  const cargoTomlVersion = parseCargoPackageVersion(contents["src-tauri/Cargo.toml"]);
  const cargoLockVersion = parseCargoLockRootVersion(contents["src-tauri/Cargo.lock"]);
  const tauriVersion = JSON.parse(contents["src-tauri/tauri.conf.json"]).version;
  const sources = Object.freeze({
    "package.json": packageVersion,
    "src-tauri/Cargo.toml": cargoTomlVersion,
    "src-tauri/Cargo.lock": cargoLockVersion,
    "src-tauri/tauri.conf.json": tauriVersion,
  });
  const versions = Object.values(sources);
  for (const [fileName, version] of Object.entries(sources)) {
    assertCanonicalVersion(version, `${fileName} version`);
  }
  if (new Set(versions).size !== 1) {
    throw new Error(
      `Application version drifted: ${Object.entries(sources)
        .map(([fileName, version]) => `${fileName}=${version}`)
        .join(", ")}`
    );
  }
  const version = versions[0];
  return Object.freeze({ version, tag: deriveReleaseTag(version), sources });
}

export function readSynchronizedApplicationVersion(root = repoRoot) {
  const contents = Object.fromEntries(
    VERSION_FILES.map((fileName) => [fileName, readFileSync(join(root, fileName), "utf8")])
  );
  return readVersionFromContents(contents);
}

export function readSynchronizedApplicationVersionAtRevision(revision, root = repoRoot) {
  assertCanonicalSha(revision, "Git revision");
  const contents = Object.fromEntries(
    VERSION_FILES.map((fileName) => [
      fileName,
      execFileSync("git", ["show", `${revision}:${fileName}`], {
        cwd: root,
        encoding: "utf8",
        maxBuffer: 16 * 1024 * 1024,
      }),
    ])
  );
  return readVersionFromContents(contents);
}

function findTarget(targetId) {
  return CLOUD_BUILD_TARGETS.find((target) => target.id === targetId) ?? null;
}

function parseTargetIds(rawTargetIds, defaultIds) {
  const targetIds =
    rawTargetIds == null
      ? [...defaultIds]
      : rawTargetIds
          .split(",")
          .map((value) => value.trim())
          .filter(Boolean);
  if (targetIds.length === 0 || new Set(targetIds).size !== targetIds.length) {
    throw new Error(`Target ids must be a non-empty unique list: ${rawTargetIds ?? "<default>"}`);
  }
  return targetIds;
}

function selectReleaseTargets(rawTargetIds = null) {
  const targetIds = parseTargetIds(rawTargetIds, FORK_RELEASE_TARGET_IDS);
  return targetIds.map((targetId) => {
    const target = findTarget(targetId);
    if (!target || !target.releaseCandidate || !FORK_RELEASE_TARGET_IDS.includes(targetId)) {
      throw new Error(
        `Unsupported release target id: ${targetId}. Expected: ${FORK_RELEASE_TARGET_IDS.join(", ")}`
      );
    }
    return target;
  });
}

function selectManualTargets(rawTargetId = null) {
  if (rawTargetId == null) return [...CLOUD_BUILD_TARGETS];
  const target = findTarget(rawTargetId);
  if (!target) {
    throw new Error(
      `Unsupported development target id: ${rawTargetId}. Expected: ${CLOUD_BUILD_TARGETS.map((item) => item.id).join(", ")}`
    );
  }
  return [target];
}

export function buildReleaseMatrix(targets = selectReleaseTargets()) {
  return targets.map((target) => ({
    target_id: target.id,
    runner: target.runner,
    target: target.tauriTarget,
    rustup_targets: target.rustupTargets.join(","),
    bundles: target.releaseBundles,
    updater_platform: target.updaterPlatform,
    stable_label: target.stableLabel,
    portable_asset_name: target.portableAssetName,
  }));
}

export function buildManualCloudMatrix(targets = CLOUD_BUILD_TARGETS) {
  return targets.map((target) => ({
    target_id: target.id,
    runner: target.runner,
    tauri_target: target.tauriTarget,
    rustup_targets: target.rustupTargets.join(","),
    bundles: target.devBundles,
    artifact_label: target.stableLabel,
  }));
}

function buildDesktopCiMatrix() {
  return EXPECTED_DESKTOP_OS_FAMILIES.map((osFamily) => {
    const target = CLOUD_BUILD_TARGETS.find((item) => item.osFamily === osFamily);
    if (!target) throw new Error(`Missing desktop target for ${osFamily}.`);
    return { os_family: osFamily, runner: target.runner };
  });
}

export function expectedCandidateFiles(targetIds = FORK_RELEASE_TARGET_IDS) {
  const specs = [];
  for (const target of selectReleaseTargets(targetIds.join(","))) {
    specs.push(
      { name: target.latestAssetName, targetId: target.id },
      { name: target.latestSignatureName, targetId: target.id },
      { name: target.portableAssetName, targetId: target.id }
    );
  }
  return specs.sort((left, right) => left.name.localeCompare(right.name));
}

function renderMarkdownTable(headers, rows) {
  const headerLine = `| ${headers.join(" | ")} |`;
  const separatorLine = `| ${headers.map(() => "---").join(" | ")} |`;
  const bodyLines = rows.map((row) => `| ${row.join(" | ")} |`);
  return [headerLine, separatorLine, ...bodyLines].join("\n");
}

function renderReadmeReleaseDownloadTable(locale) {
  const headers =
    locale === "zh" ? ["平台", "官方发布安装包"] : ["Platform", "Official release packages"];
  const rows = selectReleaseTargets().map((target) => [
    target.buildLabel[locale],
    target.releaseDownloadPackages[locale],
  ]);
  return renderMarkdownTable(headers, rows);
}

function renderReadmeSourceBuildTable(locale) {
  const headers =
    locale === "zh"
      ? ["分类", "云端工作流目标", "说明"]
      : ["Scope", "Cloud workflow target", "Notes"];
  const rows = CLOUD_BUILD_TARGETS.map((target) => [
    target.releaseCandidate
      ? locale === "zh"
        ? "正式发布 / 开发制品"
        : "Release / development"
      : locale === "zh"
        ? "开发制品"
        : "Development",
    `Actions \`dev-build\`: \`${target.id}\``,
    `${target.buildLabel[locale]}；${target.cloudBuildNote[locale]}`.replace(
      "；",
      locale === "zh" ? "；" : "; "
    ),
  ]);
  return renderMarkdownTable(headers, rows);
}

function renderReadmeBlock(section, locale) {
  const markers = README_MARKERS[section];
  const table =
    section === "releaseDownload"
      ? renderReadmeReleaseDownloadTable(locale)
      : renderReadmeSourceBuildTable(locale);
  return `${markers.start}\n${table}\n${markers.end}`;
}

export function planReleaseCandidate({
  eventName,
  eventRef,
  eventSha,
  beforeSha,
  repository,
  recoveryRequested,
  candidateSha,
  expectedTag,
  loadVersionAtRevision,
}) {
  if (eventRef !== "refs/heads/main") {
    return Object.freeze({ enabled: false, mode: "none" });
  }

  if (eventName === "push") {
    assertCanonicalRepository(repository);
    if (repository !== CANONICAL_REPOSITORY) {
      throw new Error(`Release candidates are restricted to ${CANONICAL_REPOSITORY}.`);
    }
    assertCanonicalSha(eventSha, "main push SHA");
    assertCanonicalSha(beforeSha, "main push before SHA");
    if (/^0{40}$/.test(beforeSha)) {
      throw new Error("A zero before SHA cannot authorize a release candidate.");
    }
    const sourceVersion = loadVersionAtRevision(eventSha);
    const beforeVersion = loadVersionAtRevision(beforeSha);
    if (sourceVersion.version === beforeVersion.version) {
      return Object.freeze({ enabled: false, mode: "none" });
    }
    return Object.freeze({
      enabled: true,
      mode: "main-push",
      sourceSha: eventSha,
      trustedControlSha: eventSha,
      version: sourceVersion.version,
      tag: sourceVersion.tag,
    });
  }

  if (eventName !== "workflow_dispatch" || recoveryRequested !== true) {
    return Object.freeze({ enabled: false, mode: "none" });
  }

  assertCanonicalRepository(repository);
  if (repository !== CANONICAL_REPOSITORY) {
    throw new Error(`Release candidate recovery is restricted to ${CANONICAL_REPOSITORY}.`);
  }
  assertCanonicalSha(eventSha, "trusted control SHA");
  assertCanonicalSha(candidateSha, "recovery candidate SHA");
  const sourceVersion = loadVersionAtRevision(candidateSha);
  assertCanonicalTag(expectedTag, sourceVersion.version);
  if (expectedTag !== sourceVersion.tag) {
    throw new Error(
      `Recovery tag mismatch: expected ${sourceVersion.tag}, received ${expectedTag}.`
    );
  }
  return Object.freeze({
    enabled: true,
    mode: "recovery",
    sourceSha: candidateSha,
    trustedControlSha: eventSha,
    version: sourceVersion.version,
    tag: sourceVersion.tag,
  });
}

function writeGithubOutput(outputPath, values) {
  const lines = Object.entries(values).map(([key, value]) => `${key}=${value ?? ""}`);
  writeFileSync(outputPath, `${lines.join("\n")}\n`, { encoding: "utf8", flag: "a" });
}

function runCandidatePlan(args) {
  assertOnlyArgs(args, [
    "event-name",
    "event-ref",
    "event-sha",
    "before-sha",
    "repository",
    "recovery-requested",
    "candidate-sha",
    "expected-tag",
    "github-output",
  ]);
  const recoveryRequested = parseBoolean(
    requireArg(args, "recovery-requested"),
    "--recovery-requested"
  );
  const plan = planReleaseCandidate({
    eventName: requireArg(args, "event-name"),
    eventRef: requireArg(args, "event-ref"),
    eventSha: requireArg(args, "event-sha"),
    beforeSha: args.get("before-sha") ?? "",
    repository: requireArg(args, "repository"),
    recoveryRequested,
    candidateSha: args.get("candidate-sha") ?? "",
    expectedTag: args.get("expected-tag") ?? "",
    loadVersionAtRevision: (revision) => readSynchronizedApplicationVersionAtRevision(revision),
  });
  const output = {
    should_build: String(plan.enabled),
    mode: plan.mode,
    source_sha: plan.sourceSha ?? "",
    trusted_control_sha: plan.trustedControlSha ?? "",
    version: plan.version ?? "",
    tag: plan.tag ?? "",
  };
  const githubOutput = args.get("github-output");
  if (githubOutput) {
    writeGithubOutput(githubOutput, output);
  } else {
    process.stdout.write(`${JSON.stringify(output)}\n`);
  }
}

function normalizeSha256(value, label) {
  const normalized = String(value)
    .replace(/^sha256:/, "")
    .toLowerCase();
  if (!SHA256_PATTERN.test(normalized)) {
    throw new Error(`Invalid SHA-256 for ${label}: ${value}`);
  }
  return normalized;
}

function hashFile(filePath) {
  return createHash("sha256").update(readFileSync(filePath)).digest("hex");
}

function assertSafeAssetName(name, label = "asset name") {
  if (
    typeof name !== "string" ||
    name.length === 0 ||
    name === "." ||
    name === ".." ||
    name.includes("/") ||
    name.includes("\\") ||
    name.includes("\0") ||
    basename(name) !== name
  ) {
    throw new Error(`Unsafe ${label}: ${name}`);
  }
  return name;
}

function assertRegularFile(filePath, label) {
  const entry = lstatSync(filePath);
  if (entry.isSymbolicLink() || !entry.isFile()) {
    throw new Error(`${label} must be a regular non-symlink file: ${filePath}`);
  }
  return entry;
}

function assertExactKeys(value, expectedKeys, label) {
  if (value == null || typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`${label} must be an object.`);
  }
  const actual = Object.keys(value).sort();
  const expected = [...expectedKeys].sort();
  if (actual.length !== expected.length || actual.some((key, index) => key !== expected[index])) {
    throw new Error(
      `${label} fields drifted. Expected ${expected.join(", ")}; got ${actual.join(", ")}.`
    );
  }
}

function assertDirectoryEntries(directory, expectedNames) {
  const actual = readdirSync(directory).sort();
  const expected = [...expectedNames].sort();
  if (actual.length !== expected.length || actual.some((name, index) => name !== expected[index])) {
    throw new Error(
      `Candidate directory entries drifted. Expected ${expected.join(", ")}; got ${actual.join(", ")}.`
    );
  }
}

export function createCandidateManifest(context, filesDirectory) {
  const repository = assertCanonicalRepository(context.repository);
  const sourceSha = assertCanonicalSha(context.sourceSha, "sourceSha");
  const trustedControlSha = assertCanonicalSha(context.trustedControlSha, "trustedControlSha");
  const sourceValidationRunId = parsePositiveInteger(
    context.sourceValidationRunId,
    "sourceValidationRunId"
  );
  const sourceValidationRunAttempt = parsePositiveInteger(
    context.sourceValidationRunAttempt,
    "sourceValidationRunAttempt"
  );
  const version = assertCanonicalVersion(context.version);
  const tag = assertCanonicalTag(context.tag, version);
  if (tag !== deriveReleaseTag(version)) {
    throw new Error(`Candidate tag must equal ${deriveReleaseTag(version)}.`);
  }
  const workflowRunId = parsePositiveInteger(context.workflowRunId, "workflowRunId");
  const workflowRunAttempt = parsePositiveInteger(context.workflowRunAttempt, "workflowRunAttempt");
  const targetIds = [...context.targetIds].sort();
  const expectedSpecs = expectedCandidateFiles(targetIds);
  assertDirectoryEntries(
    filesDirectory,
    expectedSpecs.map((item) => item.name)
  );

  const files = expectedSpecs.map((spec) => {
    assertSafeAssetName(spec.name);
    const path = join(filesDirectory, spec.name);
    const entry = assertRegularFile(path, `Candidate asset ${spec.name}`);
    if (entry.size <= 0) {
      throw new Error(`Candidate asset is empty: ${spec.name}`);
    }
    return {
      name: spec.name,
      targetId: spec.targetId,
      size: entry.size,
      sha256: hashFile(path),
    };
  });

  return {
    schemaVersion: RELEASE_CANDIDATE_SCHEMA_VERSION,
    repository,
    sourceSha,
    trustedControlSha,
    sourceValidationRunId,
    sourceValidationRunAttempt,
    version,
    tag,
    workflowRunId,
    workflowRunAttempt,
    targetIds,
    files,
  };
}

function assertExpectedContext(manifest, expectedContext) {
  const checks = [
    ["repository", expectedContext.repository],
    ["sourceSha", expectedContext.sourceSha],
    ["trustedControlSha", expectedContext.trustedControlSha],
    ["sourceValidationRunId", expectedContext.sourceValidationRunId],
    ["sourceValidationRunAttempt", expectedContext.sourceValidationRunAttempt],
    ["version", expectedContext.version],
    ["tag", expectedContext.tag],
    ["workflowRunId", expectedContext.workflowRunId],
    ["workflowRunAttempt", expectedContext.workflowRunAttempt],
  ];
  for (const [field, expected] of checks) {
    if (expected != null && String(manifest[field]) !== String(expected)) {
      throw new Error(`Candidate ${field} mismatch: expected ${expected}, got ${manifest[field]}.`);
    }
  }
}

export function verifyCandidateManifest(manifest, expectedContext, filesDirectory) {
  assertExactKeys(
    manifest,
    [
      "schemaVersion",
      "repository",
      "sourceSha",
      "trustedControlSha",
      "sourceValidationRunId",
      "sourceValidationRunAttempt",
      "version",
      "tag",
      "workflowRunId",
      "workflowRunAttempt",
      "targetIds",
      "files",
    ],
    "candidate manifest"
  );
  if (manifest.schemaVersion !== RELEASE_CANDIDATE_SCHEMA_VERSION) {
    throw new Error(`Unsupported candidate schemaVersion: ${manifest.schemaVersion}`);
  }
  assertCanonicalRepository(manifest.repository);
  assertCanonicalSha(manifest.sourceSha, "sourceSha");
  assertCanonicalSha(manifest.trustedControlSha, "trustedControlSha");
  parsePositiveInteger(manifest.sourceValidationRunId, "sourceValidationRunId");
  parsePositiveInteger(manifest.sourceValidationRunAttempt, "sourceValidationRunAttempt");
  assertCanonicalVersion(manifest.version);
  assertCanonicalTag(manifest.tag, manifest.version);
  if (manifest.tag !== deriveReleaseTag(manifest.version)) {
    throw new Error(`Candidate tag must equal ${deriveReleaseTag(manifest.version)}.`);
  }
  parsePositiveInteger(manifest.workflowRunId, "workflowRunId");
  parsePositiveInteger(manifest.workflowRunAttempt, "workflowRunAttempt");
  assertExpectedContext(manifest, expectedContext);

  if (!Array.isArray(manifest.targetIds) || manifest.targetIds.length === 0) {
    throw new Error("Candidate targetIds must be a non-empty array.");
  }
  const expectedTargetIds = [...expectedContext.targetIds].sort();
  const actualTargetIds = [...manifest.targetIds];
  if (
    new Set(actualTargetIds).size !== actualTargetIds.length ||
    actualTargetIds.some((targetId, index) => targetId !== [...actualTargetIds].sort()[index]) ||
    actualTargetIds.length !== expectedTargetIds.length ||
    actualTargetIds.some((targetId, index) => targetId !== expectedTargetIds[index])
  ) {
    throw new Error(
      `Candidate targetIds mismatch: expected ${expectedTargetIds.join(",")}, got ${actualTargetIds.join(",")}.`
    );
  }

  if (!Array.isArray(manifest.files)) {
    throw new Error("Candidate files must be an array.");
  }
  const expectedSpecs = expectedCandidateFiles(expectedTargetIds);
  const expectedByName = new Map(expectedSpecs.map((item) => [item.name, item]));
  const actualNames = manifest.files.map((file) => file?.name);
  if (
    new Set(actualNames).size !== actualNames.length ||
    actualNames.length !== expectedSpecs.length ||
    actualNames.some((name, index) => name !== expectedSpecs[index].name)
  ) {
    throw new Error(
      `Candidate file set/order mismatch. Expected ${expectedSpecs.map((item) => item.name).join(", ")}.`
    );
  }

  assertDirectoryEntries(filesDirectory, [
    RELEASE_CANDIDATE_MANIFEST,
    ...expectedSpecs.map((item) => item.name),
  ]);
  for (const file of manifest.files) {
    assertExactKeys(file, ["name", "targetId", "size", "sha256"], `candidate file ${file?.name}`);
    assertSafeAssetName(file.name);
    const expectedSpec = expectedByName.get(file.name);
    if (!expectedSpec || file.targetId !== expectedSpec.targetId) {
      throw new Error(`Candidate target mapping mismatch for ${file.name}: ${file.targetId}.`);
    }
    const expectedSize = parsePositiveInteger(file.size, `size for ${file.name}`);
    const expectedSha = normalizeSha256(file.sha256, file.name);
    if (file.sha256 !== expectedSha) {
      throw new Error(`Candidate SHA-256 must be lowercase without a prefix: ${file.name}.`);
    }
    const path = join(filesDirectory, file.name);
    const entry = assertRegularFile(path, `Candidate asset ${file.name}`);
    if (entry.size !== expectedSize) {
      throw new Error(
        `Candidate size mismatch for ${file.name}: expected ${expectedSize}, got ${entry.size}.`
      );
    }
    const actualSha = hashFile(path);
    if (actualSha !== expectedSha) {
      throw new Error(`Candidate SHA-256 mismatch for ${file.name}.`);
    }
  }
  assertRegularFile(join(filesDirectory, RELEASE_CANDIDATE_MANIFEST), "Candidate manifest");
  return manifest;
}

function candidateContextFromArgs(args, { allowOptionalProvenance = false } = {}) {
  const targetIds = parseTargetIds(requireArg(args, "target-ids"), FORK_RELEASE_TARGET_IDS);
  const context = {
    repository: requireArg(args, "repository"),
    sourceSha: requireArg(args, "source-sha"),
    trustedControlSha: allowOptionalProvenance
      ? args.get("trusted-control-sha")
      : requireArg(args, "trusted-control-sha"),
    sourceValidationRunId: allowOptionalProvenance
      ? args.get("source-validation-run-id")
      : requireArg(args, "source-validation-run-id"),
    sourceValidationRunAttempt: allowOptionalProvenance
      ? args.get("source-validation-run-attempt")
      : requireArg(args, "source-validation-run-attempt"),
    version: requireArg(args, "version"),
    tag: requireArg(args, "tag"),
    workflowRunId: requireArg(args, "workflow-run-id"),
    workflowRunAttempt: requireArg(args, "workflow-run-attempt"),
    targetIds,
  };
  return context;
}

const CANDIDATE_CONTEXT_ARGS = Object.freeze([
  "repository",
  "source-sha",
  "trusted-control-sha",
  "source-validation-run-id",
  "source-validation-run-attempt",
  "version",
  "tag",
  "workflow-run-id",
  "workflow-run-attempt",
  "target-ids",
]);

function writeCandidateManifest(args) {
  assertOnlyArgs(args, [...CANDIDATE_CONTEXT_ARGS, "assets-dir", "output"]);
  const assetsDir = requireArg(args, "assets-dir");
  const outputPath = requireArg(args, "output");
  const manifest = createCandidateManifest(candidateContextFromArgs(args), assetsDir);
  writeFileSync(outputPath, `${JSON.stringify(manifest, null, 2)}\n`, {
    encoding: "utf8",
    flag: "wx",
  });
  logger.info("[support-matrix] 候选清单已生成：%s", outputPath);
}

function verifyCandidateManifestCommand(args) {
  assertOnlyArgs(args, [...CANDIDATE_CONTEXT_ARGS, "candidate-dir", "github-output"]);
  const candidateDir = requireArg(args, "candidate-dir");
  const manifestPath = join(candidateDir, RELEASE_CANDIDATE_MANIFEST);
  assertRegularFile(manifestPath, "Candidate manifest");
  const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
  const verified = verifyCandidateManifest(
    manifest,
    candidateContextFromArgs(args, { allowOptionalProvenance: true }),
    candidateDir
  );
  const output = {
    trusted_control_sha: verified.trustedControlSha,
    source_validation_run_id: verified.sourceValidationRunId,
    source_validation_run_attempt: verified.sourceValidationRunAttempt,
  };
  const githubOutput = args.get("github-output");
  if (githubOutput) {
    writeGithubOutput(githubOutput, output);
  } else {
    process.stdout.write(`${JSON.stringify(verified)}\n`);
  }
}

function stageCandidateFiles(args) {
  assertOnlyArgs(args, ["candidate-dir", "output-dir"]);
  const candidateDir = requireArg(args, "candidate-dir");
  const outputDir = requireArg(args, "output-dir");
  const manifestPath = join(candidateDir, RELEASE_CANDIDATE_MANIFEST);
  assertRegularFile(manifestPath, "Candidate manifest");
  const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
  mkdirSync(outputDir, { recursive: true });
  if (readdirSync(outputDir).length !== 0) {
    throw new Error(`Release staging directory must be empty: ${outputDir}`);
  }
  for (const file of manifest.files ?? []) {
    assertSafeAssetName(file.name);
    const source = join(candidateDir, file.name);
    assertRegularFile(source, `Candidate asset ${file.name}`);
    copyFileSync(source, join(outputDir, file.name));
  }
}

function findOfficialTargetByUpdaterPlatform(updaterPlatform) {
  return CLOUD_BUILD_TARGETS.find((item) => item.updaterPlatform === updaterPlatform) ?? null;
}

function parseArtifactPaths(rawArtifactPaths) {
  let artifactPaths;
  try {
    artifactPaths = JSON.parse(rawArtifactPaths);
  } catch (error) {
    throw new Error(
      `Failed to parse --artifact-paths as JSON: ${error instanceof Error ? error.message : error}`
    );
  }
  if (!Array.isArray(artifactPaths) || artifactPaths.length === 0) {
    throw new Error("No artifacts found (artifactPaths is empty).");
  }
  if (artifactPaths.some((item) => typeof item !== "string" || item.length === 0)) {
    throw new Error("artifactPaths must contain non-empty strings only.");
  }
  return artifactPaths;
}

function pickSingleArtifact(artifactPaths, predicate, label) {
  const matches = artifactPaths.filter((item) => predicate(item));
  if (matches.length !== 1) {
    throw new Error(
      `Expected exactly one ${label}; found ${matches.length}.\nAvailable artifacts:\n${artifactPaths.map((item) => `- ${item}`).join("\n")}`
    );
  }
  return matches[0];
}

function copyArtifact(sourcePath, outputDir, outputName) {
  assertRegularFile(sourcePath, `Build artifact ${sourcePath}`);
  mkdirSync(outputDir, { recursive: true });
  const destinationPath = join(outputDir, assertSafeAssetName(outputName));
  if (existsSync(destinationPath)) {
    throw new Error(`Refusing to overwrite stable asset: ${destinationPath}`);
  }
  copyFileSync(sourcePath, destinationPath);
  logger.info("[support-matrix] 复制产物：%s -> %s", sourcePath, destinationPath);
}

function prepareStableAssets(args) {
  assertOnlyArgs(args, ["artifact-paths", "updater-platform", "stable-label", "output-dir"]);
  const artifactPaths = parseArtifactPaths(requireArg(args, "artifact-paths"));
  const updaterPlatform = requireArg(args, "updater-platform");
  const stableLabel = requireArg(args, "stable-label");
  const outputDir = requireArg(args, "output-dir");
  const target = findOfficialTargetByUpdaterPlatform(updaterPlatform);
  if (!target || !target.releaseCandidate) {
    throw new Error(`Unsupported release-candidate updater platform: ${updaterPlatform}`);
  }
  if (target.stableLabel !== stableLabel) {
    throw new Error(
      `Stable label drifted for ${updaterPlatform}. Expected ${target.stableLabel}; got ${stableLabel}.`
    );
  }

  if (target.stableAssetKind === "msi") {
    copyArtifact(
      pickSingleArtifact(
        artifactPaths,
        (item) => item.toLowerCase().endsWith(".msi") && !item.toLowerCase().endsWith(".msi.sig"),
        "*.msi"
      ),
      outputDir,
      target.latestAssetName
    );
    copyArtifact(
      pickSingleArtifact(
        artifactPaths,
        (item) => item.toLowerCase().endsWith(".msi.sig"),
        "*.msi.sig"
      ),
      outputDir,
      target.latestSignatureName
    );
    return;
  }

  copyArtifact(
    pickSingleArtifact(
      artifactPaths,
      (item) =>
        item.toLowerCase().endsWith(".app.tar.gz") ||
        (item.toLowerCase().endsWith(".tar.gz") && !item.toLowerCase().endsWith(".tar.gz.sig")),
      "*.app.tar.gz"
    ),
    outputDir,
    target.latestAssetName
  );
  copyArtifact(
    pickSingleArtifact(
      artifactPaths,
      (item) =>
        item.toLowerCase().endsWith(".app.tar.gz.sig") ||
        item.toLowerCase().endsWith(".tar.gz.sig"),
      "*.app.tar.gz.sig"
    ),
    outputDir,
    target.latestSignatureName
  );
}

function prepareDevelopmentAssets(args) {
  assertOnlyArgs(args, ["artifact-paths", "target-id", "output-dir"]);
  const artifactPaths = parseArtifactPaths(requireArg(args, "artifact-paths"));
  const targetId = requireArg(args, "target-id");
  if (!findTarget(targetId)) throw new Error(`Unknown development target: ${targetId}`);
  const outputDir = requireArg(args, "output-dir");
  mkdirSync(outputDir, { recursive: true });
  const copiedNames = new Set();
  for (const sourcePath of artifactPaths) {
    const entry = lstatSync(sourcePath);
    if (entry.isSymbolicLink() || !entry.isFile()) {
      throw new Error(`Development bundle must be a regular file: ${sourcePath}`);
    }
    const name = assertSafeAssetName(basename(sourcePath));
    if (name.endsWith(".sig") || copiedNames.has(name)) {
      throw new Error(`Unsafe or duplicate development artifact: ${name}`);
    }
    copiedNames.add(name);
    copyFileSync(sourcePath, join(outputDir, name));
  }
  if (copiedNames.size === 0) throw new Error("No development artifacts were staged.");
}

export function createUpdaterDisabledOverlay() {
  return { bundle: { createUpdaterArtifacts: false } };
}

function writeUpdaterDisabledOverlay(args) {
  assertOnlyArgs(args, ["output"]);
  const outputPath = resolve(requireArg(args, "output"));
  mkdirSync(dirname(outputPath), { recursive: true });
  writeFileSync(outputPath, `${JSON.stringify(createUpdaterDisabledOverlay(), null, 2)}\n`, "utf8");
  process.stdout.write(`${outputPath}\n`);
}

function loadSignature(stableAssetsDir, signatureName) {
  const signaturePath = join(stableAssetsDir, signatureName);
  assertRegularFile(signaturePath, `Updater signature ${signatureName}`);
  const signature = readFileSync(signaturePath, "utf8").replace(/[\r\n]+/g, "");
  if (signature.length === 0) throw new Error(`Updater signature is empty: ${signatureName}`);
  return signature;
}

function normalizeReleaseVersion(tag, repository) {
  const repoName = repository.split("/").at(-1) ?? "";
  if (repoName && tag.startsWith(`${repoName}-v`)) return tag.slice(repoName.length + 2);
  return tag.startsWith("v") ? tag.slice(1) : tag;
}

function buildLatestJson({
  tag,
  repository,
  pubDate,
  stableAssetsDir,
  releaseBody,
  fallbackNotes,
  targets,
}) {
  const platforms = {};
  for (const target of targets) {
    platforms[target.updaterPlatform] = {
      signature: loadSignature(stableAssetsDir, target.latestSignatureName),
      url: `https://github.com/${repository}/releases/download/${tag}/${target.latestAssetName}`,
    };
  }
  return {
    version: normalizeReleaseVersion(tag, repository),
    notes: releaseBody.trim().length > 0 ? releaseBody : fallbackNotes,
    pub_date: pubDate,
    platforms,
  };
}

function writeLatestJsonFile(args) {
  assertOnlyArgs(args, [
    "tag",
    "repo",
    "pub-date",
    "stable-assets-dir",
    "target-ids",
    "release-body-file",
    "fallback-notes",
    "output",
  ]);
  const tag = requireArg(args, "tag");
  const repository = assertCanonicalRepository(requireArg(args, "repo"));
  const pubDate = requireArg(args, "pub-date");
  if (Number.isNaN(Date.parse(pubDate))) throw new Error(`Invalid publication date: ${pubDate}`);
  const stableAssetsDir = requireArg(args, "stable-assets-dir");
  const releaseBodyFile = args.get("release-body-file");
  const releaseBody = releaseBodyFile
    ? readFileSync(releaseBodyFile, "utf8")
    : (process.env.RELEASE_BODY ?? "");
  const fallbackNotes = args.get("fallback-notes") ?? process.env.FALLBACK_NOTES ?? "";
  const targets = selectReleaseTargets(args.get("target-ids"));
  const latestJson = buildLatestJson({
    tag,
    repository,
    pubDate,
    stableAssetsDir,
    releaseBody,
    fallbackNotes,
    targets,
  });
  const outputPath = requireArg(args, "output");
  writeFileSync(outputPath, `${JSON.stringify(latestJson, null, 2)}\n`, "utf8");
  JSON.parse(readFileSync(outputPath, "utf8"));
}

function buildVersionedTagTemplate(tag, version) {
  if (!tag.includes(version)) {
    throw new Error(`Release tag must contain normalized version ${version}: ${tag}`);
  }
  return tag.replace(version, "#{version}");
}

export function buildHomebrewCask({ tag, repo, macosArmSha256, macosIntelSha256 }) {
  const version = normalizeReleaseVersion(tag, repo);
  const tagTemplate = buildVersionedTagTemplate(tag, version);
  const armSha256 = normalizeSha256(macosArmSha256, "macOS Apple Silicon zip");
  const intelSha256 = normalizeSha256(macosIntelSha256, "macOS Intel zip");
  return [
    `# This file is generated from ${repo}.`,
    "# Update it by running `node scripts/support-matrix.mjs homebrew-cask` in the source repo.",
    `cask "${HOMEBREW_CASK.token}" do`,
    '  arch arm: "arm", intel: "intel"',
    "",
    `  version "${version}"`,
    `  sha256 arm:   "${armSha256}",`,
    `         intel: "${intelSha256}"`,
    "",
    `  url "https://github.com/${repo}/releases/download/${tagTemplate}/aio-coding-hub-macos-#{arch}.zip"`,
    `  name "${HOMEBREW_CASK.name}"`,
    `  desc "${HOMEBREW_CASK.desc}"`,
    `  homepage "${HOMEBREW_CASK.homepage}"`,
    "",
    "  auto_updates true",
    "  depends_on :macos",
    "",
    `  app "${HOMEBREW_CASK.appName}"`,
    "",
    "  zap trash: [",
    `    "~/Library/Application Support/${HOMEBREW_CASK.bundleIdentifier}",`,
    `    "~/Library/Caches/${HOMEBREW_CASK.bundleIdentifier}",`,
    `    "~/Library/Preferences/${HOMEBREW_CASK.bundleIdentifier}.plist",`,
    `    "~/Library/Saved Application State/${HOMEBREW_CASK.bundleIdentifier}.savedState",`,
    "  ]",
    "end",
    "",
  ].join("\n");
}

function writeHomebrewCaskFile(args) {
  assertOnlyArgs(args, ["tag", "repo", "macos-arm-sha256", "macos-intel-sha256", "output"]);
  const cask = buildHomebrewCask({
    tag: requireArg(args, "tag"),
    repo: requireArg(args, "repo"),
    macosArmSha256: requireArg(args, "macos-arm-sha256"),
    macosIntelSha256: requireArg(args, "macos-intel-sha256"),
  });
  const outputPath = args.get("output") ?? "";
  if (!outputPath) {
    process.stdout.write(cask);
    return;
  }
  mkdirSync(dirname(outputPath), { recursive: true });
  writeFileSync(outputPath, cask, "utf8");
}

function validateReleaseVersion(args) {
  assertOnlyArgs(args, ["tag"]);
  const tag = requireArg(args, "tag");
  const info = readSynchronizedApplicationVersion();
  assertCanonicalTag(tag, info.version);
  if (tag !== info.tag) throw new Error(`Release tag must equal ${info.tag}.`);
  logger.info("[support-matrix] 发布版本校验通过：%s", info.version);
}

function extractMarkedBlock(content, markerName) {
  const { start, end } = README_MARKERS[markerName];
  const startIndex = content.indexOf(start);
  const endIndex = content.indexOf(end);
  if (startIndex === -1 || endIndex === -1 || endIndex < startIndex) {
    throw new Error(`Missing README markers: ${start} ... ${end}`);
  }
  return content.slice(startIndex, endIndex + end.length);
}

function assertUniqueTargets(items, getValue, label) {
  const seen = new Set();
  for (const item of items) {
    const value = getValue(item);
    if (seen.has(value)) throw new Error(`Duplicate ${label}: ${value}`);
    seen.add(value);
  }
}

function checkReadmes() {
  for (const item of README_LOCALES) {
    const content = readFileSync(join(repoRoot, item.fileName), "utf8");
    for (const markerName of Object.keys(README_MARKERS)) {
      const actual = extractMarkedBlock(content, markerName).trim();
      const expected = renderReadmeBlock(markerName, item.locale).trim();
      if (actual !== expected) {
        throw new Error(`${item.fileName} drifted in ${markerName}.`);
      }
    }
  }
}

function assertWorkflowContains(content, snippet, label) {
  if (!content.includes(snippet)) {
    throw new Error(`Workflow contract drifted: missing ${label}`);
  }
}

function assertWorkflowExcludes(content, snippet, label) {
  if (content.toLowerCase().includes(snippet.toLowerCase())) {
    throw new Error(`Workflow contract drifted: forbidden ${label}`);
  }
}

function assertWorkflowOccurrenceCount(content, snippet, expected, label) {
  const actual = content.split(snippet).length - 1;
  if (actual !== expected) {
    throw new Error(
      `Workflow contract drifted: ${label} expected ${expected} occurrences, found ${actual}`
    );
  }
}

function checkPinnedGithubActions(workflowPath) {
  const content = readFileSync(workflowPath, "utf8");
  for (const match of content.matchAll(/^\s*(?:-\s+)?uses:\s+([^@\s]+)@([^\s#]+)/gm)) {
    const [, actionRef, versionRef] = match;
    if (actionRef.startsWith("./") || actionRef.startsWith("docker://")) continue;
    if (!/^[0-9a-f]{40}$/.test(versionRef)) {
      throw new Error(
        `Workflow action must pin a full SHA: ${workflowPath} -> ${actionRef}@${versionRef}`
      );
    }
  }
}

function listWorkflowPaths() {
  const paths = [];
  for (const entry of readdirSync(WORKFLOW_DIRECTORY, { withFileTypes: true })) {
    if (!/\.ya?ml$/i.test(entry.name)) continue;
    if (!entry.isFile()) {
      throw new Error(`Workflow must be a regular file: ${entry.name}`);
    }
    paths.push(join(WORKFLOW_DIRECTORY, entry.name));
  }
  if (paths.length === 0) throw new Error("No GitHub Actions workflows were found.");
  return paths.sort();
}

export function checkWorkflowContractContents({ ciWorkflow, devBuildWorkflow, releaseWorkflow }) {
  for (const [snippet, label] of [
    ["ci-gate:", "stable required status gate"],
    ["inputs.build_release_candidate == true", "typed recovery gate"],
    [
      "cloud-native-fixes-${{ github.sha }}-${{ github.run_id }}-${{ github.run_attempt }}",
      "drift artifact identity",
    ],
    ["node scripts/cloud-native-drift.mjs classify", "native drift classifier"],
    ["node scripts/pnpm-cli.selftest.mjs", "cross-platform pnpm invocation self-test"],
    ["node scripts/check-local-native-boundary.selftest.mjs", "local native boundary self-test"],
    ["node scripts/check-local-native-boundary.mjs", "local native boundary enforcement"],
    ["environment: release-signing", "protected signing environment"],
    ["printf 'TAURI_SIGNING_PRIVATE_KEY=%s\\n' \"$normalized_key\"", "normalized signing key"],
    ['-p "$TAURI_SIGNING_PRIVATE_KEY_PASSWORD_SECRET"', "Tauri 2.9 signing probe password"],
    ["tauriScript: pnpm exec tauri", "explicit Tauri script"],
    ["-- --locked", "locked Cargo build args"],
    [
      'Compress-Archive -Path "$portableDir/*" -DestinationPath "stable-assets/${{ matrix.portable_asset_name }}" -Force',
      "PowerShell portable archive syntax",
    ],
    [
      "release-candidate-${{ needs.candidate-plan.outputs.source_sha }}-${{ github.run_id }}-${{ github.run_attempt }}",
      "final candidate identity",
    ],
    ["retention-days: 30", "candidate retention"],
    ["--source-validation-run-id", "source validation manifest field"],
    ["--trusted-control-sha", "trusted control manifest field"],
  ]) {
    assertWorkflowContains(ciWorkflow, snippet, label);
  }
  const ciGateBlock = /\n  ci-gate:\n([\s\S]*)$/.exec(ciWorkflow)?.[1] ?? "";
  for (const [snippet, label] of [
    ["if: always()", "stable gate always evaluation"],
    ["needs.assemble-release-candidate.result", "candidate assembly result binding"],
    [
      "needs.assemble-release-candidate.outputs.run_attempt",
      "current-attempt candidate assembly binding",
    ],
    ["require_result frontend", "frontend result gate"],
    ["require_result rust", "Rust result gate"],
    ["require_result assemble-release-candidate", "candidate assembly gate"],
  ]) {
    assertWorkflowContains(ciGateBlock, snippet, label);
  }
  assertWorkflowOccurrenceCount(
    ciWorkflow,
    "TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}",
    0,
    "raw signing key passed directly to build action"
  );
  const frontendBlock =
    /\n  frontend:\n([\s\S]*?)(?=\n  [a-zA-Z0-9_-]+:\n)/.exec(ciWorkflow)?.[1] ?? "";
  for (const forbidden of [
    "cargo ",
    "rust-toolchain",
    "check:generated-bindings",
    "Install system deps (Tauri/Linux)",
  ]) {
    assertWorkflowExcludes(frontendBlock, forbidden, `frontend native work: ${forbidden}`);
  }

  for (const targetId of CLOUD_BUILD_TARGETS.map((target) => target.id)) {
    assertWorkflowContains(devBuildWorkflow, `- ${targetId}`, `development target ${targetId}`);
  }
  for (const [snippet, label] of [
    ["ref: ${{ github.sha }}", "dispatch SHA checkout"],
    ["inputs.target_id", "target-specific selector/concurrency"],
    ["createUpdaterArtifacts", "updater-disabled overlay"],
    ["tauriScript: pnpm exec tauri", "explicit development Tauri script"],
    ["-- --locked", "locked development Cargo args"],
    ["retention-days: 7", "development retention"],
    ["startsWith(matrix.target_id, 'windows-')", "GitHub-hosted development runner allowlist"],
  ]) {
    assertWorkflowContains(devBuildWorkflow, snippet, label);
  }
  for (const forbidden of ["TAURI_SIGNING_", "release-signing", "retention-days: 1"]) {
    assertWorkflowExcludes(devBuildWorkflow, forbidden, `development workflow ${forbidden}`);
  }
  assertWorkflowExcludes(
    devBuildWorkflow,
    "runs-on: ${{ matrix.runner }}",
    "matrix-controlled development runner"
  );

  for (const [snippet, label] of [
    ["resolve-and-verify:", "read-only resolver job"],
    ["publish:", "independent publisher job"],
    ["artifact-ids:", "exact artifact ID download"],
    ["artifact_digest", "artifact digest binding"],
    ["candidateRun.path !== '.github/workflows/ci.yml'", "exact candidate workflow path"],
    ["AIO_CODING_HUB_RELEASE_WORKFLOW_V1", "managed draft marker"],
    ["contents: write", "publish permission"],
    ["actions: read", "artifact read permission"],
    ["sha256:", "remote release asset digest verification"],
  ]) {
    assertWorkflowContains(releaseWorkflow, snippet, label);
  }
  assertWorkflowOccurrenceCount(
    releaseWorkflow,
    "artifact-ids:",
    2,
    "independent exact artifact-ID downloads"
  );
  assertWorkflowOccurrenceCount(
    releaseWorkflow,
    "verify-candidate-manifest",
    2,
    "independent candidate verification"
  );
  const resolveBlock =
    /\n  resolve-and-verify:\n([\s\S]*?)(?=\n  publish:\n)/.exec(releaseWorkflow)?.[1] ?? "";
  assertWorkflowContains(resolveBlock, "actions: read", "resolver actions read permission");
  assertWorkflowContains(resolveBlock, "contents: read", "resolver contents read permission");
  assertWorkflowExcludes(resolveBlock, "contents: write", "resolver write permission");
  for (const forbidden of [
    "cargo ",
    "rust-toolchain",
    "pnpm install",
    "tauri-action",
    "TAURI_SIGNING_",
    "fallback build",
    "build_matrix",
    "\n  build:",
  ]) {
    assertWorkflowExcludes(
      releaseWorkflow,
      forbidden,
      `release native/fallback path: ${forbidden}`
    );
  }
}

function checkWorkflowContracts() {
  if (existsSync(WORKFLOW_PATHS.releasePrSyncCargoLock)) {
    throw new Error("Orphaned release-pr-sync-cargo-lock workflow must be deleted.");
  }
  const contents = {
    ciWorkflow: readFileSync(WORKFLOW_PATHS.ci, "utf8"),
    devBuildWorkflow: readFileSync(WORKFLOW_PATHS.devBuild, "utf8"),
    releaseWorkflow: readFileSync(WORKFLOW_PATHS.release, "utf8"),
  };
  checkWorkflowContractContents(contents);
  for (const workflowPath of listWorkflowPaths()) {
    checkPinnedGithubActions(workflowPath);
  }
}

function runSupportMatrixCheck({ workflowsOnly = false } = {}) {
  checkWorkflowContracts();
  if (workflowsOnly) return;
  checkReadmes();
  assertUniqueTargets(CLOUD_BUILD_TARGETS, (item) => item.id, "cloud target id");
  assertUniqueTargets(CLOUD_BUILD_TARGETS, (item) => item.tauriTarget, "Tauri target");
  assertUniqueTargets(
    CLOUD_BUILD_TARGETS.filter((item) => item.updaterPlatform),
    (item) => item.updaterPlatform,
    "updater platform"
  );
  const releaseIds = selectReleaseTargets().map((target) => target.id);
  if (releaseIds.join(",") !== FORK_RELEASE_TARGET_IDS.join(",")) {
    throw new Error(`Fork release target order drifted: ${releaseIds.join(",")}`);
  }
  const manualIds = buildManualCloudMatrix().map((target) => target.target_id);
  if (manualIds.length !== 6 || new Set(manualIds).size !== 6) {
    throw new Error(`Manual cloud matrix must contain exactly six targets: ${manualIds.join(",")}`);
  }
  const universal = findTarget("macos-universal");
  if (
    universal.tauriTarget !== "universal-apple-darwin" ||
    universal.rustupTargets.join(",") !== "aarch64-apple-darwin,x86_64-apple-darwin"
  ) {
    throw new Error("macOS universal Tauri/rustup target contract drifted.");
  }
  readSynchronizedApplicationVersion();
  logger.info("[support-matrix] 云端构建与制品晋升合同校验通过。");
}

function printReleaseMatrix(args) {
  assertOnlyArgs(args, ["target-ids"]);
  process.stdout.write(
    JSON.stringify(buildReleaseMatrix(selectReleaseTargets(args.get("target-ids"))))
  );
}

function printManualBuildMatrix(args) {
  assertOnlyArgs(args, ["target-id"]);
  process.stdout.write(
    JSON.stringify(buildManualCloudMatrix(selectManualTargets(args.get("target-id"))))
  );
}

function printDesktopCiMatrix(args) {
  assertOnlyArgs(args, []);
  process.stdout.write(JSON.stringify(buildDesktopCiMatrix()));
}

function printReadmeBlock(args) {
  assertOnlyArgs(args, ["locale", "section"]);
  const locale = requireArg(args, "locale");
  if (!["zh", "en"].includes(locale)) throw new Error(`Unsupported README locale: ${locale}`);
  const section = requireArg(args, "section");
  if (!README_MARKERS[section]) throw new Error(`Unsupported README section: ${section}`);
  process.stdout.write(`${renderReadmeBlock(section, locale)}\n`);
}

function printUsageAndExit() {
  logger.error(
    "Usage: node scripts/support-matrix.mjs <build-matrix|manual-build-matrix|ci-matrix|check|check-workflows|validate-release-version|plan-release-candidate|prepare-stable-assets|prepare-development-assets|write-dev-overlay|create-candidate-manifest|verify-candidate-manifest|stage-candidate-files|generate-latest-json|homebrew-cask|readme-block> [--key value]"
  );
  process.exit(1);
}

export function main(argv = process.argv.slice(2)) {
  const [command, ...restArgs] = argv;
  if (!command) printUsageAndExit();
  const args = parseArgs(restArgs);
  switch (command) {
    case "build-matrix":
      printReleaseMatrix(args);
      return;
    case "manual-build-matrix":
      printManualBuildMatrix(args);
      return;
    case "ci-matrix":
      printDesktopCiMatrix(args);
      return;
    case "check":
      assertOnlyArgs(args, []);
      runSupportMatrixCheck();
      return;
    case "check-workflows":
      assertOnlyArgs(args, []);
      runSupportMatrixCheck({ workflowsOnly: true });
      return;
    case "validate-release-version":
      validateReleaseVersion(args);
      return;
    case "plan-release-candidate":
      runCandidatePlan(args);
      return;
    case "prepare-stable-assets":
      prepareStableAssets(args);
      return;
    case "prepare-development-assets":
      prepareDevelopmentAssets(args);
      return;
    case "write-dev-overlay":
      writeUpdaterDisabledOverlay(args);
      return;
    case "create-candidate-manifest":
      writeCandidateManifest(args);
      return;
    case "verify-candidate-manifest":
      verifyCandidateManifestCommand(args);
      return;
    case "stage-candidate-files":
      stageCandidateFiles(args);
      return;
    case "generate-latest-json":
      writeLatestJsonFile(args);
      return;
    case "homebrew-cask":
      writeHomebrewCaskFile(args);
      return;
    case "readme-block":
      printReadmeBlock(args);
      return;
    default:
      throw new Error(`Unsupported command: ${command}`);
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  main();
}
