# 52: Sofle RGB: native outline, mounting and clearances

Status: needs-info
Type: build
Blocked by: [51](51-sofle-rgb-real-components.md), [46](46-sofle-hardware-inputs.md)
Parent: [25](25-shipped-boards-review.md) · Spec: [spec.md](../spec.md)

## Source evidence

[Sofle measured inputs](../../../../content/layouts/sofle-layouts.json), `layouts.rgb`: `Sofle_RGB/PCB/SofleKeyboard.kicad_pcb` at `fb294f7c58d0f91c379a9ef339159d4d056643e9`; SHA-256 `2162c58ea07f378e4352421c9d2bed31b774a08539114df97515d9f06c058739`. [Phase 1 audit](../audit/shipped-boards-review.md) records the `sofle-rgb` baseline; re-inventory after ticket 20, because its IDs intentionally change.

Historical linked gaps: controller bay 0.10 mm and switch cutout 1.066 mm on both halves. The superseded right bay overlap −3.30 mm and switch gap 0.009 mm are not current requirements. Keep the 14 measured D31–D37 LEDs separate from the 58 matrix LED companions. Matrix LEDs currently use reverse_mount:true; D31–D37 use false. TH1–TH7 remain source holes on each half.

## Hardware gate

[46](46-sofle-hardware-inputs.md) records the user-specified target installed hardware/BOM, qualified plate/switch/stabiliser profile and any remaining outline/mount/clearance decisions. Screws/spacers may be added. Use BoardStudio’s existing case-generation tool for gasket, plate and tolerance; inspect supported defaults/options before asking for a remaining setting. Do not require upstream manufacturing specifications or invent a separate geometry/tolerance workflow. Current Core [builtin_profile](../../../../core/src/mechanical.rs) distinguishes MX, Choc v1 and v2 and retains source provenance; select and validate the intended profile instead of assuming a new Choc dimension or reusing a hardcoded test square.

## One-hour slice and ownership

Convert this variant’s 35-point measured outline through the native outline/version workflow, recording its frame transform and intentional deviations. Configure the selected system through public typed outline/mechanical edits and the existing native case generator. Generate gasket/plate/tolerance geometry, mounts, supports and openings from that native state and the user-specified component envelopes. Any missing generator capability needs a separately scoped follow-up, not an ad hoc geometry implementation. Recalculate closure-to-switch, stabiliser, source-hole, controller and access clearances; do not copy old proposal coordinates. Author/link features once from the construction source and verify both effective halves; keep source TH holes distinct from generated closures.

Own this board’s mechanical/profile branch in `core/examples/demo_projects/mechanical.rs` (and its dispatch in `mod.rs`), outline selection in `recipes.rs` and focused assertions in `core/tests/included_keyboards.rs`. Generated goldens/assets are handed to the designated golden owner; see the phase 2 execution contract in ticket 25. No other board, archive, Core mirror algorithm, catalogue/footprint generator or UI edits belong here.

## Acceptance and regression

- [ ] Public-edit regression first fails on this board’s missing native outline/selected system or approved clearance. It then checks outline/frame correspondence, actual resolved cutout and hardware envelopes, linked feature ownership where applicable, and the approved minimum clearance without loosening a tolerance.
- [ ] PCB/plate/case outputs expose the expected source holes, generated mounts and component access. Store numerical per-board measurements and remaining findings; export success is separate from hardware fit.
- [ ] Undo/Redo and save/reopen preserve the native outline, mounting profile, openings and exceptions. No obsolete historical overlap is presented as the rebuilt result.

## Verification

`repo lint test typecheck build` sequentially after golden-owner handoff; targeted actual PCB/plate/case exports for this board. A fresh Sol verifier disables one native selection or moves one critical mount into a cutout to demonstrate the regression, restores, then reproduces the measurements. If native generation cannot satisfy the approved constraints, retain this gate and file one bounded algorithm follow-up. Ticket 56 owns the targeted pre-sign-off Sofle mounted pass; ticket 23 retains final cross-board mounted-browser/global reviews/full batch; physical fit stays unqualified until its stated evidence exists.
