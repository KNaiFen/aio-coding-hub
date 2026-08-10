from __future__ import annotations

import json
import tempfile
import unittest
from argparse import Namespace
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from common.task_store import cmd_create
from task import cmd_start


class TaskRoutingTests(unittest.TestCase):
    def test_create_uses_main_by_default_and_allows_explicit_base(self) -> None:
        for requested_base, expected_base in ((None, "main"), ("release/next", "release/next")):
            with self.subTest(requested_base=requested_base):
                with tempfile.TemporaryDirectory() as raw_root:
                    repo_root = Path(raw_root)
                    args = Namespace(
                        title="Routing contract",
                        slug="routing-contract",
                        assignee="tester",
                        priority="P2",
                        description="Verify task PR target selection.",
                        base_branch=requested_base,
                        parent=None,
                        package=None,
                        no_start=True,
                    )

                    with (
                        patch("common.task_store.get_repo_root", return_value=repo_root),
                        patch("common.task_store.get_developer", return_value="tester"),
                        patch("common.task_store.generate_task_date_prefix", return_value="08-10"),
                        patch("common.task_store.run_task_hooks"),
                    ):
                        result = cmd_create(args)

                    self.assertEqual(result, 0)
                    task_json = (
                        repo_root
                        / ".trellis"
                        / "tasks"
                        / "08-10-routing-contract"
                        / "task.json"
                    )
                    data = json.loads(task_json.read_text(encoding="utf-8"))
                    self.assertEqual(data["base_branch"], expected_base)

    def test_start_without_session_identity_updates_status_without_pointer(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            repo_root = Path(raw_root)
            task_dir = repo_root / ".trellis" / "tasks" / "08-10-degraded-start"
            task_dir.mkdir(parents=True)
            task_json = task_dir / "task.json"
            task_json.write_text(
                json.dumps({"name": "degraded-start", "status": "planning"}) + "\n",
                encoding="utf-8",
            )

            with (
                patch("task.get_repo_root", return_value=repo_root),
                patch("task.resolve_context_key", return_value=None),
                patch("task.run_task_hooks") as run_hooks,
            ):
                result = cmd_start(Namespace(dir="08-10-degraded-start"))

            self.assertEqual(result, 0)
            data = json.loads(task_json.read_text(encoding="utf-8"))
            self.assertEqual(data["status"], "in_progress")
            self.assertFalse((repo_root / ".trellis" / ".runtime" / "sessions").exists())
            run_hooks.assert_called_once_with("after_start", task_json, repo_root)

    def test_start_with_session_identity_sets_pointer_state_and_hook(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            repo_root = Path(raw_root)
            task_dir = repo_root / ".trellis" / "tasks" / "08-10-normal-start"
            task_dir.mkdir(parents=True)
            task_json = task_dir / "task.json"
            task_json.write_text(
                json.dumps({"name": "normal-start", "status": "planning"}) + "\n",
                encoding="utf-8",
            )
            active = SimpleNamespace(task_path=".trellis/tasks/08-10-normal-start", source="test")

            with (
                patch("task.get_repo_root", return_value=repo_root),
                patch("task.resolve_context_key", return_value="session-id"),
                patch("task.set_active_task", return_value=active) as set_active,
                patch("task.run_task_hooks") as run_hooks,
            ):
                result = cmd_start(Namespace(dir="08-10-normal-start"))

            self.assertEqual(result, 0)
            set_active.assert_called_once_with(".trellis/tasks/08-10-normal-start", repo_root)
            data = json.loads(task_json.read_text(encoding="utf-8"))
            self.assertEqual(data["status"], "in_progress")
            run_hooks.assert_called_once_with("after_start", task_json, repo_root)

    def test_start_without_session_identity_rejects_invalid_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            repo_root = Path(raw_root)
            task_dir = repo_root / ".trellis" / "tasks" / "08-10-invalid-start"
            task_dir.mkdir(parents=True)
            (task_dir / "task.json").write_text("{not-json}\n", encoding="utf-8")

            with (
                patch("task.get_repo_root", return_value=repo_root),
                patch("task.resolve_context_key", return_value=None),
                patch("task.run_task_hooks") as run_hooks,
                patch("task.write_json") as write_json,
            ):
                result = cmd_start(Namespace(dir="08-10-invalid-start"))

            self.assertEqual(result, 1)
            run_hooks.assert_not_called()
            write_json.assert_not_called()

    def test_start_without_session_identity_rejects_status_write_failure(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            repo_root = Path(raw_root)
            task_dir = repo_root / ".trellis" / "tasks" / "08-10-write-failure"
            task_dir.mkdir(parents=True)
            task_json = task_dir / "task.json"
            task_json.write_text(
                json.dumps({"name": "write-failure", "status": "planning"}) + "\n",
                encoding="utf-8",
            )

            with (
                patch("task.get_repo_root", return_value=repo_root),
                patch("task.resolve_context_key", return_value=None),
                patch("task.run_task_hooks") as run_hooks,
                patch("task.write_json", return_value=False),
            ):
                result = cmd_start(Namespace(dir="08-10-write-failure"))

            self.assertEqual(result, 1)
            run_hooks.assert_not_called()
            data = json.loads(task_json.read_text(encoding="utf-8"))
            self.assertEqual(data["status"], "planning")

    def test_start_clears_active_pointer_when_status_write_fails(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            repo_root = Path(raw_root)
            task_dir = repo_root / ".trellis" / "tasks" / "08-10-pointer-cleanup"
            task_dir.mkdir(parents=True)
            (task_dir / "task.json").write_text(
                json.dumps({"name": "pointer-cleanup", "status": "planning"}) + "\n",
                encoding="utf-8",
            )

            with (
                patch("task.get_repo_root", return_value=repo_root),
                patch("task.resolve_context_key", return_value="session-id"),
                patch(
                    "task.set_active_task",
                    return_value=SimpleNamespace(task_path=".trellis/tasks/08-10-pointer-cleanup", source="test"),
                ),
                patch("task.write_json", return_value=False),
                patch("task.clear_active_task") as clear_active,
                patch("task.run_task_hooks") as run_hooks,
            ):
                result = cmd_start(Namespace(dir="08-10-pointer-cleanup"))

            self.assertEqual(result, 1)
            clear_active.assert_called_once_with(repo_root)
            run_hooks.assert_not_called()


if __name__ == "__main__":
    unittest.main()
