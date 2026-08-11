import assert from "node:assert/strict";

import {
  assertCloudOnlyVerificationContract,
  loadCloudOnlyVerificationFixture,
} from "./check-cloud-only-verification.mjs";
import { assertGithubActionsEnvironment } from "./require-github-actions.mjs";

const valid = loadCloudOnlyVerificationFixture();
assert.doesNotThrow(() => assertCloudOnlyVerificationContract(valid));
assert.doesNotThrow(() =>
  assertCloudOnlyVerificationContract({
    ...valid,
    ciWorkflow: valid.ciWorkflow.replace("    steps:\n", "    steps: # executable steps\n"),
  })
);
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
  [
    "manual CI guard condition",
    (fixture) => {
      fixture.ciWorkflow = fixture.ciWorkflow.replace(
        "    if: github.event_name == 'workflow_dispatch'",
        "    if: true"
      );
    },
    /ci\.yml manual-dispatch-guard if must equal/,
  ],
  [
    "manual CI main boundary",
    (fixture) => {
      fixture.ciWorkflow = fixture.ciWorkflow.replace(
        '[[ "$EVENT_REF" == "refs\/heads\/main" ]]',
        '[[ -n "$EVENT_REF" ]]'
      );
    },
    /manual-dispatch-guard must retain the approved fail-closed script/,
  ],
  [
    "manual CI gate name",
    (fixture) => {
      fixture.ciWorkflow = fixture.ciWorkflow.replace(
        "    name: ${{ github.event_name == 'workflow_dispatch' && 'manual-ci-gate' || 'ci-gate' }}",
        "    name: ci-gate"
      );
    },
    /ci\.yml ci-gate name must equal/,
  ],
  [
    "inline PR title dependency",
    (fixture) => {
      fixture.ciWorkflow = fixture.ciWorkflow.replace(
        "      - manual-dispatch-guard\n",
        "      - manual-dispatch-guard\n      - pr-title\n"
      );
    },
    /must not inline or depend on the independent pr-title check/,
  ],
  [
    "PR title edited trigger",
    (fixture) => {
      fixture.prTitleWorkflow = fixture.prTitleWorkflow.replace("edited, ", "");
    },
    /pr-title\.yml must trigger for pull_request edited/,
  ],
  [
    "PR title checkout",
    (fixture) => {
      fixture.prTitleWorkflow = fixture.prTitleWorkflow.replace(
        "    steps:\n",
        "    steps:\n      - uses: actions/checkout@1111111111111111111111111111111111111111\n"
      );
    },
    /pr-title\.yml must not checkout pull request code/,
  ],
  [
    "PR title validation",
    (fixture) => {
      fixture.prTitleWorkflow = fixture.prTitleWorkflow.replace(
        '[[ "$PR_TITLE" =~ $pattern ]]',
        "true"
      );
    },
    /pr-title\.yml pr-title must retain the approved fail-closed script/,
  ],
  [
    "performance pull request trigger",
    (fixture) => {
      fixture.performanceWorkflow = fixture.performanceWorkflow.replace(
        "  workflow_dispatch:\n",
        "  pull_request:\n  workflow_dispatch:\n"
      );
    },
    /performance\.yml must declare only the workflow_dispatch trigger/,
  ],
  [
    "performance main guard",
    (fixture) => {
      fixture.performanceWorkflow = fixture.performanceWorkflow.replace(
        '[[ "$EVENT_REF" == "refs\/heads\/main" ]]',
        '[[ -n "$EVENT_REF" ]]'
      );
    },
    /performance\.yml main-guard must retain the approved fail-closed script/,
  ],
  [
    "performance benchmark",
    (fixture) => {
      fixture.performanceWorkflow = fixture.performanceWorkflow.replace(
        "provider_trend_million_ledger_rows_release_under_one_second",
        "provider_trend_smoke_test"
      );
    },
    /performance\.yml provider-trend-benchmark must retain the approved fail-closed script/,
  ],
  [
    "performance signing access",
    (fixture) => {
      fixture.performanceWorkflow += "\nenvironment: release-signing\n";
    },
    /performance\.yml must not receive release signing access/,
  ],
]) {
  expectContractFailure(name, mutate, expected);
}

expectContractFailure(
  "docs contract must override skipped ancestors explicitly",
  (fixture) => {
    fixture.ciWorkflow = fixture.ciWorkflow.replace(
      "    if: >-\n      always() &&\n      needs.change-scope.result == 'success' &&\n      needs.change-scope.outputs.docs_checks == 'true'",
      "    if: needs.change-scope.outputs.docs_checks == 'true'"
    );
  },
  /ci\.yml docs-contract if must equal/
);
expectContractFailure(
  "frontend must require support contract success explicitly",
  (fixture) => {
    fixture.ciWorkflow = fixture.ciWorkflow.replace(
      "      needs.change-scope.outputs.frontend_ci == 'true' &&\n      needs.support-contract.result == 'success'\n    runs-on: ubuntu-latest",
      "      needs.change-scope.outputs.frontend_ci == 'true'\n    runs-on: ubuntu-latest"
    );
  },
  /ci\.yml frontend if must equal/
);
expectContractFailure(
  "support contract must run for either code domain",
  (fixture) => {
    fixture.ciWorkflow = fixture.ciWorkflow.replace(
      "      (needs.change-scope.outputs.frontend_ci == 'true' || needs.change-scope.outputs.rust_ci == 'true')",
      "      needs.change-scope.outputs.full_ci == 'true'"
    );
  },
  /ci\.yml support-contract if must equal/
);
expectContractFailure(
  "candidate plan must override skipped ancestors explicitly",
  (fixture) => {
    fixture.ciWorkflow = fixture.ciWorkflow.replace(
      "      always() &&\n      needs.change-scope.result == 'success' &&\n      needs.change-scope.outputs.full_ci == 'true' &&\n      github.repository == 'KNaiFen/aio-coding-hub' &&",
      "      needs.change-scope.outputs.full_ci == 'true' &&\n      github.repository == 'KNaiFen/aio-coding-hub' &&"
    );
  },
  /ci\.yml candidate-plan if must equal/
);
expectContractFailure(
  "candidate builds must require every dependency explicitly",
  (fixture) => {
    fixture.ciWorkflow = fixture.ciWorkflow.replace(
      "    if: >-\n      always() &&\n      needs.support-contract.result == 'success' &&\n      needs.frontend.result == 'success' &&\n      needs.rust.result == 'success' &&\n      needs.candidate-plan.result == 'success' &&\n      needs.candidate-plan.outputs.should_build == 'true'",
      "    if: needs.candidate-plan.outputs.should_build == 'true'"
    );
  },
  /ci\.yml build-release-candidate if must equal/
);

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
    job === "rust" && /^(?:cargo fmt|cargo update|cargo run)/.test(command)
      ? /ci\.yml rust must retain the approved fail-closed script/
      : new RegExp(`ci\\.yml ${job} must include`)
  );
}

