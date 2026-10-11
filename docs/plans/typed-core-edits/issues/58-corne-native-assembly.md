# 58: Corne: native key assembly profile

Status: wontfix
Type: build
Blocked by: [22](22-mirrored-half-integrated-verification.md), [44](44-native-recipe-assembly-profiles.md), [57](57-measured-split-hardware-inputs.md), [56](56-sofle-sign-off.md)
Parent: [25](25-shipped-boards-review.md) · Spec: [spec.md](../spec.md)

## Source evidence

[Measured inputs](../../../../content/layouts/keyboard-layouts.json), `corne`: `https://github.com/foostan/crkbd`, `corne-cherry/pcb/corne-cherry.kicad_pcb` at `1f56ad19bf50435666d2d86269726450e3b08eff`; SHA-256 `25a860dc9c79b08fe19173a2956b163ace871eafeb2d87243318b57d540e1bc7`. [Phase 1 audit](../audit/shipped-boards-review.md) records the `measured-corne` baseline; re-inventory after ticket 20, because its IDs intentionally change.

Current profile: 42 MX switches and 42 diodes, 21 switches per half (3×6 + 3 thumbs); 19.05 × 19.05 mm base pitch, MX. No authored TH holes are present in the adapted fixture; its four closure holes per half are generated proposals.

## One-hour slice

Give this board its own named BoardStudio assembly profile using ticket 44’s native matrix path. Preserve measured key poses, widths, cluster relationships and source references; store its explicit pitch override and 1.0 mm keycap edge gap separately. A global preference must not respace a measured example. Use ticket 20’s one authored half and native correspondence. Switch tilt takes `-r`; independent parts take `180-r` over 360° through Core. Set duplicate-position diodes explicitly; LED overrides require the hardware packet. Keep side and generator parameters unchanged during mirroring.

## File ownership

One scheduled board implementer owns only this board’s branch/profile in `core/examples/demo_projects/{recipes,physical,mechanical}.rs` and its focused assertions in `core/tests/included_keyboards.rs` (or one board-specific test file). Shared helpers require [44](44-native-recipe-assembly-profiles.md) first. Generated goldens/assets are handed to the designated golden owner; see the phase 2 execution contract in ticket 25. No other board, archive, Core mirror algorithm, catalogue/footprint generator or UI edits belong here.

## Acceptance and regression

- [ ] This board references its own profile; Core materialises exactly 42 switches and the approved companions from matrix membership, with no hand-created duplicate key parts.
- [ ] A public-boundary regression fails first because this board still uses generic preset/profile data; then passes with source-reference pose correspondence within 1e-9 mm and full switch-heading comparison. Assert profile identity and duplicate diode offsets separately from the generated golden.
- [ ] Editing one native matrix value, Undo/Redo and save/reopen preserve the profile, measured exceptions and unrelated parts. Mirror edits reconcile through the native pair.

## Verification and remaining facts

Implementer: `repo lint test typecheck`; add `build` when the designated owner regenerates demo outputs. Fresh Sol verifier reproduces the regression and disables this board’s profile selection once to prove it fails. Ticket 23 owns the final mounted-browser/Standards/Spec/full-batch gate. [57](57-measured-split-hardware-inputs.md) must confirm switch/diode/LED hardware; this ticket does not certify physical fit. Split new generator or shared-algorithm work into a follow-up rather than widening this slice.

## Supersession answer (2026-10-11)

The separate per-board implementation ticket is retired. The board requirement is superseded by ticket [25](25-shipped-boards-review.md): author one ordinary saved project for this board through the existing Core/Archive/Artifact JSON request examples and common user operations. Do not add a per-board builder, branch, behavior suite, or extracted helper. Keep the source research, geometry, component evidence, and prior proposals above as historical context only; they do not authorize implementation. Shared mirrored-half/link functionality remains required separately under tickets 19/22, while board creation does not wait for 19, 22, or 44.
