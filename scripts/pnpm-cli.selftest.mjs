import assert from "node:assert/strict";

import {
  createAuditListInvocation,
  createCheckScriptInvocation,
  createCoverageMergeInvocation,
  createCoverageShardInvocation,
} from "./lib/pnpm-cli.mjs";

const posixJs = {
  platform: "linux",
  execPath: "/usr/bin/node",
  npmExecPath: "/opt/pnpm/bin/pnpm.cjs",
};
const windowsJs = {
  platform: "win32",
  execPath: "C:\\Program Files\\nodejs\\node.exe",
  npmExecPath: "C:\\pnpm\\pnpm.cjs",
};

assert.deepEqual(createCheckScriptInvocation("lint", posixJs), {
  command: "/usr/bin/node",
  args: ["/opt/pnpm/bin/pnpm.cjs", "run", "lint"],
});
assert.deepEqual(createCheckScriptInvocation("test:unit:coverage:shards", windowsJs), {
  command: "C:\\Program Files\\nodejs\\node.exe",
  args: ["C:\\pnpm\\pnpm.cjs", "run", "test:unit:coverage:shards"],
});

assert.deepEqual(
  createCheckScriptInvocation("typecheck", {
    platform: "linux",
    execPath: "/usr/bin/node",
    npmExecPath: "/opt/pnpm/bin/pnpm",
  }),
  { command: "/opt/pnpm/bin/pnpm", args: ["run", "typecheck"] }
);
assert.deepEqual(
  createCheckScriptInvocation("typecheck", {
    platform: "win32",
    execPath: "C:\\node.exe",
    npmExecPath: "C:\\pnpm\\pnpm.exe",
  }),
  { command: "C:\\pnpm\\pnpm.exe", args: ["run", "typecheck"] }
);

assert.deepEqual(createAuditListInvocation(posixJs), {
  command: "/usr/bin/node",
  args: [
    "/opt/pnpm/bin/pnpm.cjs",
    "list",
    "--recursive",
    "--prod",
    "--depth",
    "Infinity",
    "--json",
  ],
});
assert.deepEqual(createCoverageMergeInvocation(posixJs), {
  command: "/usr/bin/node",
  args: ["/opt/pnpm/bin/pnpm.cjs", "exec", "vitest", "run", "--merge-reports", "--coverage"],
});
assert.deepEqual(createCoverageShardInvocation(2, posixJs).args, [
  "/opt/pnpm/bin/pnpm.cjs",
  "exec",
  "vitest",
  "run",
  "--reporter=blob",
  "--coverage",
  "--coverage.thresholds.statements=0",
  "--coverage.thresholds.branches=0",
  "--coverage.thresholds.functions=0",
  "--coverage.thresholds.lines=0",
  "--shard=2/4",
]);

for (const script of ["rebuild", "tauri:build", "bad\0script"]) {
  assert.throws(() => createCheckScriptInvocation(script, posixJs));
}
for (const shard of [0, 5, 1.5, "1"]) {
  assert.throws(() => createCoverageShardInvocation(shard, posixJs));
}
for (const invalid of [
  { platform: "linux", execPath: "node", npmExecPath: "/opt/pnpm/bin/pnpm.cjs" },
  { platform: "linux", execPath: "/usr/bin/node", npmExecPath: "pnpm.cjs" },
  { platform: "linux", execPath: "/usr/bin/node", npmExecPath: "/usr/local/bin/not-pnpm" },
  { platform: "win32", execPath: "C:\\node.exe", npmExecPath: "C:\\pnpm\\pnpm.cmd" },
]) {
  assert.throws(() => createCheckScriptInvocation("lint", invalid));
}

console.error("[pnpm-cli:selftest] all assertions passed.");
