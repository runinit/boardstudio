# 62: Lily58: match real peripheral components

Status: wontfix
Type: build
Blocked by: [61](61-lily58-native-assembly.md), [57](57-measured-split-hardware-inputs.md)
Parent: [25](25-shipped-boards-review.md) · Spec: [spec.md](../spec.md)

## Source evidence

[Measured inputs](../../../../content/layouts/keyboard-layouts.json), `lily58`: `https://github.com/kata0510/Lily58`, `Pro/PCB/Lily58_Pro.kicad_pcb` at `58c649797c185b7b32b9928d4546665153cfacfd`; SHA-256 `078339300973d987cb5f50bae0221c713f09d7d404fbf40783abe05a3a659615`. [Phase 1 audit](../audit/shipped-boards-review.md) records the `measured-lily58` baseline; re-inventory after ticket 20, because its IDs intentionally change.

The current peripheral definitions come from [recipes.rs](../../../../core/examples/demo_projects/recipes.rs), not a qualified BOM. The measured source is right-authored. Preserve ticket 20’s construction source and source-numbering correspondence; do not reverse columns again. No source TH holes are recorded.

## Hardware gate

[57](57-measured-split-hardware-inputs.md) must pin the target revision/configuration, part/footprint IDs, source licence/hash, datum transform, terminal mapping, side/reversibility and component/service envelope. Unanswered rows keep this ticket `needs-info`. Reuse an existing qualified library entry; a missing entry gets its own onboarding ticket under [component onboarding](../../../hardware/component-onboarding.md) before this slice starts.

## One-hour slice and ownership

Replace the recipe-selected U1/RST/TRRS definitions and fixed bay poses (y −15/−45/−65) with the approved real component identities and placements. Confirm whether the selected target actually uses a TRRS, the chosen controller and the assumed reset; absence requires an explicit approved configuration, not an invented part. Use ticket 20’s one authored half and native correspondence. Switch tilt takes `-r`; independent parts take `180-r` over 360° through Core. Set duplicate-position diodes explicitly; LED overrides require the hardware packet. Keep side and generator parameters unchanged during mirroring.

Own only this board’s peripheral selection/pose branch in `core/examples/demo_projects/recipes.rs`, its named access specifications in `mechanical.rs` and focused assertions in `core/tests/included_keyboards.rs`. Generated goldens/assets are handed to the designated golden owner; see the phase 2 execution contract in ticket 25. No other board, archive, Core mirror algorithm, catalogue/footprint generator or UI edits belong here.

## Acceptance and regression

- [ ] Each intended component has an exact library identity, recorded datum and reviewed terminal/pin assignment; pad/drill/artwork/model alignment evidence is linked from the hardware packet. No placeholder is relabelled as qualified.
- [ ] First add a public recipe/export regression that fails on this board’s old definition or datum. Assert the approved identities, pose, pad/drill/layer and terminal mapping; include a rotated/back-side or mirrored placement where applicable. Reject accidental side/parameter mutation.
- [ ] Undo/Redo and portable reopen retain selected assets, unrelated wiring and manual placements. Uncertain right-half pad geometry stays a physical gate; Core adds no pad-symmetry rule.

## Verification

`repo lint test typecheck`, plus `build` after generated-output handoff. The independent verifier changes one component selection/datum back to its old value and observes the focused regression fail, restores it, then checks both affected PCB placements. Ticket 23 retains the mounted-browser and global reviews; hardware fit/electrical qualification remains explicit until observed.

## Supersession answer (2026-10-11)

The separate per-board implementation ticket is retired. The board requirement is superseded by ticket [25](25-shipped-boards-review.md): author one ordinary saved project for this board through the existing Core/Archive/Artifact JSON request examples and common user operations. Do not add a per-board builder, branch, behavior suite, or extracted helper. Keep the source research, geometry, component evidence, and prior proposals above as historical context only; they do not authorize implementation. Shared mirrored-half/link functionality remains required separately under tickets 19/22, while board creation does not wait for 19, 22, or 44.
