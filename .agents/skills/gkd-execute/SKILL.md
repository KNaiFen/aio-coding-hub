---
name: gkd-execute
description: Execute one registered AIO Coding Hub task in an independent worktree session. Use when a handoff points to execution.md and this session owns implementation, commits, the task PR, CI repair, delivery.md, and the final pause before main acceptance.
---

# GKD Execute

Work only in the registered task worktree. This skill does not grant merge, archive, acceptance, or cleanup authority.

## Preflight

1. Read root `AGENTS.md` and the handed-off `execution.md`.
2. Run the exact task path from the handoff:

   ```bash
   python3 .trellis/scripts/task.py status <task> --json
   python3 .trellis/scripts/task.py doctor <task>
   ```

3. Stop on any mismatch. Do not repair branch, worktree, base, planning commit, or writer fields by hand.
4. Read the task files in the order declared by `execution.md`.
5. Read only `docs/operations/multi-worktree/execution-and-delivery.md` for the shared execution procedure.

## Execute

- Implement only the locked scope; preserve unrelated changes.
- Create or update the task PR, push the task branch, and repair task-related CI.
- Keep `delivery.md` focused on actual behavior, AC evidence, deviations, validation, and risks. GitHub remains the source for the latest head and CI.
- When blocked, run `task.py block` with the real reason, recovery condition, and owner; update the task-specific delivery facts, then pause.
- When ready, commit and push implementation plus `delivery.md`, run `task.py deliver`, commit and push that state transition, wait for the final head's applicable CI, make the PR reviewable, and pause.

Do not spawn another implementation chain, merge, enable auto-merge, write `acceptance.md`, archive, update PENDING history, or remove the worktree. Resume only after main updates the persistent state and explicitly returns write ownership.
