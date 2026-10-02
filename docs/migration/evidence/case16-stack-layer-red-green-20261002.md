# Case16 configured stack layer regression

This records the browser-level red/green for preserving configured mechanical
stack rows when Case CAD has no matching generated body.

## Red

The disposable worktree `codex/case-stack-red-evidence-20261002` was based on
`fc6ac67451e9de37849fe227b7df2fc366c14ca0`. In that worktree only, the
`assembly_layers_with_stack` implementation was mutated to return the previous
generated-body-only projection. The mounted test used an explicit presence
assertion before checking disabled and unchecked state.

Command:

```sh
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=/home/chris/.cache/.wasm-pack/wasm-bindgen-9411bdb1a3e2bbb9/wasm-bindgen-test-runner \
CHROMEDRIVER=/usr/bin/chromedriver \
WASM_BINDGEN_TEST_ONLY_WEB=1 \
CARGO_TARGET_DIR=/home/chris/.local/share/boardstudio/retained-tmp/20261002/case-assembly-layer-target \
cargo test --manifest-path web/Cargo.toml --target wasm32-unknown-unknown \
  --bin boardstudio-web --features page \
  presentation::case_assembly_layers::tests::layer_menu_toggles_exact_rows_and_unavailable_rows_are_not_checked
```

The expected red was exit 1 at `configured battery stack row must remain visible
when its CAD mesh is absent`. The raw output is retained at
`/home/chris/.local/share/boardstudio/retained-tmp/20261002/case-stack-old-union-red.log`
(SHA-256 `edb560042fa9941aa6144781508b5a8d7e7a5594b6c37c7f05b727d402d4c9dc`).
The disposable source mutation and test diagnostic edit were not committed.

## Green

Implementation source is `9a0aa99209a8c535bea639dd40bac725ef9ea724`.
Generated body IDs remain available; configured stack IDs are unioned in order,
and stack-only rows such as `battery` are rendered disabled and unchecked. The
same-source guard is retained at the shared viewer. React's display-label
overrides for `plate`, `plate-foam`, `bottom-foam`, and `bottom` apply to both
generated and stack-only rows.

The focused mounted Chrome/WASM Case layer suite passed 5/5. Its raw output is
retained at
`/home/chris/.local/share/boardstudio/retained-tmp/20261002/case-stack-union-green.log`
(SHA-256 `cacd78b88c67a4354128131500093e11068789fb26ede39e51fabcb3d3746522`).
Strict page all-target WASM Clippy (`-D warnings`), `cargo fmt --check`, and
`git diff --check` also passed at the same implementation source.

This source and test receipt do not claim packaged browser or Case parent
acceptance.
