# 65: KLOR: match real peripheral components

Status: needs-info
Type: build
Blocked by: [64](64-klor-native-assembly.md), [57](57-measured-split-hardware-inputs.md)
Parent: [25](25-shipped-boards-review.md) · Spec: [spec.md](../spec.md)

## Source evidence

[Measured inputs](../../../../content/layouts/keyboard-layouts.json), `klor`: `https://github.com/GEIGEIGEIST/KLOR`, `PCB/klor1_3/klor1_3.kicad_pcb` at `e2f9c2cde5200189e57c4f89eeff90686cc359fe`; SHA-256 `4955cae6514bbd91c52e26a32a3b8930c4e71bbc3d18698a8dbcf5bf94d9996a`. [Phase 1 audit](../audit/shipped-boards-review.md) records the `measured-klor` baseline; re-inventory after ticket 20, because its IDs intentionally change.

The current peripheral definitions come from [recipes.rs](../../../../core/examples/demo_projects/recipes.rs), not a qualified BOM. The measured source is right-authored. Ticket 20 owns removal of the 6.5 mm outline allowance and the 6.38 mm pocket-repair asymmetry; this ticket must consume that result, not add another mirror or tolerance.

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
