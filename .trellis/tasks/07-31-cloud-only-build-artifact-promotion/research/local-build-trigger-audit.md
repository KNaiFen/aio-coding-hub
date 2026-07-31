# Local Build Trigger Audit

Date: 2026-07-31

Scope: repository-controlled local commands, Git configuration, hooks, editor tasks, Trellis lifecycle hooks and package scripts. No build command was executed.

## Confirmed automatic triggers

1. `package.json` defines both `hooks:install` and `postinstall` as `node scripts/install-git-hooks.mjs`.
2. `scripts/install-git-hooks.mjs` writes `core.hooksPath=.githooks` outside CI.
3. The current clone is actively configured with `local file:.git/config .githooks`.
4. `.githooks/pre-commit` is executable and can run `cargo fmt`, `cargo check --locked`, fallback Cargo resolution, update `Cargo.lock`, and stage changed files.
5. `.githooks/pre-push` is executable and unconditionally invokes the full pre-push stage for every ref/remote.
6. Trellis lifecycle hooks are not configured, but Trellis can auto-commit; while the repository hook path is active, that is an indirect trigger when unrelated native files are already staged.

## Misleading or explicit native entry points

- `check:generated-bindings` runs `cargo run --example export-bindings` and then writes the generated TypeScript file with Prettier.
- `check:precommit`, `check:precommit:full`, `check:prepush` and `check:plugin-hardening` include Cargo-backed stages.
- `plugin:perf-smoke`, `tauri:*`, target build aliases, test, Clippy and generation scripts all invoke Rust/Tauri.
- Any Cargo compilation invokes `src-tauri/build.rs` and `tauri_build::build()`.
- README and live Trellis specs still direct developers/agents to run these commands locally.

## Negative findings

- `.git/hooks` contains only inactive `*.sample` files.
- No `.vscode/tasks.json`, `.idea`, `.fleet`, Husky, lint-staged, Lefthook, Makefile, Justfile, Taskfile or Cargo config adds another automatic build.
- `.trellis/config.yaml` contains only commented lifecycle hook examples.
- Subpackages use TypeScript-only tests/typechecks.
- Root `pnpm build` is a frontend TypeScript/Vite build and does not invoke Cargo.
- `support-matrix.mjs` currently validates text/contracts and does not itself launch a build.

## Required cleanup boundary

- Remove automatic hook installation and tracked repository hooks.
- Remove repository-approved local native package scripts and local aggregate references.
- Move native canonicalization and validation to CI with a downloadable patch for drift.
- Replace advertised local target builds with explicit manual cloud artifacts.
- Update agent/docs/spec instructions; preserve archived historical evidence.

Repository files cannot technically prevent a user from directly invoking a system-installed Cargo binary. The enforceable contract is the absence of repository-controlled triggers and the cloud-only project guidance.
