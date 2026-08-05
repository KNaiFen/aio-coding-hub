import assert from "node:assert/strict";

import { assertCiQualityGates } from "./check-ci-quality-gates.mjs";

const packageJson = {
  scripts: {
    "check:no-instant-now-sub":
      "node scripts/check-no-instant-now-sub.selftest.mjs && node scripts/check-no-instant-now-sub.mjs",
    "create-aio-plugin:typecheck":
      "node scripts/check-create-aio-plugin-typecheck.selftest.mjs && pnpm --filter create-aio-plugin typecheck",
    "check:ci-quality-gates":
      "node scripts/check-ci-quality-gates.selftest.mjs && node scripts/check-ci-quality-gates.mjs",
  },
};
const checks = {
  "ci-quality-gates": "pnpm check:ci-quality-gates",
  "create-aio-plugin-typecheck": "pnpm create-aio-plugin:typecheck",
};
const stages = {
  prepush: ["no-instant-now-sub", "ci-quality-gates", "create-aio-plugin-typecheck"],
  "plugin-hardening": ["create-aio-plugin-typecheck"],
};
const workflow = `
  support-contract:
    steps:
      - run: node scripts/check-ci-quality-gates.selftest.mjs && node scripts/check-ci-quality-gates.mjs
      - run: node scripts/check-no-instant-now-sub.selftest.mjs && node scripts/check-no-instant-now-sub.mjs
  frontend:
    steps:
      - run: pnpm create-aio-plugin:typecheck
  rust:
`;

const valid = { packageJson, checks, stages, workflow };
assert.doesNotThrow(() => assertCiQualityGates(valid));

for (const [name, fixture, expected] of [
  [
    "prepush Instant gate",
    { ...valid, stages: { ...stages, prepush: stages.prepush.slice(1) } },
    /STAGES\.prepush must include no-instant-now-sub/,
  ],
  [
    "prepush stage wiring",
    { ...valid, stages: { ...stages, prepush: [] } },
    /STAGES\.prepush must include no-instant-now-sub/,
  ],
  [
    "plugin hardening typecheck",
    {
      ...valid,
      stages: { ...stages, "plugin-hardening": [] },
    },
    /STAGES\.plugin-hardening must include create-aio-plugin-typecheck/,
  ],
  [
    "support Instant step",
    {
      ...valid,
      workflow: workflow.replace(
        "      - run: node scripts/check-no-instant-now-sub.selftest.mjs && node scripts/check-no-instant-now-sub.mjs\n",
        ""
      ),
    },
    /support-contract must execute the Instant underflow guard/,
  ],
  [
    "frontend scaffolder typecheck",
    {
      ...valid,
      workflow: workflow.replace("      - run: pnpm create-aio-plugin:typecheck\n", ""),
    },
    /frontend CI must execute the plugin scaffolder typecheck/,
  ],
]) {
  assert.throws(() => assertCiQualityGates(fixture), expected, name);
}

console.error("[ci-quality-gates:selftest] all assertions passed");
