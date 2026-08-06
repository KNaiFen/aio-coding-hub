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

function workflowRunBodies(workflow, job) {
  const body = workflowJobBody(workflow, job);
  if (!body) return [];

  const lines = body.split(/\r?\n/);
  const runs = [];
  let inSteps = false;
  for (let index = 0; index < lines.length; index += 1) {
    if (/^ {4}steps:\s*$/.test(lines[index])) {
      inSteps = true;
      continue;
    }
    if (!inSteps) continue;
    if (/^ {4}\S/.test(lines[index])) {
      inSteps = false;
      continue;
    }

    const match = /^(?:( {6})-\s+run:|( {8})run:)\s*(.*)$/.exec(lines[index]);
    if (!match) continue;

    const keyIndent = match[1]?.length ?? match[2]?.length ?? 0;
    const value = stripWorkflowComment(match[3]).trim();
    if (!/^[>|]/.test(value)) {
      if (value) runs.push(value);
      continue;
    }

    const block = [];
    let contentIndent;
    while (index + 1 < lines.length) {
      const next = lines[index + 1];
      if (!next.trim()) {
        block.push("");
        index += 1;
        continue;
      }

      const nextIndent = /^\s*/.exec(next)[0].length;
      if (nextIndent <= keyIndent) break;
      contentIndent ??= nextIndent;
      block.push(stripWorkflowComment(next.slice(contentIndent)));
      index += 1;
    }
    runs.push(block.join("\n"));
  }
  return runs;
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

function assertWorkflowRunCommands(workflow, job, commands, failures) {
  const runs = workflowRunBodies(workflow, job);
  if (runs.length === 0) {
    failures.push(`ci.yml must define ${job} with a run step`);
    return;
  }

  for (const command of commands) {
    if (!runs.some((run) => run.includes(command))) {
      failures.push(`ci.yml ${job} must include ${command}`);
    }
  }
}

function assertWorkflowJobPropertyEquals(workflow, job, property, expected, failures) {
  if (!workflowJobBody(workflow, job)) {
    failures.push(`ci.yml must define ${job}`);
    return;
  }
  if (workflowJobProperty(workflow, job, property) !== expected) {
    failures.push(`ci.yml ${job} ${property} must equal ${expected}`);
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
  const onBlock = /^on:\s*\n([\s\S]*?)(?=^(?:permissions|concurrency|env|defaults|jobs):)/m.exec(workflow)?.[1];
  if (!onBlock) {
    failures.push("dev-build.yml must define an on block");
    return;
  }

  const triggers = [...onBlock.matchAll(/^\s{2}([A-Za-z][A-Za-z0-9_-]*):/gm)].map(
    (match) => match[1]
  );
  if (triggers.length !== 1 || triggers[0] !== "workflow_dispatch") {
    failures.push("dev-build.yml must declare only the workflow_dispatch trigger");
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
      "cargo fmt --manifest-path src-tauri/Cargo.toml --all",
      "cargo update --manifest-path src-tauri/Cargo.toml --workspace",
      "cargo run --manifest-path src-tauri/Cargo.toml --locked --example export-bindings",
      "cargo clippy --workspace --all-targets --locked -- -D warnings",
      "cargo test --workspace --locked -- --test-threads=1",
      "cargo audit",
    ],
    failures
  );
  assertCandidatePrBoundary(ciWorkflow, failures);
  assertWorkflowRunCommands(
    ciWorkflow,
    "ci-gate",
    [
      '[[ "$PLAN_RESULT" == "skipped" ]]',
      '[[ "$BUILD_RESULT" == "skipped" ]]',
      '[[ "$TUI_BUILD_RESULT" == "skipped" ]]',
      '[[ "$ASSEMBLE_RESULT" == "skipped" ]]',
    ],
    failures
  );

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
