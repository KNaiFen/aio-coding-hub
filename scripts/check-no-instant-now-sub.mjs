import fs from "node:fs";
import path from "node:path";

const ROOT = process.cwd();
const TARGET_DIR = path.join(ROOT, "src-tauri", "src");
const NEEDLE = "Instant::now()";

function isRustSource(filePath) {
  return filePath.endsWith(".rs");
}

function walk(dirPath, out) {
  const entries = fs.readdirSync(dirPath, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(dirPath, entry.name);
    if (entry.isDirectory()) {
      walk(fullPath, out);
      continue;
    }
    if (entry.isFile() && isRustSource(fullPath)) {
      out.push(fullPath);
    }
  }
}

function skipRustTrivia(text, start) {
  let cursor = start;
  while (cursor < text.length) {
    if (/\s/.test(text[cursor])) {
      cursor += 1;
      continue;
    }
    if (text.startsWith("//", cursor)) {
      const newline = text.indexOf("\n", cursor + 2);
      cursor = newline === -1 ? text.length : newline + 1;
      continue;
    }
    if (text.startsWith("/*", cursor)) {
      let depth = 1;
      cursor += 2;
      while (cursor < text.length && depth > 0) {
        if (text.startsWith("/*", cursor)) {
          depth += 1;
          cursor += 2;
        } else if (text.startsWith("*/", cursor)) {
          depth -= 1;
          cursor += 2;
        } else {
          cursor += 1;
        }
      }
      continue;
    }
    break;
  }
  return cursor;
}

function sourceLine(text, offset) {
  const lineStart = text.lastIndexOf("\n", offset - 1) + 1;
  const lineEnd = text.indexOf("\n", offset);
  return {
    lineNumber: text.slice(0, lineStart).split("\n").length,
    line: text.slice(lineStart, lineEnd === -1 ? text.length : lineEnd),
  };
}

function findViolations(filePath) {
  const text = fs.readFileSync(filePath, "utf8");
  const hits = [];
  let searchFrom = 0;
  while (searchFrom < text.length) {
    const matchStart = text.indexOf(NEEDLE, searchFrom);
    if (matchStart === -1) break;
    const operator = skipRustTrivia(text, matchStart + NEEDLE.length);
    if (text[operator] === "-") {
      hits.push(sourceLine(text, matchStart));
    }
    searchFrom = matchStart + NEEDLE.length;
  }
  return hits;
}

if (!fs.existsSync(TARGET_DIR)) {
  console.error(`Expected directory not found: ${TARGET_DIR}`);
  process.exit(2);
}

const files = [];
walk(TARGET_DIR, files);

const violations = [];
for (const filePath of files) {
  const hits = findViolations(filePath);
  if (hits.length === 0) continue;
  violations.push({ filePath, hits });
}

if (violations.length === 0) {
  process.exit(0);
}

console.error(
  "Forbidden pattern detected: `Instant::now() - <Duration>` (can panic on underflow).\n" +
    "Use `Instant::checked_sub(...)` or `saturating_duration_since(...)` style patterns instead.\n"
);

for (const v of violations) {
  const rel = path.relative(ROOT, v.filePath);
  for (const hit of v.hits) {
    console.error(`${rel}:${hit.lineNumber}: ${hit.line.trim()}`);
  }
}

process.exit(1);
