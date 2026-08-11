import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { assertSyncUpstreamPolicy } from "./check-sync-upstream-policy.mjs";

const workflowPath = fileURLToPath(
  new URL("../.github/workflows/sync-upstream.yml", import.meta.url)
);
const workflow = readFileSync(workflowPath, "utf8");

assert.doesNotThrow(() => assertSyncUpstreamPolicy(workflow));

function expectRejected(name, from, to, expected) {
  assert.ok(workflow.includes(from), `${name}: fixture source must exist`);
  const mutant = workflow.replace(from, to);
  assert.notEqual(mutant, workflow, `${name}: fixture must mutate the workflow`);
  assert.throws(() => assertSyncUpstreamPolicy(mutant), expected, name);
}

function expectMutantRejected(name, mutant, expected) {
  assert.notEqual(mutant, workflow, `${name}: fixture must mutate the workflow`);
  assert.throws(() => assertSyncUpstreamPolicy(mutant), expected, name);
}

const stepsComment = workflow.replace("    steps:\n", "    steps: # security-sensitive steps\n");
assert.notEqual(stepsComment, workflow, "steps comment: fixture must mutate the workflow");
assert.doesNotThrow(() => assertSyncUpstreamPolicy(stepsComment));

const scalarComments = workflow
  .replace("  contents: read\n", "  contents: read # default token remains read-only\n")
  .replace("    timeout-minutes: 10\n", "    timeout-minutes: 10 # bounded execution\n");
assert.notEqual(scalarComments, workflow, "scalar comments: fixture must mutate the workflow");
assert.doesNotThrow(() => assertSyncUpstreamPolicy(scalarComments));

