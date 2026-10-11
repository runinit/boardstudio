# 94: Voyager104: native outline, mounting and clearances

Status: wontfix
Type: build
Blocked by: [93](93-voyager104-real-components.md), [79](79-unibody-hardware-inputs.md)
Parent: [25](25-shipped-boards-review.md) · Spec: [spec.md](../spec.md)

## Source evidence

[Measured inputs](../../../../content/layouts/keyboard-layouts.json), `voyager104`: `https://github.com/ai03-2725/Voyager104`, `Voyager104.kicad_pcb` at `0d2180bbcf49532f56c3e6bc3b5fcd9d44128fb1`; SHA-256 `2e3560c90f4df074ffef28bce7bc6b34946cfbdd1985dd2d611d4070e4e18a06`. [Phase 1 audit](../audit/shipped-boards-review.md) records the `measured-voyager104` baseline; re-inventory after ticket 20, because its IDs intentionally change.

No mechanical configuration, case body or mounting-hole parts exist in the current fixture. Generate the selected native system and clearances, including the numpad. The current fixture contains seven 2u and one 6.25u stabilisers. Preserve all ANSI widths and source references.

## Hardware gate

[79](79-unibody-hardware-inputs.md) must select outline intent, mounting system, plate/switch/stabiliser profile, installed hardware, tolerances and minimum component/service/closure clearance. Current Core [builtin_profile](../../../../core/src/mechanical.rs) distinguishes MX, Choc v1 and v2 and retains source provenance; select and validate the intended profile instead of assuming a new Choc dimension or reusing a hardcoded test square.

## One-hour slice and ownership

Convert the reviewed generated outline into the chosen native outline/version representation. The pinned measured JSON contains key poses, not an upstream boundary; any imported boundary or intentional shape change needs its own source pin and approval in the hardware packet. Configure the selected system through public typed outline/mechanical edits. Generate mounts, supports and openings from that native state and the selected component envelopes. Recalculate closure-to-switch, stabiliser, source-hole, controller and access clearances; do not copy old proposal coordinates. Configure one independent instance, with no artificial pair.

Own this board’s mechanical/profile branch in `core/examples/demo_projects/mechanical.rs` (and its dispatch in `mod.rs`), outline selection in `recipes.rs` and focused assertions in `core/tests/included_keyboards.rs`. Generated goldens/assets are handed to the designated golden owner; see the phase 2 execution contract in ticket 25. No other board, archive, Core mirror algorithm, catalogue/footprint generator or UI edits belong here.

## Acceptance and regression

- [ ] Public-edit regression first fails on this board’s missing native outline/selected system or approved clearance. It then checks outline/frame correspondence, actual resolved cutout and hardware envelopes, linked feature ownership where applicable, and the approved minimum clearance without loosening a tolerance.
- [ ] PCB/plate/case outputs expose the expected source holes, generated mounts and component access. Store numerical per-board measurements and remaining findings; export success is separate from hardware fit.
- [ ] Undo/Redo and save/reopen preserve the native outline, mounting profile, openings and exceptions. No obsolete historical overlap is presented as the rebuilt result.

## Verification

`repo lint test typecheck build` sequentially after golden-owner handoff; targeted actual PCB/plate/case exports for this board. A fresh Sol verifier disables one native selection or moves one critical mount into a cutout to demonstrate the regression, restores, then reproduces the measurements. If native generation cannot satisfy the approved constraints, retain this gate and file one bounded algorithm follow-up. Ticket 23 owns mounted-browser/global reviews/full batch; physical fit stays unqualified until its stated evidence exists.

## Supersession answer (2026-10-11)

The separate per-board implementation ticket is retired. The board requirement is superseded by ticket [25](25-shipped-boards-review.md): author one ordinary saved project for this board through the existing Core/Archive/Artifact JSON request examples and common user operations. Do not add a per-board builder, branch, behavior suite, or extracted helper. Keep the source research, geometry, component evidence, and prior proposals above as historical context only; they do not authorize implementation. Shared mirrored-half/link functionality remains required separately under tickets 19/22, while board creation does not wait for 19, 22, or 44.
