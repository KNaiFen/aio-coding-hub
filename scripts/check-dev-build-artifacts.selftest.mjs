import assert from "node:assert/strict";
import { chmodSync, mkdirSync, mkdtempSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

import { assertDevBuildArtifacts } from "./check-dev-build-artifacts.mjs";

const devBuild = `
jobs:
  plan:
    steps:
      - run: |
          case "$TARGET_ID" in
            windows-x64)
              echo "runner=windows-latest"
              echo "target=x86_64-pc-windows-msvc"
              echo "bundles=msi"
              ;;
            macos-x64)
              echo "runner=macos-latest"
              echo "target=x86_64-apple-darwin"
              echo "bundles=app"
              ;;
            macos-arm64)
              echo "runner=macos-latest"
              echo "target=aarch64-apple-darwin"
              echo "bundles=app"
              ;;
            linux-x64)
              echo "runner=ubuntu-22.04"
              echo "target=x86_64-unknown-linux-gnu"
              echo "bundles=deb,appimage"
              ;;
          esac
  build:
    steps:
      - name: Prepare Windows development artifact
        if: inputs.target_id == 'windows-x64'
        run: |
          if ($msiFiles.Count -ne 1) { throw "MSI" }
          if ($exeFiles.Count -ne 1) { throw "EXE" }
          Copy-Item $msiFiles[0] -Destination "dev-build-artifact/bundle/msi"
      - name: Prepare macOS development artifact
        if: startsWith(inputs.target_id, 'macos-')
        run: |
          ditto -c -k --sequesterRsrc --keepParent "$app_path" "$archive_path"
          ditto -x -k "$archive_path" "$verify_dir"
          [[ -x "$main_executable" ]]
      - name: Prepare Linux development artifact
        if: inputs.target_id == 'linux-x64'
        run: |
          cp -p "$appimage_path" "$stage_dir/"
          tar -czf "$archive_path" -C "$stage_dir" .
          tar -xzf "$archive_path" -C "$verify_dir"
          [[ -x "$extracted_appimage" ]]
      - name: Summarize development artifact
        run: |
          echo "payload" >> "$GITHUB_STEP_SUMMARY"
      - name: Upload development artifact
        uses: actions/upload-artifact@pinned
        with:
          path: dev-build-artifact/*
          if-no-files-found: error
`;

const ci = `
jobs:
  contracts:
    steps:
      - run: node scripts/check-dev-build-artifacts.selftest.mjs && node scripts/check-dev-build-artifacts.mjs
  frontend:
`;

const valid = { devBuild, ci };
assert.doesNotThrow(() => assertDevBuildArtifacts(valid));

function expectRejected(name, files, expected) {
  assert.throws(() => assertDevBuildArtifacts(files), expected, name);
}

expectRejected(
  "changed macOS arm mapping",
  {
    ...valid,
    devBuild: devBuild.replace("aarch64-apple-darwin", "x86_64-apple-darwin"),
  },
  /macos-arm64 must retain target mapping: echo "target=aarch64-apple-darwin"/
);
expectRejected(
  "flattened Windows MSI",
  {
    ...valid,
    devBuild: devBuild.replace("dev-build-artifact/bundle/msi", "dev-build-artifact"),
  },
  /preserve the MSI bundle layout/
);
expectRejected(
  "raw bundle upload",
  { ...valid, devBuild: devBuild.replace("path: dev-build-artifact/*", "path: bundle/**") },
  /raw bundle/
);
expectRejected(
  "missing macOS archive",
  { ...valid, devBuild: devBuild.replace("ditto -c -k --sequesterRsrc --keepParent", "echo") },
  /archived with ditto/
);
expectRejected(
  "macOS archive command only echoed",
  {
    ...valid,
    devBuild: devBuild.replace(
      'ditto -c -k --sequesterRsrc --keepParent "$app_path" "$archive_path"',
      'echo \'ditto -c -k --sequesterRsrc --keepParent "$app_path" "$archive_path"\''
    ),
  },
  /archived with ditto/
);
expectRejected(
  "macOS extraction command only echoed",
  {
    ...valid,
    devBuild: devBuild.replace(
      'ditto -x -k "$archive_path" "$verify_dir"',
      'echo \'ditto -x -k "$archive_path" "$verify_dir"\''
    ),
  },
  /extracted before mode verification/
);
expectRejected(
  "macOS mode check only echoed",
  {
    ...valid,
    devBuild: devBuild.replace(
      '[[ -x "$main_executable" ]]',
      "echo '[[ -x \"$main_executable\" ]]'"
    ),
  },
  /verify the extracted main executable mode/
);
expectRejected(
  "Linux condition on unrelated step",
  {
    ...valid,
    devBuild: devBuild.replace(
      "- name: Prepare Linux development artifact",
      "- name: Install Linux system dependencies"
    ),
  },
  /Linux-only artifact preparation step/
);
expectRejected(
  "missing Linux mode preservation",
  { ...valid, devBuild: devBuild.replace('cp -p "$appimage_path"', 'cp "$appimage_path"') },
  /preserve the AppImage mode/
);
expectRejected(
  "Linux mode command only assigned",
  {
    ...valid,
    devBuild: devBuild.replace(
      'cp -p "$appimage_path" "$stage_dir/"',
      'copy_command=\'cp -p "$appimage_path" "$stage_dir/"\''
    ),
  },
  /preserve the AppImage mode/
);
expectRejected(
  "Linux archive command only echoed",
  {
    ...valid,
    devBuild: devBuild.replace(
      'tar -czf "$archive_path" -C "$stage_dir" .',
      'echo \'tar -czf "$archive_path" -C "$stage_dir" .\''
    ),
  },
  /archived with tar/
);
expectRejected(
  "Linux mode check only echoed",
  {
    ...valid,
    devBuild: devBuild.replace(
      '[[ -x "$extracted_appimage" ]]',
      "echo '[[ -x \"$extracted_appimage\" ]]'"
    ),
  },
  /verify the extracted AppImage mode/
);
expectRejected(
  "duplicate upload",
  {
    ...valid,
    devBuild: devBuild.replace(
      "uses: actions/upload-artifact@pinned",
      "uses: actions/upload-artifact@pinned\n      - uses: actions/upload-artifact@duplicate"
    ),
  },
  /exactly one upload-artifact/
);
expectRejected(
  "missing contracts command",
  {
    ...valid,
    ci: ci.replace("node scripts/check-dev-build-artifacts.selftest.mjs", "node missing"),
  },
  /contracts must execute/
);
expectRejected(
  "contracts command in a comment",
  {
    ...valid,
    ci: ci.replace(
      "- run: node scripts/check-dev-build-artifacts.selftest.mjs",
      "# run: node scripts/check-dev-build-artifacts.selftest.mjs"
    ),
  },
  /contracts must execute/
);
expectRejected(
  "contracts command only echoed",
  {
    ...valid,
    ci: ci.replace(
      "- run: node scripts/check-dev-build-artifacts.selftest.mjs && node scripts/check-dev-build-artifacts.mjs",
      "- run: echo 'node scripts/check-dev-build-artifacts.selftest.mjs && node scripts/check-dev-build-artifacts.mjs'"
    ),
  },
  /contracts must execute/
);

function run(command, args) {
  const result = spawnSync(command, args, { encoding: "utf8" });
  assert.equal(result.status, 0, `${command}: ${result.stdout}\n${result.stderr}`);
}

if (process.platform !== "win32") {
  const root = mkdtempSync(join(tmpdir(), "aio-dev-build-artifact-"));
  try {
    const stage = join(root, "linux-stage");
    const extracted = join(root, "linux-extracted");
    const appImage = join(stage, "AIO_Coding_Hub.AppImage");
    const archive = join(root, "aio-coding-hub-linux-x64.tar.gz");
    mkdirSync(stage);
    mkdirSync(extracted);
    writeFileSync(appImage, "fixture\n");
    chmodSync(appImage, 0o755);
    run("tar", ["-czf", archive, "-C", stage, "."]);
    run("tar", ["-xzf", archive, "-C", extracted]);
    assert.notEqual(statSync(join(extracted, "AIO_Coding_Hub.AppImage")).mode & 0o111, 0);

    if (process.platform === "darwin") {
      const app = join(root, "AIO Coding Hub.app");
      const executable = join(app, "Contents", "MacOS", "AIO Coding Hub");
      const zip = join(root, "aio-coding-hub-macos-arm64.zip");
      const unzipped = join(root, "macos-extracted");
      mkdirSync(join(app, "Contents", "MacOS"), { recursive: true });
      mkdirSync(unzipped);
      writeFileSync(executable, "fixture\n");
      chmodSync(executable, 0o755);
      run("ditto", ["-c", "-k", "--sequesterRsrc", "--keepParent", app, zip]);
      run("ditto", ["-x", "-k", zip, unzipped]);
      assert.notEqual(
        statSync(join(unzipped, "AIO Coding Hub.app", "Contents", "MacOS", "AIO Coding Hub")).mode &
          0o111,
        0
      );
    }
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

console.log("[dev-build-artifacts:selftest] all assertions passed");
