import { spawnSync } from "node:child_process";
import { existsSync, readFileSync, statSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = dirname(scriptDir);
const markdownLinkPattern = /!?\[[^\]]*\]\(([^)\n]+)\)/g;

function trackedAndUnignoredMarkdownFiles() {
  const result = spawnSync("git", ["ls-files", "--cached", "--others", "--exclude-standard"], {
    cwd: repoRoot,
    encoding: "utf8",
  });
  if (result.status !== 0) {
    throw new Error(result.stderr.trim() || "git ls-files failed");
  }
  return result.stdout
    .split(/\r?\n/)
    .filter((path) => path.endsWith(".md"))
    .map((path) => resolve(repoRoot, path))
    .filter((path) => existsSync(path));
}

function normalizedTarget(rawTarget) {
  const trimmed = rawTarget.trim();
  const bracketed = trimmed.match(/^<([^>]+)>/);
  const target = bracketed ? bracketed[1] : trimmed.split(/\s+/, 1)[0];
  try {
    return decodeURI(target);
  } catch {
    return target;
  }
}

function isExternalTarget(target) {
  return /^(?:[a-z][a-z0-9+.-]*:|\/\/)/i.test(target);
}

function githubAnchor(text) {
  return text
    .replace(/<[^>]*>/g, "")
    .replace(/`([^`]*)`/g, "$1")
    .trim()
    .toLowerCase()
    .replace(/[^\p{L}\p{N}\s-]/gu, "")
    .replace(/[\s-]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

const anchorCache = new Map();

function anchorsFor(markdownPath) {
  if (anchorCache.has(markdownPath)) return anchorCache.get(markdownPath);
  const counts = new Map();
  const anchors = new Set();
  const lines = readFileSync(markdownPath, "utf8").split(/\r?\n/);
  for (const line of lines) {
    const heading = line.match(/^#{1,6}\s+(.+?)\s*#*\s*$/);
    if (!heading) continue;
    const base = githubAnchor(heading[1]);
    if (!base) continue;
    const count = counts.get(base) ?? 0;
    counts.set(base, count + 1);
    anchors.add(count === 0 ? base : `${base}-${count}`);
  }
  anchorCache.set(markdownPath, anchors);
  return anchors;
}

function lineNumber(text, offset) {
  return text.slice(0, offset).split("\n").length;
}

const failures = [];

for (const markdownPath of trackedAndUnignoredMarkdownFiles()) {
  const text = readFileSync(markdownPath, "utf8");
  for (const match of text.matchAll(markdownLinkPattern)) {
    const target = normalizedTarget(match[1] ?? "");
    if (!target || target.startsWith("#") || isExternalTarget(target)) continue;

    const [pathPart, rawFragment] = target.split("#", 2);
    const targetPath = resolve(dirname(markdownPath), pathPart || ".");
    const displayPath = markdownPath.replace(`${repoRoot}/`, "");
    const line = lineNumber(text, match.index ?? 0);
    try {
      statSync(targetPath);
    } catch {
      failures.push(`${displayPath}:${line}: missing ${pathPart || "."}`);
      continue;
    }

    if (!rawFragment || !targetPath.endsWith(".md") || !existsSync(targetPath)) continue;
    const fragment = githubAnchor(rawFragment);
    if (!anchorsFor(targetPath).has(fragment)) {
      failures.push(`${displayPath}:${line}: missing anchor #${rawFragment} in ${pathPart}`);
    }
  }
}

if (failures.length > 0) {
  console.error("Markdown links are broken:");
  for (const failure of failures) console.error(`- ${failure}`);
  process.exit(1);
}
