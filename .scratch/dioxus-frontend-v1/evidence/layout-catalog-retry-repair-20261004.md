# Layout catalogue retry repair — 2026-10-04

## Change

`matrix_transform_controller.rs` now tracks an in-flight catalogue load separately from the last requested construction. A completed error remains quiet until a later assembly action requests the same construction, which clears the request tag and restarts the load. Stale construction results remain ignored. Component assembly admission now checks the requested definitions: an accepted document-owned switch/attachment edit or removal can proceed while the bundled catalogue is unavailable; a new/changed non-document definition still waits for the matching construction's catalogue.

## Regression evidence

Added native tests in `matrix_transform_operation.rs` for document-owned attachment removal, the same-construction requested → loading → failed → user retry → loading → success transition, document-owned switch change and attachment replacement, and a new bundled switch remaining gated. The removal test also runs the existing `KeyAttached` operation builder and verifies the accepted matrix action removes the attachment. Existing mirrored-target locality operation tests remain in the focused run. The transition test exercises the production retry policy and models the controller's signal transitions; it cannot mount the wasm-only Dioxus hook natively.

The new removal and retry regressions were run **before** the behavior change with:

```text
python3 scripts/migration-deliver.py focused-test -- cargo test --manifest-path web/Cargo.toml --features page --lib matrix_transform_operation::tests::failed_catalogue -- --nocapture
```

Result: 0 passed, 2 failed at the intended admission/retry assertions. After the repair, the focused operation module was run with:

```text
python3 scripts/migration-deliver.py focused-test -- cargo test --manifest-path web/Cargo.toml --features page --lib matrix_transform_operation::tests:: -- --nocapture
```

Result: 19 passed, 0 failed; this includes the two existing mirrored-target locality cases. `git diff --check` passed. There is no existing mounted test seam for this controller; the parent reserved wasm/page compilation until the shared presentation source graph settles, so this report makes no wasm mounting or compile claim.
