import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)));

const RELEASE_TARGETS = Object.freeze([
  {
    id: "windows-x64",
    updaterPlatform: "windows-x86_64",
    stableLabel: "win64",
    kind: "msi",
    assetName: "aio-coding-hub-win64.msi",
    signatureName: "aio-coding-hub-win64.msi.sig",
  },
  {
    id: "macos-arm64",
    updaterPlatform: "darwin-aarch64",
    stableLabel: "macos-arm",
    kind: "tarball",
    assetName: "aio-coding-hub-macos-arm.tar.gz",
    signatureName: "aio-coding-hub-macos-arm.tar.gz.sig",
  },
]);

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
  for (let index = 0; index < rawArgs.length; index += 2) {
    const token = rawArgs[index];
    const value = rawArgs[index + 1];
    if (!token?.startsWith("--") || value == null || value.startsWith("--")) {
      throw new Error(`Invalid argument near ${token ?? "<end>"}`);
    }
    const name = token.slice(2);
    if (args.has(name)) throw new Error(`Duplicate argument: ${token}`);
    args.set(name, value);
  }
  return args;
}

function requireArg(args, name) {
  const value = args.get(name);
  if (!value) throw new Error(`Missing required argument: --${name}`);
  return value;
}

function selectedTargets(args) {
  const ids = (args.get("target-ids") ?? RELEASE_TARGETS.map((target) => target.id).join(","))
    .split(",")
    .map((value) => value.trim())
    .filter(Boolean);
  if (ids.length === 0 || new Set(ids).size !== ids.length) {
    throw new Error("--target-ids must contain unique release targets");
  }
  return ids.map((id) => {
    const target = RELEASE_TARGETS.find((candidate) => candidate.id === id);
    if (!target) throw new Error(`Unsupported release target: ${id}`);
    return target;
  });
}

function readJson(path) {
  return JSON.parse(readFileSync(join(repoRoot, path), "utf8"));
}

function validateReleaseVersion(args) {
  const tag = requireArg(args, "tag");
  const match = /^aio-coding-hub-v((?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*))$/.exec(tag);
  if (!match) throw new Error(`Invalid release tag: ${tag}`);
  const expected = match[1];
  const cargoToml = readFileSync(join(repoRoot, "src-tauri/Cargo.toml"), "utf8");
  const cargoLock = readFileSync(join(repoRoot, "src-tauri/Cargo.lock"), "utf8");
  const versions = new Map([
    ["package.json", readJson("package.json").version],
    ["src-tauri/tauri.conf.json", readJson("src-tauri/tauri.conf.json").version],
    ["src-tauri/Cargo.toml", /^version\s*=\s*"([^"]+)"/m.exec(cargoToml)?.[1]],
    [
      "src-tauri/Cargo.lock",
      /\[\[package\]\]\s+name\s*=\s*"aio-coding-hub"\s+version\s*=\s*"([^"]+)"/m.exec(
        cargoLock
      )?.[1],
    ],
  ]);
  for (const [path, version] of versions) {
    if (version !== expected) {
      throw new Error(`Release version mismatch: tag=${expected}, ${path}=${version ?? "missing"}`);
    }
  }
  console.error(`[support-matrix] release version ${expected} is consistent`);
}

function parseArtifactPaths(args) {
  const raw = requireArg(args, "artifact-paths");
  const paths = JSON.parse(raw);
  if (
    !Array.isArray(paths) ||
    paths.length === 0 ||
    paths.some((path) => typeof path !== "string")
  ) {
    throw new Error("--artifact-paths must be a non-empty JSON string array");
  }
  return paths;
}

function pickArtifact(paths, predicate, label) {
  const path = paths.find((candidate) => predicate(candidate.toLowerCase()));
  if (!path) throw new Error(`Missing ${label} in Tauri artifact paths`);
  return path;
}

