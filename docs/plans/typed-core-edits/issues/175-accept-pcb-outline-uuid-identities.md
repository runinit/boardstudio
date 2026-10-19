# 175: Import modern PCB outline UUIDs without losing source identity

Status: ready-for-agent
Type: build
Priority: P1
Blocked by: None (can start immediately)
Related: [Input-boundary audit research](../../../research/audit-input-boundaries-2026-10-18.md)

## What to build

A valid PCB using modern outline UUID fields can be imported into a module's
physical board geometry, including a board exported by BoardStudio itself.
Retain exact source-owned segment identities, geometry and placement; legacy
boards using timestamp identifiers must continue to work.

The audit's BoardStudio-exported plain-text board loads in KiCad 10 but physical
board import rejects its outline identifiers. Changing only the identifier key
from modern UUID to legacy timestamp makes the same packaged request succeed.
The IDs and outline geometry themselves are unchanged.

## Acceptance criteria

- [ ] First reproduce refusal of a valid modern-ID board and success of its
  otherwise identical legacy-ID control. After the fix both import successfully
  with equivalent contours and the exact authored primitive IDs.
- [ ] Accept a nonempty modern UUID or legacy timestamp identifier for each
  supported outline primitive. Keep deterministic identity handling when both
  keys are present, consistent with existing source-ID policy.
- [ ] Missing/empty and duplicate source identities still fail explicitly.
  Include modern-only and mixed-identifier duplicate controls; do not synthesize
  replacement IDs or weaken the source provenance requirement.
- [ ] Exercise supported outline primitive kinds and a placed/rotated/flipped
  board through physical source extraction. Apply the source placement once,
  preserve source geometry, and retain unsupported-geometry refusals.
- [ ] Fresh packaged Core evidence imports a valid current BoardStudio export
  through the real board-source/module import boundary and retains its physical
  outline/source IDs. A real KiCad load control establishes input validity;
  parser-only test doubles do not establish the complete import behavior.
- [ ] Run affected physical PCB source, module import and export regression
  checks. Existing drill/via identity behavior, fabrication blockers and CAD
  qualification uncertainty remain unchanged.

## Comments

Created from the user's audit findings after primary-source research. Independent
of 174's free-text correction: the plain-text export already demonstrates this
defect. This is an identity-field compatibility fix, not permission to qualify
unsupported curves or replace exact source evidence with preview meshes.
