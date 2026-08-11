<div align="center">
  <img src="public/logo.jpg" width="120" alt="AIO Coding Hub Logo" />

# AIO Coding Hub

**Local AI CLI Unified Gateway** — Route Claude Code / Codex / Gemini CLI through a single entry point

[![Release](https://img.shields.io/github/v/release/KNaiFen/aio-coding-hub?style=flat-square)](https://github.com/KNaiFen/aio-coding-hub/releases)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20|%20macOS%20|%20Linux-lightgrey?style=flat-square)](#installation)

[简体中文](./README.md) | English

</div>

> **Credits** — Inspired by [cc-switch](https://github.com/farion1231/cc-switch), [claude-code-hub](https://github.com/ding113/claude-code-hub), and [code-switch-R](https://github.com/Rogers-F/code-switch-R).

> **Fork Note** — This repository is a personal fork for `vibe coding`, experiments, and ad-hoc changes. Code may change at any time and **does not guarantee availability, stability, or compatibility**; it is not suitable as a default production dependency. For the original feature set, use the upstream repository as the source of truth.
>
> Fork-side references:
> - Codex reasoning-token guard / retry design: [codex-retry-gateway](https://github.com/nonononull/codex-retry-gateway)
> - Continuation repair design: [CodexCont](https://github.com/neteroster/CodexCont)

---

## Why?

| Problem | How AIO Coding Hub Solves It |
|---------|------------------------------|
| Each CLI needs separate API config | **Unified gateway** — all CLIs route through `127.0.0.1` |
| Upstream goes down, requests fail | **Smart failover** — auto-switch providers with circuit breaker |
| Different scenarios need different provider sets | **Sort templates** — multiple sets, per-CLI activation |
| No idea how many tokens or how much it costs | **Full observability** — trace, usage stats, cost estimation |
| Different projects need different Prompts / MCP configs | **Workspace isolation** — per-project CLI config, one-click switch |

---

## Screenshots

### Home — Heatmap, usage trends, active sessions, request logs

![Home](public/screenshots/home.png)

### Usage — Token stats, cache hit rate, latency, cost leaderboard

![Usage](public/screenshots/usage.png)

### Model Validation — Multi-dimensional channel verification

![Model Validation](public/screenshots/modelValidate.png)

---

## Features

### Gateway Proxy

- Single entry point for Claude Code / Codex / Gemini CLI
- Per-CLI proxy toggle on Home, one-click on/off
- Custom model name mapping
- Auto-fix for SSE / JSON responses

### Smart Routing & Resilience

- Multi-provider priority ordering + automatic failover
- Circuit breaker (configurable threshold & recovery time)
- Sticky session for consistent provider routing
- Sort templates: multiple provider sets, activated per CLI
- Drag-to-reorder, per-provider toggle, instant switching

### Usage & Observability

- Token usage analytics (by CLI / provider / model)
- Cost estimation + auto-synced model pricing
- Request trace & real-time console logs
- Request heatmap (time-of-day distribution)
- Cache trend chart: per-provider hit rate, 60% warning line
- Availability: provider timeline dots, 15s auto-refresh

### Workspace Management

- Per-project isolation for Prompts, MCP, and Skill configs
- Workspace compare, clone, switch & rollback
- Auto-sync configs to each CLI

### Skill Market

- Discover and install Skills from Git repositories
- Repository management, filtering, and sorting
- Batch management linked to workspaces

### Plugin System

- Official bundled Privacy Filter and community Extension Host plugins
- Gateway and log hooks, commands, provider extension values, and host-rendered UI contributions
- Capability-gated APIs, manifest validation, configuration schemas, audit reports, quarantine, and rollback
- SDK and scaffolder: `@aio-coding-hub/plugin-sdk` and `create-aio-plugin`

Start with the [Plugin Development Guide](docs/plugins/README.md). Community plugins use the Extension Host; WASM, process, and native runtimes are unsupported pre-release legacy paths.

### CLI Management

- Direct editing of Claude Code settings
- CodeMirror editor for Codex config.toml
- Environment variable conflict detection
- Local session history browser (project → session → messages)

### Model Validation

- Multi-dimensional validation templates (token truncation, Extended Thinking, etc.)
- Cross-provider signature verification
- Batch validation + history

### More

- Auto-update, autostart, single instance
- Data import / export / reset
- WSL support

---

## Installation

### Download from Releases (Recommended)

Go to [Releases](https://github.com/KNaiFen/aio-coding-hub/releases) and download for your platform.
This fork currently publishes tagged builds only for Windows x64 and macOS Apple Silicon; use manual cloud development builds for other targets or use the upstream releases:

<!-- SUPPORT_MATRIX_RELEASE_DOWNLOAD:START -->
| Platform | Official release packages |
| --- | --- |
| Windows x64 | `.msi` / `-portable.zip` |
| macOS Apple Silicon | `.zip` |
<!-- SUPPORT_MATRIX_RELEASE_DOWNLOAD:END -->

The desktop updater matrix still covers only the two targets above; every tagged Release also includes standalone `aio-tui` archives for all four targets.

### SSH / Codex CLI terminal panel

When the desktop AIO app is running, it exposes a read-only loopback observer. The observer never changes gateway forwarding and the TUI never starts the desktop app. Download the matching `aio-tui-*` Release asset and put the binary on `PATH`:

| Platform | TUI asset |
| --- | --- |
| Windows x64 | `aio-tui-win64.zip` |
| macOS Intel | `aio-tui-macos-intel.tar.gz` |
| macOS Apple Silicon | `aio-tui-macos-arm.tar.gz` |
| Linux x64 | `aio-tui-linux-x64.tar.gz` |

```bash
# macOS / Linux
tar -xzf aio-tui-macos-arm.tar.gz   # use aio-tui-macos-intel.tar.gz on Intel
chmod +x aio-tui
sudo install -m 0755 aio-tui /usr/local/bin/aio-tui

aio-tui
aio-tui status
aio-tui status --once --cli codex
aio-tui status --items preferred-provider,last-request,concurrency,today-cost
aio-tui statusline
```

Windows users can unzip `aio-tui-win64.zip` and add its directory to `PATH`. Use `--cli claude|codex|grok|gemini|all`; in `all` scope the status line follows the CLI of the newest terminal inference request. In `aio-tui statusline`, use Space to toggle items, Left/Right to reorder, `c` to toggle colors, and Enter to save; `--items` overrides only the current run. The default fields are preferred provider, last request, dominant provider in the last ten requests, concurrency, today's cost, and today's tokens. Set `NO_COLOR` to force plain output. Concurrency is the global count of active model-inference requests, so every request from the same Session or a sub-agent counts as one. Offline mode keeps the last snapshot, shows a stale label, and never starts AIO.

Standalone TUI archives are not included in the desktop updater `latest.json`; verify them with the release `SHA256SUMS.txt`.

<details>
<summary>Linux Arch / Wayland users</summary>

**Recommended: AUR package** (uses system libraries, best compatibility)

```bash
paru -S aio-coding-hub-bin
# or
yay -S aio-coding-hub-bin
```

**AppImage users**

The app automatically detects Wayland sessions and sets `WEBKIT_DISABLE_COMPOSITING_MODE=1`
to prevent EGL display initialisation crashes (see [issue #93](https://github.com/FingerCaster/aio-coding-hub/issues/93)).
The repository provides no local native repackaging entry point; use the cloud `dev-build`
workflow when a desktop artifact is required.

</details>

<details>
<summary>macOS security note</summary>

If you see "can't be opened / unverified developer":

```bash
sudo xattr -cr /Applications/"AIO Coding Hub.app"
```

</details>

### Zero-Artifact Local Checks and Cloud Validation

Do not install repository dependencies, start a development server, or run formatting, type checking, linting, tests, or builds locally. The allowed local checks do not need `node_modules` and do not create Node or Rust artifacts:

```bash
node scripts/check-cloud-only-verification.selftest.mjs
node scripts/check-cloud-only-verification.mjs
git diff --check
```

For a changed `.mjs` file, `node --check <changed-file.mjs>` is also allowed directly. Regular pull requests and protected-branch pushes trigger `ci` automatically; use the commit's `ci-gate` and `pr-title` results. Do not start an additional manual `ci` run for routine validation. `workflow_dispatch` is reserved for `main` recovery or candidate builds, while the Provider trend release benchmark runs on relevant automatic CI paths or the standalone `performance` workflow. Run `dev-build` from Actions only when a desktop integration artifact is needed.

<!-- SUPPORT_MATRIX_SOURCE_BUILD:START -->
| Scope | Cloud workflow target | Notes |
| --- | --- | --- |
| Release / development | Actions `dev-build`: `windows-x64` | Windows x64; Signed candidate from `main` CI; unsigned development artifact from the manual workflow |
| Development | Actions `dev-build`: `macos-x64` | macOS Intel; Unsigned development artifact from the manual workflow; excluded from Release/updater |
| Release / development | Actions `dev-build`: `macos-arm64` | macOS Apple Silicon; Signed candidate from `main` CI; unsigned development artifact from the manual workflow |
| Development | Actions `dev-build`: `linux-x64` | Linux x64; Unsigned development artifact from the manual workflow; excluded from Release/updater |
<!-- SUPPORT_MATRIX_SOURCE_BUILD:END -->

Manual cloud artifacts are unsigned and cannot be promoted by Release. Formal releases only promote the signed Windows x64 and macOS Apple Silicon candidates produced by successful `main` CI.

---

## Quick Start

```
1. Providers page → Add upstream (official API / self-hosted proxy / company gateway)
2. Home page → Toggle "Proxy" switch for target CLI
3. Run CLI in terminal → View trace & stats in Console / Usage page
```

Verify the gateway is running:

```bash
curl http://127.0.0.1:37123/health
# {"status":"ok"}
```

---

## Project Documentation

- [Project knowledge base](docs/README.md): the canonical map for product, architecture, plugin, operations, task, and historical documentation.
- [Pending work](PENDING.md) and [completed work](PENDING_COMPLETED.md): deferred items and delivery evidence.
- [Trellis task index](.trellis/tasks/README.md): plans, research, checks, and archived task context.

Current code and machine-readable contracts take precedence over historical audits, superseded plans, and session journals.

## Tech Stack

| Layer | Technology |
|-------|------------|
| **Frontend** | React 19 · TypeScript · Tailwind CSS · Vite |
| **State** | TanStack Query · React Hooks |
| **Desktop** | Tauri 2 |
| **Backend** | Rust · Axum (HTTP Gateway) |
| **Database** | SQLite (rusqlite) |
| **Testing** | Vitest · Testing Library · MSW · Cargo Test |

---

## Quality Assurance

GitHub Actions owns dependency auditing, frontend lint, TypeScript, plugin SDK/scaffolder tests, E2E, coverage, the Vite build, Rust formatting, `Cargo.lock`, generated bindings, Clippy, Rust tests, and audit. `ci-gate` closes over those results. Cross-platform desktop packaging remains a main-candidate or on-demand `dev-build` concern, not a required job for every PR.

When CI reports formatting, lockfile, or generated-binding drift, download and review its bounded patch instead of regenerating files locally.

---

## Not Designed For

- Public deployment / remote access / multi-tenant
- Enterprise RBAC

> This is a **local desktop tool + local gateway**. All data stays on your machine.

---

## Contributing

Issues and PRs welcome! We follow [Conventional Commits](https://www.conventionalcommits.org/).

```bash
feat(ui): add usage heatmap
fix(gateway): handle timeout correctly
docs: update installation guide
```

---

## License

[MIT License](LICENSE)

---

[![Stargazers over time](https://starchart.cc/KNaiFen/aio-coding-hub.svg?variant=adaptive)](https://starchart.cc/KNaiFen/aio-coding-hub)
