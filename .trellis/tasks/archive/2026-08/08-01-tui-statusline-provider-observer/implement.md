# Implementation

1. Add the bounded TUI config module, status item catalog, styled wrapping, interactive
   `statusline` picker, `--items` override, documentation, and unit coverage. Commit as
   `feat(tui): add configurable colored statusline`.
2. Replace observer DB `try_acquire` degradation with bounded fair acquisition/cache recheck;
   map Busy to retained stale state; render request cards with semantic spans and separators.
   Commit as `fix(tui): stabilize concurrent views and log rendering`.
3. Add the secret-free bounded provider projection and optional v1 snapshot section, legacy
   fallback, dashboard view state, five-line cards, details, navigation, docs, and tests. Update
   the observer/TUI spec and archive this task in the same commit:
   `feat(tui): add provider status view`.
4. Run only allowed local checks: `pnpm typecheck`, `pnpm lint`, `pnpm test:unit`,
   `pnpm build`, and relevant Node contract checks. Review every staged diff and exclude the
   user-owned workspace directory.
5. Synchronize package/Tauri/workspace version declarations to 0.60.40 without invoking Cargo.
   Commit as `chore(release): bump version to 0.60.40`.
6. Push to `origin`, open a ready PR against main, and let GitHub Actions run native formatting,
   lock synchronization, generated bindings, Rust tests, Clippy, audit, signing, and packaging.
   Apply only bounded CI drift artifacts or focused fixes and rerun until green.
7. Merge normally, wait for successful exact-main-SHA CI and release candidate, then push
   `aio-coding-hub-v0.60.40`. Verify the release, desktop assets, four TUI archives, and checksums.

## Rollback points

- Each feature commit is independently revertible.
- Busy handling leaves the prior successful snapshot untouched.
- Provider response is additive/optional; disabling its query restores current behavior.
- No database migration or persistent desktop configuration change is introduced.
