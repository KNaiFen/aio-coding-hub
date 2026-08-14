import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ACTIONS_GUARD = "node scripts/require-github-actions.mjs && ";
const ROOT_TEST_INCLUDE = 'include: ["src/**/*.{test,spec}.{ts,tsx}"],';
const SOURCE_CONTRACT_STEP_IF =
  "needs.change-scope.outputs.frontend_ci == 'true' || needs.change-scope.outputs.rust_ci == 'true'";
const PLUGIN_CONTRACT_STEP_IF =
  "needs.change-scope.outputs.docs_checks == 'true' || needs.change-scope.outputs.frontend_ci == 'true'";
const DOCS_CONTRACT_STEP_IF = "needs.change-scope.outputs.docs_checks == 'true'";
const RUST_CANONICALIZE_RUN = `set -euo pipefail
cargo fmt --manifest-path src-tauri/Cargo.toml --all
cargo update --manifest-path src-tauri/Cargo.toml --workspace
cargo run --manifest-path src-tauri/Cargo.toml --locked --example export-bindings
pnpm exec prettier --write src/generated/bindings.ts
if git diff --quiet -- src-tauri src/generated/bindings.ts; then
  echo "drift=false" >> "$GITHUB_OUTPUT"
else
  git diff --binary -- src-tauri src/generated/bindings.ts > cloud-native-fixes.patch
  echo "drift=true" >> "$GITHUB_OUTPUT"
fi`;
const PR_TITLE_RUN = `set -euo pipefail
pattern='^(feat|fix|docs|chore|style|refactor|perf|test|ci|build|revert)(\\([^)]+\\))?: .+'
[[ "$PR_TITLE" =~ $pattern ]] || {
  echo "::error::PR title must use Conventional Commits."
  exit 1
}`;
const PERFORMANCE_BENCHMARK_RUN = `set -euo pipefail
started_at="$(date +%s)"
cargo test --release --locked --lib \\
  provider_trend_million_ledger_rows_release_under_one_second -- \\
  --ignored --test-threads=1
duration_seconds="$(( $(date +%s) - started_at ))"
{
  echo "## Provider trend benchmark"
  echo "- Commit: \\\`$GITHUB_SHA\\\`"
  echo "- Rust: 1.90.0"
  echo "- Duration: \${duration_seconds}s"
} >> "$GITHUB_STEP_SUMMARY"`;
const CI_GATE_RESULT_ENV = new Map([
  ["EVENT_NAME", "${{ github.event_name }}"],
  ["EVENT_REF", "${{ github.ref }}"],
  ["MANUAL_GUARD_RESULT", "${{ needs.manual-dispatch-guard.result }}"],
  ["CHANGE_SCOPE_RESULT", "${{ needs.change-scope.result }}"],
  ["SCOPE", "${{ needs.change-scope.outputs.scope }}"],
  ["FULL_CI", "${{ needs.change-scope.outputs.full_ci }}"],
  ["FRONTEND_CI", "${{ needs.change-scope.outputs.frontend_ci }}"],
  ["RUST_CI", "${{ needs.change-scope.outputs.rust_ci }}"],
  ["SHARED_CI", "${{ needs.change-scope.outputs.shared_ci }}"],
  ["DOCS_CHECKS", "${{ needs.change-scope.outputs.docs_checks }}"],
  ["CONTRACTS_RESULT", "${{ needs.contracts.result }}"],
  ["FRONTEND_RESULT", "${{ needs.frontend.result }}"],
  ["RUST_RESULT", "${{ needs.rust.result }}"],
  ["PLAN_RESULT", "${{ needs.candidate-plan.result }}"],
  ["SHOULD_BUILD", "${{ needs.candidate-plan.outputs.should_build }}"],
  ["BUILD_RESULT", "${{ needs.build-release-candidate.result }}"],
  ["TUI_BUILD_RESULT", "${{ needs.build-tui-release-candidate.result }}"],
  ["ASSEMBLE_RESULT", "${{ needs.assemble-release-candidate.result }}"],
]);
const CI_GATE_RUN_SHA256 = "e1b5dce438571ce9cf94e818fa1bd62acf70d964c09a78879733f4367dd0e3ca";
const CODEQL_STRATEGY_BLOCK = `strategy:
  fail-fast: false
  matrix:
    include:
      - language: javascript-typescript
        build-mode: none
      - language: rust
        build-mode: none`;
