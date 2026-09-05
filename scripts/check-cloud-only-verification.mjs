import { readFileSync, readdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const modulePath = fileURLToPath(import.meta.url);
const repoRoot = resolve(dirname(modulePath), "..");

const ROOT_GUARD = "node scripts/require-github-actions.mjs";
const WORKSPACE_GUARD = "node ../../scripts/require-github-actions.mjs";
const ROOT_TEST_INCLUDE = 'include: ["src/**/*.{test,spec}.{ts,tsx}"],';
const LOCAL_COMMAND_LINE =
  /^\s*(?:(?:pnpm|npm|yarn)\s+|cargo\s+(?:audit|build|check|clippy|fmt|run|test)\b|(?:rustfmt|clippy)\s+(?:--|\w)|tauri\s+(?:build|dev|icon|info|signer)\b)/i;
const LOCAL_QUALITY_INSTRUCTION =
  /^\s*(?:-\s*)?(?:run|regenerate|execute)\b.*\b(?:rust|frontend|bindings?|type-?check|lint|clippy|format(?:ting)?|test(?:s| suite)?|cargo|pnpm)\b/i;
const README_FENCE = /```(?:bash|sh|shell)?\s*\n([\s\S]*?)```/gi;
const MANUAL_CI_GUARD_RUN = `set -euo pipefail
[[ "$EVENT_REF" == "refs/heads/main" ]] || {
  echo "::error::Manual ci runs are restricted to the main branch. PR validation is automatic."
  exit 1
}`;

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
    vitestConfig: readText(root, "vitest.config.ts"),
    tauriConfig: readJson(root, "src-tauri/tauri.conf.json"),
    agents: readText(root, "AGENTS.md"),
    readme: readText(root, "README.md"),
    readmeEn: readText(root, "README_EN.md"),
    activeSpecs: readMarkdownTree(root, ".trellis/spec/aio-coding-hub"),
    ciWorkflow: readText(root, ".github/workflows/ci.yml"),
    devBuildWorkflow: readText(root, ".github/workflows/dev-build.yml"),
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

function assertCandidatePrBoundary(workflow, failures) {
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
    '[[ "$FRONTEND_CI" == "true" ]]',
    '[[ "$RUST_CI" == "true" ]]',
    '[[ "$SHARED_CI" == "true" ]]',
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
}

export function assertCloudOnlyVerificationContract(fixture) {
  const failures = [];
  const {
    rootPackage,
    pluginSdkPackage,
    scaffolderPackage,
    vitestConfig,
    tauriConfig,
    agents,
    readme,
    readmeEn,
    activeSpecs,
    ciWorkflow,
    devBuildWorkflow,
  } = fixture;

  assertActionsOnlyScripts(rootPackage, "root package.json", ROOT_GUARD, failures);
  assertActionsOnlyScripts(pluginSdkPackage, "plugin SDK package.json", WORKSPACE_GUARD, failures);
  assertActionsOnlyScripts(scaffolderPackage, "plugin scaffolder package.json", WORKSPACE_GUARD, failures);

  for (const name of ["dev", "preview", "check:precommit", "check:precommit:full", "check:prepush"]) {
    if (rootPackage?.scripts?.[name] !== undefined) {
      failures.push(`root package.json must not expose local ${name} entry point`);
    }
  }
  if (rootPackage?.scripts?.["test:e2e"] !== undefined) {
    failures.push("root package.json test:e2e must stay absent so coverage is the only E2E entry");
  }
  if (
    rootPackage?.scripts?.["test:unit:coverage"] !==
    `${ROOT_GUARD} && vitest run --coverage`
  ) {
    failures.push("root package.json test:unit:coverage must remain the cloud coverage entry");
  }
  if (
    !vitestConfig.includes(ROOT_TEST_INCLUDE) ||
    /exclude:\s*\[[^\]]*src\/e2e/s.test(vitestConfig)
  ) {
    failures.push("vitest.config.ts must discover src/e2e through the root test include");
  }

  if (tauriConfig?.build?.beforeDevCommand !== undefined) {
    failures.push("src-tauri/tauri.conf.json must not define beforeDevCommand");
  }
  if (tauriConfig?.build?.beforeBuildCommand !== "pnpm build") {
    failures.push("src-tauri/tauri.conf.json must retain the cloud frontend build hook");
  }

  requireText(agents, "$gkd-main", "AGENTS.md", failures);
  for (const handoffFile of [".gkd/plan.md", ".gkd/execution.md", ".gkd/progress.md", ".gkd/review.md"]) {
    requireText(agents, handoffFile, "AGENTS.md", failures);
  }
  requireAbsent(
    agents,
    /gkd-task|gkd-role|gkd_acceptor|gkd-local-verify|gkd-verify|TrustedMainRuntimeBridge/i,
    "AGENTS.md",
    failures
  );
  requireAbsent(agents, /Use `pnpm dev`/i, "AGENTS.md", failures);
  for (const [label, text] of [
    ["README.md", readme],
    ["README_EN.md", readmeEn],
  ]) {
    requireText(text, "workflow_dispatch", label, failures);
    requireAbsent(
      text,
      /gkd-task|gkd-role|gkd_acceptor|gkd-local-verify|gkd-verify|TrustedMainRuntimeBridge/i,
      label,
      failures
    );
    assertNoForbiddenReadmeCommand(text, label, failures);
  }

  requireAbsent(agents, /task\.py\s+(?:accept|start|delegate|deliver)/i, "AGENTS.md", failures);
  assertActiveSpecs(activeSpecs, failures);

  if (!/^\s*workflow_dispatch:\s*$/m.test(ciWorkflow)) {
    failures.push("ci.yml must retain workflow_dispatch");
  }
  assertManualCiBoundary(ciWorkflow, failures);
  assertWorkflowRunCommands(
    ciWorkflow,
    "contracts",
    [
      "node scripts/check-cloud-only-verification.mjs",
    ],
    failures
  );
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
