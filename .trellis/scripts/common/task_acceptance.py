"""Fail-closed fixed-head acceptance merge from a trusted main checkout."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

from .git import run_git
from .log import Colors, colored
from .paths import get_repo_root


GITHUB_REPO = "KNaiFen/aio-coding-hub"
BASE_BRANCH = "main"
REQUIRED_CHECKS = {"ci-gate", "pr-title"}
SHA_RE = re.compile(r"^[0-9a-f]{40}$")
MERGE_CONFIRM_ATTEMPTS = 5
MERGE_CONFIRM_INTERVAL_SECONDS = 2


class TaskAcceptanceError(ValueError):
    """Raised when a candidate cannot be accepted and merged safely."""


def _git_value(cwd: Path, args: list[str], label: str) -> str:
    code, stdout, stderr = run_git(args, cwd=cwd)
    if code != 0:
        detail = stderr.strip() or stdout.strip() or f"git {' '.join(args)} failed"
        raise TaskAcceptanceError(f"{label}: {detail}")
    return stdout.strip()


def _run_command(command: list[str], *, cwd: Path) -> str:
    try:
        result = subprocess.run(
            command,
            cwd=cwd,
            check=False,
            capture_output=True,
            text=True,
            encoding="utf-8",
            timeout=60,
        )
    except subprocess.TimeoutExpired as error:
        raise TaskAcceptanceError(f"command timed out: {' '.join(command)}") from error
    except OSError as error:
        raise TaskAcceptanceError(f"cannot run {' '.join(command)}: {error}") from error
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip() or "command failed"
        raise TaskAcceptanceError(f"{' '.join(command)}: {detail}")
    return result.stdout.strip()


def _run_json(command: list[str], *, cwd: Path) -> Any:
    output = _run_command(command, cwd=cwd)
    try:
        return json.loads(output)
    except json.JSONDecodeError as error:
        raise TaskAcceptanceError(
            f"{' '.join(command)} returned invalid JSON: {error}"
        ) from error


def _pr_view_command(pr: int) -> list[str]:
    return [
        "gh",
        "pr",
        "view",
        str(pr),
        "-R",
        GITHUB_REPO,
        "--json",
        (
            "state,isDraft,baseRefName,headRefName,headRefOid,headRepository,"
            "isCrossRepository,mergeable,mergeStateStatus,reviewDecision,autoMergeRequest"
        ),
    ]


def _checks_command(pr: int) -> list[str]:
    return [
        "gh",
        "pr",
        "checks",
        str(pr),
        "-R",
        GITHUB_REPO,
        "--json",
        "name,state,bucket,event,link",
    ]


def _required_checks_command(pr: int) -> list[str]:
    command = _checks_command(pr)
    command.insert(command.index("--json"), "--required")
    return command


def _rules_command() -> list[str]:
    return ["gh", "api", f"repos/{GITHUB_REPO}/rules/branches/{BASE_BRANCH}"]


def _merge_command(pr: int, head: str) -> list[str]:
    return [
        "gh",
        "api",
        "--method",
        "PUT",
        f"repos/{GITHUB_REPO}/pulls/{pr}/merge",
        "-f",
        f"sha={head}",
        "-f",
        "merge_method=squash",
    ]


def _merged_view_command(pr: int) -> list[str]:
    return [
        "gh",
        "pr",
        "view",
        str(pr),
        "-R",
        GITHUB_REPO,
        "--json",
        "state,headRefOid,mergeCommit,mergedAt",
    ]


def _validate_pr_metadata(
    metadata: Any,
    *,
    branch: str,
    head: str,
) -> None:
    if not isinstance(metadata, dict):
        raise TaskAcceptanceError("GitHub PR metadata must be an object")
    required_fields = {
        "state",
        "isDraft",
        "baseRefName",
        "headRefName",
        "headRefOid",
        "headRepository",
        "isCrossRepository",
        "mergeable",
        "mergeStateStatus",
        "reviewDecision",
        "autoMergeRequest",
    }
    missing = sorted(required_fields - metadata.keys())
    if missing:
        raise TaskAcceptanceError(f"GitHub PR metadata is missing: {', '.join(missing)}")

    expected = {
        "state": "OPEN",
        "isDraft": False,
        "baseRefName": BASE_BRANCH,
        "headRefName": branch,
        "headRefOid": head,
        "isCrossRepository": False,
        "mergeable": "MERGEABLE",
        "mergeStateStatus": "CLEAN",
    }
    mismatches = [
        f"{field}={metadata.get(field)!r}, expected {value!r}"
        for field, value in expected.items()
        if metadata.get(field) != value
    ]
    head_repository = metadata.get("headRepository")
    if (
        not isinstance(head_repository, dict)
        or head_repository.get("nameWithOwner") != GITHUB_REPO
    ):
        mismatches.append(
            f"headRepository.nameWithOwner must be {GITHUB_REPO!r}"
        )
    if metadata.get("reviewDecision") == "CHANGES_REQUESTED":
        mismatches.append("reviewDecision=CHANGES_REQUESTED")
    if metadata.get("autoMergeRequest") is not None:
        mismatches.append("autoMergeRequest must be null")
    if mismatches:
        raise TaskAcceptanceError("PR is not a mergeable fixed candidate: " + "; ".join(mismatches))


def _check_failure(check: Any) -> str | None:
    if not isinstance(check, dict):
        return "invalid check record"
    if (
        check.get("state") == "SUCCESS"
        and check.get("bucket") == "pass"
        and check.get("event") == "pull_request"
    ):
        return None
    return (
        f"{check.get('name') or '(unnamed)'}="
        f"{check.get('state')}/{check.get('bucket')}/{check.get('event')}"
    )


def _validate_named_checks(checks: Any) -> None:
    if not isinstance(checks, list) or not checks:
        raise TaskAcceptanceError("PR checks are missing")
    named = [
        check
        for check in checks
        if isinstance(check, dict) and check.get("name") in REQUIRED_CHECKS
    ]
    names = {check["name"] for check in named}
    failures = [failure for check in named if (failure := _check_failure(check))]
    missing = sorted(REQUIRED_CHECKS - names)
    if missing:
        failures.append(f"missing {', '.join(missing)}")
    if failures:
        raise TaskAcceptanceError("named checks are not green: " + "; ".join(failures))


def _required_contexts(rules: Any) -> set[str]:
    if not isinstance(rules, list):
        raise TaskAcceptanceError("branch rules must be a list")
    contexts: set[str] = set()
    found_rule = False
    for rule in rules:
        if not isinstance(rule, dict) or rule.get("type") != "required_status_checks":
            continue
        found_rule = True
        parameters = rule.get("parameters")
        records = parameters.get("required_status_checks") if isinstance(parameters, dict) else None
        if not isinstance(records, list):
            raise TaskAcceptanceError("required status check rule is malformed")
        for record in records:
            context = record.get("context") if isinstance(record, dict) else None
            if not isinstance(context, str) or not context:
                raise TaskAcceptanceError("required status check context is malformed")
            contexts.add(context)
    if not found_rule or not contexts:
        raise TaskAcceptanceError("branch rules do not declare required status checks")
    return contexts


def _validate_required_checks(checks: Any, expected_contexts: set[str]) -> None:
    if not isinstance(checks, list) or not checks:
        raise TaskAcceptanceError("required checks are missing")
    names = {
        check.get("name")
        for check in checks
        if isinstance(check, dict) and isinstance(check.get("name"), str)
    }
    failures: list[str] = []
    for check in checks:
        failure = _check_failure(check)
        if failure:
            failures.append(failure)
    missing = sorted(expected_contexts - names)
    if missing:
        failures.append(f"missing {', '.join(missing)}")
    if failures:
        raise TaskAcceptanceError("required checks are not green: " + "; ".join(failures))


def _validate_remote_candidate(
    *,
    cwd: Path,
    pr: int,
    branch: str,
    head: str,
) -> None:
    _validate_pr_metadata(
        _run_json(_pr_view_command(pr), cwd=cwd),
        branch=branch,
        head=head,
    )
    _validate_named_checks(_run_json(_checks_command(pr), cwd=cwd))
    expected_contexts = _required_contexts(_run_json(_rules_command(), cwd=cwd))
    _validate_required_checks(
        _run_json(_required_checks_command(pr), cwd=cwd),
        expected_contexts,
    )


def _validate_main_checkout(
    repo_root: Path,
    *,
    fetch: bool,
    allow_behind: bool = False,
) -> bool:
    if _git_value(repo_root, ["status", "--porcelain"], "read main worktree status"):
        raise TaskAcceptanceError("accept requires a clean trusted main checkout")
    branch = _git_value(repo_root, ["branch", "--show-current"], "read main branch")
    if branch != BASE_BRANCH:
        raise TaskAcceptanceError(f"accept must run from branch {BASE_BRANCH}, not {branch or '(detached)'}")
    if fetch:
        _run_command(["git", "fetch", "--no-tags", "origin", BASE_BRANCH], cwd=repo_root)
    local_head = _git_value(repo_root, ["rev-parse", "HEAD"], "read local main HEAD")
    origin_head = _git_value(
        repo_root,
        ["rev-parse", f"refs/remotes/origin/{BASE_BRANCH}"],
        "read origin/main HEAD",
    )
    if local_head != origin_head:
        if allow_behind:
            merge_base = _git_value(
                repo_root,
                ["merge-base", local_head, origin_head],
                "compare local and origin main",
            )
            if merge_base == local_head:
                return False
        raise TaskAcceptanceError(
            f"trusted main is not synchronized with origin/main: {local_head} != {origin_head}"
        )
    return True


def _git_common_dir(worktree: Path) -> Path:
    value = _git_value(worktree, ["rev-parse", "--git-common-dir"], "read Git common directory")
    path = Path(value)
    return (path if path.is_absolute() else worktree / path).resolve()


def _resolve_candidate_task(worktree: Path, task_ref: str) -> Path:
    if not isinstance(task_ref, str) or not task_ref:
        raise TaskAcceptanceError("task path must be explicit")
    if "\\" in task_ref or Path(task_ref).is_absolute():
        raise TaskAcceptanceError("task path must be a POSIX repo-relative path")
    parts = task_ref.split("/")
    if any(part in {"", ".", ".."} for part in parts):
        raise TaskAcceptanceError("task path cannot contain empty, dot, or traversal components")
    if len(parts) != 3 or parts[:2] != [".trellis", "tasks"] or parts[2] == "archive":
        raise TaskAcceptanceError("task must be one active .trellis/tasks/<task> directory")

    task_dir = worktree.joinpath(*parts)
    for path in (worktree / ".trellis", worktree / ".trellis" / "tasks", task_dir):
        if path.is_symlink():
            raise TaskAcceptanceError(f"task path cannot use symlinks: {path}")
    if not task_dir.is_dir():
        raise TaskAcceptanceError(f"task directory not found: {task_ref}")
    task_json = task_dir / "task.json"
    if task_json.is_symlink():
        raise TaskAcceptanceError(f"task.json cannot be a symlink: {task_json}")
    return task_dir


def _read_head_manifest(worktree: Path, *, head: str, task_ref: str) -> dict[str, Any]:
    manifest_path = f"{task_ref}/task.json"
    output = _git_value(
        worktree,
        ["show", f"{head}:{manifest_path}"],
        "read candidate task.json from fixed HEAD",
    )
    try:
        data = json.loads(output)
    except json.JSONDecodeError as error:
        raise TaskAcceptanceError(f"fixed-head task.json is invalid: {error}") from error
    if not isinstance(data, dict):
        raise TaskAcceptanceError("fixed-head task.json root must be an object")
    return data


def _require_head_file(worktree: Path, *, head: str, path: str) -> None:
    output = _git_value(
        worktree,
        ["ls-tree", head, "--", path],
        f"read fixed-head file mode for {path}",
    )
    records = [line for line in output.splitlines() if line]
    if len(records) != 1:
        raise TaskAcceptanceError(f"fixed HEAD requires one tracked regular file: {path}")
    metadata, _, recorded_path = records[0].partition("\t")
    fields = metadata.split()
    if (
        recorded_path != path
        or len(fields) != 3
        or fields[0] not in {"100644", "100755"}
        or fields[1] != "blob"
        or not SHA_RE.fullmatch(fields[2].lower())
    ):
        raise TaskAcceptanceError(f"fixed HEAD requires a regular file: {path}")


def _validate_candidate(
    *,
    main_root: Path,
    worktree_arg: str,
    task_ref: str,
    head: str,
) -> tuple[Path, Path, dict[str, Any], str]:
    worktree_input = Path(worktree_arg)
    if not worktree_input.is_absolute():
        raise TaskAcceptanceError("--worktree must be an absolute path")
    try:
        worktree = worktree_input.resolve(strict=True)
    except OSError as error:
        raise TaskAcceptanceError(f"candidate worktree cannot be resolved: {error}") from error
    if not worktree.is_dir():
        raise TaskAcceptanceError(f"candidate worktree not found: {worktree_arg}")
    if worktree == main_root.resolve():
        raise TaskAcceptanceError("candidate worktree must be separate from the trusted main checkout")
    top_level = Path(
        _git_value(worktree, ["rev-parse", "--show-toplevel"], "read candidate root")
    ).resolve()
    if top_level != worktree:
        raise TaskAcceptanceError(f"--worktree is not the candidate Git root: {worktree}")
    if _git_common_dir(worktree) != _git_common_dir(main_root):
        raise TaskAcceptanceError("candidate is not a worktree of the trusted main repository")

    if _git_value(worktree, ["rev-parse", "HEAD"], "read candidate HEAD") != head:
        raise TaskAcceptanceError("candidate HEAD does not match --head")
    task_dir = _resolve_candidate_task(worktree, task_ref)
    for required_file in ("task.json", "execution.md", "delivery.md"):
        path = task_dir / required_file
        if not path.is_file() or path.is_symlink():
            raise TaskAcceptanceError(f"candidate requires a regular {required_file}")
        _require_head_file(
            worktree,
            head=head,
            path=f"{task_ref}/{required_file}",
        )

    data = _read_head_manifest(worktree, head=head, task_ref=task_ref)
    coordination = data.get("coordination")
    if not isinstance(coordination, dict):
        raise TaskAcceptanceError("accept requires coordination.version=1")
    if type(coordination.get("version")) is not int or coordination.get("version") != 1:
        raise TaskAcceptanceError("accept requires coordination.version to be integer 1")
    expected_state = {
        "status": "in_progress",
        "route": "delegated",
        "phase": "delivered",
        "writer": "main",
        "block": None,
        "base_branch": BASE_BRANCH,
    }
    actual_state = {
        "status": data.get("status"),
        "route": coordination.get("route"),
        "phase": coordination.get("phase"),
        "writer": coordination.get("writer"),
        "block": coordination.get("block"),
        "base_branch": data.get("base_branch"),
    }
    mismatches = [
        f"{field}={actual_state[field]!r}, expected {value!r}"
        for field, value in expected_state.items()
        if actual_state[field] != value
    ]
    registered = data.get("worktree_path")
    if not isinstance(registered, str) or Path(registered).resolve() != worktree:
        mismatches.append("worktree_path does not match --worktree")
    branch = data.get("branch")
    if not isinstance(branch, str) or not branch:
        mismatches.append("branch must be a non-empty string")
    if mismatches:
        raise TaskAcceptanceError("candidate task state is invalid: " + "; ".join(mismatches))

    base_sha = coordination.get("base_sha")
    planning_commit = coordination.get("planning_commit")
    for label, value in (("base_sha", base_sha), ("planning_commit", planning_commit)):
        if not isinstance(value, str) or not SHA_RE.fullmatch(value):
            raise TaskAcceptanceError(f"candidate {label} must be a full lowercase SHA")
        _git_value(worktree, ["cat-file", "-e", f"{value}^{{commit}}"], f"resolve {label}")
    if _git_value(worktree, ["merge-base", base_sha, head], "validate base ancestry") != base_sha:
        raise TaskAcceptanceError("candidate base_sha is not an ancestor of the fixed head")
    if (
        _git_value(worktree, ["merge-base", planning_commit, head], "validate planning ancestry")
        != planning_commit
    ):
        raise TaskAcceptanceError("candidate planning_commit is not an ancestor of the fixed head")

    if _git_value(worktree, ["status", "--porcelain"], "read candidate status"):
        raise TaskAcceptanceError("accept requires a clean candidate worktree")
    if _git_value(worktree, ["branch", "--show-current"], "read candidate branch") != branch:
        raise TaskAcceptanceError("candidate branch does not match task.json")
    return worktree, task_dir, data, branch


def _read_merge_status(*, cwd: Path, pr: int, head: str) -> str | None:
    metadata = _run_json(_merged_view_command(pr), cwd=cwd)
    if not isinstance(metadata, dict) or metadata.get("headRefOid") != head:
        raise TaskAcceptanceError("PR state does not match the accepted head")
    if metadata.get("state") == "OPEN":
        return None
    merge_commit = metadata.get("mergeCommit")
    merge_oid = merge_commit.get("oid") if isinstance(merge_commit, dict) else None
    if (
        metadata.get("state") == "MERGED"
        and isinstance(merge_oid, str)
        and SHA_RE.fullmatch(merge_oid.lower())
    ):
        return merge_oid.lower()
    raise TaskAcceptanceError("PR is neither open nor confirmed merged for the accepted head")


def _confirm_merge_after_error(
    *,
    cwd: Path,
    pr: int,
    head: str,
    merge_error: TaskAcceptanceError,
) -> str:
    last_confirmation_error: TaskAcceptanceError | None = None
    saw_open = False
    for attempt in range(MERGE_CONFIRM_ATTEMPTS):
        try:
            merge_oid = _read_merge_status(cwd=cwd, pr=pr, head=head)
            if merge_oid:
                return merge_oid
            saw_open = True
        except TaskAcceptanceError as confirm_error:
            last_confirmation_error = confirm_error
        if attempt + 1 < MERGE_CONFIRM_ATTEMPTS:
            time.sleep(MERGE_CONFIRM_INTERVAL_SECONDS)
    if saw_open and last_confirmation_error is None:
        raise TaskAcceptanceError(f"merge did not complete and PR remains open: {merge_error}")
    detail = last_confirmation_error or merge_error
    raise TaskAcceptanceError(
        f"merge result is unknown after {merge_error}; last confirmation: {detail}"
    )


def _merge_fixed_head(*, cwd: Path, pr: int, head: str) -> str:
    try:
        response = _run_json(_merge_command(pr, head), cwd=cwd)
        merge_sha = response.get("sha") if isinstance(response, dict) else None
        if (
            not isinstance(response, dict)
            or response.get("merged") is not True
            or not isinstance(merge_sha, str)
            or not SHA_RE.fullmatch(merge_sha.lower())
        ):
            message = response.get("message") if isinstance(response, dict) else None
            raise TaskAcceptanceError(
                f"GitHub did not synchronously merge the PR: {message or 'invalid response'}"
            )
        return merge_sha.lower()
    except TaskAcceptanceError as merge_error:
        return _confirm_merge_after_error(
            cwd=cwd,
            pr=pr,
            head=head,
            merge_error=merge_error,
        )


def cmd_accept(args: argparse.Namespace) -> int:
    repo_root = get_repo_root().resolve()
    try:
        head = str(args.head)
        if not SHA_RE.fullmatch(head):
            raise TaskAcceptanceError("--head must be a full lowercase 40-character SHA")
        pr = args.pr
        if type(pr) is not int or pr <= 0:
            raise TaskAcceptanceError("--pr must be a positive integer")

        main_is_current = _validate_main_checkout(
            repo_root,
            fetch=True,
            allow_behind=True,
        )
        existing_merge = _read_merge_status(cwd=repo_root, pr=pr, head=head)
        if existing_merge:
            merge_oid = existing_merge
            print(colored(f"✓ PR #{pr} was already merged at the accepted head", Colors.GREEN))
            print(f"Accepted head: {head}")
            print(f"Merge commit: {merge_oid}")
            print("Next: main records acceptance, archives the task, and cleans the worktree.")
            return 0
        if not main_is_current:
            raise TaskAcceptanceError(
                "trusted main is behind origin/main while the accepted PR remains open"
            )
        worktree_arg = str(args.worktree)
        worktree, _, initial_manifest, branch = _validate_candidate(
            main_root=repo_root,
            worktree_arg=worktree_arg,
            task_ref=str(args.dir),
            head=head,
        )
        _validate_remote_candidate(
            cwd=repo_root,
            pr=pr,
            branch=branch,
            head=head,
        )

        # Repeat every mutable local and remote check immediately before the
        # atomic REST merge. Candidate files are only read as data; no candidate
        # Python module or script is imported or executed.
        _validate_main_checkout(repo_root, fetch=True)
        final_worktree, _, final_manifest, final_branch = _validate_candidate(
            main_root=repo_root,
            worktree_arg=worktree_arg,
            task_ref=str(args.dir),
            head=head,
        )
        if final_worktree != worktree or final_manifest != initial_manifest or final_branch != branch:
            raise TaskAcceptanceError("candidate task manifest changed during acceptance")
        _validate_remote_candidate(
            cwd=repo_root,
            pr=pr,
            branch=branch,
            head=head,
        )
        merge_oid = _merge_fixed_head(cwd=repo_root, pr=pr, head=head)
    except (TaskAcceptanceError, ValueError) as error:
        print(colored(f"Error: {error}", Colors.RED), file=sys.stderr)
        return 1

    print(colored(f"✓ PR #{pr} accepted and merged", Colors.GREEN))
    print(f"Accepted head: {head}")
    print(f"Merge commit: {merge_oid}")
    print("Next: main records acceptance, archives the task, and cleans the worktree.")
    return 0
