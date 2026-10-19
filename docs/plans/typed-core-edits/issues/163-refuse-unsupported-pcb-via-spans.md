# 163: Refuse unsupported via spans in physical PCB geometry

Status: resolved
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

- [x] Public Core regressions reproduce the current incorrect acceptance of blind
  and microvias with partial layer spans. Include a buried span and an ordinary
  front-to-back through-via control.
- [x] Inspect both via type and layer span before emitting a physical drill.
  Unsupported or unresolved depth produces a diagnostic naming the source feature
  and reason, using existing source-geometry errors or mechanical gates.
- [x] Through-vias retain their authored center and diameter, including transformed
  PCB sources. Existing round pad drills and slots retain their behavior.
- [x] Direct board-source reads and module/physical-PCB preparation propagate the
  refusal. No affected source can become an exportable exact PCB or a successful
  collision operand with an invented through-hole.
- [x] Preserve original source data and existing independent layout/preview
  availability where the current mechanical-gate policy permits it. The fix requires
  no new partial-depth geometry schema or public API.
- [x] Run affected Core source/module-import tests and native physical-PCB tests.
  Add focused Runtime or worker verification for any changed propagation boundary.
- [x] Complete one combined Standards+Spec review of the finished geometry behavior
  under the CAD review policy; record checks and supported via limits.

## Comments

Created from audit finding 2 at the user's request. The smallest delivery is an
explicit refusal for unsupported depth, with ordinary through-holes preserved.

## Answer — 2026-10-18

Implemented in `76ec07481`: physical drills accept only omitted via type with
exactly one F.Cu/B.Cu span, either order. Blind/buried, micro, unknown types and
missing, partial, wildcard or ambiguous spans return a named source refusal.
Legacy locked flags and tstamp/uuid identity remain supported. Through-via
center/diameter and transformed source geometry, pad drills and slots are kept.

Public Core RED reproduced the invented through-drill. GREEN includes 22 module
import tests, 14 native PCB tests and two Runtime source-PCB browser tests.
Mechanical gates preserve independent source/layout/circuit data while refusing
exact physical preparation and export; no nominal fallback or public API change.
The parent also checked the upstream KiCad via grammar.

Integrated verification at `ed7fe9d5f`: `python3 scripts/check.py` passed
**repo, tooling, lint, build, test and browser** in **1005.7 s**, including native
name guards and freshly built provider/page artifacts. Logs: `.evidence/tickets162-165/full-check.log`;
stream RED/GREEN and mutation receipts: `.evidence/tickets162-165/streams/`.

Claude Opus medium combined closure review: **Standards zero findings; Spec
zero blocking findings**. The initial low eviction concern was withdrawn after
reviewing the existing bounded retry policy and LRU regression. Its optional
error-wording suggestion remains outside this slice; the capacity behavior is
recorded in ticket 164. Reports: `.evidence/tickets162-165/opus-final-review-round1.md`
and `opus-final-review-round2.md`. Review used source inspection and parent test
evidence. Broader CAD migration and performance acceptance remain outside these
four tickets.
