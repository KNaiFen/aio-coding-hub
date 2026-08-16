from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from common.active_task import resolve_active_task


class ActiveTaskRoutingTests(unittest.TestCase):
    def _task(self, repo: Path, name: str, *, status: str = "in_progress") -> str:
        task_dir = repo / ".trellis" / "tasks" / name
        task_dir.mkdir(parents=True)
        (task_dir / "task.json").write_text(
            json.dumps({"name": name, "status": status}) + "\n",
            encoding="utf-8",
        )
        return f".trellis/tasks/{name}"

    def _session(self, repo: Path, key: str, task_ref: str) -> Path:
        path = repo / ".trellis" / ".runtime" / "sessions" / f"{key}.json"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps({"current_task": task_ref}) + "\n", encoding="utf-8")
        return path

    def test_exact_stale_pointer_is_removed_without_cross_session_fallback(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            repo = Path(raw_root)
            stale = self._session(repo, "exact", ".trellis/tasks/missing")
            live_ref = self._task(repo, "08-16-live")
            self._session(repo, "other", live_ref)

            with patch("common.active_task.resolve_context_key", return_value="exact"):
                active = resolve_active_task(repo)

            self.assertIsNone(active.task_path)
            self.assertFalse(stale.exists())

    def test_sole_stale_fallback_pointer_is_removed(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            repo = Path(raw_root)
            stale = self._session(repo, "stale", ".trellis/tasks/missing")
            with patch("common.active_task.resolve_context_key", return_value=None):
                active = resolve_active_task(repo)
            self.assertIsNone(active.task_path)
            self.assertFalse(stale.exists())

    def test_stale_pointer_is_pruned_before_single_live_fallback(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            repo = Path(raw_root)
            live_ref = self._task(repo, "08-16-live")
            live = self._session(repo, "live", live_ref)
            stale = self._session(repo, "stale", ".trellis/tasks/missing")
            with patch("common.active_task.resolve_context_key", return_value=None):
                active = resolve_active_task(repo)
            self.assertEqual(active.task_path, live_ref)
            self.assertEqual(active.source_type, "session-fallback")
            self.assertTrue(live.exists())
            self.assertFalse(stale.exists())

    def test_two_live_sessions_still_refuse_to_guess(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            repo = Path(raw_root)
            self._session(repo, "one", self._task(repo, "08-16-one"))
            self._session(repo, "two", self._task(repo, "08-16-two"))
            with patch("common.active_task.resolve_context_key", return_value=None):
                active = resolve_active_task(repo)
            self.assertIsNone(active.task_path)

    def test_corrupt_session_is_preserved_and_blocks_fallback(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            repo = Path(raw_root)
            self._session(repo, "live", self._task(repo, "08-16-live"))
            corrupt = repo / ".trellis" / ".runtime" / "sessions" / "corrupt.json"
            corrupt.write_text("{not-json}\n", encoding="utf-8")
            with patch("common.active_task.resolve_context_key", return_value=None):
                active = resolve_active_task(repo)
            self.assertIsNone(active.task_path)
            self.assertTrue(corrupt.exists())

    def test_completed_task_pointer_is_not_live(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            repo = Path(raw_root)
            ref = self._task(repo, "08-16-completed", status="completed")
            session = self._session(repo, "completed", ref)
            with patch("common.active_task.resolve_context_key", return_value=None):
                active = resolve_active_task(repo)
            self.assertIsNone(active.task_path)
            self.assertFalse(session.exists())


if __name__ == "__main__":
    unittest.main()
