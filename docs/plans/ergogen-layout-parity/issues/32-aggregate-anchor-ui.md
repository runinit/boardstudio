# 32: Aggregate anchor UI (Inspector, Pick, Relations)

Status: resolved
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

## Answer

Resolved on branch `docs/ergogen-layout-comparison` jointly with tickets
28–31. Slice A9 landed as RED `cbfae9086` / `82b82b7af` / `66c99af65` /
`0455a6e36` / `2add61adb` / `1c9bce631` then GREEN `b73d17d12` (web adapts
single-key anchors to the committed aggregate arm), `d2be8c2f0` (Anchor
section part links), `5fd3d9942` (method switch), `5a0506721` (part removal),
`50dcb968e` (canvas multi-Pick with draft/commit/Esc) and `223e152c5`
(Relations multi-contributor chains, hover/leave proof; format `4f569c483`,
style `bcc7381d9`). No new drag gesture was added, so no trusted-pointer
proof was owed. The dual review (spec + standards) PASSED with no open
findings. Joint native baseline `6c0e4f30` (+53/−0). Joint gates green: quick
`check: all steps passed: repo, tooling, lint, typecheck, test, browser`
(468.3s) and full
`check: all steps passed: repo, tooling, lint, build, test, browser`
(419.5s).

## Checks

Mounted browser tests for the new surfaces; trusted-pointer proof only where
a new drag gesture is added (reuse ticket-23 smoke if so, else state why not);
affected quick/browser checks. Prototype/ untouched.
