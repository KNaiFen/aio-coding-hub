import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { assertCiQualityGates } from "./check-ci-quality-gates.mjs";

const guard = "node scripts/require-github-actions.mjs && ";
const packageJson = {
  scripts: {
    "check:no-instant-now-sub":
      `${guard}node scripts/check-no-instant-now-sub.selftest.mjs && node scripts/check-no-instant-now-sub.mjs`,
    "create-aio-plugin:typecheck":
      `${guard}node scripts/check-create-aio-plugin-typecheck.selftest.mjs && pnpm --filter create-aio-plugin typecheck`,
    "check:ci-quality-gates":
      `${guard}node scripts/check-ci-quality-gates.selftest.mjs && node scripts/check-ci-quality-gates.mjs`,
  },
};
const checks = {
  "cloud-only-verification": "node scripts/check-cloud-only-verification.mjs",
  "ci-quality-gates": "pnpm check:ci-quality-gates",
  "create-aio-plugin-typecheck": "pnpm create-aio-plugin:typecheck",
};
const stages = {
  "full-ci": [
    "cloud-only-verification",
    "no-instant-now-sub",
    "ci-quality-gates",
    "create-aio-plugin-typecheck",
  ],
  "plugin-hardening": ["cloud-only-verification", "create-aio-plugin-typecheck"],
};
const readFixture = (relativePath) =>
  readFileSync(new URL(`../${relativePath}`, import.meta.url), "utf8");
const ciWorkflow = readFixture(".github/workflows/ci.yml");
const prTitleWorkflow = readFixture(".github/workflows/pr-title.yml");
const performanceWorkflow = readFixture(".github/workflows/performance.yml");
const codeqlWorkflow = readFixture(".github/workflows/codeql.yml");
const dependabotConfig = readFixture(".github/dependabot.yml");

const valid = {
  codeqlWorkflow,
  dependabotConfig,
  packageJson,
  checks,
  stages,
  ciWorkflow,
  performanceWorkflow,
  prTitleWorkflow,
};
assert.doesNotThrow(() => assertCiQualityGates(valid));
assert.doesNotThrow(() =>
  assertCiQualityGates({
    ...valid,
    ciWorkflow: ciWorkflow.replace("    steps:\n", "    steps: # executable steps\n"),
  })
);

