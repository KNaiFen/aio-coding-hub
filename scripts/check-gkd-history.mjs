import { spawnSync } from "node:child_process";
import { lstatSync, readFileSync, realpathSync } from "node:fs";
import { dirname, isAbsolute, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { AdapterError, readHistoryAdapter } from "./check-gkd-adapter.mjs";

const modulePath = fileURLToPath(import.meta.url);
const repoRoot = dirname(dirname(modulePath));
const PATH_SEGMENT = /^[A-Za-z0-9][A-Za-z0-9._-]*$/;

export class HistoryError extends Error {}

function fail(code) {
  throw new HistoryError(code);
}

function isRecord(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function gitTrackedPaths(root, pathspec) {
  const result = spawnSync("git", ["ls-files", "-z", "--", pathspec], {
    cwd: root,
    encoding: null,
    shell: false,
    maxBuffer: 16 * 1024 * 1024,
  });
  if (result.error || result.status !== 0 || !Buffer.isBuffer(result.stdout)) fail("HISTORY_GIT_FAILED");

  const decoder = new TextDecoder("utf-8", { fatal: true });
  const paths = [];
  let start = 0;
  try {
    for (let index = 0; index < result.stdout.length; index += 1) {
      if (result.stdout[index] !== 0) continue;
      if (index > start) paths.push(decoder.decode(result.stdout.subarray(start, index)));
      start = index + 1;
    }
    if (start !== result.stdout.length) fail("HISTORY_GIT_OUTPUT_INVALID");
  } catch (error) {
    if (error instanceof HistoryError) throw error;
    fail("HISTORY_GIT_OUTPUT_INVALID");
  }
  return paths;
}

function assertRepositoryRoot(root) {
  const result = spawnSync("git", ["rev-parse", "--show-toplevel"], {
    cwd: root,
    encoding: "utf8",
    shell: false,
  });
  if (result.error || result.status !== 0) fail("HISTORY_REPOSITORY_INVALID");
  try {
    if (realpathSync(result.stdout.trim()) !== realpathSync(root)) fail("HISTORY_REPOSITORY_INVALID");
  } catch (error) {
    if (error instanceof HistoryError) throw error;
    fail("HISTORY_REPOSITORY_INVALID");
  }
}

function readManifest(root, path) {
  const absolute = resolve(root, path);
  const fromRoot = relative(root, absolute);
  if (!fromRoot || fromRoot.startsWith("..") || isAbsolute(fromRoot)) fail("HISTORY_MANIFEST_PATH_INVALID");

  let metadata;
  let value;
  try {
    metadata = lstatSync(absolute);
    if (!metadata.isFile() || metadata.isSymbolicLink()) fail("HISTORY_MANIFEST_INVALID");
    const resolved = realpathSync(absolute);
    const resolvedFromRoot = relative(realpathSync(root), resolved);
    if (!resolvedFromRoot || resolvedFromRoot.startsWith("..") || isAbsolute(resolvedFromRoot)) {
      fail("HISTORY_MANIFEST_PATH_INVALID");
    }
    value = JSON.parse(readFileSync(absolute, "utf8"));
  } catch (error) {
    if (error instanceof HistoryError) throw error;
    fail("HISTORY_MANIFEST_INVALID");
  }
  if (!isRecord(value)) fail("HISTORY_MANIFEST_INVALID");
  return value;
}

function classifyManifestPaths(paths, adapter) {
  const activePrefix = `${adapter.active.root}/`;
  const archivePrefix = `${adapter.archive.root}/`;
  const active = [];
  const archived = [];

  for (const path of paths) {
    if (!path.endsWith(`/${adapter.manifestName}`)) continue;

    if (path.startsWith(archivePrefix)) {
      const segments = path.slice(archivePrefix.length).split("/");
      if (
        segments.length < 2 ||
        segments.at(-1) !== adapter.manifestName ||
        segments.slice(0, -1).some((part) => part === "." || part === ".." || !PATH_SEGMENT.test(part))
      ) {
        fail("HISTORY_MANIFEST_LOCATION_INVALID");
      }
      archived.push(path);
      continue;
    }

    if (path.startsWith(activePrefix)) {
      const segments = path.slice(activePrefix.length).split("/");
      if (
        segments.length !== 2 ||
        segments[0] === "archive" ||
        segments[0] === "." ||
        segments[0] === ".." ||
        !PATH_SEGMENT.test(segments[0]) ||
        segments[1] !== adapter.manifestName
      ) {
        fail("HISTORY_MANIFEST_LOCATION_INVALID");
      }
      active.push(path);
    }
  }
  return { active, archived };
}

function validateActiveManifest(manifest, adapter) {
  if (!Object.hasOwn(manifest, "worktree_path") || manifest.worktree_path !== null) {
    fail("HISTORY_ACTIVE_WORKTREE_PATH_INVALID");
  }
  if (!Object.hasOwn(manifest, "coordination")) return;
  if (
    !isRecord(manifest.coordination) ||
    !Object.hasOwn(manifest.coordination, "version") ||
    !Number.isInteger(manifest.coordination.version)
  ) {
    fail("HISTORY_ACTIVE_COORDINATION_INVALID");
  }
  if (adapter.active.coordinationVersionsRejected.includes(manifest.coordination.version)) {
    fail("HISTORY_ACTIVE_COORDINATION_LEGACY");
  }
}

function validateArchiveManifest(manifest, adapter) {
  if (manifest.status !== adapter.archive.requiredStatus) fail("HISTORY_ARCHIVE_STATUS_INVALID");
}

export function verifyHistory(root = repoRoot) {
  assertRepositoryRoot(root);
  let adapter;
  try {
    adapter = readHistoryAdapter(root);
  } catch (error) {
    if (error instanceof AdapterError) fail(error.message);
    throw error;
  }

  const tracked = gitTrackedPaths(root, adapter.active.root);
  const manifests = classifyManifestPaths(tracked, adapter);
  if (manifests.active.length !== adapter.active.requiredCount) fail("HISTORY_ACTIVE_COUNT_INVALID");

  validateActiveManifest(readManifest(root, manifests.active[0]), adapter);
  for (const path of manifests.archived) validateArchiveManifest(readManifest(root, path), adapter);

  return {
    outcome: "history_ready",
    activeCount: manifests.active.length,
    archivedCount: manifests.archived.length,
  };
}

if (process.argv[1] && resolve(process.argv[1]) === modulePath) {
  try {
    console.log(JSON.stringify(verifyHistory()));
  } catch (error) {
    console.error(
      JSON.stringify({ outcome: "history_failure", reason: error instanceof HistoryError ? error.message : "HISTORY_FAILURE" })
    );
    process.exit(1);
  }
}
