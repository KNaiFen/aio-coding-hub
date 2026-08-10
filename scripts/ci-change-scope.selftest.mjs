import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";

import {
  classifyNameStatus,
  classifyPath,
  classifyPaths,
  collectChangedPaths,
  loadPolicy,
  parseNameStatus,
  runClassifier,
  shouldRunProviderTrendBenchmark,
  validatePolicy,
} from "./ci-change-scope.mjs";

const policyPath = fileURLToPath(new URL("../.github/ci-scope.json", import.meta.url));
const policy = loadPolicy(policyPath);
const sha = (character) => character.repeat(40);

function expectScope(paths, expected) {
  const result = classifyPaths(paths, policy);
  assert.equal(result.scope, expected.scope);
  assert.equal(result.fullCi, expected.fullCi);
  assert.equal(result.docsChecks, expected.docsChecks);
  assert.equal(result.providerTrendBenchmark, expected.providerTrendBenchmark ?? false);
}

expectScope(["PENDING.md", ".trellis/tasks/08-03-task/task.json", "omx_wiki/guide.md"], {
  scope: "process-docs",
  fullCi: false,
  docsChecks: false,
});
expectScope(["README.md", "docs/plugins/authoring.md", ".trellis/spec/example/rule.md"], {
  scope: "checked-docs",
  fullCi: false,
  docsChecks: true,
});
expectScope(["PENDING.md", "docs/plugins/authoring.md"], {
  scope: "checked-docs",
  fullCi: false,
  docsChecks: true,
});
expectScope(["src/main.tsx", "docs/plugins/authoring.md"], {
  scope: "full",
  fullCi: true,
  docsChecks: true,
});

for (const path of [
  ".github/workflows/ci.yml",
  ".github/ci-scope.json",
  ".trellis/config.yaml",
  ".trellis/scripts/task.py",
  "docs/plugins/plugin-api-v1-contract.json",
  "docs/image.png",
  "package.json",
  "pnpm-lock.yaml",
  "scripts/check-spec-links.mjs",
  "src/main.tsx",
]) {
  assert.equal(classifyPath(path, policy).tier, "full", path);
}
for (const path of ["../README.md", "/README.md", "docs\\guide.md", "docs//guide.md"]) {
  assert.equal(classifyPath(path, policy).tier, "full", path);
}

const permissivePolicy = structuredClone(policy);
permissivePolicy.processDocumentation.exactPaths.push(
  ".github/ci-scope.json",
  "scripts/ci-change-scope.mjs"
);
validatePolicy(permissivePolicy);
assert.equal(classifyPath(".github/ci-scope.json", permissivePolicy).reason, "ci-control-plane");
assert.equal(classifyPath(".github/workflows/ci.yml", permissivePolicy).reason, "ci-control-plane");
assert.equal(
  classifyPath("scripts/ci-change-scope.mjs", permissivePolicy).reason,
  "ci-control-plane"
);

const ambiguousPolicy = structuredClone(policy);
ambiguousPolicy.checkedDocumentation.exactPaths.push("PENDING.md");
validatePolicy(ambiguousPolicy);
assert.equal(classifyPath("PENDING.md", ambiguousPolicy).reason, "ambiguous-policy");

expectScope([], {
  scope: "full",
  fullCi: true,
  docsChecks: false,
  providerTrendBenchmark: true,
});

expectScope(["src-tauri/src/domain/usage_stats/trend_common.rs"], {
  scope: "full",
  fullCi: true,
  docsChecks: false,
  providerTrendBenchmark: true,
});
assert.equal(
  shouldRunProviderTrendBenchmark(["src-tauri/src/infra/usage_provider_daily_rollup.rs"]),
  true
);
assert.equal(shouldRunProviderTrendBenchmark(["src/main.tsx", "README.md"]), false);

const parsed = parseNameStatus(
  "M\0PENDING.md\0D\0docs/old.md\0R100\0PENDING.md\0src/pending.ts\0C090\0README.md\0docs/copy.md\0"
);
assert.deepEqual(parsed, [
  { status: "M", paths: ["PENDING.md"] },
  { status: "D", paths: ["docs/old.md"] },
  { status: "R100", paths: ["PENDING.md", "src/pending.ts"] },
  { status: "C090", paths: ["README.md", "docs/copy.md"] },
]);
assert.throws(() => parseNameStatus("M\0README.md"), /NUL terminated/);
assert.throws(() => parseNameStatus("R101\0README.md\0docs/readme.md\0"), /invalid R101/);
assert.throws(() => parseNameStatus("Q\0README.md\0"), /invalid Q/);

expectScope(
  parseNameStatus("R100\0.trellis/tasks/a/prd.md\0.trellis/tasks/archive/a/prd.md\0").flatMap(
    ({ paths }) => paths
  ),
  { scope: "process-docs", fullCi: false, docsChecks: false }
);
assert.equal(classifyNameStatus("R100\0PENDING.md\0src/pending.ts\0", policy).scope, "full");
assert.equal(classifyNameStatus("D\0docs/removed.md\0", policy).scope, "checked-docs");

const baseSha = sha("a");
const headSha = sha("b");
const mergeBaseSha = sha("c");
const pullCalls = [];
const pull = collectChangedPaths({ eventName: "pull_request", baseSha, headSha }, (args) => {
  pullCalls.push(args);
  return args[0] === "merge-base" ? `${mergeBaseSha}\n` : "M\0README.md\0";
});
assert.deepEqual(pull.paths, ["README.md"]);
assert.deepEqual(pullCalls, [
  ["merge-base", baseSha, headSha],
  [
    "diff",
    "--name-status",
    "-z",
    "--find-renames",
    "--find-copies-harder",
    mergeBaseSha,
    headSha,
    "--",
  ],
]);

const pushCalls = [];
const push = collectChangedPaths({ eventName: "push", beforeSha: baseSha, headSha }, (args) => {
  pushCalls.push(args);
  return "D\0PENDING.md\0";
});
assert.deepEqual(push.paths, ["PENDING.md"]);
assert.deepEqual(pushCalls[0].slice(-3), [baseSha, headSha, "--"]);
assert.deepEqual(collectChangedPaths({ eventName: "workflow_dispatch" }), {
  forceFull: true,
  reason: "manual-dispatch",
  paths: [],
});
assert.throws(
  () => collectChangedPaths({ eventName: "push", beforeSha: sha("0"), headSha }),
  /before SHA/
);

const failedClosed = runClassifier({
  eventName: "push",
  beforeSha: sha("0"),
  headSha,
  policyPath,
});
assert.equal(failedClosed.scope, "full");
assert.equal(failedClosed.fullCi, true);
assert.equal(failedClosed.providerTrendBenchmark, true);
assert.equal(failedClosed.reason, "classification-error");

const manual = runClassifier({ eventName: "workflow_dispatch", policyPath });
assert.equal(manual.providerTrendBenchmark, false);
assert.equal(manual.fullCi, true);
assert.equal(manual.reason, "manual-dispatch");

assert.throws(
  () =>
    validatePolicy({
      ...policy,
      version: 2,
    }),
  /version must be 1/
);

console.log("CI change-scope self-test passed.");
