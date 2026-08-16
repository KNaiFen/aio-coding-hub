---
name: aio-trellis-main
description: Coordinate AIO Coding Hub tasks as the primary session. Use for requirement planning, task routing, worktree handoff, fixed-head acceptance, merge decisions, archival, and cleanup; do not use inside an assigned execution worktree session.
---

# AIO Trellis Main

Keep decisions in task Markdown and machine facts in `task.json`. Do not maintain a second handwritten status table.

## Start

1. Read root `AGENTS.md`.
2. For an existing task, run:

   ```bash
   python3 .trellis/scripts/task.py status <task> --json
   python3 .trellis/scripts/task.py doctor <task>
   ```

3. Read `prd.md`; read `design.md` and `implement.md` only when present and relevant.
4. Select one phase reference below. Do not load every phase document.

## Route

- Planning, task creation, worktree registration, or execution handoff: read `docs/operations/multi-worktree/planning-and-handoff.md`.
- Reviewing a delivered PR or routing rework: read `docs/operations/multi-worktree/acceptance-and-rework.md`.
- Merging, archiving, knowledge updates, or cleanup: read `docs/operations/multi-worktree/merge-archive-cleanup.md`.
- Direct low-risk main-session changes: read `docs/operations/task-documentation-records.md` instead of the worktree workflow.

## Commands

- Register an already-created worktree with `task.py delegate`; commit the generated task state.
- Run `task.py start`; commit the `ready -> implementing` transition.
- Generate the user-facing launch package with `task.py handoff`; do not handwrite branch, worktree, writer, base, or planning SHA.
- Use `task.py block/resume` for persistent blockers and writer recovery.
- A delivered task is main-owned for review. For rework, commit `findings.md`, then run `task.py start <task> --writer <execution-session>` before returning the worktree.
- Re-read live PR head and CI for acceptance. Do not cache them as canonical task state.

Only main merges, writes terminal `acceptance.md`, archives, updates long-term knowledge, resolves PENDING, or removes a worktree. Stop if user decisions are unresolved, writer ownership is unclear, or local state differs from the registered task.
