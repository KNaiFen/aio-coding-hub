import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";

import { canonicalBytes } from "./check-gkd-adapter.mjs";
import { verifyHistory } from "./check-gkd-history.mjs";

const historyAdapter = {
  active: {
    coordinationVersionsRejected: [1],
    location: "tracked-immediate-child",
    requiredCount: 1,
    root: ".trellis/tasks",
    worktreePath: "must-be-null",
  },
  archive: {
    location: "tracked-descendants",
    requiredStatus: "completed",
    root: ".trellis/tasks/archive",
    worktreePath: "ignored",
  },
  manifestName: "task.json",
  schemaVersion: 1,
};

function runGit(root, args) {
  const result = spawnSync("git", args, { cwd: root, encoding: "utf8", shell: false });
  assert.equal(result.status, 0, result.stderr || result.stdout);
  return result.stdout;
}

function writeManifest(root, relativePath, value) {
  mkdirSync(dirname(join(root, relativePath)), { recursive: true });
  writeFileSync(join(root, relativePath), `${JSON.stringify(value)}\n`);
}

function createFixture(configure) {
  const root = mkdtempSync(join(tmpdir(), "aio-gkd-history-"));
  runGit(root, ["init", "--initial-branch=main"]);
  runGit(root, ["config", "user.name", "History Selftest"]);
  runGit(root, ["config", "user.email", "history@example.invalid"]);
  mkdirSync(join(root, ".gkd"), { recursive: true });
  writeFileSync(join(root, ".gkd/history-adapter.json"), canonicalBytes(historyAdapter));
  configure(root);
  runGit(root, ["add", ".gkd/history-adapter.json", ".trellis/tasks"]);
  runGit(root, ["commit", "-m", "fixture"]);
  return root;
}

function withFixture(configure, check) {
  const root = createFixture(configure);
  try {
    check(root);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

function activeManifest(overrides = {}) {
  return { id: "active", status: "planning", worktree_path: null, ...overrides };
}

function archiveManifest(worktreePath, includeWorktreePath = true) {
  const manifest = { id: "archived", status: "completed" };
  if (includeWorktreePath) manifest.worktree_path = worktreePath;
  return manifest;
}

const repositoryResult = verifyHistory();
assert.deepEqual(repositoryResult, { outcome: "history_ready", activeCount: 1, archivedCount: 107 });

withFixture(
  (root) => {
    writeManifest(root, ".trellis/tasks/active/task.json", activeManifest());
    writeManifest(root, ".trellis/tasks/archive/2026-01/unix/task.json", archiveManifest("/missing/unix"));
    writeManifest(root, ".trellis/tasks/archive/2026-01/windows/task.json", archiveManifest("C:\\missing\\windows"));
    writeManifest(root, ".trellis/tasks/archive/2026-01/relative/task.json", archiveManifest("../missing"));
    writeManifest(root, ".trellis/tasks/archive/2026-01/null/task.json", archiveManifest(null));
    writeManifest(root, ".trellis/tasks/archive/2026-01/absent/task.json", archiveManifest(undefined, false));
  },
  (root) => {
    const before = runGit(root, ["status", "--porcelain=v1", "-z"]);
    const first = verifyHistory(root);
    const second = verifyHistory(root);
    const after = runGit(root, ["status", "--porcelain=v1", "-z"]);
    assert.deepEqual(first, { outcome: "history_ready", activeCount: 1, archivedCount: 5 });
    assert.deepEqual(second, first);
    assert.equal(after, before);

    writeManifest(root, ".trellis/tasks/untracked/task.json", activeManifest({ id: "untracked" }));
    assert.deepEqual(verifyHistory(root), first);
  }
);

withFixture(
  (root) => {
    writeManifest(root, ".trellis/tasks/archive/2026-01/done/task.json", archiveManifest(null));
  },
  (root) => assert.throws(() => verifyHistory(root), /HISTORY_ACTIVE_COUNT_INVALID/)
);

withFixture(
  (root) => {
    writeManifest(root, ".trellis/tasks/first/task.json", activeManifest({ id: "first" }));
    writeManifest(root, ".trellis/tasks/second/task.json", activeManifest({ id: "second" }));
  },
  (root) => assert.throws(() => verifyHistory(root), /HISTORY_ACTIVE_COUNT_INVALID/)
);

for (const worktreePath of ["/tmp/active", "C:\\active", "../active"]) {
  withFixture(
    (root) => writeManifest(root, ".trellis/tasks/active/task.json", activeManifest({ worktree_path: worktreePath })),
    (root) => assert.throws(() => verifyHistory(root), /HISTORY_ACTIVE_WORKTREE_PATH_INVALID/)
  );
}

withFixture(
  (root) => writeManifest(root, ".trellis/tasks/active/task.json", { id: "active", status: "planning" }),
  (root) => assert.throws(() => verifyHistory(root), /HISTORY_ACTIVE_WORKTREE_PATH_INVALID/)
);

withFixture(
  (root) => {
    writeManifest(root, ".trellis/tasks/active/task.json", activeManifest({ coordination: { version: 1 } }));
  },
  (root) => assert.throws(() => verifyHistory(root), /HISTORY_ACTIVE_COORDINATION_LEGACY/)
);

withFixture(
  (root) => {
    writeManifest(root, ".trellis/tasks/active/task.json", activeManifest({ coordination: { version: "1" } }));
  },
  (root) => assert.throws(() => verifyHistory(root), /HISTORY_ACTIVE_COORDINATION_INVALID/)
);

withFixture(
  (root) => {
    writeManifest(root, ".trellis/tasks/active/task.json", activeManifest());
    writeManifest(root, ".trellis/tasks/archive/2026-01/not-done/task.json", { status: "planning" });
  },
  (root) => assert.throws(() => verifyHistory(root), /HISTORY_ARCHIVE_STATUS_INVALID/)
);

withFixture(
  (root) => {
    writeManifest(root, ".trellis/tasks/active/task.json", activeManifest());
    const path = join(root, ".trellis/tasks/archive/2026-01/malformed/task.json");
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, "{\n");
  },
  (root) => assert.throws(() => verifyHistory(root), /HISTORY_MANIFEST_INVALID/)
);

withFixture(
  (root) => {
    writeManifest(root, ".trellis/tasks/active/task.json", activeManifest());
    writeManifest(root, ".trellis/tasks/nested/deeper/task.json", activeManifest());
  },
  (root) => assert.throws(() => verifyHistory(root), /HISTORY_MANIFEST_LOCATION_INVALID/)
);

withFixture(
  (root) => {
    const target = join(root, "outside.json");
    writeManifest(root, ".trellis/tasks/active/task.json", activeManifest());
    writeFileSync(target, `${JSON.stringify(archiveManifest(null))}\n`);
    const link = join(root, ".trellis/tasks/archive/2026-01/link/task.json");
    mkdirSync(dirname(link), { recursive: true });
    symlinkSync(target, link);
  },
  (root) => assert.throws(() => verifyHistory(root), /HISTORY_MANIFEST_PATH_INVALID|HISTORY_MANIFEST_INVALID/)
);

console.log("[gkd-history:selftest] all assertions passed");
