# 30: Aggregate part validation and lifecycle parity

Status: claimed
Type: build
Blocked by: [28](28-aggregate-anchor-core.md) (aggregate shape and evaluation)
Parent: [deferred plan](../deferred-items-plan.md) · Research: [deferred-items-research.md](../deferred-items-research.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)
Ownership: Plan slices A2 (validation) and A5 (lifecycle) only. Does not
change evaluation, Follow, reflection, fixtures, or UI.

## Outcome

Aggregate anchors refuse atomically with the document unchanged: unknown part
keys, empty part lists, `intersect` with ≠2 parts, unknown methods, and
single+aggregate both-arms documents (admission grammar from ticket 28 is
reused, not reworked). Compaction remaps every part reference to the surviving
physical key (research §1.4.5 — remap, never detach-on-`>=`-slot); removal or
shrink past any part detaches the dependant keeping its world frame (spec §4
rule, extended from single-key).

## First RED / GREEN slice

Fail one public-route refusal (unknown part key) asserting the exact error,
no document change, and no history entry; then implement the refusal style
following ticket-19's atomic-refusal precedent in
`core/src/matrix/anchors.rs::validate_target` extended to a part list.

## Subsequent acceptance

- One proving test per refusal: unknown part, empty parts, `intersect` with
  1 and 3 parts, unknown method, both-arms coexistence — all via public
  routes, all asserting document-unchanged.
- Compaction shifts part references to surviving keys (parts past the slot
  shift, none detach); removal/shrink past any part detaches the dependant
  with pose preserved in one Undo entry. Seams: `anchors.rs` (remap/shift),
  `deletion.rs` (ticket-21 grouped-edit exclusion pattern).

## Checks

Core integration tests via public `Open`/`Edit` routes (spec §11 seam rule);
affected quick/browser checks. Normal tests do not invoke Ergogen/Node.
