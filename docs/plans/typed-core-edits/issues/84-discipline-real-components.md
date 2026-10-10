# 84: Discipline: match real peripheral components

Status: needs-info
Type: build
Blocked by: [83](83-discipline-native-assembly.md), [79](79-unibody-hardware-inputs.md)
Parent: [25](25-shipped-boards-review.md) · Spec: [spec.md](../spec.md)

## Source evidence

[Measured inputs](../../../../content/layouts/keyboard-layouts.json), `discipline`: `https://github.com/coseyfannitutti/discipline`, `discipline-pcb.kicad_pcb` at `f3b887110dab14ada007cea96483e4f3e65ca78c`; SHA-256 `1fb4e0691fd960440c3bbffb53d03528750075b75b8363f7f0ea90fafb045910`. [Phase 1 audit](../audit/shipped-boards-review.md) records the `measured-discipline` baseline; re-inventory after ticket 20, because its IDs intentionally change.

The current peripheral definitions come from [recipes.rs](../../../../core/examples/demo_projects/recipes.rs), not a qualified BOM. The current stabiliser count is three 2u and one 6.25u, not four 2u. Preserve measured ANSI widths and key references.

## Hardware gate

[79](79-unibody-hardware-inputs.md) must pin the target revision/configuration, part/footprint IDs, source licence/hash, datum transform, terminal mapping, side/reversibility and component/service envelope. Unanswered rows keep this ticket `needs-info`. Reuse an existing qualified library entry; a missing entry gets its own onboarding ticket under [component onboarding](../../../hardware/component-onboarding.md) before this slice starts.

## One-hour slice and ownership

Replace the generated nice!nano U1 and reset RST with the approved target controller and reset identities/poses. Preserve any approved native adaptation and document its difference from the source board. Do not reproduce an entire upstream through-hole circuit, routing or BOM in this component-selection slice. Keep one unibody board and independent mechanical instance. Multiple key clusters do not imply a mirror pair. Preserve legitimate wiring and wide-key/member relationships.

Own only this board’s peripheral selection/pose branch in `core/examples/demo_projects/recipes.rs`, its named access specifications in `mechanical.rs` and focused assertions in `core/tests/included_keyboards.rs`. Generated goldens/assets are handed to the designated golden owner; see the phase 2 execution contract in ticket 25. No other board, archive, Core mirror algorithm, catalogue/footprint generator or UI edits belong here.

## Acceptance and regression

- [ ] Each intended component has an exact library identity, recorded datum and reviewed terminal/pin assignment; pad/drill/artwork/model alignment evidence is linked from the hardware packet. No placeholder is relabelled as qualified.
- [ ] First add a public recipe/export regression that fails on this board’s old definition or datum. Assert the approved identities, pose, pad/drill/layer and terminal mapping; include a rotated/back-side or mirrored placement where applicable. Reject accidental side/parameter mutation.
- [ ] Undo/Redo and portable reopen retain selected assets, unrelated wiring and manual placements. Uncertain right-half pad geometry stays a physical gate; Core adds no pad-symmetry rule.

## Verification

`repo lint test typecheck`, plus `build` after generated-output handoff. The independent verifier changes one component selection/datum back to its old value and observes the focused regression fail, restores it, then checks both affected PCB placements. Ticket 23 retains the mounted-browser and global reviews; hardware fit/electrical qualification remains explicit until observed.
