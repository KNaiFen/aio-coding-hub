---
name: gkd-accept
description: Perform read-only acceptance review for one frozen AIO Coding Hub task candidate. Use when main requests independent evidence about PR diff, requirements, regressions, tests, documentation, or CI before main decides acceptance.
---

# GKD Acceptance Review

Return evidence to main. Never modify files, task state, PR state, branches, or worktrees.

## Review

1. Read root `AGENTS.md` and `docs/operations/multi-worktree/acceptance-and-rework.md`.
2. Run `task.py status <task> --json` and `task.py doctor <task>` as read-only preflight.
3. Read `execution.md`, `delivery.md`, `prd.md`, and only the design, implementation, or findings files needed for the assigned risk.
4. Confirm the supplied full PR head SHA is still current before using CI or diff evidence.
5. Inspect the live diff and required checks for that exact head. Treat a new commit as invalidating earlier conclusions.
6. Review only the assigned scope unless a material blocker is directly visible.

## Return

Lead with actionable findings ordered by severity. Include stable finding intent, `file:line` or symbol evidence, impact, expected result, and a concrete recheck. Separate blocking findings from non-blocking suggestions.

If no issue is found, say so and name residual test or environment risk. Do not write `findings.md`; main owns acceptance records, rework routing, merge, and closeout.
