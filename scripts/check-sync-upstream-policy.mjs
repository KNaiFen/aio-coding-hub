import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const modulePath = fileURLToPath(import.meta.url);
const repoRoot = dirname(dirname(modulePath));
const workflowPath = resolve(repoRoot, ".github/workflows/sync-upstream.yml");
const EXPECTED_PREFLIGHT_RUN = `set -euo pipefail
[[ -n "$SYNC_UPSTREAM_APP_ID" ]] || {
  echo "::error::SYNC_UPSTREAM_APP_ID is not configured."
  exit 1
}
[[ "$SYNC_UPSTREAM_APP_ID" =~ ^[0-9]+$ ]] || {
  echo "::error::SYNC_UPSTREAM_APP_ID must be a decimal GitHub App ID."
  exit 1
}
[[ -n "$SYNC_UPSTREAM_APP_PRIVATE_KEY" ]] || {
  echo "::error::SYNC_UPSTREAM_APP_PRIVATE_KEY is not configured."
  exit 1
}`;
const EXPECTED_CONFIGURE_GIT_RUN = `git config user.name "github-actions[bot]"
git config user.email "41898282+github-actions[bot]@users.noreply.github.com"`;
const EXPECTED_FETCH_RUN = `set -euo pipefail

git check-ref-format --branch "\${TARGET_BRANCH}"

git remote add upstream "https://github.com/\${UPSTREAM_REPO}.git"

git fetch origin "+refs/heads/\${TARGET_BRANCH}:refs/remotes/origin/\${TARGET_BRANCH}"
git fetch upstream "+refs/heads/\${TARGET_BRANCH}:refs/remotes/upstream/\${TARGET_BRANCH}"`;
// The full PR script is locked so indirect shell/API calls cannot bypass token policy.
const EXPECTED_OPEN_PR_RUN_SHA256 = "431e4072bee08c5b5ef17daa304bf23f26c7b900b05e8070c53a2e3ed67a4990";
const EXPECTED_SYNC_STEP_NAMES = [
  "Validate GitHub App credentials",
  "Create GitHub App token",
  "Checkout repository",
  "Configure Git",
  "Fetch origin and upstream",
  "Open upstream sync PR",
];

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

function executableLines(source) {
  return source
    .split(/\r?\n/)
    .map((line) => stripYamlComment(line).trim())
    .filter(Boolean);
}

function countMatchingLines(source, pattern) {
  return executableLines(source).filter((line) => pattern.test(line)).length;
}

function indentation(line) {
  return line.length - line.trimStart().length;
}

function scalar(value) {
  const normalized = stripYamlComment(value).trim();
  if (
    (normalized.startsWith('"') && normalized.endsWith('"')) ||
    (normalized.startsWith("'") && normalized.endsWith("'"))
  ) {
    return normalized.slice(1, -1);
  }
  return normalized;
}

function mappingEntry(line, expectedIndent, listItem = false) {
  if (indentation(line) !== expectedIndent) return undefined;
  let content = stripYamlComment(line).slice(expectedIndent);
  if (listItem) {
    const item = /^-\s+/.exec(content);
    if (!item) return undefined;
    content = content.slice(item[0].length);
  }
  const match = /^([A-Za-z0-9_-]+):\s*(.*)$/.exec(content);
  if (!match) return undefined;
  return { key: match[1], value: scalar(match[2]) };
}

function topLevelBlock(source, name) {
  const lines = source.split(/\r?\n/);
  const start = lines.findIndex(
    (line) => indentation(line) === 0 && stripYamlComment(line).trim() === `${name}:`
  );
  if (start === -1) return "";

  let end = start + 1;
  while (
    end < lines.length &&
    (!stripYamlComment(lines[end]).trim() || indentation(lines[end]) > 0)
  ) {
    end += 1;
  }
  return lines.slice(start, end).join("\n");
}

