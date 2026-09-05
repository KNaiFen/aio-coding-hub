# Thinking Guides

> **Purpose**: Expand your thinking to catch things you might not have considered.

---

## Why Thinking Guides?

**Most bugs and tech debt come from "didn't think of that"**, not from lack of skill:

- Didn't think about what happens at layer boundaries → cross-layer bugs
- Didn't think about code patterns repeating → duplicated code everywhere
- Didn't think about edge cases → runtime errors
- Didn't think about future maintainers → unreadable code

These guides help you **ask the right questions before coding**.

---

## Available Guides

| Guide | Purpose | When to Use |
|-------|---------|-------------|
| [Code Reuse Thinking Guide](./code-reuse-thinking-guide.md) | Identify patterns and reduce duplication | When you notice repeated patterns |
| [Cross-Layer Thinking Guide](./cross-layer-thinking-guide.md) | Think through data flow across layers | Features spanning multiple layers |
| [Upstream Merge Scope Guide](./upstream-merge-scope-guide.md) | Separate integration conflicts from upstream-origin defects | Before or during upstream merge/drift work |

---

## Quick Reference: Thinking Triggers

### When to Think About Cross-Layer Issues

- [ ] Feature changes a boundary between API, service, component, or database
- [ ] Data format changes between layers
- [ ] Multiple consumers need the same data
- [ ] You're not sure where to put some logic
- [ ] You are adding an event kind, JSONL record, RPC payload, or config field
- [ ] UI / command code starts casting raw payload fields directly

→ Read [Cross-Layer Thinking Guide](./cross-layer-thinking-guide.md)

### When to Think About Code Reuse

- [ ] You're writing similar code to something that exists
- [ ] Multiple consumers repeat the same contract or behavior
- [ ] You're adding a new field to multiple places
- [ ] You're modifying a shared constant or config
- [ ] You're creating a helper for behavior that may already have an owner
- [ ] Two files read the same untyped payload field with local casts
- [ ] Multiple branches update the same derived state from `kind` / `action`

→ Read [Code Reuse Thinking Guide](./code-reuse-thinking-guide.md)

### When Synchronizing Upstream

- [ ] You are fetching, merging, or auditing a pinned upstream revision
- [ ] Review found a defect in code imported from upstream
- [ ] A test failure may be unchanged upstream behavior rather than a merge regression
- [ ] A proposed edit is adjacent to a conflict but may not be required to resolve it

→ Read [Upstream Merge Scope Guide](./upstream-merge-scope-guide.md)

### When Verifying AI Cross-Review Results

- [ ] Reviewer claims "user input can be malicious" → Check the actual data source (internal manifest? user config? external API?)
- [ ] Reviewer flags "missing validation" → Is the data from a trusted internal source?
- [ ] Reviewer says "behavior change" → Read the code comments — is it intentional design?
- [ ] Reviewer identifies a "bug" in test → Mentally delete the feature being tested — does the test still pass? If yes → tautological test

**Common AI reviewer false-positive patterns**:
1. **Trust boundary confusion**: Treating internal data (bundled JSON manifests) as untrusted external input
2. **Ignoring design comments**: Flagging intentional behavior documented in code comments as bugs
3. **Variable misreading**: Not tracing a variable to its actual definition (e.g., Map keyed by path vs name)

**Verification rule**: Verify findings against the actual code, data source, and intended behavior before prioritizing. Do not assume a fixed false-positive rate.

---

## Search Scope Before Modification

Search definitions and consumers when changing shared configuration, protocols,
constants, or names, or when the impact is unclear. Start in the owning module
and expand across the repository only when the references or uncertainty require
it. An isolated wording or local-value change does not require a repository-wide
search.

```bash
# Search the relevant owner and consumers
rg "symbol_or_config_key" path/to/affected/module
```

Review the affected consumers together so shared changes remain consistent.

---

## How to Use This Directory

1. **Before coding**: Read a guide only when its trigger applies
2. **During coding**: If something feels repetitive or complex, check the guides
3. **After bugs**: Add new insights to the relevant guide (learn from mistakes)

---

## Contributing

Found a new "didn't think of that" moment? Add it to the relevant guide.

---

**Core Principle**: Match investigation and verification to the changed behavior.
