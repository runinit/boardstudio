# 22: Mirrored halves integrated verification

Status: ready-for-agent
Type: task
Blocked by: [21](21-mirrored-half-web-adapter.md), [41](41-inspector-companion-mode.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md) · [16](16-native-mirrored-halves.md)

## Request

Verify the native mirrored-half feature as a whole using independently authored synthetic
documents. Shipped/demo boards and their fixtures were retired by [138](138-retire-shipped-demos.md)
and are not acceptance inputs.

## What to do

- Boundary transcript native and WASM; the full `python3 scripts/check.py` (never `release`).
- Mounted browser pass on synthetic linked halves, including a reused-PCB case: create, edit
  from each half, local exception, Undo/Redo, save and reopen; record anything not run. Define
  geometry and expected results independently; do not restore or copy demo inputs.
- Standards and Spec reviews of Core, web and data changes; plan-wide review.

## Acceptance

- [ ] Evidence recorded with commands and results; gaps listed honestly.

## Progress (2026-10-14)

`python3 scripts/check.py` passed all steps on `dev` in 272.3 s (repo 0.4, tooling 4.8, lint 4.5,
build 106.3, test 37.3, browser 119.1). This includes the native/WASM boundary transcript checks,
15 CAD browser checks, and 96 page WASM tests. The synthetic linked-half mounted workflow with a
reused PCB, save/reopen, and the required Standards, Spec, and plan-wide reviews have not been run.
Ticket 22 remains open.
