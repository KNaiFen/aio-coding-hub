import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { CHECKS, STAGES } from "./run-checks.mjs";

const ACTIONS_GUARD = "node scripts/require-github-actions.mjs && ";
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
  ["DOCS_CHECKS", "${{ needs.change-scope.outputs.docs_checks }}"],
  ["DOCS_RESULT", "${{ needs.docs-contract.result }}"],
  ["SUPPORT_RESULT", "${{ needs.support-contract.result }}"],
  ["FRONTEND_RESULT", "${{ needs.frontend.result }}"],
  ["RUST_RESULT", "${{ needs.rust.result }}"],
  ["PLAN_RESULT", "${{ needs.candidate-plan.result }}"],
  ["SHOULD_BUILD", "${{ needs.candidate-plan.outputs.should_build }}"],
  ["BUILD_RESULT", "${{ needs.build-release-candidate.result }}"],
  ["TUI_BUILD_RESULT", "${{ needs.build-tui-release-candidate.result }}"],
  ["ASSEMBLE_RESULT", "${{ needs.assemble-release-candidate.result }}"],
]);
const CODEQL_STRATEGY_BLOCK = `strategy:
  fail-fast: false
  matrix:
    include:
      - language: javascript-typescript
        build-mode: none
      - language: rust
        build-mode: autobuild`;

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

function requireWorkflowCommands(workflow, label, jobName, commands, failures) {
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
          stepRunsUnconditionally(step) &&
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

function requireStage(stages, stage, check, failures) {
  if (!stages[stage]?.includes(check)) {
    failures.push(`STAGES.${stage} must include ${check}`);
  }
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
    "Autobuild compiled language",
    "github/codeql-action/autobuild@5595ccaf912efad79be6eef63a5619ff05969be3",
    "matrix.build-mode == 'autobuild'",
    [],
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
  if (
    !step ||
    step.malformed.length > 0 ||
    step.properties.get("shell") !== "bash" ||
    !stepRunsUnconditionally(step)
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
}

export function assertCiQualityGates({
  codeqlWorkflow,
  dependabotConfig,
  packageJson,
  checks,
  stages,
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

  if (checks["cloud-only-verification"] !== "node scripts/check-cloud-only-verification.mjs") {
    failures.push("aggregate checks must define cloud-only-verification");
  }
  if (checks["ci-quality-gates"] !== "pnpm check:ci-quality-gates") {
    failures.push("aggregate checks must define ci-quality-gates");
  }
  if (checks["create-aio-plugin-typecheck"] !== "pnpm create-aio-plugin:typecheck") {
    failures.push("aggregate checks must define create-aio-plugin-typecheck");
  }

  requireStage(stages, "full-ci", "cloud-only-verification", failures);
  requireStage(stages, "full-ci", "no-instant-now-sub", failures);
  requireStage(stages, "full-ci", "ci-quality-gates", failures);
  requireStage(stages, "full-ci", "create-aio-plugin-typecheck", failures);
  requireStage(stages, "plugin-hardening", "cloud-only-verification", failures);
  requireStage(stages, "plugin-hardening", "create-aio-plugin-typecheck", failures);

  requireWorkflowCommands(
    ciWorkflow,
    "ci.yml",
    "support-contract",
    [
      "node scripts/check-cloud-only-verification.selftest.mjs && node scripts/check-cloud-only-verification.mjs",
      "node scripts/check-ci-quality-gates.selftest.mjs && node scripts/check-ci-quality-gates.mjs",
      "node scripts/check-github-actions-pin-policy.selftest.mjs && node scripts/check-github-actions-pin-policy.mjs",
      "node scripts/check-no-instant-now-sub.selftest.mjs && node scripts/check-no-instant-now-sub.mjs",
    ],
    failures
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
      "pnpm --filter create-aio-plugin test",
      "pnpm test:e2e",
      "pnpm test:unit:coverage",
      "pnpm build",
    ],
    failures
  );
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
  for (const job of ["manual-dispatch-guard", "support-contract", "frontend", "rust"]) {
    if (!ciGateNeeds.includes(job)) failures.push(`ci.yml ci-gate must include - ${job}`);
  }
  if (workflowJobBody(ciWorkflow, "pr-title") || ciGateNeeds.includes("pr-title")) {
    failures.push("ci.yml must keep pr-title independent from the aggregate quality gate");
  }
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
      checks: CHECKS,
      stages: STAGES,
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
