# 011: Measure integrated CAD performance

Status: ready-for-agent
Type: build
Blocked by: 010: Production cache, worker and browser integration

## What to build

Measure the integrated replacement under comparable conditions and correct material performance regressions without weakening accuracy.

## Acceptance criteria

- [ ] Bind the Cadrum baseline and Monstertruck runs to source, artifacts, workload, options, environment and cache state.
- [ ] Measure build time/size, startup, representative cold/warm CAD operations and memory; coordinate expensive runs to avoid incomparable load.
- [ ] Report measured deltas and correct material regressions; do not invent numerical acceptance limits or trade away geometry/accuracy silently.
- [ ] Verify any performance changes with meaningful focused regressions and both review axes, retaining required native/WASM/browser acceptance.

## Comments

2026-10-17: User approved this follow-on slice and its blocking edges. Existing 001–004 assignments and evidence continue unchanged. Qualification milestones do not establish production acceptance.
