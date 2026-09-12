import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, extname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = dirname(scriptDir);
const roots = [
  join(repoRoot, ".trellis", "spec"),
  join(repoRoot, "docs"),
  join(repoRoot, "src", "templates", "markdown", "spec"),
];
const markdownLinkPattern = /\[[^\]]+\]\(([^)]+)\)/g;
const missingLinks = [];

function walk(dir) {
  const entries = readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = join(dir, entry.name);
    if (entry.isDirectory()) {
      walk(fullPath);
      continue;
    }
    if (entry.isFile() && extname(entry.name) === ".md") checkFile(fullPath);
  }
}

function checkFile(filePath) {
  const text = readFileSync(filePath, "utf8");
  for (const match of text.matchAll(markdownLinkPattern)) {
    const target = match[1]?.trim();
    if (!target) continue;
    if (
      target.startsWith("#") ||
      target.startsWith("http://") ||
      target.startsWith("https://") ||
      target.startsWith("mailto:")
    ) {
      continue;
    }

    const relativePath = target.split("#", 1)[0];
    const resolvedPath = resolve(dirname(filePath), relativePath);
    try {
      statSync(resolvedPath);
    } catch {
      missingLinks.push({
        file: filePath.replace(`${repoRoot}/`, ""),
        target: relativePath,
      });
    }
  }
}

for (const root of roots) {
  if (!existsSync(root)) continue;
  walk(root);
}

if (missingLinks.length > 0) {
  console.error("Documentation links are broken:");
  for (const item of missingLinks) {
    console.error(`- ${item.file} -> ${item.target}`);
  }
  process.exit(1);
}
