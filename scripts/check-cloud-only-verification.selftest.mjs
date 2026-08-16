import assert from "node:assert/strict";

import {
  assertCloudOnlyVerificationContract,
  loadCloudOnlyVerificationFixture,
} from "./check-cloud-only-verification.mjs";
import { assertGithubActionsEnvironment } from "./require-github-actions.mjs";

const valid = loadCloudOnlyVerificationFixture();
assert.doesNotThrow(() => assertCloudOnlyVerificationContract(valid));
assert.doesNotThrow(() =>
  assertCloudOnlyVerificationContract({
    ...valid,
    ciWorkflow: valid.ciWorkflow.replace("    steps:\n", "    steps: # executable steps\n"),
  })
);
assert.throws(() => assertGithubActionsEnvironment({}), /GitHub Actions-only/);
assert.doesNotThrow(() => assertGithubActionsEnvironment({ GITHUB_ACTIONS: "true" }));

function cloneFixture() {
  return {
    ...structuredClone({
      ...valid,
      activeSpecs: undefined,
    }),
    activeSpecs: new Map(valid.activeSpecs),
  };
}

function expectContractFailure(name, mutate, expected) {
  const fixture = cloneFixture();
  mutate(fixture);
  assert.throws(() => assertCloudOnlyVerificationContract(fixture), expected, name);
}

for (const [name, mutate, expected] of [
  [
    "root script guard",
    (fixture) => {
      fixture.rootPackage.scripts.build = "tsc && vite build";
    },
    /root package\.json script build must start/,
  ],
  [
    "local dev entry",
    (fixture) => {
      fixture.rootPackage.scripts.dev = "vite";
    },
    /must not expose local dev entry point/,
  ],
  [
    "dedicated E2E package entry",
    (fixture) => {
      fixture.rootPackage.scripts["test:e2e"] =
        "node scripts/require-github-actions.mjs && vitest run src/e2e";
    },
    /test:e2e must stay absent/,
  ],
  [
    "root coverage entry",
    (fixture) => {
      fixture.rootPackage.scripts["test:unit:coverage"] =
        "node scripts/require-github-actions.mjs && vitest run";
    },
    /test:unit:coverage must remain/,
  ],
  [
    "root E2E discovery",
    (fixture) => {
      fixture.vitestConfig = fixture.vitestConfig.replace(
        '    include: ["src/**/*.{test,spec}.{ts,tsx}"],\n',
        ""
      );
    },
    /vitest\.config\.ts must discover src\/e2e/,
  ],
  [
    "explicit E2E exclusion",
    (fixture) => {
      fixture.vitestConfig = fixture.vitestConfig.replace(
        '    exclude: ["**/node_modules/**", ".codex-temp/**", "packages/**"],',
        '    exclude: ["**/node_modules/**", ".codex-temp/**", "packages/**", "src/e2e/**"],'
      );
    },
    /vitest\.config\.ts must discover src\/e2e/,
  ],
  [
    "workspace guard",
    (fixture) => {
      fixture.pluginSdkPackage.scripts.test = "vitest run";
    },
    /plugin SDK package\.json script test must start/,
  ],
  [
    "README local install",
    (fixture) => {
      fixture.readme += "\n```bash\npnpm install\n```\n";
    },
    /README\.md must not present a package\/native command/,
  ],
  [
    "Trellis local lint instruction",
    (fixture) => {
      fixture.trellisWorkflow += "\nRun project lint and type-check\n";
    },
    /\.trellis\/workflow\.md contains a prohibited local instruction/,
  ],
  [
    "GKD role skill prefix",
    (fixture) => {
      fixture.roleSkills[0].instructions = fixture.roleSkills[0].instructions.replace(
        "name: gkd-main",
        "name: main"
      );
    },
    /gkd-main\/SKILL\.md must include "name: gkd-main"/,
  ],
  [
    "active spec bare cargo command",
    (fixture) => {
      fixture.activeSpecs.set("cross-layer/example.md", "cargo test --locked\n");
    },
    /active spec cross-layer\/example\.md contains a bare local package\/native command/,
  ],
  [
    "active spec local quality instruction",
    (fixture) => {
      fixture.activeSpecs.set("cross-layer/example.md", "- Run the full Rust suite after this change.\n");
    },
    /active spec cross-layer\/example\.md contains a local quality-gate instruction/,
  ],
  [
    "Tauri local dev hook",
    (fixture) => {
      fixture.tauriConfig.build.beforeDevCommand = "pnpm dev";
    },
    /must not define beforeDevCommand/,
  ],
  [
    "manual dev-build",
    (fixture) => {
      fixture.devBuildWorkflow = fixture.devBuildWorkflow.replace(/^\s*workflow_dispatch:\s*$/m, "");
    },
    /dev-build\.yml must declare only the workflow_dispatch trigger/,
  ],
  [
    "dev-build pull request trigger",
    (fixture) => {
      fixture.devBuildWorkflow = fixture.devBuildWorkflow.replace(
        "  workflow_dispatch:\n",
        "  pull_request:\n  workflow_dispatch:\n"
      );
    },
    /dev-build\.yml must declare only the workflow_dispatch trigger/,
  ],
  [
    "manual CI guard condition",
    (fixture) => {
      fixture.ciWorkflow = fixture.ciWorkflow.replace(
        "    if: github.event_name == 'workflow_dispatch'",
        "    if: true"
      );
    },
    /ci\.yml manual-dispatch-guard if must equal/,
  ],
  [
    "manual CI main boundary",
    (fixture) => {
      fixture.ciWorkflow = fixture.ciWorkflow.replace(
        '[[ "$EVENT_REF" == "refs\/heads\/main" ]]',
        '[[ -n "$EVENT_REF" ]]'
      );
    },
    /manual-dispatch-guard must retain the approved fail-closed script/,
  ],
  [
    "manual CI gate name",
    (fixture) => {
      fixture.ciWorkflow = fixture.ciWorkflow.replace(
        "    name: ${{ github.event_name == 'workflow_dispatch' && 'manual-ci-gate' || 'ci-gate' }}",
        "    name: ci-gate"
      );
    },
    /ci\.yml ci-gate name must equal/,
  ],
  [
    "contracts cloud-only step",
    (fixture) => {
      fixture.ciWorkflow = fixture.ciWorkflow.replace(
        "        run: node scripts/check-cloud-only-verification.mjs",
        "        run: true"
      );
    },
    /ci\.yml contracts must include node scripts\/check-cloud-only-verification\.mjs/,
  ],
  [
    "contracts cloud-only step cannot be conditional",
    (fixture) => {
      fixture.ciWorkflow = fixture.ciWorkflow.replace(
        "      - name: Validate cloud-only verification contract\n        run:",
        "      - name: Validate cloud-only verification contract\n        if: false\n        run:"
      );
    },
    /ci\.yml contracts must include node scripts\/check-cloud-only-verification\.mjs/,
  ],
]) {
  expectContractFailure(name, mutate, expected);
}

