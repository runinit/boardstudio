# 44: Build recipe key profiles through the native matrix boundary

Status: wontfix
Type: build
Blocked by: [20](20-recipes-build-one-half.md), [31](31-companion-placement-modes.md), 39 (integration)
Parent: [25](25-shipped-boards-review.md) · Spec: [spec.md](../spec.md)

## Source evidence

physical::append (`core/examples/demo_projects/physical.rs`, retired) currently constructs raw matrix cells and separate switch/diode/LED/stabiliser Parts with generic `preset/...` identities. Core's public `EditOperation::SetMatrix` derives live membership/parts; [prepare_matrix](../../../../web/crates/catalogue/src/matrix_setup_operation.rs) is an example of preparing a typed matrix and definitions while leaving materialisation to Core. Native recipes must not import the WASM web crate.

## One-hour slice and ownership

One recipe-foundation implementer owns `core/examples/demo_projects/physical.rs`, a small private helper alongside it if necessary, the call boundary in `mod.rs`, and a focused Core recipe test. Browser preference tickets 42/43 are independent: native recipes supply explicit measured pitch and do not consume a browser preference. Introduce a typed profile input (name, switch/companion definitions, explicit pitch/edge gap and approved local offsets). Keep the existing measured transform calculation; send the prepared matrix/definitions through the existing public Core edit path rather than manually append duplicate live Parts. Establish the path on one bounded synthetic/representative recipe probe; individual shipped profiles belong to 47–103.

Do not redesign Core assembly schemas, move a web helper into Core, widen visibility, alter mirror algorithms or rebuild every board here. Ticket 20 owns one-half creation and numbering. Ticket 39 on integration owns stabiliser heading/fallback behavior and must be resolved before a profile using those semantics is verified; its file is absent on the planning lane.

## Acceptance and regression

- [ ] A profile produces Core-owned switches/companions/membership exactly once, with stable profile identity, explicit spacing, side and parameters retained.
- [ ] Public-boundary test covers measured rotated keys, a disabled cell, an RGB companion and a wide-key stabiliser, and compares derived pose/member identities and Undo/reopen behavior. Demonstrate that bypassing the native edit/materialisation site fails the test.
- [ ] No shipped hardware choice is invented. Duplicate-position mode is selected for current shipped diode data by per-board tickets; default user-created literal mirror behavior remains.

## Verification

`repo lint test typecheck`; `build` only if output changes, coordinated with the designated golden owner. Fresh Sol verifier reproduces the behavior and one-site mutation. Shared algorithm defects become separate tickets; the planner's missing lane index is not a safe-impact verdict. Board dispatches use the current post-20 interfaces and explicit file ownership.

## Supersession answer (2026-10-11)

The separate per-board implementation ticket is retired. The board requirement is superseded by ticket [25](25-shipped-boards-review.md): author one ordinary saved project for this board through the existing Core/Archive/Artifact JSON request examples and common user operations. Do not add a per-board builder, branch, behavior suite, or extracted helper. Keep the source research, geometry, component evidence, and prior proposals above as historical context only; they do not authorize implementation. Shared mirrored-half/link functionality remains required separately under tickets 19/22, while board creation does not wait for 19, 22, or 44.
