import { execFileSync } from "node:child_process";

export function readRepositoryPaths(repoRoot) {
  return execFileSync("git", ["ls-files", "--cached", "--others", "--exclude-standard", "-z"], {
    cwd: repoRoot,
    encoding: "utf8",
    maxBuffer: 16 * 1024 * 1024,
    shell: false,
  });
}

export function readScopedHooksConfig(repoRoot) {
  return execFileSync(
    "git",
    ["config", "--show-scope", "--show-origin", "--get-all", "core.hooksPath"],
    { cwd: repoRoot, encoding: "utf8", shell: false }
  );
}
