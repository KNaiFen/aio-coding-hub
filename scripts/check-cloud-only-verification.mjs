import { createHash } from "node:crypto";
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const modulePath = fileURLToPath(import.meta.url);
const repoRoot = resolve(dirname(modulePath), "..");

const ROOT_GUARD = "node scripts/require-github-actions.mjs";
const WORKSPACE_GUARD = "node ../../scripts/require-github-actions.mjs";
const LOCAL_COMMAND_LINE =
  /^\s*(?:(?:pnpm|npm|yarn)\s+|cargo\s+(?:audit|build|check|clippy|fmt|run|test)\b|(?:rustfmt|clippy)\s+(?:--|\w)|tauri\s+(?:build|dev|icon|info|signer)\b)/i;
const LOCAL_QUALITY_INSTRUCTION =
  /^\s*(?:-\s*)?(?:run|regenerate|execute)\b.*\b(?:rust|frontend|bindings?|type-?check|lint|clippy|format(?:ting)?|test(?:s| suite)?|cargo|pnpm)\b/i;
const README_FENCE = /```(?:bash|sh|shell)?\s*\n([\s\S]*?)```/gi;
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
const MANUAL_CI_GUARD_RUN = `set -euo pipefail
[[ "$EVENT_REF" == "refs/heads/main" ]] || {
  echo "::error::Manual ci runs are restricted to the main branch. PR validation is automatic."
  exit 1
}`;
const PERFORMANCE_GUARD_RUN = `set -euo pipefail
[[ "$EVENT_REF" == "refs/heads/main" ]] || {
  echo "::error::Performance runs are restricted to the main branch."
  exit 1
}`;
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
// The aggregation script's complete shell control flow is part of the required gate.
const CI_GATE_RUN_SHA256 = "e05c0c0da1c482e4e96e5d4a5abcd487ad7493b5ff8b8cf32ffd23d59fac2eb6";

function readText(root, relativePath) {
  return readFileSync(join(root, relativePath), "utf8");
}

function readJson(root, relativePath) {
  return JSON.parse(readText(root, relativePath));
}

function readMarkdownTree(root, relativeDir) {
  const files = new Map();
  const visit = (current, relative) => {
    for (const entry of readdirSync(current, { withFileTypes: true })) {
      const nextRelative = join(relative, entry.name);
      const nextPath = join(current, entry.name);
      if (entry.isDirectory()) {
        visit(nextPath, nextRelative);
      } else if (entry.isFile() && entry.name.endsWith(".md")) {
        files.set(nextRelative, readFileSync(nextPath, "utf8"));
      }
    }
  };

  visit(join(root, relativeDir), "");
  return files;
}

export function loadCloudOnlyVerificationFixture(root = repoRoot) {
  return {
    rootPackage: readJson(root, "package.json"),
    pluginSdkPackage: readJson(root, "packages/plugin-sdk/package.json"),
    scaffolderPackage: readJson(root, "packages/create-aio-plugin/package.json"),
    tauriConfig: readJson(root, "src-tauri/tauri.conf.json"),
    agents: readText(root, "AGENTS.md"),
    readme: readText(root, "README.md"),
    readmeEn: readText(root, "README_EN.md"),
    trellisWorkflow: readText(root, ".trellis/workflow.md"),
    implementAgent: readText(root, ".trellis/agents/implement.md"),
    checkAgent: readText(root, ".trellis/agents/check.md"),
    activeSpecs: readMarkdownTree(root, ".trellis/spec/aio-coding-hub"),
    ciWorkflow: readText(root, ".github/workflows/ci.yml"),
    devBuildWorkflow: readText(root, ".github/workflows/dev-build.yml"),
    performanceWorkflow: readText(root, ".github/workflows/performance.yml"),
    prTitleWorkflow: readText(root, ".github/workflows/pr-title.yml"),
  };
}

function requireText(value, expected, label, failures) {
  if (!value.includes(expected)) failures.push(`${label} must include ${JSON.stringify(expected)}`);
}

function requireAbsent(value, pattern, label, failures) {
  if (pattern.test(value)) failures.push(`${label} contains a prohibited local instruction`);
}