expectContractFailure(
  "candidate PR skip boundary",
  (fixture) => {
    fixture.ciWorkflow = fixture.ciWorkflow.replace(
      `            else
              [[ "$PLAN_RESULT" == "skipped" ]]
              [[ "$BUILD_RESULT" == "skipped" ]]
              [[ "$TUI_BUILD_RESULT" == "skipped" ]]
              [[ "$ASSEMBLE_RESULT" == "skipped" ]]
            fi
          else`,
      `            else
              [[ "$PLAN_RESULT" == "skipped" ]]
              [[ "$BUILD_RESULT" == "skipped" ]]
              [[ "$TUI_BUILD_RESULT" == "success" ]]
              [[ "$ASSEMBLE_RESULT" == "skipped" ]]
            fi
          else`
    );
  },
  /ci\.yml ci-gate must require candidate desktop\/TUI jobs to be skipped/
);

expectContractFailure(
  "candidate boundary must remain under full CI",
  (fixture) => {
    fixture.ciWorkflow = fixture.ciWorkflow.replace(
      'if [[ "$FULL_CI" == "true" ]]; then',
      'if [[ "$FULL_CI" == "false" ]]; then'
    );
  },
  /ci\.yml ci-gate must require candidate desktop\/TUI jobs to be skipped/
);

console.error("[cloud-only-verification:selftest] all assertions passed");
