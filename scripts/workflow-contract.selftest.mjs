import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { checkWorkflowContractContents } from "./support-matrix.mjs";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const baseline = {
  ciWorkflow: readFileSync(join(repoRoot, ".github/workflows/ci.yml"), "utf8"),
  devBuildWorkflow: readFileSync(join(repoRoot, ".github/workflows/dev-build.yml"), "utf8"),
  releaseWorkflow: readFileSync(join(repoRoot, ".github/workflows/release.yml"), "utf8"),
};

checkWorkflowContractContents(baseline);

function expectMutationRejected(field, from, to) {
  assert.ok(baseline[field].includes(from), `Fixture token is missing: ${from}`);
  const mutated = { ...baseline, [field]: baseline[field].replaceAll(from, to) };
  assert.throws(() => checkWorkflowContractContents(mutated));
}

expectMutationRejected("ciWorkflow", "environment: release-signing", "environment: unprotected");
expectMutationRejected("ciWorkflow", "  ci-gate:\n", "  ci-final:\n");
expectMutationRejected("ciWorkflow", "  ci-gate:\n", "  renamed-gate:\n");
expectMutationRejected("ciWorkflow", "retention-days: 30", "retention-days: 29");
expectMutationRejected("ciWorkflow", "-- --locked", "-- --offline");
expectMutationRejected(
  "ciWorkflow",
  `          node scripts/check-local-native-boundary.mjs
          node scripts/pnpm-cli.selftest.mjs
          node scripts/check-local-native-boundary.selftest.mjs`,
  `          node scripts/check-local-native-boundary.selftest.mjs
          node scripts/pnpm-cli.selftest.mjs
          node scripts/check-local-native-boundary.mjs`
);
expectMutationRejected(
  "ciWorkflow",
  "printf 'TAURI_SIGNING_PRIVATE_KEY=%s\\n' \"$normalized_key\"",
  "printf 'TAURI_SIGNING_PRIVATE_KEY=%s\\n' \"$TAURI_SIGNING_PRIVATE_KEY_SECRET\""
);
expectMutationRejected(
  "ciWorkflow",
  '-p "$TAURI_SIGNING_PRIVATE_KEY_PASSWORD_SECRET"',
  "TAURI_SIGNING_PRIVATE_KEY_PASSWORD=unsupported"
);
expectMutationRejected(
  "ciWorkflow",
  "needs.assemble-release-candidate.outputs.run_attempt",
  "needs.assemble-release-candidate.result"
);
expectMutationRejected(
  "ciWorkflow",
  'Compress-Archive -Path "$portableDir/*" -DestinationPath "stable-assets/${{ matrix.portable_asset_name }}" -Force',
  'Compress-Archive \\\n            -Path "$portableDir/*"'
);
expectMutationRejected("devBuildWorkflow", "          - windows-arm64\n", "");
expectMutationRejected("devBuildWorkflow", "retention-days: 7", "retention-days: 1");
expectMutationRejected("devBuildWorkflow", "createUpdaterArtifacts", "updaterArtifacts");
expectMutationRejected("releaseWorkflow", "artifact-ids:", "artifact-name:");
expectMutationRejected("releaseWorkflow", "artifact_digest", "archive_digest");
expectMutationRejected(
  "releaseWorkflow",
  "candidateRun.path !== '.github/workflows/ci.yml'",
  "candidateRun.path?.startsWith('.github/workflows/ci.yml@')"
);
expectMutationRejected("releaseWorkflow", "publish:\n", "publish-release:\n");

for (const forbidden of [
  "\n      - name: Reintroduced Cargo build\n        run: cargo build --locked\n",
  "\n      - name: Reintroduced dependency install\n        run: pnpm install\n",
  "\n      - name: Reintroduced signing secret\n        run: echo TAURI_SIGNING_PRIVATE_KEY\n",
  "\n  build:\n    runs-on: ubuntu-latest\n",
]) {
  assert.throws(() =>
    checkWorkflowContractContents({
      ...baseline,
      releaseWorkflow: `${baseline.releaseWorkflow}${forbidden}`,
    })
  );
}

console.log("workflow contract self-test passed");
