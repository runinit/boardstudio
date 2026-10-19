# 03: Bound constraint solver recursion depth

Status: ready-for-agent
Type: build
Blocked by: None (can start immediately)

**What to build:** Resolving part poses across constraint chains (including framed `source_owner` and `target_owner` dependencies) enforces a bounded traversal depth or uses an explicit heap work stack, returning a descriptive error on overly deep acyclic chains instead of exhausting the native or WASM call stack.

## Acceptance criteria

- [ ] A deep acyclic constraint chain (e.g. 12,000 parts in a linear offset chain) returns a clean `Err` on open and edit rather than aborting with a stack overflow.
- [ ] Ordinary multi-step constraint chains and framed mirrored-half constraints continue to resolve to identical poses.
- [ ] Cyclic constraint graphs continue to be detected and rejected with a cycle error.
- [ ] Native tests cover both ordinary constraint chains and an over-budget deep acyclic chain.
