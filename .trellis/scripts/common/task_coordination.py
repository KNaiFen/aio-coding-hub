"""Deterministic coordination state for Trellis task handoffs."""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Callable

from .active_task import resolve_active_task
from .git import run_git
from .log import Colors, colored
from .paths import FILE_TASK_JSON, get_repo_root
from .task_utils import resolve_task_dir


COORDINATION_VERSION = 1
ROUTES = {"main", "delegated"}
PHASES = {"planning", "ready", "implementing", "blocked", "delivered", "completed"}
STATUS_RE = re.compile(r"^[A-Za-z0-9_-]+$")
SHA_RE = re.compile(r"^[0-9a-fA-F]{40}$")


class TaskCoordinationError(ValueError):
    """Raised when a task manifest or coordination transition is invalid."""


def utc_now() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def new_coordination(*, writer: str = "main") -> dict[str, Any]:
    return {
        "version": COORDINATION_VERSION,
        "route": "main",
        "phase": "planning",
        "writer": writer,
        "base_sha": None,
        "planning_commit": None,
        "block": None,
        "updated_at": utc_now(),
    }


def load_task_manifest(task_dir: Path) -> dict[str, Any]:
    path = task_dir / FILE_TASK_JSON
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as error:
        raise TaskCoordinationError(f"task.json not found: {path}") from error
    except (OSError, json.JSONDecodeError) as error:
        raise TaskCoordinationError(f"task.json is unreadable or invalid: {path}: {error}") from error
    if not isinstance(data, dict):
        raise TaskCoordinationError(f"task.json root must be an object: {path}")
    return data


def write_task_manifest_path(path: Path, data: dict[str, Any]) -> bool:
    """Atomically replace a task manifest while preserving unknown fields."""
    temp_name: str | None = None
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.NamedTemporaryFile(
            mode="w",
            encoding="utf-8",
            dir=path.parent,
            prefix=f".{path.name}.",
            suffix=".tmp",
            delete=False,
        ) as handle:
            temp_name = handle.name
            json.dump(data, handle, indent=2, ensure_ascii=False)
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temp_name, path)
        return True
    except OSError:
        if temp_name:
            try:
                Path(temp_name).unlink(missing_ok=True)
            except OSError:
                pass
        return False


def mutate_task_manifest(
    task_dir: Path,
    mutator: Callable[[dict[str, Any]], None],
) -> dict[str, Any]:
    data = load_task_manifest(task_dir)
    mutator(data)
    path = task_dir / FILE_TASK_JSON
    if not write_task_manifest_path(path, data):
        raise TaskCoordinationError(f"failed to write task.json: {path}")
    return data


def mark_started(data: dict[str, Any], *, writer: str | None = None) -> bool:
    """Apply the task start transition in one manifest write."""
    changed = False
    if data.get("status") == "planning":
        data["status"] = "in_progress"
        changed = True

    coordination = data.get("coordination")
    if isinstance(coordination, dict) and coordination.get("version") == COORDINATION_VERSION:
        phase = coordination.get("phase")
        if phase == "delivered" and not (writer and writer.strip()):
            raise TaskCoordinationError("restarting a delivered task requires --writer")
        if phase in {"ready", "delivered"}:
            coordination["phase"] = "implementing"
            if writer and writer.strip():
                coordination["writer"] = writer.strip()
            changed = True
        if changed:
            coordination["updated_at"] = utc_now()
    return changed


def mark_completed(data: dict[str, Any]) -> None:
    coordination = data.get("coordination")
    if isinstance(coordination, dict) and coordination.get("version") == COORDINATION_VERSION:
        coordination["phase"] = "completed"
        coordination["block"] = None
        coordination["updated_at"] = utc_now()


def _resolve_task(task_input: str | None, repo_root: Path) -> Path:
    if task_input:
        task_dir = resolve_task_dir(task_input, repo_root)
    else:
        active = resolve_active_task(repo_root)
        if not active.task_path or active.stale:
            raise TaskCoordinationError(
                "no live current task; pass an explicit task directory"
            )
        task_dir = resolve_task_dir(active.task_path, repo_root)
    if not task_dir.is_dir():
        raise TaskCoordinationError(f"task directory not found: {task_input or task_dir}")
    return task_dir


