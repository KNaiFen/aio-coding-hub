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

from common.task_acceptance import (
    GITHUB_REPO,
    TaskAcceptanceError,
    _checks_command,
    _merge_command,
    _merge_fixed_head,
    _merged_view_command,
    _pr_view_command,
    _required_contexts,
    _required_checks_command,
    _rules_command,
    _resolve_candidate_task,
    _run_command,
    _validate_candidate,
    _validate_main_checkout,
    _validate_named_checks,
    _validate_pr_metadata,
    _validate_required_checks,
    cmd_accept,
)


class TaskAcceptanceTests(unittest.TestCase):
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

    def _make_candidate_repo(
        self,
        raw_root: str,
    ) -> tuple[Path, Path, str, Path]:
        root = Path(raw_root)
        remote = root / "remote.git"
        main = root / "main"
        candidate = root / "candidate"
        subprocess.run(
            ["git", "init", "--bare", str(remote)],
            check=True,
            capture_output=True,
            text=True,
            encoding="utf-8",
        )
        main.mkdir()
        self._git(main, "init", "-b", "main")
        self._git(main, "config", "user.name", "Trellis Test")
        self._git(main, "config", "user.email", "trellis@example.invalid")
        (main / "README.md").write_text("base\n", encoding="utf-8")
        self._git(main, "add", "README.md")
        self._git(main, "commit", "-m", "base")
        base_sha = self._git(main, "rev-parse", "HEAD")
        self._git(main, "remote", "add", "origin", str(remote))
        self._git(main, "push", "-u", "origin", "main")
        self._git(main, "worktree", "add", "-b", "task/example", str(candidate))

        task_dir = candidate / ".trellis" / "tasks" / "08-16-example"
        task_dir.mkdir(parents=True)
        (task_dir / "execution.md").write_text("# Execution\n", encoding="utf-8")
        (task_dir / "delivery.md").write_text("# Delivery\n", encoding="utf-8")
        (task_dir / "task.json").write_text(
            json.dumps(
                {
                    "name": "example",
                    "status": "in_progress",
                    "base_branch": "main",
                    "branch": "task/example",
                    "worktree_path": str(candidate.resolve()),
                    "coordination": {
                        "version": 1,
                        "route": "delegated",
                        "phase": "delivered",
                        "writer": "main",
                        "base_sha": base_sha,
                        "planning_commit": base_sha,
                        "block": None,
                        "updated_at": "2026-08-16T00:00:00Z",
                    },
                },
                indent=2,
            )
            + "\n",
            encoding="utf-8",
        )
        marker = root / "candidate-code-ran"
        malicious_script = candidate / ".trellis" / "scripts" / "task.py"
        malicious_script.parent.mkdir(parents=True)
        malicious_script.write_text(
            "from pathlib import Path\n"
            f"Path({str(marker)!r}).write_text('unsafe', encoding='utf-8')\n",
            encoding="utf-8",
        )
        self._git(candidate, "add", ".trellis")
        self._git(candidate, "commit", "-m", "delivered candidate")
        return main, candidate, self._git(candidate, "rev-parse", "HEAD"), marker

    def _metadata(self, head: str) -> dict[str, object]:
        return {
            "state": "OPEN",
            "isDraft": False,
            "baseRefName": "main",
            "headRefName": "task/example",
            "headRefOid": head,
            "headRepository": {"nameWithOwner": GITHUB_REPO},
            "isCrossRepository": False,
            "mergeable": "MERGEABLE",
            "mergeStateStatus": "CLEAN",
            "reviewDecision": "",
            "autoMergeRequest": None,
        }

    def _checks(self) -> list[dict[str, str]]:
        return [
            {
                "name": name,
                "state": "SUCCESS",
                "bucket": "pass",
                "event": "pull_request",
                "link": f"https://example.invalid/{name}",
            }
            for name in ("ci-gate", "pr-title")
        ]

    def _rules(self) -> list[dict[str, object]]:
        return [
            {
                "type": "required_status_checks",
                "parameters": {
                    "strict_required_status_checks_policy": True,
                    "required_status_checks": [
                        {"context": "ci-gate", "integration_id": 15368},
                        {"context": "pr-title", "integration_id": 15368},
                    ],
                },
            }
        ]

    def _open_merge_state(self, head: str) -> str:
        return json.dumps(
            {
                "state": "OPEN",
                "headRefOid": head,
                "mergeCommit": None,
                "mergedAt": None,
            }
        )

    def test_merge_command_uses_fixed_head_rest_squash(self) -> None:
        head = "a" * 40
        command = _merge_command(12, head)
        self.assertEqual(
            command,
            [
                "gh",
                "api",
                "--method",
                "PUT",
                f"repos/{GITHUB_REPO}/pulls/12/merge",
                "-f",
                f"sha={head}",
                "-f",
                "merge_method=squash",
            ],
        )
        self.assertNotEqual(command[:3], ["gh", "pr", "merge"])
        for prohibited in ("--admin", "--auto", "--delete-branch"):
            self.assertNotIn(prohibited, command)

    def test_metadata_and_required_checks_fail_closed(self) -> None:
        head = "b" * 40
        metadata = self._metadata(head)
        _validate_pr_metadata(metadata, branch="task/example", head=head)
        _validate_named_checks(self._checks())
        contexts = _required_contexts(self._rules())
        _validate_required_checks(self._checks(), contexts)

        for field, value in (
            ("headRefOid", "c" * 40),
            ("isDraft", True),
            ("mergeStateStatus", "UNSTABLE"),
            ("autoMergeRequest", {"enabledAt": "now"}),
            ("isCrossRepository", True),
            ("headRepository", {"nameWithOwner": "other/repo"}),
        ):
            with self.subTest(field=field):
                invalid = dict(metadata)
                invalid[field] = value
                with self.assertRaises(TaskAcceptanceError):
                    _validate_pr_metadata(invalid, branch="task/example", head=head)

        with self.assertRaisesRegex(TaskAcceptanceError, "missing pr-title"):
            _validate_named_checks(self._checks()[:1])

        unrelated_failure = self._checks() + [
            {
                "name": "optional-job",
                "state": "FAILURE",
                "bucket": "fail",
                "event": "pull_request",
                "link": "https://example.invalid/optional-job",
            }
        ]
        _validate_named_checks(unrelated_failure)
        with self.assertRaisesRegex(TaskAcceptanceError, "optional-job"):
            _validate_required_checks(unrelated_failure, contexts | {"optional-job"})

        with self.assertRaisesRegex(TaskAcceptanceError, "missing future-required"):
            _validate_required_checks(self._checks(), contexts | {"future-required"})

    def test_task_path_rejects_escape_archive_and_symlink(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            worktree = Path(raw_root)
            task = worktree / ".trellis" / "tasks" / "live"
            task.mkdir(parents=True)
            self.assertEqual(
                _resolve_candidate_task(worktree, ".trellis/tasks/live"),
                task,
            )
            for invalid in (
                ".trellis/tasks/../outside",
                ".trellis/tasks/archive",
                "/tmp/task",
                "live",
            ):
                with self.subTest(path=invalid), self.assertRaises(TaskAcceptanceError):
                    _resolve_candidate_task(worktree, invalid)

            target = worktree / "target"
            target.mkdir()
            (worktree / ".trellis" / "tasks" / "linked").symlink_to(
                target,
                target_is_directory=True,
            )
            with self.assertRaisesRegex(TaskAcceptanceError, "symlinks"):
                _resolve_candidate_task(worktree, ".trellis/tasks/linked")

    def test_main_checkout_must_match_fetched_origin_main(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            main, _, _, _ = self._make_candidate_repo(raw_root)
            (main / "local-only.txt").write_text("ahead\n", encoding="utf-8")
            self._git(main, "add", "local-only.txt")
            self._git(main, "commit", "-m", "local ahead")
            with self.assertRaisesRegex(TaskAcceptanceError, "not synchronized"):
                _validate_main_checkout(main, fetch=True)

    def test_accept_rechecks_and_never_executes_candidate_code(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            main, candidate, head, marker = self._make_candidate_repo(raw_root)
            metadata = json.dumps(self._metadata(head))
            checks = json.dumps(self._checks())
            merge_commit = "d" * 40
            outputs = [
                self._open_merge_state(head),
                metadata,
                checks,
                json.dumps(self._rules()),
                checks,
                metadata,
                checks,
                json.dumps(self._rules()),
                checks,
                json.dumps({"merged": True, "sha": merge_commit, "message": "merged"}),
            ]
            calls: list[list[str]] = []

            def fake_run(command: list[str], *, cwd: Path) -> str:
                calls.append(command)
                if command[:2] == ["git", "fetch"]:
                    return _run_command(command, cwd=cwd)
                self.assertEqual(cwd, main.resolve())
                return outputs.pop(0)

            output = io.StringIO()
            with (
                patch("common.task_acceptance.get_repo_root", return_value=main),
                patch("common.task_acceptance._run_command", side_effect=fake_run),
                redirect_stdout(output),
            ):
                self.assertEqual(
                    cmd_accept(
                        Namespace(
                            dir=".trellis/tasks/08-16-example",
                            worktree=str(candidate),
                            pr=12,
                            head=head,
                        )
                    ),
                    0,
                )

            self.assertEqual(calls.count(_pr_view_command(12)), 2)
            self.assertEqual(calls.count(_checks_command(12)), 2)
            self.assertEqual(calls.count(_rules_command()), 2)
            self.assertEqual(calls.count(_required_checks_command(12)), 2)
            self.assertEqual(calls.count(_merge_command(12, head)), 1)
            self.assertEqual(outputs, [])
            self.assertFalse(marker.exists())
            self.assertIn(merge_commit, output.getvalue())

    def test_final_candidate_drift_stops_before_merge(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            main, candidate, head, _ = self._make_candidate_repo(raw_root)
            calls: list[list[str]] = []

            def fake_run(command: list[str], *, cwd: Path) -> str:
                calls.append(command)
                if command[:2] == ["git", "fetch"]:
                    return _run_command(command, cwd=cwd)
                if command == _merged_view_command(12):
                    return self._open_merge_state(head)
                if command == _pr_view_command(12):
                    return json.dumps(self._metadata(head))
                if command == _checks_command(12):
                    (candidate / "README.md").write_text("drift\n", encoding="utf-8")
                    return json.dumps(self._checks())
                if command == _rules_command():
                    return json.dumps(self._rules())
                if command == _required_checks_command(12):
                    return json.dumps(self._checks())
                self.fail(f"unexpected command: {command}")

            errors = io.StringIO()
            with (
                patch("common.task_acceptance.get_repo_root", return_value=main),
                patch("common.task_acceptance._run_command", side_effect=fake_run),
                redirect_stderr(errors),
            ):
                self.assertEqual(
                    cmd_accept(
                        Namespace(
                            dir=".trellis/tasks/08-16-example",
                            worktree=str(candidate),
                            pr=12,
                            head=head,
                        )
                    ),
                    1,
                )
            self.assertNotIn(_merge_command(12, head), calls)
            self.assertIn("clean candidate worktree", errors.getvalue())

    def test_manifest_is_read_from_fixed_head_not_skip_worktree_file(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            main, candidate, _, _ = self._make_candidate_repo(raw_root)
            task_path = ".trellis/tasks/08-16-example/task.json"
            manifest_path = candidate / task_path
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            manifest["coordination"]["phase"] = "implementing"
            manifest["coordination"]["writer"] = "execution-session"
            manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
            self._git(candidate, "add", task_path)
            self._git(candidate, "commit", "-m", "not delivered")
            head = self._git(candidate, "rev-parse", "HEAD")

            self._git(candidate, "update-index", "--skip-worktree", task_path)
            manifest["coordination"]["phase"] = "delivered"
            manifest["coordination"]["writer"] = "main"
            manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
            self.assertEqual(self._git(candidate, "status", "--porcelain"), "")

            with self.assertRaisesRegex(TaskAcceptanceError, "phase='implementing'"):
                _validate_candidate(
                    main_root=main.resolve(),
                    worktree_arg=str(candidate),
                    task_ref=".trellis/tasks/08-16-example",
                    head=head,
                )

    def test_already_merged_head_is_idempotent_when_main_is_behind(self) -> None:
        with tempfile.TemporaryDirectory() as raw_root:
            main, candidate, head, _ = self._make_candidate_repo(raw_root)
            updater = Path(raw_root) / "updater"
            subprocess.run(
                ["git", "clone", "--branch", "main", str(Path(raw_root) / "remote.git"), str(updater)],
                check=True,
                capture_output=True,
                text=True,
                encoding="utf-8",
            )
            self._git(updater, "config", "user.name", "Trellis Test")
            self._git(updater, "config", "user.email", "trellis@example.invalid")
            (updater / "merged.txt").write_text("merged\n", encoding="utf-8")
            self._git(updater, "add", "merged.txt")
            self._git(updater, "commit", "-m", "advance main")
            self._git(updater, "push", "origin", "main")
            merge_commit = self._git(updater, "rev-parse", "HEAD")

            def fake_run(command: list[str], *, cwd: Path) -> str:
                if command[:2] == ["git", "fetch"]:
                    return _run_command(command, cwd=cwd)
                if command == _merged_view_command(12):
                    return json.dumps(
                        {
                            "state": "MERGED",
                            "headRefOid": head,
                            "mergeCommit": {"oid": merge_commit},
                            "mergedAt": "2026-08-16T00:00:00Z",
                        }
                    )
                self.fail(f"unexpected command: {command}")

            output = io.StringIO()
            with (
                patch("common.task_acceptance.get_repo_root", return_value=main),
                patch("common.task_acceptance._run_command", side_effect=fake_run),
                redirect_stdout(output),
            ):
                self.assertEqual(
                    cmd_accept(
                        Namespace(
                            dir=".trellis/tasks/08-16-example",
                            worktree=str(candidate),
                            pr=12,
                            head=head,
                        )
                    ),
                    0,
                )
            self.assertIn("already merged", output.getvalue())

    def test_merge_timeout_recovers_only_for_exact_merged_head(self) -> None:
        head = "e" * 40
        merge_commit = "f" * 40
        calls: list[list[str]] = []

        confirmations = [
            self._open_merge_state(head),
            json.dumps(
                {
                    "state": "MERGED",
                    "headRefOid": head,
                    "mergeCommit": {"oid": merge_commit},
                    "mergedAt": "2026-08-16T00:00:00Z",
                }
            ),
        ]

        def fake_run(command: list[str], *, cwd: Path) -> str:
            calls.append(command)
            if command == _merge_command(12, head):
                raise TaskAcceptanceError("command timed out")
            if command == _merged_view_command(12):
                return confirmations.pop(0)
            self.fail(f"unexpected command: {command}")

        with (
            patch("common.task_acceptance._run_command", side_effect=fake_run),
            patch("common.task_acceptance.time.sleep"),
        ):
            self.assertEqual(
                _merge_fixed_head(cwd=Path("/tmp"), pr=12, head=head),
                merge_commit,
            )
        self.assertEqual(
            calls,
            [_merge_command(12, head), _merged_view_command(12), _merged_view_command(12)],
        )

    def test_merge_failure_reports_exact_head_still_open(self) -> None:
        head = "a" * 40

        def fake_run(command: list[str], *, cwd: Path) -> str:
            if command == _merge_command(12, head):
                raise TaskAcceptanceError("HTTP 409")
            if command == _merged_view_command(12):
                return json.dumps(
                    {
                        "state": "OPEN",
                        "headRefOid": head,
                        "mergeCommit": None,
                        "mergedAt": None,
                    }
                )
            self.fail(f"unexpected command: {command}")

        with (
            patch("common.task_acceptance._run_command", side_effect=fake_run),
            patch("common.task_acceptance.time.sleep"),
            self.assertRaisesRegex(TaskAcceptanceError, "PR remains open"),
        ):
            _merge_fixed_head(cwd=Path("/tmp"), pr=12, head=head)


if __name__ == "__main__":
    unittest.main()
