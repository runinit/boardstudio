# Linked mirrored-pair source checks

This evidence is source-level only. It does not claim packaged/browser parity or close the F3.2 parent and linked-layout Inspector joins.

The geometry regressions were first run against deliberate mutations:

- `mirror-axis-red.log`: the implementation was temporarily changed to put the left source matrix in `Mirror::None`. `projection_matches_react_pair_and_leaves_reflection_to_core` failed because the React source contract requires the left matrix to use `Mirror::X` and the preview right matrix to use `Mirror::None`. The implementation was restored.
- `advanced-token-red.log`: the result predicate was temporarily changed to check revision advancement without checking that the result token differs from the captured base token. `created_pair_selection_requires_exact_advanced_saved_result` failed on a base-token result at revision 12. The distinct-token guard was restored.

The source check then passed all four `mirrored_pair_geometry::tests`, including pair projection, invalid inputs/ID collisions, preview symmetry, and exact advanced/saved result identity. Strict WASM Clippy passed with `-D warnings` after the same source was formatted. The native test target emits unrelated existing dead-code warnings because presentation code is not used in that target.

Commands:

```text
CARGO_TARGET_DIR=/tmp/layout-linked-target cargo test --manifest-path web/Cargo.toml --bin boardstudio-web --features page mirrored_pair_geometry::tests -- --nocapture
CARGO_TARGET_DIR=/tmp/layout-linked-wasm-target cargo clippy --manifest-path web/Cargo.toml --bin boardstudio-web --target wasm32-unknown-unknown --features page,core-worker -- -D warnings
```

Both commands ran in the `layout-linked-authoring-20261002` worktree after merge commit `de8eb2f949addf6e163f5a105312093ccc042531`. The two red test runs used the same test target before restoring their respective mutations.