def _display_path(path: Path, repo_root: Path) -> str:
    try:
        return path.relative_to(repo_root).as_posix()
    except ValueError:
        return str(path)


def _coordination(data: dict[str, Any], *, required: bool) -> dict[str, Any] | None:
    value = data.get("coordination")
    if value is None and not required:
        return None
    if not isinstance(value, dict):
        raise TaskCoordinationError("coordination must be an object")
    return value


def _required_text(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise TaskCoordinationError(f"{label} must be a non-empty string")
    return value.strip()


def _git_value(cwd: Path, args: list[str], label: str) -> str:
    code, stdout, stderr = run_git(args, cwd=cwd)
    if code != 0:
        detail = stderr.strip() or stdout.strip() or f"git {' '.join(args)} failed"
        raise TaskCoordinationError(f"{label}: {detail}")
    return stdout.strip()


def _validate_git_registration(
    repo_root: Path,
    *,
    worktree: str,
    branch: str,
    base_sha: str,
    planning_commit: str,
) -> list[str]:
    errors: list[str] = []
    if not Path(worktree).is_absolute():
        errors.append("worktree_path must be absolute")
    else:
        try:
            if Path(worktree).resolve() != repo_root.resolve():
                errors.append(
                    f"registered worktree does not match current repo: {worktree} != {repo_root.resolve()}"
                )
        except OSError as error:
            errors.append(f"cannot resolve worktree_path: {error}")

    if not branch.strip():
        errors.append("branch is required")
    for label, value in (("base_sha", base_sha), ("planning_commit", planning_commit)):
        if not SHA_RE.fullmatch(value or ""):
            errors.append(f"{label} must be a full 40-character SHA")

    if errors:
        return errors

    try:
        actual_branch = _git_value(repo_root, ["branch", "--show-current"], "read branch")
        if actual_branch != branch:
            errors.append(f"branch mismatch: registered {branch}, actual {actual_branch}")
        for label, value in (("base_sha", base_sha), ("planning_commit", planning_commit)):
            _git_value(repo_root, ["cat-file", "-e", f"{value}^{{commit}}"], f"resolve {label}")
        merge_base = _git_value(repo_root, ["merge-base", base_sha, "HEAD"], "resolve base merge-base")
        if merge_base != base_sha:
            errors.append(f"base_sha is not the current HEAD merge-base: {merge_base}")
        planning_base = _git_value(
            repo_root,
            ["merge-base", planning_commit, "HEAD"],
            "resolve planning commit ancestry",
        )
        if planning_base != planning_commit:
            errors.append("planning_commit is not an ancestor of HEAD")
    except TaskCoordinationError as error:
        errors.append(str(error))
    return errors


def validate_task_manifest(task_dir: Path, repo_root: Path) -> list[str]:
    try:
        data = load_task_manifest(task_dir)
    except TaskCoordinationError as error:
        return [str(error)]

    errors: list[str] = []
    status = data.get("status")
    if not isinstance(status, str) or not STATUS_RE.fullmatch(status):
        errors.append("status must match [A-Za-z0-9_-]+")

    try:
        coordination = _coordination(data, required=False)
    except TaskCoordinationError as error:
        errors.append(str(error))
        return errors
    if coordination is None:
        return errors

    if coordination.get("version") != COORDINATION_VERSION:
        errors.append(f"coordination.version must be {COORDINATION_VERSION}")
    route = coordination.get("route")
    phase = coordination.get("phase")
    writer = coordination.get("writer")
    if route not in ROUTES:
        errors.append(f"coordination.route must be one of: {', '.join(sorted(ROUTES))}")
    if phase not in PHASES:
        errors.append(f"coordination.phase must be one of: {', '.join(sorted(PHASES))}")
    if not isinstance(writer, str) or not writer.strip():
        errors.append("coordination.writer must be a non-empty string")

    block = coordination.get("block")
    if phase == "blocked":
        if not isinstance(block, dict):
            errors.append("blocked phase requires coordination.block")
        else:
            for field in (
                "reason",
                "resume_condition",
                "owner",
                "blocked_at",
                "previous_phase",
                "previous_writer",
            ):
                if not isinstance(block.get(field), str) or not block[field].strip():
                    errors.append(f"coordination.block.{field} must be a non-empty string")
    elif block is not None:
        errors.append("coordination.block must be null outside the blocked phase")

    if route != "delegated":
        return errors

    if not (task_dir / "execution.md").is_file():
        errors.append("delegated task requires execution.md")
    branch = data.get("branch")
    worktree = data.get("worktree_path")
    base_sha = coordination.get("base_sha")
    planning_commit = coordination.get("planning_commit")
    if not all(isinstance(value, str) for value in (branch, worktree, base_sha, planning_commit)):
        errors.append("delegated task requires branch, worktree_path, base_sha, and planning_commit")
    else:
        errors.extend(
            _validate_git_registration(
                repo_root,
                worktree=worktree,
                branch=branch,
                base_sha=base_sha,
                planning_commit=planning_commit,
            )
        )

    if phase == "ready" and status != "planning":
        errors.append("ready phase requires top-level status=planning")
    if phase in {"implementing", "blocked", "delivered"} and status != "in_progress":
        errors.append(f"{phase} phase requires top-level status=in_progress")
    if phase == "completed" and status != "completed":
        errors.append("completed phase requires top-level status=completed")
    return errors


def _state_view(task_dir: Path, repo_root: Path, data: dict[str, Any]) -> dict[str, Any]:
    coordination = data.get("coordination") if isinstance(data.get("coordination"), dict) else {}
    return {
        "task": _display_path(task_dir, repo_root),
        "title": data.get("title") or data.get("name") or task_dir.name,
        "status": data.get("status"),
        "route": coordination.get("route", "legacy"),
        "phase": coordination.get("phase", "legacy"),
        "writer": coordination.get("writer"),
        "branch": data.get("branch"),
        "base_branch": data.get("base_branch"),
        "worktree_path": data.get("worktree_path"),
        "base_sha": coordination.get("base_sha"),
        "planning_commit": coordination.get("planning_commit"),
        "candidate_commit": data.get("commit"),
        "pr_url": data.get("pr_url"),
        "block": coordination.get("block"),
        "updated_at": coordination.get("updated_at"),
    }


def cmd_status(args: argparse.Namespace) -> int:
    repo_root = get_repo_root()
    try:
        task_dir = _resolve_task(getattr(args, "dir", None), repo_root)
        view = _state_view(task_dir, repo_root, load_task_manifest(task_dir))
    except TaskCoordinationError as error:
        print(colored(f"Error: {error}", Colors.RED), file=sys.stderr)
        return 1

    if getattr(args, "json", False):
        print(json.dumps(view, indent=2, ensure_ascii=False))
        return 0
    labels = (
        ("Task", "task"),
        ("Status", "status"),
        ("Route", "route"),
        ("Phase", "phase"),
        ("Writer", "writer"),
        ("Branch", "branch"),
        ("Base branch", "base_branch"),
        ("Worktree", "worktree_path"),
        ("Base SHA", "base_sha"),
        ("Planning commit", "planning_commit"),
        ("Candidate", "candidate_commit"),
        ("PR", "pr_url"),
    )
    for label, key in labels:
        print(f"{label}: {view[key] if view[key] is not None else '(none)'}")
    if view["block"]:
        print(f"Block: {view['block']['reason']}")
        print(f"Resume condition: {view['block']['resume_condition']}")
    return 0


def cmd_doctor(args: argparse.Namespace) -> int:
    repo_root = get_repo_root()
    try:
        task_dir = _resolve_task(getattr(args, "dir", None), repo_root)
    except TaskCoordinationError as error:
        print(colored(f"Error: {error}", Colors.RED), file=sys.stderr)
        return 1
    errors = validate_task_manifest(task_dir, repo_root)
    if errors:
        for error in errors:
            print(colored(f"Error: {error}", Colors.RED), file=sys.stderr)
        return 1
    print(colored(f"✓ Task coordination is valid: {_display_path(task_dir, repo_root)}", Colors.GREEN))
    return 0


def cmd_delegate(args: argparse.Namespace) -> int:
    repo_root = get_repo_root()
    try:
        task_dir = _resolve_task(args.dir, repo_root)
        data = load_task_manifest(task_dir)
        if data.get("status") != "planning":
            raise TaskCoordinationError("delegate requires top-level status=planning")
        if not (task_dir / "execution.md").is_file():
            raise TaskCoordinationError("delegate requires execution.md")
        writer = _required_text(args.writer, "writer")
        registration_errors = _validate_git_registration(
            repo_root,
            worktree=args.worktree,
            branch=args.branch,
            base_sha=args.base_sha,
            planning_commit=args.planning_commit,
        )
        if registration_errors:
            raise TaskCoordinationError("; ".join(registration_errors))

        def apply_delegate(manifest: dict[str, Any]) -> None:
            manifest["branch"] = args.branch
            manifest["worktree_path"] = str(Path(args.worktree).resolve())
            manifest["coordination"] = {
                "version": COORDINATION_VERSION,
                "route": "delegated",
                "phase": "ready",
                "writer": writer,
                "base_sha": args.base_sha.lower(),
                "planning_commit": args.planning_commit.lower(),
                "block": None,
                "updated_at": utc_now(),
            }

        mutate_task_manifest(task_dir, apply_delegate)
    except TaskCoordinationError as error:
        print(colored(f"Error: {error}", Colors.RED), file=sys.stderr)
        return 1
    print(colored("✓ Delegated task registration written", Colors.GREEN))
    print("Next: commit task.json, run task.py start, commit the start transition, then run task.py handoff.")
    return 0


def _handoff_view(task_dir: Path, repo_root: Path, data: dict[str, Any]) -> dict[str, Any]:
    view = _state_view(task_dir, repo_root, data)
    view["execution"] = f"{view['task']}/execution.md"
    view["preflight"] = [
        f"cd {view['worktree_path']}",
        f"python3 .trellis/scripts/task.py status {view['task']}",
        f"python3 .trellis/scripts/task.py doctor {view['task']}",
    ]
    view["prompt"] = (
        f"在 {view['worktree_path']} 开始独立执行 session。先运行 `python3 "
        f".trellis/scripts/task.py status {view['task']}` 和 `python3 .trellis/scripts/task.py "
        f"doctor {view['task']}`，然后按 `{view['execution']}` 施工。提交并推送任务分支、"
        f"维护 PR/CI、填写并提交 delivery.md，再运行 `python3 .trellis/scripts/task.py deliver "
        f"{view['task']}`、提交状态转换并等待最终 head CI 后暂停；不要合并、归档或清理 worktree。"
    )
    return view


def cmd_handoff(args: argparse.Namespace) -> int:
    repo_root = get_repo_root()
    try:
        task_dir = _resolve_task(getattr(args, "dir", None), repo_root)
        data = load_task_manifest(task_dir)
        errors = validate_task_manifest(task_dir, repo_root)
        if errors:
            raise TaskCoordinationError("; ".join(errors))
        coordination = _coordination(data, required=True)
        if coordination.get("route") != "delegated" or coordination.get("phase") != "implementing":
            raise TaskCoordinationError("handoff requires delegated phase=implementing")
        dirty = _git_value(repo_root, ["status", "--porcelain"], "read worktree status")
        if dirty:
            raise TaskCoordinationError("handoff requires a clean worktree; commit the state transition first")
        view = _handoff_view(task_dir, repo_root, data)
    except TaskCoordinationError as error:
        print(colored(f"Error: {error}", Colors.RED), file=sys.stderr)
        return 1

    if getattr(args, "json", False):
        print(json.dumps(view, indent=2, ensure_ascii=False))
        return 0
    print("# Execution handoff")
    print()
    for label, key in (
        ("Task", "task"),
        ("Execution entry", "execution"),
        ("Worktree", "worktree_path"),
        ("Branch", "branch"),
        ("Base SHA", "base_sha"),
        ("Planning commit", "planning_commit"),
        ("Writer", "writer"),
    ):
        print(f"- {label}: `{view[key]}`")
    print("- Preflight:")
    for command in view["preflight"]:
        print(f"  - `{command}`")
    print()
    print("## Prompt")
    print()
    print(view["prompt"])
    return 0


def cmd_deliver(args: argparse.Namespace) -> int:
    repo_root = get_repo_root()
    try:
        task_dir = _resolve_task(getattr(args, "dir", None), repo_root)
        data = load_task_manifest(task_dir)
        errors = validate_task_manifest(task_dir, repo_root)
        if errors:
            raise TaskCoordinationError("; ".join(errors))
        coordination = _coordination(data, required=True)
        if coordination.get("route") != "delegated" or coordination.get("phase") != "implementing":
            raise TaskCoordinationError("deliver requires delegated phase=implementing")
        reviewer = _required_text(getattr(args, "reviewer", "main"), "reviewer")
        if not (task_dir / "delivery.md").is_file():
            raise TaskCoordinationError("deliver requires delivery.md")
        dirty = _git_value(repo_root, ["status", "--porcelain"], "read worktree status")
        if dirty:
            raise TaskCoordinationError(
                "deliver requires a clean worktree; commit the implementation and delivery report first"
            )

        def apply_delivery(manifest: dict[str, Any]) -> None:
            state = _coordination(manifest, required=True)
            state["phase"] = "delivered"
            state["writer"] = reviewer
            state["updated_at"] = utc_now()

        mutate_task_manifest(task_dir, apply_delivery)
    except TaskCoordinationError as error:
        print(colored(f"Error: {error}", Colors.RED), file=sys.stderr)
        return 1
    print(colored("✓ Task marked delivered", Colors.GREEN))
    print("Next: commit and push task.json, wait for required CI on that head, then pause for main review.")
    return 0


def cmd_block(args: argparse.Namespace) -> int:
    repo_root = get_repo_root()
    try:
        task_dir = _resolve_task(args.dir, repo_root)
        reason = _required_text(args.reason, "reason")
        resume_condition = _required_text(args.resume_condition, "resume_condition")
        owner = _required_text(args.owner, "owner")

        def apply_block(data: dict[str, Any]) -> None:
            coordination = _coordination(data, required=True)
            phase = coordination.get("phase")
            if coordination.get("version") != COORDINATION_VERSION:
                raise TaskCoordinationError("block requires coordination.version=1")
            if phase not in {"ready", "implementing", "delivered"}:
                raise TaskCoordinationError(f"cannot block task from phase={phase}")
            coordination["phase"] = "blocked"
            previous_writer = _required_text(coordination.get("writer"), "current writer")
            coordination["block"] = {
                "reason": reason,
                "resume_condition": resume_condition,
                "owner": owner,
                "blocked_at": utc_now(),
                "previous_phase": phase,
                "previous_writer": previous_writer,
            }
            coordination["writer"] = owner
            coordination["updated_at"] = utc_now()

        mutate_task_manifest(task_dir, apply_block)
    except TaskCoordinationError as error:
        print(colored(f"Error: {error}", Colors.RED), file=sys.stderr)
        return 1
    print(colored("✓ Task marked blocked", Colors.GREEN))
    return 0


def cmd_resume(args: argparse.Namespace) -> int:
    repo_root = get_repo_root()
    try:
        task_dir = _resolve_task(args.dir, repo_root)
        writer = _required_text(args.writer, "writer")

        def apply_resume(data: dict[str, Any]) -> None:
            coordination = _coordination(data, required=True)
            block = coordination.get("block")
            if coordination.get("version") != COORDINATION_VERSION:
                raise TaskCoordinationError("resume requires coordination.version=1")
            if coordination.get("phase") != "blocked" or not isinstance(block, dict):
                raise TaskCoordinationError("resume requires phase=blocked with blocker details")
            previous_phase = block.get("previous_phase")
            if previous_phase not in {"ready", "implementing", "delivered"}:
                raise TaskCoordinationError("blocked state has an invalid previous_phase")
            coordination["phase"] = previous_phase
            coordination["writer"] = writer
            coordination["block"] = None
            coordination["updated_at"] = utc_now()

        mutate_task_manifest(task_dir, apply_resume)
    except TaskCoordinationError as error:
        print(colored(f"Error: {error}", Colors.RED), file=sys.stderr)
        return 1
    print(colored("✓ Task resumed", Colors.GREEN))
    return 0
