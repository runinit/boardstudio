# 43: Apply key-spacing preferences to new native assemblies

Status: ready-for-agent
Type: build
Blocked by: [42](42-global-key-spacing-preference.md)
Parent: [25](25-shipped-boards-review.md) · Spec: [spec.md](../spec.md)

## Source evidence

`PITCH_MM` and `prepare_matrix` in [matrix_setup_operation.rs](../../../../web/crates/catalogue/src/matrix_setup_operation.rs) currently seed 19.05 mm and edgeGap 1.0 mm. [matrix_placement_controller.rs](../../../../web/crates/layout/src/objects/matrix_placement_controller.rs) also contains a matrix seed. The Parts assembly route uses [assembly_presets.rs](../../../../web/crates/parts/src/parts/assembly_presets.rs); identify whether it consumes spacing before changing it. Graph evidence must be refreshed/bound at execution; the planning lane index was unavailable.

## One-hour slice and ownership

One creation-route implementer owns the spacing input to `web/crates/catalogue/src/matrix_setup_operation.rs`, its caller in `web/crates/layout/src/objects/matrix_setup_controller.rs`, and the matrix seed in `matrix_placement_controller.rs`. Read other callers before any shared signature change. Consume ticket 42's preference only when creating a new matrix/assembly without explicit spacing. Keep normal native assembly behavior, explicit per-board choices, loaded matrices and a matrix's own edgeGap independent. Parts placement already expressing explicit keycap/spacing data must keep it; file any separate missing route rather than expand this slice.

## Acceptance and regression

- [ ] New default and 19.00 mm preference requests persist the chosen pitch plus independent edgeGap, and Core derives positions from those values.
- [ ] Public creation regression first fails on hardcoded pitch, then checks two adjacent keys, separate edge-gap preview, Undo/Redo and portable reopen. An explicit 18 × 17 or 19.05 board override wins over the preference.
- [ ] Reopen/edit of an existing measured matrix changes no pose, width, spacing or profile because the preference changed. The preference is captured for creation, not read again to rewrite a queued edit.

## Verification

`repo lint test typecheck browser`; meaningful native/catalogue and mounted creation tests, followed by a fresh Sol verifier reverting one creation site at a time and observing the corresponding failure. No demo/golden change is expected. If the graph reports HIGH/CRITICAL shared-signature risk, report callers/processes before editing. Ticket 23 keeps global integration reviews/full check.
