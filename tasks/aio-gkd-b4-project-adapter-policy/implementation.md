# AIO GKD Project Adapter Policy Implementation

## Internal Design

- One canonical `.gkd/adapter-policy.json` holds AIO-only declarative workflow facts.
- `scripts/check-gkd-adapter.mjs` remains the sole zero-dependency reader and enforces exact structure and values alongside the existing bundle, review, generic policy, and resource bindings.
- The selftest creates the same canonical fixture and mutates each material boundary to prove fail-closed behavior.

## Execution Details

1. Add the minimal canonical policy with verification, CI, and release sections derived from current versioned repository sources.
2. Extend the existing validator and selftest without adding a helper package, YAML parser, network lookup, or duplicated GKD implementation.
3. Update only the adapter operations documentation and root fact-inventory rule needed to explain the new file.
4. Run the registered AIO local verifier and deliver one fixed PR head with `delivery.md` committed before the final task transition.
5. Stop before acceptance, merge, cleanup, historical migration, CI workflow edits, tag, Release, deployment, or production side effects.
