# 015: Decide headings for other y-symmetric parts

Status: needs-info
Type: decision
Blocked by: none
Parent: [001: Recovery specification](001-ui-integration-recovery-spec.md)
Related: [012](012-review-mirrored-heading-adrs.md), [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## What to decide

Should independent parts symmetric about their local y axis, such as screens and encoders, retain the current `180 - r` rule or use plain reflection `-r`? ADR-0010's 2026-10-10 verification note raises the question. No change is implied until the user decides.

## Acceptance criteria

- [ ] User chooses the intended rule and identifies the affected part classes.
- [ ] Record rationale and any implementation or documentation follow-up separately.

## Verification

Compare representative headings at 0, 90, 180 and 270 degrees with the chosen physical orientation and current Core rule; record the user's decision and examples.

## Comments

- 2026-11-06: User comment (quoted): "screen, encoders, and basically any component
  will rotate if its not at 0 or 180 already.. however if the pcb footprint is marked
  'reversible' and is currently being used as a reversible footprint in the design, we
  can technically 'flip' the components footprint.. this is a bit nuanced and getting
  into the weeds a bit.. the physical placement of that part will be mirrored, but the
  actually footprint would get flipped on the x axis".

  Read as: screens and encoders take plain reflection `-r` (unchanged at 0 and 180,
  turned only at other headings), not `180 - r`. That would differ from Core today,
  which gives `180 - r` to every non-controller. The reversible-footprint flip is
  recorded as a possible later refinement, not part of this decision, and is not
  specified.

  **Not settled, so this stays `needs-info`:** "basically any component" may or may not
  include parts that face an edge, such as a TRRS jack or a side reset button. ADR-0010
  (2026-10-10, the user's description) turns those through 180 degrees so they still
  face outward; under `-r` a jack at 0 would keep facing the same way and so face
  inward on the mirrored half unless its reversible footprint is flipped. Asked of the
  user: do TRRS/reset-style edge-facing parts keep `180 - r`, or do all non-switch
  components take `-r` (with the reversible flip handling the facing)?

  No ADR or Core change until that is answered; then a follow-up ticket would change
  `mirrored_part_heading` from a controller flag to a part class and amend ADR-0010.
