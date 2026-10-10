# 78: Cantor: native outline, mounting and clearances

Status: needs-info
Type: build
Blocked by: [77](77-cantor-real-components.md), [57](57-measured-split-hardware-inputs.md)
Parent: [25](25-shipped-boards-review.md) · Spec: [spec.md](../spec.md)

## Source evidence

[Measured inputs](../../../../content/layouts/keyboard-layouts.json), `cantor`: `https://github.com/diepala/cantor`, `Cantor_Classic/keyboard_pcb.kicad_pcb` at `13f8bc03bb8a894c7c2871ae64845d5ef927c328`; SHA-256 `c20140c589c1ef393db769f83cfb69a4c94c137528d758de38a88a4fe17b6373`. [Phase 1 audit](../audit/shipped-boards-review.md) records the `measured-cantor` baseline; re-inventory after ticket 20, because its IDs intentionally change.

Historical linked switch gap 0.031 mm and bay clearance 47.17 mm. Recompute from the selected Choc cutout and hardware envelope; accepting 0.031 mm is not part of this plan. No authored TH holes are recorded. Identify the target controller/transport from the selected board configuration; a recipe TRRS is not source evidence that the target uses it.

## Hardware gate

[57](57-measured-split-hardware-inputs.md) must select outline intent, mounting system, plate/switch/stabiliser profile, installed hardware, tolerances and minimum component/service/closure clearance. Current Core [builtin_profile](../../../../core/src/mechanical.rs) distinguishes MX, Choc v1 and v2 and retains source provenance; select and validate the intended profile instead of assuming a new Choc dimension or reusing a hardcoded test square.

## One-hour slice and ownership

Convert the reviewed generated outline into the chosen native outline/version representation. The pinned measured JSON contains key poses, not an upstream boundary; any imported boundary or intentional shape change needs its own source pin and approval in the hardware packet. Configure the selected system through public typed outline/mechanical edits. Generate mounts, supports and openings from that native state and the selected component envelopes. Recalculate closure-to-switch, stabiliser, source-hole, controller and access clearances; do not copy old proposal coordinates. Author/link features once from the construction source and verify both effective halves; keep source TH holes distinct from generated closures.

Own this board’s mechanical/profile branch in `core/examples/demo_projects/mechanical.rs` (and its dispatch in `mod.rs`), outline selection in `recipes.rs` and focused assertions in `core/tests/included_keyboards.rs`. Generated goldens/assets are handed to the designated golden owner; see the phase 2 execution contract in ticket 25. No other board, archive, Core mirror algorithm, catalogue/footprint generator or UI edits belong here.

## Acceptance and regression

- [ ] Public-edit regression first fails on this board’s missing native outline/selected system or approved clearance. It then checks outline/frame correspondence, actual resolved cutout and hardware envelopes, linked feature ownership where applicable, and the approved minimum clearance without loosening a tolerance.
- [ ] PCB/plate/case outputs expose the expected source holes, generated mounts and component access. Store numerical per-board measurements and remaining findings; export success is separate from hardware fit.
- [ ] Undo/Redo and save/reopen preserve the native outline, mounting profile, openings and exceptions. No obsolete historical overlap is presented as the rebuilt result.

## Verification

`repo lint test typecheck build` sequentially after golden-owner handoff; targeted actual PCB/plate/case exports for this board. A fresh Sol verifier disables one native selection or moves one critical mount into a cutout to demonstrate the regression, restores, then reproduces the measurements. If native generation cannot satisfy the approved constraints, retain this gate and file one bounded algorithm follow-up. Ticket 23 owns mounted-browser/global reviews/full batch; physical fit stays unqualified until its stated evidence exists.
