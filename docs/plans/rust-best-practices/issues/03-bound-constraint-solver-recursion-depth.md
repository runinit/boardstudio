# 03: Bound constraint solver recursion depth

Status: resolved
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

## Answer

Resolved on `t3/implement-rust-best-practices` (base `c071dedcf`).

- `constraints::pose` checks a repeated active node for a cycle first, then enforces one named
  conservative active-dependency budget (`MAX_DEPENDENCY_DEPTH = 256`) before inserting the
  node. Every recursion edge (source authority, framed source/target owner and implicit
  assembly parent) goes through `pose`, so the budget covers them all while completed-node
  memoization and precedence are unchanged.
- Native unit tests cover below/at/above budget, a 12,000-part reverse-ordered chain, both
  framed-owner edges, assembly parents, and cycle precedence at the budget. Public Core tests
  cover a 12,000-part Open refusal, at-budget Open plus above-budget edit refusal with
  preserved document/history, and a refused deep Open. The packaged-WASM host test exercises
  the 256/257 boundary and a 2,000-part refusal without a trap.

Evidence: `cargo test -p boardstudio_core --locked --lib constraints::tests` (10 passed),
`--test integration constraint_depth` (4 passed), host browser crate 41 passed including
`wasm_constraint_depth_boundary_is_conservative`.
