# 03: Bound constraint solver recursion depth

Status: ready-for-agent
Type: build
Blocked by: None (can start immediately)

**What to build:** Resolving part poses enforces one documented conservative active dependency-depth limit across source authority, framed `source_owner`/`target_owner` dependencies and implicit assembly parents. Over-budget traversal returns a descriptive error instead of exhausting the native or WASM call stack.

## Research decision (2026-10-19)

Keep: cycle detection does not bound an acyclic recursion path. The smallest change is a named active-depth budget at `pose` entry, reusing the active-path set where practical; preserve completed-node memoization, precedence, poses and cycle detection. Check for a repeated active node before reporting depth exhaustion. An iterative solver rewrite is not required for this fix. Select the numerical limit with native and packaged-WASM boundary tests, not an assumed portable stack-frame count. See [research 03](../../../research/rust-best-practices-ticket-validation.md#03--constraint-recursion).

The deep fixture must force traversal: the lexicographically first target should depend on later targets. Source-first IDs can let the `BTreeMap` loop resolve every predecessor from cache and never exercise deep recursion.

## Acceptance criteria

- [ ] A deep acyclic constraint chain (e.g. 12,000 parts in a linear offset chain) returns a clean `Err` on open and edit rather than aborting with a stack overflow.
- [ ] Ordinary multi-step constraint chains and framed mirrored-half constraints continue to resolve to identical poses.
- [ ] Cyclic constraint graphs continue to be detected and rejected with a cycle error.
- [ ] Native tests cover below/at/above-budget chains, both framed-owner dependency edges, and implicit assembly parents. Deliberately reverse-ordered deep IDs prove that memoization cannot bypass the fixture's active-depth test.
- [ ] Refused Open/edit preserves the previously accepted document and history. Packaged-WASM tests exercise the boundary and deep refusal without a stack trap, establishing that the selected budget is conservative for both targets.
