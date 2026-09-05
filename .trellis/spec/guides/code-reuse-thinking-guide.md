# Code Reuse Thinking Guide

> **Purpose**: Reuse existing behavior when it reduces inconsistency and maintenance cost.

## Before Writing Related Code

When the change repeats existing behavior or adds a shared helper, search the
owning module and its consumers for an existing implementation. Broaden the
search only when ownership or impact is unclear.

```bash
rg "functionName|related_keyword" path/to/affected/module
```

| Question | Decision |
| --- | --- |
| Does an existing function own the same behavior? | Reuse it, or extend it when the current contract fits. |
| Is the similarity semantic or only visual? | Share behavior that must evolve together; keep independent behavior local. |
| Do consumers repeat a protocol or constant? | Keep that contract with its owner and import it. |
| Would extraction reduce real maintenance cost? | Extract only when the resulting interface is simpler than the duplication. |

## When to Abstract

Use DRY for knowledge that must stay consistent: shared constants, protocol
decoding, and state transitions should have a clear owner. Similar text, a copy,
or an arbitrary number of occurrences alone does not justify an abstraction.

Prefer existing helpers and local patterns. Do not create utilities for imagined
future consumers, one-off operations, or trivial expressions when the new
interface adds more complexity than it removes. Components with similar markup
may remain separate when their behavior and ownership differ.

## Payload and State Ownership

Repeated casts of the same external JSON or event field can duplicate a
contract even when the code is short. Decode external data at its existing
boundary and let consumers use the resulting types or projections. Trust typed
internal data and framework guarantees; do not add another validation layer
merely to share a helper.

When multiple consumers derive the same state from `action`, `kind`, or
`status`, keep the transition logic with the state owner. Use the existing
reducer or dispatcher where appropriate; do not impose a new reducer on an
isolated branch that is already clear.

For event replay, display code and commands consume the owning replay model
instead of reimplementing its transitions. Error handling stays visible at the
boundary; shared helpers must not silently turn failures into default values.

## After Related Changes

Check only the affected owners and consumers:

- Did shared contract changes reach every relevant consumer?
- Do repeated implementations express the same behavior and need to evolve together?
- Is a new abstraction smaller and clearer than the duplication it replaces?
- Are shared constants, payload decoding, and derived state still owned in one place?
- Can any speculative option, wrapper, or helper added in this change be removed?

## Workflow Ownership

GKD owns task lifecycle and handoff rules. Project code and documentation must
use those interfaces rather than copy workflow parsers, state formats, or a
second script tree.
