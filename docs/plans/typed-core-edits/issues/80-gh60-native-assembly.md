# 80: GH60: native key assembly profile

Status: wontfix
Type: build
Blocked by: [22](22-mirrored-half-integrated-verification.md), [44](44-native-recipe-assembly-profiles.md), [79](79-unibody-hardware-inputs.md), [78](78-cantor-native-mechanics.md)
Parent: [25](25-shipped-boards-review.md) · Spec: [spec.md](../spec.md)

## Source evidence

Measured inputs (`content/layouts/keyboard-layouts.json`, retired), `gh60`: `https://github.com/komar007/gh60`, `keyboard.kicad_pcb` at `e7948cf44dbd3d43eb70296023a6fe3f471b44ba`; SHA-256 `a99aafb9f1ca77c5cac67037d161fa849b9b200de5dec9f63964520af3ff3daf`. [Phase 1 audit](../audit/shipped-boards-review.md) records the `measured-gh60` baseline; re-inventory after ticket 20, because its IDs intentionally change.

Current profile: 61 MX switches and 61 diodes; 19.05 × 19.05 mm base pitch, MX. One 6.25u and four 2u stabilisers are in the current fixture. Retain key widths and select the approved stabiliser variant/mounting datums.

## One-hour slice

Give this board its own named BoardStudio assembly profile using ticket 44’s native matrix path. Preserve measured key poses, widths, cluster relationships and source references; store its explicit pitch override and 1.0 mm keycap edge gap separately. A global preference must not respace a measured example. Keep one unibody board and independent mechanical instance. Multiple key clusters do not imply a mirror pair. Preserve legitimate wiring and wide-key/member relationships. Preserve one 6.25u and four 2u stabilisers and their qualified family/heading contract; ticket 39 on integration owns the shared stabiliser rule.

## File ownership

One scheduled board implementer owns only this board’s branch/profile in `core/examples/demo_projects/{recipes,physical,mechanical}.rs` and its focused assertions in `core/tests/included_keyboards.rs` (or one board-specific test file). Shared helpers require [44](44-native-recipe-assembly-profiles.md) first. Generated goldens/assets are handed to the designated golden owner; see the phase 2 execution contract in ticket 25. No other board, archive, Core mirror algorithm, catalogue/footprint generator or UI edits belong here.

## Acceptance and regression

- [ ] This board references its own profile; Core materialises exactly 61 switches and the approved companions from matrix membership, with no hand-created duplicate key parts.
- [ ] A public-boundary regression fails first because this board still uses generic preset/profile data; then passes with source-reference pose correspondence within 1e-9 mm and full switch-heading comparison. Assert profile identity and duplicate diode offsets separately from the generated golden.
- [ ] Editing one native matrix value, Undo/Redo and save/reopen preserve the profile, measured exceptions and unrelated parts.

## Verification and remaining facts

Implementer: `repo lint test typecheck`; add `build` when the designated owner regenerates demo outputs. Fresh Sol verifier reproduces the regression and disables this board’s profile selection once to prove it fails. Ticket 23 owns the final mounted-browser/Standards/Spec/full-batch gate. [79](79-unibody-hardware-inputs.md) must confirm switch/diode/LED hardware; this ticket does not certify physical fit. Split new generator or shared-algorithm work into a follow-up rather than widening this slice.

## Supersession answer (2026-10-11)

The separate per-board implementation ticket is retired. The board requirement is superseded by ticket [25](25-shipped-boards-review.md): author one ordinary saved project for this board through the existing Core/Archive/Artifact JSON request examples and common user operations. Do not add a per-board builder, branch, behavior suite, or extracted helper. Keep the source research, geometry, component evidence, and prior proposals above as historical context only; they do not authorize implementation. Shared mirrored-half/link functionality remains required separately under tickets 19/22, while board creation does not wait for 19, 22, or 44.
