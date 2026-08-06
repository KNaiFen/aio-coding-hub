import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { CHECKS, STAGES } from "./run-checks.mjs";

const ACTIONS_GUARD = "node scripts/require-github-actions.mjs && ";

function workflowJobBody(workflow, jobName) {
  const job = new RegExp(`^  ${jobName}:\\s*$`, "m").exec(workflow);
  if (!job) return "";
  const after = workflow.slice(job.index + job[0].length);
  const nextJob = /^  [A-Za-z0-9_-]+:\s*$/m.exec(after);
  return nextJob ? after.slice(0, nextJob.index) : after;
}

function requireWorkflowCommands(workflow, jobName, commands, failures) {
  const body = workflowJobBody(workflow, jobName);
  if (!body) {
    failures.push(`ci.yml must define ${jobName}`);
    return;
  }
  for (const command of commands) {
    if (!body.includes(command)) failures.push(`ci.yml ${jobName} must include ${command}`);
  }
}

function requireStage(stages, stage, check, failures) {
  if (!stages[stage]?.includes(check)) {
    failures.push(`STAGES.${stage} must include ${check}`);
  }
}

export function assertCiQualityGates({ packageJson, checks, stages, workflow }) {
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
    workflow,
    "support-contract",
    [
      "node scripts/check-cloud-only-verification.selftest.mjs && node scripts/check-cloud-only-verification.mjs",
      "node scripts/check-ci-quality-gates.selftest.mjs && node scripts/check-ci-quality-gates.mjs",
      "node scripts/check-no-instant-now-sub.selftest.mjs && node scripts/check-no-instant-now-sub.mjs",
    ],
    failures
  );
  requireWorkflowCommands(
    workflow,
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
    workflow,
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
  requireWorkflowCommands(workflow, "ci-gate", ["- support-contract", "- frontend", "- rust"], failures);

  if (failures.length > 0) {
    throw new Error(`CI quality gate contract failed:\n- ${failures.join("\n- ")}`);
  }
}

const modulePath = fileURLToPath(import.meta.url);
if (process.argv[1] && resolve(process.argv[1]) === modulePath) {
  const repoRoot = dirname(dirname(modulePath));
  try {
    assertCiQualityGates({
      packageJson: JSON.parse(readFileSync(join(repoRoot, "package.json"), "utf8")),
      checks: CHECKS,
      stages: STAGES,
      workflow: readFileSync(join(repoRoot, ".github", "workflows", "ci.yml"), "utf8"),
    });
    console.error("[ci-quality-gates] repository contract passed");
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}