function assertActionsOnlyScripts(pkg, label, guard, failures) {
  const scripts = pkg?.scripts;
  if (!scripts || typeof scripts !== "object") {
    failures.push(`${label} must define scripts`);
    return;
  }

  if (scripts.preinstall !== guard) {
    failures.push(`${label} preinstall must be exactly ${JSON.stringify(guard)}`);
  }

  for (const [name, command] of Object.entries(scripts)) {
    if (typeof command !== "string" || !command.startsWith(guard)) {
      failures.push(`${label} script ${name} must start with the GitHub Actions guard`);
    }
  }
}

function assertNoForbiddenReadmeCommand(text, label, failures) {
  const blocks = text.matchAll(README_FENCE);
  for (const block of blocks) {
    if (block[1].split(/\r?\n/).some((line) => LOCAL_COMMAND_LINE.test(line))) {
      failures.push(`${label} must not present a package/native command as a local code example`);
      return;
    }
  }
}

function assertActiveSpecs(specs, failures) {
  if (!(specs instanceof Map) || specs.size === 0) {
    failures.push("active AIO Trellis specs must be readable");
    return;
  }

  for (const [path, text] of specs) {
    const lines = text.split(/\r?\n/);
    if (lines.some((line) => LOCAL_COMMAND_LINE.test(line))) {
      failures.push(`active spec ${path} contains a bare local package/native command`);
    }
    if (
      lines.some(
        (line) =>
          LOCAL_QUALITY_INSTRUCTION.test(line) && !/GitHub Actions|cloud-owned|cloud-only/i.test(line)
      )
    ) {
      failures.push(`active spec ${path} contains a local quality-gate instruction`);
    }
  }

  const index = specs.get("cross-layer/index.md") ?? "";
  requireText(index, "cloud-only-verification-contract.md", "cross-layer index", failures);
  const contract = specs.get("cross-layer/cloud-only-verification-contract.md") ?? "";
  requireText(contract, "GitHub-Actions-only", "cloud-only verification contract", failures);
  requireText(contract, "ci-gate", "cloud-only verification contract", failures);
}

