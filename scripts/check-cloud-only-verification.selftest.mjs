import assert from "node:assert/strict";

import {
  assertCloudOnlyVerificationContract,
  loadCloudOnlyVerificationFixture,
} from "./check-cloud-only-verification.mjs";
import { assertGithubActionsEnvironment } from "./require-github-actions.mjs";

const valid = loadCloudOnlyVerificationFixture();
assert.doesNotThrow(() => assertCloudOnlyVerificationContract(valid));
assert.throws(() => assertGithubActionsEnvironment({}), /GitHub Actions-only/);
assert.doesNotThrow(() => assertGithubActionsEnvironment({ GITHUB_ACTIONS: "true" }));

function cloneFixture() {
  return {
    ...structuredClone({
      ...valid,
      activeSpecs: undefined,
    }),
    activeSpecs: new Map(valid.activeSpecs),
  };
}

function expectContractFailure(name, mutate, expected) {
  const fixture = cloneFixture();
  mutate(fixture);
  assert.throws(() => assertCloudOnlyVerificationContract(fixture), expected, name);
}

for (const [name, mutate, expected] of [
  [
    "root script guard",
    (fixture) => {
      fixture.rootPackage.scripts.build = "tsc && vite build";
    },
    /root package\.json script build must start/,
  ],
  [
    "local dev entry",
    (fixture) => {
      fixture.rootPackage.scripts.dev = "vite";
    },
    /must not expose local dev entry point/,
  ],
  [
    "workspace guard",
    (fixture) => {
      fixture.pluginSdkPackage.scripts.test = "vitest run";
    },
    /plugin SDK package\.json script test must start/,
  ],
  [
    "README local install",
    (fixture) => {
      fixture.readme += "\n```bash\npnpm install\n```\n";
    },
    /README\.md must not present a package\/native command/,
  ],
  [
    "Trellis local lint instruction",
    (fixture) => {
      fixture.trellisWorkflow += "\nRun project lint and type-check\n";
    },
    /\.trellis\/workflow\.md contains a prohibited local instruction/,
  ],
  [
    "active spec bare cargo command",
    (fixture) => {
      fixture.activeSpecs.set("cross-layer/example.md", "cargo test --locked\n");
    },
    /active spec cross-layer\/example\.md contains a bare local package\/native command/,
  ],
  [
    "active spec local quality instruction",
    (fixture) => {
      fixture.activeSpecs.set("cross-layer/example.md", "- Run the full Rust suite after this change.\n");
    },
    /active spec cross-layer\/example\.md contains a local quality-gate instruction/,
  ],
  [
    "frontend build gate",
    (fixture) => {
      fixture.ciWorkflow = fixture.ciWorkflow.replace("      - name: Build frontend\n        run: pnpm build\n", "");
    },
    /ci\.yml frontend must include pnpm build/,
  ],
  [
    "Rust clippy gate",
    (fixture) => {
      fixture.ciWorkflow = fixture.ciWorkflow.replace(
        "        run: cargo clippy --workspace --all-targets --locked -- -D warnings",
        "        run: true"
      );
    },
    /ci\.yml rust must include cargo clippy/,
  ],
  [
    "manual dev-build",
    (fixture) => {
      fixture.devBuildWorkflow = fixture.devBuildWorkflow.replace(/^\s*workflow_dispatch:\s*$/m, "");
    },
    /dev-build\.yml must declare only the workflow_dispatch trigger/,
  ],
  [
    "dev-build pull request trigger",
    (fixture) => {
      fixture.devBuildWorkflow = fixture.devBuildWorkflow.replace(
        "  workflow_dispatch:\n",
        "  pull_request:\n  workflow_dispatch:\n"
      );
    },
    /dev-build\.yml must declare only the workflow_dispatch trigger/,
  ],
]) {
  expectContractFailure(name, mutate, expected);
}

for (const [job, command] of [
  ["frontend", "pnpm install --frozen-lockfile"],
  ["frontend", "pnpm audit:deps"],
  ["frontend", "pnpm lint"],
  ["frontend", "pnpm plugin-sdk:typecheck"],
  ["frontend", "pnpm create-aio-plugin:typecheck"],
  ["frontend", "pnpm plugin-sdk:test"],
  ["frontend", "pnpm --filter create-aio-plugin test"],
  ["frontend", "pnpm test:e2e"],
  ["frontend", "pnpm test:unit:coverage"],
  ["frontend", "pnpm build"],
  ["rust", "cargo fmt --manifest-path src-tauri/Cargo.toml --all"],
  ["rust", "cargo update --manifest-path src-tauri/Cargo.toml --workspace"],
  ["rust", "cargo run --manifest-path src-tauri/Cargo.toml --locked --example export-bindings"],
  ["rust", "cargo clippy --workspace --all-targets --locked -- -D warnings"],
  ["rust", "cargo test --workspace --locked -- --test-threads=1"],
  ["rust", "cargo audit"],
]) {
  expectContractFailure(
    `${job} quality gate ${command}`,
    (fixture) => {
      fixture.ciWorkflow = fixture.ciWorkflow.replace(command, "true");
    },
    new RegExp(`ci\\.yml ${job} must include`)
  );
}

