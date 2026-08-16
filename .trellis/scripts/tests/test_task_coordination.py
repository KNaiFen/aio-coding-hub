from __future__ import annotations

import io
import json
import subprocess
import tempfile
import unittest
from argparse import Namespace
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest.mock import patch

from common.task_coordination import (
    cmd_block,
    cmd_delegate,
    cmd_deliver,
    cmd_doctor,
    cmd_handoff,
    cmd_resume,
    cmd_status,
    load_task_manifest,
    mark_started,
    validate_task_manifest,
    write_task_manifest_path,
    TaskCoordinationError,
)


class TaskCoordinationTests(unittest.TestCase):
    def _git(self, repo: Path, *args: str) -> str:
        result = subprocess.run(
            ["git", *args],
            cwd=repo,
            check=True,
            capture_output=True,
            text=True,
            encoding="utf-8",
        )
        return result.stdout.strip()

    def _make_repo(self, raw_root: str) -> tuple[Path, Path, str, str]:
        repo = Path(raw_root)
        self._git(repo, "init", "-b", "main")
        self._git(repo, "config", "user.name", "Trellis Test")
        self._git(repo, "config", "user.email", "trellis@example.invalid")
        (repo / "README.md").write_text("base\n", encoding="utf-8")
        self._git(repo, "add", "README.md")
        self._git(repo, "commit", "-m", "base")
        base_sha = self._git(repo, "rev-parse", "HEAD")

        task_dir = repo / ".trellis" / "tasks" / "08-16-example"
        task_dir.mkdir(parents=True)
        (task_dir / "execution.md").write_text("# Execution\n", encoding="utf-8")
        (task_dir / "task.json").write_text(
            json.dumps(
                {
                    "name": "example",
                    "title": "Example",
                    "status": "planning",
                    "base_branch": "main",
                    "branch": None,
                    "worktree_path": None,
                    "commit": None,
                    "pr_url": None,
                    "future": {"preserve": True},
                    "coordination": {
                        "version": 1,
                        "route": "main",
                        "phase": "planning",
                        "writer": "main",
                        "base_sha": None,
                        "planning_commit": None,
                        "block": None,
                        "updated_at": "old",
                        "future": {"preserve": True},
                    },
                },
                indent=2,
            )
            + "\n",
            encoding="utf-8",
        )
        self._git(repo, "add", ".trellis/tasks/08-16-example")
        self._git(repo, "commit", "-m", "planning")
        planning_commit = self._git(repo, "rev-parse", "HEAD")
        return repo, task_dir, base_sha, planning_commit

    def test_delegate_start_doctor_and_handoff(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            repo, task_dir, base_sha, planning_commit = self._make_repo(raw_root)
            args = Namespace(
                dir="08-16-example",
                worktree=str(repo.resolve()),
                branch="main",
                base_sha=base_sha,
                planning_commit=planning_commit,
                writer="execution-session",
            )
            with patch("common.task_coordination.get_repo_root", return_value=repo):
                self.assertEqual(cmd_delegate(args), 0)

            data = load_task_manifest(task_dir)
            self.assertEqual(data["coordination"]["phase"], "ready")
            self.assertEqual(data["coordination"]["future"], {"preserve": True})
            self.assertEqual(data["future"], {"preserve": True})
            self._git(repo, "add", ".trellis/tasks/08-16-example/task.json")
            self._git(repo, "commit", "-m", "delegate")

            data = load_task_manifest(task_dir)
            self.assertTrue(mark_started(data))
            self.assertTrue(write_task_manifest_path(task_dir / "task.json", data))
            self._git(repo, "add", ".trellis/tasks/08-16-example/task.json")
            self._git(repo, "commit", "-m", "start")

            with patch("common.task_coordination.get_repo_root", return_value=repo):
                self.assertEqual(cmd_doctor(Namespace(dir="08-16-example")), 0)
                output = io.StringIO()
                with redirect_stdout(output):
                    self.assertEqual(cmd_handoff(Namespace(dir="08-16-example", json=False)), 0)
            handoff = output.getvalue()
            self.assertIn("# Execution handoff", handoff)
            self.assertIn(str(repo.resolve()), handoff)
            self.assertIn(planning_commit, handoff)
            self.assertIn("task.py deliver", handoff)
            self.assertIn("$aio-trellis-execute", handoff)

    def test_ready_block_and_resume_remain_valid(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            repo, task_dir, base_sha, planning_commit = self._make_repo(raw_root)
            with patch("common.task_coordination.get_repo_root", return_value=repo):
                self.assertEqual(
                    cmd_delegate(
                        Namespace(
                            dir="08-16-example",
                            worktree=str(repo.resolve()),
                            branch="main",
                            base_sha=base_sha,
                            planning_commit=planning_commit,
                            writer="execution-session",
                        )
                    ),
                    0,
                )
                self.assertEqual(
                    cmd_block(
                        Namespace(
                            dir="08-16-example",
                            reason="Wait for dependency",
                            resume_condition="Dependency merged",
                            owner="main",
                        )
                    ),
                    0,
                )
                self.assertEqual(cmd_doctor(Namespace(dir="08-16-example")), 0)
                self.assertEqual(
                    cmd_resume(Namespace(dir="08-16-example", writer="execution-session")),
                    0,
                )
                self.assertEqual(cmd_doctor(Namespace(dir="08-16-example")), 0)

            resumed = load_task_manifest(task_dir)
            self.assertEqual(resumed["status"], "planning")
            self.assertEqual(resumed["coordination"]["phase"], "ready")

    def test_start_rejects_writer_override_and_terminal_phases(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            _, task_dir, _, _ = self._make_repo(raw_root)
            data = load_task_manifest(task_dir)
            data["coordination"]["phase"] = "ready"
            with self.assertRaisesRegex(TaskCoordinationError, "only valid"):
                mark_started(data, writer="other-session")

            for phase in ("blocked", "completed"):
                with self.subTest(phase=phase):
                    data["coordination"]["phase"] = phase
                    with self.assertRaisesRegex(TaskCoordinationError, f"phase={phase}"):
                        mark_started(data)

    def test_status_handles_invalid_block_without_traceback(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            repo, task_dir, _, _ = self._make_repo(raw_root)
            data = load_task_manifest(task_dir)
            data["coordination"]["block"] = "invalid"
            self.assertTrue(write_task_manifest_path(task_dir / "task.json", data))
            output = io.StringIO()
            with (
                patch("common.task_coordination.get_repo_root", return_value=repo),
                redirect_stdout(output),
            ):
                self.assertEqual(cmd_status(Namespace(dir="08-16-example", json=False)), 0)
            self.assertIn("invalid coordination.block", output.getvalue())

    def test_coordination_version_requires_integer_one(self) -> None:
        for invalid_version in (True, 1.0):
            with self.subTest(version=invalid_version), tempfile.TemporaryDirectory() as raw_root:
                repo, task_dir, _, _ = self._make_repo(raw_root)
                data = load_task_manifest(task_dir)
                data["coordination"]["version"] = invalid_version
                self.assertTrue(write_task_manifest_path(task_dir / "task.json", data))
                self.assertIn(
                    "coordination.version must be 1",
                    validate_task_manifest(task_dir, repo),
                )

    def test_block_and_resume_restore_previous_phase(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            repo, task_dir, base_sha, planning_commit = self._make_repo(raw_root)
            data = load_task_manifest(task_dir)
            data["status"] = "in_progress"
            data["coordination"].update(
                {
                    "route": "delegated",
                    "phase": "implementing",
                    "writer": "execution-session",
                    "base_sha": base_sha,
                    "planning_commit": planning_commit,
                }
            )
            data["branch"] = "main"
            data["worktree_path"] = str(repo.resolve())
            self.assertTrue(write_task_manifest_path(task_dir / "task.json", data))

            with patch("common.task_coordination.get_repo_root", return_value=repo):
                self.assertEqual(
                    cmd_block(
                        Namespace(
                            dir="08-16-example",
                            reason="Need a decision",
                            resume_condition="Decision recorded",
                            owner="main",
                        )
                    ),
                    0,
                )
                blocked = load_task_manifest(task_dir)
                self.assertEqual(blocked["coordination"]["writer"], "main")
                self.assertEqual(
                    blocked["coordination"]["block"]["previous_writer"],
                    "execution-session",
                )
                self.assertEqual(
                    cmd_resume(Namespace(dir="08-16-example", writer="execution-session")),
                    0,
                )

            resumed = load_task_manifest(task_dir)
            self.assertEqual(resumed["coordination"]["phase"], "implementing")
            self.assertEqual(resumed["coordination"]["writer"], "execution-session")
            self.assertIsNone(resumed["coordination"]["block"])

    def test_deliver_requires_clean_worktree_and_delivery_report(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            repo, task_dir, base_sha, planning_commit = self._make_repo(raw_root)
            data = load_task_manifest(task_dir)
            data["status"] = "in_progress"
            data["branch"] = "main"
            data["worktree_path"] = str(repo.resolve())
            data["coordination"].update(
                {
                    "route": "delegated",
                    "phase": "implementing",
                    "writer": "execution-session",
                    "base_sha": base_sha,
                    "planning_commit": planning_commit,
                }
            )
            self.assertTrue(write_task_manifest_path(task_dir / "task.json", data))
            (task_dir / "delivery.md").write_text("# Delivery\n", encoding="utf-8")
            self._git(repo, "add", ".trellis/tasks/08-16-example")
            self._git(repo, "commit", "-m", "implementation")

            with patch("common.task_coordination.get_repo_root", return_value=repo):
                self.assertEqual(
                    cmd_deliver(Namespace(dir="08-16-example", reviewer="main")),
                    0,
                )

            delivered = load_task_manifest(task_dir)
            self.assertEqual(delivered["coordination"]["phase"], "delivered")
            self.assertEqual(delivered["coordination"]["writer"], "main")
            with self.assertRaisesRegex(TaskCoordinationError, "requires --writer"):
                mark_started(delivered)
            self.assertTrue(mark_started(delivered, writer="execution-session"))
            self.assertEqual(delivered["coordination"]["phase"], "implementing")
            self.assertEqual(delivered["coordination"]["writer"], "execution-session")

    def test_deliver_rejects_dirty_worktree(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            repo, task_dir, base_sha, planning_commit = self._make_repo(raw_root)
            data = load_task_manifest(task_dir)
            data["status"] = "in_progress"
            data["branch"] = "main"
            data["worktree_path"] = str(repo.resolve())
            data["coordination"].update(
                {
                    "route": "delegated",
                    "phase": "implementing",
                    "writer": "execution-session",
                    "base_sha": base_sha,
                    "planning_commit": planning_commit,
                }
            )
            self.assertTrue(write_task_manifest_path(task_dir / "task.json", data))
            (task_dir / "delivery.md").write_text("# Delivery\n", encoding="utf-8")
            errors = io.StringIO()
            with (
                patch("common.task_coordination.get_repo_root", return_value=repo),
                redirect_stderr(errors),
            ):
                self.assertEqual(
                    cmd_deliver(Namespace(dir="08-16-example", reviewer="main")),
                    1,
                )
            self.assertIn("clean worktree", errors.getvalue())
            self.assertEqual(
                load_task_manifest(task_dir)["coordination"]["phase"],
                "implementing",
            )

    def test_legacy_manifest_accepts_custom_status_without_git_checks(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            repo = Path(raw_root)
            task_dir = repo / ".trellis" / "tasks" / "legacy"
            task_dir.mkdir(parents=True)
            (task_dir / "task.json").write_text(
                json.dumps({"name": "legacy", "status": "waiting_external"}) + "\n",
                encoding="utf-8",
            )
            self.assertEqual(validate_task_manifest(task_dir, repo), [])

    def test_invalid_resume_fails_without_rewriting_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            repo, task_dir, _, _ = self._make_repo(raw_root)
            before = (task_dir / "task.json").read_text(encoding="utf-8")
            errors = io.StringIO()
            with (
                patch("common.task_coordination.get_repo_root", return_value=repo),
                redirect_stderr(errors),
            ):
                self.assertEqual(
                    cmd_resume(Namespace(dir="08-16-example", writer="execution-session")),
                    1,
                )
            self.assertIn("resume requires phase=blocked", errors.getvalue())
            self.assertEqual((task_dir / "task.json").read_text(encoding="utf-8"), before)


if __name__ == "__main__":
    unittest.main()
