# 22: Mirrored halves integrated verification

Status: ready-for-agent
Type: task
Blocked by: [20](20-recipes-build-one-half.md), [21](21-mirrored-half-web-adapter.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md) · [16](16-native-mirrored-halves.md)

## Request

Verify the native mirrored half as a whole before it replaces the old shipped data.

## What to do

- Boundary transcript native and WASM; the full `python3 scripts/check.py` (never `release`).
- Live pass on two shipped splits and the reused-PCB case: create, edit from each half,
  local exception, Undo/Redo, save and reopen; record anything not run.
- Standards and Spec reviews of Core, web and data changes; plan-wide review.

## Acceptance

- [ ] Evidence recorded with commands and results; gaps listed honestly.
