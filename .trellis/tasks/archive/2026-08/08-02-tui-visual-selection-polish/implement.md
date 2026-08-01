# Implementation Plan

1. Refactor dashboard selections to optional request/provider indices with
   independent five-second deadlines and exact expiry handling in the event loop.
2. Add the shared header separator and update request/provider list rendering to
   preserve scrolling only while a real selection exists.
3. Replace provider whole-line styling with bounded styled segments, implement
   the semantic color matrix, and omit absent OAuth rows in cards and detail.
4. Add the two-column interactive status gutter and replace bright accents with
   the Codex-style fixed fallback palette.
5. Update the local observer/TUI contract and add deterministic Ratatui tests for
   selection expiry, list reset, semantic spans, optional rows, narrow CJK
   rendering and status indentation.
6. Commit each behavior group separately, push to `origin`, open a PR to `main`,
   apply only CI-produced native formatting drift if needed, and merge only when
   PR and main CI are green.

## Validation

- Local: inspect diffs and run only repository checks that do not invoke Rust.
- Cloud: canonical Rust formatting/bindings check, Clippy, workspace Rust tests,
  dependency audit and four-platform standalone TUI builds.
- Do not run Cargo, rustfmt, Clippy, Rust tests, binding generation or Tauri
  commands locally.