function topLevelKeys(source) {
  const keys = [];
  const malformed = [];
  for (const line of source.split(/\r?\n/)) {
    const visible = stripYamlComment(line);
    if (!visible.trim() || indentation(visible) !== 0) continue;
    const match = /^([A-Za-z0-9_-]+):(?:\s|$)/.exec(visible);
    if (!match) {
      malformed.push(visible.trim());
    } else {
      keys.push(match[1]);
    }
  }
  return { keys, malformed };
}

function workflowJobBody(source, jobName) {
  const lines = source.split(/\r?\n/);
  const jobsStart = lines.findIndex(
    (line) => indentation(line) === 0 && stripYamlComment(line).trim() === "jobs:"
  );
  if (jobsStart === -1) return "";

  let start = -1;
  let end = lines.length;
  for (let index = jobsStart + 1; index < lines.length; index += 1) {
    const match = /^ {2}([A-Za-z0-9_-]+):\s*(?:#.*)?$/.exec(lines[index]);
    if (!match) continue;
    if (start === -1 && match[1] === jobName) {
      start = index + 1;
      continue;
    }
    if (start !== -1) {
      end = index;
      break;
    }
  }
  return start === -1 ? "" : lines.slice(start, end).join("\n");
}

function parseStep(stepLines) {
  const properties = new Map();
  const mappings = new Map();
  const malformed = [];
  const first = mappingEntry(stepLines[0], 6, true);
  if (!first) {
    malformed.push(stripYamlComment(stepLines[0]).trim());
  } else {
    properties.set(first.key, first.value);
  }

  for (let index = 1; index < stepLines.length; index += 1) {
    const visible = stripYamlComment(stepLines[index]);
    if (!visible.trim() || indentation(visible) !== 8) continue;
    const property = mappingEntry(stepLines[index], 8);
    if (!property) {
      malformed.push(visible.trim());
      continue;
    }
    const { key, value } = property;
    if (properties.has(key)) malformed.push(`duplicate ${key}`);
    properties.set(key, value);

    if (value) continue;
    const mapping = new Map();
    for (let child = index + 1; child < stepLines.length; child += 1) {
      const childLine = stepLines[child];
      if (!stripYamlComment(childLine).trim()) continue;
      const childIndent = indentation(childLine);
      if (childIndent <= 8) break;
      const entry = mappingEntry(childLine, 10);
      if (!entry) {
        malformed.push(stripYamlComment(childLine).trim());
      } else {
        if (mapping.has(entry.key)) malformed.push(`duplicate ${key}.${entry.key}`);
        mapping.set(entry.key, entry.value);
      }
    }
    mappings.set(key, mapping);
  }

  return { lines: stepLines, properties, mappings, malformed };
}

function workflowSteps(jobBody) {
  if (!jobBody) return [];
  const lines = jobBody.split(/\r?\n/);
  const stepsStart = lines.findIndex(
    (line) => indentation(line) === 4 && stripYamlComment(line).trim() === "steps:"
  );
  if (stepsStart === -1) return [];

  const steps = [];
  let current = [];
  for (let index = stepsStart + 1; index < lines.length; index += 1) {
    const line = lines[index];
    const visible = stripYamlComment(line);
    if (visible.trim() && indentation(line) <= 4) break;
    if (/^ {6}-\s+/.test(visible)) {
      if (current.length > 0) steps.push(parseStep(current));
      current = [line];
      continue;
    }
    if (current.length > 0) current.push(line);
  }
  if (current.length > 0) steps.push(parseStep(current));
  return steps;
}

function stepNamed(steps, name) {
  return steps.filter((step) => step.properties.get("name") === name);
}

function stepRun(step) {
  for (let index = 0; index < step.lines.length; index += 1) {
    const inline = /^ {6}-\s+run:\s*(.*)$/.exec(step.lines[index]);
    const nested = /^ {8}run:\s*(.*)$/.exec(step.lines[index]);
    const match = inline ?? nested;
    if (!match) continue;

    const run = scalar(match[1]);
    if (!/^[>|]/.test(run)) return run;
    const keyIndent = inline ? 6 : 8;
    const body = [];
    for (let child = index + 1; child < step.lines.length; child += 1) {
      const line = step.lines[child];
      if (!stripYamlComment(line).trim()) {
        body.push("");
        continue;
      }
      if (indentation(line) <= keyIndent) break;
      body.push(stripYamlComment(line).slice(keyIndent + 2));
    }
    return body.join("\n");
  }
  return "";
}

function stepMapping(step, name) {
  return step.mappings.get(name) ?? new Map();
}

function directJobProperties(jobBody) {
  const properties = new Map();
  const malformed = [];
  for (const line of jobBody.split(/\r?\n/)) {
    const visible = stripYamlComment(line);
    if (!visible.trim()) continue;
    const indent = indentation(visible);
    if (indent > 4) continue;
    if (indent < 4) {
      malformed.push(visible.trim());
      continue;
    }
    const entry = mappingEntry(line, 4);
    if (!entry) {
      malformed.push(visible.trim());
      continue;
    }
    if (properties.has(entry.key)) malformed.push(`duplicate ${entry.key}`);
    properties.set(entry.key, entry.value);
  }
  return { malformed, properties };
}

function mapMatches(actual, expected) {
  return (
    actual.size === expected.size &&
    [...expected].every(([key, value]) => actual.get(key) === value)
  );
}

function stepMatchesSchema(step, expectedProperties, expectedMappings = new Map()) {
  if (!step || step.malformed.length > 0 || !mapMatches(step.properties, expectedProperties)) {
    return false;
  }
  if (step.mappings.size !== expectedMappings.size) return false;
  return [...expectedMappings].every(([key, expected]) =>
    mapMatches(stepMapping(step, key), expected)
  );
}

function runDigest(step) {
  return createHash("sha256").update(stepRun(step).trimEnd()).digest("hex");
}

export function validateSyncUpstreamPolicy(source) {
  const failures = [];
  const root = topLevelKeys(source);
  if (
    root.malformed.length > 0 ||
    root.keys.join("\n") !== ["name", "on", "permissions", "concurrency", "env", "jobs"].join("\n")
  ) {
    failures.push("workflow must use only the approved canonical top-level keys");
  }
  const permissions = topLevelBlock(source, "permissions");
  const triggers = executableLines(topLevelBlock(source, "on"));
  const workflowEnv = executableLines(topLevelBlock(source, "env")).join("\n");

  if (!executableLines(permissions).includes("contents: read")) {
    failures.push("top-level contents permission must be read-only");
  }
  const defaultPermissionEntries = executableLines(permissions).filter(
    (line) => line !== "permissions:"
  );
  if (defaultPermissionEntries.length !== 1 || defaultPermissionEntries[0] !== "contents: read") {
    failures.push("the default GITHUB_TOKEN must receive only contents: read");
  }

  const syncJob = workflowJobBody(source, "sync");
  if (!syncJob) failures.push("workflow must define the sync job");
  if (!executableLines(syncJob).includes("timeout-minutes: 10")) {
    failures.push("sync job must retain a 10-minute timeout");
  }
  const syncJobStructure = directJobProperties(syncJob);
  if (
    syncJobStructure.malformed.length > 0 ||
    !mapMatches(
      syncJobStructure.properties,
      new Map([
        ["name", "Sync upstream branch"],
        ["runs-on", "ubuntu-latest"],
        ["timeout-minutes", "10"],
        ["steps", ""],
      ])
    )
  ) {
    failures.push("sync job must use only the approved canonical job properties");
  }

  const steps = workflowSteps(syncJob);
  const namedPreflight = stepNamed(steps, "Validate GitHub App credentials");
  const namedAppToken = stepNamed(steps, "Create GitHub App token");
  const namedCheckout = stepNamed(steps, "Checkout repository");
  const namedConfigureGit = stepNamed(steps, "Configure Git");
  const namedFetch = stepNamed(steps, "Fetch origin and upstream");
  const namedOpenPr = stepNamed(steps, "Open upstream sync PR");
  const preflightStep = namedPreflight.length === 1 ? namedPreflight[0] : undefined;
  const appTokenStep = namedAppToken.length === 1 ? namedAppToken[0] : undefined;
  const checkoutStep = namedCheckout.length === 1 ? namedCheckout[0] : undefined;
  const configureGitStep = namedConfigureGit.length === 1 ? namedConfigureGit[0] : undefined;
  const fetchStep = namedFetch.length === 1 ? namedFetch[0] : undefined;
  const openPrStep = namedOpenPr.length === 1 ? namedOpenPr[0] : undefined;
  const preflightIndex = steps.indexOf(preflightStep);
  const appTokenIndex = steps.indexOf(appTokenStep);
  const checkoutIndex = steps.indexOf(checkoutStep);
  const openPrIndex = steps.indexOf(openPrStep);

  if (
    steps.length !== EXPECTED_SYNC_STEP_NAMES.length ||
    steps.some((step, index) => step.properties.get("name") !== EXPECTED_SYNC_STEP_NAMES[index])
  ) {
    failures.push("sync job must contain only the approved ordered steps");
  }
  const requiredPreflightEnv = new Map([
    ["SYNC_UPSTREAM_APP_ID", "${{ vars.SYNC_UPSTREAM_APP_ID }}"],
    ["SYNC_UPSTREAM_APP_PRIVATE_KEY", "${{ secrets.SYNC_UPSTREAM_APP_PRIVATE_KEY }}"],
  ]);
  const requiredAppTokenInputs = new Map([
    ["app-id", "${{ vars.SYNC_UPSTREAM_APP_ID }}"],
    ["private-key", "${{ secrets.SYNC_UPSTREAM_APP_PRIVATE_KEY }}"],
    ["owner", "${{ github.repository_owner }}"],
    ["repositories", "${{ github.event.repository.name }}"],
    ["permission-contents", "read"],
    ["permission-pull-requests", "write"],
  ]);
  const approvedStepSchemas = [
    [
      preflightStep,
      new Map([
        ["name", "Validate GitHub App credentials"],
        ["shell", "bash"],
        ["env", ""],
        ["run", "|"],
      ]),
      new Map([["env", requiredPreflightEnv]]),
    ],
    [
      appTokenStep,
      new Map([
        ["name", "Create GitHub App token"],
        ["id", "app-token"],
        [
          "uses",
          "actions/create-github-app-token@fee1f7d63c2ff003460e3d139729b119787bc349",
        ],
        ["with", ""],
      ]),
      new Map([["with", requiredAppTokenInputs]]),
    ],
    [
      checkoutStep,
      new Map([
        ["name", "Checkout repository"],
        ["uses", "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1"],
        ["with", ""],
      ]),
      new Map([
        [
          "with",
          new Map([
            ["fetch-depth", "0"],
            ["token", "${{ steps.app-token.outputs.token }}"],
          ]),
        ],
      ]),
    ],
    [
      configureGitStep,
      new Map([
        ["name", "Configure Git"],
        ["run", "|"],
      ]),
    ],
    [
      fetchStep,
      new Map([
        ["name", "Fetch origin and upstream"],
        ["run", "|"],
      ]),
    ],
    [
      openPrStep,
      new Map([
        ["name", "Open upstream sync PR"],
        ["env", ""],
        ["run", "|"],
      ]),
      new Map([
        ["env", new Map([["GH_TOKEN", "${{ steps.app-token.outputs.token }}"]])],
      ]),
    ],
  ];
  if (
    approvedStepSchemas.some(([step, properties, mappings]) =>
      !stepMatchesSchema(step, properties, mappings)
    )
  ) {
    failures.push("sync job steps must retain the approved canonical properties and inputs");
  }

  if (!preflightStep) {
    failures.push("workflow must validate GitHub App credentials exactly once in the sync job");
  }
  if (!appTokenStep) {
    failures.push("workflow must create a GitHub App token exactly once in the sync job");
  }
  if (!checkoutStep) {
    failures.push("workflow must retain repository checkout exactly once in the sync job");
  }
  if (!openPrStep) {
    failures.push("workflow must retain the upstream PR step exactly once in the sync job");
  }
  if (
    !(
      preflightIndex >= 0 &&
      preflightIndex < appTokenIndex &&
      appTokenIndex < checkoutIndex &&
      checkoutIndex < openPrIndex
    )
  ) {
    failures.push("GitHub App validation and token creation must precede all token consumers");
  }

  for (const [label, step] of [
    ["credential validation", preflightStep],
    ["App token creation", appTokenStep],
    ["repository checkout", checkoutStep],
    ["upstream PR", openPrStep],
  ]) {
    if (step?.properties.has("if") || step?.properties.has("continue-on-error")) {
      failures.push(`${label} step must run unconditionally and fail closed`);
    }
  }

  for (const [key, expected] of requiredPreflightEnv) {
    if (!preflightStep || stepMapping(preflightStep, "env").get(key) !== expected) {
      failures.push(`GitHub App credential validation must include ${key}: ${expected}`);
    }
  }
  if (
    !preflightStep ||
    preflightStep.properties.get("shell") !== "bash" ||
    preflightStep.properties.get("run") !== "|" ||
    stepRun(preflightStep).trimEnd() !== EXPECTED_PREFLIGHT_RUN
  ) {
    failures.push("GitHub App credential validation must retain the exact fail-closed Bash script");
  }
  if (!configureGitStep || stepRun(configureGitStep).trimEnd() !== EXPECTED_CONFIGURE_GIT_RUN) {
    failures.push("Git configuration must retain the approved script");
  }
  if (!fetchStep || stepRun(fetchStep).trimEnd() !== EXPECTED_FETCH_RUN) {
    failures.push("upstream fetch must retain the approved fail-closed script");
  }
  if (!openPrStep || runDigest(openPrStep) !== EXPECTED_OPEN_PR_RUN_SHA256) {
    failures.push("upstream PR creation must retain the approved fail-closed script");
  }

  const appTokenSteps = steps.filter((step) =>
    step.properties.get("uses")?.startsWith("actions/create-github-app-token@")
  );
  if (appTokenSteps.length !== 1 || appTokenSteps[0] !== appTokenStep) {
    failures.push("sync job must use exactly one named GitHub App token step");
  }
  const requiredAppTokenScalars = [
    ["id", "app-token"],
    ["uses", "actions/create-github-app-token@fee1f7d63c2ff003460e3d139729b119787bc349"],
  ];
  for (const [key, expected] of requiredAppTokenScalars) {
    if (!appTokenStep || appTokenStep.properties.get(key) !== expected) {
      failures.push(`GitHub App token step must include ${key}: ${expected}`);
    }
  }
  for (const [key, expected] of requiredAppTokenInputs) {
    if (!appTokenStep || stepMapping(appTokenStep, "with").get(key) !== expected) {
      failures.push(`GitHub App token step must include ${key}: ${expected}`);
    }
  }
  const appPermissionEntries = [
    ...stepMapping(appTokenStep ?? { mappings: new Map() }, "with").entries(),
  ]
    .filter(([key]) => key.startsWith("permission-"))
    .map(([key, value]) => `${key}: ${value}`)
    .sort();
  if (
    appPermissionEntries.join("\n") !==
    ["permission-contents: read", "permission-pull-requests: write"].sort().join("\n")
  ) {
    failures.push("GitHub App token must request only contents: read and pull-requests: write");
  }
  if (stepMapping(appTokenStep ?? { mappings: new Map() }, "with").has("skip-token-revoke")) {
    failures.push("GitHub App token must be revoked automatically after the job");
  }
  const checkoutActions = steps.filter((step) =>
    step.properties.get("uses")?.startsWith("actions/checkout@")
  );
  if (checkoutActions.length !== 1 || checkoutActions[0] !== checkoutStep) {
    failures.push("sync job must use exactly one named repository checkout step");
  }
  if (
    !checkoutStep ||
    stepMapping(checkoutStep, "with").get("token") !== "${{ steps.app-token.outputs.token }}"
  ) {
    failures.push("checkout must use the GitHub App token");
  }
  const workflowTokenEntries = steps.flatMap((step) =>
    [...stepMapping(step, "env").entries()]
      .filter(([key]) => key === "GH_TOKEN" || key === "GITHUB_TOKEN")
      .map(([key, value]) => ({ key, step, value }))
  );
  if (
    !openPrStep ||
    openPrStep.properties.get("run") !== "|" ||
    stepMapping(openPrStep, "env").get("GH_TOKEN") !== "${{ steps.app-token.outputs.token }}" ||
    stepMapping(openPrStep, "env").size !== 1 ||
    workflowTokenEntries.length !== 1 ||
    workflowTokenEntries[0].key !== "GH_TOKEN" ||
    workflowTokenEntries[0].step !== openPrStep ||
    workflowTokenEntries[0].value !== "${{ steps.app-token.outputs.token }}"
  ) {
    failures.push("GitHub CLI must use the GitHub App token");
  }
  const appTokenReferenceLines = executableLines(syncJob).filter((line) =>
    line.includes("${{ steps.app-token.outputs.token }}")
  );
  if (
    appTokenReferenceLines.length !== 2 ||
    !appTokenReferenceLines.includes("token: ${{ steps.app-token.outputs.token }}") ||
    !appTokenReferenceLines.includes("GH_TOKEN: ${{ steps.app-token.outputs.token }}")
  ) {
    failures.push("GitHub App token output may be consumed only by checkout and GitHub CLI");
  }
  if (steps.some((step) => stepRun(step).includes("GH_TOKEN"))) {
    failures.push("GitHub CLI token must be declared only through the approved step environment");
  }
  if (executableLines(syncJob).join("\n").includes("github.token")) {
    failures.push("workflow must not fall back to github.token");
  }
  if (executableLines(syncJob).join("\n").includes("SYNC_UPSTREAM_TOKEN")) {
    failures.push("workflow must not retain the legacy PAT secret");
  }
  if (executableLines(syncJob).some((line) => line.startsWith("<<:"))) {
    failures.push("sync job must not use YAML merge keys for security-sensitive configuration");
  }
  const secretReferenceLines = executableLines(syncJob).filter((line) =>
    /\bsecrets(?:\.|\[)/.test(line)
  );
  const expectedSecretReferenceLines = [
    "SYNC_UPSTREAM_APP_PRIVATE_KEY: ${{ secrets.SYNC_UPSTREAM_APP_PRIVATE_KEY }}",
    "private-key: ${{ secrets.SYNC_UPSTREAM_APP_PRIVATE_KEY }}",
  ].sort();
  if (secretReferenceLines.sort().join("\n") !== expectedSecretReferenceLines.join("\n")) {
    failures.push("sync job may reference only the approved GitHub App private-key secret");
  }

  const allCommandText = executableLines(steps.map((step) => stepRun(step)).join("\n"))
    .join("\n")
    .replace(/\\\s*\n/g, " ");
  const commands = executableLines(openPrStep ? stepRun(openPrStep) : "");
  const commandText = commands.join("\n").replace(/\\\s*\n/g, " ");
  const requireCommandText = (text, message) => {
    if (!commandText.includes(text)) failures.push(message);
  };
  const requireAllCommandText = (text, message) => {
    if (!allCommandText.includes(text)) failures.push(message);
  };

  if (/\bgit\b[^\n]*\bpush(?:\s|$)/m.test(allCommandText)) {
    failures.push("workflow must not push a target branch directly");
  }
  if (/\bgit\s+merge(?:\s|$)/m.test(allCommandText)) {
    failures.push("workflow must not merge upstream commits locally");
  }
  if (/\bgh\s+pr\s+merge(?:\s|$)/m.test(allCommandText)) {
    failures.push("workflow must not merge a pull request automatically");
  }
  if (/\bgh\s+pr\s+review(?:\s|$)/m.test(allCommandText)) {
    failures.push("workflow must not approve a pull request automatically");
  }
  if (/\bgh\s+api(?:\s|$)/m.test(allCommandText)) {
    failures.push("workflow must not invoke GitHub API commands, including automatic merge or approval");
  }
  if (
    /\bgh\s+api\b[^\n]*(?:\/pulls\/[^\s]+\/merge(?=["'\s]|$)|mergePullRequest)/m.test(
      allCommandText
    ) ||
    allCommandText.includes("mergePullRequest")
  ) {
    failures.push("workflow must not merge a pull request through the GitHub API");
  }

  if (!commands.some((line) => /^gh\s+pr\s+create(?:\s|$)/.test(line))) {
    failures.push("workflow must retain pull request creation");
  }
  if (!commands.some((line) => /^gh\s+pr\s+edit(?:\s|$)/.test(line))) {
    failures.push("workflow must retain existing pull request updates");
  }
  const existingPrLookup =
    /existing_pr="\$\(\s*gh\s+pr\s+list\s+--repo\s+"\$\{GITHUB_REPOSITORY\}"\s+--head\s+"\$\{UPSTREAM_HEAD\}"\s+--base\s+"\$\{TARGET_BRANCH\}"\s+--state\s+open\s+--json\s+number\s+--jq\s+'\.\[0\]\.number\s+\/\/\s+empty'\s*\)"/.test(
      commandText
    );
  if (countMatchingLines(commandText, /^gh\s+pr\s+list(?:\s|$)/) !== 1 || !existingPrLookup) {
    failures.push(
      "workflow must query an existing sync pull request exactly once with the approved restricted list query"
    );
  }
  if (countMatchingLines(commandText, /^gh\s+pr\s+create(?:\s|$)/) !== 1) {
    failures.push("workflow must create a new sync pull request exactly once");
  }
  if (!/created_pr_url="\$\(\s*gh\s+pr\s+create(?:\s|$)/.test(commandText)) {
    failures.push("workflow must capture gh pr create stdout as the new pull request URL");
  }
  requireCommandText(
    'local created_pr_url_prefix="https://github.com/${GITHUB_REPOSITORY}/pull/"',
    "workflow must bind new pull request URLs to the current GitHub repository"
  );
  requireCommandText(
    'if [[ "${created_pr_url}" != "${created_pr_url_prefix}"* ]]; then',
    "workflow must fail closed when the new pull request URL is outside the current repository"
  );
  requireCommandText(
    'pr_number="${created_pr_url#"$created_pr_url_prefix"}"',
    "workflow must extract the new pull request number directly from gh pr create output"
  );
  requireCommandText(
    'if [[ ! "${pr_number}" =~ ^[1-9][0-9]*$ ]]; then',
    "workflow must fail closed when the new pull request number is not a non-zero positive integer"
  );
  requireCommandText(
    "Failed to resolve sync PR number from gh pr create output. Manual handling required.",
    "workflow must report malformed gh pr create output as requiring manual handling"
  );
  requireCommandText('--base "${TARGET_BRANCH}"', "sync pull requests must target TARGET_BRANCH");
  requireCommandText('--head "${UPSTREAM_HEAD}"', "sync pull requests must use the upstream head");
  requireCommandText("Please review and merge manually.", "pull request body must require manual review");
  requireCommandText(
    "Manual review and merge required.",
    "step summary must state that manual review and merge are required"
  );
  if (!triggers.includes("schedule:")) failures.push("workflow must retain the scheduled trigger");
  if (!triggers.includes('- cron: "0 0 * * *"')) {
    failures.push("workflow must retain the daily sync schedule");
  }
  if (!triggers.includes("workflow_dispatch:")) failures.push("workflow must retain the manual trigger");
  if (!workflowEnv.includes("UPSTREAM_REPO: dyndynjyxa/aio-coding-hub")) {
    failures.push("workflow must retain the configured upstream repository");
  }
  if (!workflowEnv.includes("TARGET_BRANCH: ${{ github.event.inputs.target_branch || 'main' }}")) {
    failures.push("workflow must retain the configured target branch");
  }
  requireAllCommandText(
    'git fetch origin "+refs/heads/${TARGET_BRANCH}:refs/remotes/origin/${TARGET_BRANCH}"',
    "workflow must fetch the target branch from origin"
  );
  requireAllCommandText(
    'git fetch upstream "+refs/heads/${TARGET_BRANCH}:refs/remotes/upstream/${TARGET_BRANCH}"',
    "workflow must fetch the target branch from upstream"
  );
  requireCommandText("--json mergeStateStatus", "workflow must inspect the open PR merge state once");
  requireCommandText(
    '[ -z "${merge_state}" ]',
    "workflow must fail closed when the open PR merge state is unavailable"
  );
  requireCommandText(
    '[ "${merge_state}" = "DIRTY" ]',
    "workflow must fail closed when the open PR has conflicts"
  );
  requireCommandText(
    '[ "${merge_state}" = "UNKNOWN" ]',
    "workflow must fail closed when the open PR merge state is unknown"
  );
  if (commandText.includes('[ "${merge_state}" = "BLOCKED" ]')) {
    failures.push("workflow must leave review-blocked pull requests open for manual review");
  }
  requireCommandText(
    "Manual conflict resolution required",
    "conflicted sync pull requests must report manual resolution"
  );

  const noOpMarker = 'if git merge-base --is-ancestor "${UPSTREAM}" "${LOCAL}"; then';
  const fastForwardMarker = 'if git merge-base --is-ancestor "${LOCAL}" "${UPSTREAM}"; then';
  const divergedMarker =
    'echo "Branches have diverged. Creating cross-repository sync PR instead."';
  requireCommandText(noOpMarker, "workflow must retain the already-synchronized no-op branch");
  requireCommandText("Already up to date. Nothing to sync.", "no-op branch must remain explicit");
  requireCommandText(fastForwardMarker, "workflow must distinguish the fast-forward topology");
  requireCommandText(divergedMarker, "workflow must retain the diverged topology path");

  const fastForwardStart = commandText.indexOf(fastForwardMarker);
  const divergedStart = commandText.indexOf(divergedMarker);
  if (fastForwardStart !== -1 && divergedStart > fastForwardStart) {
    const fastForwardBlock = commandText.slice(fastForwardStart, divergedStart);
    if (countMatchingLines(fastForwardBlock, /^if(?:\s|$)/) !== 1) {
      failures.push("fast-forward topology must not conditionally bypass pull request creation");
    }
    if (countMatchingLines(fastForwardBlock, /^create_or_update_upstream_pr(?:\s|$)/) !== 1) {
      failures.push("fast-forward topology must create or update exactly one pull request");
    }
  }

  if (divergedStart !== -1) {
    const divergedBlock = commandText.slice(divergedStart);
    if (countMatchingLines(divergedBlock, /^create_or_update_upstream_pr(?:\s|$)/) !== 1) {
      failures.push("diverged topology must create or update exactly one pull request");
    }
  }

  return failures;
}

export function assertSyncUpstreamPolicy(source) {
  const failures = validateSyncUpstreamPolicy(source);
  if (failures.length > 0) {
    throw new Error(`Sync upstream policy check failed:\n- ${failures.join("\n- ")}`);
  }
}

if (process.argv[1] && resolve(process.argv[1]) === modulePath) {
  assertSyncUpstreamPolicy(readFileSync(workflowPath, "utf8"));
  console.log("Sync upstream manual-review policy check passed.");
}
