import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const modulePath = fileURLToPath(import.meta.url);
const repoRoot = dirname(dirname(modulePath));

function jobBlock(source, jobName) {
  const lines = source.split(/\r?\n/);
  const start = lines.findIndex((line) => line === `  ${jobName}:`);
  if (start === -1) return "";

  let end = start + 1;
  while (end < lines.length && !/^  [A-Za-z0-9_-]+:\s*$/.test(lines[end])) end += 1;
  return lines.slice(start, end).join("\n");
}

function executableLines(source) {
  return source
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0 && !line.startsWith("#"));
}

function caseBlock(source, targetId) {
  const lines = source.split(/\r?\n/);
  const start = lines.findIndex((line) => line.trim() === `${targetId})`);
  if (start === -1) return "";

  let end = start + 1;
  while (end < lines.length && lines[end].trim() !== ";;") end += 1;
  return lines.slice(start, Math.min(end + 1, lines.length)).join("\n");
}

function stepBlock(source, stepName) {
  const lines = source.split(/\r?\n/);
  const start = lines.findIndex((line) => line.trim() === `- name: ${stepName}`);
  if (start === -1) return "";

  let end = start + 1;
  while (end < lines.length && !/^\s{6}- (?:name:|uses:)/.test(lines[end])) end += 1;
  return lines.slice(start, end).join("\n");
}

function requireToken(lines, token, message, failures) {
  if (!lines.some((line) => line.includes(token))) failures.push(message);
}

function requireCommand(lines, pattern, message, failures) {
  if (!lines.some((line) => pattern.test(line))) failures.push(message);
}

function countToken(lines, token) {
  return lines.filter((line) => line.includes(token)).length;
}

function hasRunCommand(source, command) {
  return source.split(/\r?\n/).some((line) => {
    const run = line.match(/^\s*(?:-\s+)?run:\s*(.+)$/);
    return run && run[1].trim() === command;
  });
}

export function validateDevBuildArtifacts({ devBuild, ci }) {
  const failures = [];
  const planBlock = jobBlock(devBuild, "plan");
  const buildBlock = jobBlock(devBuild, "build");
  const build = executableLines(buildBlock);
  const windows = executableLines(stepBlock(buildBlock, "Prepare Windows development artifact"));
  const macos = executableLines(stepBlock(buildBlock, "Prepare macOS development artifact"));
  const linux = executableLines(stepBlock(buildBlock, "Prepare Linux development artifact"));
  const summary = executableLines(stepBlock(buildBlock, "Summarize development artifact"));
  const upload = executableLines(stepBlock(buildBlock, "Upload development artifact"));
  const supportBlock = jobBlock(ci, "support-contract");

  const targetMappings = {
    "windows-x64": [
      'echo "runner=windows-latest"',
      'echo "target=x86_64-pc-windows-msvc"',
      'echo "bundles=msi"',
    ],
    "macos-x64": [
      'echo "runner=macos-latest"',
      'echo "target=x86_64-apple-darwin"',
      'echo "bundles=app"',
    ],
    "macos-arm64": [
      'echo "runner=macos-latest"',
      'echo "target=aarch64-apple-darwin"',
      'echo "bundles=app"',
    ],
    "linux-x64": [
      'echo "runner=ubuntu-22.04"',
      'echo "target=x86_64-unknown-linux-gnu"',
      'echo "bundles=deb,appimage"',
    ],
  };
  for (const [targetId, tokens] of Object.entries(targetMappings)) {
    const target = executableLines(caseBlock(planBlock, targetId));
    for (const token of tokens) {
      requireToken(target, token, `${targetId} must retain target mapping: ${token}`, failures);
    }
  }

  requireToken(
    windows,
    "if: inputs.target_id == 'windows-x64'",
    "build job must have a Windows-only artifact preparation step",
    failures
  );
  requireToken(
    windows,
    "if ($msiFiles.Count -ne 1)",
    "Windows preparation must require exactly one MSI",
    failures
  );
  requireToken(
    windows,
    "if ($exeFiles.Count -ne 1)",
    "Windows preparation must require exactly one EXE",
    failures
  );
  requireToken(
    windows,
    'Destination "dev-build-artifact/bundle/msi"',
    "Windows preparation must preserve the MSI bundle layout",
    failures
  );
  requireToken(
    macos,
    "if: startsWith(inputs.target_id, 'macos-')",
    "build job must have a macOS-only artifact preparation step",
    failures
  );
  requireCommand(
    macos,
    /^ditto -c -k --sequesterRsrc --keepParent "\$app_path" "\$archive_path"$/,
    "macOS app must be archived with ditto before upload",
    failures
  );
  requireCommand(
    macos,
    /^ditto -x -k "\$archive_path" "\$verify_dir"$/,
    "macOS archive must be extracted before mode verification",
    failures
  );
  requireCommand(
    macos,
    /^\[\[ -x "\$main_executable" \]\]$/,
    "macOS archive must verify the extracted main executable mode",
    failures
  );
  requireToken(
    linux,
    "if: inputs.target_id == 'linux-x64'",
    "build job must have a Linux-only artifact preparation step",
    failures
  );
  requireCommand(
    linux,
    /^cp -p "\$appimage_path" "\$stage_dir\/"$/,
    "Linux preparation must preserve the AppImage mode before archiving",
    failures
  );
  requireCommand(
    linux,
    /^tar -czf "\$archive_path" -C "\$stage_dir" \.$/,
    "Linux payload must be archived with tar before upload",
    failures
  );
  requireCommand(
    linux,
    /^tar -xzf "\$archive_path" -C "\$verify_dir"$/,
    "Linux archive must be extracted before mode verification",
    failures
  );
  requireCommand(
    linux,
    /^\[\[ -x "\$extracted_appimage" \]\]$/,
    "Linux archive must verify the extracted AppImage mode",
    failures
  );

  if (build.some((line) => line.includes("bundle/**"))) {
    failures.push("upload path must not include raw bundle/** content");
  }
  if (countToken(build, "actions/upload-artifact@") !== 1) {
    failures.push("build job must contain exactly one upload-artifact step");
  }
  if (countToken(upload, "path: dev-build-artifact/*") !== 1) {
    failures.push("upload-artifact must read only the prepared artifact directory");
  }
  requireToken(
    upload,
    "if-no-files-found: error",
    "upload-artifact must fail when the prepared payload is missing",
    failures
  );
  requireToken(
    summary,
    '>> "$GITHUB_STEP_SUMMARY"',
    "workflow summary must list the prepared development payload",
    failures
  );

  if (
    !hasRunCommand(
      supportBlock,
      "node scripts/check-dev-build-artifacts.selftest.mjs && node scripts/check-dev-build-artifacts.mjs"
    )
  ) {
    failures.push("support-contract must execute the dev-build artifact contract");
  }

  return failures;
}

export function assertDevBuildArtifacts(files) {
  const failures = validateDevBuildArtifacts(files);
  if (failures.length > 0) {
    throw new Error(`Dev-build artifact contract failed:\n- ${failures.join("\n- ")}`);
  }
}

if (process.argv[1] && resolve(process.argv[1]) === modulePath) {
  assertDevBuildArtifacts({
    devBuild: readFileSync(resolve(repoRoot, ".github/workflows/dev-build.yml"), "utf8"),
    ci: readFileSync(resolve(repoRoot, ".github/workflows/ci.yml"), "utf8"),
  });
  console.log("[dev-build-artifacts] workflow contract passed");
}
