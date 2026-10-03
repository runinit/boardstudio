# F3.6 3D entry cancels preparing placement

This is a narrow root-composition repair for the existing F3.6 2D/3D transition and F3.2c placement owner. It does not add a ticket, alter the task graph, or claim F3.6/F3.7/F7 viewer acceptance.

## Source pin

- Frozen integration base: `aea1c31eccdb802d0e676bace3cc23cc3aeebc0e`.
- Root's accepted test-only descendant used for checks: `e6dc6c5368ad5e99600be7482c8ecb51e9bddec4`.
- Repair source commit: `98c71bf1e5f0b67c0f6f88b58d6d1a8982239baa`.
- `web/src/presentation.rs`: `7decdc4491d60e68b278d0eaa8da6e1ec79675281d2a5511fc0c371ba0e97bbd`.
- `web/src/presentation/part_placement.rs`: `d9de50f30876d009f897ecc25a50fa86b2fc9f5380a97f63ec15debffce5a068`.

## Behavior and regression seam

The Layout view-mode event handler is factored into one private production handler used by the Editor and the mounted test. On a current-owner 2D → 3D request it cancels a PartPlacement mount when `busy || projection.is_some()`. `busy` includes a still-preparing standalone component whose projection is not published yet. The existing `on_cancel` remains the authority for exact owner admission and leaves already-submitted committing operations alone. The view-mode handler releases the arbiter after cancellation.

The regression mounts the real `use_controller_placement` hook with a suspended catalogue loader, starts generic Add Object placement, and invokes the same production Layout view-mode handler the Editor passes to the toolbar. It asserts the preparing owner has `busy=true` and no projection, enters 3D, observes the one expected `SelectParts(empty)` cancellation, then resolves the old loader and verifies no extra event, edit, or ghost projection appears.

The regression was first run against the old projection-only predicate and failed at the post-switch busy assertion. That expected-red mutant run is recorded in [old-projection-only-red.log](old-projection-only-red.log), SHA-256 `9931788f5264293d87abe195fab6621bc91c40928d402567719ed46b8b5ebc5d`. The fixed focused run is in [fixed-green.log](fixed-green.log), SHA-256 `968c51c445e7e44537b4252d3b960175b2cecb10bcee407a60aad43205d0c279`.

## Verification

- `cargo fmt --manifest-path web/Cargo.toml -- --check` — pass.
- `wasm-pack test --headless --chrome --mode no-install web --no-default-features --features page --bin boardstudio-web -- layout_3d_transition_cancels_suspended_component_preparation_without_ghost` — pass (1 test).
- `wasm-pack test --headless --chrome --mode no-install web --no-default-features --features page --bin boardstudio-web -- part_placement::tests` — pass (35 tests).
- `cargo clippy --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --no-default-features --features page --all-targets -- -D warnings` — pass.

No browser comparison or full F3.6/F3.7 acceptance is claimed by this source regression.
