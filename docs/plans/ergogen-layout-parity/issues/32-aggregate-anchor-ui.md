# 32: Aggregate anchor UI (Inspector, Pick, Relations)

Status: claimed
Type: build
Blocked by: [28](28-aggregate-anchor-core.md) (Core aggregate facts)
Parent: [deferred plan](../deferred-items-plan.md) · Research: [deferred-items-research.md](../deferred-items-research.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)
Ownership: Plan slice A9 only. Core facts are consumed, never recomputed;
Inspector/Objects/Relations surfaces stay owned by
[23](23-inspector-toolbar-and-handles.md) / [24](24-relations-and-highlights.md)
conventions. No Core behavior changes in this ticket.

## Outcome

The Inspector Anchor section edits aggregate method and parts, including a
canvas multi-Pick reusing ticket 23's armed-pick pattern; Objects lists N part
links; Relations explains multi-contributor chains from Core-published facts
(spec §8). Part links navigate without writing; refusals surface Core's
reason strings.

## First RED / GREEN slice

Fail a mounted test: opening the Anchor section of an average-anchored matrix
lists its two part links, and activating a part link navigates to the part key
with revision unchanged and no Undo entry.

## Subsequent acceptance

- Method switch (average/intersection) writes one Core intent with one Undo
  entry; invalid edits surface the Core refusal inline.
- Canvas multi-Pick adds parts to the pending aggregate; Esc cancels with
  nothing written.
- Relations renders the multi-contributor chain (one entry per part) from
  Core facts; hovering a part entry highlights the part key; leave clears.
- No client-side geometry: poses, means, and intersections are displayed
  from Core-published values only.

## Checks

Mounted browser tests for the new surfaces; trusted-pointer proof only where
a new drag gesture is added (reuse ticket-23 smoke if so, else state why not);
affected quick/browser checks. Prototype/ untouched.
