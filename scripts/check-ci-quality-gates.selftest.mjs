import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import {
  assertCiQualityGates,
  workflowJobProperty,
  workflowStepRun,
  workflowSteps,
} from "./check-ci-quality-gates.mjs";
import { runClassifier } from "./ci-change-scope.mjs";

const guard = "node scripts/require-github-actions.mjs && ";
const packageJson = {
  scripts: {
    "check:no-instant-now-sub":
      `${guard}node scripts/check-no-instant-now-sub.selftest.mjs && node scripts/check-no-instant-now-sub.mjs`,
    "create-aio-plugin:typecheck":
      `${guard}node scripts/check-create-aio-plugin-typecheck.selftest.mjs && pnpm --filter create-aio-plugin typecheck`,
    "check:ci-quality-gates":
      `${guard}node scripts/check-ci-quality-gates.selftest.mjs && node scripts/check-ci-quality-gates.mjs`,
    "test:unit:coverage": `${guard}vitest run --coverage`,
  },
};
const readFixture = (relativePath) =>
  readFileSync(new URL(`../${relativePath}`, import.meta.url), "utf8");
const ciWorkflow = readFixture(".github/workflows/ci.yml");
const prTitleWorkflow = readFixture(".github/workflows/pr-title.yml");
const performanceWorkflow = readFixture(".github/workflows/performance.yml");
const codeqlWorkflow = readFixture(".github/workflows/codeql.yml");
const dependabotConfig = readFixture(".github/dependabot.yml");
const vitestConfig = readFixture("vitest.config.ts");