for (const [name, fixture, expected] of [
  [
    "full CI Instant gate",
    {
      ...valid,
      stages: {
        ...stages,
        "full-ci": [stages["full-ci"][0], ...stages["full-ci"].slice(2)],
      },
    },
    /STAGES\.full-ci must include no-instant-now-sub/,
  ],
  [
    "cloud contract stage wiring",
    { ...valid, stages: { ...stages, "full-ci": [] } },
    /STAGES\.full-ci must include cloud-only-verification/,
  ],
  [
    "plugin hardening typecheck",
    { ...valid, stages: { ...stages, "plugin-hardening": [] } },
    /STAGES\.plugin-hardening must include cloud-only-verification/,
  ],
  [
    "support cloud-only step",
    {
      ...valid,
      ciWorkflow: ciWorkflow.replace(
        "        run: node scripts/check-cloud-only-verification.selftest.mjs && node scripts/check-cloud-only-verification.mjs\n",
        ""
      ),
    },
    /ci\.yml support-contract must include node scripts\/check-cloud-only-verification/,
  ],
  [
    "frontend build",
    { ...valid, ciWorkflow: ciWorkflow.replace("        run: pnpm build\n", "") },
    /ci\.yml frontend must include pnpm build/,
  ],
  [
    "Actions pin policy step",
    {
      ...valid,
      ciWorkflow: ciWorkflow.replace(
        "        run: node scripts/check-github-actions-pin-policy.selftest.mjs && node scripts/check-github-actions-pin-policy.mjs\n",
        ""
      ),
    },
    /ci\.yml support-contract must include node scripts\/check-github-actions-pin-policy/,
  ],
  [
    "Actions pin policy comment is not executable",
    {
      ...valid,
      ciWorkflow: ciWorkflow.replace(
        "        run: node scripts/check-github-actions-pin-policy.selftest.mjs && node scripts/check-github-actions-pin-policy.mjs",
        "        # run: node scripts/check-github-actions-pin-policy.selftest.mjs && node scripts/check-github-actions-pin-policy.mjs"
      ),
    },
    /ci\.yml support-contract must include node scripts\/check-github-actions-pin-policy/,
  ],
  [
    "frontend env text is not executable",
    {
      ...valid,
      ciWorkflow: ciWorkflow.replace("        run: pnpm build", "        env:\n          run: pnpm build"),
    },
    /ci\.yml frontend must include pnpm build/,
  ],
  [
    "frontend echoed command is not executable",
    {
      ...valid,
      ciWorkflow: ciWorkflow.replace("        run: pnpm build", "        run: echo pnpm build"),
    },
    /ci\.yml frontend must include pnpm build/,
  ],
  [
    "frontend ignored failure is not a quality gate",
    {
      ...valid,
      ciWorkflow: ciWorkflow.replace("        run: pnpm build", "        run: pnpm build || true"),
    },
    /ci\.yml frontend must include pnpm build/,
  ],
  [
    "frontend conditional no-op is not executable",
    {
      ...valid,
      ciWorkflow: ciWorkflow.replace(
        "        run: pnpm build",
        "        run: if false; then pnpm build; fi"
      ),
    },
    /ci\.yml frontend must include pnpm build/,
  ],
  [
    "frontend step condition cannot skip a required command",
    {
      ...valid,
      ciWorkflow: ciWorkflow.replace(
        "      - name: Build frontend\n        run: pnpm build",
        "      - name: Build frontend\n        if: ${{ false }}\n        run: pnpm build"
      ),
    },
    /ci\.yml frontend must include pnpm build/,
  ],
  [
    "frontend required command cannot ignore failures",
    {
      ...valid,
      ciWorkflow: ciWorkflow.replace(
        "      - name: Build frontend\n        run: pnpm build",
        "      - name: Build frontend\n        continue-on-error: true\n        run: pnpm build"
      ),
    },
    /ci\.yml frontend must include pnpm build/,
  ],
  [
    "frontend non-canonical condition cannot hide a required command",
    {
      ...valid,
      ciWorkflow: ciWorkflow.replace(
        "      - name: Build frontend\n        run: pnpm build",
        "      - name: Build frontend\n        if : ${{ false }}\n        run: pnpm build"
      ),
    },
    /ci\.yml frontend must include pnpm build/,
  ],
  [
    "Rust clippy",
    {
      ...valid,
      ciWorkflow: ciWorkflow.replace(
        "        run: cargo clippy --workspace --all-targets --locked -- -D warnings\n",
        ""
      ),
    },
    /ci\.yml rust must include cargo clippy/,
  ],
  [
    "manual gate name",
    {
      ...valid,
      ciWorkflow: ciWorkflow.replace(
        "    name: ${{ github.event_name == 'workflow_dispatch' && 'manual-ci-gate' || 'ci-gate' }}",
        "    name: ci-gate"
      ),
    },
    /ci\.yml ci-gate must include name:/,
  ],
  [
    "aggregate gate cannot be skipped",
    {
      ...valid,
      ciWorkflow: ciWorkflow.replace(
        "  ci-gate:\n    name: ${{ github.event_name == 'workflow_dispatch' && 'manual-ci-gate' || 'ci-gate' }}\n    if: always()",
        "  ci-gate:\n    name: ${{ github.event_name == 'workflow_dispatch' && 'manual-ci-gate' || 'ci-gate' }}\n    if: false"
      ),
    },
    /ci\.yml ci-gate must use if: always\(\)/,
  ],
  [
    "docs contract cannot inherit a skipped manual guard",
    {
      ...valid,
      ciWorkflow: ciWorkflow.replace(
        "    if: >-\n      always() &&\n      needs.change-scope.result == 'success' &&\n      needs.change-scope.outputs.docs_checks == 'true'",
        "    if: needs.change-scope.outputs.docs_checks == 'true'"
      ),
    },
    /ci\.yml docs-contract if must equal/,
  ],
  [
    "frontend must require the support contract to succeed",
    {
      ...valid,
      ciWorkflow: ciWorkflow.replace(
        "      needs.change-scope.outputs.full_ci == 'true' &&\n      needs.support-contract.result == 'success'\n    runs-on: ubuntu-latest",
        "      needs.change-scope.outputs.full_ci == 'true'\n    runs-on: ubuntu-latest"
      ),
    },
    /ci\.yml frontend if must equal/,
  ],
  [
    "candidate build cannot inherit a skipped manual guard",
    {
      ...valid,
      ciWorkflow: ciWorkflow.replace(
        "    if: >-\n      always() &&\n      needs.support-contract.result == 'success' &&\n      needs.frontend.result == 'success' &&\n      needs.rust.result == 'success' &&\n      needs.candidate-plan.result == 'success' &&\n      needs.candidate-plan.outputs.should_build == 'true'",
        "    if: needs.candidate-plan.outputs.should_build == 'true'"
      ),
    },
    /ci\.yml build-release-candidate if must equal/,
  ],
  [
    "candidate assembly must require the plan job to succeed",
    {
      ...valid,
      ciWorkflow: ciWorkflow.replace(
        "      needs.candidate-plan.result == 'success' &&\n      needs.candidate-plan.outputs.should_build == 'true' &&\n      needs.frontend.result == 'success'",
        "      needs.candidate-plan.outputs.should_build == 'true' &&\n      needs.frontend.result == 'success'"
      ),
    },
    /ci\.yml assemble-release-candidate if must equal/,
  ],
  [
    "aggregate gate must consume the real full-CI output",
    {
      ...valid,
      ciWorkflow: ciWorkflow.replace(
        "          FULL_CI: ${{ needs.change-scope.outputs.full_ci }}",
        "          FULL_CI: 'true'"
      ),
    },
    /ci\.yml ci-gate must bind aggregation results directly from needs\.\*/,
  ],
  [
    "independent PR title",
    { ...valid, prTitleWorkflow: "" },
    /pr-title\.yml pr-title must retain the approved fail-closed script/,
  ],
  [
    "PR title validation command",
    { ...valid, prTitleWorkflow: prTitleWorkflow.replace('[[ "$PR_TITLE" =~ $pattern ]]', "true") },
    /pr-title\.yml pr-title must retain the approved fail-closed script/,
  ],
  [
    "performance benchmark",
    { ...valid, performanceWorkflow: "" },
    /performance\.yml provider-trend-benchmark must retain the approved fail-closed script/,
  ],
  [
    "CodeQL Rust language",
    {
      ...valid,
      codeqlWorkflow: codeqlWorkflow.replace(
        "          - language: rust\n            build-mode: none\n",
        ""
      ),
    },
    /codeql\.yml must retain the approved language\/build-mode pairs/,
  ],
  [
    "CodeQL privileged trigger",
    {
      ...valid,
      codeqlWorkflow: codeqlWorkflow.replace("  pull_request:\n", "  pull_request_target:\n"),
    },
    /codeql\.yml must declare only push, pull_request/,
  ],
  [
    "CodeQL duplicate quoted permissions cannot override defaults",
    {
      ...valid,
      codeqlWorkflow: codeqlWorkflow.replace(
        "permissions:\n  contents: read\n  security-events: write",
        'permissions:\n  contents: read\n  security-events: write\n"permissions":\n  contents: write'
      ),
    },
    /codeql\.yml must use only the approved canonical top-level keys/,
  ],
  [
    "CodeQL job permissions cannot override defaults",
    {
      ...valid,
      codeqlWorkflow: codeqlWorkflow.replace(
        "    timeout-minutes: 45\n    strategy:",
        "    timeout-minutes: 45\n    permissions: write-all\n    strategy:"
      ),
    },
    /codeql\.yml analyze must use only the approved canonical job properties/,
  ],
  [
    "CodeQL action comment",
    {
      ...valid,
      codeqlWorkflow: codeqlWorkflow.replace(
        "        uses: github/codeql-action/analyze@5595ccaf912efad79be6eef63a5619ff05969be3 # v4.37.6",
        "        # uses: github/codeql-action/analyze@5595ccaf912efad79be6eef63a5619ff05969be3 # v4.37.6"
      ),
    },
    /codeql\.yml must retain the Analyze action step/,
  ],
  [
    "CodeQL nested with.uses is not an action step",
    {
      ...valid,
      codeqlWorkflow: codeqlWorkflow.replace(
        "        uses: github/codeql-action/init@5595ccaf912efad79be6eef63a5619ff05969be3 # v4.37.6\n        with:\n",
        "        with:\n          uses: github/codeql-action/init@5595ccaf912efad79be6eef63a5619ff05969be3\n"
      ),
    },
    /codeql\.yml must retain the Initialize CodeQL action step/,
  ],
  [
    "CodeQL Rust must use the supported no-build mode",
    {
      ...valid,
      codeqlWorkflow: codeqlWorkflow.replace(
        "          - language: rust\n            build-mode: none",
        "          - language: rust\n            build-mode: autobuild"
      ),
    },
    /codeql\.yml must retain the approved language\/build-mode pairs/,
  ],
  [
    "CodeQL no-build matrix must not retain an Autobuild step",
    {
      ...valid,
      codeqlWorkflow: codeqlWorkflow.replace(
        "      - name: Analyze\n",
        "      - name: Autobuild compiled language\n        uses: github/codeql-action/autobuild@5595ccaf912efad79be6eef63a5619ff05969be3 # v4.37.6\n\n      - name: Analyze\n"
      ),
    },
    /codeql\.yml analyze must contain only checkout, Initialize CodeQL, and Analyze action steps/,
  ],
  [
    "CodeQL initialization cannot be skipped",
    {
      ...valid,
      codeqlWorkflow: codeqlWorkflow.replace(
        "      - name: Initialize CodeQL\n        uses:",
        "      - name: Initialize CodeQL\n        if: ${{ false }}\n        uses:"
      ),
    },
    /codeql\.yml Initialize CodeQL must not be conditionally skipped/,
  ],
  [
    "Dependabot Cargo directory",
    {
      ...valid,
      dependabotConfig: dependabotConfig.replace("    directory: /src-tauri", "    directory: /"),
    },
    /dependabot\.yml cargo directory must be \/src-tauri/,
  ],
  [
    "Dependabot Actions entry",
    {
      ...valid,
      dependabotConfig: dependabotConfig.replace(
        "  - package-ecosystem: github-actions\n    directory: /\n    schedule:\n      interval: weekly\n",
        ""
      ),
    },
    /dependabot\.yml must define exactly one github-actions update entry/,
  ],
]) {
  assert.throws(() => assertCiQualityGates(fixture), expected, name);
}

console.error("[ci-quality-gates:selftest] all assertions passed");
