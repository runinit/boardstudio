# 012: Review the mirrored-heading ADRs against shipped behavior

Status: resolved
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
- [ ] Present the rules as one table by part class (key switch, controller,
  other independent part such as a facing-outward connector) and source heading,
  with the reason each class differs, so a reader no longer has to combine three
  separate statements. Cover companions (diode, LED) only as far as the review
  touches them.
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
- 2026-11-06: The user found the rules confusing because they differ by part and by
  source placement, and said (comment on the 90/270 analysis, quoted): "an MCU placed
  facing up should not be rotated (0 degrees vs -0 degrees).. at 180 degrees it ALSO
  should be rotated, we care about the x axis mirror only for components.. keyswitches
  are handled differently". The "ALSO should be rotated" wording is ambiguous (possibly
  "should not be"); pending the user's confirmation it is read as: components mirror
  position across the X axis only, a controller at 0 or 180 keeps its heading, and
  keyswitches follow their own rule. That matches Core at 0 and 180. The 180 case is
  an open question for the user.
- 2026-11-06: The user confirmed the readings (a controller keeps its heading at 0 and
  180, a source MCU at 90 ends at -90, components mirror position on X only, keyswitches
  have their own rule) and said "update ADR". Reviewed and applied below.

## Answer

**Verdict.** The ADRs were incomplete, not contradicted by Core. Core is internally
consistent: every path gives a controller `-r` and every other independent part `180 - r`
(creation, reconcile, inverse and constraint endpoints all call `mirrored_part_heading`).
No Core or test change is needed. Two things were wrong: ADR-0010 never recorded the
controller exception, and code comments describe it as "keeping the world heading".

| Statement | Code | Agrees? |
| --- | --- | --- |
| ADR-0010 heading amendment, "`180 - r` over 360 degrees" | `pair.rs:67-73` (non-controller `reflected + 180`); `derive.rs:24`; `layout.rs:2112`, `3384`, `3547` | Yes, for non-controllers |
| ADR-0010, "Key switches mirror on X only; tilt `-r`" | matrix relative-negate rule, `layout.rs:1519`; `part_anchors.rs:284` | Yes |
| ADR-0010, "Stabilisers never rotated" | `derive.rs:79`, `335`; `endpoints.rs:145` | Yes |
| ADR-0010, "Everything else follows `180 - r`" | controllers: `pair.rs:69-70`, `endpoints.rs:307`, `369`, `part_anchors.rs:299` give `-r` | **No, omitted exception** (fixed by the amendment) |
| ADR-0039, partner takes "`-x`, mirrored heading" | same helper | Incomplete: did not name the controller rule (pointer added) |
| ADR-0040, "non-controllers keep the shared half turn" | `part_anchors.rs:299` | Yes (already implies the exception) |
| Comments "share the same world rotation / faces the same way" | `pair.rs:61`; `derive.rs:20`; `layout.rs:2090`, `3363`, `4808`, test message `4840` | **Imprecise**: true only at 0 and 180 |

**Decision recorded.** A controller mirrors by plain reflection `-r` at every heading,
including 90 and 270 (user, 2026-11-06). The MCU is symmetric about its local y axis, so
this is the true mirror image; `180 - r` is that plus a half turn and applies to parts that
face an edge. An early version of this ticket wrongly called `-r` a defect at 90/270.

**Applied (user-authorized).** A dated amendment in ADR-0010 with one table by part class,
and a pointer in ADR-0039. ADR-0010's earlier text is unchanged.

**Not done.** The imprecise comments above remain; this ticket changes no code. Correcting
them is a one-line wording fix at five sites and can ride along with any change to those
files. Whether `180 - r` is right for other parts symmetric about local y (screen,
encoder) is not decided; ADR-0010's own 2026-10-10 verification note raises it. No wider
ADR audit is proposed: the review found the other heading statements consistent with Core,
but I did not check ADRs outside the mirrored-heading rules.
