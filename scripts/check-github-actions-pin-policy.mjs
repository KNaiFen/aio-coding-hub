import { readdirSync, readFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const modulePath = fileURLToPath(import.meta.url);
const repoRoot = dirname(dirname(modulePath));
const REMOTE_ACTION = /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+(?:\/[^@\s]+)?@[0-9a-fA-F]{40}$/;
const DOCKER_ACTION = /^docker:\/\/[^\s@]+@sha256:[0-9a-fA-F]{64}$/;

function stripYamlComment(line) {
  let singleQuoted = false;
  let doubleQuoted = false;
  let escaped = false;

  for (let index = 0; index < line.length; index += 1) {
    const character = line[index];
    if (doubleQuoted) {
      if (escaped) {
        escaped = false;
      } else if (character === "\\") {
        escaped = true;
      } else if (character === '"') {
        doubleQuoted = false;
      }
      continue;
    }
    if (singleQuoted) {
      if (character === "'" && line[index + 1] === "'") {
        index += 1;
      } else if (character === "'") {
        singleQuoted = false;
      }
      continue;
    }
    if (character === '"') {
      doubleQuoted = true;
      continue;
    }
    if (character === "'") {
      singleQuoted = true;
      continue;
    }
    if (character === "#" && (index === 0 || /\s/.test(line[index - 1]))) {
      return line.slice(0, index).trimEnd();
    }
  }
  return line.trimEnd();
}

function indentation(line) {
  return line.length - line.trimStart().length;
}

function collectYamlFiles(root, directory, mode) {
  const absoluteDirectory = join(root, directory);
  const files = [];

  function visit(current) {
    let entries;
    try {
      entries = readdirSync(current, { withFileTypes: true });
    } catch (error) {
      if (error?.code === "ENOENT") return;
      throw error;
    }

    for (const entry of entries) {
      const path = join(current, entry.name);
      if (entry.isDirectory()) {
        visit(path);
        continue;
      }
      if (!entry.isFile()) continue;
      if (mode === "workflows" && /\.ya?ml$/i.test(entry.name)) files.push(path);
      if (mode === "actions" && /^action\.ya?ml$/i.test(entry.name)) files.push(path);
    }
  }

  visit(absoluteDirectory);
  return files.sort();
}

function normalizeUsesValue(raw) {
  const withoutComment = stripYamlComment(raw).trim();
  if (
    (withoutComment.startsWith('"') && withoutComment.endsWith('"')) ||
    (withoutComment.startsWith("'") && withoutComment.endsWith("'"))
  ) {
    return withoutComment.slice(1, -1);
  }
  return withoutComment;
}

function splitFlowMapping(source) {
  const entries = [];
  let start = 0;
  let depth = 0;
  let singleQuoted = false;
  let doubleQuoted = false;
  let escaped = false;

  for (let index = 0; index < source.length; index += 1) {
    const character = source[index];
    if (doubleQuoted) {
      if (escaped) {
        escaped = false;
      } else if (character === "\\") {
        escaped = true;
      } else if (character === '"') {
        doubleQuoted = false;
      }
      continue;
    }
    if (singleQuoted) {
      if (character === "'" && source[index + 1] === "'") {
        index += 1;
      } else if (character === "'") {
        singleQuoted = false;
      }
      continue;
    }
    if (character === '"') {
      doubleQuoted = true;
      continue;
    }
    if (character === "'") {
      singleQuoted = true;
      continue;
    }
    if (character === "{" || character === "[") {
      depth += 1;
      continue;
    }
    if (character === "}" || character === "]") {
      depth -= 1;
      continue;
    }
    if (character === "," && depth === 0) {
      entries.push(source.slice(start, index));
      start = index + 1;
    }
  }
  entries.push(source.slice(start));
  return entries;
}

function flowMappingValue(source, key) {
  const body = source.slice(1, -1);
  for (const entry of splitFlowMapping(body)) {
    const separator = entry.indexOf(":");
    if (separator === -1) continue;
    const entryKey = normalizeUsesValue(entry.slice(0, separator));
    if (entryKey === key) return normalizeUsesValue(entry.slice(separator + 1));
  }
  return undefined;
}

function yamlStructureLines(source) {
  const lines = [];
  let blockScalarIndent;

  for (const [index, raw] of source.split(/\r?\n/).entries()) {
    const visible = stripYamlComment(raw);
    if (!visible.trim()) continue;
    const indent = indentation(visible);
    if (blockScalarIndent !== undefined) {
      if (indent > blockScalarIndent) continue;
      blockScalarIndent = undefined;
    }

    const content = visible.trimStart();
    lines.push({ content, indent, line: index + 1, visible });
    if (/^(?:-\s+)?(?:[A-Za-z0-9_-]+|["'][^"']+["']):\s*[>|]/.test(content)) {
      blockScalarIndent = indent;
    }
  }
  return lines;
}

function canonicalStructureFailures(source) {
  const failures = [];
  const lines = yamlStructureLines(source);
  const push = (line, message) => failures.push({ line, message });
  const topLevelKeys = new Map();

  for (const entry of lines) {
    if (/^(?:-\s+)?[A-Za-z0-9_-]+\s+:/.test(entry.content)) {
      push(entry.line, "mapping keys must not contain whitespace before the colon");
    }
    if (/^(?:-\s+)?["'][^"']+["']\s*:/.test(entry.content)) {
      push(entry.line, "quoted block mapping keys are not supported by the pin policy");
    }
    if (/^(?:-\s+)?<<\s*:/.test(entry.content)) {
      push(entry.line, "YAML merge keys are not supported by the pin policy");
    }
    if (/(?:^|\s)[&*][A-Za-z0-9_-]+(?:\s|$)/.test(entry.content)) {
      push(entry.line, "YAML anchors and aliases are not supported by the pin policy");
    }
    if (/^(?:-\s+)?(?:\?|!\S*)\s/.test(entry.content)) {
      push(entry.line, "complex or tagged YAML mapping keys are not supported by the pin policy");
    }
    if (entry.indent <= 2 && /^runs:\s*\S/.test(entry.content)) {
      push(entry.line, "runs must use block mapping syntax so composite actions can be audited");
    }
    if (/^steps:\s*/.test(entry.content) && entry.indent !== 2 && entry.indent !== 4) {
      push(entry.line, "steps must use the repository's canonical two-space indentation");
    }
    const topLevelKey = entry.indent === 0 ? /^([A-Za-z0-9_-]+):/.exec(entry.content)?.[1] : undefined;
    if (topLevelKey) {
      if (topLevelKeys.has(topLevelKey)) {
        push(entry.line, `duplicate top-level key ${topLevelKey} is not supported`);
      } else {
        topLevelKeys.set(topLevelKey, entry.line);
      }
    }
  }

  const jobsIndex = lines.findIndex(
    (entry) => entry.indent === 0 && /^jobs:\s*$/.test(entry.content)
  );
  if (jobsIndex !== -1) {
    let jobsEnd = lines.length;
    for (let index = jobsIndex + 1; index < lines.length; index += 1) {
      if (lines[index].indent === 0) {
        jobsEnd = index;
        break;
      }
    }
    const jobsBody = lines.slice(jobsIndex + 1, jobsEnd);
    if (jobsBody.length > 0 && jobsBody[0].indent !== 2) {
      push(jobsBody[0].line, "jobs must use the repository's canonical two-space indentation");
    }

    const jobIndexes = [];
    for (let index = 0; index < jobsBody.length; index += 1) {
      if (jobsBody[index].indent !== 2) continue;
      if (!/^[A-Za-z0-9_-]+:\s*$/.test(jobsBody[index].content)) {
        push(jobsBody[index].line, "job ids must use unquoted block mapping syntax");
        continue;
      }
      jobIndexes.push(index);
    }
    for (const [position, jobIndex] of jobIndexes.entries()) {
      const nextJob = jobIndexes[position + 1] ?? jobsBody.length;
      const body = jobsBody.slice(jobIndex + 1, nextJob);
      if (body.length > 0 && body[0].indent !== 4) {
        push(body[0].line, "job properties must use the repository's canonical two-space indentation");
      }
    }
  }

  for (const [index, entry] of lines.entries()) {
    if (!/^steps:\s*$/.test(entry.content)) continue;
    const child = lines[index + 1];
    if (!child || child.indent <= entry.indent) continue;
    if (child.indent !== entry.indent + 2 || !child.content.startsWith("-")) {
      push(child.line, "step entries must use the repository's canonical two-space indentation");
    }
  }
  return failures;
}

function workflowUses(source) {
  const uses = [];
  const structuralFailures = canonicalStructureFailures(source);
  const lines = source.split(/\r?\n/);
  let inJobs = false;
  let inJob = false;
  let stepsIndent;

  for (let index = 0; index < lines.length; index += 1) {
    const visible = stripYamlComment(lines[index]);
    if (!visible.trim()) continue;
    const indent = indentation(visible);

    if (indent === 0) {
      const jobs = /^jobs:\s*(.*)$/.exec(visible);
      if (jobs) {
        inJobs = true;
        inJob = false;
        if (jobs[1].trim()) {
          structuralFailures.push({
            line: index + 1,
            message: "jobs must use block mapping syntax so action references can be audited",
          });
        }
      } else if (inJobs) {
        inJobs = false;
        inJob = false;
      }
    } else if (inJobs && indent === 2) {
      const job = /^ {2}[A-Za-z0-9_-]+:\s*(.*)$/.exec(visible);
      if (job) {
        inJob = !job[1].trim();
        if (!inJob) {
          structuralFailures.push({
            line: index + 1,
            message: "jobs must use block mapping syntax so reusable workflows can be audited",
          });
        }
      }
    } else if (inJobs && inJob && indent === 4) {
      const reusable = /^ {4}(?:uses|["']uses["']):\s*(.+)$/.exec(visible);
      if (reusable) {
        uses.push({ line: index + 1, value: normalizeUsesValue(reusable[1]) });
      }
    }

    if (stepsIndent !== undefined && indent <= stepsIndent) stepsIndent = undefined;
    const steps = /^(\s*)steps:\s*(.*)$/.exec(visible);
    if (steps && (steps[1].length === 2 || steps[1].length === 4)) {
      stepsIndent = steps[1].length;
      if (steps[2].trim()) {
        structuralFailures.push({
          line: index + 1,
          message: "steps must use block sequence syntax so action references can be audited",
        });
        stepsIndent = undefined;
      }
      continue;
    }
    if (stepsIndent === undefined) continue;

    const stepIndent = stepsIndent + 2;
    if (indent === stepIndent) {
      const step = visible.slice(stepIndent);
      const direct = /^-\s+(?:uses|["']uses["']):\s*(.+)$/.exec(step);
      if (direct) {
        uses.push({ line: index + 1, value: normalizeUsesValue(direct[1]) });
        continue;
      }

      const flow = /^-\s*(\{.*\})\s*$/.exec(step);
      if (flow) {
        const value = flowMappingValue(flow[1], "uses");
        if (value !== undefined) uses.push({ line: index + 1, value });
        continue;
      }
      if (/^-\s*\{/.test(step)) {
        structuralFailures.push({
          line: index + 1,
          message: "multi-line flow step mappings are not supported by the pin policy",
        });
      }
      continue;
    }

    if (indent === stepIndent + 2) {
      const direct = new RegExp(
        `^ {${stepIndent + 2}}(?:uses|["']uses["']):\\s*(.+)$`
      ).exec(visible);
      if (direct) uses.push({ line: index + 1, value: normalizeUsesValue(direct[1]) });
    }
  }
  return { structuralFailures, uses };
}

function workflowJobs(source) {
  const lines = source.split(/\r?\n/);
  const jobsStart = lines.findIndex(
    (line) => indentation(line) === 0 && stripYamlComment(line).trim() === "jobs:"
  );
  if (jobsStart === -1) return [];

  let jobsEnd = lines.length;
  for (let index = jobsStart + 1; index < lines.length; index += 1) {
    if (stripYamlComment(lines[index]).trim() && indentation(lines[index]) === 0) {
      jobsEnd = index;
      break;
    }
  }
  const jobsLines = lines.slice(jobsStart + 1, jobsEnd);
  const matches = [];
  for (const [index, line] of jobsLines.entries()) {
    const match = /^ {2}([A-Za-z0-9_-]+):\s*$/.exec(stripYamlComment(line));
    if (match) matches.push({ index, name: match[1] });
  }
  return matches.map((match, index) => ({
    name: match.name,
    body: jobsLines.slice(match.index + 1, matches[index + 1]?.index ?? jobsLines.length).join("\n"),
  }));
}

export function validateGithubActionsPinPolicy(sources) {
  const failures = [];
  for (const [path, source] of Object.entries(sources).sort(([left], [right]) =>
    left.localeCompare(right)
  )) {
    const { structuralFailures, uses } = workflowUses(source);
    for (const failure of structuralFailures) {
      failures.push(`${path}:${failure.line} ${failure.message}`);
    }
    for (const action of uses) {
      if (action.value.startsWith("./")) continue;
      if (action.value.startsWith("docker://")) {
        if (!DOCKER_ACTION.test(action.value)) {
          failures.push(`${path}:${action.line} Docker action must use a sha256 digest`);
        }
        continue;
      }
      if (!REMOTE_ACTION.test(action.value)) {
        failures.push(`${path}:${action.line} remote action must use a full 40-character commit SHA`);
      }
    }
  }
  return failures;
}

export function assertGithubActionsPinPolicy(sources) {
  const failures = validateGithubActionsPinPolicy(sources);
  if (failures.length > 0) {
    throw new Error(`GitHub Actions pin policy check failed:\n- ${failures.join("\n- ")}`);
  }
}

export function validateGithubActionsTimeoutPolicy(sources) {
  const failures = [];
  for (const [path, source] of Object.entries(sources).sort(([left], [right]) =>
    left.localeCompare(right)
  )) {
    if (!/^\.github\/workflows\/.*\.ya?ml$/i.test(path)) continue;
    for (const failure of canonicalStructureFailures(source)) {
      failures.push(`${path}:${failure.line} ${failure.message}`);
    }
    const jobs = workflowJobs(source);
    if (jobs.length === 0) {
      failures.push(`${path} must define at least one job`);
      continue;
    }
    for (const job of jobs) {
      if (/^    uses:\s*/m.test(job.body)) continue;
      if (!/^    runs-on:\s*\S+/m.test(job.body)) {
        failures.push(`${path} job ${job.name} must define runs-on or a reusable workflow`);
        continue;
      }
      if (!/^    timeout-minutes:\s*[1-9][0-9]*\s*(?:#.*)?$/m.test(job.body)) {
        failures.push(`${path} job ${job.name} must define a positive timeout-minutes`);
      }
    }
  }
  return failures;
}

export function assertGithubActionsTimeoutPolicy(sources) {
  const failures = validateGithubActionsTimeoutPolicy(sources);
  if (failures.length > 0) {
    throw new Error(`GitHub Actions timeout policy check failed:\n- ${failures.join("\n- ")}`);
  }
}

export function loadGithubActionsPinPolicySources(root = repoRoot) {
  const paths = [
    ...collectYamlFiles(root, ".github/workflows", "workflows"),
    ...collectYamlFiles(root, ".github/actions", "actions"),
  ];
  return Object.fromEntries(
    paths.map((path) => [relative(root, path).replaceAll("\\", "/"), readFileSync(path, "utf8")])
  );
}

if (process.argv[1] && resolve(process.argv[1]) === modulePath) {
  const sources = loadGithubActionsPinPolicySources();
  assertGithubActionsPinPolicy(sources);
  assertGithubActionsTimeoutPolicy(sources);
  console.log("GitHub Actions pin and timeout policy check passed.");
}