const CI_JOB_CONDITIONS = new Map([
  [
    "contracts",
    "always() && needs.change-scope.result == 'success' && (needs.change-scope.outputs.docs_checks == 'true' || needs.change-scope.outputs.frontend_ci == 'true' || needs.change-scope.outputs.rust_ci == 'true')",
  ],
  [
    "frontend",
    "always() && needs.change-scope.result == 'success' && needs.change-scope.outputs.frontend_ci == 'true' && needs.contracts.result == 'success'",
  ],
  [
    "rust",
    "always() && needs.change-scope.result == 'success' && needs.change-scope.outputs.rust_ci == 'true' && needs.contracts.result == 'success'",
  ],
  [
    "candidate-plan",
    "always() && needs.change-scope.result == 'success' && needs.change-scope.outputs.full_ci == 'true' && github.repository == 'KNaiFen/aio-coding-hub' && github.ref == 'refs/heads/main' && (github.event_name == 'push' || github.event_name == 'workflow_dispatch')",
  ],
  [
    "build-release-candidate",
    "always() && needs.contracts.result == 'success' && needs.frontend.result == 'success' && needs.rust.result == 'success' && needs.candidate-plan.result == 'success' && needs.candidate-plan.outputs.should_build == 'true'",
  ],
  [
    "build-tui-release-candidate",
    "always() && needs.contracts.result == 'success' && needs.frontend.result == 'success' && needs.rust.result == 'success' && needs.candidate-plan.result == 'success' && needs.candidate-plan.outputs.should_build == 'true'",
  ],
  [
    "assemble-release-candidate",
    "always() && needs.candidate-plan.result == 'success' && needs.candidate-plan.outputs.should_build == 'true' && needs.frontend.result == 'success' && needs.rust.result == 'success' && needs.build-release-candidate.result == 'success' && needs.build-tui-release-candidate.result == 'success'",
  ],
]);

function workflowJobBody(workflow, jobName) {
  const job = new RegExp(`^  ${jobName}:\\s*$`, "m").exec(workflow);
  if (!job) return "";
  const after = workflow.slice(job.index + job[0].length);
  const nextJob = /^  [A-Za-z0-9_-]+:\s*$/m.exec(after);
  return nextJob ? after.slice(0, nextJob.index) : after;
}

function topLevelBlock(source, name) {
  const lines = source.split(/\r?\n/);
  const start = lines.findIndex((line) => line === `${name}:`);
  if (start === -1) return "";

  let end = start + 1;
  while (end < lines.length && (lines[end].trim() === "" || /^\s/.test(lines[end]))) end += 1;
  return lines.slice(start, end).join("\n");
}

function topLevelKeys(source) {
  const keys = [];
  const malformed = [];
  for (const line of source.split(/\r?\n/)) {
    const visible = stripWorkflowComment(line);
    if (!visible.trim() || indentation(visible) !== 0) continue;
    const match = /^([A-Za-z0-9_-]+):(?:\s|$)/.exec(visible);
    if (!match) {
      malformed.push(visible.trim());
    } else {
      keys.push(match[1]);
    }
  }
  return { keys, malformed };
}

function topLevelTriggers(workflow) {
  return [...topLevelBlock(workflow, "on").matchAll(/^  ([A-Za-z][A-Za-z0-9_-]*):/gm)].map(
    (match) => match[1]
  );
}

