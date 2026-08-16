import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const modulePath = fileURLToPath(import.meta.url);

export function assertGithubActionsEnvironment(env = process.env) {
  if (env.GITHUB_ACTIONS === "true") return;

  throw new Error(
    "This repository package script is GitHub Actions-only. " +
      "Run node scripts/check-local-verification.mjs --base <full-task-base-sha> locally instead."
  );
}

if (process.argv[1] && resolve(process.argv[1]) === modulePath) {
  try {
    assertGithubActionsEnvironment();
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}