function prepareStableAssets(args) {
  const updaterPlatform = requireArg(args, "updater-platform");
  const stableLabel = requireArg(args, "stable-label");
  const outputDir = requireArg(args, "output-dir");
  const target = RELEASE_TARGETS.find((candidate) => candidate.updaterPlatform === updaterPlatform);
  if (!target || target.stableLabel !== stableLabel) {
    throw new Error(`Unsupported release asset target: ${updaterPlatform}/${stableLabel}`);
  }

  const paths = parseArtifactPaths(args);
  let artifact;
  let signature;
  if (target.kind === "msi") {
    artifact = pickArtifact(paths, (path) => path.endsWith(".msi"), "MSI");
    signature = pickArtifact(paths, (path) => path.endsWith(".msi.sig"), "MSI signature");
  } else {
    artifact = pickArtifact(
      paths,
      (path) =>
        path.endsWith(".app.tar.gz") || (path.endsWith(".tar.gz") && !path.endsWith(".tar.gz.sig")),
      "macOS updater tarball"
    );
    signature = pickArtifact(
      paths,
      (path) => path.endsWith(".app.tar.gz.sig") || path.endsWith(".tar.gz.sig"),
      "macOS updater signature"
    );
  }

  mkdirSync(outputDir, { recursive: true });
  copyFileSync(artifact, join(outputDir, target.assetName));
  copyFileSync(signature, join(outputDir, target.signatureName));
}

function normalizedVersion(tag, repo) {
  const prefix = `${repo.split("/").at(-1)}-v`;
  return tag.startsWith(prefix) ? tag.slice(prefix.length) : tag.replace(/^v/, "");
}

function readSignature(directory, name) {
  const path = join(directory, name);
  if (!existsSync(path)) throw new Error(`Missing signature file: ${path}`);
  return readFileSync(path, "utf8").replace(/[\r\n]+/g, "");
}

function generateLatestJson(args) {
  const tag = requireArg(args, "tag");
  const repo = requireArg(args, "repo");
  const pubDate = requireArg(args, "pub-date");
  const assetsDirectory = requireArg(args, "stable-assets-dir");
  const output = requireArg(args, "output");
  const platforms = {};
  for (const target of selectedTargets(args)) {
    platforms[target.updaterPlatform] = {
      signature: readSignature(assetsDirectory, target.signatureName),
      url: `https://github.com/${repo}/releases/download/${tag}/${target.assetName}`,
    };
  }
  const releaseBody = process.env.RELEASE_BODY?.trim();
  const document = {
    version: normalizedVersion(tag, repo),
    notes: releaseBody || process.env.FALLBACK_NOTES || "",
    pub_date: pubDate,
    platforms,
  };
  writeFileSync(output, `${JSON.stringify(document, null, 2)}\n`, "utf8");
}

function normalizeSha256(value, label) {
  const normalized = value.replace(/^sha256:/, "").toLowerCase();
  if (!/^[0-9a-f]{64}$/.test(normalized)) {
    throw new Error(`Invalid SHA-256 for ${label}`);
  }
  return normalized;
}

function buildHomebrewCask({ tag, repo, macosArmSha256, macosIntelSha256 }) {
  const version = normalizedVersion(tag, repo);
  if (!tag.includes(version)) throw new Error(`Release tag does not contain ${version}`);
  const tagTemplate = tag.replace(version, "#{version}");
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

function writeHomebrewCask(args) {
  const cask = buildHomebrewCask({
    tag: requireArg(args, "tag"),
    repo: requireArg(args, "repo"),
    macosArmSha256: requireArg(args, "macos-arm-sha256"),
    macosIntelSha256: requireArg(args, "macos-intel-sha256"),
  });
  const output = args.get("output");
  if (!output) {
    process.stdout.write(cask);
    return;
  }
  mkdirSync(dirname(output), { recursive: true });
  writeFileSync(output, cask, "utf8");
}

function main() {
  const [command, ...rawArgs] = process.argv.slice(2);
  const args = parseArgs(rawArgs);
  switch (command) {
    case "validate-release-version":
      validateReleaseVersion(args);
      return;
    case "prepare-stable-assets":
      prepareStableAssets(args);
      return;
    case "generate-latest-json":
      generateLatestJson(args);
      return;
    case "homebrew-cask":
      writeHomebrewCask(args);
      return;
    default:
      throw new Error(
        "Usage: node scripts/support-matrix.mjs <validate-release-version|prepare-stable-assets|generate-latest-json|homebrew-cask> [--key value]"
      );
  }
}

main();
