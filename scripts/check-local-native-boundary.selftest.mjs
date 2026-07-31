import assert from "node:assert/strict";

import { evaluateLocalNativeBoundary } from "./check-local-native-boundary.mjs";

function snapshot(overrides = {}) {
  return {
    collectionErrors: [],
    trackedPaths: ["package.json", "scripts/run-checks.mjs"],
    hooksPaths: [],
    manifests: [
      {
        path: "package.json",
        scripts: {
          dev: "vite",
          typecheck: "tsc -p tsconfig.json",
          check: "node scripts/check-local-native-boundary.mjs",
        },
      },
    ],
    files: { "scripts/run-checks.mjs": 'const checks = ["lint", "typecheck"];\n' },
    ...overrides,
  };
}

function expectFailure(value, phrase) {
  const violations = evaluateLocalNativeBoundary(value);
  assert.ok(
    violations.some((violation) => violation.includes(phrase)),
    violations.join("\n")
  );
}

assert.deepEqual(evaluateLocalNativeBoundary(snapshot()), []);
assert.deepEqual(
  evaluateLocalNativeBoundary(
    snapshot({
      manifests: [
        { path: "package.json", scripts: { dev: "vite" } },
        { path: "packages/data/package.json" },
      ],
    })
  ),
  []
);
expectFailure(
  snapshot({ manifests: [{ path: "package.json", scripts: "cargo test" }] }),
  "scripts must be an object"
);

expectFailure(
  snapshot({ trackedPaths: ["package.json", ".githooks/pre-commit"] }),
  "tracked repository hook"
);
expectFailure(
  snapshot({ trackedPaths: ["package.json", ".husky/pre-push"] }),
  "tracked repository hook"
);
expectFailure(
  snapshot({ trackedPaths: ["package.json", "scripts/install-git-hooks.mjs"] }),
  "forbidden local native helper"
);
expectFailure(snapshot({ hooksPaths: [".githooks"] }), "repository-local override is forbidden");
expectFailure(
  snapshot({ hooksPaths: [".custom-hooks"] }),
  "repository-local override is forbidden"
);

expectFailure(
  snapshot({
    manifests: [{ path: "package.json", scripts: { postinstall: "node scripts/setup.mjs" } }],
  }),
  "postinstall: forbidden"
);
expectFailure(
  snapshot({
    manifests: [{ path: "package.json", scripts: { "tauri:dev": "node scripts/dev.mjs" } }],
  }),
  "tauri:dev: forbidden"
);
expectFailure(
  snapshot({
    manifests: [
      { path: "package.json", scripts: { prepare: "git config core.hooksPath .githooks" } },
    ],
  }),
  "configures repository hooks"
);

for (const command of [
  "cargo test --locked",
  "C:\\toolchains\\cargo.exe test",
  "rustfmt --check src-tauri/src/lib.rs",
  "pnpm exec tauri build",
  "sh -c 'cargo test'",
  "wasm-pack build",
  "cargo run --example export-bindings",
]) {
  expectFailure(
    snapshot({ manifests: [{ path: "package.json", scripts: { check: command } }] }),
    "script check:"
  );
}

expectFailure(
  snapshot({
    manifests: [
      { path: "package.json", scripts: { check: "pnpm --filter native-package test" } },
      { path: "packages/native-package/package.json", scripts: { test: "cargo test" } },
    ],
  }),
  "packages/native-package/package.json script test: invokes Cargo"
);
expectFailure(
  snapshot({
    manifests: [{ path: "package.json", scripts: { check: "node scripts/desktop-check.mjs" } }],
  }),
  "helper scripts/desktop-check.mjs is not in the Node/frontend allowlist"
);
expectFailure(
  snapshot({
    manifests: [{ path: "package.json", scripts: { check: "node -e 'process.exit(0)'" } }],
  }),
  "inline Node execution"
);
for (const command of [
  "node --import scripts/desktop-check.mjs scripts/check-local-native-boundary.mjs",
  "node -r scripts/desktop-check.cjs scripts/check-local-native-boundary.mjs",
  "tsx --require scripts/desktop-check.cjs scripts/check-local-native-boundary.mjs",
]) {
  expectFailure(
    snapshot({ manifests: [{ path: "package.json", scripts: { check: command } }] }),
    "must directly invoke an approved helper file"
  );
}
expectFailure(
  snapshot({
    manifests: [{ path: "package.json", scripts: { check: "python scripts/desktop-check.py" } }],
  }),
  "executable python is not in the Node/frontend allowlist"
);
expectFailure(
  snapshot({ files: { "scripts/run-checks.mjs": 'const checks = ["pnpm exec tauri build"];\n' } }),
  "scripts/run-checks.mjs: invokes Tauri CLI"
);
expectFailure(
  snapshot({
    trackedPaths: ["package.json", ".vscode/tasks.json"],
    files: { ".vscode/tasks.json": '{"command":"cargo test"}' },
  }),
  ".vscode/tasks.json: local automation invokes Cargo"
);
expectFailure(
  snapshot({ files: { ".trellis/config.yaml": "hooks:\n  after_archive:\n    - cargo test\n" } }),
  ".trellis/config.yaml: active lifecycle hooks"
);

const multiple = evaluateLocalNativeBoundary(
  snapshot({
    trackedPaths: ["package.json", ".githooks/pre-push"],
    hooksPaths: [".githooks"],
    manifests: [{ path: "package.json", scripts: { check: "cargo test" } }],
  })
);
assert.deepEqual(multiple, [...multiple].sort());
assert.equal(new Set(multiple).size, multiple.length);

console.error("[local-native-boundary:selftest] all assertions passed.");
