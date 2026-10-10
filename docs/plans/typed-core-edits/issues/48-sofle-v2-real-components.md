# 48: Sofle v2: match real peripheral components

Status: needs-info
Type: build
Blocked by: [47](47-sofle-v2-native-assembly.md), [46](46-sofle-hardware-inputs.md)
Parent: [25](25-shipped-boards-review.md) · Spec: [spec.md](../spec.md)

## Source evidence

[Sofle measured inputs](../../../../content/layouts/sofle-layouts.json), `layouts.v2`: `Sofle_v2/PCB/SofleKeyboard.kicad_pcb` at `fb294f7c58d0f91c379a9ef339159d4d056643e9`; SHA-256 `e34c4981c039bd5c0ab8613c7454422cae117f0dc320b99cb37fd26592d36cce`. [Phase 1 audit](../audit/shipped-boards-review.md) records the `sofle` baseline; re-inventory after ticket 20, because its IDs intentionally change.

The current peripheral definitions come from [recipes.rs](../../../../core/examples/demo_projects/recipes.rs), not a qualified BOM. TH1–TH7 are seven source mounting holes per half, not closure mounts. The current v2 fixture uses a gasket and has no closure mounts; this does not choose the rebuilt mounting system.

## Hardware gate

[46](46-sofle-hardware-inputs.md) must pin the target revision/configuration, part/footprint IDs, source licence/hash, datum transform, terminal mapping, side/reversibility and component/service envelope. Unanswered rows keep this ticket `needs-info`. Reuse an existing qualified library entry; a missing entry gets its own onboarding ticket under [component onboarding](../../../hardware/component-onboarding.md) before this slice starts.

## One-hour slice and ownership

Match U1 controller/socket, SW25 encoder, J3 display, J2 TRRS and RSW1 reset to the approved parts. Account explicitly for the current source-to-library datum offsets: J2 +4 mm, RSW1 +1 mm, J3 (+3.81, −16.7) mm, and the source v2 U1 heading 270° versus recipe 0°. Preserve TH1–TH7 source-hole identities and select their actual drill/hardware dimensions. Use ticket 20’s one authored half and native correspondence. Switch tilt takes `-r`; independent parts take `180-r` over 360° through Core. Set duplicate-position diodes explicitly; LED overrides require the hardware packet. Keep side and generator parameters unchanged during mirroring.

Own only this board’s peripheral selection/pose branch in `core/examples/demo_projects/recipes.rs`, its named access specifications in `mechanical.rs` and focused assertions in `core/tests/included_keyboards.rs`. Generated goldens/assets are handed to the designated golden owner; see the phase 2 execution contract in ticket 25. No other board, archive, Core mirror algorithm, catalogue/footprint generator or UI edits belong here.

## Acceptance and regression

- [ ] Each intended component has an exact library identity, recorded datum and reviewed terminal/pin assignment; pad/drill/artwork/model alignment evidence is linked from the hardware packet. No placeholder is relabelled as qualified.
- [ ] First add a public recipe/export regression that fails on this board’s old definition or datum. Assert the approved identities, pose, pad/drill/layer and terminal mapping; include a rotated/back-side or mirrored placement where applicable. Reject accidental side/parameter mutation.
- [ ] Undo/Redo and portable reopen retain selected assets, unrelated wiring and manual placements. Uncertain right-half pad geometry stays a physical gate; Core adds no pad-symmetry rule.

## Verification

`repo lint test typecheck`, plus `build` after generated-output handoff. The independent verifier changes one component selection/datum back to its old value and observes the focused regression fail, restores it, then checks both affected PCB placements. Ticket 23 retains the mounted-browser and global reviews; hardware fit/electrical qualification remains explicit until observed.
