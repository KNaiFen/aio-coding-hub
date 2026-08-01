import { readFileSync } from "node:fs";

const files = {
  ci: readFileSync(".github/workflows/ci.yml", "utf8"),
  release: readFileSync(".github/workflows/release.yml", "utf8"),
  cargo: readFileSync("src-tauri/Cargo.toml", "utf8"),
  readme: readFileSync("README.md", "utf8"),
  readmeEn: readFileSync("README_EN.md", "utf8"),
};

const assets = [
  "aio-tui-win64.zip",
  "aio-tui-macos-intel.tar.gz",
  "aio-tui-macos-arm.tar.gz",
  "aio-tui-linux-x64.tar.gz",
];

function requireText(label, content, expected) {
  if (!content.includes(expected)) {
    throw new Error(`${label} is missing required contract text: ${expected}`);
  }
}

requireText("CI", files.ci, "build-tui-release-candidate:");
requireText("CI", files.ci, "--package aio-tui");
requireText("CI", files.ci, "--target-ids windows-x64,macos-arm64");
requireText("release workflow", files.release, "workflow_dispatch:");
requireText("release workflow", files.release, "TAG_NAME: ${{ inputs.tag || github.ref_name }}");
requireText("release workflow", files.release, 'tag_ref="refs/tags/$TAG_NAME"');
requireText("release workflow", files.release, "refs/heads/main:refs/remotes/origin/main");
requireText("release workflow", files.release, "artifact-ids:");
requireText("release workflow", files.release, "merge-multiple: true");
requireText("Cargo workspace", files.cargo, '"crates/aio-observer-protocol"');
requireText("Cargo workspace", files.cargo, '"crates/aio-tui"');

for (const asset of assets) {
  requireText("CI", files.ci, asset);
  requireText("release workflow", files.release, asset);
  requireText("Chinese README", files.readme, asset);
  requireText("English README", files.readmeEn, asset);
}

console.log("[tui-release-contract] observer workspace and four release assets are consistent");
