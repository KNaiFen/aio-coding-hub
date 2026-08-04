import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const releaseWorkflowPath = fileURLToPath(
  new URL("../.github/workflows/release.yml", import.meta.url)
);
const releaseWorkflow = readFileSync(releaseWorkflowPath, "utf8");
const tagFetchCommand = 'git fetch --no-tags origin "$tag_ref"';
const sourceResolveCommand = 'source_sha="$(git rev-parse --verify "FETCH_HEAD^{commit}")"';
const mainFetchCommand = 'git fetch --no-tags origin "refs/heads/main:refs/remotes/origin/main"';

const tagFetchIndex = releaseWorkflow.indexOf(tagFetchCommand);
const sourceResolveIndex = releaseWorkflow.indexOf(sourceResolveCommand);
const mainFetchIndex = releaseWorkflow.indexOf(mainFetchCommand);

assert.notEqual(tagFetchIndex, -1, "release workflow must fetch the tag into FETCH_HEAD");
assert.notEqual(sourceResolveIndex, -1, "release workflow must peel FETCH_HEAD to a commit");
assert.notEqual(mainFetchIndex, -1, "release workflow must fetch origin/main");
assert.ok(
  tagFetchIndex < sourceResolveIndex && sourceResolveIndex < mainFetchIndex,
  "release workflow must resolve FETCH_HEAD before the main fetch overwrites it"
);
assert.ok(
  !releaseWorkflow.includes('"$tag_ref:$tag_ref"'),
  "release workflow must not overwrite a checkout-created local tag"
);

function runGit(cwd, args, { allowFailure = false } = {}) {
  const result = spawnSync("git", args, { cwd, encoding: "utf8" });
  if (result.error) throw result.error;
  if (!allowFailure && result.status !== 0) {
    throw new Error(
      `git ${args.join(" ")} failed (${result.status}): ${result.stderr || result.stdout}`
    );
  }
  return result;
}

function createConsumer(root, name, origin, sourceSha, localTag) {
  const consumer = join(root, name);
  runGit(root, ["init", "--initial-branch=main", consumer]);
  runGit(consumer, ["remote", "add", "origin", origin]);
  runGit(consumer, ["fetch", "--no-tags", "origin", "refs/heads/main"]);
  runGit(consumer, ["checkout", "--detach", "FETCH_HEAD"]);
  if (localTag) runGit(consumer, ["tag", localTag, sourceSha]);
  return consumer;
}

function resolveReleaseSource(cwd, tagName) {
  if (!/^aio-coding-hub-v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$/.test(tagName)) {
    throw new Error(`Invalid release tag: ${tagName}`);
  }

  const tagRef = `refs/tags/${tagName}`;
  runGit(cwd, ["fetch", "--no-tags", "origin", tagRef]);
  const sourceSha = runGit(cwd, ["rev-parse", "--verify", "FETCH_HEAD^{commit}"]).stdout.trim();
  runGit(cwd, ["fetch", "--no-tags", "origin", "refs/heads/main:refs/remotes/origin/main"]);
  const ancestor = runGit(cwd, ["merge-base", "--is-ancestor", sourceSha, "origin/main"], {
    allowFailure: true,
  });
  if (ancestor.status !== 0) {
    throw new Error(`Release source ${sourceSha} is not an ancestor of origin/main`);
  }
  return sourceSha;
}

const root = mkdtempSync(join(tmpdir(), "aio-release-source-"));

try {
  const origin = join(root, "origin.git");
  const source = join(root, "source");
  const releaseTag = "aio-coding-hub-v0.60.47";
  const detachedTag = "aio-coding-hub-v0.60.48";
  const releaseRef = `refs/tags/${releaseTag}`;

  runGit(root, ["init", "--bare", "--initial-branch=main", origin]);
  runGit(root, ["init", "--initial-branch=main", source]);
  runGit(source, ["config", "user.name", "Release Source Selftest"]);
  runGit(source, ["config", "user.email", "release-source@example.invalid"]);
  runGit(source, ["commit", "--allow-empty", "-m", "release source"]);
  const sourceSha = runGit(source, ["rev-parse", "HEAD"]).stdout.trim();
  const sourceTree = runGit(source, ["rev-parse", "HEAD^{tree}"]).stdout.trim();
  runGit(source, ["remote", "add", "origin", origin]);
  runGit(source, ["push", "--set-upstream", "origin", "main"]);
  runGit(source, ["tag", "-a", releaseTag, "-m", releaseTag]);
  runGit(source, ["push", "origin", releaseRef]);

  const detachedSha = runGit(source, [
    "commit-tree",
    sourceTree,
    "-m",
    "detached release source",
  ]).stdout.trim();
  runGit(source, ["tag", "-a", detachedTag, detachedSha, "-m", detachedTag]);
  runGit(source, ["push", "origin", `refs/tags/${detachedTag}`]);

  const tagPushCheckout = createConsumer(root, "tag-push", origin, sourceSha, releaseTag);
  assert.equal(runGit(tagPushCheckout, ["cat-file", "-t", releaseRef]).stdout.trim(), "commit");

  const legacyFetch = runGit(
    tagPushCheckout,
    ["fetch", "--no-tags", "origin", `${releaseRef}:${releaseRef}`],
    { allowFailure: true }
  );
  assert.notEqual(legacyFetch.status, 0, "legacy ref overwrite must reproduce the tag collision");
  assert.equal(resolveReleaseSource(tagPushCheckout, releaseTag), sourceSha);
  assert.equal(
    runGit(tagPushCheckout, ["cat-file", "-t", releaseRef]).stdout.trim(),
    "commit",
    "FETCH_HEAD resolution must leave the checkout-created local tag untouched"
  );

  const manualCheckout = createConsumer(root, "manual", origin, sourceSha);
  assert.equal(resolveReleaseSource(manualCheckout, releaseTag), sourceSha);
  assert.notEqual(
    runGit(manualCheckout, ["show-ref", "--verify", "--quiet", releaseRef], {
      allowFailure: true,
    }).status,
    0,
    "FETCH_HEAD resolution must not create a local tag"
  );

  assert.throws(() => resolveReleaseSource(manualCheckout, "invalid-tag"), /Invalid release tag/);
  assert.throws(() => resolveReleaseSource(manualCheckout, "aio-coding-hub-v0.60.99"), /git fetch/);
  assert.throws(
    () => resolveReleaseSource(manualCheckout, detachedTag),
    /not an ancestor of origin\/main/
  );
} finally {
  rmSync(root, { recursive: true, force: true });
}

console.log("Release source resolution self-test passed.");
