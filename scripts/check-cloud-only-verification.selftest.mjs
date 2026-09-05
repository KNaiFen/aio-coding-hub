import assert from "node:assert/strict";

import {
  assertCloudOnlyVerificationContract,
  loadCloudOnlyVerificationFixture,
} from "./check-cloud-only-verification.mjs";
import { assertGithubActionsEnvironment } from "./require-github-actions.mjs";

const valid = loadCloudOnlyVerificationFixture();
assert.doesNotThrow(() => assertCloudOnlyVerificationContract(valid));

const wordingFixture = {
  ...valid,
  agents: `$gkd-main .gkd/plan.md .gkd/execution.md .gkd/progress.md .gkd/review.md
Keep the local checkout zero-artifact.
普通 PR 等自动 \`ci-gate\` 与 \`pr-title\`，不额外手动启动常规 \`ci\`。`,
  readme: "workflow_dispatch 仅用于 main 恢复或候选构建；不要为常规验证额外手动运行 `ci`。",
  readmeEn: "workflow_dispatch is for main recovery or candidates. Do not start an additional manual `ci` run for routine validation.",
};
assert.doesNotThrow(() => assertCloudOnlyVerificationContract(wordingFixture));
for (const [field, original, equivalent] of [
  ["agents", "Keep the local checkout zero-artifact.", "本地工作树不得留下依赖或构建产物。"],
  [
    "agents",
    "普通 PR 等自动 `ci-gate` 与 `pr-title`，不额外手动启动常规 `ci`。",
    "常规 PR 由自动 `ci-gate` 与 `pr-title` 验证，不再手动重复触发 `ci`。",
  ],
  [
    "readme",
    "不要为常规验证额外手动运行 `ci`。",
    "普通 PR 使用自动检查，常规验证不得再启动一轮手动 `ci`。",
  ],
  [
    "readmeEn",
    "Do not start an additional manual `ci` run for routine validation.",
    "Use automatic PR checks without launching another manual `ci` run for routine validation.",
  ],
]) {
  const rewritten = wordingFixture[field].replace(original, equivalent);
  assert.notEqual(rewritten, wordingFixture[field], `${field} wording fixture must change`);
  assert.doesNotThrow(() =>
    assertCloudOnlyVerificationContract({ ...wordingFixture, [field]: rewritten })
  );
}
assert.doesNotThrow(() =>
  assertCloudOnlyVerificationContract({
    ...valid,
    agents: `${valid.agents}\n$gkd-ci-monitor gkd_accept .gkd/archive/\n`,
    readme: `${valid.readme}\n$gkd-ci-monitor .gkd/execution.md\n`,
    readmeEn: `${valid.readmeEn}\n$gkd-ci-monitor .gkd/execution.md\n`,
  })
);
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
    "legacy GKD references",
    (fixture) => {
      fixture.agents += `\n\`${["gkd", "task"].join("-")}\`\n`;
    },
    /AGENTS\.md contains a prohibited local instruction/,
  ],
  [
    "missing GKD workflow entry",
    (fixture) => {
      fixture.agents = fixture.agents.replaceAll("$gkd-main", "project workflow");
    },
    /AGENTS\.md must include "\$gkd-main"/,
  ],
  [
    "missing worktree handoff file",
    (fixture) => {
      fixture.agents = fixture.agents.replaceAll("progress.md", "status.md");
    },
    /AGENTS\.md must include "\.gkd\/progress\.md"/,
  ],
  [
    "legacy plan-only execution handoff",
    (fixture) => {
      fixture.agents = fixture.agents.replaceAll(".gkd/execution.md", ".gkd/plan.md");
    },
    /AGENTS\.md must include "\.gkd\/execution\.md"/,
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
