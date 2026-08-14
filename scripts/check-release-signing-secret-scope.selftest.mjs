import assert from "node:assert/strict";

import { assertReleaseSigningSecretScope } from "./check-release-signing-secret-scope.mjs";

const workflow = `
jobs:
  contracts:
    steps:
      - name: Validate release signing secret scope
        run: node scripts/check-release-signing-secret-scope.selftest.mjs && node scripts/check-release-signing-secret-scope.mjs

  build-release-candidate:
    steps:
      - name: Validate updater signing secrets
        shell: bash
        env:
          TAURI_SIGNING_PRIVATE_KEY_SECRET: \${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD_SECRET: \${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
        run: |
          umask 077
          key_path="$RUNNER_TEMP/tauri-updater.key"
          printf '%s' "$normalized_key" > "$key_path"
          chmod 600 "$key_path"
          pnpm exec tauri signer sign \\
            -f "$key_path" \\
            -p "$TAURI_SIGNING_PRIVATE_KEY_PASSWORD_SECRET" \\
            "$test_path"

      - id: tauri
        name: Build signed Tauri candidate
        uses: tauri-apps/tauri-action@pinned
        env:
          TAURI_SIGNING_PRIVATE_KEY: \${{ runner.temp }}/tauri-updater.key
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: \${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}

      - name: Delete updater signing key
        if: always()
        shell: bash
        run: rm -f "$RUNNER_TEMP/tauri-updater.key"

      - name: Prepare updater assets
        run: node scripts/support-matrix.mjs prepare-stable-assets

  rust:
`;
const privateKeySecret = "${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}";

function expectRejected(name, ci, expected) {
  assert.throws(() => assertReleaseSigningSecretScope({ ci }), expected, name);
}

assert.doesNotThrow(() => assertReleaseSigningSecretScope({ ci: workflow }));

for (const commandFile of [
  "GITHUB_ENV",
  "GITHUB_OUTPUT",
  "GITHUB_PATH",
  "GITHUB_STATE",
  "GITHUB_STEP_SUMMARY",
]) {
  expectRejected(
    `command-file signing key (${commandFile})`,
    workflow.replace(
      'printf \'%s\' "$normalized_key" > "$key_path"',
      `printf 'TAURI_SIGNING_PRIVATE_KEY=%s\\n' "$normalized_key" >> "$${commandFile}"`
    ),
    new RegExp(`must not promote signing data through ${commandFile}`)
  );
}
expectRejected(
  "direct secret in build",
  workflow.replace(
    "TAURI_SIGNING_PRIVATE_KEY: \${{ runner.temp }}/tauri-updater.key",
    "TAURI_SIGNING_PRIVATE_KEY: \${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}"
  ),
  /must receive only the runner-temp signing key path/
);
expectRejected(
  "private key outside validation",
  workflow
    .replace(
      `          TAURI_SIGNING_PRIVATE_KEY_SECRET: ${privateKeySecret}`,
      "          UNUSED_SECRET: unavailable"
    )
    .replace(
      "      - name: Validate updater signing secrets",
      `      - name: Bootstrap
        env:
          BOOTSTRAP_SECRET: ${privateKeySecret}
        run: true

      - name: Validate updater signing secrets`
    ),
  /validation must receive the private key through its step-scoped secret environment/
);
expectRejected(
  "workspace key path",
  workflow.replace("$RUNNER_TEMP/tauri-updater.key", "$GITHUB_WORKSPACE/tauri-updater.key"),
  /fixed runner-temp signing key path/
);
expectRejected(
  "loose signing key creation permissions",
  workflow.replace("          umask 077\n", ""),
  /restrict permissions before writing/
);
expectRejected(
  "missing cleanup",
  workflow.replace(
    `      - name: Delete updater signing key
        if: always()
        shell: bash
        run: rm -f "$RUNNER_TEMP/tauri-updater.key"

`,
    ""
  ),
  /cleanup step is required/
);
expectRejected(
  "cleanup not adjacent",
  workflow.replace(
    "      - name: Delete updater signing key",
    "      - name: Unrelated action\n        run: true\n\n      - name: Delete updater signing key"
  ),
  /cleanup must immediately follow/
);
expectRejected(
  "cleanup only echoed",
  workflow.replace(
    'run: rm -f "$RUNNER_TEMP/tauri-updater.key"',
    "run: echo 'rm -f \"$RUNNER_TEMP/tauri-updater.key\"'"
  ),
  /cleanup must execute rm/
);
expectRejected(
  "later key reference",
  workflow.replace(
    "      - name: Prepare updater assets\n        run:",
    "      - name: Prepare updater assets\n        env:\n          TAURI_SIGNING_PRIVATE_KEY: still-visible\n        run:"
  ),
  /steps after signing key cleanup must not reference/
);
expectRejected(
  "contracts job disconnected",
  workflow.replace(
    "node scripts/check-release-signing-secret-scope.selftest.mjs && node scripts/check-release-signing-secret-scope.mjs",
    "node missing-release-signing-contract.mjs"
  ),
  /contracts must execute/
);

console.log("[release-signing-secret-scope:selftest] all assertions passed");
