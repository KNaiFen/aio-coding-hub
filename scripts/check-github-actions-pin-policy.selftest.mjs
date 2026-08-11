import assert from "node:assert/strict";

import {
  assertGithubActionsPinPolicy,
  assertGithubActionsTimeoutPolicy,
  validateGithubActionsPinPolicy,
  validateGithubActionsTimeoutPolicy,
} from "./check-github-actions-pin-policy.mjs";

const sha = "1234567890abcdef1234567890abcdef12345678";
const digest = "a".repeat(64);
const valid = {
  ".github/workflows/ci.yml": `
jobs:
  verify:
    uses: owner/repository/.github/workflows/reusable.yml@${sha} # v1.2.3
  test:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - uses: actions/checkout@${sha} # v7.0.1
      - uses: ./.github/actions/local
      - uses: docker://registry.example.com/tool@sha256:${digest}
      - { name: Cached action, uses: actions/cache@${sha} }
`,
  ".github/actions/local/action.yml": `
runs:
  using: composite
  steps:
    - uses: actions/setup-node@${sha}
`,
};

assert.deepEqual(validateGithubActionsPinPolicy(valid), []);
assert.doesNotThrow(() => assertGithubActionsPinPolicy(valid));
assert.deepEqual(validateGithubActionsTimeoutPolicy(valid), []);
assert.doesNotThrow(() => assertGithubActionsTimeoutPolicy(valid));

for (const [name, value, expected] of [
  ["tag", "actions/checkout@v7", /full 40-character commit SHA/],
  ["branch", "actions/checkout@main", /full 40-character commit SHA/],
  ["short SHA", "actions/checkout@1234567", /full 40-character commit SHA/],
  ["missing ref", "actions/checkout", /full 40-character commit SHA/],
  ["dynamic ref", "actions/checkout@${{ github.sha }}", /full 40-character commit SHA/],
  ["Docker tag", "docker://alpine:3.22", /sha256 digest/],
  ["short Docker digest", "docker://alpine@sha256:abc123", /sha256 digest/],
]) {
  assert.throws(
    () =>
      assertGithubActionsPinPolicy({
        ".github/workflows/invalid.yml": `jobs:\n  test:\n    steps:\n      - uses: ${value}\n`,
      }),
    expected,
    name
  );
}

assert.throws(
  () =>
    assertGithubActionsPinPolicy({
      ".github/workflows/flow-step.yml": `
jobs:
  test:
    steps:
      - { name: Checkout, uses: actions/checkout@v7 }
`,
    }),
  /full 40-character commit SHA/,
  "flow-style step mappings must not bypass pinning"
);

assert.throws(
  () =>
    assertGithubActionsPinPolicy({
      ".github/workflows/noncanonical-indent.yml": `
jobs:
   test:
      steps:
         - uses: actions/checkout@v7
`,
    }),
  /canonical two-space indentation/,
  "noncanonical indentation must fail closed"
);

assert.throws(
  () =>
    assertGithubActionsPinPolicy({
      ".github/workflows/quoted-uses.yml": `
jobs:
  test:
    steps:
      - "uses": actions/checkout@v7
`,
    }),
  /quoted block mapping keys are not supported/,
  "quoted uses keys must fail closed"
);

assert.throws(
  () =>
    assertGithubActionsPinPolicy({
      ".github/workflows/spaced-uses.yml": `
jobs:
  test:
    steps:
      - uses : actions/checkout@v7
`,
    }),
  /mapping keys must not contain whitespace before the colon/,
  "whitespace before a step uses colon must fail closed"
);

assert.throws(
  () =>
    assertGithubActionsPinPolicy({
      ".github/workflows/spaced-reusable-uses.yml": `
jobs:
  reuse:
    uses : owner/repository/.github/workflows/reusable.yml@v1
`,
    }),
  /mapping keys must not contain whitespace before the colon/,
  "whitespace before a reusable-workflow uses colon must fail closed"
);

assert.throws(
  () =>
    assertGithubActionsPinPolicy({
      ".github/workflows/quoted-job.yml": `
jobs:
  "reuse":
    uses: owner/repository/.github/workflows/reusable.yml@v1
`,
    }),
  /quoted block mapping keys are not supported|job ids must use unquoted block mapping syntax/,
  "quoted job ids must fail closed"
);

assert.throws(
  () =>
    assertGithubActionsPinPolicy({
      ".github/workflows/aliased-step.yml": `
x-checkout: &checkout-step
  uses: actions/checkout@v7
jobs:
  test:
    steps:
      - *checkout-step
`,
    }),
  /YAML anchors and aliases are not supported/,
  "aliased steps must fail closed"
);

assert.throws(
  () =>
    assertGithubActionsPinPolicy({
      ".github/actions/flow-runs/action.yml": `
runs: { using: composite, steps: [{ uses: actions/checkout@v7 }] }
`,
    }),
  /runs must use block mapping syntax/,
  "flow-style composite runs must fail closed"
);

assert.deepEqual(
  validateGithubActionsPinPolicy({
    ".github/workflows/comments.yml": `
jobs:
  test:
    steps:
      # - uses: actions/checkout@v7
      - run: echo "uses: actions/checkout@v7"
`,
  }),
  []
);

const timeoutComments = {
  ".github/workflows/comments.yml": `
jobs: # workflow jobs
  test: # bounded job
    runs-on: ubuntu-latest
    timeout-minutes: 10 # bounded execution
    steps:
      - run: true
`,
};
assert.deepEqual(validateGithubActionsPinPolicy(timeoutComments), []);
assert.deepEqual(validateGithubActionsTimeoutPolicy(timeoutComments), []);

assert.deepEqual(
  validateGithubActionsPinPolicy({
    ".github/workflows/non-action-uses.yml": `
jobs:
  test:
    steps:
      - name: Preserve an ordinary env key
        env: { uses: actions/checkout@v7 }
        run: echo "$uses"
`,
  }),
  []
);

assert.throws(
  () =>
    assertGithubActionsTimeoutPolicy({
      ".github/workflows/no-timeout.yml": "jobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - run: true\n",
    }),
  /job test must define a positive timeout-minutes/
);
assert.throws(
  () =>
    assertGithubActionsTimeoutPolicy({
      ".github/workflows/duplicate-jobs.yml": `
jobs:
  bounded:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - run: true
jobs:
  unbounded:
    runs-on: ubuntu-latest
    steps:
      - run: true
`,
    }),
  /duplicate top-level key jobs is not supported/,
  "duplicate jobs blocks must not bypass timeout validation"
);
assert.throws(
  () =>
    assertGithubActionsTimeoutPolicy({
      ".github/workflows/invalid-timeout.yml":
        "jobs:\n  test:\n    runs-on: ubuntu-latest\n    timeout-minutes: 0\n    steps:\n      - run: true\n",
    }),
  /job test must define a positive timeout-minutes/
);

console.log("GitHub Actions pin policy self-test passed.");
