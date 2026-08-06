import assert from "node:assert/strict";

import { assertCiQualityGates } from "./check-ci-quality-gates.mjs";

const guard = "node scripts/require-github-actions.mjs && ";
const packageJson = {
  scripts: {
    "check:no-instant-now-sub":
      `${guard}node scripts/check-no-instant-now-sub.selftest.mjs && node scripts/check-no-instant-now-sub.mjs`,
    "create-aio-plugin:typecheck":
      `${guard}node scripts/check-create-aio-plugin-typecheck.selftest.mjs && pnpm --filter create-aio-plugin typecheck`,
    "check:ci-quality-gates":
      `${guard}node scripts/check-ci-quality-gates.selftest.mjs && node scripts/check-ci-quality-gates.mjs`,
  },
};
const checks = {
  "cloud-only-verification": "node scripts/check-cloud-only-verification.mjs",
  "ci-quality-gates": "pnpm check:ci-quality-gates",
  "create-aio-plugin-typecheck": "pnpm create-aio-plugin:typecheck",
};
const stages = {
  "full-ci": [
    "cloud-only-verification",
    "no-instant-now-sub",
    "ci-quality-gates",
    "create-aio-plugin-typecheck",
  ],
  "plugin-hardening": ["cloud-only-verification", "create-aio-plugin-typecheck"],
};
const workflow = `
  support-contract:
    steps:
      - run: node scripts/check-cloud-only-verification.selftest.mjs && node scripts/check-cloud-only-verification.mjs
      - run: node scripts/check-ci-quality-gates.selftest.mjs && node scripts/check-ci-quality-gates.mjs
      - run: node scripts/check-no-instant-now-sub.selftest.mjs && node scripts/check-no-instant-now-sub.mjs
  frontend:
    steps:
      - run: pnpm install --frozen-lockfile
      - run: pnpm audit:deps
      - run: pnpm lint
      - run: pnpm plugin-sdk:typecheck
      - run: pnpm create-aio-plugin:typecheck
      - run: pnpm plugin-sdk:test
      - run: pnpm --filter create-aio-plugin test
      - run: pnpm test:e2e
      - run: pnpm test:unit:coverage
      - run: pnpm build
  rust:
    steps:
      - run: cargo fmt --manifest-path src-tauri/Cargo.toml --all
      - run: cargo update --manifest-path src-tauri/Cargo.toml --workspace
      - run: cargo run --manifest-path src-tauri/Cargo.toml --locked --example export-bindings
      - run: cargo clippy --workspace --all-targets --locked -- -D warnings
      - run: cargo test --workspace --locked -- --test-threads=1
      - run: cargo audit
  ci-gate:
    needs:
      - support-contract
      - frontend
      - rust
`;

const valid = { packageJson, checks, stages, workflow };
assert.doesNotThrow(() => assertCiQualityGates(valid));

for (const [name, fixture, expected] of [
  [
    "full CI Instant gate",
    {
      ...valid,
      stages: {
        ...stages,
        "full-ci": [stages["full-ci"][0], ...stages["full-ci"].slice(2)],
      },
    },
    /STAGES\.full-ci must include no-instant-now-sub/,
  ],
  [
    "cloud contract stage wiring",
    { ...valid, stages: { ...stages, "full-ci": [] } },
    /STAGES\.full-ci must include cloud-only-verification/,
  ],
  [
    "plugin hardening typecheck",
    { ...valid, stages: { ...stages, "plugin-hardening": [] } },
    /STAGES\.plugin-hardening must include cloud-only-verification/,
  ],
  [
    "support cloud-only step",
    {
      ...valid,
      workflow: workflow.replace(
        "      - run: node scripts/check-cloud-only-verification.selftest.mjs && node scripts/check-cloud-only-verification.mjs\n",
        ""
      ),
    },
    /ci\.yml support-contract must include node scripts\/check-cloud-only-verification/,
  ],
  [
    "frontend build",
    { ...valid, workflow: workflow.replace("      - run: pnpm build\n", "") },
    /ci\.yml frontend must include pnpm build/,
  ],
  [
    "Rust clippy",
    {
      ...valid,
      workflow: workflow.replace(
        "      - run: cargo clippy --workspace --all-targets --locked -- -D warnings\n",
        ""
      ),
    },
    /ci\.yml rust must include cargo clippy/,
  ],
]) {
  assert.throws(() => assertCiQualityGates(fixture), expected, name);
}

console.error("[ci-quality-gates:selftest] all assertions passed");
