# AIO GKD v0.1.5 Bundle Pin Delivery

## Result

- Updated the AIO consumer pin to published GKD `v0.1.5`.
- The canonical pin binds release source `60ac0c49f1054ce2edea49b3ab6758bfbd3432b3`, execution bundle digest `d749b753fb11aeab44d41b4e1d8bec44c7fa2d18a4b08148fbc0e0c127e27e6d`, and asset SHA-256 `f259475f4ca6c3425e53d734d03633541d6a1997e41991eb5a6115958d06a298`.
- Updated the strict adapter validator, its selftest fixture, and adapter documentation to the same release facts.

## Verification

- `scripts/gkd-verify --base-sha 58e1b36b67f160782670d610738a8476d7f050ce` passed the versioned local contract, including adapter selftest and smoke, diff checks, and changed Node syntax checks.
- Required GitHub checks `ci-gate` and `pr-title` remain pending for the fixed delivery head.
- Dependency installation, formatting, linting, type checking, tests, coverage, builds, generators, Rust/Tauri checks, and signing or packaging remain cloud-owned.

## Scope And Risk

- `.gkd/policy.json`, `.gkd/review-adapter.json`, `.gkd/resource-facts.json`, workflows, runner configuration, GitHub settings, product code, Trellis history, releases, and production installations were not changed.
- The adapter remains an explicit local validator; no dynamic release discovery, source lookup, or production behavior was added.

## Candidate Output Bundle

- Deterministic Git source archive of implementation head `19d2d22115ad26622a39602322c8ed934607ed5c` SHA-256: `95cfb6281dcb715b3847cd728186c75b1fe021dbb94f7493d87f25f8cf149694`.