const valid = {
  codeqlWorkflow,
  dependabotConfig,
  packageJson,
  vitestConfig,
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
    "observer macOS activity command",
    { ...valid, ciWorkflow: ciWorkflow.replace("        run: |\n          cargo test --manifest-path src-tauri/Cargo.toml --locked --lib app::observer::activity -- --test-threads=1\n", "") },
    /observer-macos must retain the approved fail-closed script/,
  ],
  [
    "observer macOS gate dependency",
    { ...valid, ciWorkflow: ciWorkflow.replace("      - observer-macos\n", "") },
    /ci-gate must include - observer-macos/,
  ],
  [
    "observer macOS failed result accepted",
    { ...valid, ciWorkflow: ciWorkflow.replace('[[ "$OBSERVER_MACOS_RESULT" == "success" ]]', '[[ "$OBSERVER_MACOS_RESULT" != "cancelled" ]]') },
    /approved fail-closed aggregation script/,
  ],
  [
    "observer macOS missing skipped validation",
    { ...valid, ciWorkflow: ciWorkflow.replace('            [[ "$OBSERVER_MACOS_RESULT" == "skipped" ]]\n', "") },
    /approved fail-closed aggregation script/,
  ],
  [
    "dedicated E2E package entry",
    {
      ...valid,
      packageJson: {
        scripts: { ...packageJson.scripts, "test:e2e": `${guard}vitest run src/e2e` },
      },
    },
    /test:e2e must stay absent/,
  ],
  [
    "root coverage package entry",
    {
      ...valid,
      packageJson: {
        scripts: { ...packageJson.scripts, "test:unit:coverage": `${guard}vitest run` },
      },
    },
    /test:unit:coverage must remain/,
  ],
  [
    "root E2E discovery",
    { ...valid, vitestConfig: vitestConfig.replace('    include: ["src/**/*.{test,spec}.{ts,tsx}"],\n', "") },
    /vitest\.config\.ts must discover src\/e2e/,
  ],
  [
    "contracts cloud-only step",
    {
      ...valid,
      ciWorkflow: ciWorkflow.replace(
        "        run: node scripts/check-cloud-only-verification.mjs\n",
        ""
      ),
    },
    /ci\.yml contracts must include node scripts\/check-cloud-only-verification/,
  ],
  [
    "frontend build",
    { ...valid, ciWorkflow: ciWorkflow.replace("        run: pnpm build\n", "") },
    /ci\.yml frontend must include pnpm build/,
  ],
  [
    "dedicated frontend E2E step",
    {
      ...valid,
      ciWorkflow: ciWorkflow.replace(
        "      - name: Unit tests\n",
        "      - name: Plugin GUI E2E smoke\n        run: pnpm test:e2e\n\n      - name: Unit tests\n"
      ),
    },
    /ci\.yml frontend must not run a dedicated pnpm test:e2e step/,
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
    /ci\.yml contracts must include node scripts\/check-github-actions-pin-policy/,
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
    /ci\.yml contracts must include node scripts\/check-github-actions-pin-policy/,
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
    "contracts job cannot inherit a skipped manual guard",
    {
      ...valid,
      ciWorkflow: ciWorkflow.replace(
        "    if: >-\n      always() &&\n      needs.change-scope.result == 'success' &&\n      (needs.change-scope.outputs.docs_checks == 'true' ||\n      needs.change-scope.outputs.frontend_ci == 'true' ||\n      needs.change-scope.outputs.rust_ci == 'true')",
        "    if: needs.change-scope.outputs.docs_checks == 'true'"
      ),
    },
    /ci\.yml contracts if must equal/,
  ],
  [
    "contracts must run for checked docs or either code domain",
    {
      ...valid,
      ciWorkflow: ciWorkflow.replace(
        "      (needs.change-scope.outputs.docs_checks == 'true' ||\n      needs.change-scope.outputs.frontend_ci == 'true' ||\n      needs.change-scope.outputs.rust_ci == 'true')",
        "      needs.change-scope.outputs.full_ci == 'true'"
      ),
    },
    /ci\.yml contracts if must equal/,
  ],
  [
    "source contracts step condition",
    {
      ...valid,
      ciWorkflow: ciWorkflow.replace(
        "      - name: Enforce GitHub Actions pin policy\n        if: needs.change-scope.outputs.frontend_ci == 'true' || needs.change-scope.outputs.rust_ci == 'true'",
        "      - name: Enforce GitHub Actions pin policy\n        if: false"
      ),
    },
    /ci\.yml contracts must include node scripts\/check-github-actions-pin-policy/,
  ],
  [
    "candidate build cannot inherit a skipped manual guard",
    {
      ...valid,
      ciWorkflow: ciWorkflow.replace(
        "    if: >-\n      always() &&\n      needs.contracts.result == 'success' &&\n      needs.frontend.result == 'success' &&\n      needs.rust.result == 'success' &&\n      needs.candidate-plan.result == 'success' &&\n      needs.candidate-plan.outputs.should_build == 'true'",
        "    if: needs.candidate-plan.outputs.should_build == 'true'"
      ),
    },
    /ci\.yml build-release-candidate if must equal/,
  ],
  [
    "candidate plan main-only boundary",
    {
      ...valid,
      ciWorkflow: ciWorkflow.replace(
        "github.ref == 'refs/heads/main' &&\n      (github.event_name == 'push' || github.event_name == 'workflow_dispatch')",
        "github.ref == 'refs/heads/dev' &&\n      (github.event_name == 'push' || github.event_name == 'workflow_dispatch')"
      ),
    },
    /ci\.yml candidate-plan if must equal/,
  ],
  [
    "candidate plan cannot expand to pull requests",
    {
      ...valid,
      ciWorkflow: ciWorkflow.replace(
        "(github.event_name == 'push' || github.event_name == 'workflow_dispatch')",
        "(github.event_name == 'push' || github.event_name == 'workflow_dispatch' || github.event_name == 'pull_request')"
      ),
    },
    /ci\.yml candidate-plan if must equal/,
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
    "aggregate gate must consume the real frontend-CI output",
    {
      ...valid,
      ciWorkflow: ciWorkflow.replace(
        "          FRONTEND_CI: ${{ needs.change-scope.outputs.frontend_ci }}",
        "          FRONTEND_CI: 'true'"
      ),
    },
    /ci\.yml ci-gate must bind aggregation results directly from needs\.\*/,
  ],
  [
    "aggregate gate script cannot be changed",
    {
      ...valid,
      ciWorkflow: ciWorkflow.replace(
        "          set -euo pipefail\n\n          if [[ \"$EVENT_NAME\" == \"workflow_dispatch\" ]]; then",
        "          set -euo pipefail\n\n          if false; then\n            :\n          fi\n\n          if [[ \"$EVENT_NAME\" == \"workflow_dispatch\" ]]; then"
      ),
    },
    /ci\.yml ci-gate must retain the approved fail-closed aggregation script/,
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
    "CodeQL analysis cannot ignore failures",
    {
      ...valid,
      codeqlWorkflow: codeqlWorkflow.replace(
        "      - name: Analyze\n",
        "      - name: Analyze\n        continue-on-error: true\n"
      ),
    },
    /codeql\.yml Analyze must not ignore failures/,
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

for (const job of ["frontend", "rust", "observer-macos"]) {
  for (const needs of ["change-scope", "contracts", "[]"]) {
    assert.throws(
      () => assertCiQualityGates({
        ...valid,
        ciWorkflow: ciWorkflow.replace(
          `  ${job}:\n    needs: [change-scope, contracts]`,
          `  ${job}:\n    needs: ${needs}`
        ),
      }),
      new RegExp(`ci.yml ${job} must need change-scope and contracts`),
      `${job} needs ${needs}`
    );
  }
  const condition = workflowJobProperty(ciWorkflow, job, "if");
  for (const clause of [
    "always() && ",
    "needs.change-scope.result == 'success' && ",
    "needs.contracts.result == 'success' && ",
  ]) {
    assert.throws(
      () => assertCiQualityGates({
        ...valid,
        ciWorkflow: ciWorkflow.replace(
          `  ${job}:\n    needs: [change-scope, contracts]\n    if: >-\n      ${condition.split(" && ").join(" &&\n      ")}`,
          `  ${job}:\n    needs: [change-scope, contracts]\n    if: ${condition.replace(clause, "")}`
        ),
      }),
      new RegExp(`ci.yml ${job} if must equal`),
      `${job} missing ${clause}`
    );
  }
}

for (const [name, from, to, expected] of [
  ["classifier automatic events", "if: github.event_name == 'push' || github.event_name == 'pull_request'", "if: always()", /change-scope must run only for push and pull_request/],
  ["classifier full history", "fetch-depth: 0", "fetch-depth: 1", /change-scope must checkout full history/],
  ["classifier Node version", "node-version: 22", "node-version: 20", /change-scope must use Node 22/],
  ["classifier read-only permission", "      contents: read", "      contents: write", /change-scope must grant only contents: read/],
  ["classifier timeout", "timeout-minutes: 10", "timeout-minutes: 45", /change-scope must use ubuntu-latest with a 10-minute timeout/],
  ["classifier command", "node scripts/ci-change-scope.mjs", "echo scripts/ci-change-scope.mjs", /change-scope must invoke the existing classifier/],
  ["classifier command inputs", '--before "$CI_BEFORE_SHA"', "", /change-scope must invoke the existing classifier/],
  ["classifier skipped step", "      - id: scope\n", "      - id: scope\n        if: false\n", /change-scope must retain checkout, Node setup, and classification steps/],
  ["classifier ignored job failure", "  change-scope:\n", "  change-scope:\n    continue-on-error: true\n", /change-scope must use only the approved canonical job properties/],
  ["analysis dependency", "    needs: change-scope\n", "", /analyze must need change-scope/],
  ["analysis classifier result", "needs.change-scope.result == 'success' &&", "", /analyze must skip only proven documentation/],
  ["analysis cancellation", "!cancelled() &&", "always() &&", /analyze must skip only proven documentation/],
  ["analysis implicit success", "!cancelled() &&", "", /analyze must skip only proven documentation/],
  ["analysis absent frontend output", "needs.change-scope.outputs.frontend_ci == 'false'", "needs.change-scope.outputs.frontend_ci != 'true'", /analyze must skip only proven documentation/],
  ["analysis absent Rust output", "needs.change-scope.outputs.rust_ci == 'false'", "needs.change-scope.outputs.rust_ci != 'true'", /analyze must skip only proven documentation/],
  ["analysis ignored job failure", "    timeout-minutes: 45\n", "    timeout-minutes: 45\n    continue-on-error: true\n", /analyze must use only the approved canonical job properties/],
  ["workflow path filter", "  push:\n", "  push:\n    paths-ignore: ['**/*.md']\n", /must not filter workflow paths/],
]) {
  assert.notEqual(codeqlWorkflow.replace(from, to), codeqlWorkflow, name);
  assert.throws(
    () => assertCiQualityGates({ ...valid, codeqlWorkflow: codeqlWorkflow.replace(from, to) }),
    expected,
    name
  );
}
for (const output of ["scope", "frontend_ci", "rust_ci"]) {
  assert.throws(
    () => assertCiQualityGates({
      ...valid,
      codeqlWorkflow: codeqlWorkflow.replace(
        `      ${output}: \${{ steps.scope.outputs.${output} }}`,
        `      ${output}: ''`
      ),
    }),
    /change-scope must expose the classifier scope and domain outputs/,
    output
  );
}
for (const input of ["CI_EVENT_NAME", "CI_BASE_SHA", "CI_HEAD_SHA", "CI_BEFORE_SHA"]) {
  assert.throws(
    () => assertCiQualityGates({
      ...valid,
      codeqlWorkflow: codeqlWorkflow.replace(new RegExp(`          ${input}: .*`), `          ${input}: ''`),
    }),
    /change-scope must bind the event and exact diff SHAs/,
    input
  );
}

// Evaluate the actual workflow conditions, with only Actions context syntax adapted to JavaScript.
function jobSelected(workflow, job, { eventName = "pull_request", result = "success", outputs = {}, contractsResult = "success", cancelled = false } = {}) {
  const condition = workflowJobProperty(workflow, job, "if")
    .replace(/^\$\{\{\s*|\s*\}\}$/g, "")
    .replaceAll("needs.change-scope", 'needs["change-scope"]');
  return new Function("github", "needs", "always", "cancelled", `return (${condition});`)(
    { event_name: eventName },
    { "change-scope": { result, outputs }, contracts: { result: contractsResult } },
    () => true,
    () => cancelled
  );
}

const policyPath = fileURLToPath(new URL("../.github/ci-scope.json", import.meta.url));
const gateStep = workflowSteps(ciWorkflow, "ci-gate").find(
  (step) => step.properties.get("name") === "Require expected jobs"
);
const gateRun = workflowStepRun(gateStep).value;
function gateStatus(env) {
  const result = spawnSync("bash", ["-c", gateRun], {
    encoding: "utf8",
    env: { ...process.env, ...env },
  });
  if (result.error) throw result.error;
  assert.equal(result.signal, null, result.stderr);
  return result.status;
}

for (const eventName of ["pull_request", "push"]) {
  for (const [name, diff, analyze] of [
    ["process documents", "M\0.gkd/plan.md\0M\0.gkd/progress.md\0", false],
    ["checked documents", "M\0README.md\0M\0AGENTS.md\0M\0.trellis/spec/example/rule.md\0", false],
    ["frontend", "M\0src/main.tsx\0", true],
    ["Rust", "M\0src-tauri/src/lib.rs\0", true],
    ["shared", "M\0src/generated/bindings.ts\0", true],
    ["unknown", "M\0.gkd/state.json\0", true],
    ["mixed domains", "M\0src/main.tsx\0M\0src-tauri/src/lib.rs\0", true],
    ["mixed documents and source", "M\0.gkd/progress.md\0M\0src/main.tsx\0", true],
    ["control plane", "M\0.github/workflows/codeql.yml\0", true],
    ["empty diff", "", true],
    ["deleted checked document", "D\0docs/removed.md\0", false],
    ["deleted source", "D\0src-tauri/src/removed.rs\0", true],
    ["archived process document", "R100\0.gkd/progress.md\0.gkd/archive/workflow/progress.md\0", false],
    ["source moved to documentation", "R100\0src/main.tsx\0.gkd/archive/workflow/main.md\0", true],
    ["cross-domain rename", "R100\0src/old.tsx\0src-tauri/src/new.rs\0", true],
    ["cross-domain copy", "C090\0src-tauri/src/old.rs\0src/new.tsx\0", true],
    ["documentation copy", "C090\0README.md\0docs/copy.md\0", false],
    ["documentation copied to source", "C090\0docs/old.md\0src/copied.tsx\0", true],
    ["invalid diff", "M\0README.md", true],
    ["Git failure", null, true],
  ]) {
    const classified = runClassifier({
      eventName,
      baseSha: "a".repeat(40),
      beforeSha: "a".repeat(40),
      headSha: "b".repeat(40),
      policyPath,
    }, (args) => {
      if (diff === null) throw new Error("fixture Git failure");
      return args[0] === "merge-base" ? "c".repeat(40) : diff;
    });
    const outputs = {
      scope: classified.scope,
      frontend_ci: String(classified.frontendCi),
      rust_ci: String(classified.rustCi),
    };
    const label = `${eventName}: ${name}`;
    assert.equal(jobSelected(codeqlWorkflow, "change-scope", { eventName }), true, label);
    assert.equal(jobSelected(codeqlWorkflow, "analyze", { eventName, outputs }), analyze, label);
    for (const [job, selected] of [
      ["frontend", classified.frontendCi],
      ["rust", classified.rustCi],
      ["observer-macos", classified.rustCi],
    ]) {
      for (const contractsResult of ["success", "failure", "cancelled", "skipped", ""]) {
        for (const result of ["success", "failure", "cancelled", "skipped", ""]) {
          assert.equal(
            jobSelected(ciWorkflow, job, { eventName, result, outputs, contractsResult }),
            selected && contractsResult === "success" && result === "success",
            `${label}: ${job}, classifier ${result}, contracts ${contractsResult}`
          );
        }
      }
    }
    const env = {
      EVENT_NAME: eventName,
      EVENT_REF: eventName === "push" ? "refs/heads/dev" : "refs/pull/1/merge",
      MANUAL_GUARD_RESULT: "skipped",
      CHANGE_SCOPE_RESULT: "success",
      SCOPE: classified.scope,
      FULL_CI: String(classified.fullCi),
      FRONTEND_CI: outputs.frontend_ci,
      RUST_CI: outputs.rust_ci,
      SHARED_CI: String(classified.sharedCi),
      DOCS_CHECKS: String(classified.docsChecks),
      CONTRACTS_RESULT: classified.docsChecks || classified.frontendCi || classified.rustCi ? "success" : "skipped",
      FRONTEND_RESULT: classified.frontendCi ? "success" : "skipped",
      RUST_RESULT: classified.rustCi ? "success" : "skipped",
      OBSERVER_MACOS_RESULT: classified.rustCi ? "success" : "skipped",
      PLAN_RESULT: "skipped",
      SHOULD_BUILD: "",
      BUILD_RESULT: "skipped",
      TUI_BUILD_RESULT: "skipped",
      ASSEMBLE_RESULT: "skipped",
    };
    assert.equal(gateStatus(env), 0, label);
    for (const key of ["CHANGE_SCOPE_RESULT", "CONTRACTS_RESULT", "FRONTEND_RESULT", "RUST_RESULT", "OBSERVER_MACOS_RESULT"]) {
      if (env[key] !== "success") continue;
      for (const result of ["failure", "cancelled", "skipped", ""]) {
        assert.notEqual(gateStatus({ ...env, [key]: result }), 0, `${label}: ${key} ${result}`);
      }
    }
    if (classified.frontendCi || classified.rustCi) {
      for (const result of ["failure", "cancelled", "skipped"]) {
        assert.notEqual(gateStatus({
          ...env,
          CONTRACTS_RESULT: result,
          FRONTEND_RESULT: "skipped",
          RUST_RESULT: "skipped",
          OBSERVER_MACOS_RESULT: "skipped",
        }), 0, `${label}: blocked by contracts ${result}`);
      }
    }
  }
}

for (const eventName of ["schedule", "workflow_dispatch"]) {
  assert.equal(jobSelected(codeqlWorkflow, "change-scope", { eventName }), false, eventName);
  assert.equal(jobSelected(codeqlWorkflow, "analyze", { eventName, result: "skipped" }), true, eventName);
}
for (const scope of ["process-docs", "checked-docs"]) {
  const docs = { scope, frontend_ci: "false", rust_ci: "false" };
  for (const result of ["failure", "cancelled", "skipped", ""]) {
    assert.equal(jobSelected(codeqlWorkflow, "analyze", { result, outputs: docs }), true, `${scope}: ${result}`);
  }
  for (const key of ["scope", "frontend_ci", "rust_ci"]) {
    for (const value of [undefined, "", "true", "unknown"]) {
      const outputs = { ...docs, [key]: value };
      assert.equal(jobSelected(codeqlWorkflow, "analyze", { outputs }), true, `${key}: ${value}`);
    }
  }
}
assert.equal(jobSelected(codeqlWorkflow, "analyze"), true, "missing classifier outputs");
assert.equal(jobSelected(codeqlWorkflow, "analyze", { cancelled: true }), false, "workflow cancelled");

console.error("[ci-quality-gates:selftest] all assertions passed");
