# Linked mirrored-pair owner repair evidence

This packet records source-level owner regressions for the linked-pair feature. It does not claim packaged/browser parity, complete PartPlacement arbitration, or close the F3.2 parent and linked-layout Inspector joins.

Four new tests were observed red under targeted mutations, then all six mirrored-pair tests passed with the repairs restored:

- `escape-red.log`: Escape's return-to-form transition was removed; the regression failed because the retained values remained in Placement instead of returning to Setup.
- `settlement-transaction-red.log`: the exact accepted `SceneDelta.transaction_id` check was removed; a later unrelated saved transaction was accepted as this pair's result.
- `preview-form-hidden-red.log`: setup visibility was forced true; the regression failed because the form remained visible during canvas placement.
- `core-projection-red.log`: preview adaptation was temporarily changed to use the input matrix origin instead of the Core-projected cell pose; the regression failed on the fixture's offset/stagger/splay/rotation geometry.

The repaired owner retains form values when Escape exits the ghost, hides the setup form while placement owns the canvas, and binds successful selection to the exact transaction ID, advanced snapshot token/revision, Ready lifecycle, and Saved durability. Candidate preview uses the existing private `CoreRequest::ProjectMatrices` worker path. The UI adapts Core cell poses rather than reimplementing matrix offset, stagger, splay, mirror, or rotation geometry.

The focused native run passed six tests, including Core projection with a disabled cell and asymmetric geometry. Strict WASM Clippy passed with `-D warnings`. The native binary test target reports unrelated dead-code warnings because presentation-only code is not exercised there.

Commands, run in this worktree:

```text
CARGO_TARGET_DIR=/tmp/layout-linked-target cargo test --manifest-path web/Cargo.toml --bin boardstudio-web --features page mirrored_pair -- --nocapture
CARGO_TARGET_DIR=/tmp/layout-linked-wasm-target cargo clippy --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --features page,core-worker -- -D warnings
```

The earlier source checks `mirror-axis-red.log` and `advanced-token-red.log` remain from the original packet. No browser/build evidence is asserted by this repair packet.
