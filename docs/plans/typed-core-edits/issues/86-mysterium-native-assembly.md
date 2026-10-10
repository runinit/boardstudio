# 86: Mysterium: native key assembly profile

Status: ready-for-agent
Type: build
Blocked by: [22](22-mirrored-half-integrated-verification.md), [44](44-native-recipe-assembly-profiles.md), [79](79-unibody-hardware-inputs.md), [85](85-discipline-native-mechanics.md)
Parent: [25](25-shipped-boards-review.md) · Spec: [spec.md](../spec.md)

## Source evidence

[Measured inputs](../../../../content/layouts/keyboard-layouts.json), `mysterium`: `https://github.com/coseyfannitutti/mysterium`, `mysterium-pcb.kicad_pcb` at `5e5ab18d8d9608759e5f50ad9e62fc7329271551`; SHA-256 `dee1f0f51fdf5ebf93690aee18d2e1bd6d435ad0588a80b0d23e7e0a75e031a1`. [Phase 1 audit](../audit/shipped-boards-review.md) records the `measured-mysterium` baseline; re-inventory after ticket 20, because its IDs intentionally change.

Current profile: 87 MX switches and 87 diodes across function, keys, navigation and arrows; 19.05 × 19.05 mm base pitch, MX. There is no numpad matrix. The current fixture contains four 2u and one 6.25u stabilisers; preserve the separate cluster origins and wide-key geometry.

## One-hour slice

Give this board its own named BoardStudio assembly profile using ticket 44’s native matrix path. Preserve measured key poses, widths, cluster relationships and source references; store its explicit pitch override and 1.0 mm keycap edge gap separately. A global preference must not respace a measured example. Keep one unibody board and independent mechanical instance. Multiple key clusters do not imply a mirror pair. Preserve legitimate wiring and wide-key/member relationships. Preserve one 6.25u and four 2u stabilisers and their qualified family/heading contract; ticket 39 on integration owns the shared stabiliser rule.

## File ownership

One scheduled board implementer owns only this board’s branch/profile in `core/examples/demo_projects/{recipes,physical,mechanical}.rs` and its focused assertions in `core/tests/included_keyboards.rs` (or one board-specific test file). Shared helpers require [44](44-native-recipe-assembly-profiles.md) first. Generated goldens/assets are handed to the designated golden owner; see the phase 2 execution contract in ticket 25. No other board, archive, Core mirror algorithm, catalogue/footprint generator or UI edits belong here.

## Acceptance and regression

- [ ] This board references its own profile; Core materialises exactly 87 switches and the approved companions from matrix membership, with no hand-created duplicate key parts.
- [ ] A public-boundary regression fails first because this board still uses generic preset/profile data; then passes with source-reference pose correspondence within 1e-9 mm and full switch-heading comparison. Assert profile identity and duplicate diode offsets separately from the generated golden.
- [ ] Editing one native matrix value, Undo/Redo and save/reopen preserve the profile, measured exceptions and unrelated parts.

## Verification and remaining facts

Implementer: `repo lint test typecheck`; add `build` when the designated owner regenerates demo outputs. Fresh Sol verifier reproduces the regression and disables this board’s profile selection once to prove it fails. Ticket 23 owns the final mounted-browser/Standards/Spec/full-batch gate. [79](79-unibody-hardware-inputs.md) must confirm switch/diode/LED hardware; this ticket does not certify physical fit. Split new generator or shared-algorithm work into a follow-up rather than widening this slice.
