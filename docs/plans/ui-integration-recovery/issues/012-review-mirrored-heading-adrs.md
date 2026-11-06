# 012: Review the mirrored-heading ADRs against shipped behavior

Status: ready-for-agent
Type: research
Blocked by: none
Parent: [001: Recovery specification](001-ui-integration-recovery-spec.md)
Related: [Plan map](../map.md), [002: Diagnose MCU rotation](002-diagnose-mcu-rotation.md), [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md), [ADR-0039](../../../adr/0039-half-frame-parts-mirrored-controller.md), [Audit A8](../../../research/2026-11-06-dev-ui-integration-audit.md)

## What to build

Review whether the ADRs that govern mirrored-part headings still describe what
Core does, and list what must change. The user suspects the ADRs were not updated
correctly when decisions changed; this ticket tests that suspicion instead of
assuming it. It was added on 2026-11-06 after 002 and 003 closed (002 on a stale
cached build, 003 `wontfix`), and it carries the A8 controller-heading
documentation item that 003 would have owned.

Known starting facts, each to be re-verified rather than trusted:

- ADR-0010's heading amendment (2026-10-10) says every independent part maps
  heading `r` to `180 - r`, with key switches as the only exception. It does not
  record the controller exception.
- Core gives a controller `-r` (`mirrored_part_heading`,
  `core/src/mechanical/pair.rs:67`, pinned by the test at `:292`), introduced by
  `40df01155`. Its comments and ticket 12 say "controllers keep the world
  heading", which holds only at 0 and 180 degrees; at 90 and 270 the mirror
  differs from the source by 180 in world terms.
- ADR-0039 says the partner takes "`-x`, mirrored heading" and does not name the
  controller exception either.

## Decision so far (user, 2026-11-06)

The user asked whether a controller at 90 or 270 degrees is mirrored as well, and
that a source MCU at 90 ends at -90 because it is mirrored. That is correct, and
it is what Core does. Reflecting a part across the vertical axis maps its frame
rotation `r` to `-r` when the part is symmetric about its own local y axis, as an
MCU is left to right. Checked numerically (reflect the placed footprint, then match
it to a rotated copy): the mirrored MCU is at `-r` at every heading.

| Source `r` | Mirrored MCU (reflection, and Core's controller rule `-r`) | ADR-0010 general rule `180 - r` |
| --- | --- | --- |
| 0 | 0 | 180 |
| 90 | 270 (-90) | 90 |
| 180 | 180 | 0 |
| 270 | 90 (-270) | 270 |

So Core's controller rule is the true reflection, and `180 - r` adds a further
half turn. `180 - r` suits parts whose facing direction is local +x (a TRRS jack
or side reset button facing outward); it does not describe a part symmetric about
local y. The unit test at `core/src/mechanical/pair.rs:292` pins `-r` and is
consistent with the user's answer. No Core change is indicated.

What is wrong is documentation wording: code comments, commit `40df01155` and
ticket 12 say controllers "keep the world heading" or "share the same heading".
That holds only at 0 and 180; at 90 and 270 the mirror is 180 degrees from the
source in world terms, as a mirror image is. The ADR-0010 amendment should state
the controller rule as the plain reflection `-r`, with no extra half turn, and say
why. ADR-0010's own verification note (2026-10-10) already observes that
`180 - r` without a flip turns a part rather than mirroring it; whether the general
rule needs revisiting for other y-symmetric parts is a separate question for this
review, not decided here.

## Acceptance criteria

- [ ] Read each ADR that states a mirrored-part or controller heading rule
  (start with 0010 and 0039; search the ADR set and the split-board-editing and
  typed-core-edits tickets for others) and record, per statement, whether the
  shipped code agrees, with file and line.
- [ ] State the controller rule at every heading, including 90 and 270 degrees,
  as the code implements it, and say which statements in docs and comments
  misdescribe it.
- [ ] Separate what is a documentation error from what needs a product decision
  (for example, whether a controller at 90 or 270 degrees should keep its world
  heading or its reflected heading). Put decisions to the user; do not change
  Core, tests or the code comments in this ticket.
- [ ] Say which ADRs, if any, are outdated beyond heading rules where the review
  happens to show it, without widening the review into a full ADR audit. Propose
  that audit as a separate ticket if the evidence warrants it.
- [ ] Propose exact ADR amendments, following the existing amendment style and
  [domain docs](../../../agents/domain.md) rules. Historical text stays; new
  decisions are dated amendments, not silent rewrites.

## Verification and completion

A written answer in this ticket with the per-statement table and the open
decisions. No code or test run is required. Apply the ADR amendments only after
the user approves them; if approved here, record them in a comment.

## Comments

- 2026-11-06: Added at the user's request after reviewing the 002 hand-off:
  "i dont think the ADRs were updated correctly".
- 2026-11-06: The user answered "yes" to whether a controller at 90 and 270
  degrees is mirrored as well, and asked whether a source MCU at 90 ends at -90.
  It does. An earlier version of this ticket wrongly called Core's `-r` a defect
  at 90 and 270, using `180 - r` as the true mirror; that was my error and is
  corrected above. No fix ticket is needed.