expectRejected("content write permission", "  contents: read", "  contents: write", /read-only/);
expectRejected(
  "default PR write permission",
  "permissions:\n  contents: read",
  "permissions:\n  contents: read\n  pull-requests: write",
  /default GITHUB_TOKEN must receive only/
);
expectRejected(
  "duplicate quoted top-level permissions",
  "permissions:\n  contents: read",
  'permissions:\n  contents: read\n"permissions":\n  contents: write',
  /approved canonical top-level keys/
);
expectRejected(
  "missing sync timeout",
  "    timeout-minutes: 10\n",
  "",
  /sync job must retain a 10-minute timeout/
);
expectRejected(
  "missing credential preflight",
  "      - name: Validate GitHub App credentials",
  "      - name: Disabled GitHub App credential validation",
  /must validate GitHub App credentials/
);
expectRejected(
  "invalid App ID source",
  "SYNC_UPSTREAM_APP_ID: ${{ vars.SYNC_UPSTREAM_APP_ID }}",
  "SYNC_UPSTREAM_APP_ID: ${{ github.app_id }}",
  /credential validation must include SYNC_UPSTREAM_APP_ID/
);
expectRejected(
  "missing private key validation",
  '[[ -n "$SYNC_UPSTREAM_APP_PRIVATE_KEY" ]]',
  '[[ -n "$SYNC_UPSTREAM_APP_ID" ]]',
  /credential validation must retain the exact fail-closed Bash script/
);
expectRejected(
  "credential validation text decoy",
  '          [[ -n "$SYNC_UPSTREAM_APP_ID" ]] || {',
  '          echo \'[[ -n "$SYNC_UPSTREAM_APP_ID" ]]\' || {',
  /credential validation must retain the exact fail-closed Bash script/
);
expectRejected(
  "credential validation conditional step",
  "        shell: bash\n        env:\n          SYNC_UPSTREAM_APP_ID:",
  "        if: ${{ false }}\n        shell: bash\n        env:\n          SYNC_UPSTREAM_APP_ID:",
  /credential validation step must run unconditionally and fail closed/
);
expectRejected(
  "credential validation continue on error",
  "        shell: bash\n        env:\n          SYNC_UPSTREAM_APP_ID:",
  "        continue-on-error: true\n        shell: bash\n        env:\n          SYNC_UPSTREAM_APP_ID:",
  /credential validation step must run unconditionally and fail closed/
);
expectRejected(
  "credential validation key with whitespace before colon",
  "        shell: bash\n        env:\n          SYNC_UPSTREAM_APP_ID:",
  "        if : ${{ false }}\n        shell: bash\n        env:\n          SYNC_UPSTREAM_APP_ID:",
  /approved canonical properties and inputs/
);
expectRejected(
  "credential validation continue-on-error key with whitespace before colon",
  "        shell: bash\n        env:\n          SYNC_UPSTREAM_APP_ID:",
  "        continue-on-error : true\n        shell: bash\n        env:\n          SYNC_UPSTREAM_APP_ID:",
  /approved canonical properties and inputs/
);
expectRejected(
  "credential validation escaped quoted key",
  "        shell: bash\n        env:\n          SYNC_UPSTREAM_APP_ID:",
  '        "\\u0069f": ${{ false }}\n        shell: bash\n        env:\n          SYNC_UPSTREAM_APP_ID:',
  /approved canonical properties and inputs/
);
expectRejected(
  "credential validation ignored failure",
  '          [[ -n "$SYNC_UPSTREAM_APP_ID" ]] || {',
  '          [[ -n "$SYNC_UPSTREAM_APP_ID" ]] || true || {',
  /credential validation must retain the exact fail-closed Bash script/
);
expectRejected(
  "credential validation folded block",
  "        run: |\n          set -euo pipefail\n",
  "        run: >\n          :\n          set -euo pipefail\n",
  /credential validation must retain the exact fail-closed Bash script/
);
expectRejected(
  "unpinned App token action",
  "actions/create-github-app-token@fee1f7d63c2ff003460e3d139729b119787bc349 # v2.2.2",
  "actions/create-github-app-token@v2",
  /GitHub App token step must include uses:/
);
expectMutantRejected(
  "App token action comment decoy",
  workflow.replace(
    "        uses: actions/create-github-app-token@fee1f7d63c2ff003460e3d139729b119787bc349 # v2.2.2",
    "        uses: actions/create-github-app-token@v2\n        # uses: actions/create-github-app-token@fee1f7d63c2ff003460e3d139729b119787bc349 # v2.2.2"
  ),
  /GitHub App token step must include uses:/
);
expectRejected(
  "App contents write",
  "permission-contents: read",
  "permission-contents: write",
  /GitHub App token step must include permission-contents: read/
);
expectRejected(
  "extra App permission",
  "          permission-pull-requests: write\n",
  "          permission-pull-requests: write\n          permission-issues: write\n",
  /must request only contents: read and pull-requests: write/
);
expectRejected(
  "checkout token fallback",
  "token: ${{ steps.app-token.outputs.token }}",
  "token: ${{ steps.app-token.outputs.token || github.token }}",
  /checkout must use the GitHub App token/
);
expectMutantRejected(
  "checkout token comment decoy",
  workflow.replace(
    "          token: ${{ steps.app-token.outputs.token }}",
    "          token: ${{ secrets.UNRELATED_WRITE_TOKEN }}\n          # token: ${{ steps.app-token.outputs.token }}"
  ),
  /checkout must use the GitHub App token/
);
expectMutantRejected(
  "other job checkout decoy",
  `${workflow.replace(
    "      - name: Checkout repository",
    "      - name: Unsafe checkout"
  )}\n  decoy:\n    runs-on: ubuntu-latest\n    timeout-minutes: 5\n    steps:\n      - name: Checkout repository\n        uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1\n        with:\n          token: \${{ steps.app-token.outputs.token }}\n`,
  /retain repository checkout exactly once in the sync job/
);
expectRejected(
  "CLI token fallback",
  "GH_TOKEN: ${{ steps.app-token.outputs.token }}",
  "GH_TOKEN: ${{ steps.app-token.outputs.token || github.token }}",
  /GitHub CLI must use the GitHub App token/
);
expectRejected(
  "legacy PAT fallback",
  "GH_TOKEN: ${{ steps.app-token.outputs.token }}",
  "GH_TOKEN: ${{ secrets.SYNC_UPSTREAM_TOKEN }}",
  /legacy PAT secret/
);
expectRejected(
  "token revoke disabled",
  "          permission-pull-requests: write\n",
  "          permission-pull-requests: write\n          skip-token-revoke: true\n",
  /must be revoked automatically/
);
expectRejected(
  "token revoke disabled by expression",
  "          permission-pull-requests: write\n",
  "          permission-pull-requests: write\n          skip-token-revoke: ${{ true }}\n",
  /must be revoked automatically/
);
expectRejected(
  "token revoke disabled by quoted key",
  "          permission-pull-requests: write\n",
  '          permission-pull-requests: write\n          "skip-token-revoke": true\n',
  /approved canonical properties and inputs/
);
expectRejected(
  "security inputs through YAML merge",
  "          permission-pull-requests: write\n",
  "          permission-pull-requests: write\n          <<: *unsafe-token-inputs\n",
  /must not use YAML merge keys/
);
expectRejected(
  "indirect CLI token override",
  "          set -euo pipefail\n\n          LOCAL=",
  '          set -euo pipefail\n          token_name=GH_TOKEN\n          export "${token_name}=${{ secrets.UNRELATED_WRITE_TOKEN }}"\n\n          LOCAL=',
  /token must be declared only through the approved step environment/
);
expectMutantRejected(
  "quoted CLI token with named decoy",
  workflow.replace(
    "      - name: Open upstream sync PR\n        env:\n          GH_TOKEN: ${{ steps.app-token.outputs.token }}\n        run: |",
    '      - name: Open upstream sync PR\n        env:\n          GH_TOKEN: ${{ steps.app-token.outputs.token }}\n        run: echo "decoy"\n\n      - name: Unsafe upstream sync PR\n        env:\n          "GH_TOKEN": ${{ secrets.UNRELATED_WRITE_TOKEN }}\n        run: |'
  ),
  /GitHub CLI must use the GitHub App token|retain pull request creation/
);
expectRejected(
  "folded upstream PR script",
  "      - name: Open upstream sync PR\n        env:\n          GH_TOKEN: ${{ steps.app-token.outputs.token }}\n        run: |",
  "      - name: Open upstream sync PR\n        env:\n          GH_TOKEN: ${{ steps.app-token.outputs.token }}\n        run: >",
  /GitHub CLI must use the GitHub App token/
);
expectRejected(
  "token minted after checkout",
  "      - name: Create GitHub App token",
  "      - name: Z Create GitHub App token",
  /must create a GitHub App token/
);
expectRejected(
  "direct target push",
  'create_or_update_upstream_pr "target branch can be fast-forwarded but requires fork semantic review."',
  ':; git -c http.extraheader=redacted push origin "HEAD:${TARGET_BRANCH}"',
  /must not push/
);
expectRejected(
  "automatic CLI merge",
  'echo "Manual review and merge required."',
  ':; gh pr merge "${pr_number}" --repo "${GITHUB_REPOSITORY}" --merge\n              echo "Manual review and merge required."',
  /must not merge/
);
expectRejected(
  "automatic pull request approval",
  'echo "Manual review and merge required."',
  'gh pr review "${pr_number}" --repo "${GITHUB_REPOSITORY}" --approve\n              echo "Manual review and merge required."',
  /must not approve/
);
expectRejected(
  "automatic API merge",
  'echo "Manual review and merge required."',
  'gh api -X PUT "repos/${GITHUB_REPOSITORY}/pulls/${pr_number}/merge"\n              echo "Manual review and merge required."',
  /GitHub API/
);
expectRejected(
  "indirect GitHub API merge",
  'echo "Manual review and merge required."',
  'gh_bin=gh\n              "${gh_bin}" api -X PUT "repos/${GITHUB_REPOSITORY}/pulls/${pr_number}/merge"\n              echo "Manual review and merge required."',
  /upstream PR creation must retain the approved fail-closed script/
);
expectMutantRejected(
  "extra action cannot consume the App token",
  workflow.replace(
    "      - name: Open upstream sync PR",
    `      - name: Unsafe token consumer
        uses: actions/github-script@3a2844b7e9c422d3c10d287c895573f7108da1b3
        with:
          github-token: \${{ steps.app-token.outputs.token }}
          script: github.rest.pulls.merge({ owner: context.repo.owner, repo: context.repo.repo, pull_number: 1 })

      - name: Open upstream sync PR`
  ),
  /only the approved ordered steps|may be consumed only by checkout and GitHub CLI/
);
expectRejected(
  "job-level permissions cannot override the read-only token",
  "    timeout-minutes: 10\n",
  "    timeout-minutes: 10\n    permissions: { contents: write, pull-requests: write }\n",
  /only the approved canonical job properties/
);
expectRejected(
  "missing PR creation",
  "gh pr create \\",
  "gh pr view \\",
  /retain pull request creation/
);
expectRejected(
  "new PR stdout is not captured",
  '              created_pr_url="$(\n',
  '              created_pr_url="not-a-command"\n',
  /capture gh pr create stdout/
);
expectRejected(
  "new PR list fallback",
  '              if [[ ! "${pr_number}" =~ ^[1-9][0-9]*$ ]]; then',
  '              gh pr list\n\n              if [[ ! "${pr_number}" =~ ^[1-9][0-9]*$ ]]; then',
  /existing sync pull request exactly once/
);
expectRejected(
  "existing PR list is not limited to open PRs",
  "--state open",
  "--state all",
  /existing sync pull request exactly once/
);
expectRejected(
  "new PR URL repository validation removed",
  'if [[ "${created_pr_url}" != "${created_pr_url_prefix}"* ]]; then',
  "if false; then",
  /new pull request URL is outside the current repository/
);
expectRejected(
  "new PR zero number allowed",
  "^[1-9][0-9]*$",
  "^[0-9][0-9]*$",
  /not a non-zero positive integer/
);
expectRejected(
  "missing manual review body",
  "Please review and merge manually.",
  "This pull request is ready.",
  /body must require manual review/
);
expectRejected(
  "missing manual review summary",
  'echo "Manual review and merge required."',
  'echo "Pull request ready."',
  /step summary/
);
expectRejected(
  "fast-forward bypass",
  'create_or_update_upstream_pr "target branch can be fast-forwarded but requires fork semantic review."',
  'echo "Fast-forward path skipped."',
  /fast-forward topology must create/
);
expectRejected(
  "local conflict merge",
  'create_or_update_upstream_pr "target branch has local commits and cannot be safely fast-forwarded."',
  ':; git merge "${UPSTREAM}"',
  /must not merge upstream commits locally/
);
expectRejected(
  "missing conflict failure",
  '[ "${merge_state}" = "DIRTY" ]',
  '[ "${merge_state}" = "CLEAN" ]',
  /fail closed when the open PR has conflicts/
);
expectRejected(
  "missing unavailable merge-state failure",
  '[ -z "${merge_state}" ] ||',
  '[ "${merge_state}" = "CLEAN" ] ||',
  /merge state is unavailable/
);
expectRejected(
  "review-blocked PR remains open",
  '[ "${merge_state}" = "UNKNOWN" ]; then',
  '[ "${merge_state}" = "UNKNOWN" ] || [ "${merge_state}" = "BLOCKED" ]; then',
  /leave review-blocked pull requests open/
);
expectRejected(
  "missing no-op branch",
  "Already up to date. Nothing to sync.",
  "No-op branch removed.",
  /no-op branch must remain explicit/
);
expectRejected("missing schedule", "  schedule:", "  disabled-schedule:", /scheduled trigger/);
expectRejected(
  "missing manual trigger",
  "  workflow_dispatch:",
  "  disabled-workflow-dispatch:",
  /manual trigger/
);
expectRejected(
  "changed upstream repository",
  "UPSTREAM_REPO: dyndynjyxa/aio-coding-hub",
  "UPSTREAM_REPO: example/other",
  /configured upstream repository/
);
expectRejected(
  "missing upstream fetch",
  'git fetch upstream "+refs/heads/${TARGET_BRANCH}:refs/remotes/upstream/${TARGET_BRANCH}"',
  'echo "upstream fetch removed"',
  /fetch the target branch from upstream/
);

console.log("Sync upstream manual-review policy self-test passed.");