function workflowJobBody(workflow, job) {
  const jobStart = new RegExp(`^  ${job}:\\s*$`, "m").exec(workflow);
  if (!jobStart) return "";

  const afterJob = workflow.slice(jobStart.index + jobStart[0].length);
  const nextJob = /^  [A-Za-z0-9_-]+:\s*$/m.exec(afterJob);
  return nextJob ? afterJob.slice(0, nextJob.index) : afterJob;
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

function workflowSteps(workflow, job) {
  const body = workflowJobBody(workflow, job);
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

function workflowRunBodies(workflow, job) {
  return workflowSteps(workflow, job)
    .map((step) => workflowStepRun(step)?.value)
    .filter((run) => run !== undefined);
}

function runExecutesCommand(run, command) {
  return run.split(/\r?\n/).some((line) => {
    const normalized = line.trim().replace(/\\\s*$/, "").trim();
    return normalized === command;
  });
}

function stepRunsUnconditionally(step) {
  return (
    step.malformed.length === 0 &&
    !step.properties.has("if") &&
    !step.properties.has("continue-on-error")
  );
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

function workflowJobProperty(workflow, job, property) {
  const body = workflowJobBody(workflow, job);
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

function assertWorkflowRunCommands(workflow, job, commands, failures, label = "ci.yml") {
  const steps = workflowSteps(workflow, job);
  if (steps.length === 0) {
    failures.push(`${label} must define ${job} with a run step`);
    return;
  }

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
      failures.push(`${label} ${job} must include ${command}`);
    }
  }
}

function assertWorkflowRunScript(workflow, job, expected, failures, label = "ci.yml") {
  const steps = workflowSteps(workflow, job);
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
    failures.push(`${label} ${job} must retain the approved fail-closed script`);
  }
}

function assertWorkflowJobPropertyEquals(
  workflow,
  job,
  property,
  expected,
  failures,
  label = "ci.yml"
) {
  if (!workflowJobBody(workflow, job)) {
    failures.push(`${label} must define ${job}`);
    return;
  }
  if (workflowJobProperty(workflow, job, property) !== expected) {
    failures.push(`${label} ${job} ${property} must equal ${expected}`);
  }
}

function assertCiGateClosure(workflow, failures) {
  assertWorkflowJobPropertyEquals(workflow, "ci-gate", "if", "always()", failures);
  const matches = workflowSteps(workflow, "ci-gate").filter(
    (step) => step.properties.get("name") === "Require expected jobs"
  );
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

function assertCandidatePrBoundary(workflow, failures) {
  const candidatePlanCondition = [
    "needs.change-scope.outputs.full_ci == 'true' &&",
    "github.repository == 'KNaiFen/aio-coding-hub' &&",
    "github.ref == 'refs/heads/main' &&",
    "(github.event_name == 'push' || github.event_name == 'workflow_dispatch')",
  ].join(" ");
  assertWorkflowJobPropertyEquals(
    workflow,
    "candidate-plan",
    "if",
    candidatePlanCondition,
    failures
  );
  for (const job of ["build-release-candidate", "build-tui-release-candidate"]) {
    assertWorkflowJobPropertyEquals(
      workflow,
      job,
      "if",
      "needs.candidate-plan.outputs.should_build == 'true'",
      failures
    );
  }

  const ciGateRuns = workflowRunBodies(workflow, "ci-gate").join("\n");
  const mainCandidateCondition =
    'if [[ "$EVENT_REF" == "refs/heads/main" && ( "$EVENT_NAME" == "push" || "$EVENT_NAME" == "workflow_dispatch" ) ]]; then';
  if (!ciGateRuns.includes(mainCandidateCondition)) {
    failures.push("ci.yml ci-gate must only require candidate jobs for eligible main runs");
  }
  const normalizedCiGate = ciGateRuns
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .join("\n");
  const candidateBoundary = [
    'if [[ "$FULL_CI" == "true" ]]; then',
    '[[ "$SCOPE" == "full" ]]',
    '[[ "$SUPPORT_RESULT" == "success" ]]',
    '[[ "$FRONTEND_RESULT" == "success" ]]',
    '[[ "$RUST_RESULT" == "success" ]]',
    mainCandidateCondition,
    '[[ "$PLAN_RESULT" == "success" ]]',
    'if [[ "$SHOULD_BUILD" == "true" ]]; then',
    '[[ "$BUILD_RESULT" == "success" ]]',
    '[[ "$TUI_BUILD_RESULT" == "success" ]]',
    '[[ "$ASSEMBLE_RESULT" == "success" ]]',
    "else",
    '[[ "$BUILD_RESULT" == "skipped" ]]',
    '[[ "$TUI_BUILD_RESULT" == "skipped" ]]',
    '[[ "$ASSEMBLE_RESULT" == "skipped" ]]',
    "fi",
    "else",
    '[[ "$PLAN_RESULT" == "skipped" ]]',
    '[[ "$BUILD_RESULT" == "skipped" ]]',
    '[[ "$TUI_BUILD_RESULT" == "skipped" ]]',
    '[[ "$ASSEMBLE_RESULT" == "skipped" ]]',
    "fi",
  ].join("\n");
  if (!normalizedCiGate.includes(candidateBoundary)) {
    failures.push("ci.yml ci-gate must require candidate desktop/TUI jobs to be skipped outside eligible main runs");
  }
}

function assertManualDevBuildWorkflow(workflow, failures) {
  assertOnlyWorkflowDispatch(workflow, "dev-build.yml", failures);
}

function workflowTriggers(workflow, label, failures) {
  const onBlock = /^on:\s*\n([\s\S]*?)(?=^(?:permissions|concurrency|env|defaults|jobs):)/m.exec(workflow)?.[1];
  if (!onBlock) {
    failures.push(`${label} must define an on block`);
    return { onBlock: "", triggers: [] };
  }

  const triggers = [...onBlock.matchAll(/^\s{2}([A-Za-z][A-Za-z0-9_-]*):/gm)].map(
    (match) => match[1]
  );
  return { onBlock, triggers };
}

function assertOnlyWorkflowDispatch(workflow, label, failures) {
  const { triggers } = workflowTriggers(workflow, label, failures);
  if (triggers.length !== 1 || triggers[0] !== "workflow_dispatch") {
    failures.push(`${label} must declare only the workflow_dispatch trigger`);
  }
}

function assertManualCiBoundary(workflow, failures) {
  assertWorkflowJobPropertyEquals(
    workflow,
    "manual-dispatch-guard",
    "if",
    "github.event_name == 'workflow_dispatch'",
    failures
  );
  assertWorkflowJobPropertyEquals(
    workflow,
    "change-scope",
    "needs",
    "manual-dispatch-guard",
    failures
  );
  assertWorkflowJobPropertyEquals(
    workflow,
    "change-scope",
    "if",
    "always() && (github.event_name != 'workflow_dispatch' || needs.manual-dispatch-guard.result == 'success')",
    failures
  );
  assertWorkflowJobPropertyEquals(
    workflow,
    "ci-gate",
    "name",
    "${{ github.event_name == 'workflow_dispatch' && 'manual-ci-gate' || 'ci-gate' }}",
    failures
  );

  assertWorkflowRunScript(
    workflow,
    "manual-dispatch-guard",
    MANUAL_CI_GUARD_RUN,
    failures
  );

  const gateBody = workflowJobBody(workflow, "ci-gate");
  const gateRuns = workflowRunBodies(workflow, "ci-gate");
  if (!gateBody.includes("- manual-dispatch-guard")) {
    failures.push("ci.yml ci-gate must depend on manual-dispatch-guard");
  }
  if (
    !gateRuns.some((run) => runExecutesCommand(run, '[[ "$MANUAL_GUARD_RESULT" == "success" ]]')) ||
    !gateRuns.some((run) => runExecutesCommand(run, '[[ "$MANUAL_GUARD_RESULT" == "skipped" ]]'))
  ) {
    failures.push("ci.yml ci-gate must validate the manual guard result for every event type");
  }
  if (workflowJobBody(workflow, "pr-title") || gateBody.includes("pr-title")) {
    failures.push("ci.yml must not inline or depend on the independent pr-title check");
  }
}

function assertPrTitleWorkflow(workflow, failures) {
  const { onBlock, triggers } = workflowTriggers(workflow, "pr-title.yml", failures);
  if (triggers.length !== 1 || triggers[0] !== "pull_request") {
    failures.push("pr-title.yml must declare only the pull_request trigger");
  }
  for (const type of ["edited", "opened", "reopened", "synchronize"]) {
    if (!onBlock.includes(type)) failures.push(`pr-title.yml must trigger for pull_request ${type}`);
  }
  if (/uses:\s+actions\/checkout@/.test(workflow)) {
    failures.push("pr-title.yml must not checkout pull request code");
  }
  assertWorkflowJobPropertyEquals(
    workflow,
    "pr-title",
    "name",
    "pr-title",
    failures,
    "pr-title.yml"
  );
  assertWorkflowRunScript(workflow, "pr-title", PR_TITLE_RUN, failures, "pr-title.yml");
}

function assertPerformanceWorkflow(workflow, failures) {
  assertOnlyWorkflowDispatch(workflow, "performance.yml", failures);
  assertWorkflowRunScript(
    workflow,
    "main-guard",
    PERFORMANCE_GUARD_RUN,
    failures,
    "performance.yml"
  );
  assertWorkflowJobPropertyEquals(
    workflow,
    "provider-trend-benchmark",
    "needs",
    "main-guard",
    failures,
    "performance.yml"
  );
  assertWorkflowRunScript(
    workflow,
    "provider-trend-benchmark",
    PERFORMANCE_BENCHMARK_RUN,
    failures,
    "performance.yml"
  );
  if (/release-signing|TAURI_SIGNING_PRIVATE_KEY/.test(workflow)) {
    failures.push("performance.yml must not receive release signing access");
  }
}

export function assertCloudOnlyVerificationContract(fixture) {
  const failures = [];
  const {
    rootPackage,
    pluginSdkPackage,
    scaffolderPackage,
    tauriConfig,
    agents,
    readme,
    readmeEn,
    trellisWorkflow,
    implementAgent,
    checkAgent,
    activeSpecs,
    ciWorkflow,
    devBuildWorkflow,
    performanceWorkflow,
    prTitleWorkflow,
  } = fixture;

  assertActionsOnlyScripts(rootPackage, "root package.json", ROOT_GUARD, failures);
  assertActionsOnlyScripts(pluginSdkPackage, "plugin SDK package.json", WORKSPACE_GUARD, failures);
  assertActionsOnlyScripts(scaffolderPackage, "plugin scaffolder package.json", WORKSPACE_GUARD, failures);

  for (const name of ["dev", "preview", "check:precommit", "check:precommit:full", "check:prepush"]) {
    if (rootPackage?.scripts?.[name] !== undefined) {
      failures.push(`root package.json must not expose local ${name} entry point`);
    }
  }

  if (tauriConfig?.build?.beforeDevCommand !== undefined) {
    failures.push("src-tauri/tauri.conf.json must not define beforeDevCommand");
  }
  if (tauriConfig?.build?.beforeBuildCommand !== "pnpm build") {
    failures.push("src-tauri/tauri.conf.json must retain the cloud frontend build hook");
  }

  requireText(agents, "Keep the local checkout zero-artifact.", "AGENTS.md", failures);
  requireText(agents, "check-cloud-only-verification.mjs", "AGENTS.md", failures);
  requireAbsent(agents, /Use `pnpm dev`/i, "AGENTS.md", failures);
  for (const [label, text] of [
    ["README.md", readme],
    ["README_EN.md", readmeEn],
  ]) {
    requireText(text, "check-cloud-only-verification.mjs", label, failures);
    requireText(text, "workflow_dispatch", label, failures);
    assertNoForbiddenReadmeCommand(text, label, failures);
  }

  requireText(trellisWorkflow, "repository-authorized verification", ".trellis/workflow.md", failures);
  requireAbsent(trellisWorkflow, /run project lint and type-check|ensure lint and type-check pass|lint \/ type-check \/ tests/i, ".trellis/workflow.md", failures);
  for (const [label, text] of [
    [".trellis/agents/implement.md", implementAgent],
    [".trellis/agents/check.md", checkAgent],
  ]) {
    requireText(text, "repository-authorized", label, failures);
    requireAbsent(text, /Run the project'?s lint and typecheck|Run lint and typecheck/i, label, failures);
  }
  assertActiveSpecs(activeSpecs, failures);

  if (!/^\s*workflow_dispatch:\s*$/m.test(ciWorkflow)) {
    failures.push("ci.yml must retain workflow_dispatch");
  }
  requireText(agents, "Do not start an additional manual `ci` run for routine PR validation.", "AGENTS.md", failures);
  requireText(readme, "不要为常规验证额外手动运行 `ci`", "README.md", failures);
  requireText(readmeEn, "Do not start an additional manual `ci` run for routine validation.", "README_EN.md", failures);
  assertManualCiBoundary(ciWorkflow, failures);
  assertPrTitleWorkflow(prTitleWorkflow, failures);
  assertPerformanceWorkflow(performanceWorkflow, failures);
  assertWorkflowRunCommands(
    ciWorkflow,
    "docs-contract",
    ["node scripts/check-cloud-only-verification.mjs"],
    failures
  );
  assertWorkflowRunCommands(
    ciWorkflow,
    "support-contract",
    [
      "node scripts/check-cloud-only-verification.selftest.mjs && node scripts/check-cloud-only-verification.mjs",
      "node scripts/check-ci-quality-gates.selftest.mjs && node scripts/check-ci-quality-gates.mjs",
    ],
    failures
  );
  assertWorkflowRunCommands(
    ciWorkflow,
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
  assertWorkflowRunCommands(
    ciWorkflow,
    "rust",
    [
      "cargo clippy --workspace --all-targets --locked -- -D warnings",
      "cargo test --workspace --locked -- --test-threads=1",
      "cargo audit",
    ],
    failures
  );
  assertWorkflowRunScript(ciWorkflow, "rust", RUST_CANONICALIZE_RUN, failures);
  assertCiGateClosure(ciWorkflow, failures);
  assertCandidatePrBoundary(ciWorkflow, failures);

  assertManualDevBuildWorkflow(devBuildWorkflow, failures);
  requireText(devBuildWorkflow, "pnpm exec tauri build", "dev-build.yml", failures);
  requireText(devBuildWorkflow, "Build desktop artifact in the cloud", "dev-build.yml", failures);

  if (failures.length > 0) {
    throw new Error(`Cloud-only verification contract failed:\n- ${failures.join("\n- ")}`);
  }
}

if (process.argv[1] && resolve(process.argv[1]) === modulePath) {
  try {
    assertCloudOnlyVerificationContract(loadCloudOnlyVerificationFixture());
    console.error("[cloud-only-verification] repository contract passed");
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}