expectContractFailure(
  "docs cloud-only step",
  (fixture) => {
    fixture.ciWorkflow = fixture.ciWorkflow.replace(
      "        run: node scripts/check-cloud-only-verification.mjs",
      "        run: true"
    );
  },
  /ci\.yml docs-contract must include node scripts\/check-cloud-only-verification\.mjs/
);
expectContractFailure(
  "support cloud-only step",
  (fixture) => {
    fixture.ciWorkflow = fixture.ciWorkflow.replace(
      "node scripts/check-cloud-only-verification.selftest.mjs && node scripts/check-cloud-only-verification.mjs",
      "true"
    );
  },
  /ci\.yml support-contract must include node scripts\/check-cloud-only-verification/
);
expectContractFailure(
  "candidate main-only condition",
  (fixture) => {
    fixture.ciWorkflow = fixture.ciWorkflow.replace(
      "github.ref == 'refs/heads/main' &&\n      (github.event_name == 'push' || github.event_name == 'workflow_dispatch')",
      "github.ref == 'refs/heads/dev' &&\n      (github.event_name == 'push' || github.event_name == 'workflow_dispatch')"
    );
  },
  /ci\.yml candidate-plan if must equal/
);
expectContractFailure(
  "candidate condition cannot expand to pull requests",
  (fixture) => {
    fixture.ciWorkflow = fixture.ciWorkflow.replace(
      "(github.event_name == 'push' || github.event_name == 'workflow_dispatch')",
      "(github.event_name == 'push' || github.event_name == 'workflow_dispatch' || github.event_name == 'pull_request')"
    );
  },
  /ci\.yml candidate-plan if must equal/
);
expectContractFailure(
  "candidate PR skip boundary",
  (fixture) => {
    fixture.ciWorkflow = fixture.ciWorkflow.replace(
      `            else
              [[ "$PLAN_RESULT" == "skipped" ]]
              [[ "$BUILD_RESULT" == "skipped" ]]
              [[ "$TUI_BUILD_RESULT" == "skipped" ]]
              [[ "$ASSEMBLE_RESULT" == "skipped" ]]
            fi
          else`,
      `            else
              [[ "$PLAN_RESULT" == "skipped" ]]
              [[ "$BUILD_RESULT" == "skipped" ]]
              [[ "$TUI_BUILD_RESULT" == "success" ]]
              [[ "$ASSEMBLE_RESULT" == "skipped" ]]
            fi
          else`
    );
  },
  /ci\.yml ci-gate must require candidate desktop\/TUI jobs to be skipped/
);
expectContractFailure(
  "candidate boundary must remain under full CI",
  (fixture) => {
    fixture.ciWorkflow = fixture.ciWorkflow.replace(
      'if [[ "$FULL_CI" == "true" ]]; then',
      'if [[ "$FULL_CI" == "false" ]]; then'
    );
  },
  /ci\.yml ci-gate must require candidate desktop\/TUI jobs to be skipped/
);
expectContractFailure(
  "workflow comment is not a frontend gate",
  (fixture) => {
    fixture.ciWorkflow = fixture.ciWorkflow.replace(
      "        run: pnpm build",
      "        # pnpm build"
    );
  },
  /ci\.yml frontend must include pnpm build/
);
expectContractFailure(
  "workflow env run field is not a frontend gate",
  (fixture) => {
    fixture.ciWorkflow = fixture.ciWorkflow.replace(
      "        run: pnpm build",
      "        env:\n          run: pnpm build"
    );
  },
  /ci\.yml frontend must include pnpm build/
);
expectContractFailure(
  "workflow non-step run field is not a frontend gate",
  (fixture) => {
    fixture.ciWorkflow = fixture.ciWorkflow.replace(
      "        run: pnpm build",
      "        with:\n          run: pnpm build"
    );
  },
  /ci\.yml frontend must include pnpm build/
);

console.error("[cloud-only-verification:selftest] all assertions passed");
