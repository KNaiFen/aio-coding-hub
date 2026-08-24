# AIO GKD v0.1.5 Bundle Pin Implementation

## Internal Design

- One explicit AIO consumer pin remains the source of truth; existing strict validator and selftest consume the same four immutable release facts.

## Execution Details

1. Replace the four published release facts in the canonical pin and existing adapter expectations.
2. Run the declared AIO local verifier and deliver one fixed PR head with `delivery.md` bound before the final task transition.
3. Stop before acceptance, merge, release, production, or cleanup side effects.