function dependabotUpdates(source) {
  const updatesStart = /^updates:\s*$/m.exec(source);
  if (!updatesStart) return [];

  const updatesSource = source.slice(updatesStart.index + updatesStart[0].length);
  const matches = [...updatesSource.matchAll(/^  - package-ecosystem:\s*([^\s#]+).*$/gm)];
  return matches.map((match, index) => ({
    ecosystem: match[1].replace(/^['"]|['"]$/g, ""),
    body: updatesSource.slice(
      match.index + match[0].length,
      matches[index + 1]?.index ?? updatesSource.length
    ),
  }));
}

function stripWorkflowComment(line) {
  return line.replace(/(^|\s)#.*$/, "$1").trimEnd();
}

function indentation(line) {
  return line.length - line.trimStart().length;
}

function parseWorkflowStep(lines) {
  const properties = new Map();
  const mappings = new Map();
  const propertyIndexes = [];
  const malformed = [];

  for (let index = 0; index < lines.length; index += 1) {
    const line = stripWorkflowComment(lines[index]);
    if (!line.trim()) continue;
    const indent = indentation(line);
    const pattern =
      index === 0
        ? /^ {6}-\s+([A-Za-z0-9_-]+):\s*(.*)$/
        : indent === 8
          ? /^ {8}([A-Za-z0-9_-]+):\s*(.*)$/
          : undefined;
    if (!pattern) continue;
    const match = pattern.exec(line);
    if (!match) {
      malformed.push(line.trim());
      continue;
    }
    properties.set(match[1], match[2].trim());
    propertyIndexes.push({ index, key: match[1], value: match[2].trim() });
  }

  for (const property of propertyIndexes) {
    if (property.value) continue;
    const mapping = new Map();
    for (let index = property.index + 1; index < lines.length; index += 1) {
      const line = stripWorkflowComment(lines[index]);
      if (!line.trim()) continue;
      const indent = indentation(line);
      if (indent <= 8) break;
      if (indent !== 10) continue;
      const match = /^ {10}([A-Za-z0-9_-]+):\s*(.*)$/.exec(line);
      if (!match) {
        malformed.push(line.trim());
        continue;
      }
      mapping.set(match[1], match[2].trim());
    }
    mappings.set(property.key, mapping);
  }

  return { lines, properties, mappings, malformed };
}

function workflowSteps(workflow, jobName) {
  const body = workflowJobBody(workflow, jobName);
  if (!body) return [];

  const lines = body.split(/\r?\n/);
  const stepsStart = lines.findIndex((line) => /^ {4}steps:\s*(?:#.*)?$/.test(line));
  if (stepsStart === -1) return [];

  const steps = [];
  let current = [];
  for (let index = stepsStart + 1; index < lines.length; index += 1) {
    const line = lines[index];
    const visible = stripWorkflowComment(line);
    if (visible.trim() && indentation(line) <= 4) break;
    if (/^ {6}-\s+/.test(visible)) {
      if (current.length > 0) steps.push(parseWorkflowStep(current));
      current = [line];
    } else if (current.length > 0) {
      current.push(line);
    }
  }
  if (current.length > 0) steps.push(parseWorkflowStep(current));
  return steps;
}

function workflowStepRun(step) {
  for (let index = 0; index < step.lines.length; index += 1) {
    const inline = /^ {6}-\s+run:\s*(.*)$/.exec(step.lines[index]);
    const nested = /^ {8}run:\s*(.*)$/.exec(step.lines[index]);
    const match = inline ?? nested;
    if (!match) continue;

    const value = stripWorkflowComment(match[1]).trim();
    if (!/^[>|]/.test(value)) return { style: "scalar", value };

    const keyIndent = inline ? 6 : 8;
    const block = [];
    let contentIndent;
    for (let child = index + 1; child < step.lines.length; child += 1) {
      const line = step.lines[child];
      if (!stripWorkflowComment(line).trim()) {
        block.push("");
        continue;
      }
      const childIndent = indentation(line);
      if (childIndent <= keyIndent) break;
      contentIndent ??= childIndent;
      block.push(stripWorkflowComment(line.slice(contentIndent)));
    }
    return { style: value[0], value: block.join("\n") };
  }
  return undefined;
}

function workflowJobScalar(workflow, jobName, property) {
  const body = workflowJobBody(workflow, jobName);
  if (!body) return "";
  const pattern = new RegExp(`^ {4}${property}:\\s*(.*)$`);
  for (const line of body.split(/\r?\n/)) {
    const match = pattern.exec(line);
    if (match) return stripWorkflowComment(match[1]).trim();
  }
  return "";
}

function workflowJobProperty(workflow, jobName, property) {
  const body = workflowJobBody(workflow, jobName);
  if (!body) return "";

  const lines = body.split(/\r?\n/);
  const propertyPattern = new RegExp(`^ {4}${property}:\\s*(.*)$`);
  for (let index = 0; index < lines.length; index += 1) {
    const match = propertyPattern.exec(lines[index]);
    if (!match) continue;

    const value = stripWorkflowComment(match[1]).trim();
    if (!/^[>|]/.test(value)) return value;

    const block = [];
    while (index + 1 < lines.length && /^ {6}\S/.test(lines[index + 1])) {
      block.push(stripWorkflowComment(lines[index + 1].slice(6)).trim());
      index += 1;
    }
    return block.join(" ");
  }
  return "";
}

function workflowJobList(workflow, jobName, property) {
  const body = workflowJobBody(workflow, jobName);
  if (!body) return [];
  const lines = body.split(/\r?\n/);
  const start = lines.findIndex((line) => new RegExp(`^ {4}${property}:\\s*$`).test(line));
  if (start === -1) return [];

  const values = [];
  for (let index = start + 1; index < lines.length; index += 1) {
    const match = /^ {6}-\s*(.+)$/.exec(lines[index]);
    if (match) {
      values.push(stripWorkflowComment(match[1]).trim());
      continue;
    }
    if (!lines[index].trim()) continue;
    break;
  }
  return values;
}

function workflowJobDirectKeys(workflow, jobName) {
  const keys = [];
  const malformed = [];
  for (const line of workflowJobBody(workflow, jobName).split(/\r?\n/)) {
    const visible = stripWorkflowComment(line);
    if (!visible.trim() || indentation(visible) !== 4) continue;
    const match = /^ {4}([A-Za-z0-9_-]+):(?:\s|$)/.exec(visible);
    if (!match) {
      malformed.push(visible.trim());
    } else {
      keys.push(match[1]);
    }
  }
  return { keys, malformed };
}

function workflowJobBlock(workflow, jobName, property) {
  const body = workflowJobBody(workflow, jobName);
  const lines = body.split(/\r?\n/);
  const start = lines.findIndex((line) =>
    new RegExp(`^ {4}${property}:\\s*(?:#.*)?$`).test(line)
  );
  if (start === -1) return "";
  const block = [stripWorkflowComment(lines[start].slice(4))];
  for (let index = start + 1; index < lines.length; index += 1) {
    const line = lines[index];
    if (!line.trim()) {
      block.push("");
      continue;
    }
    if (indentation(line) <= 4) break;
    block.push(stripWorkflowComment(line.slice(4)));
  }
  return block.join("\n").trimEnd();
}

function stepRunsUnconditionally(step) {
  return (
    step.malformed.length === 0 &&
    !step.properties.has("if") &&
    !step.properties.has("continue-on-error")
  );
}

function requireWorkflowCommands(workflow, label, jobName, commands, failures, expectedIf) {
  const body = workflowJobBody(workflow, jobName);
  if (!body) {
    failures.push(`${label} must define ${jobName}`);
    return;
  }
  const steps = workflowSteps(workflow, jobName);
  for (const command of commands) {
    if (
      !steps.some((step) => {
        const run = workflowStepRun(step);
        return (
          step.malformed.length === 0 &&
          !step.properties.has("continue-on-error") &&
          (expectedIf === undefined
            ? !step.properties.has("if")
            : step.properties.get("if") === expectedIf) &&
          run?.style === "scalar" &&
          run.value === command
        );
      })
    ) {
      failures.push(`${label} ${jobName} must include ${command}`);
    }
  }
}

function requireWorkflowRunScript(workflow, label, jobName, expected, failures) {
  const steps = workflowSteps(workflow, jobName);
  if (
    !steps.some((step) => {
      const run = workflowStepRun(step);
      return (
        stepRunsUnconditionally(step) &&
        run?.style === "|" &&
        run.value.trimEnd() === expected
      );
    })
  ) {
    failures.push(`${label} ${jobName} must retain the approved fail-closed script`);
  }
}

function requireStepMapping(step, property, expected, label, failures) {
  const mapping = step?.mappings.get(property) ?? new Map();
  if (
    mapping.size !== expected.size ||
    [...expected].some(([key, value]) => mapping.get(key) !== value)
  ) {
    failures.push(label);
  }
}

function workflowStepNamed(workflow, jobName, name) {
  return workflowSteps(workflow, jobName).filter((step) => step.properties.get("name") === name);
}

function requireCodeqlStep(workflow, name, action, expectedIf, expectedWith, failures) {
  const matches = workflowStepNamed(workflow, "analyze", name);
  const step = matches.length === 1 ? matches[0] : undefined;
  if (!step || step.malformed.length > 0 || step.properties.get("uses") !== action) {
    failures.push(`codeql.yml must retain the ${name} action step`);
    return;
  }
  if (expectedIf === undefined) {
    if (step.properties.has("if")) {
      failures.push(`codeql.yml ${name} must not be conditionally skipped`);
    }
  } else if (step.properties.get("if") !== expectedIf) {
    failures.push(`codeql.yml ${name} must use its approved condition`);
  }
  if (step.properties.has("continue-on-error")) {
    failures.push(`codeql.yml ${name} must not ignore failures`);
  }
  requireStepMapping(
    step,
    "with",
    new Map(expectedWith),
    `codeql.yml ${name} must retain its approved inputs`,
    failures
  );
}

function assertCodeqlContract(workflow, failures) {
  const root = topLevelKeys(workflow);
  if (
    root.malformed.length > 0 ||
    root.keys.join("\n") !== ["name", "on", "permissions", "concurrency", "jobs"].join("\n")
  ) {
    failures.push("codeql.yml must use only the approved canonical top-level keys");
  }
  const triggers = topLevelTriggers(workflow).sort();
  const expectedTriggers = ["pull_request", "push", "schedule", "workflow_dispatch"].sort();
  if (triggers.join("\n") !== expectedTriggers.join("\n")) {
    failures.push("codeql.yml must declare only push, pull_request, schedule, and workflow_dispatch");
  }
  if ((workflow.match(/branches:\s*\[dev, main\]/g) ?? []).length !== 2) {
    failures.push("codeql.yml push and pull_request must target dev and main");
  }
  if (!workflow.includes('- cron: "17 3 * * 1"')) {
    failures.push("codeql.yml must retain the weekly schedule");
  }

  const permissionEntries = topLevelBlock(workflow, "permissions")
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line && line !== "permissions:")
    .sort();
  if (
    permissionEntries.join("\n") !== ["contents: read", "security-events: write"].sort().join("\n")
  ) {
    failures.push("codeql.yml must grant only contents: read and security-events: write");
  }

  if (workflowJobScalar(workflow, "analyze", "timeout-minutes") !== "45") {
    failures.push("codeql.yml analyze must retain a 45-minute timeout");
  }
  const analyzeKeys = workflowJobDirectKeys(workflow, "analyze");
  if (
    analyzeKeys.malformed.length > 0 ||
    analyzeKeys.keys.join("\n") !==
      ["name", "runs-on", "timeout-minutes", "strategy", "steps"].join("\n")
  ) {
    failures.push("codeql.yml analyze must use only the approved canonical job properties");
  }
  if (workflowJobBlock(workflow, "analyze", "strategy") !== CODEQL_STRATEGY_BLOCK) {
    failures.push("codeql.yml must retain the approved language/build-mode pairs");
  }
  const codeqlSteps = workflowSteps(workflow, "analyze");
  const expectedActions = [
    "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1",
    "github/codeql-action/init@5595ccaf912efad79be6eef63a5619ff05969be3",
    "github/codeql-action/analyze@5595ccaf912efad79be6eef63a5619ff05969be3",
  ];
  if (
    codeqlSteps.some((step) => step.malformed.length > 0) ||
    codeqlSteps.map((step) => step.properties.get("uses") ?? "").join("\n") !==
      expectedActions.join("\n")
  ) {
    failures.push(
      "codeql.yml analyze must contain only checkout, Initialize CodeQL, and Analyze action steps"
    );
  }
  requireCodeqlStep(
    workflow,
    "Initialize CodeQL",
    "github/codeql-action/init@5595ccaf912efad79be6eef63a5619ff05969be3",
    undefined,
    [
      ["languages", "${{ matrix.language }}"],
      ["build-mode", "${{ matrix.build-mode }}"],
    ],
    failures
  );
  requireCodeqlStep(
    workflow,
    "Analyze",
    "github/codeql-action/analyze@5595ccaf912efad79be6eef63a5619ff05969be3",
    undefined,
    [["category", "/language:${{ matrix.language }}"]],
    failures
  );
  if (/pull_request_target|release-signing|TAURI_SIGNING_PRIVATE_KEY/.test(workflow)) {
    failures.push("codeql.yml must not use privileged PR triggers or release signing access");
  }
}

function assertCiDependencyConditions(workflow, failures) {
  for (const [job, expected] of CI_JOB_CONDITIONS) {
    if (workflowJobProperty(workflow, job, "if") !== expected) {
      failures.push(`ci.yml ${job} if must equal ${expected}`);
    }
  }
}

function assertDependabotContract(source, failures) {
  if (!/^version:\s*2\s*$/m.test(source)) failures.push("dependabot.yml must use version 2");
  const updates = dependabotUpdates(source);
  const expected = new Map([
    ["npm", "/"],
    ["cargo", "/src-tauri"],
    ["github-actions", "/"],
  ]);
  for (const [ecosystem, directory] of expected) {
    const matches = updates.filter((update) => update.ecosystem === ecosystem);
    if (matches.length !== 1) {
      failures.push(`dependabot.yml must define exactly one ${ecosystem} update entry`);
      continue;
    }
    if (!new RegExp(`^    directory:\\s*${directory.replace("/", "\\/")}\\s*$`, "m").test(matches[0].body)) {
      failures.push(`dependabot.yml ${ecosystem} directory must be ${directory}`);
    }
    if (!/^      interval:\s*weekly\s*$/m.test(matches[0].body)) {
      failures.push(`dependabot.yml ${ecosystem} updates must run weekly`);
    }
  }
}

function assertCiGateClosure(workflow, failures) {
  if (workflowJobScalar(workflow, "ci-gate", "if") !== "always()") {
    failures.push("ci.yml ci-gate must use if: always()");
  }
  const matches = workflowStepNamed(workflow, "ci-gate", "Require expected jobs");
  const step = matches.length === 1 ? matches[0] : undefined;
  const run = step ? workflowStepRun(step) : undefined;
  if (
    !step ||
    step.malformed.length > 0 ||
    step.properties.get("shell") !== "bash" ||
    !stepRunsUnconditionally(step) ||
    run?.style !== "|"
  ) {
    failures.push("ci.yml ci-gate must retain an unconditional Bash aggregation step");
    return;
  }
  requireStepMapping(
    step,
    "env",
    CI_GATE_RESULT_ENV,
    "ci.yml ci-gate must bind aggregation results directly from needs.*",
    failures
  );
  const digest = createHash("sha256").update(run.value.trimEnd()).digest("hex");
  if (digest !== CI_GATE_RUN_SHA256) {
    failures.push("ci.yml ci-gate must retain the approved fail-closed aggregation script");
  }
}

export function assertCiQualityGates({
  codeqlWorkflow,
  dependabotConfig,
  packageJson,
  vitestConfig,
  ciWorkflow,
  performanceWorkflow,
  prTitleWorkflow,
}) {
  const failures = [];
  const scripts = packageJson?.scripts ?? {};

  if (
    scripts["check:no-instant-now-sub"] !==
    `${ACTIONS_GUARD}node scripts/check-no-instant-now-sub.selftest.mjs && node scripts/check-no-instant-now-sub.mjs`
  ) {
    failures.push("check:no-instant-now-sub must be Actions-only and run its self-test first");
  }
  if (
    scripts["create-aio-plugin:typecheck"] !==
    `${ACTIONS_GUARD}node scripts/check-create-aio-plugin-typecheck.selftest.mjs && pnpm --filter create-aio-plugin typecheck`
  ) {
    failures.push("create-aio-plugin:typecheck must be Actions-only and run its negative fixture first");
  }
  if (
    scripts["check:ci-quality-gates"] !==
    `${ACTIONS_GUARD}node scripts/check-ci-quality-gates.selftest.mjs && node scripts/check-ci-quality-gates.mjs`
  ) {
    failures.push("check:ci-quality-gates must be Actions-only and run its self-test first");
  }
  if (scripts["test:e2e"] !== undefined) {
    failures.push("test:e2e must stay absent so root coverage is the only E2E entry point");
  }
  if (
    scripts["test:unit:coverage"] !== `${ACTIONS_GUARD}vitest run --coverage`
  ) {
    failures.push("test:unit:coverage must remain the Actions-only root coverage entry point");
  }
  if (
    !vitestConfig.includes(ROOT_TEST_INCLUDE) ||
    /exclude:\s*\[[^\]]*src\/e2e/s.test(vitestConfig)
  ) {
    failures.push("vitest.config.ts must discover src/e2e through the root test include");
  }

  requireWorkflowCommands(
    ciWorkflow,
    "ci.yml",
    "contracts",
    [
      "node scripts/check-cloud-only-verification.mjs",
      "node scripts/check-tui-release-contract.mjs",
    ],
    failures
  );
  requireWorkflowCommands(
    ciWorkflow,
    "ci.yml",
    "contracts",
    [
      "node scripts/check-cloud-only-verification.selftest.mjs",
      "node scripts/ci-change-scope.selftest.mjs",
      "node scripts/check-ci-quality-gates.selftest.mjs && node scripts/check-ci-quality-gates.mjs",
      "node scripts/check-github-actions-pin-policy.selftest.mjs && node scripts/check-github-actions-pin-policy.mjs",
      "node scripts/check-no-instant-now-sub.selftest.mjs && node scripts/check-no-instant-now-sub.mjs",
      "node scripts/check-dev-build-artifacts.selftest.mjs && node scripts/check-dev-build-artifacts.mjs",
      "node scripts/release-source.selftest.mjs",
      "node scripts/check-sync-upstream-policy.selftest.mjs",
      "node scripts/check-sync-upstream-policy.mjs",
      "node scripts/release-promotion.selftest.mjs",
      "node scripts/support-matrix.homebrew-cask.selftest.mjs",
      "node scripts/check-release-signing-secret-scope.selftest.mjs && node scripts/check-release-signing-secret-scope.mjs",
    ],
    failures,
    SOURCE_CONTRACT_STEP_IF
  );
  requireWorkflowCommands(
    ciWorkflow,
    "ci.yml",
    "contracts",
    ["node scripts/check-plugin-system-docs.mjs", "node scripts/check-plugin-api-contract.mjs"],
    failures,
    PLUGIN_CONTRACT_STEP_IF
  );
  requireWorkflowCommands(
    ciWorkflow,
    "ci.yml",
    "contracts",
    ["node scripts/check-spec-links.mjs"],
    failures,
    DOCS_CONTRACT_STEP_IF
  );
  assertCodeqlContract(codeqlWorkflow, failures);
  assertDependabotContract(dependabotConfig, failures);
  if (workflowJobBody(ciWorkflow, "ci-gate").includes("codeql")) {
    failures.push("ci.yml must keep initial CodeQL analysis outside the required ci-gate");
  }
  requireWorkflowCommands(
    ciWorkflow,
    "ci.yml",
    "frontend",
    [
      "pnpm install --frozen-lockfile",
      "pnpm audit:deps",
      "pnpm lint",
      "pnpm plugin-sdk:typecheck",
      "pnpm create-aio-plugin:typecheck",
      "pnpm plugin-sdk:test",
      "pnpm create-aio-plugin:test",
      "pnpm test:unit:coverage",
      "pnpm build",
    ],
    failures
  );
  if (
    workflowSteps(ciWorkflow, "frontend").some(
      (step) => workflowStepRun(step)?.value === "pnpm test:e2e"
    )
  ) {
    failures.push("ci.yml frontend must not run a dedicated pnpm test:e2e step");
  }
  requireWorkflowCommands(
    ciWorkflow,
    "ci.yml",
    "rust",
    [
      "cargo clippy --workspace --all-targets --locked -- -D warnings",
      "cargo test --workspace --locked -- --test-threads=1",
      "cargo audit",
    ],
    failures
  );
  requireWorkflowRunScript(ciWorkflow, "ci.yml", "rust", RUST_CANONICALIZE_RUN, failures);
  const ciGateName = "${{ github.event_name == 'workflow_dispatch' && 'manual-ci-gate' || 'ci-gate' }}";
  if (workflowJobScalar(ciWorkflow, "ci-gate", "name") !== ciGateName) {
    failures.push(`ci.yml ci-gate must include name: ${ciGateName}`);
  }
  const ciGateNeeds = workflowJobList(ciWorkflow, "ci-gate", "needs");
  for (const job of ["manual-dispatch-guard", "contracts", "frontend", "rust"]) {
    if (!ciGateNeeds.includes(job)) failures.push(`ci.yml ci-gate must include - ${job}`);
  }
  if (workflowJobBody(ciWorkflow, "pr-title") || ciGateNeeds.includes("pr-title")) {
    failures.push("ci.yml must keep pr-title independent from the aggregate quality gate");
  }
  assertCiDependencyConditions(ciWorkflow, failures);
  assertCiGateClosure(ciWorkflow, failures);
  requireWorkflowRunScript(prTitleWorkflow, "pr-title.yml", "pr-title", PR_TITLE_RUN, failures);
  requireWorkflowRunScript(
    performanceWorkflow,
    "performance.yml",
    "provider-trend-benchmark",
    PERFORMANCE_BENCHMARK_RUN,
    failures
  );

  if (failures.length > 0) {
    throw new Error(`CI quality gate contract failed:\n- ${failures.join("\n- ")}`);
  }
}

const modulePath = fileURLToPath(import.meta.url);
if (process.argv[1] && resolve(process.argv[1]) === modulePath) {
  const repoRoot = dirname(dirname(modulePath));
  try {
    assertCiQualityGates({
      codeqlWorkflow: readFileSync(join(repoRoot, ".github", "workflows", "codeql.yml"), "utf8"),
      dependabotConfig: readFileSync(join(repoRoot, ".github", "dependabot.yml"), "utf8"),
      packageJson: JSON.parse(readFileSync(join(repoRoot, "package.json"), "utf8")),
      vitestConfig: readFileSync(join(repoRoot, "vitest.config.ts"), "utf8"),
      ciWorkflow: readFileSync(join(repoRoot, ".github", "workflows", "ci.yml"), "utf8"),
      performanceWorkflow: readFileSync(
        join(repoRoot, ".github", "workflows", "performance.yml"),
        "utf8"
      ),
      prTitleWorkflow: readFileSync(join(repoRoot, ".github", "workflows", "pr-title.yml"), "utf8"),
    });
    console.error("[ci-quality-gates] repository contract passed");
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}
