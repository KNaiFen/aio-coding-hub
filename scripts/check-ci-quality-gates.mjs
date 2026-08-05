import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { CHECKS, STAGES } from "./run-checks.mjs";

function hasWorkflowRun(workflow, jobName, command) {
  let currentJob = "";
  for (const line of workflow.split(/\r?\n/)) {
    const job = line.match(/^  ([A-Za-z0-9_-]+):\s*$/);
    if (job) {
      currentJob = job[1];
      continue;
    }
    if (currentJob !== jobName) continue;
    const run = line.match(/^\s*(?:-\s+)?run:\s*(.+)$/);
    if (run && !run[1].trimStart().startsWith("#") && run[1].includes(command)) {
      return true;
    }
  }
  return false;
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
    "node scripts/check-no-instant-now-sub.selftest.mjs && node scripts/check-no-instant-now-sub.mjs"
  ) {
    failures.push("check:no-instant-now-sub must run its self-test before the repository scan");
  }
  if (
    scripts["create-aio-plugin:typecheck"] !==
    "node scripts/check-create-aio-plugin-typecheck.selftest.mjs && pnpm --filter create-aio-plugin typecheck"
  ) {
    failures.push(
      "create-aio-plugin:typecheck must run the negative fixture and package typecheck"
    );
  }
  if (
    scripts["check:ci-quality-gates"] !==
    "node scripts/check-ci-quality-gates.selftest.mjs && node scripts/check-ci-quality-gates.mjs"
  ) {
    failures.push("check:ci-quality-gates must run its self-test before the repository contract");
  }

  if (checks["ci-quality-gates"] !== "pnpm check:ci-quality-gates") {
    failures.push("aggregate checks must define ci-quality-gates");
  }
  if (checks["create-aio-plugin-typecheck"] !== "pnpm create-aio-plugin:typecheck") {
    failures.push("aggregate checks must define create-aio-plugin-typecheck");
  }

  requireStage(stages, "prepush", "no-instant-now-sub", failures);
  requireStage(stages, "prepush", "ci-quality-gates", failures);
  requireStage(stages, "prepush", "create-aio-plugin-typecheck", failures);
  requireStage(stages, "plugin-hardening", "create-aio-plugin-typecheck", failures);

  if (
    !hasWorkflowRun(
      workflow,
      "support-contract",
      "node scripts/check-ci-quality-gates.selftest.mjs && node scripts/check-ci-quality-gates.mjs"
    )
  ) {
    failures.push("support-contract must validate the CI quality matrix");
  }
  if (
    !hasWorkflowRun(
      workflow,
      "support-contract",
      "node scripts/check-no-instant-now-sub.selftest.mjs && node scripts/check-no-instant-now-sub.mjs"
    )
  ) {
    failures.push("support-contract must execute the Instant underflow guard");
  }

  if (!hasWorkflowRun(workflow, "frontend", "pnpm create-aio-plugin:typecheck")) {
    failures.push("frontend CI must execute the plugin scaffolder typecheck");
  }

  if (failures.length > 0) {
    throw new Error(`CI quality gate contract failed:\n- ${failures.join("\n- ")}`);
  }
}

const modulePath = fileURLToPath(import.meta.url);
if (process.argv[1] && resolve(process.argv[1]) === modulePath) {
  const repoRoot = dirname(dirname(modulePath));
  assertCiQualityGates({
    packageJson: JSON.parse(readFileSync(join(repoRoot, "package.json"), "utf8")),
    checks: CHECKS,
    stages: STAGES,
    workflow: readFileSync(join(repoRoot, ".github", "workflows", "ci.yml"), "utf8"),
  });

  console.error("[ci-quality-gates] repository contract passed");
}
