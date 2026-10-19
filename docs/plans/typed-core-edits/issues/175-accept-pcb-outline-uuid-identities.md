# 175: Import modern PCB outline UUIDs without losing source identity

Status: resolved
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

- [x] First reproduce refusal of a valid modern-ID board and success of its
  otherwise identical legacy-ID control. After the fix both import successfully
  with equivalent contours and the exact authored primitive IDs.
- [x] Accept a nonempty modern UUID or legacy timestamp identifier for each
  supported outline primitive. Keep deterministic identity handling when both
  keys are present, consistent with existing source-ID policy.
- [x] Missing/empty and duplicate source identities still fail explicitly.
  Include modern-only and mixed-identifier duplicate controls; do not synthesize
  replacement IDs or weaken the source provenance requirement.
- [x] Exercise supported outline primitive kinds and a placed/rotated/flipped
  board through physical source extraction. Apply the source placement once,
  preserve source geometry, and retain unsupported-geometry refusals.
- [x] Fresh packaged Core evidence imports a valid current BoardStudio export
  through the real board-source/module import boundary and retains its physical
  outline/source IDs. A real KiCad load control establishes input validity;
  parser-only test doubles do not establish the complete import behavior.
- [x] Run affected physical PCB source, module import and export regression
  checks. Existing drill/via identity behavior, fabrication blockers and CAD
  qualification uncertainty remain unchanged.

## Comments

Created from the user's audit findings after primary-source research. Independent
of 174's free-text correction: the plain-text export already demonstrates this
defect. This is an identity-field compatibility fix, not permission to qualify
unsupported curves or replace exact source evidence with preview meshes.

## Answer — 2026-10-19

Outline source IDs now use the existing nonempty `tstamp`-then-`uuid` policy. A
legacy timestamp wins when both are present; an empty timestamp can fall back to
a UUID. Missing, empty and duplicate IDs still refuse import. Drill and via
identity handling is unchanged.

The native regression first failed with “Edge.Cuts primitive is missing its stable
tstamp”. All 26 module-import tests and 10 generator-export tests now pass,
including primitive coverage and independently checked rotated/flipped placement.
The first equality control was narrowed to change only the outline identifier key,
after changing unrelated footprint/pad keys correctly exposed an unrelated fixture
difference. Both UUID-removal and duplicate-admission mutants were killed, with no
compile failures.

A fresh official Core worker build exported a current board, and the real KiCad 10
`pcb export stats` command accepted it. Actual packaged `read-board-source` and
`import-module-board` requests imported that board and an identifier-key-only legacy
control with equivalent physical contours and the four exact exported UUIDs.
The packaged duplicate-UUID control refused import. Raw evidence is under
`.evidence/implement-141-175-007/175-*`.

Fresh independent Spec and Standards reviews of the scoped diff against
`1458d1220` found no actionable findings. The full six-step repository check passes
in 550.7 seconds: 54/54 commands, all 2,074 retained native names plus five new
tests, all 14 mounted crate launches, 77 page tests and actual
provider/worker/browser smoke gates. Receipt:
`.evidence/implement-141-175-007/full-current.json` (current dirty checkout,
unrelated work preserved). Implementation checkpoint:
`e89366a4d273780279d5daecbf01ef6727300ec5`.
