# Repository Rules

- For PR review, merge, release, and other repository operations, default to the `origin` remote. Do not inspect or operate on `upstream` unless the user explicitly requests upstream work.
- For GitHub CLI operations, do not rely on implicit repository resolution when both `origin` and `upstream` exist. Use `gh repo set-default KNaiFen/aio-coding-hub` for this clone and prefer explicit `--repo` / `-R KNaiFen/aio-coding-hub` on `gh` commands that mutate state or inspect Actions, releases, PRs, or issues.
- For fork releases, default to incrementing only the patch component (for example, `0.60.31` to `0.60.32`) unless the user explicitly requests a larger bump or compatibility requires one.
- Release workflow builds must not checkout by release tag alone. Draft GitHub Releases can exist before their Git tag is fetchable; resolve or create the release tag first, then pass an immutable commit SHA to downstream build jobs.
- Keep `upstream` fetch-only for normal work. Do not restore an `upstream` push URL unless the user explicitly requests upstream push access.
- When the user explicitly requests upstream merge or drift repair work, carry forward non-conflicting `upstream/main` changes. If an upstream change conflicts with fork-specific product behavior or functionality, pause and ask the user with concrete file/behavior evidence and viable options before choosing either side.
- Keep upstream merge and drift-repair tasks integration-only. Make only the minimal changes required to resolve concrete textual or semantic conflicts and preserve an explicit fork decision. Do not fix defects that already exist in the pinned upstream revision independently of the merge, even when review or validation discovers them; record those defects as out-of-scope findings and handle them in a separately authorized follow-up task, not in the merge task or merge commit.
- Keep the local checkout zero-artifact. Never install repository dependencies or run package-manager scripts, development servers, formatters, type checks, linters, tests, builds, Cargo, rustfmt, Clippy, Specta binding generation, Tauri, signing, or packaging on a local machine, even when an existing `node_modules` or Rust target directory is present.
- Local validation is limited to direct dependency-free Node.js source contracts and syntax parsing that do not write files, plus `git diff --check`. The canonical entry is `node scripts/check-cloud-only-verification.mjs`; run its self-test directly with `node scripts/check-cloud-only-verification.selftest.mjs`. Use `node --check <changed-file.mjs>` only for changed Node source. Do not invoke these checks through `pnpm`.
- GitHub Actions owns dependency installation, frontend lint/typecheck/tests/build, Rust formatting and lock synchronization, generated bindings, Clippy, Rust tests, audit, signing, and desktop packaging. Apply only a bounded CI drift patch when Actions reports generated-file drift. Use the full `ci` workflow_dispatch for validation and the `dev-build` workflow only when a desktop integration artifact is actually needed.

## Multi-worktree Delivery

- The canonical independent-session workflow and Markdown templates live in `docs/operations/multi-worktree-delivery.md`. Use `execution.md` as the main-authored entry point, `delivery.md` as the execution-session handoff, and `findings.md` only when main requests changes.
- The coordinator checkout is `main`; implementation happens in a sibling task worktree and never directly in the coordinator checkout.
- An independent execution session may commit and push its assigned task branch, create or update its PR, and fix failures until the latest PR commit has passed the required CI and relevant compile jobs. It must not push `main`, merge a PR, enable auto-merge, or remove a worktree.
- Implementation is complete only when the latest PR commit is green in the required cloud checks and the task worktree contains a Markdown handoff with the PR link, changed files and code locations, deviations from the plan, verification results, and open issues.
- After handoff, the execution session pauses. Main reviews the latest PR diff against the task artifacts and current contracts; main may perform the review itself or ask a read-only sub-agent to report findings.
- If acceptance fails, main writes a Markdown `findings.md` in the task worktree. The execution session continues from that document; every new push requires the relevant CI checks to pass again before re-acceptance.
- Only main merges an accepted PR. Include required knowledge-base updates in the accepted PR whenever possible; if a new current fact is discovered after merge, main opens a small follow-up documentation PR instead of pushing directly to protected `main`. After the documentation is merged, main archives the Trellis task, synchronizes local `main`, and removes the task worktree only after confirming it is clean and no session is using it.
- Do not add a second custom JSON gate for this workflow. Use the existing Trellis task artifacts where applicable, Markdown handoffs for delivery, and GitHub PR checks for CI evidence.

## Project Knowledge Base

- `docs/README.md` is the canonical navigation entry for product, architecture, plugin, operations, task, and historical documentation.
- Treat current code and machine-readable contracts as authoritative over prose. Current specifications outrank task records; task records outrank historical audits and session journals.
- Move completed or superseded evidence into the indexed history/archive locations instead of leaving parallel current-looking documents at the repository root.

## Deferred Work List

- `PENDING.md` is the canonical active list for unresolved small issues and improvements that the user asks to accumulate for a later batch. Completed and explicitly dropped history lives in `PENDING_COMPLETED.md` and is not part of the mandatory pre-planning context.
- Recording an item is not authorization to implement it. When the user asks only to record or discuss an issue, update `PENDING.md` and do not change product code for that item.
- Before producing any formal implementation plan, and before starting changes after an explicit instruction such as "start", "implement", or "begin modifying", read `PENDING.md` and include every unresolved `pending` or `planned` entry in the proposed work checklist. Do not silently omit an entry; surface conflicts, dependencies, or scope risks and ask before deferring it again.
- Give each new entry a stable sequential ID, and record its status, date, observed problem, locked user decisions, proposed direction, and acceptance criteria. After a `done` entry has merge/release evidence, or a `dropped` entry has an explicit user decision and reason, move the complete entry to `PENDING_COMPLETED.md`; never delete or compress its history.
- When an entry is selected for implementation, link it to the corresponding Trellis task and change its status to `planned`. Mark it `done` only after the implementation is merged and verified, including the PR, commit, or release evidence. Use `dropped` only with an explicit user decision and record the reason.
<!-- TRELLIS:START -->
# Trellis Instructions

These instructions are for AI assistants working in this project.

This project is managed by Trellis. The working knowledge you need lives under `.trellis/`:

- `.trellis/workflow.md` — development phases, when to create tasks, skill routing
- `.trellis/spec/` — package- and layer-scoped coding guidelines (read before writing code in a given layer)
- `.trellis/workspace/` — per-developer journals and session traces
- `.trellis/tasks/` — active and archived tasks (PRDs, research, jsonl context)

If a Trellis command is available on your platform (e.g. `/trellis:finish-work`, `/trellis:continue`), prefer it over manual steps. Not every platform exposes every command.

If you're using Codex or another agent-capable tool, additional project-scoped helpers may live in:
- `.agents/skills/` — reusable Trellis skills
- `.codex/agents/` — optional custom subagents

Managed by Trellis. Edits outside this block are preserved; edits inside may be overwritten by a future `trellis update`.

<!-- TRELLIS:END -->
