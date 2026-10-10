# 88: Mysterium: native outline, mounting and clearances

Status: needs-info
Type: build
Blocked by: [87](87-mysterium-real-components.md), [79](79-unibody-hardware-inputs.md)
Parent: [25](25-shipped-boards-review.md) · Spec: [spec.md](../spec.md)

## Source evidence

[Measured inputs](../../../../content/layouts/keyboard-layouts.json), `mysterium`: `https://github.com/coseyfannitutti/mysterium`, `mysterium-pcb.kicad_pcb` at `5e5ab18d8d9608759e5f50ad9e62fc7329271551`; SHA-256 `dee1f0f51fdf5ebf93690aee18d2e1bd6d435ad0588a80b0d23e7e0a75e031a1`. [Phase 1 audit](../audit/shipped-boards-review.md) records the `measured-mysterium` baseline; re-inventory after ticket 20, because its IDs intentionally change.

No mechanical configuration, case body or mounting-hole parts exist in the current fixture. Generate the approved system and do not report missing historical fit results as a pass. There is no numpad matrix. The current fixture contains four 2u and one 6.25u stabilisers; preserve the separate cluster origins and wide-key geometry.

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
