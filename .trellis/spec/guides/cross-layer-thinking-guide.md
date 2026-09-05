# Cross-Layer Thinking Guide

> **Purpose**: Trace the boundaries affected by a change before implementing it.

Use this guide when data formats, ownership, or behavior cross a layer boundary.
Read the relevant project contracts; a local change does not require every
checklist or unrelated contract.

## Map the Affected Data Flow

```text
Source -> Transform -> Store -> Retrieve -> Transform -> Display
```

For each affected boundary, identify the input, output, owner, and possible
errors. A short trace is enough unless complex branching or state transitions
need a separate diagram.

| Boundary | What to Check |
| --- | --- |
| API / service | Field types, optional fields, and error meaning |
| Service / database | Conversions, persistence, and round trips |
| Rust / TypeScript / UI | Serialization, generated bindings, and consumer behavior |
| Component / component | Props, state ownership, and derived values |

## Keep Contracts With Their Owners

- Convert formats explicitly at the boundary that owns the conversion.
- Validate user input, external APIs, and network data at the entry point.
  Trust typed internal code and framework guarantees instead of repeating checks
  in every layer.
- Keep external payload decoding and types with the data owner. Consumers
  format typed data without redefining its contract through local casts.
- Keep derived state and transitions with their existing owner. Share code
  when it removes duplicated behavior, without introducing speculative helpers.
- Preserve error meaning across layers. Distinguish absent data from failed
  reads; do not swallow errors or silently invent defaults.

## Verification for Changed Boundaries

- Trace the affected producers and consumers, including alternative entry paths
  that use the changed result.
- Check relevant empty, invalid, or failed external inputs and confirm errors
  remain visible at the owning boundary.
- Verify persisted or serialized data survives the required round trip.
- Check generated bindings and their consumers when a public type changes.
- Use necessary regression coverage for changed user workflows and core behavior.
  Do not add tests that merely repeat a small implementation detail.
- Run verification only through the project-authorized local checks and GitHub
  Actions gates; this guide does not create an additional runner or full-suite rule.

## Generated or Runtime-Parsed Data

When changing a generated file or a template consumed at runtime, identify its
owning generator and parser and update the relevant contract. Keep transient
runtime state out of Git. GKD owns workflow handoffs; do not copy external
workflow templates or add a project parser for workflow state.

## Remote Probes and Download Modes

When a CLI or adapter chooses behavior from a remote response:

- Trace all entry paths that use the probe result, including noninteractive and
  shortcut paths.
- Distinguish not-found responses from transient failures; propagate failures
  or use the protocol's defined retry behavior.
- Reset source-specific cached or prefetched data when the source changes.
- Parse a complete response or use a streaming parser; a fixed-size prefix is
  not a complete JSON document.
- Preserve every field and its position when rebuilding a protocol identifier.
- Verify shortcut consumers preserve the same error distinctions as the probe.

## Event Logs and Projections

When changing an event kind, field, filter, or replay model, trace the writer,
reader, reducer, and affected display consumers. Keep event types and external
normalization at the event boundary, identifier assignment in the writer, and
state projection in the existing reducer. Derived state should refer to the
source event identifier instead of introducing a second cursor.

Where replay and live filtering must agree, verify that changed behavior
through the same owning model. Add regression coverage when the change affects
that user-visible contract; unrelated changes do not trigger replay tests.
