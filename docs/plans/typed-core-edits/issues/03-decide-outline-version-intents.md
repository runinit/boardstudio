# 03: Decide the outline version intents

Status: ready-for-human
Type: grilling
Blocked by: 01, [module deepening 06](../../module-deepening/issues/06-split-outline-lifecycle.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Source: [review candidate 06](../../../investigations/architecture-review-2026-10-07.html#c6)

## Question

Which typed Core intents replace the outline action planner's whole-document
replacements, and what do they own?

The planner (`plan_action`, separated from UI by module deepening 06) allocates outline
version and entity IDs, checks polygon validity, and owns active-version selection,
cutouts, bridges, protected gaps and corner finishing. The map lists this as the
strongest outline-rule duplication, with broader semantics than a scalar edit.

## Decide

1. One intent per `OutlineAction` variant, or a typed batch intent for actions that
   change several features atomically?
2. Who allocates new version and entity IDs: Core (deterministic from the document) or
   the resolver (seeded, as today)?
3. Validity: which polygon checks become Core edit errors, and which stay as resolver
   retire reasons?
4. Changed IDs and outline recomputation: which intents affect outlines (see
   `affects_outline`, `core/src/lib.rs`)?
5. Domain terms touched (outline version, active outline, protected gap, outline
   bridge): any sharpening goes into `CONTEXT.md`; ADR-0001/0002 stand.

## Done when

`## Answer` records each decision and the build tickets that follow are sliced into
this effort.
