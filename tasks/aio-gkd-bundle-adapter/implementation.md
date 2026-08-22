# AIO GKD Bundle Pin And Project Adapter Implementation

## Internal Design

GKD generic components remain in the verified bundle. AIO contributes only declarative identity and capability facts under `.gkd`, plus a small project-owned compatibility smoke. The pin record contains release identity and digests but no local installation root. The review adapter uses the bundle schema and digest implementation, so it cannot silently diverge from the portable contract.

The candidate worktree receives generated `.codex`, `.agents` and `.gkd/runtime-project.json` only during staging. Those files are machine-local evidence, must be excluded from Git, and are removed after their result is recorded. The tracked `.gkd` policy and adapter remain declarative and portable.

## Execution Details

1. Re-verify the release archive, isolate an installation beneath a system temporary root, and use the installed executable with a Python 3.11+ interpreter.
2. Inspect the live AIO origin and required-check names, then create the policy, release pin and review adapter with canonical JSON serialization.
3. Compute the review adapter digest using the installed bundle library; add a small Node smoke that rejects policy/repository/origin and pin drift without running product checks.
4. Run only `node scripts/check-local-verification.mjs --base <registered-full-base>` and the new compatibility smoke. Keep dependency, frontend, Rust, packaging and runtime UI checks cloud-owned.
5. Stage and project-verify only after tracked changes are cleanly committed; capture redacted digest facts for `delivery.md`, remove machine staging, push the task branch and stop at its complete fixed head.
