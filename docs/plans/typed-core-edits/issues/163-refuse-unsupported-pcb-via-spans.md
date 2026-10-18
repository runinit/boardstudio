# 163: Refuse unsupported via spans in physical PCB geometry

Status: claimed
Type: build
Priority: P2
Blocked by: None (can start immediately)
Related: [CAD 013](../../cad-backend-replacement/issues/013-layered-case-construction.md), [CAD 014](../../cad-backend-replacement/issues/014-vendor-mesh-collision-validation.md)

## What to build

Physical PCB preparation must preserve the authored meaning of drilled features.
Blind, buried and microvias must produce a named unsupported-source result until
their depth can be represented faithfully. They must not silently become cuts
through the entire PCB in exact exports or collision analysis.

The 2026-10-18 audit at `f955050c3` submitted through, blind and microvias through
the public Core board-source operation. All three returned the same round-hole
description without an error, despite the latter two spanning only the front and
first internal copper layers. The physical builder applies that description
through the board thickness.

## Acceptance criteria

- [ ] Public Core regressions reproduce the current incorrect acceptance of blind
  and microvias with partial layer spans. Include a buried span and an ordinary
  front-to-back through-via control.
- [ ] Inspect both via type and layer span before emitting a physical drill.
  Unsupported or unresolved depth produces a diagnostic naming the source feature
  and reason, using existing source-geometry errors or mechanical gates.
- [ ] Through-vias retain their authored center and diameter, including transformed
  PCB sources. Existing round pad drills and slots retain their behavior.
- [ ] Direct board-source reads and module/physical-PCB preparation propagate the
  refusal. No affected source can become an exportable exact PCB or a successful
  collision operand with an invented through-hole.
- [ ] Preserve original source data and existing independent layout/preview
  availability where the current mechanical-gate policy permits it. The fix requires
  no new partial-depth geometry schema or public API.
- [ ] Run affected Core source/module-import tests and native physical-PCB tests.
  Add focused Runtime or worker verification for any changed propagation boundary.
- [ ] Complete one combined Standards+Spec review of the finished geometry behavior
  under the CAD review policy; record checks and supported via limits.

## Comments

Created from audit finding 2 at the user's request. The smallest delivery is an
explicit refusal for unsupported depth, with ordinary through-holes preserved.