expectContractFailure(
  "frontend echoed command is not executable",
  (fixture) => {
    fixture.ciWorkflow = fixture.ciWorkflow.replace("        run: pnpm build", "        run: echo pnpm build");
  },
  /ci\.yml frontend must include pnpm build/
);
expectContractFailure(
  "frontend ignored failure is not a quality gate",
  (fixture) => {
    fixture.ciWorkflow = fixture.ciWorkflow.replace("        run: pnpm build", "        run: pnpm build || true");
  },
  /ci\.yml frontend must include pnpm build/
);
expectContractFailure(
  "Rust conditional no-op is not executable",
  (fixture) => {
    fixture.ciWorkflow = fixture.ciWorkflow.replace(
      "        run: cargo audit",
      "        run: if false; then cargo audit; fi"
    );
  },
  /ci\.yml rust must include cargo audit/
);

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
expectContractFailure(
  "frontend required step cannot be skipped",
  (fixture) => {
    fixture.ciWorkflow = fixture.ciWorkflow.replace(
      "      - name: Build frontend\n        run: pnpm build",
      "      - name: Build frontend\n        if: ${{ false }}\n        run: pnpm build"
    );
  },
  /ci\.yml frontend must include pnpm build/
);
expectContractFailure(
  "frontend required step cannot ignore failures",
  (fixture) => {
    fixture.ciWorkflow = fixture.ciWorkflow.replace(
      "      - name: Build frontend\n        run: pnpm build",
      "      - name: Build frontend\n        continue-on-error: true\n        run: pnpm build"
    );
  },
  /ci\.yml frontend must include pnpm build/
);
expectContractFailure(
  "aggregate gate must always run",
  (fixture) => {
    fixture.ciWorkflow = fixture.ciWorkflow.replace(
      "  ci-gate:\n    name: ${{ github.event_name == 'workflow_dispatch' && 'manual-ci-gate' || 'ci-gate' }}\n    if: always()",
      "  ci-gate:\n    name: ${{ github.event_name == 'workflow_dispatch' && 'manual-ci-gate' || 'ci-gate' }}\n    if: false"
    );
  },
  /ci\.yml ci-gate if must equal always\(\)/
);
expectContractFailure(
  "aggregate gate must bind the real Rust result",
  (fixture) => {
    fixture.ciWorkflow = fixture.ciWorkflow.replace(
      "          RUST_RESULT: ${{ needs.rust.result }}",
      "          RUST_RESULT: success"
    );
  },
  /ci\.yml ci-gate must bind aggregation results directly from needs\.\*/
);
expectContractFailure(
  "aggregate gate must bind the real frontend selection",
  (fixture) => {
    fixture.ciWorkflow = fixture.ciWorkflow.replace(
      "          FRONTEND_CI: ${{ needs.change-scope.outputs.frontend_ci }}",
      "          FRONTEND_CI: true"
    );
  },
  /ci\.yml ci-gate must bind aggregation results directly from needs\.\*/
);
expectContractFailure(
  "aggregate gate script cannot be conditionally disabled",
  (fixture) => {
    fixture.ciWorkflow = fixture.ciWorkflow.replace(
      "          set -euo pipefail\n\n          if [[ \"$EVENT_NAME\" == \"workflow_dispatch\" ]]; then",
      "          set -euo pipefail\n\n          if false; then\n            :\n          fi\n\n          if [[ \"$EVENT_NAME\" == \"workflow_dispatch\" ]]; then"
    );
  },
  /ci\.yml ci-gate must retain the approved fail-closed aggregation script/
);
expectContractFailure(
  "PR title step cannot be skipped",
  (fixture) => {
    fixture.prTitleWorkflow = fixture.prTitleWorkflow.replace(
      "      - name: Check PR title\n        shell: bash",
      "      - name: Check PR title\n        if: ${{ false }}\n        shell: bash"
    );
  },
  /pr-title\.yml pr-title must retain the approved fail-closed script/
);
expectContractFailure(
  "performance benchmark step cannot ignore failures",
  (fixture) => {
    fixture.performanceWorkflow = fixture.performanceWorkflow.replace(
      "      - name: Provider trend million-row release benchmark\n        working-directory:",
      "      - name: Provider trend million-row release benchmark\n        continue-on-error: true\n        working-directory:"
    );
  },
  /performance\.yml provider-trend-benchmark must retain the approved fail-closed script/
);

console.error("[cloud-only-verification:selftest] all assertions passed");
