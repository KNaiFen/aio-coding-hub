import assert from "node:assert/strict";

import { createPnpmInvocation } from "./lib/pnpm-cli.mjs";

assert.deepEqual(
  createPnpmInvocation(["lint"], {
    platform: "linux",
    execPath: "/usr/bin/node",
    npmExecPath: "/opt/pnpm/bin/pnpm.cjs",
  }),
  {
    command: "/usr/bin/node",
    args: ["/opt/pnpm/bin/pnpm.cjs", "lint"],
  }
);

assert.deepEqual(
  createPnpmInvocation(["test:unit"], {
    platform: "win32",
    execPath: "C:\\Program Files\\nodejs\\node.exe",
    npmExecPath: "C:\\pnpm\\pnpm.cjs",
  }),
  {
    command: "C:\\Program Files\\nodejs\\node.exe",
    args: ["C:\\pnpm\\pnpm.cjs", "test:unit"],
  }
);

for (const invalid of [
  { platform: "linux", execPath: "node", npmExecPath: "/opt/pnpm/bin/pnpm.cjs" },
  { platform: "linux", execPath: "/usr/bin/node", npmExecPath: "pnpm.cjs" },
  { platform: "linux", execPath: "/usr/bin/node", npmExecPath: "/usr/local/bin/pnpm" },
  { platform: "win32", execPath: "C:\\node.exe", npmExecPath: "C:\\pnpm\\pnpm.cmd" },
]) {
  assert.throws(() => createPnpmInvocation([], invalid));
}

assert.throws(() =>
  createPnpmInvocation(["bad\0argument"], {
    platform: "linux",
    execPath: "/usr/bin/node",
    npmExecPath: "/opt/pnpm/bin/pnpm.cjs",
  })
);

console.error("[pnpm-cli:selftest] all assertions passed.");
