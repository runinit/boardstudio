# 50: Sofle RGB: native key assembly profile

Status: wontfix
Type: build
Blocked by: [22](22-mirrored-half-integrated-verification.md), [44](44-native-recipe-assembly-profiles.md), [46](46-sofle-hardware-inputs.md), [49](49-sofle-v2-native-mechanics.md)
Parent: [25](25-shipped-boards-review.md) · Spec: [spec.md](../spec.md)

## Source evidence

Sofle measured inputs (`content/layouts/sofle-layouts.json`, retired), `layouts.rgb`: `Sofle_RGB/PCB/SofleKeyboard.kicad_pcb` at `fb294f7c58d0f91c379a9ef339159d4d056643e9`; SHA-256 `2162c58ea07f378e4352421c9d2bed31b774a08539114df97515d9f06c058739`. [Phase 1 audit](../audit/shipped-boards-review.md) records the `sofle-rgb` baseline; re-inventory after ticket 20, because its IDs intentionally change.

Current profile: MX hotswap switches, 58 diodes and 58 matrix SK6812 LEDs; 19.05 × 19.05 mm base pitch, MX. Keep the 14 measured D31–D37 LEDs separate from the 58 matrix LED companions. Matrix LEDs currently use reverse_mount:true; D31–D37 use false. TH1–TH7 remain source holes on each half.

## One-hour slice

Give this board its own named BoardStudio assembly profile using ticket 44’s native matrix path. Preserve measured key poses, widths, cluster relationships and source references; store its explicit pitch override and 1.0 mm keycap edge gap separately. A global preference must not respace a measured example. Use ticket 20’s one authored half and native correspondence. Switch tilt takes `-r`; independent parts take `180-r` over 360° through Core. Set duplicate-position diodes explicitly; LED overrides require the hardware packet. Keep side and generator parameters unchanged during mirroring.

## File ownership

One scheduled board implementer owns only this board’s branch/profile in `core/examples/demo_projects/{recipes,physical,mechanical}.rs` and its focused assertions in `core/tests/included_keyboards.rs` (or one board-specific test file). Shared helpers require [44](44-native-recipe-assembly-profiles.md) first. Generated goldens/assets are handed to the designated golden owner; see the phase 2 execution contract in ticket 25. No other board, archive, Core mirror algorithm, catalogue/footprint generator or UI edits belong here.

## Acceptance and regression

- [ ] This board references its own profile; Core materialises exactly 58 switches and the approved companions from matrix membership, with no hand-created duplicate key parts.
- [ ] A public-boundary regression fails first because this board still uses generic preset/profile data; then passes with source-reference pose correspondence within 1e-9 mm and full switch-heading comparison. Assert profile identity and duplicate diode offsets separately from the generated golden.
- [ ] Editing one native matrix value, Undo/Redo and save/reopen preserve the profile, measured exceptions and unrelated parts. Mirror edits reconcile through the native pair.

## Verification and remaining facts

Implementer: `repo lint test typecheck`; add `build` when the designated owner regenerates demo outputs. Fresh Sol verifier reproduces the regression and disables this board’s profile selection once to prove it fails. Ticket 23 owns the final mounted-browser/Standards/Spec/full-batch gate. [46](46-sofle-hardware-inputs.md) must confirm switch/diode/LED hardware; this ticket does not certify physical fit. Split new generator or shared-algorithm work into a follow-up rather than widening this slice.

## Supersession answer (2026-10-11)

The separate per-board implementation ticket is retired. The board requirement is superseded by ticket [25](25-shipped-boards-review.md): author one ordinary saved project for this board through the existing Core/Archive/Artifact JSON request examples and common user operations. Do not add a per-board builder, branch, behavior suite, or extracted helper. Keep the source research, geometry, component evidence, and prior proposals above as historical context only; they do not authorize implementation. Shared mirrored-half/link functionality remains required separately under tickets 19/22, while board creation does not wait for 19, 22, or 44.
