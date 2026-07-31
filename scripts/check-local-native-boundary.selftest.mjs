import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import {
  evaluateLocalNativeBoundary,
  parseScopedHooksPaths,
} from "./check-local-native-boundary.mjs";

const SAFE_PNPM_WORKSPACE = `packages:
  - .

allowBuilds:
  es5-ext: false
  esbuild: true
  msw: false

onlyBuiltDependencies:
  - esbuild
`;
const processFixtures = JSON.parse(
  readFileSync(
    new URL("./fixtures/local-native-boundary-process-cases.json", import.meta.url),
    "utf8"
  )
);

function snapshot(overrides = {}) {
  return {
    collectionErrors: [],
    trackedPaths: ["package.json", "pnpm-workspace.yaml", "scripts/run-checks.mjs"],
    hooksPaths: [],
    manifests: [
      {
        path: "package.json",
        name: "test-root",
        scripts: {
          dev: "vite",
          typecheck: "tsc -p tsconfig.json",
          check: "node scripts/check-local-native-boundary.mjs",
          "check:local-native-boundary":
            "node scripts/check-local-native-boundary.mjs && node scripts/pnpm-cli.selftest.mjs && node scripts/check-local-native-boundary.selftest.mjs",
        },
      },
    ],
    files: {
      "pnpm-workspace.yaml": SAFE_PNPM_WORKSPACE,
      "scripts/run-checks.mjs": 'const checks = ["lint", "typecheck"];\n',
    },
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
expectFailure(
  snapshot({
    manifests: [
      {
        path: "package.json",
        name: "test-root",
        scripts: {
          "check:local-native-boundary":
            "node scripts/check-local-native-boundary.selftest.mjs && node scripts/check-local-native-boundary.mjs",
        },
      },
    ],
  }),
  "live boundary scan must run before boundary self-tests"
);
assert.deepEqual(
  evaluateLocalNativeBoundary(
    snapshot({
      manifests: [
        { path: "package.json", name: "test-root", scripts: { dev: "vite" } },
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
expectFailure(
  snapshot({ trackedPaths: ["package.json", "scripts/repack-linux-appimage-wayland.sh"] }),
  "forbidden local native helper"
);
expectFailure(
  snapshot({
    hooksPaths: [{ scope: "local", origin: "file:.git/config", value: ".githooks" }],
  }),
  "repository-owned override is forbidden"
);
expectFailure(
  snapshot({
    hooksPaths: [
      { scope: "worktree", origin: "file:.git/config.worktree", value: ".custom-hooks" },
    ],
  }),
  "repository-owned override is forbidden"
);
assert.deepEqual(
  evaluateLocalNativeBoundary(
    snapshot({
      hooksPaths: [{ scope: "global", origin: "file:/tmp/global.gitconfig", value: "/hooks" }],
    })
  ),
  []
);
assert.deepEqual(
  parseScopedHooksPaths(
    "global\tfile:/tmp/global.gitconfig\t/hooks\nworktree\tfile:.git/config.worktree\t.local-hooks\n"
  ),
  [
    { scope: "global", origin: "file:/tmp/global.gitconfig", value: "/hooks" },
    { scope: "worktree", origin: "file:.git/config.worktree", value: ".local-hooks" },
  ]
);
assert.throws(() => parseScopedHooksPaths("malformed"));

expectFailure(
  snapshot({
    manifests: [{ path: "package.json", scripts: { postinstall: "node scripts/setup.mjs" } }],
  }),
  "postinstall: forbidden"
);
for (const lifecycle of [
  "preinstall",
  "install",
  "prepare",
  "prepack",
  "postpack",
  "pnpm:devPreinstall",
]) {
  expectFailure(
    snapshot({
      manifests: [
        {
          path: "package.json",
          scripts: { [lifecycle]: "node scripts/check-spec-links.mjs" },
        },
      ],
    }),
    `${lifecycle}: forbidden`
  );
}
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
      { path: "package.json", scripts: { check: "pnpm --filter native-package run test" } },
      { path: "packages/native-package/package.json", scripts: { test: "cargo test" } },
    ],
  }),
  "packages/native-package/package.json script test: invokes Cargo"
);
expectFailure(
  snapshot({
    manifests: [{ path: "package.json", scripts: { rebuild: "vite", check: "pnpm rebuild" } }],
  }),
  "command is outside the exact Node/frontend grammar"
);
expectFailure(
  snapshot({
    manifests: [{ path: "package.json", scripts: { check: "node scripts/desktop-check.mjs" } }],
  }),
  "command is outside the exact Node/frontend grammar"
);
expectFailure(
  snapshot({
    manifests: [{ path: "package.json", scripts: { check: "node -e 'process.exit(0)'" } }],
  }),
  "command is outside the exact Node/frontend grammar"
);
for (const command of [
  "node --import scripts/desktop-check.mjs scripts/check-local-native-boundary.mjs",
  "node -r scripts/desktop-check.cjs scripts/check-local-native-boundary.mjs",
  "tsx --require scripts/desktop-check.cjs scripts/check-local-native-boundary.mjs",
  "node /scripts/check-spec-links.mjs",
  "node C:\\scripts\\check-spec-links.mjs",
]) {
  expectFailure(
    snapshot({ manifests: [{ path: "package.json", scripts: { check: command } }] }),
    "command is outside the exact Node/frontend grammar"
  );
}
expectFailure(
  snapshot({
    manifests: [{ path: "package.json", scripts: { check: "python scripts/desktop-check.py" } }],
  }),
  "command is outside the exact Node/frontend grammar"
);

for (const command of [
  '"node" scripts/native-wrapper.mjs',
  "pnpm dlx arbitrary-native-builder",
  'pnpm exec c""argo test',
  "vite --config scripts/native-config.ts",
]) {
  expectFailure(
    snapshot({ manifests: [{ path: "package.json", scripts: { check: command } }] }),
    "command is outside the exact Node/frontend grammar"
  );
}

expectFailure(
  snapshot({
    files: {
      "scripts/check-spec-links.mjs": processFixtures.directCargo,
    },
  }),
  "scripts/check-spec-links.mjs: process execution is not approved"
);
expectFailure(
  snapshot({
    files: {
      "scripts/run-checks.mjs": processFixtures.directCargo,
    },
  }),
  'scripts/run-checks.mjs: spawnSync command "cargo" is outside its process contract'
);
assert.deepEqual(
  evaluateLocalNativeBoundary(
    snapshot({
      files: {
        "pnpm-workspace.yaml": SAFE_PNPM_WORKSPACE,
        "scripts/run-checks.mjs": processFixtures.shellDisabled,
      },
    })
  ),
  []
);
expectFailure(
  snapshot({
    files: {
      "scripts/run-checks.mjs": processFixtures.shellEnabled,
    },
  }),
  "scripts/run-checks.mjs: shell-enabled process execution is forbidden"
);
expectFailure(
  snapshot({
    files: {
      "scripts/run-checks.mjs": processFixtures.customShell,
    },
  }),
  "scripts/run-checks.mjs: shell-enabled process execution is forbidden"
);
expectFailure(
  snapshot({
    files: {
      "scripts/check-local-native-boundary.selftest.mjs": processFixtures.directCargo,
    },
  }),
  "scripts/check-local-native-boundary.selftest.mjs: process execution is not approved"
);
expectFailure(
  snapshot({ files: { "scripts/run-checks.mjs": processFixtures.namespaceCargo } }),
  "scripts/run-checks.mjs: process execution cannot be statically audited"
);
for (const contents of [
  processFixtures.aliasedCargo,
  processFixtures.optionalCargo,
  processFixtures.renamedImportCargo,
]) {
  expectFailure(
    snapshot({ files: { "scripts/run-checks.mjs": contents } }),
    "scripts/run-checks.mjs: indirect process dispatch is forbidden"
  );
}
expectFailure(
  snapshot({ files: { "scripts/run-checks.mjs": processFixtures.reflectApplyCargo } }),
  "scripts/run-checks.mjs: indirect process dispatch is forbidden"
);
expectFailure(
  snapshot({ files: { "vite.config.ts": processFixtures.dynamicNamespaceCargo } }),
  "vite.config.ts: process execution is not approved"
);
expectFailure(
  snapshot({
    trackedPaths: ["package.json", ".vscode/tasks.json"],
    files: { ".vscode/tasks.json": '{"command":"cargo test"}' },
  }),
  ".vscode/tasks.json: repository-controlled local automation file is forbidden"
);
for (const path of [
  "packages/desktop/Makefile",
  "packages/desktop/GNUmakefile",
  "packages/desktop/.justfile",
  "packages/desktop/.vscode/tasks.json",
  "packages/desktop/.vscode/settings.json",
  "packages/desktop/.vscode/launch.json",
  "packages/desktop/.zed/tasks.json",
  "packages/desktop/project.code-workspace",
]) {
  expectFailure(
    snapshot({
      trackedPaths: ["package.json", path],
      files: { [path]: "cargo test\n" },
    }),
    `${path}: repository-controlled local automation file is forbidden`
  );
}
expectFailure(
  snapshot({
    trackedPaths: ["package.json", "Makefile", "scripts/native.sh"],
    files: {
      Makefile: "check:\n\tscripts/native.sh\n",
      "scripts/native.sh": "cargo test\n",
    },
  }),
  "Makefile: repository-controlled local automation file is forbidden"
);
expectFailure(
  snapshot({
    trackedPaths: ["package.json", "pnpm-workspace.yaml", "binding.gyp"],
  }),
  "binding.gyp: implicit node-gyp install build is forbidden"
);
expectFailure(
  snapshot({
    trackedPaths: ["package.json", "pnpm-workspace.yaml", ".trellis/scripts/task.py"],
    files: { ".trellis/scripts/task.py": 'subprocess.run(["cargo", "test"])\n' },
  }),
  ".trellis/scripts/task.py: local script invokes Cargo"
);
expectFailure(
  snapshot({ trackedPaths: ["package.json", ".pnpmfile.cjs"] }),
  ".pnpmfile.cjs: executable pnpm install hook is forbidden"
);
for (const [path, contents] of [
  [".npmrc", "pnpmfile=scripts/install-hook.cjs\n"],
  [".npmrc", "global-pnpmfile=../native-hook.cjs\n"],
  ["pnpm-workspace.yaml", "pnpmfile: scripts/install-hook.cjs\n"],
]) {
  expectFailure(
    snapshot({ trackedPaths: ["package.json", path], files: { [path]: contents } }),
    `${path}: custom pnpmfile install hook is forbidden`
  );
}
expectFailure(
  snapshot({
    trackedPaths: ["package.json", "pnpm-workspace.yaml", ".npmrc"],
    files: {
      "pnpm-workspace.yaml": SAFE_PNPM_WORKSPACE,
      ".npmrc": "only-built-dependencies=esbuild,native-builder\n",
    },
  }),
  ".npmrc: pnpm dependency build policy override is forbidden"
);
expectFailure(
  snapshot({
    trackedPaths: ["package.json", "pnpm-workspace.yaml", ".npmrc"],
    files: {
      "pnpm-workspace.yaml": SAFE_PNPM_WORKSPACE,
      ".npmrc": "only-built-dependencies-file=./native-builds.json\n",
    },
  }),
  ".npmrc: pnpm dependency build policy override is forbidden"
);
for (const [path, contents] of [
  [".npmrc", "script-shell=/bin/sh\n"],
  [".npmrc", "node-options=--require=./scripts/preload.cjs\n"],
  ["pnpm-workspace.yaml", `${SAFE_PNPM_WORKSPACE}\nscriptShell: /bin/sh\n`],
]) {
  const trackedPaths =
    path === "pnpm-workspace.yaml"
      ? ["package.json", "pnpm-workspace.yaml"]
      : ["package.json", "pnpm-workspace.yaml", path];
  expectFailure(
    snapshot({
      trackedPaths,
      files: { "pnpm-workspace.yaml": SAFE_PNPM_WORKSPACE, [path]: contents },
    }),
    `${path}: custom pnpm execution override is forbidden`
  );
}
expectFailure(
  snapshot({
    files: { "pnpm-workspace.yaml": `${SAFE_PNPM_WORKSPACE}\ndangerouslyAllowAllBuilds: true\n` },
  }),
  "pnpm-workspace.yaml: unexpected pnpm dependency build override is forbidden"
);
for (const contents of [
  `${SAFE_PNPM_WORKSPACE}\n"nodeOptions": "--require=./scripts/preload.cjs"\n`,
  `${SAFE_PNPM_WORKSPACE}\n"scriptShell": ./native-shell\n`,
  `${SAFE_PNPM_WORKSPACE}\n"dangerouslyAllowAllBuilds": true\n`,
  `${SAFE_PNPM_WORKSPACE}\n"globalPnpmfile": ./native-hook.cjs\n`,
  `${SAFE_PNPM_WORKSPACE}\n<<: *nativePolicy\n`,
  `${SAFE_PNPM_WORKSPACE}\n? scriptShell\n: ./native-shell\n`,
]) {
  expectFailure(
    snapshot({ files: { "pnpm-workspace.yaml": contents } }),
    "pnpm-workspace.yaml: top-level keys must use plain canonical syntax"
  );
}
expectFailure(
  snapshot({
    files: { "pnpm-workspace.yaml": `${SAFE_PNPM_WORKSPACE}\nscriptShell: ./native-shell\n` },
  }),
  'pnpm-workspace.yaml: unexpected top-level key "scriptShell"'
);
expectFailure(
  snapshot({
    files: { "pnpm-workspace.yaml": `${SAFE_PNPM_WORKSPACE}\npackages:\n  - packages/*\n` },
  }),
  'pnpm-workspace.yaml: duplicate top-level key "packages"'
);
expectFailure(
  snapshot({
    files: {
      "pnpm-workspace.yaml": `${SAFE_PNPM_WORKSPACE}\nonlyBuiltDependenciesFile: ./native-builds.json\n`,
    },
  }),
  "pnpm-workspace.yaml: unexpected pnpm dependency build override is forbidden"
);
for (const contents of [
  SAFE_PNPM_WORKSPACE.replace("  msw: false", "  msw: false\n  native-builder: true"),
  SAFE_PNPM_WORKSPACE.replace("  - esbuild", "  - esbuild\n  - native-builder"),
]) {
  expectFailure(
    snapshot({ files: { "pnpm-workspace.yaml": contents } }),
    "pnpm-workspace.yaml: dependency build allowlist may only enable esbuild"
  );
}
expectFailure(
  snapshot({
    manifests: [{ path: "package.json", scripts: { check: "vite" }, pnpm: {} }],
  }),
  "package-level pnpm lifecycle policy is forbidden"
);
for (const key of ["husky", "simple-git-hooks", "lefthook"]) {
  expectFailure(
    snapshot({
      manifests: [
        {
          path: "package.json",
          scripts: { check: "vite" },
          hookConfigKeys: [key],
        },
      ],
    }),
    `${key} hook configuration is forbidden`
  );
}
for (const path of [".pre-commit-config.yaml", "lefthook.yml", "packages/app/.husky/pre-commit"]) {
  expectFailure(
    snapshot({ trackedPaths: ["package.json", "pnpm-workspace.yaml", path] }),
    `${path}: tracked repository hook is forbidden`
  );
}
expectFailure(
  snapshot({ files: { ".trellis/config.yaml": "hooks:\n  after_archive:\n    - cargo test\n" } }),
  ".trellis/config.yaml: active lifecycle hooks"
);
for (const indent of ["  ", "\t"]) {
  expectFailure(
    snapshot({
      files: {
        ".trellis/config.yaml": `${indent}hooks:\n${indent}  after_archive:\n${indent}    - cargo test\n`,
      },
    }),
    ".trellis/config.yaml: active lifecycle hooks"
  );
}

const multiple = evaluateLocalNativeBoundary(
  snapshot({
    trackedPaths: ["package.json", ".githooks/pre-push"],
    hooksPaths: [{ scope: "local", origin: "file:.git/config", value: ".githooks" }],
    manifests: [{ path: "package.json", scripts: { check: "cargo test" } }],
  })
);
assert.deepEqual(multiple, [...multiple].sort());
assert.equal(new Set(multiple).size, multiple.length);

console.error("[local-native-boundary:selftest] all assertions passed.");
