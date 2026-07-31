import { isAbsolute as isPosixAbsolute, basename as posixBasename } from "node:path/posix";
import { isAbsolute as isWindowsAbsolute, basename as windowsBasename } from "node:path/win32";

export function createPnpmInvocation(
  args,
  {
    platform = process.platform,
    execPath = process.execPath,
    npmExecPath = process.env.npm_execpath,
  } = {}
) {
  if (!Array.isArray(args) || args.some((arg) => typeof arg !== "string" || arg.includes("\0"))) {
    throw new Error("pnpm arguments must be NUL-free strings.");
  }

  const windows = platform === "win32";
  const isAbsolute = windows ? isWindowsAbsolute : isPosixAbsolute;
  const basename = windows ? windowsBasename : posixBasename;
  if (typeof execPath !== "string" || !isAbsolute(execPath)) {
    throw new Error("Node executable path must be absolute.");
  }
  if (
    typeof npmExecPath !== "string" ||
    !isAbsolute(npmExecPath) ||
    !/^pnpm\.(?:c?js|mjs)$/i.test(basename(npmExecPath))
  ) {
    throw new Error("Run this check through pnpm so its JavaScript CLI path is available.");
  }

  return {
    command: execPath,
    args: [npmExecPath, ...args],
  };
}
